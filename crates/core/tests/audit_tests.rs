use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use rustget_core::db::DownloadRecord;
use rustget_core::downloader::{DownloadOptions, DownloadProgress, Downloader};
use rustget_core::probe::{redact_url_for_log, sanitize_filename, validate_http_url};
use rustget_core::segment::Segment;
use rustget_core::server::get_or_create_bridge_token;
use rustget_core::state::DownloadState;
use rustget_core::{BridgeServer, Category, Database, QueueManager};

#[test]
fn test_url_and_filename_security_and_redaction() {
    assert!(validate_http_url("https://example.com/file.zip").is_ok());
    assert!(validate_http_url("http://127.0.0.1:8080/payload.bin").is_ok());
    assert!(validate_http_url("javascript:alert(1)").is_err());
    assert!(validate_http_url("file:///C:/Windows/win.ini").is_err());
    assert!(validate_http_url("").is_err());
    assert!(validate_http_url("/relative/path.zip").is_err());

    let trav = sanitize_filename("..\\..\\evil.exe");
    assert!(!trav.contains('\\') && !trav.contains('/'));
    assert_eq!(std::path::Path::new(&trav).components().count(), 1);
    assert!(trav.ends_with("evil.exe"));
    assert_eq!(sanitize_filename("CON"), "_CON");
    assert_eq!(sanitize_filename("nul.txt"), "_nul.txt");
    assert_eq!(sanitize_filename("COM1.log"), "_COM1.log");
    assert_eq!(sanitize_filename("lpt9"), "_lpt9");

    let long_name = format!("{}.zip", "a".repeat(250));
    let sanitized_long = sanitize_filename(&long_name);
    assert!(sanitized_long.len() <= 180);
    assert!(sanitized_long.ends_with(".zip"));

    let redacted = redact_url_for_log("https://user:secret@cdn.example.com/file.zip?token=abc123&sig=xyz");
    assert!(!redacted.contains("secret"));
    assert!(!redacted.contains("abc123"));
    assert!(redacted.contains("cdn.example.com"));
}

#[tokio::test]
async fn test_collision_auto_rename() {
    let payload = b"cosmo-auto-rename-payload".to_vec();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let data = Arc::new(payload.clone());

    let srv_data = Arc::clone(&data);
    let srv = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let d = Arc::clone(&srv_data);
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                let req = String::from_utf8_lossy(&buf[..n]);
                if req.starts_with("HEAD") {
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccept-Ranges: none\r\nConnection: close\r\n\r\n",
                        d.len()
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                } else {
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        d.len()
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.write_all(&d).await;
                }
                let _ = stream.shutdown().await;
            });
        }
    });

    let tmp = tempdir().unwrap();
    let p = tmp.path().join("archive.zip");
    std::fs::write(&p, b"existing-first").unwrap();

    let (tx1, mut rx1) = mpsc::channel::<DownloadProgress>(16);
    tokio::spawn(async move { while rx1.recv().await.is_some() {} });
    let dl1 = Downloader::new(
        DownloadOptions {
            url: format!("http://{}/archive.zip", addr),
            output_path: Some(p.clone()),
            num_segments: 1,
            resume: false,
            ..Default::default()
        },
        CancellationToken::new(),
    );
    let saved1 = dl1.start(tx1).await.unwrap();
    assert_eq!(saved1, tmp.path().join("archive (1).zip"));
    assert_eq!(std::fs::read(&p).unwrap(), b"existing-first");
    assert_eq!(std::fs::read(&saved1).unwrap(), payload);

    let (tx2, mut rx2) = mpsc::channel::<DownloadProgress>(16);
    tokio::spawn(async move { while rx2.recv().await.is_some() {} });
    let dl2 = Downloader::new(
        DownloadOptions {
            url: format!("http://{}/archive.zip", addr),
            output_path: Some(p.clone()),
            num_segments: 1,
            resume: false,
            ..Default::default()
        },
        CancellationToken::new(),
    );
    let saved2 = dl2.start(tx2).await.unwrap();
    assert_eq!(saved2, tmp.path().join("archive (2).zip"));
    assert_eq!(std::fs::read(&saved2).unwrap(), payload);

    srv.abort();
}

#[tokio::test]
async fn test_truncated_segment_never_produces_corrupt_final_file() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let total_size = 131072usize;

    let handle = tokio::spawn(async move {
        for _ in 0..8 {
            let Ok((mut stream, _)) = tokio::time::timeout(Duration::from_secs(2), listener.accept())
                .await
                .unwrap_or(Err(std::io::Error::other("timeout")))
            else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                let req = String::from_utf8_lossy(&buf[..n]);
                if req.starts_with("HEAD") {
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nETag: \"v1\"\r\nConnection: close\r\n\r\n",
                        total_size
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                } else {
                    // Always return truncated partial content (only 100 bytes instead of full segment)
                    let resp = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-99/{}\r\nContent-Length: 32768\r\nConnection: close\r\n\r\n",
                        total_size
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.write_all(&[0x41u8; 100]).await;
                }
                let _ = stream.shutdown().await;
            });
        }
    });

    let tmp = tempdir().unwrap();
    let target = tmp.path().join("broken.bin");
    let (tx, mut rx) = mpsc::channel::<DownloadProgress>(16);
    tokio::spawn(async move { while rx.recv().await.is_some() {} });

    let dl = Downloader::new(
        DownloadOptions {
            url: format!("http://{}/broken.bin", addr),
            output_path: Some(target.clone()),
            num_segments: 4,
            resume: false,
            ..Default::default()
        },
        CancellationToken::new(),
    );
    let res = dl.start(tx).await;

    assert!(res.is_err(), "Truncated download must fail");
    assert!(!target.exists(), "Final file must NOT exist after failed/truncated download");
    handle.abort();
}

#[tokio::test]
async fn test_single_stream_range_resume_and_etag_mismatch() {
    let data = Arc::new((0..65536).map(|i| (i % 251) as u8).collect::<Vec<u8>>());
    let full_hash = format!("{:x}", Sha256::digest(&*data));
    let etag = Arc::new(AtomicUsize::new(1));
    let saw_range = Arc::new(AtomicBool::new(false));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let s_data = Arc::clone(&data);
    let s_etag = Arc::clone(&etag);
    let s_saw_range = Arc::clone(&saw_range);

    let srv = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let d = Arc::clone(&s_data);
            let e = Arc::clone(&s_etag);
            let sr = Arc::clone(&s_saw_range);
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                let req = String::from_utf8_lossy(&buf[..n]);
                let tag = e.load(Ordering::SeqCst);
                if req.starts_with("HEAD") {
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nETag: \"tag-{}\"\r\nConnection: close\r\n\r\n",
                        d.len(),
                        tag
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                } else {
                    let mut start = 0usize;
                    for line in req.lines() {
                        if let Some(rest) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                            if let Some(first) = rest.split('-').next() {
                                if let Ok(v) = first.trim().parse::<usize>() {
                                    start = v;
                                    sr.store(true, Ordering::SeqCst);
                                }
                            }
                        }
                    }
                    if start > 0 && start < d.len() {
                        let rem = &d[start..];
                        let resp = format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            start,
                            d.len() - 1,
                            d.len(),
                            rem.len()
                        );
                        let _ = stream.write_all(resp.as_bytes()).await;
                        let _ = stream.write_all(rem).await;
                    } else {
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: \"tag-{}\"\r\nConnection: close\r\n\r\n",
                            d.len(),
                            tag
                        );
                        let _ = stream.write_all(resp.as_bytes()).await;
                        let _ = stream.write_all(&d).await;
                    }
                }
                let _ = stream.shutdown().await;
            });
        }
    });

    // Seed half-downloaded .cosmo.part + .cosmo.part.state.json with single segment & matching ETag
    let tmp = tempdir().unwrap();
    let target = tmp.path().join("single.bin");
    let part = tmp.path().join("single.bin.cosmo.part");
    let half = 32768usize;
    std::fs::write(&part, &data[..half]).unwrap();
    let mut state = DownloadState {
        id: "single-1".to_string(),
        url: format!("http://{}/single.bin", addr),
        target_path: target.clone(),
        part_path: part.clone(),
        total_size: Some(data.len() as u64),
        accepts_ranges: true,
        etag: Some("\"tag-1\"".to_string()),
        last_modified: None,
        expected_sha256: Some(full_hash.clone()),
        segments: vec![Segment {
            index: 0,
            start: 0,
            end: (data.len() - 1) as u64,
            downloaded: half as u64,
            complete: false,
        }],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.save_atomic().await.unwrap();

    let (tx1, mut rx1) = mpsc::channel::<DownloadProgress>(16);
    tokio::spawn(async move { while rx1.recv().await.is_some() {} });
    let dl = Downloader::new(
        DownloadOptions {
            url: format!("http://{}/single.bin", addr),
            output_path: Some(target.clone()),
            num_segments: 1,
            expected_sha256: Some(full_hash.clone()),
            resume: true,
            ..Default::default()
        },
        CancellationToken::new(),
    );
    dl.start(tx1).await.unwrap();

    assert!(saw_range.load(Ordering::SeqCst), "Single-stream resume should send Range header");
    let downloaded = std::fs::read(&target).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&downloaded)), full_hash);

    // Now test ETag mismatch cleanup: seed corrupt half + old tag-1 while server now serves tag-2
    let target2 = tmp.path().join("changed.bin");
    let part2 = tmp.path().join("changed.bin.cosmo.part");
    std::fs::write(&part2, vec![0xFFu8; half]).unwrap();
    etag.store(2, Ordering::SeqCst);
    let mut state2 = DownloadState {
        id: "single-2".to_string(),
        url: format!("http://{}/changed.bin", addr),
        target_path: target2.clone(),
        part_path: part2.clone(),
        total_size: Some(data.len() as u64),
        accepts_ranges: true,
        etag: Some("\"tag-1\"".to_string()),
        last_modified: None,
        expected_sha256: Some(full_hash.clone()),
        segments: vec![Segment {
            index: 0,
            start: 0,
            end: (data.len() - 1) as u64,
            downloaded: half as u64,
            complete: false,
        }],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state2.save_atomic().await.unwrap();

    let (tx2, mut rx2) = mpsc::channel::<DownloadProgress>(16);
    tokio::spawn(async move { while rx2.recv().await.is_some() {} });
    let dl2 = Downloader::new(
        DownloadOptions {
            url: format!("http://{}/changed.bin", addr),
            output_path: Some(target2.clone()),
            num_segments: 1,
            expected_sha256: Some(full_hash.clone()),
            resume: true,
            ..Default::default()
        },
        CancellationToken::new(),
    );
    dl2.start(tx2).await.unwrap();

    let downloaded2 = std::fs::read(&target2).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&downloaded2)), full_hash);
    srv.abort();
}

#[tokio::test]
async fn test_queue_recovery_and_shutdown_save() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("queue_test.db");
    let db = Database::open(&db_path).unwrap();

    let rec = DownloadRecord {
        id: "interrupted-1".to_string(),
        url: "https://example.com/file.iso".to_string(),
        filename: "file.iso".to_string(),
        target_path: tmp.path().join("file.iso").to_string_lossy().to_string(),
        part_path: tmp.path().join("file.iso.cosmo.part").to_string_lossy().to_string(),
        category: Category::Compressed.name().to_string(),
        total_size: Some(1024),
        downloaded_bytes: 512,
        status: "downloading".to_string(),
        num_segments: 8,
        expected_sha256: None,
        custom_headers: None,
        user_agent: None,
        referer: None,
        cookie: None,
        max_speed_bps: 0,
        retry_count: 1,
        max_retries: 3,
        error_message: None,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    };
    db.insert_download(&rec).await.unwrap();

    let recovered = db.recover_interrupted_downloads().await.unwrap();
    assert_eq!(recovered, 1);

    let after = db.get_download("interrupted-1").await.unwrap().unwrap();
    assert_eq!(after.status, "queued");

    let (ev_tx, mut ev_rx) = mpsc::channel(16);
    tokio::spawn(async move { while ev_rx.recv().await.is_some() {} });
    let qm = Arc::new(QueueManager::new(db, 3, 0, ev_tx));
    qm.shutdown_and_save().await;
}

async fn raw_http(port: u16, req: &str) -> String {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream.write_all(req.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
    let mut buf = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), stream.read_to_end(&mut buf)).await;
    String::from_utf8_lossy(&buf).to_string()
}

#[tokio::test]
async fn test_bridge_server_csrf_url_validation_and_quit_auth() {
    let tmp = tempdir().unwrap();
    let db = Database::open(&tmp.path().join("bridge.db")).unwrap();
    let (ev_tx, mut ev_rx) = mpsc::channel(16);
    tokio::spawn(async move { while ev_rx.recv().await.is_some() {} });
    let qm = Arc::new(QueueManager::new(db, 3, 0, ev_tx));

    let probe_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe_listener.local_addr().unwrap().port();
    drop(probe_listener);

    let (quit_tx, mut quit_rx) = mpsc::channel(1);
    let cancel = CancellationToken::new();
    let srv_cancel = cancel.clone();
    let bridge = BridgeServer::new(Arc::clone(&qm), port).with_quit_notifier(quit_tx);
    tokio::spawn(async move {
        bridge.run(srv_cancel).await;
    });
    tokio::time::sleep(Duration::from_millis(120)).await;

    // 1. Cross-origin / null Origin / Sec-Fetch-Site: cross-site rejected with 403
    let r1 = raw_http(
        port,
        "POST /download HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: https://evil.example\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
    )
    .await;
    assert!(r1.starts_with("HTTP/1.1 403"), "Unexpected response: {}", r1);

    let r2 = raw_http(
        port,
        "POST /download HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: null\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
    )
    .await;
    assert!(r2.starts_with("HTTP/1.1 403"), "Origin: null must be blocked: {}", r2);

    let r3 = raw_http(
        port,
        "POST /download HTTP/1.1\r\nHost: 127.0.0.1\r\nSec-Fetch-Site: cross-site\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
    )
    .await;
    assert!(r3.starts_with("HTTP/1.1 403"), "cross-site must be blocked: {}", r3);

    // 2. Invalid scheme in /download payload rejected with 400
    let bad_body = r#"{"url":"javascript:alert(1)"}"#;
    let r4 = raw_http(
        port,
        &format!(
            "POST /download HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            bad_body.len(),
            bad_body
        ),
    )
    .await;
    assert!(r4.starts_with("HTTP/1.1 400"), "javascript URL must return 400: {}", r4);

    // 3. /quit requires valid X-Cosmo-Token
    let r5 = raw_http(
        port,
        "POST /quit HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(
        r5.starts_with("HTTP/1.1 401") || r5.starts_with("HTTP/1.1 403"),
        "/quit without token must fail: {}",
        r5
    );

    let tok = get_or_create_bridge_token();
    let r6 = raw_http(
        port,
        &format!(
            "POST /quit HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Cosmo-Token: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            tok
        ),
    )
    .await;
    assert!(r6.starts_with("HTTP/1.1 200"), "/quit with token must succeed: {}", r6);
    assert!(tokio::time::timeout(Duration::from_secs(1), quit_rx.recv()).await.is_ok());

    cancel.cancel();
}

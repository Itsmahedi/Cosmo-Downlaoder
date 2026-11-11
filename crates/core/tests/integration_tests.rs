use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use reqwest::header::HeaderMap;
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use rustget_core::{DownloadOptions, Downloader};

/// Simple mock HTTP server handling HEAD and GET with Range header support.
struct MockServer {
    addr: SocketAddr,
    shutdown_token: CancellationToken,
}

impl MockServer {
    async fn start(data: Vec<u8>, range_supported: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let data = Arc::new(data);
        let range_supported = Arc::new(AtomicBool::new(range_supported));
        let shutdown_token = CancellationToken::new();

        let s_data = Arc::clone(&data);
        let s_ranges = Arc::clone(&range_supported);
        let s_token = shutdown_token.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = s_token.cancelled() => break,
                    accept_res = listener.accept() => {
                        let (mut stream, _) = match accept_res {
                            Ok(res) => res,
                            Err(_) => break,
                        };

                        let c_data = Arc::clone(&s_data);
                        let c_ranges = Arc::clone(&s_ranges);

                        tokio::spawn(async move {
                            let mut buf = [0u8; 4096];
                            let n = match stream.read(&mut buf).await {
                                Ok(n) if n > 0 => n,
                                _ => return,
                            };

                            let req_str = String::from_utf8_lossy(&buf[..n]);
                            let is_head = req_str.starts_with("HEAD");
                            let total_len = c_data.len();
                            let supports_range = c_ranges.load(Ordering::SeqCst);

                            // Parse Range header: Range: bytes=X-Y
                            let mut range_req: Option<(usize, usize)> = None;
                            if supports_range {
                                for line in req_str.lines() {
                                    if line.to_ascii_lowercase().starts_with("range: bytes=") {
                                        if let Some(r_str) = line.split('=').nth(1) {
                                            let parts: Vec<&str> = r_str.split('-').collect();
                                            if parts.len() == 2 {
                                                let start = parts[0].trim().parse::<usize>().ok().unwrap_or(0);
                                                let end = parts[1].trim().parse::<usize>().ok().unwrap_or(total_len - 1);
                                                range_req = Some((start, end.min(total_len - 1)));
                                            }
                                        }
                                    }
                                }
                            }

                            if is_head {
                                let ranges_val = if supports_range { "bytes" } else { "none" };
                                let resp = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccept-Ranges: {}\r\nContent-Disposition: attachment; filename=\"payload.bin\"\r\nConnection: close\r\n\r\n",
                                    total_len, ranges_val
                                );
                                let _ = stream.write_all(resp.as_bytes()).await;
                            } else if let Some((start, end)) = range_req {
                                let chunk_len = (end - start) + 1;
                                let resp_header = format!(
                                    "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    start, end, total_len, chunk_len
                                );
                                let _ = stream.write_all(resp_header.as_bytes()).await;
                                let _ = stream.write_all(&c_data[start..=end]).await;
                            } else {
                                let ranges_val = if supports_range { "bytes" } else { "none" };
                                let resp_header = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccept-Ranges: {}\r\nConnection: close\r\n\r\n",
                                    total_len, ranges_val
                                );
                                let _ = stream.write_all(resp_header.as_bytes()).await;
                                let _ = stream.write_all(&c_data).await;
                            }

                            let _ = stream.flush().await;
                        });
                    }
                }
            }
        });

        Self {
            addr,
            shutdown_token,
        }
    }

    fn url(&self) -> String {
        format!("http://{}/download", self.addr)
    }

    fn stop(self) {
        self.shutdown_token.cancel();
    }
}

#[tokio::test]
async fn test_multisegment_download_integration() {
    // 1. Generate 128 KB of deterministic test payload
    let mut payload = Vec::with_capacity(128 * 1024);
    for i in 0..(128 * 1024) {
        payload.push((i % 251) as u8);
    }

    let server = MockServer::start(payload.clone(), true).await;
    let dir = tempdir().unwrap();
    let out_file = dir.path().join("payload.bin");

    let cancel = CancellationToken::new();
    let options = DownloadOptions {
        url: server.url(),
        output_path: Some(out_file.clone()),
        num_segments: 4,
        headers: HeaderMap::new(),
        user_agent: Some("RustGet-Test/1.0".to_string()),
        referer: None,
        cookie: None,
        expected_sha256: None,
        resume: true,
    };

    let downloader = Downloader::new(options, cancel);
    let (tx, mut rx) = mpsc::channel(100);

    // Drain progress
    tokio::spawn(async move {
        while rx.recv().await.is_some() {}
    });

    let res = downloader.start(tx).await;
    assert!(res.is_ok(), "Download failed: {:?}", res.err());
    let saved_path = res.unwrap();
    assert_eq!(saved_path, out_file);
    assert!(out_file.exists());

    // Verify content matches perfectly
    let downloaded_bytes = tokio::fs::read(&out_file).await.unwrap();
    assert_eq!(downloaded_bytes.len(), payload.len());
    assert_eq!(downloaded_bytes, payload);

    server.stop();
}

#[tokio::test]
async fn test_single_stream_fallback() {
    let payload = b"Hello, this server does not support HTTP Range requests!".to_vec();
    let server = MockServer::start(payload.clone(), false).await;
    let dir = tempdir().unwrap();
    let out_file = dir.path().join("fallback.txt");

    let cancel = CancellationToken::new();
    let options = DownloadOptions {
        url: server.url(),
        output_path: Some(out_file.clone()),
        num_segments: 4, // Requests 4 segments, but server doesn't support ranges
        headers: HeaderMap::new(),
        user_agent: None,
        referer: None,
        cookie: None,
        expected_sha256: None,
        resume: false,
    };

    let downloader = Downloader::new(options, cancel);
    let (tx, mut rx) = mpsc::channel(100);
    tokio::spawn(async move { while rx.recv().await.is_some() {} });

    let res = downloader.start(tx).await;
    assert!(res.is_ok(), "Single connection fallback failed: {:?}", res.err());

    let downloaded = tokio::fs::read(&out_file).await.unwrap();
    assert_eq!(downloaded, payload);

    server.stop();
}

#[tokio::test]
async fn test_checksum_verification_integration() {
    let payload = b"Cryptographically signed payload for RustGet testing".to_vec();
    let server = MockServer::start(payload.clone(), true).await;
    let dir = tempdir().unwrap();
    let out_file = dir.path().join("verified.dat");

    // Compute expected hash
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&payload);
    let correct_sha256 = format!("{:x}", hasher.finalize());

    let cancel = CancellationToken::new();
    let options = DownloadOptions {
        url: server.url(),
        output_path: Some(out_file.clone()),
        num_segments: 2,
        headers: HeaderMap::new(),
        user_agent: None,
        referer: None,
        cookie: None,
        expected_sha256: Some(correct_sha256),
        resume: false,
    };

    let downloader = Downloader::new(options, cancel);
    let (tx, mut rx) = mpsc::channel(100);
    tokio::spawn(async move { while rx.recv().await.is_some() {} });

    let res = downloader.start(tx).await;
    assert!(res.is_ok(), "Checksum verification failed: {:?}", res.err());

    // Test with incorrect checksum
    let out_file_bad = dir.path().join("bad.dat");
    let cancel_bad = CancellationToken::new();
    let bad_options = DownloadOptions {
        url: server.url(),
        output_path: Some(out_file_bad),
        num_segments: 2,
        headers: HeaderMap::new(),
        user_agent: None,
        referer: None,
        cookie: None,
        expected_sha256: Some("0000000000000000000000000000000000000000000000000000000000000000".to_string()),
        resume: false,
    };

    let downloader_bad = Downloader::new(bad_options, cancel_bad);
    let (tx_bad, mut rx_bad) = mpsc::channel(100);
    tokio::spawn(async move { while rx_bad.recv().await.is_some() {} });

    let res_bad = downloader_bad.start(tx_bad).await;
    assert!(res_bad.is_err(), "Expected checksum mismatch error");

    server.stop();
}

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use rustget_core::{
    db::Database,
    downloader::DownloadOptions,
    queue::QueueManager,
};

const HOST_NAME: &str = "com.cosmodownloader.downloader";
const LEGACY_HOST_NAME: &str = "com.rustget.downloader";
const LOCAL_BRIDGE_URL: &str = "http://127.0.0.1:42424/download";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum HostAction {
    Ping,
    Download,
    Status,
}

#[derive(Debug, Deserialize)]
struct ExtensionMessage {
    action: Option<HostAction>,
    url: Option<String>,
    filename: Option<String>,
    cookies: Option<String>,
    #[serde(alias = "referrer")]
    referer: Option<String>,
    user_agent: Option<String>,
    num_segments: Option<usize>,
}

#[derive(Debug, Serialize)]
struct HostResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Route tracing / logs to stderr ONLY. stdout is strictly for binary Native Messaging frames.
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--register" | "register" | "--install" => {
                let manifest_path = if args.len() > 2 {
                    PathBuf::from(&args[2])
                } else {
                    let mut dir = std::env::current_exe()?;
                    dir.pop();
                    dir.push(format!("{}.chrome.json", HOST_NAME));
                    dir
                };
                register_manifests(&manifest_path)?;
                return Ok(());
            }
            "--unregister" | "unregister" => {
                unregister_manifests()?;
                return Ok(());
            }
            "--help" | "-h" => {
                eprintln!("CosmoDownloader Native Messaging Host");
                eprintln!("Usage:");
                eprintln!("  cosmodownloader-host                   Run as Native Messaging host (stdin/stdout)");
                eprintln!("  cosmodownloader-host --register [path] Register host in Chrome, Edge, and Firefox registries");
                eprintln!("  cosmodownloader-host --unregister      Remove registry entries");
                return Ok(());
            }
            _ => {
                // If launched with an origin argument from browser
            }
        }
    }

    info!("CosmoDownloader native messaging host started");

    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    loop {
        // Read 4-byte message length (little-endian)
        let mut len_bytes = [0u8; 4];
        if let Err(e) = stdin.read_exact(&mut len_bytes) {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                // Extension closed the pipe
                break;
            }
            error!("Failed to read message length: {}", e);
            break;
        }

        let len = u32::from_ne_bytes(len_bytes) as usize;
        if len == 0 || len > 1024 * 1024 {
            error!("Invalid message length: {} bytes", len);
            break;
        }

        let mut body = vec![0u8; len];
        if let Err(e) = stdin.read_exact(&mut body) {
            error!("Failed to read message body: {}", e);
            break;
        }

        let response = match serde_json::from_slice::<ExtensionMessage>(&body) {
            Ok(msg) => handle_message(msg).await,
            Err(e) => HostResponse {
                success: false,
                message: None,
                task_id: None,
                error: Some(format!("Invalid JSON request: {}", e)),
            },
        };

        let resp_json = match serde_json::to_vec(&response) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to serialize response: {}", e);
                continue;
            }
        };

        let resp_len = (resp_json.len() as u32).to_ne_bytes();
        if stdout.write_all(&resp_len).is_err() || stdout.write_all(&resp_json).is_err() || stdout.flush().is_err() {
            error!("Failed to write response to stdout");
            break;
        }
    }

    Ok(())
}

async fn handle_message(msg: ExtensionMessage) -> HostResponse {
    match msg.action {
        Some(HostAction::Ping) => HostResponse {
            success: true,
            message: Some("pong: Cosmo Downloader Host v1.10".to_string()),
            task_id: None,
            error: None,
        },
        Some(HostAction::Status) => {
            let running = check_bridge_health().await;
            HostResponse {
                success: true,
                message: Some(if running { "CosmoDownloader GUI is active" } else { "CosmoDownloader GUI is offline (using queue DB)" }.to_string()),
                task_id: None,
                error: None,
            }
        }
        _ => {
            let url = match msg.url {
                Some(u) if !u.trim().is_empty() => u.trim().to_string(),
                _ => {
                    return HostResponse {
                        success: false,
                        message: None,
                        task_id: None,
                        error: Some("Missing 'url' parameter in download request".to_string()),
                    };
                }
            };

            // First attempt: HTTP POST to local bridge server (if CosmoDownloader GUI or CLI queue is running)
            if let Ok(task_id) = forward_to_bridge_server(&url, &msg.filename, &msg.cookies, &msg.referer, &msg.user_agent, msg.num_segments).await {
                return HostResponse {
                    success: true,
                    message: Some("Download sent to CosmoDownloader bridge server".to_string()),
                    task_id: Some(task_id),
                    error: None,
                };
            }

            // If GUI/bridge is offline, launch on-demand popup
            if let Ok(mut exe) = std::env::current_exe() {
                exe.pop();
                exe.push("CosmoDownloader.exe");
                if exe.exists() {
                    let temp_dir = std::env::temp_dir();
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0);
                    let temp_file = temp_dir.join(format!("cosmo_dl_req_{}_{}.json", std::process::id(), timestamp));
                    let payload = serde_json::json!({
                        "url": url,
                        "filename": msg.filename,
                        "cookies": msg.cookies,
                        "referer": msg.referer,
                        "user_agent": msg.user_agent,
                        "num_segments": msg.num_segments.unwrap_or(8),
                    });
                    if std::fs::write(&temp_file, payload.to_string()).is_ok() {
                        let mut popup_cmd = Command::new(&exe);
                        popup_cmd.arg("--popup-file").arg(&temp_file);
                        let _ = popup_cmd.spawn();

                        return HostResponse {
                            success: true,
                            message: Some("CosmoDownloader on-demand popup opened".to_string()),
                            task_id: Some("popup".to_string()),
                            error: None,
                        };
                    }
                }
            }

            // Fallback: Directly insert into SQLite database via QueueManager
            match queue_directly_in_db(&url, msg.filename.as_deref(), msg.cookies, msg.referer, msg.user_agent, msg.num_segments.unwrap_or(8)).await {
                Ok(task_id) => HostResponse {
                    success: true,
                    message: Some("Download saved directly to CosmoDownloader database queue".to_string()),
                    task_id: Some(task_id),
                    error: None,
                },
                Err(e) => HostResponse {
                    success: false,
                    message: None,
                    task_id: None,
                    error: Some(format!("Failed to queue download: {}", e)),
                },
            }
        }
    }
}

async fn check_bridge_health() -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build();
    if let Ok(c) = client {
        if let Ok(resp) = c.get("http://127.0.0.1:42424/health").send().await {
            return resp.status().is_success();
        }
    }
    false
}

async fn forward_to_bridge_server(
    url: &str,
    filename: &Option<String>,
    cookies: &Option<String>,
    referer: &Option<String>,
    user_agent: &Option<String>,
    num_segments: Option<usize>,
) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()?;

    let payload = serde_json::json!({
        "url": url,
        "filename": filename,
        "cookies": cookies,
        "referer": referer,
        "user_agent": user_agent,
        "num_segments": num_segments.unwrap_or(8),
    });

    let resp = client.post(LOCAL_BRIDGE_URL)
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()
        .await?;

    if resp.status().is_success() {
        let body_str = resp.text().await?;
        let v: serde_json::Value = serde_json::from_str(&body_str)?;
        if let Some(task_id) = v.get("task_id").and_then(|t| t.as_str()) {
            return Ok(task_id.to_string());
        }
        return Ok("queued".to_string());
    }

    anyhow::bail!("Bridge server returned status: {}", resp.status())
}

async fn queue_directly_in_db(
    url: &str,
    custom_filename: Option<&str>,
    cookies: Option<String>,
    referer: Option<String>,
    user_agent: Option<String>,
    num_segments: usize,
) -> anyhow::Result<String> {
    let db = Database::open_default()?;
    let (tx, _rx) = tokio::sync::mpsc::channel(16);
    let queue = QueueManager::new(db, 3, 0, tx);

    let output_path = custom_filename.map(PathBuf::from);
    let options = DownloadOptions {
        url: url.to_string(),
        output_path,
        num_segments,
        cookie: cookies,
        referer,
        user_agent,
        ..Default::default()
    };

    let id = queue.add_download(options, None, 0).await?;
    info!("Directly inserted download {} to SQLite database queue", id);
    Ok(id)
}

fn register_manifests(chrome_manifest: &Path) -> anyhow::Result<()> {
    let chrome_path_str = chrome_manifest.to_string_lossy();

    for name in [HOST_NAME, LEGACY_HOST_NAME] {
        // Register Chrome
        let chrome_key = format!(r"HKCU\Software\Google\Chrome\NativeMessagingHosts\{}", name);
        let _ = Command::new("reg")
            .args(["add", &chrome_key, "/ve", "/t", "REG_SZ", "/d", &chrome_path_str, "/f"])
            .status();

        // Register Edge
        let edge_key = format!(r"HKCU\Software\Microsoft\Edge\NativeMessagingHosts\{}", name);
        let mut edge_path = chrome_manifest.to_path_buf();
        edge_path.set_file_name(format!("{}.edge.json", name));
        let edge_path_str = if edge_path.exists() {
            edge_path.to_string_lossy().to_string()
        } else {
            chrome_path_str.to_string()
        };
        let _ = Command::new("reg")
            .args(["add", &edge_key, "/ve", "/t", "REG_SZ", "/d", &edge_path_str, "/f"])
            .status();

        // Register Mozilla Firefox
        let firefox_key = format!(r"HKCU\Software\Mozilla\NativeMessagingHosts\{}", name);
        let mut firefox_path = chrome_manifest.to_path_buf();
        firefox_path.set_file_name(format!("{}.firefox.json", name));
        let firefox_path_str = if firefox_path.exists() {
            firefox_path.to_string_lossy().to_string()
        } else {
            chrome_path_str.to_string()
        };
        let _ = Command::new("reg")
            .args(["add", &firefox_key, "/ve", "/t", "REG_SZ", "/d", &firefox_path_str, "/f"])
            .status();
    }

    println!("Registered CosmoDownloader native messaging hosts in Windows Registry.");
    Ok(())
}

fn unregister_manifests() -> anyhow::Result<()> {
    for name in [HOST_NAME, LEGACY_HOST_NAME] {
        let chrome_key = format!(r"HKCU\Software\Google\Chrome\NativeMessagingHosts\{}", name);
        let _ = Command::new("reg").args(["delete", &chrome_key, "/f"]).status();

        let edge_key = format!(r"HKCU\Software\Microsoft\Edge\NativeMessagingHosts\{}", name);
        let _ = Command::new("reg").args(["delete", &edge_key, "/f"]).status();

        let firefox_key = format!(r"HKCU\Software\Mozilla\NativeMessagingHosts\{}", name);
        let _ = Command::new("reg").args(["delete", &firefox_key, "/f"]).status();
    }

    println!("Unregistered Native Messaging hosts from Windows Registry.");
    Ok(())
}

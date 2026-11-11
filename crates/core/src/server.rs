use std::net::SocketAddr;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::downloader::DownloadOptions;
use crate::queue::QueueManager;

/// Request payload from browser extension or external tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionDownloadRequest {
    pub url: String,
    pub filename: Option<String>,
    pub cookies: Option<String>,
    #[serde(alias = "referrer")]
    pub referer: Option<String>,
    pub user_agent: Option<String>,
    pub num_segments: Option<usize>,
}

/// Spawns an on-demand CosmoDownloader popup window with the given request payload.
pub fn spawn_on_demand_popup(dl_req: &ExtensionDownloadRequest) -> bool {
    if let Ok(mut exe) = std::env::current_exe() {
        if exe.file_name().map(|n| n == "cosmodownloader-host.exe" || n == "cosmodownloader-cli.exe").unwrap_or(false) {
            exe.pop();
            exe.push("CosmoDownloader.exe");
        }
        if exe.exists() {
            let temp_dir = std::env::temp_dir();
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let temp_file = temp_dir.join(format!("cosmo_popup_{}_{}.json", std::process::id(), timestamp));
            if let Ok(json) = serde_json::to_string(dl_req) {
                if std::fs::write(&temp_file, json).is_ok() {
                    let mut cmd = std::process::Command::new(&exe);
                    cmd.arg("--popup-file").arg(&temp_file);
                    if let Ok(_) = cmd.spawn() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Local HTTP server for browser extension integration.
pub struct BridgeServer {
    queue: Arc<QueueManager>,
    port: u16,
    prompt_tx: Option<tokio::sync::mpsc::Sender<ExtensionDownloadRequest>>,
    show_tx: Option<tokio::sync::mpsc::Sender<()>>,
    wake_cb: Option<Arc<dyn Fn() + Send + Sync>>,
    download_wake_cb: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl BridgeServer {
    pub fn new(queue: Arc<QueueManager>, port: u16) -> Self {
        Self {
            queue,
            port,
            prompt_tx: None,
            show_tx: None,
            wake_cb: None,
            download_wake_cb: None,
        }
    }

    pub fn with_prompt(
        queue: Arc<QueueManager>,
        port: u16,
        prompt_tx: tokio::sync::mpsc::Sender<ExtensionDownloadRequest>,
    ) -> Self {
        Self {
            queue,
            port,
            prompt_tx: Some(prompt_tx),
            show_tx: None,
            wake_cb: None,
            download_wake_cb: None,
        }
    }

    pub fn with_notifier(mut self, tx: tokio::sync::mpsc::Sender<ExtensionDownloadRequest>) -> Self {
        self.prompt_tx = Some(tx);
        self
    }

    pub fn with_show_notifier(mut self, tx: tokio::sync::mpsc::Sender<()>) -> Self {
        self.show_tx = Some(tx);
        self
    }

    pub fn with_wake_callback<F>(mut self, cb: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.wake_cb = Some(Arc::new(cb));
        self
    }

    pub fn with_download_wake_callback<F>(mut self, cb: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.download_wake_cb = Some(Arc::new(cb));
        self
    }

    /// Run the server until cancellation token is triggered.
    pub async fn run(self, cancel: CancellationToken) {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => {
                info!("Cosmo Downloader local browser bridge listening on http://{}", addr);
                l
            }
            Err(e) => {
                warn!("Failed to bind browser bridge on port {}: {}", self.port, e);
                return;
            }
        };

        let prompt_tx = self.prompt_tx.map(Arc::new);
        let show_tx = self.show_tx.map(Arc::new);
        let wake_cb = self.wake_cb.clone();
        let download_wake_cb = self.download_wake_cb.clone();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                accept_res = listener.accept() => {
                    let (mut socket, _) = match accept_res {
                        Ok(res) => res,
                        Err(_) => continue,
                    };

                    let queue = Arc::clone(&self.queue);
                    let prompt_tx = prompt_tx.clone();
                    let show_tx = show_tx.clone();
                    let wake_cb = wake_cb.clone();
                    let download_wake_cb = download_wake_cb.clone();

                    tokio::spawn(async move {
                        let mut data = Vec::new();
                        let mut buf = [0u8; 4096];
                        let mut content_length: Option<usize> = None;

                        loop {
                            let n = match socket.read(&mut buf).await {
                                Ok(n) if n > 0 => n,
                                _ => break,
                            };
                            data.extend_from_slice(&buf[..n]);

                            if content_length.is_none() {
                                let req_str = String::from_utf8_lossy(&data);
                                if req_str.contains("Expect: 100-continue") || req_str.contains("expect: 100-continue") {
                                    let _ = socket.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").await;
                                }

                                if let Some(pos) = req_str.find("\r\n\r\n") {
                                    for line in req_str[..pos].lines() {
                                        let lower = line.to_ascii_lowercase();
                                        if lower.starts_with("content-length:") {
                                            if let Some(val) = line.split(':').nth(1) {
                                                content_length = val.trim().parse().ok();
                                            }
                                        }
                                    }
                                    if content_length.is_none() {
                                        break;
                                    }
                                }
                            }

                            if let Some(cl) = content_length {
                                let req_str = String::from_utf8_lossy(&data);
                                if let Some(pos) = req_str.find("\r\n\r\n") {
                                    let header_len = pos + 4;
                                    if data.len() >= header_len + cl {
                                        break;
                                    }
                                }
                            }
                        }

                        if data.is_empty() {
                            return;
                        }

                        let req = String::from_utf8_lossy(&data);

                        // Handle CORS Preflight OPTIONS
                        if req.starts_with("OPTIONS ") {
                            let resp = "HTTP/1.1 204 No Content\r\n\
                                        Access-Control-Allow-Origin: *\r\n\
                                        Access-Control-Allow-Methods: POST, GET, OPTIONS\r\n\
                                        Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
                                        Access-Control-Allow-Private-Network: true\r\n\
                                        Access-Control-Max-Age: 86400\r\n\
                                        Connection: close\r\n\r\n";
                            let _ = socket.write_all(resp.as_bytes()).await;
                            return;
                        }

                        // Handle POST /download
                        if req.starts_with("POST /download") || req.starts_with("POST /api/download") {
                            // Extract JSON body after \r\n\r\n
                            let body = req.split("\r\n\r\n").nth(1).unwrap_or_default();

                            match serde_json::from_str::<ExtensionDownloadRequest>(body) {
                                Ok(dl_req) => {
                                    // Trigger GUI download wake callback if registered
                                    if let Some(ref cb) = download_wake_cb {
                                        cb();
                                    }

                                    // If GUI is listening with a prompt channel, send it to show the IDM popup dialog!
                                    if let Some(ref tx) = prompt_tx {
                                        let _ = tx.send(dl_req).await;
                                        let json_resp = "{\"success\":true,\"message\":\"Prompted in CosmoDownloader dialog\"}";
                                        let resp = format!(
                                            "HTTP/1.1 200 OK\r\n\
                                             Access-Control-Allow-Origin: *\r\n\
                                             Access-Control-Allow-Private-Network: true\r\n\
                                             Content-Type: application/json\r\n\
                                             Content-Length: {}\r\n\
                                             Connection: close\r\n\r\n{}",
                                            json_resp.len(),
                                            json_resp
                                        );
                                        let _ = socket.write_all(resp.as_bytes()).await;
                                        let _ = socket.flush().await;
                                        return;
                                    }

                                    // Fallback: direct queue insertion
                                    let options = DownloadOptions {
                                        url: dl_req.url.clone(),
                                        num_segments: dl_req.num_segments.unwrap_or(8),
                                        cookie: dl_req.cookies,
                                        referer: dl_req.referer,
                                        user_agent: dl_req.user_agent,
                                        ..Default::default()
                                    };

                                    match queue.add_download(options, None, 0).await {
                                        Ok(id) => {
                                            let json_resp = format!(
                                                "{{\"success\":true,\"message\":\"Queued\",\"task_id\":\"{}\"}}",
                                                id
                                            );
                                            let resp = format!(
                                                "HTTP/1.1 200 OK\r\n\
                                                 Access-Control-Allow-Origin: *\r\n\
                                                 Content-Type: application/json\r\n\
                                                 Content-Length: {}\r\n\
                                                 Connection: close\r\n\r\n{}",
                                                json_resp.len(),
                                                json_resp
                                            );
                                            let _ = socket.write_all(resp.as_bytes()).await;
                                            let _ = socket.flush().await;
                                        }
                                        Err(e) => {
                                            let err_resp = format!("{{\"success\":false,\"error\":\"{}\"}}", e);
                                            let resp = format!(
                                                "HTTP/1.1 500 Internal Error\r\n\
                                                 Access-Control-Allow-Origin: *\r\n\
                                                 Content-Type: application/json\r\n\
                                                 Content-Length: {}\r\n\
                                                 Connection: close\r\n\r\n{}",
                                                err_resp.len(),
                                                err_resp
                                            );
                                            let _ = socket.write_all(resp.as_bytes()).await;
                                            let _ = socket.flush().await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    let err_resp = format!("{{\"success\":false,\"error\":\"Invalid JSON: {}\"}}", e);
                                    let resp = format!(
                                        "HTTP/1.1 400 Bad Request\r\n\
                                         Access-Control-Allow-Origin: *\r\n\
                                         Content-Type: application/json\r\n\
                                         Content-Length: {}\r\n\
                                         Connection: close\r\n\r\n{}",
                                        err_resp.len(),
                                        err_resp
                                    );
                                    let _ = socket.write_all(resp.as_bytes()).await;
                                }
                            }
                        } else if req.starts_with("GET /health") {
                            let json_resp = "{\"status\":\"ok\"}";
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 Access-Control-Allow-Origin: *\r\n\
                                 Content-Type: application/json\r\n\
                                 Content-Length: {}\r\n\
                                 Connection: close\r\n\r\n{}",
                                json_resp.len(),
                                json_resp
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                            let _ = socket.flush().await;
                        } else if req.starts_with("POST /show") || req.starts_with("GET /show") {
                            if let Some(ref tx) = show_tx {
                                let _ = tx.send(()).await;
                            }
                            if let Some(ref cb) = wake_cb {
                                cb();
                            }
                            let json_resp = "{\"status\":\"ok\"}";
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 Access-Control-Allow-Origin: *\r\n\
                                 Content-Type: application/json\r\n\
                                 Content-Length: {}\r\n\
                                 Connection: close\r\n\r\n{}",
                                json_resp.len(),
                                json_resp
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                            let _ = socket.flush().await;
                        } else {
                            let resp = "HTTP/1.1 404 Not Found\r\n\
                                        Access-Control-Allow-Origin: *\r\n\
                                        Content-Length: 0\r\n\
                                        Connection: close\r\n\r\n";
                            let _ = socket.write_all(resp.as_bytes()).await;
                            let _ = socket.flush().await;
                        }
                    });
                }
            }
        }
    }
}

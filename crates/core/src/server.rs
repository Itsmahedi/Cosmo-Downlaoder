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

/// Retrieves or generates a secure local bridge token for authorized IPC commands (such as /show).
pub fn get_or_create_bridge_token() -> String {
    let token_path = std::env::temp_dir().join("cosmo_bridge_token.txt");
    if let Ok(existing) = std::fs::read_to_string(&token_path) {
        let trimmed = existing.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    let new_token = uuid::Uuid::new_v4().to_string();
    let _ = std::fs::write(&token_path, &new_token);
    new_token
}

/// Validates Origin and Sec-Fetch-Site headers.
/// Only browser extensions (`chrome-extension://`, `moz-extension://`, `extension://`)
/// or native requests with no `Origin` (and non-`cross-site` `Sec-Fetch-Site`) are allowed.
fn validate_request_origin(headers_part: &str) -> Result<Option<String>, &'static str> {
    let mut origin_val: Option<String> = None;
    for line in headers_part.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("sec-fetch-site:") {
            if let Some(pos) = line.find(':') {
                let sfs = line[pos + 1..].trim().to_ascii_lowercase();
                if sfs == "cross-site" {
                    return Err("Forbidden: cross-site browser requests are rejected");
                }
            }
        }
        if lower.starts_with("origin:") {
            if let Some(pos) = line.find(':') {
                let origin = line[pos + 1..].trim();
                if origin == "null" {
                    return Err("Forbidden: null origin is rejected");
                }
                if origin.is_empty() {
                    continue;
                }
                if origin.starts_with("chrome-extension://")
                    || origin.starts_with("moz-extension://")
                    || origin.starts_with("extension://")
                {
                    origin_val = Some(origin.to_string());
                } else {
                    return Err("Forbidden: request origin is not an authorized extension");
                }
            }
        }
    }
    Ok(origin_val)
}

/// Verifies whether the incoming request contains the valid authorization token for /show.
fn is_authorized_show_request(headers_part: &str, expected_token: &str) -> bool {
    if expected_token.is_empty() {
        return false;
    }
    for line in headers_part.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("x-cosmo-token:") {
            if let Some(pos) = line.find(':') {
                if line[pos + 1..].trim() == expected_token.trim() {
                    return true;
                }
            }
        }
        if lower.starts_with("authorization:") {
            if let Some(pos) = line.find(':') {
                let val = line[pos + 1..].trim();
                if val.starts_with("Bearer ") && val[7..].trim() == expected_token.trim() {
                    return true;
                }
            }
        }
    }
    // Check URL query param token=...
    if let Some(first_line) = headers_part.lines().next() {
        if let Some(pos) = first_line.find("token=") {
            let rest = &first_line[pos + 6..];
            let token_val = rest.split_whitespace().next().unwrap_or("").split('&').next().unwrap_or("");
            if token_val == expected_token.trim() {
                return true;
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
    quit_tx: Option<tokio::sync::mpsc::Sender<()>>,
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
            quit_tx: None,
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
            quit_tx: None,
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

    pub fn with_quit_notifier(mut self, tx: tokio::sync::mpsc::Sender<()>) -> Self {
        self.quit_tx = Some(tx);
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
        let quit_tx = self.quit_tx.map(Arc::new);
        let wake_cb = self.wake_cb.clone();
        let download_wake_cb = self.download_wake_cb.clone();
        let bridge_token = Arc::new(get_or_create_bridge_token());

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
                    let quit_tx = quit_tx.clone();
                    let wake_cb = wake_cb.clone();
                    let download_wake_cb = download_wake_cb.clone();
                    let bridge_token = bridge_token.clone();

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

                        // Validate browser Origin header to block external website CSRF/cross-origin probes
                        let allowed_origin = match validate_request_origin(&req) {
                            Ok(orig) => orig,
                            Err(_) => {
                                let err_resp = "{\"error\":\"forbidden_origin\"}";
                                let resp = format!(
                                    "HTTP/1.1 403 Forbidden\r\n\
                                     Content-Type: application/json\r\n\
                                     Content-Length: {}\r\n\
                                     Connection: close\r\n\r\n{}",
                                    err_resp.len(),
                                    err_resp
                                );
                                let _ = socket.write_all(resp.as_bytes()).await;
                                let _ = socket.flush().await;
                                return;
                            }
                        };

                        let cors_header = match allowed_origin {
                            Some(ref orig) => format!("Access-Control-Allow-Origin: {}\r\n", orig),
                            None => String::new(),
                        };

                        // Handle CORS Preflight OPTIONS
                        if req.starts_with("OPTIONS ") {
                            let resp = format!(
                                "HTTP/1.1 204 No Content\r\n\
                                 {}Access-Control-Allow-Methods: POST, GET, OPTIONS\r\n\
                                 Access-Control-Allow-Headers: Content-Type, Authorization, X-Cosmo-Token\r\n\
                                 Access-Control-Allow-Private-Network: true\r\n\
                                 Access-Control-Max-Age: 86400\r\n\
                                 Connection: close\r\n\r\n",
                                cors_header
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                            return;
                        }

                        // Handle POST /download
                        if req.starts_with("POST /download") || req.starts_with("POST /api/download") {
                            let body = req.split("\r\n\r\n").nth(1).unwrap_or_default();

                            match serde_json::from_str::<ExtensionDownloadRequest>(body) {
                                Ok(mut dl_req) => {
                                    if let Err(e) = crate::probe::validate_http_url(&dl_req.url) {
                                        let err_json = serde_json::json!({
                                            "success": false,
                                            "error": e.to_string(),
                                        }).to_string();
                                        let resp = format!(
                                            "HTTP/1.1 400 Bad Request\r\n\
                                             {}Content-Type: application/json\r\n\
                                             Content-Length: {}\r\n\
                                             Connection: close\r\n\r\n{}",
                                            cors_header,
                                            err_json.len(),
                                            err_json
                                        );
                                        let _ = socket.write_all(resp.as_bytes()).await;
                                        let _ = socket.flush().await;
                                        return;
                                    }

                                    dl_req.filename = dl_req
                                        .filename
                                        .as_deref()
                                        .map(str::trim)
                                        .filter(|s| !s.is_empty())
                                        .map(crate::probe::sanitize_filename);

                                    if let Some(ref cb) = download_wake_cb {
                                        cb();
                                    }

                                    if let Some(ref tx) = prompt_tx {
                                        let _ = tx.send(dl_req).await;
                                        let json_resp = "{\"success\":true,\"message\":\"Prompted in CosmoDownloader dialog\"}";
                                        let resp = format!(
                                            "HTTP/1.1 200 OK\r\n\
                                             {}Access-Control-Allow-Private-Network: true\r\n\
                                             Content-Type: application/json\r\n\
                                             Content-Length: {}\r\n\
                                             Connection: close\r\n\r\n{}",
                                            cors_header,
                                            json_resp.len(),
                                            json_resp
                                        );
                                        let _ = socket.write_all(resp.as_bytes()).await;
                                        let _ = socket.flush().await;
                                        return;
                                    }

                                    let output_path = dl_req.filename.as_ref().map(|fname| {
                                        let cat = crate::category::Category::detect_from_filename(fname);
                                        cat.default_dir().join(fname)
                                    });

                                    let options = DownloadOptions {
                                        url: dl_req.url.clone(),
                                        output_path,
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
                                                 {}Content-Type: application/json\r\n\
                                                 Content-Length: {}\r\n\
                                                 Connection: close\r\n\r\n{}",
                                                cors_header,
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
                                                 {}Content-Type: application/json\r\n\
                                                 Content-Length: {}\r\n\
                                                 Connection: close\r\n\r\n{}",
                                                cors_header,
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
                                         {}Content-Type: application/json\r\n\
                                         Content-Length: {}\r\n\
                                         Connection: close\r\n\r\n{}",
                                        cors_header,
                                        err_resp.len(),
                                        err_resp
                                    );
                                    let _ = socket.write_all(resp.as_bytes()).await;
                                }
                            }
                        } else if req.starts_with("GET /health") {
                            // Health endpoint: returns only {"status":"ok"} without leaking any sensitive data
                            let json_resp = "{\"status\":\"ok\"}";
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 {}Content-Type: application/json\r\n\
                                 Content-Length: {}\r\n\
                                 Connection: close\r\n\r\n{}",
                                cors_header,
                                json_resp.len(),
                                json_resp
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                            let _ = socket.flush().await;
                        } else if req.starts_with("POST /show") || req.starts_with("GET /show") {
                            // /show requires valid authorization token
                            if !is_authorized_show_request(&req, &bridge_token) {
                                let err_resp = "{\"error\":\"unauthorized\"}";
                                let resp = format!(
                                    "HTTP/1.1 401 Unauthorized\r\n\
                                     {}Content-Type: application/json\r\n\
                                     Content-Length: {}\r\n\
                                     Connection: close\r\n\r\n{}",
                                    cors_header,
                                    err_resp.len(),
                                    err_resp
                                );
                                let _ = socket.write_all(resp.as_bytes()).await;
                                let _ = socket.flush().await;
                                return;
                            }

                            if let Some(ref tx) = show_tx {
                                let _ = tx.send(()).await;
                            }
                            if let Some(ref cb) = wake_cb {
                                cb();
                            }
                            let json_resp = "{\"status\":\"ok\"}";
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 {}Content-Type: application/json\r\n\
                                 Content-Length: {}\r\n\
                                 Connection: close\r\n\r\n{}",
                                cors_header,
                                json_resp.len(),
                                json_resp
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                            let _ = socket.flush().await;
                        } else if req.starts_with("POST /quit") {
                            if !is_authorized_show_request(&req, &bridge_token) {
                                let err_resp = "{\"error\":\"unauthorized\"}";
                                let resp = format!(
                                    "HTTP/1.1 401 Unauthorized\r\n\
                                     {}Content-Type: application/json\r\n\
                                     Content-Length: {}\r\n\
                                     Connection: close\r\n\r\n{}",
                                    cors_header,
                                    err_resp.len(),
                                    err_resp
                                );
                                let _ = socket.write_all(resp.as_bytes()).await;
                                let _ = socket.flush().await;
                                return;
                            }

                            if let Some(ref tx) = quit_tx {
                                let _ = tx.send(()).await;
                            }
                            let json_resp = "{\"status\":\"quitting\"}";
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 {}Content-Type: application/json\r\n\
                                 Content-Length: {}\r\n\
                                 Connection: close\r\n\r\n{}",
                                cors_header,
                                json_resp.len(),
                                json_resp
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                            let _ = socket.flush().await;
                        } else {
                            let resp = format!(
                                "HTTP/1.1 404 Not Found\r\n\
                                 {}Content-Length: 0\r\n\
                                 Connection: close\r\n\r\n",
                                cors_header
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                            let _ = socket.flush().await;
                        }
                    });
                }
            }
        }
    }
}

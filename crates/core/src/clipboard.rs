use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use arboard::Clipboard;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};
use url::Url;

/// Monitors the system clipboard for downloadable URLs.
pub struct ClipboardMonitor {
    cancel_token: CancellationToken,
    enabled: Arc<AtomicBool>,
    wake_cb: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl ClipboardMonitor {
    pub fn new(cancel_token: CancellationToken) -> Self {
        Self {
            cancel_token,
            enabled: Arc::new(AtomicBool::new(false)),
            wake_cb: None,
        }
    }

    pub fn with_enabled(self, enabled: bool) -> Self {
        self.enabled.store(enabled, Ordering::Relaxed);
        self
    }

    pub fn enabled_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.enabled)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn with_wake_callback<F>(mut self, cb: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.wake_cb = Some(Arc::new(cb));
        self
    }

    /// Spawns a background task that polls the clipboard while enabled and emits valid new download URLs.
    pub fn start(&self, url_tx: mpsc::Sender<String>) {
        let cancel = self.cancel_token.clone();
        let enabled = Arc::clone(&self.enabled);
        let wake_cb = self.wake_cb.clone();

        tokio::task::spawn_blocking(move || {
            let mut clipboard: Option<Clipboard> = None;
            let mut was_enabled = false;
            let mut last_seen: Option<String> = None;
            info!("Clipboard monitor worker started");

            while !cancel.is_cancelled() {
                if !enabled.load(Ordering::Relaxed) {
                    was_enabled = false;
                    std::thread::sleep(Duration::from_millis(1000));
                    continue;
                }

                if clipboard.is_none() {
                    clipboard = Clipboard::new().ok();
                }

                if let Some(ref mut cb) = clipboard {
                    if let Ok(text) = cb.get_text() {
                        let trimmed = text.trim();
                        if !was_enabled {
                            // Snapshot current clipboard when monitor turns on so existing URLs aren't popped up unexpectedly.
                            last_seen = if trimmed.is_empty() { None } else { Some(trimmed.to_string()) };
                            was_enabled = true;
                        } else if (trimmed.starts_with("http://") || trimmed.starts_with("https://"))
                            && Some(trimmed.to_string()) != last_seen
                        {
                            if let Ok(parsed_url) = Url::parse(trimmed) {
                                if parsed_url.host().is_some() && is_likely_download_url(&parsed_url) {
                                    info!("Detected download URL in clipboard: {}", crate::probe::redact_url_for_log(trimmed));
                                    last_seen = Some(trimmed.to_string());
                                    let _ = url_tx.blocking_send(trimmed.to_string());
                                    if let Some(ref wcb) = wake_cb {
                                        wcb();
                                    }
                                }
                            }
                        }
                    }
                }

                std::thread::sleep(Duration::from_millis(750));
            }

            debug!("Clipboard monitor stopped");
        });
    }
}

/// Heuristic check whether a URL is a downloadable file or stream.
fn is_likely_download_url(url: &Url) -> bool {
    let path = url.path().to_ascii_lowercase();

    // Check file extension
    let has_file_ext = [
        ".zip", ".rar", ".7z", ".tar", ".gz", ".iso", ".exe", ".msi",
        ".mp4", ".mkv", ".mp3", ".flac", ".wav", ".pdf", ".docx", ".xlsx",
        ".bin", ".dmg", ".apk", ".deb", ".rpm", ".appimage"
    ].iter().any(|ext| path.ends_with(ext));

    if has_file_ext {
        return true;
    }

    // Check query params or path segments containing common download keywords
    let query = url.query().unwrap_or("").to_ascii_lowercase();
    let has_kw = path.contains("/download") || query.contains("download=") || query.contains("dl=1");

    has_kw || path.split('/').next_back().map(|s| s.contains('.')).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_likely_download_url() {
        assert!(is_likely_download_url(&Url::parse("https://example.com/files/app.zip").unwrap()));
        assert!(is_likely_download_url(&Url::parse("https://example.com/download?id=123&dl=1").unwrap()));
        assert!(is_likely_download_url(&Url::parse("https://example.com/movie.mp4").unwrap()));
        assert!(!is_likely_download_url(&Url::parse("https://google.com").unwrap()));
        assert!(!is_likely_download_url(&Url::parse("https://example.com/article/").unwrap()));
    }
}

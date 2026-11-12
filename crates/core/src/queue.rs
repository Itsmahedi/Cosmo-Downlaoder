use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use reqwest::header::HeaderMap;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::category::Category;
use crate::db::{Database, DownloadRecord};
use crate::downloader::{DownloadOptions, DownloadProgress, Downloader};
use crate::error::{DownloadError, Result};
use crate::probe::{extract_filename, sanitize_filename, validate_http_url};
use crate::rate_limiter::TokenBucketRateLimiter;

/// Events broadcast by the queue manager.
#[derive(Debug, Clone)]
pub enum QueueEvent {
    TaskAdded(String),
    StatusChanged { id: String, status: String },
    Progress { id: String, progress: DownloadProgress },
    TaskCompleted(String),
    TaskFailed { id: String, error: String },
}

/// Download Queue Manager coordinating concurrency, rate-limits, and retries.
pub struct QueueManager {
    db: Database,
    active_tasks: Arc<Mutex<HashMap<String, CancellationToken>>>,
    global_limiter: TokenBucketRateLimiter,
    max_concurrent: Arc<Mutex<usize>>,
    proxy_url: Arc<Mutex<Option<String>>>,
    shutting_down: Arc<AtomicBool>,
    event_tx: mpsc::Sender<QueueEvent>,
}

impl QueueManager {
    pub fn new(
        db: Database,
        max_concurrent: usize,
        global_speed_bps: u64,
        event_tx: mpsc::Sender<QueueEvent>,
    ) -> Self {
        Self {
            db,
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            global_limiter: TokenBucketRateLimiter::new(global_speed_bps),
            max_concurrent: Arc::new(Mutex::new(max_concurrent.clamp(1, 10))),
            proxy_url: Arc::new(Mutex::new(None)),
            shutting_down: Arc::new(AtomicBool::new(false)),
            event_tx,
        }
    }

    /// Access the SQLite database.
    pub fn db(&self) -> &Database {
        &self.db
    }

    /// Update maximum concurrent downloads.
    pub async fn set_max_concurrent(&self, max: usize) {
        let mut m = self.max_concurrent.lock().await;
        *m = max.clamp(1, 10);
    }

    /// Update global download speed limit in bytes/sec (0 = unlimited).
    pub async fn set_global_speed_limit(&self, bps: u64) {
        self.global_limiter.set_rate(bps).await;
    }

    /// Update the proxy URL used for future downloads (`None` or empty clears it).
    pub async fn set_proxy(&self, proxy: Option<String>) {
        let mut guard = self.proxy_url.lock().await;
        *guard = proxy.and_then(|s| {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        });
    }

    /// Recover any tasks left in `'downloading'` state after an unexpected crash or upgrade.
    pub async fn recover_interrupted_downloads(&self) -> Result<usize> {
        let count = self.db.recover_interrupted_downloads().await?;
        if count > 0 {
            info!("Recovered {} interrupted download(s) back to queued state", count);
        }
        Ok(count)
    }

    /// Gracefully cancel active downloads (preserving them as `'queued'` for auto-resume on next launch)
    /// and checkpoint SQLite WAL before process exit.
    pub async fn shutdown_and_save(&self) {
        self.shutting_down.store(true, Ordering::SeqCst);
        let tokens: Vec<CancellationToken> = {
            let tasks = self.active_tasks.lock().await;
            tasks.values().cloned().collect()
        };
        for tok in tokens {
            tok.cancel();
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            if self.active_tasks.lock().await.is_empty() {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
        let _ = self.db.recover_interrupted_downloads().await;
        let _ = self.db.flush_wal().await;
    }

    /// Build a new DownloadRecord from options.
    fn build_record(
        &self,
        options: &DownloadOptions,
        category: Option<Category>,
        max_speed_bps: u64,
        status: &str,
    ) -> Result<DownloadRecord> {
        let parsed_url = validate_http_url(&options.url)?;

        let (mut filename, target_path) = match &options.output_path {
            Some(p) if !p.is_dir() && p.file_name().is_some() => {
                let fname = p.file_name().unwrap().to_string_lossy().to_string();
                let f = if !fname.trim().is_empty() {
                    sanitize_filename(&fname)
                } else {
                    extract_filename(&options.headers, &parsed_url)
                };
                let sanitized_path = p.parent().unwrap_or(Path::new(".")).join(&f);
                (f, sanitized_path)
            }
            Some(p) => {
                let f = extract_filename(&options.headers, &parsed_url);
                let full = p.join(&f);
                (f, full)
            }
            None => {
                let f = extract_filename(&options.headers, &parsed_url);
                let cat = category.clone().unwrap_or_else(|| Category::detect_from_filename(&f));
                let full = cat.default_dir().join(&f);
                (f, full)
            }
        };

        if crate::youtube::is_streaming_site(&options.url) {
            if filename == "watch" || !filename.contains('.') {
                filename = "YouTube_Video.mp4".to_string();
            }
        }

        let cat = category.unwrap_or_else(|| Category::detect_from_filename(&filename));
        let final_target_path = if target_path.file_name().map(|n| n.to_string_lossy()) == Some(filename.as_str().into()) {
            target_path
        } else {
            target_path.parent().unwrap_or(Path::new(".")).join(&filename)
        };

        let mut part_os = final_target_path.as_os_str().to_os_string();
        part_os.push(".cosmo.part");
        let part_path = PathBuf::from(part_os);

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        Ok(DownloadRecord {
            id,
            url: options.url.clone(),
            filename,
            target_path: final_target_path.to_string_lossy().to_string(),
            part_path: part_path.to_string_lossy().to_string(),
            category: cat.name().to_string(),
            total_size: None,
            downloaded_bytes: 0,
            status: status.to_string(),
            num_segments: options.num_segments,
            expected_sha256: options.expected_sha256.clone(),
            custom_headers: None,
            user_agent: options.user_agent.clone(),
            referer: options.referer.clone(),
            cookie: options.cookie.clone(),
            max_speed_bps,
            retry_count: 0,
            max_retries: 5,
            error_message: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Add a new download task to the queue.
    pub async fn add_download(
        &self,
        options: DownloadOptions,
        category: Option<Category>,
        max_speed_bps: u64,
    ) -> Result<String> {
        let record = self.build_record(&options, category, max_speed_bps, "queued")?;
        let id = record.id.clone();
        self.db.insert_download(&record).await?;
        let _ = self.event_tx.send(QueueEvent::TaskAdded(id.clone())).await;

        info!("Added download task {} [{}] to queue", id, record.filename);
        Ok(id)
    }

    /// Start a download immediately, bypassing queued state to avoid worker races.
    pub async fn start_download_immediately(
        self: &Arc<Self>,
        options: DownloadOptions,
        category: Option<Category>,
        max_speed_bps: u64,
    ) -> Result<String> {
        let record = self.build_record(&options, category, max_speed_bps, "downloading")?;
        let id = record.id.clone();
        self.db.insert_download(&record).await?;
        let task_cancel = CancellationToken::new();
        {
            let mut tasks = self.active_tasks.lock().await;
            tasks.insert(id.clone(), task_cancel.clone());
        }
        let _ = self.event_tx.send(QueueEvent::StatusChanged {
            id: id.clone(),
            status: "downloading".to_string(),
        }).await;
        let fname = record.filename.clone();
        let self_clone = Arc::clone(self);
        tokio::spawn(async move {
            self_clone.process_download(record, task_cancel).await;
        });
        info!("Started download task {} [{}] immediately", id, fname);
        Ok(id)
    }

    /// Pause an active or queued download.
    pub async fn pause(&self, id: &str) -> Result<()> {
        let mut tasks = self.active_tasks.lock().await;
        if let Some(cancel) = tasks.remove(id) {
            cancel.cancel();
        }
        self.db.update_status(id, "paused", None).await?;
        let _ = self.event_tx.send(QueueEvent::StatusChanged {
            id: id.to_string(),
            status: "paused".to_string(),
        }).await;
        info!("Paused download task {}", id);
        Ok(())
    }

    /// Resume a paused or failed download.
    pub async fn resume(&self, id: &str) -> Result<()> {
        self.db.reset_retry_count(id).await?;
        self.db.update_status(id, "queued", None).await?;
        let _ = self.event_tx.send(QueueEvent::StatusChanged {
            id: id.to_string(),
            status: "queued".to_string(),
        }).await;
        info!("Resumed download task {}", id);
        Ok(())
    }

    /// Cancel and delete a download task.
    pub async fn remove(&self, id: &str, delete_files: bool) -> Result<()> {
        let _ = self.pause(id).await;

        if let Some(rec) = self.db.get_download(id).await? {
            if delete_files {
                let part = PathBuf::from(&rec.part_path);
                let target = PathBuf::from(&rec.target_path);
                let state_file = crate::state::DownloadState::state_file_path(&part);

                if part.exists() {
                    let _ = tokio::fs::remove_file(part).await;
                }
                if state_file.exists() {
                    let _ = tokio::fs::remove_file(state_file).await;
                }
                if target.exists() {
                    let _ = tokio::fs::remove_file(target).await;
                }
            }
        }

        self.db.delete_download(id).await?;
        info!("Deleted download task {}", id);
        Ok(())
    }

    /// Run the queue processing loop in the background.
    pub async fn run_queue_worker(self: Arc<Self>, cancel_token: CancellationToken) {
        info!("Download queue worker started");
        let _ = self.recover_interrupted_downloads().await;

        while !cancel_token.is_cancelled() && !self.shutting_down.load(Ordering::SeqCst) {
            let max_c = *self.max_concurrent.lock().await;
            let current_active = self.active_tasks.lock().await.len();

            if current_active < max_c {
                let available_slots = max_c - current_active;
                if let Ok(queued) = self.db.list_downloads(Some("queued")).await {
                    for record in queued.into_iter().take(available_slots) {
                        let self_clone = Arc::clone(&self);
                        let record_id = record.id.clone();
                        let task_cancel = CancellationToken::new();

                        {
                            let mut tasks = self_clone.active_tasks.lock().await;
                            tasks.insert(record_id.clone(), task_cancel.clone());
                        }

                        let _ = self_clone.db.update_status(&record_id, "downloading", None).await;
                        let _ = self_clone.event_tx.send(QueueEvent::StatusChanged {
                            id: record_id.clone(),
                            status: "downloading".to_string(),
                        }).await;

                        tokio::spawn(async move {
                            self_clone.process_download(record, task_cancel).await;
                        });
                    }
                }
            }

            sleep(Duration::from_millis(500)).await;
        }

        info!("Download queue worker stopped");
    }

    /// Execute a single download with retries and rate limiting.
    async fn process_download(self: Arc<Self>, record: DownloadRecord, cancel_token: CancellationToken) {
        let task_id = record.id.clone();
        let options = DownloadOptions {
            url: record.url.clone(),
            output_path: Some(PathBuf::from(&record.target_path)),
            num_segments: record.num_segments,
            headers: HeaderMap::new(),
            user_agent: record.user_agent.clone(),
            referer: record.referer.clone(),
            cookie: record.cookie.clone(),
            expected_sha256: record.expected_sha256.clone(),
            resume: true,
        };

        let limiter = self.global_limiter.clone();
        let per_task_limiter = if record.max_speed_bps > 0 {
            Some(TokenBucketRateLimiter::new(record.max_speed_bps))
        } else {
            None
        };
        let proxy_url = self.proxy_url.lock().await.clone();

        let (prog_tx, mut prog_rx) = mpsc::channel::<DownloadProgress>(64);
        let id_for_prog = task_id.clone();
        let event_tx = self.event_tx.clone();
        let db = self.db.clone();

        // Forward progress updates
        let prog_handle = tokio::spawn(async move {
            let mut last_save = std::time::Instant::now();
            while let Some(prog) = prog_rx.recv().await {
                if last_save.elapsed() >= Duration::from_millis(1000) {
                    let _ = db.update_progress(&id_for_prog, prog.downloaded_bytes, prog.total_bytes).await;
                    last_save = std::time::Instant::now();
                }

                let _ = event_tx.send(QueueEvent::Progress {
                    id: id_for_prog.clone(),
                    progress: prog,
                }).await;
            }
        });

        let is_stream = crate::youtube::is_streaming_site(&record.url);
        let ytdlp_bin = if is_stream { crate::youtube::find_ytdlp_binary() } else { None };

        let result = if let Some(bin) = ytdlp_bin {
            crate::youtube::download_streaming_video(
                &bin,
                &record.url,
                Path::new(&record.target_path),
                cancel_token.clone(),
                prog_tx,
            ).await
        } else {
            let mut downloader = Downloader::new(options, cancel_token.clone())
                .with_rate_limiter(limiter);
            if let Some(tl) = per_task_limiter {
                downloader = downloader.with_task_rate_limiter(tl);
            }
            match downloader.with_proxy(proxy_url.as_deref()) {
                Ok(configured) => configured.start(prog_tx).await,
                Err(e) => Err(e),
            }
        };

        let _ = prog_handle.await;

        // Remove from active tasks
        {
            let mut tasks = self.active_tasks.lock().await;
            tasks.remove(&task_id);
        }

        match result {
            Ok(actual_path) => {
                let actual_path_str = actual_path.to_string_lossy().to_string();
                if !actual_path_str.is_empty() && actual_path_str != record.target_path {
                    let _ = self.db.update_target_path(&task_id, &actual_path_str).await;
                }
                let _ = self.db.update_status(&task_id, "completed", None).await;
                let _ = self.event_tx.send(QueueEvent::TaskCompleted(task_id.clone())).await;
                info!("Queue download {} completed successfully", task_id);
            }
            Err(DownloadError::Cancelled) => {
                let next_status = if self.shutting_down.load(Ordering::SeqCst) {
                    "queued"
                } else {
                    "paused"
                };
                let _ = self.db.update_status(&task_id, next_status, None).await;
                let _ = self.event_tx.send(QueueEvent::StatusChanged {
                    id: task_id.clone(),
                    status: next_status.to_string(),
                }).await;
                info!("Queue download {} {}", task_id, next_status);
            }
            Err(e) => {
                if self.shutting_down.load(Ordering::SeqCst) {
                    let _ = self.db.update_status(&task_id, "queued", None).await;
                    return;
                }
                let err_msg = e.to_string();
                warn!("Queue download {} encountered error: {}", task_id, err_msg);

                let non_retryable = matches!(
                    &e,
                    DownloadError::InvalidUrl(_)
                        | DownloadError::ChecksumMismatch { .. }
                        | DownloadError::ServerFailed { status: 400 | 401 | 403 | 404 | 410 | 451, .. }
                ) || err_msg.contains("Insufficient disk space");

                // Exponential backoff retry logic for transient errors
                if !non_retryable && record.retry_count < record.max_retries {
                    let next_retry = self.db.increment_retry(&task_id, &err_msg).await.unwrap_or(record.retry_count + 1);
                    let backoff_secs = (2u64.pow(next_retry.min(5))).min(30);
                    info!(
                        "Scheduling retry {}/{} for task {} in {} seconds",
                        next_retry, record.max_retries, task_id, backoff_secs
                    );

                    let self_clone = Arc::clone(&self);
                    let retry_id = task_id.clone();
                    tokio::spawn(async move {
                        sleep(Duration::from_secs(backoff_secs)).await;
                        if !self_clone.shutting_down.load(Ordering::SeqCst) {
                            let _ = self_clone.db.update_status(&retry_id, "queued", None).await;
                            let _ = self_clone.event_tx.send(QueueEvent::StatusChanged {
                                id: retry_id,
                                status: "queued".to_string(),
                            }).await;
                        }
                    });
                } else {
                    let _ = self.db.update_status(&task_id, "failed", Some(&err_msg)).await;
                    let _ = self.event_tx.send(QueueEvent::TaskFailed {
                        id: task_id.clone(),
                        error: err_msg,
                    }).await;
                    error!("Download {} failed: retries={}/{}", task_id, record.retry_count, record.max_retries);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_add_and_pause() {
        let db = Database::open_in_memory().unwrap();
        let (tx, mut rx) = mpsc::channel(32);
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let queue = Arc::new(QueueManager::new(db, 2, 0, tx));

        let opt = DownloadOptions {
            url: "https://example.com/song.mp3".to_string(),
            ..Default::default()
        };

        let id = queue.add_download(opt, None, 0).await.unwrap();
        let rec = queue.db().get_download(&id).await.unwrap().unwrap();
        assert_eq!(rec.status, "queued");
        assert_eq!(rec.category, "Music");

        queue.pause(&id).await.unwrap();
        let paused_rec = queue.db().get_download(&id).await.unwrap().unwrap();
        assert_eq!(paused_rec.status, "paused");

        queue.resume(&id).await.unwrap();
        let resumed_rec = queue.db().get_download(&id).await.unwrap().unwrap();
        assert_eq!(resumed_rec.status, "queued");
    }
}

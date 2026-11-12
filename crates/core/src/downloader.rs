use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER, USER_AGENT};
use reqwest::Client;
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use url::Url;

use crate::checksum::verify_sha256;
use crate::error::{DownloadError, Result};
use crate::probe::{probe_url, FileInfo};
use crate::rate_limiter::TokenBucketRateLimiter;
use crate::segment::{calculate_segments, Segment};
use crate::speed::SpeedTracker;
use crate::state::DownloadState;

/// Download options provided by the caller or CLI.
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// Target URL to download.
    pub url: String,
    /// Destination directory or file path. If None, saves in current dir with detected name.
    pub output_path: Option<PathBuf>,
    /// Number of concurrent segments to split into (1 to 32, default 8).
    pub num_segments: usize,
    /// Custom HTTP headers.
    pub headers: HeaderMap,
    /// Custom User-Agent.
    pub user_agent: Option<String>,
    /// Custom HTTP Referer.
    pub referer: Option<String>,
    /// Custom Cookie header.
    pub cookie: Option<String>,
    /// Expected SHA-256 checksum for verification.
    pub expected_sha256: Option<String>,
    /// Whether to attempt resuming if a partial download exists.
    pub resume: bool,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            url: String::new(),
            output_path: None,
            num_segments: 8,
            headers: HeaderMap::new(),
            user_agent: None,
            referer: None,
            cookie: None,
            expected_sha256: None,
            resume: true,
        }
    }
}

/// Status of the download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStatus {
    Probing,
    Allocating,
    Downloading,
    Paused,
    Verifying,
    Completed,
    Failed(String),
}

/// Progress update emitted to callers during download.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub status: DownloadStatus,
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub speed_bps: f64,
    pub eta_seconds: Option<u64>,
    pub segments: Vec<Segment>,
    pub file_path: PathBuf,
}

impl DownloadProgress {
    pub fn percent(&self) -> f64 {
        match self.total_bytes {
            Some(total) if total > 0 => {
                ((self.downloaded_bytes as f64) / (total as f64) * 100.0).clamp(0.0, 100.0)
            }
            Some(0) => 100.0,
            _ => 0.0,
        }
    }
}

/// Context passed to a single segment worker task.
struct SegmentWorkerTask {
    index: usize,
    client: Client,
    headers: HeaderMap,
    url: String,
    part_path: PathBuf,
    shared_segments: Arc<Mutex<Vec<Segment>>>,
    progress_notify: mpsc::Sender<()>,
    cancel: CancellationToken,
    rate_limiter: Option<TokenBucketRateLimiter>,
}

/// Active download task manager.
pub struct Downloader {
    options: DownloadOptions,
    client: Client,
    cancel_token: CancellationToken,
    rate_limiter: Option<TokenBucketRateLimiter>,
}

impl Downloader {
    /// Create a new Downloader with the specified options and cancellation token.
    pub fn new(options: DownloadOptions, cancel_token: CancellationToken) -> Self {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(15));

        if let Some(ref ua) = options.user_agent {
            if let Ok(val) = HeaderValue::from_str(ua) {
                let mut default_headers = HeaderMap::new();
                default_headers.insert(USER_AGENT, val);
                client_builder = client_builder.default_headers(default_headers);
            }
        }

        let client = client_builder.build().unwrap_or_else(|_| Client::new());

        Self {
            options,
            client,
            cancel_token,
            rate_limiter: None,
        }
    }

    /// Attach a rate limiter to throttle download throughput.
    pub fn with_rate_limiter(mut self, limiter: TokenBucketRateLimiter) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    /// Prepare request headers incorporating User-Agent, Referer, Cookie, and custom headers.
    fn build_headers(&self) -> HeaderMap {
        let mut headers = self.options.headers.clone();

        if let Some(ref ua) = self.options.user_agent {
            if let Ok(v) = HeaderValue::from_str(ua) {
                headers.insert(USER_AGENT, v);
            }
        }
        if let Some(ref referer) = self.options.referer {
            if let Ok(v) = HeaderValue::from_str(referer) {
                headers.insert(REFERER, v);
            }
        }
        if let Some(ref cookie) = self.options.cookie {
            if let Ok(v) = HeaderValue::from_str(cookie) {
                headers.insert(COOKIE, v);
            }
        }

        headers
    }

    /// Starts or resumes the download, sending progress updates through `progress_tx`.
    pub async fn start(&self, progress_tx: mpsc::Sender<DownloadProgress>) -> Result<PathBuf> {
        let url = Url::parse(&self.options.url)
            .map_err(|e| DownloadError::InvalidUrl(format!("Invalid URL '{}': {}", self.options.url, e)))?;

        let headers = self.build_headers();

        // Send initial Probing status
        let _ = progress_tx.send(DownloadProgress {
            status: DownloadStatus::Probing,
            total_bytes: None,
            downloaded_bytes: 0,
            speed_bps: 0.0,
            eta_seconds: None,
            segments: Vec::new(),
            file_path: PathBuf::new(),
        }).await;

        let info = probe_url(&self.client, &url, &headers).await?;
        debug!("Probe result: {:?}", info);

        // Resolve destination file path
        let target_path = self.resolve_target_path(&info)?;
        let part_path = self.resolve_part_path(&target_path);
        let state_path = DownloadState::state_file_path(&part_path);

        // Check if final file already exists and is complete
        if target_path.exists() && !part_path.exists() && !state_path.exists() {
            if let Some(expected_size) = info.content_length {
                if let Ok(meta) = fs::metadata(&target_path).await {
                    if meta.len() == expected_size {
                        info!("Target file already exists with matching size: {:?}", target_path);
                        let _ = progress_tx.send(DownloadProgress {
                            status: DownloadStatus::Completed,
                            total_bytes: Some(expected_size),
                            downloaded_bytes: expected_size,
                            speed_bps: 0.0,
                            eta_seconds: Some(0),
                            segments: Vec::new(),
                            file_path: target_path.clone(),
                        }).await;
                        return Ok(target_path);
                    }
                }
            }
        }

        // Try resuming from existing state
        let mut state = if self.options.resume && state_path.exists() {
            match DownloadState::load(&state_path).await {
                Ok(Some(mut loaded)) if loaded.url == self.options.url => {
                    info!("Resuming existing download from {:?}", state_path);
                    // Verify part file exists
                    if !part_path.exists() {
                        warn!("Part file missing for existing state, resetting progress");
                        for s in &mut loaded.segments {
                            s.downloaded = 0;
                            s.complete = false;
                        }
                    }
                    loaded
                }
                _ => self.init_new_state(&info, target_path.clone(), part_path.clone()).await?,
            }
        } else {
            self.init_new_state(&info, target_path.clone(), part_path.clone()).await?
        };

        // Ensure parent directory exists
        if let Some(parent) = part_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Send Allocating progress
        let _ = progress_tx.send(DownloadProgress {
            status: DownloadStatus::Allocating,
            total_bytes: state.total_size,
            downloaded_bytes: state.downloaded_bytes(),
            speed_bps: 0.0,
            eta_seconds: None,
            segments: state.segments.clone(),
            file_path: target_path.clone(),
        }).await;

        // Multi-segment or Single-stream
        if state.accepts_ranges && state.total_size.is_some() && state.segments.len() > 1 {
            self.download_multisegment(&mut state, headers, &progress_tx).await?;
        } else {
            self.download_single_stream(&mut state, headers, &progress_tx).await?;
        }

        // Check if cancelled/paused
        if self.cancel_token.is_cancelled() {
            info!("Download paused/cancelled by user.");
            let _ = progress_tx.send(DownloadProgress {
                status: DownloadStatus::Paused,
                total_bytes: state.total_size,
                downloaded_bytes: state.downloaded_bytes(),
                speed_bps: 0.0,
                eta_seconds: None,
                segments: state.segments.clone(),
                file_path: target_path.clone(),
            }).await;
            return Err(DownloadError::Cancelled);
        }

        // Finalize: Verify checksum if provided
        if let Some(ref expected) = self.options.expected_sha256 {
            let _ = progress_tx.send(DownloadProgress {
                status: DownloadStatus::Verifying,
                total_bytes: state.total_size,
                downloaded_bytes: state.downloaded_bytes(),
                speed_bps: 0.0,
                eta_seconds: None,
                segments: state.segments.clone(),
                file_path: target_path.clone(),
            }).await;

            verify_sha256(&part_path, expected).await?;
            info!("Checksum SHA-256 verified successfully!");
        }

        // Rename part file to final target file
        if target_path.exists() {
            let _ = fs::remove_file(&target_path).await;
        }
        fs::rename(&part_path, &target_path).await?;
        state.cleanup().await?;

        info!("Download completed successfully: {:?}", target_path);

        let final_bytes = state.total_size.unwrap_or_else(|| state.downloaded_bytes());
        let _ = progress_tx.send(DownloadProgress {
            status: DownloadStatus::Completed,
            total_bytes: Some(final_bytes),
            downloaded_bytes: final_bytes,
            speed_bps: 0.0,
            eta_seconds: Some(0),
            segments: state.segments.clone(),
            file_path: target_path.clone(),
        }).await;

        Ok(target_path)
    }

    /// Initialize a fresh DownloadState based on probe results.
    async fn init_new_state(&self, info: &FileInfo, target_path: PathBuf, part_path: PathBuf) -> Result<DownloadState> {
        let accepts_ranges = info.accepts_ranges && info.content_length.is_some();
        let segments = if accepts_ranges {
            let total = info.content_length.unwrap();
            calculate_segments(total, self.options.num_segments)
        } else {
            vec![Segment {
                index: 0,
                start: 0,
                end: info.content_length.unwrap_or(0),
                downloaded: 0,
                complete: false,
            }]
        };

        let mut state = DownloadState {
            id: format!("{:x}", md5_or_simple_hash(info.url.as_ref())),
            url: info.url.to_string(),
            target_path,
            part_path,
            total_size: info.content_length,
            accepts_ranges,
            etag: info.etag.clone(),
            last_modified: info.last_modified.clone(),
            expected_sha256: self.options.expected_sha256.clone(),
            segments,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        state.save_atomic().await?;
        Ok(state)
    }

    /// Parallel multi-segment download worker pool.
    async fn download_multisegment(
        &self,
        state: &mut DownloadState,
        base_headers: HeaderMap,
        progress_tx: &mpsc::Sender<DownloadProgress>,
    ) -> Result<()> {
        let total_size = state.total_size.ok_or(DownloadError::NoContentLength)?;

        // Preallocate file on disk
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&state.part_path)
            .await?;
        file.set_len(total_size).await?;
        drop(file);

        let shared_segments = Arc::new(Mutex::new(state.segments.clone()));
        let cancel_token = self.cancel_token.clone();
        let worker_cancel = cancel_token.child_token();
        let part_path = state.part_path.clone();
        let client = self.client.clone();
        let url = state.url.clone();
        let rate_limiter = self.rate_limiter.clone();

        // Channel for notifying progress updates
        let (bytes_tx, mut bytes_rx) = mpsc::channel::<()>(128);

        let num_workers = shared_segments.lock().await.len();
        let mut worker_handles = Vec::with_capacity(num_workers);

        for seg_idx in 0..num_workers {
            let seg_clone = {
                let segs = shared_segments.lock().await;
                segs[seg_idx].clone()
            };

            if seg_clone.complete {
                continue;
            }

            let task_worker_cancel = worker_cancel.clone();
            let task = SegmentWorkerTask {
                index: seg_idx,
                client: client.clone(),
                headers: base_headers.clone(),
                url: url.clone(),
                part_path: part_path.clone(),
                shared_segments: Arc::clone(&shared_segments),
                progress_notify: bytes_tx.clone(),
                cancel: worker_cancel.clone(),
                rate_limiter: rate_limiter.clone(),
            };

            let handle = tokio::spawn(async move {
                let res = Self::run_segment_worker(task).await;
                if res.is_err() {
                    // Abort sibling segment workers quickly on any segment error
                    task_worker_cancel.cancel();
                }
                res
            });

            worker_handles.push(handle);
        }

        drop(bytes_tx); // Drop extra sender so rx detects when all workers finish

        // Progress monitor loop
        let mut tracker = SpeedTracker::default();
        let target_path = state.target_path.clone();

        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    break;
                }
                msg = bytes_rx.recv() => {
                    let segs = shared_segments.lock().await.clone();
                    let downloaded: u64 = segs.iter().map(|s| s.downloaded).sum();
                    tracker.record(downloaded);

                    let speed = tracker.speed_bps();
                    let remaining = total_size.saturating_sub(downloaded);
                    let eta = tracker.eta_seconds(remaining);

                    let _ = progress_tx.send(DownloadProgress {
                        status: DownloadStatus::Downloading,
                        total_bytes: Some(total_size),
                        downloaded_bytes: downloaded,
                        speed_bps: speed,
                        eta_seconds: eta,
                        segments: segs,
                        file_path: target_path.clone(),
                    }).await;

                    if msg.is_none() {
                        // All workers finished
                        break;
                    }
                }
                _ = sleep(Duration::from_millis(500)) => {
                    // Periodic save and UI refresh
                    let segs = shared_segments.lock().await.clone();
                    let downloaded: u64 = segs.iter().map(|s| s.downloaded).sum();
                    tracker.record(downloaded);

                    let speed = tracker.speed_bps();
                    let remaining = total_size.saturating_sub(downloaded);
                    let eta = tracker.eta_seconds(remaining);

                    state.segments = segs.clone();
                    let _ = state.save_atomic().await;

                    let _ = progress_tx.send(DownloadProgress {
                        status: DownloadStatus::Downloading,
                        total_bytes: Some(total_size),
                        downloaded_bytes: downloaded,
                        speed_bps: speed,
                        eta_seconds: eta,
                        segments: segs,
                        file_path: target_path.clone(),
                    }).await;

                    if downloaded >= total_size {
                        break;
                    }
                }
            }
        }

        // Wait for workers to cleanly terminate and capture any worker failure
        let mut first_error: Option<DownloadError> = None;
        for handle in worker_handles {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    error!("Segment worker failed: {:?}", e);
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
                Err(e) => {
                    error!("Segment worker task panicked: {:?}", e);
                    if first_error.is_none() {
                        first_error = Some(DownloadError::Io(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Segment worker task panicked: {}", e),
                        )));
                    }
                }
            }
        }

        // Update state with final segment results so progress is preserved for resume
        state.segments = shared_segments.lock().await.clone();
        state.save_atomic().await?;

        if self.cancel_token.is_cancelled() {
            return Ok(());
        }

        if let Some(err) = first_error {
            return Err(err);
        }

        let actual_downloaded = state.downloaded_bytes();
        if actual_downloaded < total_size {
            return Err(DownloadError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "Download incomplete: received {} of {} bytes",
                    actual_downloaded, total_size
                ),
            )));
        }

        Ok(())
    }

    /// Individual worker downloading a single segment range.
    async fn run_segment_worker(mut task: SegmentWorkerTask) -> Result<()> {
        let (start_offset, end_byte) = {
            let segs = task.shared_segments.lock().await;
            let s = &segs[task.index];
            (s.current_offset(), s.end)
        };

        if start_offset > end_byte {
            let mut segs = task.shared_segments.lock().await;
            segs[task.index].complete = true;
            return Ok(());
        }

        let range_header = format!("bytes={}-{}", start_offset, end_byte);
        debug!("Segment {}: requesting Range {}", task.index, range_header);
        task.headers.insert(reqwest::header::RANGE, range_header.parse().unwrap());

        let resp = task.client.get(&task.url).headers(task.headers).send().await?;
        let status = resp.status();
        if status.as_u16() != 206 {
            return Err(DownloadError::ServerFailed {
                status: status.as_u16(),
                message: format!(
                    "Segment {} expected 206 Partial Content for {}, got {}",
                    task.index, range_header, status
                ),
            });
        }

        let mut file = OpenOptions::new()
            .write(true)
            .open(&task.part_path)
            .await?;
        file.seek(SeekFrom::Start(start_offset)).await?;

        let mut stream = resp.bytes_stream();
        let mut last_notify = std::time::Instant::now();
        let max_remaining = end_byte - start_offset + 1;
        let mut written_in_run: u64 = 0;

        while let Some(chunk_res) = stream.next().await {
            if task.cancel.is_cancelled() {
                file.flush().await?;
                return Ok(());
            }

            let chunk = chunk_res?;
            let remaining_needed = max_remaining.saturating_sub(written_in_run);
            if remaining_needed == 0 {
                break;
            }
            let slice_len = std::cmp::min(chunk.len() as u64, remaining_needed) as usize;
            let slice = &chunk[..slice_len];

            // Throttle rate if limiter is configured
            if let Some(ref limiter) = task.rate_limiter {
                limiter.acquire(slice_len).await;
            }

            file.write_all(slice).await?;
            written_in_run += slice_len as u64;

            {
                let mut segs = task.shared_segments.lock().await;
                segs[task.index].downloaded = segs[task.index].downloaded.saturating_add(slice_len as u64);
                if segs[task.index].current_offset() > segs[task.index].end {
                    segs[task.index].complete = true;
                }
            }

            if last_notify.elapsed() >= Duration::from_millis(100) {
                let _ = task.progress_notify.try_send(());
                last_notify = std::time::Instant::now();
            }

            if written_in_run >= max_remaining {
                break;
            }
        }

        file.flush().await?;

        let (seg_downloaded, seg_total) = {
            let mut segs = task.shared_segments.lock().await;
            let s = &mut segs[task.index];
            let total = s.total_bytes();
            if s.downloaded >= total {
                s.complete = true;
            }
            (s.downloaded, total)
        };

        if !task.cancel.is_cancelled() && seg_downloaded < seg_total {
            return Err(DownloadError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "Segment {} ended prematurely: {} of {} bytes",
                    task.index, seg_downloaded, seg_total
                ),
            )));
        }

        let _ = task.progress_notify.try_send(());
        debug!("Segment {} completed", task.index);
        Ok(())
    }

    /// Fallback download for single connections or servers without range support.
    async fn download_single_stream(
        &self,
        state: &mut DownloadState,
        headers: HeaderMap,
        progress_tx: &mpsc::Sender<DownloadProgress>,
    ) -> Result<()> {
        let resp = self.client.get(&state.url).headers(headers).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(DownloadError::ServerFailed {
                status: status.as_u16(),
                message: format!("GET request failed with status {}", status),
            });
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&state.part_path)
            .await?;

        let mut stream = resp.bytes_stream();
        let mut tracker = SpeedTracker::default();
        let mut downloaded: u64 = 0;
        let total_size = state.total_size;

        while let Some(chunk_res) = stream.next().await {
            if self.cancel_token.is_cancelled() {
                file.flush().await?;
                return Ok(());
            }

            let chunk = chunk_res?;
            let chunk_len = chunk.len();

            // Throttle rate if limiter is configured
            if let Some(ref limiter) = self.rate_limiter {
                limiter.acquire(chunk_len).await;
            }

            file.write_all(&chunk).await?;
            downloaded += chunk_len as u64;

            tracker.record(downloaded);
            let speed = tracker.speed_bps();
            let eta = total_size.and_then(|t| tracker.eta_seconds(t.saturating_sub(downloaded)));

            if !state.segments.is_empty() {
                state.segments[0].downloaded = downloaded;
            }

            let _ = progress_tx.send(DownloadProgress {
                status: DownloadStatus::Downloading,
                total_bytes: total_size,
                downloaded_bytes: downloaded,
                speed_bps: speed,
                eta_seconds: eta,
                segments: state.segments.clone(),
                file_path: state.target_path.clone(),
            }).await;
        }

        file.flush().await?;
        if !state.segments.is_empty() {
            state.segments[0].complete = true;
        }
        state.save_atomic().await?;

        Ok(())
    }

    /// Resolve destination output path based on options and probe info.
    /// Sanitizes target path to always ensure a valid filename and extension based on MIME type.
    fn resolve_target_path(&self, info: &FileInfo) -> Result<PathBuf> {
        let mut path = match &self.options.output_path {
            Some(p) => {
                if p.is_dir() {
                    p.join(&info.suggested_filename)
                } else {
                    let mut file_path = p.clone();
                    let file_name = file_path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let lower = file_name.to_ascii_lowercase();

                    // If user or extension passed generic "download", replace with probe's suggested filename
                    if lower == "download" || lower == "download.bin" || lower.is_empty() {
                        file_path.set_file_name(&info.suggested_filename);
                    } else if !file_name.contains('.') {
                        // If file has no extension at all, append extension inferred from MIME
                        let ext = crate::probe::mime_to_extension(info.mime_type.as_deref());
                        if ext != "bin" {
                            file_path.set_file_name(format!("{}.{}", file_name, ext));
                        }
                    }
                    file_path
                }
            }
            None => {
                let current_dir = std::env::current_dir()?;
                current_dir.join(&info.suggested_filename)
            }
        };

        // Final sanity check: ensure target path doesn't end with extensionless "download"
        let current_leaf = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if current_leaf.eq_ignore_ascii_case("download") {
            let ext = crate::probe::mime_to_extension(info.mime_type.as_deref());
            path.set_file_name(format!("download.{}", ext));
        }

        Ok(path)
    }

    /// Generate the `.cosmo.part` path.
    fn resolve_part_path(&self, target_path: &Path) -> PathBuf {
        let mut cosmo_os = target_path.as_os_str().to_os_string();
        cosmo_os.push(".cosmo.part");
        let cosmo_path = PathBuf::from(cosmo_os);
        if cosmo_path.exists() {
            return cosmo_path;
        }

        let mut legacy_os = target_path.as_os_str().to_os_string();
        legacy_os.push(".rustget.part");
        let legacy_path = PathBuf::from(legacy_os);
        if legacy_path.exists() {
            return legacy_path;
        }

        cosmo_path
    }
}

/// Simple fast hash for unique ID generation.
fn md5_or_simple_hash(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

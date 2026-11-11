pub mod category;
pub mod checksum;
pub mod clipboard;
pub mod db;
pub mod downloader;
pub mod error;
pub mod probe;
pub mod queue;
pub mod rate_limiter;
pub mod segment;
pub mod server;
pub mod speed;
pub mod state;
pub mod youtube;

// Re-exports for convenient top-level access
pub use category::Category;
pub use checksum::{compute_sha256, verify_sha256};
pub use clipboard::ClipboardMonitor;
pub use db::{Database, DownloadRecord};
pub use downloader::{DownloadOptions, DownloadProgress, DownloadStatus, Downloader};
pub use error::{DownloadError, Result};
pub use probe::{extract_filename, probe_url, sanitize_filename, FileInfo};
pub use queue::{QueueEvent, QueueManager};
pub use rate_limiter::TokenBucketRateLimiter;
pub use segment::{calculate_segments, Segment};
pub use server::{BridgeServer, ExtensionDownloadRequest, spawn_on_demand_popup, get_or_create_bridge_token};
pub use speed::SpeedTracker;
pub use state::DownloadState;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

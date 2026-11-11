use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, warn};

use crate::error::{DownloadError, Result};
use crate::segment::Segment;

/// Persisted state for an active or paused download.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadState {
    /// Unique identifier for this download task.
    pub id: String,
    /// Remote URL.
    pub url: String,
    /// Destination path of the final downloaded file.
    pub target_path: PathBuf,
    /// Path to the active partial file during download.
    pub part_path: PathBuf,
    /// Total file size if known.
    pub total_size: Option<u64>,
    /// Whether the server supports byte range requests.
    pub accepts_ranges: bool,
    /// HTTP ETag validator.
    pub etag: Option<String>,
    /// HTTP Last-Modified validator.
    pub last_modified: Option<String>,
    /// User-supplied SHA-256 checksum for verification.
    pub expected_sha256: Option<String>,
    /// Segments tracking download progress.
    pub segments: Vec<Segment>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl DownloadState {
    /// Generate the path to the JSON state file associated with a part file.
    pub fn state_file_path(part_path: &Path) -> PathBuf {
        let mut path_str = part_path.as_os_str().to_os_string();
        path_str.push(".state.json");
        PathBuf::from(path_str)
    }

    /// Total bytes downloaded across all segments.
    pub fn downloaded_bytes(&self) -> u64 {
        self.segments.iter().map(|s| s.downloaded).sum()
    }

    /// Overall progress percentage [0.0, 100.0].
    pub fn progress_percent(&self) -> f64 {
        match self.total_size {
            Some(total) if total > 0 => {
                ((self.downloaded_bytes() as f64) / (total as f64) * 100.0).clamp(0.0, 100.0)
            }
            Some(0) => 100.0,
            _ => 0.0,
        }
    }

    /// Check if all segments are marked complete.
    pub fn is_complete(&self) -> bool {
        !self.segments.is_empty() && self.segments.iter().all(|s| s.complete)
    }

    /// Atomically persist this state to disk.
    ///
    /// Writes to a temporary file first and renames to the target filename,
    /// preventing state corruption if the process crashes mid-write.
    pub async fn save_atomic(&mut self) -> Result<()> {
        self.updated_at = Utc::now();
        let state_path = Self::state_file_path(&self.part_path);
        let tmp_path = state_path.with_extension("tmp");

        let json_bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;

        // Write to temp file and sync
        fs::write(&tmp_path, &json_bytes).await?;
        // Atomic rename
        fs::rename(&tmp_path, &state_path).await?;

        debug!("Saved download state to {:?}", state_path);
        Ok(())
    }

    /// Load existing download state from a state file if present.
    pub async fn load(state_path: &Path) -> Result<Option<DownloadState>> {
        if !state_path.exists() {
            return Ok(None);
        }

        let data = match fs::read(state_path).await {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("Failed to read state file {:?}: {:?}", state_path, e);
                return Err(DownloadError::Io(e));
            }
        };

        match serde_json::from_slice::<DownloadState>(&data) {
            Ok(state) => {
                debug!("Loaded state for download ID: {}", state.id);
                Ok(Some(state))
            }
            Err(e) => {
                warn!("Corrupted state file at {:?}: {:?}", state_path, e);
                Err(DownloadError::StateCorrupted(e.to_string()))
            }
        }
    }

    /// Clean up state file after a successful download.
    pub async fn cleanup(&self) -> Result<()> {
        let state_path = Self::state_file_path(&self.part_path);
        if state_path.exists() {
            let _ = fs::remove_file(state_path).await;
        }
        let tmp_path = Self::state_file_path(&self.part_path).with_extension("tmp");
        if tmp_path.exists() {
            let _ = fs::remove_file(tmp_path).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_state_atomic_save_and_load() {
        let dir = tempdir().unwrap();
        let part_path = dir.path().join("file.iso.rustget.part");
        let target_path = dir.path().join("file.iso");

        let mut state = DownloadState {
            id: "test-id-123".to_string(),
            url: "https://example.com/file.iso".to_string(),
            target_path,
            part_path: part_path.clone(),
            total_size: Some(1000),
            accepts_ranges: true,
            etag: Some("\"etag123\"".to_string()),
            last_modified: None,
            expected_sha256: None,
            segments: vec![
                Segment {
                    index: 0,
                    start: 0,
                    end: 499,
                    downloaded: 250,
                    complete: false,
                },
                Segment {
                    index: 1,
                    start: 500,
                    end: 999,
                    downloaded: 0,
                    complete: false,
                },
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        state.save_atomic().await.unwrap();

        let state_file = DownloadState::state_file_path(&part_path);
        assert!(state_file.exists());

        let loaded = DownloadState::load(&state_file).await.unwrap().unwrap();
        assert_eq!(loaded.id, "test-id-123");
        assert_eq!(loaded.downloaded_bytes(), 250);
        assert_eq!(loaded.progress_percent(), 25.0);
        assert_eq!(loaded.segments.len(), 2);

        state.cleanup().await.unwrap();
        assert!(!state_file.exists());
    }
}

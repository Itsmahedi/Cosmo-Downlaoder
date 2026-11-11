use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use chrono::Utc;
use directories::ProjectDirs;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::{DownloadError, Result};

/// Stored download task in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRecord {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub target_path: String,
    pub part_path: String,
    pub category: String,
    pub total_size: Option<u64>,
    pub downloaded_bytes: u64,
    pub status: String, // "queued", "downloading", "paused", "failed", "completed"
    pub num_segments: usize,
    pub expected_sha256: Option<String>,
    pub custom_headers: Option<String>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub cookie: Option<String>,
    pub max_speed_bps: u64,
    pub retry_count: u32,
    pub max_retries: u32,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Thread-safe SQLite database manager.
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open or create the database at the default application location.
    pub fn open_default() -> Result<Self> {
        let db_path = Self::default_db_path()?;
        Self::open(&db_path)
    }

    /// Open or create the database at a specific path.
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)
            .map_err(|e| DownloadError::StateCorrupted(format!("Failed to open SQLite db: {}", e)))?;

        // Enable WAL mode for high concurrency and crash safety
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.migrate()?;
        info!("SQLite database initialized at {:?}", db_path);
        Ok(db)
    }

    /// Create in-memory database for testing.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.migrate()?;
        Ok(db)
    }

    /// Default database file location on Windows (%LOCALAPPDATA%\CosmoDownloader\cosmodownloader.db).
    pub fn default_db_path() -> Result<PathBuf> {
        let proj = ProjectDirs::from("com", "CosmoDownloader", "CosmoDownloader")
            .ok_or_else(|| DownloadError::StateCorrupted("Could not determine AppData directory".to_string()))?;
        let db_dir = proj.data_local_dir();
        let cosmo_db = db_dir.join("cosmodownloader.db");

        // Migrate from legacy RustGet db if present
        if !cosmo_db.exists() {
            if let Some(old_proj) = ProjectDirs::from("com", "RustGet", "RustGet") {
                let old_db = old_proj.data_local_dir().join("rustget.db");
                if old_db.exists() {
                    let _ = std::fs::create_dir_all(db_dir);
                    let _ = std::fs::copy(&old_db, &cosmo_db);
                }
            }
        }

        Ok(cosmo_db)
    }

    /// Apply initial schema migrations.
    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| DownloadError::StateCorrupted("Mutex poisoned".to_string()))?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS downloads (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                filename TEXT NOT NULL,
                target_path TEXT NOT NULL,
                part_path TEXT NOT NULL,
                category TEXT NOT NULL,
                total_size INTEGER,
                downloaded_bytes INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL,
                num_segments INTEGER NOT NULL DEFAULT 8,
                expected_sha256 TEXT,
                custom_headers TEXT,
                user_agent TEXT,
                referer TEXT,
                cookie TEXT,
                max_speed_bps INTEGER NOT NULL DEFAULT 0,
                retry_count INTEGER NOT NULL DEFAULT 0,
                max_retries INTEGER NOT NULL DEFAULT 5,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_downloads_status ON downloads(status);
            CREATE INDEX IF NOT EXISTS idx_downloads_created ON downloads(created_at);

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            INSERT OR IGNORE INTO settings (key, value) VALUES ('max_concurrent_downloads', '3');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('global_speed_limit_bps', '0');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('clipboard_monitor_enabled', 'true');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('minimize_to_tray_enabled', 'true');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('show_tray_icon_enabled', 'true');
            "#,
        )
        .map_err(|e| DownloadError::StateCorrupted(format!("Migration failed: {}", e)))?;

        Ok(())
    }

    /// Insert a new download record.
    pub async fn insert_download(&self, record: &DownloadRecord) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| DownloadError::StateCorrupted("Mutex poisoned".to_string()))?;
        conn.execute(
            r#"
            INSERT INTO downloads (
                id, url, filename, target_path, part_path, category,
                total_size, downloaded_bytes, status, num_segments,
                expected_sha256, custom_headers, user_agent, referer, cookie,
                max_speed_bps, retry_count, max_retries, error_message,
                created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19,
                ?20, ?21
            )
            "#,
            params![
                record.id,
                record.url,
                record.filename,
                record.target_path,
                record.part_path,
                record.category,
                record.total_size.map(|s| s as i64),
                record.downloaded_bytes as i64,
                record.status,
                record.num_segments as i64,
                record.expected_sha256,
                record.custom_headers,
                record.user_agent,
                record.referer,
                record.cookie,
                record.max_speed_bps as i64,
                record.retry_count,
                record.max_retries,
                record.error_message,
                record.created_at,
                record.updated_at,
            ],
        )
        .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;

        debug!("Inserted download task {} into SQLite", record.id);
        Ok(())
    }

    /// Update status and optional error message.
    pub async fn update_status(&self, id: &str, status: &str, error: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| DownloadError::StateCorrupted("Mutex poisoned".to_string()))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE downloads SET status = ?1, error_message = ?2, updated_at = ?3 WHERE id = ?4",
            params![status, error, now, id],
        )
        .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;
        Ok(())
    }

    /// Update target path and filename when resolved after download.
    pub async fn update_target_path(&self, id: &str, target_path: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| DownloadError::StateCorrupted("Mutex poisoned".to_string()))?;
        let now = Utc::now().to_rfc3339();
        let fname = std::path::Path::new(target_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        conn.execute(
            "UPDATE downloads SET target_path = ?1, filename = CASE WHEN ?2 != '' THEN ?2 ELSE filename END, updated_at = ?3 WHERE id = ?4",
            params![target_path, fname, now, id],
        )
        .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;
        Ok(())
    }

    /// Update progress byte counters.
    pub async fn update_progress(&self, id: &str, downloaded: u64, total: Option<u64>) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| DownloadError::StateCorrupted("Mutex poisoned".to_string()))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE downloads SET downloaded_bytes = ?1, total_size = COALESCE(?2, total_size), updated_at = ?3 WHERE id = ?4",
            params![downloaded as i64, total.map(|t| t as i64), now, id],
        )
        .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;
        Ok(())
    }

    /// Increment retry count.
    pub async fn increment_retry(&self, id: &str, error: &str) -> Result<u32> {
        let conn = self.conn.lock().map_err(|_| DownloadError::StateCorrupted("Mutex poisoned".to_string()))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE downloads SET retry_count = retry_count + 1, error_message = ?1, updated_at = ?2 WHERE id = ?3",
            params![error, now, id],
        )
        .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;

        let count: u32 = conn.query_row(
            "SELECT retry_count FROM downloads WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;

        Ok(count)
    }

    /// Fetch a single download by ID.
    pub async fn get_download(&self, id: &str) -> Result<Option<DownloadRecord>> {
        let conn = self.conn.lock().map_err(|_| DownloadError::StateCorrupted("Mutex poisoned".to_string()))?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, url, filename, target_path, part_path, category,
                       total_size, downloaded_bytes, status, num_segments,
                       expected_sha256, custom_headers, user_agent, referer, cookie,
                       max_speed_bps, retry_count, max_retries, error_message,
                       created_at, updated_at
                FROM downloads WHERE id = ?1
                "#,
            )
            .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;

        let rec = stmt
            .query_row(params![id], |row| {
                let total_size_i64: Option<i64> = row.get(6)?;
                let dl_i64: i64 = row.get(7)?;
                let segs_i64: i64 = row.get(9)?;
                let speed_i64: i64 = row.get(15)?;

                Ok(DownloadRecord {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    filename: row.get(2)?,
                    target_path: row.get(3)?,
                    part_path: row.get(4)?,
                    category: row.get(5)?,
                    total_size: total_size_i64.map(|s| s as u64),
                    downloaded_bytes: dl_i64 as u64,
                    status: row.get(8)?,
                    num_segments: segs_i64 as usize,
                    expected_sha256: row.get(10)?,
                    custom_headers: row.get(11)?,
                    user_agent: row.get(12)?,
                    referer: row.get(13)?,
                    cookie: row.get(14)?,
                    max_speed_bps: speed_i64 as u64,
                    retry_count: row.get(16)?,
                    max_retries: row.get(17)?,
                    error_message: row.get(18)?,
                    created_at: row.get(19)?,
                    updated_at: row.get(20)?,
                })
            })
            .optional()
            .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;

        Ok(rec)
    }

    /// List downloads with optional status filter.
    pub async fn list_downloads(&self, status_filter: Option<&str>) -> Result<Vec<DownloadRecord>> {
        let conn = self.conn.lock().map_err(|_| DownloadError::StateCorrupted("Mutex poisoned".to_string()))?;
        let query = if status_filter.is_some() {
            "SELECT id, url, filename, target_path, part_path, category, total_size, downloaded_bytes, status, num_segments, expected_sha256, custom_headers, user_agent, referer, cookie, max_speed_bps, retry_count, max_retries, error_message, created_at, updated_at FROM downloads WHERE status = ?1 ORDER BY created_at DESC"
        } else {
            "SELECT id, url, filename, target_path, part_path, category, total_size, downloaded_bytes, status, num_segments, expected_sha256, custom_headers, user_agent, referer, cookie, max_speed_bps, retry_count, max_retries, error_message, created_at, updated_at FROM downloads ORDER BY created_at DESC"
        };

        let mut stmt = conn.prepare(query)
            .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;

        let map_row = |row: &rusqlite::Row| {
            let total_size_i64: Option<i64> = row.get(6)?;
            let dl_i64: i64 = row.get(7)?;
            let segs_i64: i64 = row.get(9)?;
            let speed_i64: i64 = row.get(15)?;

            Ok(DownloadRecord {
                id: row.get(0)?,
                url: row.get(1)?,
                filename: row.get(2)?,
                target_path: row.get(3)?,
                part_path: row.get(4)?,
                category: row.get(5)?,
                total_size: total_size_i64.map(|s| s as u64),
                downloaded_bytes: dl_i64 as u64,
                status: row.get(8)?,
                num_segments: segs_i64 as usize,
                expected_sha256: row.get(10)?,
                custom_headers: row.get(11)?,
                user_agent: row.get(12)?,
                referer: row.get(13)?,
                cookie: row.get(14)?,
                max_speed_bps: speed_i64 as u64,
                retry_count: row.get(16)?,
                max_retries: row.get(17)?,
                error_message: row.get(18)?,
                created_at: row.get(19)?,
                updated_at: row.get(20)?,
            })
        };

        let records = if let Some(filter) = status_filter {
            let rows = stmt.query_map(params![filter], map_row)
                .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;
            rows.filter_map(std::result::Result::ok).collect()
        } else {
            let rows = stmt.query_map([], map_row)
                .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;
            rows.filter_map(std::result::Result::ok).collect()
        };

        Ok(records)
    }

    /// Delete a download record.
    pub async fn delete_download(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| DownloadError::StateCorrupted("Mutex poisoned".to_string()))?;
        conn.execute("DELETE FROM downloads WHERE id = ?1", params![id])
            .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;
        Ok(())
    }

    /// Get setting value.
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().map_err(|_| DownloadError::StateCorrupted("Mutex poisoned".to_string()))?;
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")
            .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;
        let val = stmt.query_row(params![key], |r| r.get(0)).optional()
            .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;
        Ok(val)
    }

    /// Set setting value.
    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| DownloadError::StateCorrupted("Mutex poisoned".to_string()))?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )
        .map_err(|e| DownloadError::StateCorrupted(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_crud() {
        let db = Database::open_in_memory().unwrap();

        let record = DownloadRecord {
            id: "uuid-test-1".to_string(),
            url: "https://example.com/test.iso".to_string(),
            filename: "test.iso".to_string(),
            target_path: "C:\\Downloads\\test.iso".to_string(),
            part_path: "C:\\Downloads\\test.iso.cosmo.part".to_string(),
            category: "Compressed".to_string(),
            total_size: Some(1048576),
            downloaded_bytes: 0,
            status: "queued".to_string(),
            num_segments: 8,
            expected_sha256: None,
            custom_headers: None,
            user_agent: None,
            referer: None,
            cookie: None,
            max_speed_bps: 0,
            retry_count: 0,
            max_retries: 3,
            error_message: None,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        db.insert_download(&record).await.unwrap();

        let fetched = db.get_download("uuid-test-1").await.unwrap().unwrap();
        assert_eq!(fetched.filename, "test.iso");
        assert_eq!(fetched.status, "queued");

        db.update_status("uuid-test-1", "downloading", None).await.unwrap();
        db.update_progress("uuid-test-1", 524288, Some(1048576)).await.unwrap();

        let updated = db.get_download("uuid-test-1").await.unwrap().unwrap();
        assert_eq!(updated.status, "downloading");
        assert_eq!(updated.downloaded_bytes, 524288);

        let list = db.list_downloads(Some("downloading")).await.unwrap();
        assert_eq!(list.len(), 1);

        db.delete_download("uuid-test-1").await.unwrap();
        assert!(db.get_download("uuid-test-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_settings_storage() {
        let db = Database::open_in_memory().unwrap();
        let max_conc = db.get_setting("max_concurrent_downloads").await.unwrap().unwrap();
        assert_eq!(max_conc, "3");

        db.set_setting("max_concurrent_downloads", "5").await.unwrap();
        let updated = db.get_setting("max_concurrent_downloads").await.unwrap().unwrap();
        assert_eq!(updated, "5");
    }
}

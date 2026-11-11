use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Invalid byte range: {start}..={end}")]
    InvalidRange { start: u64, end: u64 },

    #[error("Server returned HTTP {status}: {message}")]
    ServerFailed { status: u16, message: String },

    #[error("Checksum mismatch! Expected: {expected}, Actual: {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("State file corrupted: {0}")]
    StateCorrupted(String),

    #[error("Download was cancelled or paused")]
    Cancelled,

    #[error("Download has already completed")]
    AlreadyCompleted,

    #[error("Server did not report content length and range requests were requested")]
    NoContentLength,

    #[error("Resume error: {0}")]
    ResumeError(String),
}

pub type Result<T> = std::result::Result<T, DownloadError>;

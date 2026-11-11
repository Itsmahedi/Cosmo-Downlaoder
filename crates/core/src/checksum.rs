use std::path::Path;
use sha2::{Digest, Sha256};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::error::{DownloadError, Result};

/// Computes the SHA-256 hash of a file on disk using buffered streaming.
pub async fn compute_sha256(file_path: &Path) -> Result<String> {
    let mut file = File::open(file_path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Verifies that a file's SHA-256 matches an expected hexadecimal string.
pub async fn verify_sha256(file_path: &Path, expected_hex: &str) -> Result<bool> {
    let actual_hash = compute_sha256(file_path).await?;
    let expected_clean = expected_hex.trim().to_lowercase();
    let actual_clean = actual_hash.trim().to_lowercase();

    if actual_clean == expected_clean {
        Ok(true)
    } else {
        Err(DownloadError::ChecksumMismatch {
            expected: expected_clean,
            actual: actual_clean,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_compute_and_verify_sha256() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        let mut file = File::create(path).await.unwrap();
        file.write_all(b"Hello, RustGet!").await.unwrap();
        file.flush().await.unwrap();
        drop(file);

        // Expected hash for "Hello, RustGet!"
        let mut hasher = Sha256::new();
        hasher.update(b"Hello, RustGet!");
        let expected = format!("{:x}", hasher.finalize());

        let actual = compute_sha256(path).await.unwrap();
        assert_eq!(actual, expected);

        let verified = verify_sha256(path, &expected).await.unwrap();
        assert!(verified);

        let mismatch = verify_sha256(path, "0000000000000000000000000000000000000000000000000000000000000000").await;
        assert!(mismatch.is_err());
    }
}

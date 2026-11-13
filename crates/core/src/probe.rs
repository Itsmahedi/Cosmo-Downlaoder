use percent_encoding::percent_decode_str;
use reqwest::header::{HeaderMap, ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG, LAST_MODIFIED, RANGE};
use reqwest::Client;
use tracing::{debug, warn};
use url::Url;

use crate::error::{DownloadError, Result};

/// Validates that a URL string is a well-formed HTTP or HTTPS URL with a host.
pub fn validate_http_url(raw: &str) -> Result<Url> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(DownloadError::InvalidUrl("URL cannot be empty".to_string()));
    }
    let parsed = Url::parse(trimmed)
        .map_err(|e| DownloadError::InvalidUrl(format!("Invalid URL '{}': {}", trimmed, e)))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(DownloadError::InvalidUrl(format!(
                "Unsupported URL scheme '{}': only http:// and https:// are allowed",
                other
            )));
        }
    }
    if parsed.host_str().map(|h| h.is_empty()).unwrap_or(true) {
        return Err(DownloadError::InvalidUrl("URL must contain a valid host".to_string()));
    }
    Ok(parsed)
}

/// Redacts credentials and query string tokens before writing a URL to logs.
pub fn redact_url_for_log(raw: &str) -> String {
    match Url::parse(raw) {
        Ok(mut u) => {
            if !u.username().is_empty() || u.password().is_some() {
                let _ = u.set_username("***");
                let _ = u.set_password(Some("***"));
            }
            if u.query().is_some() {
                u.set_query(Some("***"));
            }
            u.to_string()
        }
        Err(_) => "<invalid-url>".to_string(),
    }
}

/// Information obtained from probing a remote URL.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// The canonical URL after redirects.
    pub url: Url,
    /// Total size in bytes if reported by the server.
    pub content_length: Option<u64>,
    /// Whether the server supports HTTP Range requests.
    pub accepts_ranges: bool,
    /// Server ETag if provided.
    pub etag: Option<String>,
    /// Server Last-Modified timestamp string if provided.
    pub last_modified: Option<String>,
    /// Best filename determined from Content-Disposition, URL path, or default.
    pub suggested_filename: String,
    /// MIME type if reported.
    pub mime_type: Option<String>,
}

/// Probes a remote URL using HEAD (with fallback to Range GET) to determine file characteristics.
pub async fn probe_url(client: &Client, url: &Url, custom_headers: &HeaderMap) -> Result<FileInfo> {
    debug!("Probing URL via HEAD: {}", redact_url_for_log(url.as_str()));

    let head_result = client
        .head(url.clone())
        .headers(custom_headers.clone())
        .send()
        .await;

    match head_result {
        Ok(resp) if resp.status().is_success() => {
            let final_url = resp.url().clone();
            let headers = resp.headers();

            let accepts_ranges = headers
                .get(ACCEPT_RANGES)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.eq_ignore_ascii_case("bytes"))
                .unwrap_or(false);

            let content_length = headers
                .get(CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());

            let etag = headers
                .get(ETAG)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim().to_string());

            let last_modified = headers
                .get(LAST_MODIFIED)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim().to_string());

            let mime_type = headers
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(';').next().unwrap_or(s).trim().to_string());

            let suggested_filename = extract_filename(headers, &final_url);

            Ok(FileInfo {
                url: final_url,
                content_length,
                accepts_ranges,
                etag,
                last_modified,
                suggested_filename,
                mime_type,
            })
        }
        Ok(resp) if resp.status().as_u16() == 405 || resp.status().as_u16() == 403 || resp.status().as_u16() == 501 => {
            debug!("HEAD returned {}, falling back to Range GET 0-0", resp.status());
            probe_via_range_get(client, url, custom_headers).await
        }
        Ok(resp) => {
            warn!("HEAD request failed with status: {}", resp.status());
            // Try Range GET as fallback before giving up
            probe_via_range_get(client, url, custom_headers).await
        }
        Err(e) => {
            debug!("HEAD request error ({:?}), falling back to Range GET", e);
            probe_via_range_get(client, url, custom_headers).await
        }
    }
}

/// Fallback probe using GET with Range: bytes=0-0 to test both range support and content length.
async fn probe_via_range_get(client: &Client, url: &Url, custom_headers: &HeaderMap) -> Result<FileInfo> {
    let mut headers = custom_headers.clone();
    headers.insert(RANGE, "bytes=0-0".parse().map_err(|_| DownloadError::InvalidUrl(url.to_string()))?);

    let resp = client
        .get(url.clone())
        .headers(headers)
        .send()
        .await?;

    let final_url = resp.url().clone();
    let status = resp.status();
    let resp_headers = resp.headers();

    if !status.is_success() && status.as_u16() != 206 {
        return Err(DownloadError::ServerFailed {
            status: status.as_u16(),
            message: format!("HTTP request failed with status {}", status),
        });
    }

    let is_partial = status.as_u16() == 206;
    let mut accepts_ranges = is_partial;
    let mut total_length: Option<u64> = None;

    if let Some(range_val) = resp_headers.get(CONTENT_RANGE).and_then(|v| v.to_str().ok()) {
        // Format: bytes 0-0/12345 or bytes */12345
        accepts_ranges = true;
        if let Some(slash_idx) = range_val.rfind('/') {
            let total_str = &range_val[slash_idx + 1..].trim();
            if *total_str != "*" {
                total_length = total_str.parse::<u64>().ok();
            }
        }
    }

    if total_length.is_none() {
        // Fall back to Content-Length
        if !is_partial {
            total_length = resp_headers
                .get(CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
        }
    }

    if !accepts_ranges {
        accepts_ranges = resp_headers
            .get(ACCEPT_RANGES)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case("bytes"))
            .unwrap_or(false);
    }

    let etag = resp_headers
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string());

    let last_modified = resp_headers
        .get(LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string());

    let mime_type = resp_headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or(s).trim().to_string());

    let suggested_filename = extract_filename(resp_headers, &final_url);

    Ok(FileInfo {
        url: final_url,
        content_length: total_length,
        accepts_ranges,
        etag,
        last_modified,
        suggested_filename,
        mime_type,
    })
}

/// Extracts a clean, sanitized filename from Content-Disposition or the URL path.
/// Uses the MIME type from the Content-Type header to infer the correct extension
/// when the filename extracted from the URL has no extension.
pub fn extract_filename(headers: &HeaderMap, url: &Url) -> String {
    // Extract MIME type early so we can use it as a fallback for extension detection.
    let mime_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or(s).trim().to_lowercase());

    // 0. Detect YouTube and streaming platform URLs directly
    let url_str = url.as_str().to_ascii_lowercase();
    if url_str.contains("youtube.com/") || url_str.contains("youtu.be/") || url_str.contains("vimeo.com/") {
        if let Some(v_id) = url.query_pairs().find(|(k, _)| k == "v").map(|(_, v)| v.to_string()) {
            if !v_id.is_empty() {
                return format!("YouTube_{}.mp4", sanitize_filename(&v_id));
            }
        }
        return "YouTube_Video.mp4".to_string();
    }

    // 1. Try Content-Disposition
    if let Some(disp) = headers.get(CONTENT_DISPOSITION).and_then(|v| v.to_str().ok()) {
        if let Some(name) = parse_content_disposition(disp) {
            let sanitized = sanitize_filename(&name);
            if !sanitized.is_empty() {
                return sanitized;
            }
        }
    }

    // 2. Try URL path
    if let Some(segments) = url.path_segments() {
        for seg in segments.rev() {
            let decoded = percent_decode_str(seg).decode_utf8_lossy().to_string();
            let sanitized = sanitize_filename(&decoded);
            if !sanitized.is_empty() && sanitized != "." && sanitized != ".." {
                // Skip generic URL path segments that aren't real filenames
                let lower = sanitized.to_ascii_lowercase();
                if lower == "watch" || lower == "video" || lower == "view"
                    || lower == "download" || lower == "export" || lower == "uc"
                    || lower == "file" || lower == "d" || lower == "open"
                {
                    continue;
                }

                if sanitized.contains('.') {
                    return sanitized;
                }

                // No extension in URL segment — infer from MIME type
                let ext = mime_to_extension(mime_type.as_deref());
                return format!("{}.{}", sanitized, ext);
            }
        }
    }

    // 3. Fallback: use MIME-based extension for the default filename
    let ext = mime_to_extension(mime_type.as_deref());
    format!("download.{}", ext)
}

/// Maps a MIME type string to a file extension.
pub fn mime_to_extension(mime: Option<&str>) -> &'static str {
    match mime {
        // Images
        Some("image/png") => "png",
        Some("image/jpeg") | Some("image/jpg") => "jpg",
        Some("image/gif") => "gif",
        Some("image/webp") => "webp",
        Some("image/svg+xml") => "svg",
        Some("image/bmp") => "bmp",
        Some("image/tiff") => "tiff",
        Some("image/x-icon") | Some("image/vnd.microsoft.icon") => "ico",
        Some("image/avif") => "avif",
        // Video
        Some("video/mp4") => "mp4",
        Some("video/webm") => "webm",
        Some("video/x-matroska") => "mkv",
        Some("video/x-msvideo") => "avi",
        Some("video/quicktime") => "mov",
        Some("video/x-flv") => "flv",
        Some("video/mp2t") | Some("video/mpeg") => "ts",
        // Audio
        Some("audio/mpeg") => "mp3",
        Some("audio/ogg") => "ogg",
        Some("audio/wav") | Some("audio/x-wav") => "wav",
        Some("audio/flac") | Some("audio/x-flac") => "flac",
        Some("audio/aac") => "aac",
        Some("audio/mp4") => "m4a",
        Some("audio/webm") => "weba",
        // Documents
        Some("application/pdf") => "pdf",
        Some("application/zip") => "zip",
        Some("application/x-7z-compressed") => "7z",
        Some("application/x-rar-compressed") | Some("application/vnd.rar") => "rar",
        Some("application/gzip") | Some("application/x-gzip") => "gz",
        Some("application/x-tar") => "tar",
        Some("application/x-bzip2") => "bz2",
        Some("application/x-xz") => "xz",
        Some("application/json") => "json",
        Some("application/xml") | Some("text/xml") => "xml",
        Some("text/plain") => "txt",
        Some("text/html") => "html",
        Some("text/css") => "css",
        Some("text/javascript") | Some("application/javascript") => "js",
        Some("application/x-iso9660-image") => "iso",
        Some("application/vnd.microsoft.portable-executable") | Some("application/x-msdownload") => "exe",
        Some("application/vnd.debian.binary-package") | Some("application/x-deb") => "deb",
        Some("application/x-rpm") => "rpm",
        Some("application/x-apple-diskimage") => "dmg",
        Some("application/vnd.android.package-archive") => "apk",
        // MS Office
        Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document") => "docx",
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet") => "xlsx",
        Some("application/vnd.openxmlformats-officedocument.presentationml.presentation") => "pptx",
        Some("application/msword") => "doc",
        Some("application/vnd.ms-excel") => "xls",
        // Fallback
        Some(s) if s.starts_with("image/") => "bin",
        Some(s) if s.starts_with("video/") => "mp4",
        Some(s) if s.starts_with("audio/") => "mp3",
        _ => "bin",
    }
}

/// Parses RFC 6266 / RFC 5987 Content-Disposition header (case-insensitive parameter names).
fn parse_content_disposition(header: &str) -> Option<String> {
    // Look for filename*=UTF-8''encoded_name
    for part in header.split(';') {
        let trimmed = part.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("filename*=") {
            let val = trimmed["filename*=".len()..].trim().trim_matches('"');
            let lower_val = val.to_ascii_lowercase();
            if lower_val.starts_with("utf-8''") {
                let encoded = &val["utf-8''".len()..];
                if let Ok(decoded) = percent_decode_str(encoded).decode_utf8() {
                    let leaf = decoded.rsplit(['/', '\\']).next().unwrap_or(&decoded);
                    if !leaf.trim().is_empty() {
                        return Some(leaf.to_string());
                    }
                }
            }
        }
    }

    // Look for filename="name" or filename=name
    for part in header.split(';') {
        let trimmed = part.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("filename=") {
            let val = trimmed["filename=".len()..].trim().trim_matches('"').trim();
            let leaf = val.rsplit(['/', '\\']).next().unwrap_or(val).trim();
            if !leaf.is_empty() {
                return Some(leaf.to_string());
            }
        }
    }

    None
}

/// Returns true if `stem` (before any extension) matches a Windows reserved DOS device name.
fn is_windows_reserved_name(stem: &str) -> bool {
    let upper = stem.trim().to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM0"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT0"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

/// Replaces illegal Windows characters (< > : " / \ | ? *) with look-alikes (＜ ＞ ： ＂ ⧸ ⧹ ｜ ？ ＊),
/// preserves Unicode letters/digits and [ ] ( ) , -, collapses repeated spaces, trims trailing dots/spaces,
/// prefixes Windows reserved names with '_', and cuts length so full path stays under 250 characters.
pub fn sanitize_filename(name: &str) -> String {
    let mut mapped = String::with_capacity(name.len());
    let mut prev_space = false;

    for c in name.chars() {
        if c.is_control() {
            if !prev_space {
                mapped.push(' ');
                prev_space = true;
            }
            continue;
        }

        let replacement = match c {
            '<' => Some('＜'), // U+FF1C
            '>' => Some('＞'), // U+FF1E
            ':' => Some('：'), // U+FF1A
            '"' => Some('＂'), // U+FF02
            '/' => Some('⧸'), // U+29F8
            '\\' => Some('⧹'), // U+29F9
            '|' => Some('｜'), // U+FF5C
            '?' => Some('？'), // U+FF1F
            '*' => Some('＊'), // U+FF0A
            _ => None,
        };

        if let Some(rep) = replacement {
            mapped.push(rep);
            prev_space = false;
        } else if c.is_whitespace() {
            if !prev_space {
                mapped.push(' ');
                prev_space = true;
            }
        } else {
            mapped.push(c);
            prev_space = false;
        }
    }

    let trimmed = mapped
        .trim_matches(|c: char| c.is_whitespace() || c == '.')
        .trim();
    if trimmed.is_empty() {
        return "download.bin".to_string();
    }

    // Protect Windows reserved device names (e.g. CON, NUL, COM1.log, etc.)
    let stem = trimmed.split('.').next().unwrap_or(trimmed);
    let safe = if is_windows_reserved_name(stem) {
        format!("_{}", trimmed)
    } else {
        trimmed.to_string()
    };

    // Enforce safe filename length (< 180 chars) to ensure full path stays well under 250 chars
    if safe.chars().count() <= 180 {
        return safe;
    }

    if let Some(dot_idx) = safe.rfind('.') {
        let ext = &safe[dot_idx..];
        if ext.chars().count() <= 20 {
            let max_stem = 180usize.saturating_sub(ext.chars().count());
            let truncated_stem: String = safe[..dot_idx].chars().take(max_stem).collect();
            let result = format!("{}{}", truncated_stem.trim_end_matches([' ', '.']), ext);
            return if result.is_empty() {
                "download.bin".to_string()
            } else {
                result
            };
        }
    }

    safe.chars().take(180).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_disposition_standard() {
        let header = "attachment; filename=\"ubuntu-24.04-desktop.iso\"";
        assert_eq!(
            parse_content_disposition(header),
            Some("ubuntu-24.04-desktop.iso".to_string())
        );
    }

    #[test]
    fn test_parse_content_disposition_rfc5987() {
        let header = "attachment; filename*=UTF-8''my%20test%20file.pdf";
        assert_eq!(
            parse_content_disposition(header),
            Some("my test file.pdf".to_string())
        );
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("valid_name.zip"), "valid_name.zip");
        assert_eq!(sanitize_filename("bad/name:with*chars?.txt"), "bad⧸name：with＊chars？.txt");
        assert_eq!(
            sanitize_filename("[2] Famous Alfaaz Video (Full) - William Gamborg | Surtaal Music Academy"),
            "[2] Famous Alfaaz Video (Full) - William Gamborg ｜ Surtaal Music Academy"
        );
        assert_eq!(
            sanitize_filename("আমার সোনার বাংলা - পর্ব ১"),
            "আমার সোনার বাংলা - পর্ব ১"
        );
        assert_eq!(sanitize_filename("NUL.mp4"), "_NUL.mp4");
        assert_eq!(sanitize_filename("NUL"), "_NUL");
        assert_eq!(sanitize_filename(".."), "download.bin");
        assert_eq!(sanitize_filename(""), "download.bin");
    }

    #[test]
    fn test_extract_filename_from_url() {
        let url = Url::parse("https://example.com/files/archive%202026.tar.gz?token=abc").unwrap();
        let headers = HeaderMap::new();
        assert_eq!(extract_filename(&headers, &url), "archive 2026.tar.gz");
    }

    #[test]
    fn test_extract_filename_google_drive_image() {
        // Google Drive URLs have paths like /file/d/ID/view — no real filename
        let url = Url::parse("https://drive.google.com/file/d/1aBcDeFgHiJkLmNoPqRs/view").unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "image/png".parse().unwrap());
        let result = extract_filename(&headers, &url);
        assert!(result.ends_with(".png"), "Expected .png extension, got: {}", result);
    }

    #[test]
    fn test_extract_filename_no_extension_with_mime() {
        let url = Url::parse("https://example.com/somefile").unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "image/jpeg".parse().unwrap());
        assert_eq!(extract_filename(&headers, &url), "somefile.jpg");
    }

    #[test]
    fn test_extract_filename_fallback_with_mime() {
        // All segments are generic — should use MIME for default filename extension
        let url = Url::parse("https://example.com/download").unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/pdf".parse().unwrap());
        assert_eq!(extract_filename(&headers, &url), "download.pdf");
    }

    #[test]
    fn test_mime_to_extension() {
        assert_eq!(mime_to_extension(Some("image/png")), "png");
        assert_eq!(mime_to_extension(Some("image/jpeg")), "jpg");
        assert_eq!(mime_to_extension(Some("video/mp4")), "mp4");
        assert_eq!(mime_to_extension(Some("application/pdf")), "pdf");
        assert_eq!(mime_to_extension(Some("application/zip")), "zip");
        assert_eq!(mime_to_extension(None), "bin");
    }
}

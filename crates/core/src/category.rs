use std::path::{Path, PathBuf};
use directories::UserDirs;
use serde::{Deserialize, Serialize};

/// Categories for downloaded files with predefined extension sets and target directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Video,
    Music,
    Documents,
    Compressed,
    Programs,
    General,
}

impl Category {
    /// Detect the category based on the file extension.
    pub fn detect_from_filename(filename: &str) -> Self {
        let path = Path::new(filename);
        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e.to_ascii_lowercase(),
            None => return Category::General,
        };

        match ext.as_str() {
            "mp4" | "mkv" | "avi" | "mov" | "flv" | "wmv" | "webm" | "m4v" | "3gp" | "ts" => {
                Category::Video
            }
            "mp3" | "flac" | "wav" | "aac" | "ogg" | "m4a" | "wma" | "opus" | "alac" => {
                Category::Music
            }
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "csv" | "epub"
            | "rtf" | "odt" | "ods" | "md" => Category::Documents,
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso" | "dmg" | "tgz" => {
                Category::Compressed
            }
            "exe" | "msi" | "bat" | "cmd" | "appx" | "msix" => Category::Programs,
            _ => Category::General,
        }
    }

    /// Display name of the category.
    pub fn name(&self) -> &'static str {
        match self {
            Category::Video => "Video",
            Category::Music => "Music",
            Category::Documents => "Documents",
            Category::Compressed => "Compressed",
            Category::Programs => "Programs",
            Category::General => "General",
        }
    }

    /// Resolve the default download directory for this category on Windows.
    pub fn default_dir(&self) -> PathBuf {
        let user_dirs = UserDirs::new();
        let base_download_dir = user_dirs
            .as_ref()
            .and_then(|u| u.download_dir().map(Path::to_path_buf))
            .unwrap_or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(|p| PathBuf::from(p).join("Downloads"))
                    .unwrap_or_else(|| PathBuf::from("Downloads"))
            });

        match self {
            Category::Video => user_dirs
                .as_ref()
                .and_then(|u| u.video_dir().map(Path::to_path_buf))
                .unwrap_or_else(|| base_download_dir.join("Video")),
            Category::Music => user_dirs
                .as_ref()
                .and_then(|u| u.audio_dir().map(Path::to_path_buf))
                .unwrap_or_else(|| base_download_dir.join("Music")),
            Category::Documents => user_dirs
                .as_ref()
                .and_then(|u| u.document_dir().map(Path::to_path_buf))
                .unwrap_or_else(|| base_download_dir.join("Documents")),
            Category::Compressed => base_download_dir.join("Compressed"),
            Category::Programs => base_download_dir.join("Programs"),
            Category::General => base_download_dir,
        }
    }

    /// Parse a category name string.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "video" | "videos" => Some(Category::Video),
            "music" | "audio" => Some(Category::Music),
            "documents" | "document" | "docs" => Some(Category::Documents),
            "compressed" | "archives" | "archive" => Some(Category::Compressed),
            "programs" | "program" | "apps" => Some(Category::Programs),
            "general" | "other" => Some(Category::General),
            _ => None,
        }
    }
}

/// Canonical unified categorization function based on filename and optional MIME type.
pub fn categorize(filename: Option<&str>, mime: Option<&str>) -> Category {
    // 1. Try filename extension first
    if let Some(f) = filename {
        let cat = Category::detect_from_filename(f);
        if cat != Category::General {
            return cat;
        }
    }

    // 2. Try MIME type
    if let Some(m) = mime {
        let m_lower = m.trim().to_ascii_lowercase();
        if m_lower.starts_with("video/") {
            return Category::Video;
        } else if m_lower.starts_with("audio/") {
            return Category::Music;
        } else if m_lower.contains("pdf")
            || m_lower.contains("document")
            || m_lower.contains("msword")
            || m_lower.contains("spreadsheet")
            || m_lower.contains("presentation")
            || m_lower.starts_with("text/")
        {
            return Category::Documents;
        } else if m_lower.contains("zip")
            || m_lower.contains("tar")
            || m_lower.contains("compressed")
            || m_lower.contains("gzip")
            || m_lower.contains("7z")
            || m_lower.contains("rar")
        {
            return Category::Compressed;
        } else if m_lower.contains("application/x-msdownload")
            || m_lower.contains("application/x-executable")
            || m_lower.contains("application/vnd.microsoft.portable-executable")
            || m_lower.contains("application/x-msdos-program")
        {
            return Category::Programs;
        }
    }

    Category::General
}

/// Category filter enum for UI navigation and querying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CategoryFilter {
    All,
    Video,
    Music,
    Documents,
    Compressed,
    Programs,
    Other,
}

impl CategoryFilter {
    /// Human-friendly display name of the category filter.
    pub fn name(&self) -> &'static str {
        match self {
            Self::All => "All Categories",
            Self::Video => "Video",
            Self::Music => "Music",
            Self::Documents => "Documents",
            Self::Compressed => "Compressed",
            Self::Programs => "Programs",
            Self::Other => "Other",
        }
    }

    /// Check whether a category string (e.g. from DownloadRecord) matches this filter.
    pub fn matches(&self, category_str: &str) -> bool {
        match self {
            Self::All => true,
            Self::Video => category_str.eq_ignore_ascii_case("Video"),
            Self::Music => category_str.eq_ignore_ascii_case("Music"),
            Self::Documents => category_str.eq_ignore_ascii_case("Documents"),
            Self::Compressed => category_str.eq_ignore_ascii_case("Compressed"),
            Self::Programs => category_str.eq_ignore_ascii_case("Programs"),
            Self::Other => {
                let s = category_str.trim().to_ascii_lowercase();
                s == "general" || s == "other" || (!["video", "music", "documents", "compressed", "programs"].contains(&s.as_str()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_detection() {
        assert_eq!(Category::detect_from_filename("movie.mp4"), Category::Video);
        assert_eq!(Category::detect_from_filename("TRACK.FLAC"), Category::Music);
        assert_eq!(Category::detect_from_filename("report.final.pdf"), Category::Documents);
        assert_eq!(Category::detect_from_filename("backup.tar.gz"), Category::Compressed);
        assert_eq!(Category::detect_from_filename("installer.exe"), Category::Programs);
        assert_eq!(Category::detect_from_filename("unknown.xyz"), Category::General);
        assert_eq!(Category::detect_from_filename("no_extension"), Category::General);
    }

    #[test]
    fn test_categorize_unified_function() {
        // Filename priority
        assert_eq!(categorize(Some("test.mp4"), Some("audio/mpeg")), Category::Video);
        assert_eq!(categorize(Some("song.mp3"), None), Category::Music);
        assert_eq!(categorize(Some("archive.zip"), None), Category::Compressed);
        assert_eq!(categorize(Some("setup.msi"), None), Category::Programs);
        assert_eq!(categorize(Some("document.docx"), None), Category::Documents);

        // MIME fallback when filename lacks known extension
        assert_eq!(categorize(Some("download"), Some("video/mp4")), Category::Video);
        assert_eq!(categorize(Some("stream"), Some("audio/ogg")), Category::Music);
        assert_eq!(categorize(None, Some("application/pdf")), Category::Documents);
        assert_eq!(categorize(None, Some("application/zip")), Category::Compressed);
        assert_eq!(categorize(Some("binary"), Some("application/x-msdownload")), Category::Programs);

        // Unknown
        assert_eq!(categorize(Some("file.xyz"), Some("application/octet-stream")), Category::General);
        assert_eq!(categorize(None, None), Category::General);
    }

    #[test]
    fn test_category_directories() {
        for cat in [
            Category::Video,
            Category::Music,
            Category::Documents,
            Category::Compressed,
            Category::Programs,
            Category::General,
        ] {
            let dir = cat.default_dir();
            assert!(!dir.as_os_str().is_empty());
        }
    }

    #[test]
    fn test_category_filter_matches() {
        assert!(CategoryFilter::All.matches("Video"));
        assert!(CategoryFilter::All.matches("Anything"));

        assert!(CategoryFilter::Video.matches("Video"));
        assert!(CategoryFilter::Video.matches("video"));
        assert!(!CategoryFilter::Video.matches("Music"));

        assert!(CategoryFilter::Music.matches("Music"));
        assert!(CategoryFilter::Music.matches("music"));
        assert!(!CategoryFilter::Music.matches("Video"));

        assert!(CategoryFilter::Documents.matches("Documents"));
        assert!(CategoryFilter::Documents.matches("documents"));

        assert!(CategoryFilter::Compressed.matches("Compressed"));
        assert!(CategoryFilter::Compressed.matches("compressed"));

        assert!(CategoryFilter::Programs.matches("Programs"));
        assert!(CategoryFilter::Programs.matches("programs"));

        assert!(CategoryFilter::Other.matches("General"));
        assert!(CategoryFilter::Other.matches("other"));
        assert!(CategoryFilter::Other.matches("Unknown"));
        assert!(!CategoryFilter::Other.matches("Video"));
    }
}

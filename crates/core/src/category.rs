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
}

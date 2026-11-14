use rustget_core::probe::sanitize_filename;
use std::fs;

#[test]
fn test_unicode_filenames_on_disk() {
    let test_names = [
        "আমার সোনার বাংলা",
        "हिन्दी गाना",
        "مرحبا بالعالم",
        "日本語のタイトル",
        "한국어 제목",
        "中文标题",
    ];

    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");

    for base in test_names {
        // Step A: verify code points
        let codepoints: Vec<String> = base.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        println!("Checking '{}' -> {}", base, codepoints.join(" "));

        let filename = format!("{}.txt", base);
        let sanitized = sanitize_filename(&filename);
        assert_eq!(sanitized, filename);

        let file_path = temp_dir.path().join(&sanitized);
        fs::write(&file_path, b"test content").expect("Failed to write unicode file");

        assert!(file_path.exists());
        let read_back = fs::read_to_string(&file_path).expect("Failed to read unicode file");
        assert_eq!(read_back, "test content");

        // Verify directory entry matches original unicode string
        let entries: Vec<_> = fs::read_dir(temp_dir.path())
            .expect("read_dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(entries.contains(&sanitized));

        fs::remove_file(&file_path).expect("Failed to remove file");
    }
}

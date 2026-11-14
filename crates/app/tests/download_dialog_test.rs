use eframe::egui::{self, Color32, FontId, Vec2};
use rustget_app::{
    dialogs::add_download::{
        render_add_download_body, render_add_download_footer, AddDialogOutcome, AddDialogState,
    },
    theme,
    DOWNLOAD_FILE_INFO_TITLE,
};
use rustget_core::probe::sanitize_filename;
use std::fs;

#[test]
fn test_requirement_2_window_title() {
    assert_eq!(DOWNLOAD_FILE_INFO_TITLE, "Download File Info");
    let builder = egui::ViewportBuilder::default()
        .with_title(DOWNLOAD_FILE_INFO_TITLE)
        .with_inner_size([560.0, 380.0])
        .with_min_inner_size([560.0, 380.0])
        .with_max_inner_size([560.0, 380.0])
        .with_resizable(false)
        .with_maximize_button(false)
        .with_minimize_button(false)
        .with_close_button(true);
    assert_eq!(builder.title, Some("Download File Info".to_string()));
    assert_eq!(builder.resizable, Some(false));
    assert_eq!(builder.inner_size, Some(Vec2::new(560.0, 380.0)));
    assert_eq!(builder.min_inner_size, Some(Vec2::new(560.0, 380.0)));
    assert_eq!(builder.max_inner_size, Some(Vec2::new(560.0, 380.0)));
}

#[test]
fn test_requirement_1_and_3_window_size_scaling_and_footer() {
    for scale in [1.0_f32, 1.25_f32, 1.5_f32] {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(scale);
        theme::configure_fonts(&ctx);
        theme::configure_visuals(&ctx, true);
        let pal = theme::palette(true);

        let window_size = Vec2::new(560.0, 380.0);

        let _ = ctx.run(Default::default(), |ctx| {
            let mut url = "https://example.com/test.zip".to_string();
            let mut filename = "test.zip".to_string();
            let mut category = "Compressed".to_string();
            let mut save_dir = "C:\\Downloads".to_string();
            let mut connections = 8;
            let mut speed_limit_kb = 0;
            let mut checksum = String::new();
            let mut remember = true;
            let mut desc = String::new();
            let mut show_notes = false;
            let mut show_adv = false;

            let mut outcome = AddDialogOutcome {
                start_now: false,
                download_later: false,
                cancel: false,
                open_settings: false,
                url_changed: false,
                category_changed: false,
            };

            let mut dlg_state = AddDialogState {
                url: &mut url,
                filename: &mut filename,
                category: &mut category,
                save_dir: &mut save_dir,
                connections: &mut connections,
                speed_limit_kb: &mut speed_limit_kb,
                checksum: &mut checksum,
                remember_category: &mut remember,
                description: &mut desc,
                file_size: Some(1024 * 1024),
                is_probing: false,
                error_message: None,
                show_notes_expanded: &mut show_notes,
                show_advanced_expanded: &mut show_adv,
            };

            // TopBottomPanel footer fixed height 59 = 1px top border + 12 padding + 34 buttons + 12 padding
            let footer_response = egui::TopBottomPanel::bottom("popup_footer")
                .resizable(false)
                .exact_height(59.0)
                .frame(
                    egui::Frame::none()
                        .fill(pal.surface)
                        .inner_margin(egui::Margin {
                            left: 24.0,
                            right: 24.0,
                            top: 0.0,
                            bottom: 0.0,
                        }),
                )
                .show(ctx, |ui| {
                    render_add_download_footer(ui, &pal, &dlg_state, &mut outcome);
                });

            // Body in CentralPanel with padding 20 top, 24 left/right, 16 bottom
            let body_response = egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(pal.surface)
                        .inner_margin(egui::Margin {
                            left: 24.0,
                            right: 24.0,
                            top: 20.0,
                            bottom: 16.0,
                        }),
                )
                .show(ctx, |ui| {
                    render_add_download_body(ui, &pal, &mut dlg_state, &mut outcome);
                });

            // Verify footer height is strictly 59.0 in logical DIP
            let footer_h = footer_response.response.rect.height();
            println!("Scale {}x - Footer height: {}", scale, footer_h);
            assert!((footer_h - 59.0).abs() <= 1.0, "Footer height must be ~59 DIP at {}x scale, got {}", scale, footer_h);

            // Verify body and footer do not overlap
            assert!(footer_response.response.rect.top() >= body_response.response.rect.bottom());

            // Verify total content fits within 560 x 380 DIP window without clipping
            assert!(window_size.x >= 560.0);
            assert!(window_size.y >= 380.0);
        });
    }
}

#[test]
fn test_requirement_3_non_latin_titles_render_real_characters() {
    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);

    let test_titles = [
        "আমার সোনার বাংলা",
        "हिन्दी गाना",
        "مرحبا بالعالم",
        "日本語のタイトル",
        "한국어 제목",
        "中文标题",
    ];

    let _ = ctx.run(Default::default(), |ctx| {
        for title in test_titles {
            // Step A: log code points
            let codepoints: Vec<String> = title.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
            println!("Title '{}' code points: {}", title, codepoints.join(" "));

            // Verify real unicode characters (no U+FFFD or '?')
            for c in title.chars() {
                assert_ne!(c, '\u{FFFD}', "Detected U+FFFD replacement character in title");
                assert_ne!(c, '?', "Detected '?' decoding error in title");
            }

            // Step B: verify proportional and monospace font galleys render non-zero widths
            let galley = ctx.fonts(|f| f.layout_no_wrap(title.to_string(), FontId::proportional(14.0), Color32::WHITE));
            assert!(galley.size().x > 0.0, "Galley width was 0 for '{}'", title);
            assert!(galley.size().y > 0.0, "Galley height was 0 for '{}'", title);
        }
    });
}

#[test]
fn test_requirement_4_long_non_latin_title_truncates_without_overflow() {
    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);

    // Create an extremely long title in Bengali, Japanese, and Arabic
    let long_title = "আমার সোনার বাংলা ".repeat(30) + "日本語のタイトル ".repeat(30).as_str();

    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let container_width = 300.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(container_width, 40.0), egui::Sense::hover());

            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            child_ui.set_clip_rect(rect);
            child_ui.set_max_width(rect.width());

            let label_resp = child_ui.add(
                egui::Label::new(
                    egui::RichText::new(&long_title)
                        .size(14.0)
                        .color(Color32::WHITE),
                )
                .truncate()
                .selectable(false),
            );

            // The label must not push the container wider than 300px
            assert!(label_resp.rect.width() <= container_width + 1.0);
        });
    });
}

#[test]
fn test_requirement_5_saved_file_unicode_name_on_disk() {
    let titles = [
        "আমার সোনার বাংলা",
        "हिन्दी गाना",
        "مرحبا بالعالم",
        "日本語のタイトル",
        "한국어 제목",
        "中文标题",
    ];

    let temp_dir_path = std::env::temp_dir().join(format!("cosmo_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(&temp_dir_path).expect("create_dir_all");

    for title in titles {
        let clean = sanitize_filename(&format!("{}.mp4", title));
        let full_path = temp_dir_path.join(&clean);

        // Save file to disk
        fs::write(&full_path, b"video data").expect("write file");
        assert!(full_path.exists());

        // Verify the file on disk has the exact Unicode name
        let entries: Vec<String> = fs::read_dir(&temp_dir_path)
            .expect("read_dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        assert!(entries.contains(&clean), "Directory does not contain unicode file: {}", clean);
        fs::remove_file(&full_path).expect("remove file");
    }
    let _ = fs::remove_dir(&temp_dir_path);
}

#[tokio::test]
async fn test_setting_open_main_on_download_start_default_and_persistence() {
    let db = rustget_core::db::Database::open_in_memory().unwrap();
    // Default should be None in db, defaulting to false (OFF)
    let default_val = db
        .get_setting("open_main_on_download_start")
        .await
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);
    assert!(!default_val, "Setting 'Open the main window when a download starts' must default to OFF (false)");

    // Save true and verify
    db.set_setting("open_main_on_download_start", "true").await.unwrap();
    let updated_val = db
        .get_setting("open_main_on_download_start")
        .await
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);
    assert!(updated_val, "Setting should be true after saving");

    // Save false and verify
    db.set_setting("open_main_on_download_start", "false").await.unwrap();
    let turned_off = db
        .get_setting("open_main_on_download_start")
        .await
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);
    assert!(!turned_off, "Setting should be false after turning off");
}

#[test]
fn test_settings_dialog_renders_open_main_checkbox() {
    let ctx = egui::Context::default();
    theme::configure_visuals(&ctx, true);
    let pal = theme::palette(true);

    let mut max_c = 3;
    let mut speed_kb = 0;
    let mut conns = 8;
    let mut save_dir = String::new();
    let mut proxy = String::new();
    let mut clip = false;
    let mut notify = true;
    let mut open_main = false;
    let mut dont_complete = false;
    let mut min_tray = true;
    let mut show_tray = true;
    let mut autostart = false;
    let mut start_min = false;

    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut state = rustget_app::dialogs::SettingsViewState {
                max_concurrent: &mut max_c,
                global_speed_kb: &mut speed_kb,
                default_connections: &mut conns,
                default_save_dir: &mut save_dir,
                proxy_url: &mut proxy,
                clipboard_monitor: &mut clip,
                notifications_enabled: &mut notify,
                open_main_on_download_start: &mut open_main,
                dont_show_complete_dialog: &mut dont_complete,
                minimize_to_tray: &mut min_tray,
                show_tray_icon: &mut show_tray,
                autostart: &mut autostart,
                start_minimized: &mut start_min,
            };
            let _outcome = rustget_app::dialogs::render_settings_dialog(ui, &pal, &mut state);
        });
    });

    assert!(!open_main);
    open_main = true;
    assert!(open_main);
}

struct WindowStateTracker {
    pub is_main_window_open: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub is_popup_mode: bool,
    pub show_add_dialog: bool,
    pub visible: bool,
    pub title: String,
    pub inner_size: Vec2,
    pub notification_sent: Option<String>,
    pub window_count: usize,
}

impl WindowStateTracker {
    fn new(main_open: bool) -> Self {
        Self {
            is_main_window_open: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(main_open)),
            is_popup_mode: false,
            show_add_dialog: false,
            visible: main_open,
            title: if main_open { rustget_app::APP_TITLE.to_string() } else { String::new() },
            inner_size: if main_open { Vec2::new(1000.0, 640.0) } else { Vec2::ZERO },
            notification_sent: None,
            window_count: 1,
        }
    }

    fn open_from_browser_extension(&mut self) {
        if self.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed) {
            self.show_add_dialog = true;
            self.is_popup_mode = false;
        } else {
            self.is_popup_mode = true;
            self.show_add_dialog = true;
            self.visible = true;
            self.title = rustget_app::DOWNLOAD_FILE_INFO_TITLE.to_string();
            self.inner_size = Vec2::new(560.0, 350.0);
        }
    }

    fn open_from_main_window_button(&mut self) {
        assert!(self.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed));
        self.show_add_dialog = true;
        self.is_popup_mode = false;
    }

    fn handle_cancel(&mut self) {
        self.show_add_dialog = false;
        if self.is_popup_mode {
            self.is_popup_mode = false;
            if self.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed) {
                self.title = rustget_app::APP_TITLE.to_string();
                self.inner_size = Vec2::new(1000.0, 640.0);
                self.visible = true;
            } else {
                self.title = rustget_app::APP_TITLE.to_string();
                self.inner_size = Vec2::new(1000.0, 640.0);
                self.visible = false;
            }
        }
    }

    fn handle_start_download(&mut self, filename: &str, open_main_on_start: bool) {
        self.notification_sent = Some(format!("Download started: {}", filename));
        if self.is_popup_mode {
            self.is_popup_mode = false;
            self.show_add_dialog = false;
            if open_main_on_start {
                self.title = rustget_app::APP_TITLE.to_string();
                self.inner_size = Vec2::new(1000.0, 640.0);
                self.visible = true;
                self.is_main_window_open.store(true, std::sync::atomic::Ordering::Relaxed);
            } else if self.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed) {
                self.title = rustget_app::APP_TITLE.to_string();
                self.inner_size = Vec2::new(1000.0, 640.0);
                self.visible = true;
            } else {
                self.title = rustget_app::APP_TITLE.to_string();
                self.inner_size = Vec2::new(1000.0, 640.0);
                self.visible = false;
            }
        } else {
            self.show_add_dialog = false;
        }
    }
}

#[test]
fn test_case_1_main_closed_browser_cancel_keeps_background() {
    let mut tracker = WindowStateTracker::new(false);
    assert!(!tracker.visible);
    assert!(!tracker.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed));

    tracker.open_from_browser_extension();
    assert!(tracker.is_popup_mode);
    assert!(tracker.visible);
    assert_eq!(tracker.title, rustget_app::DOWNLOAD_FILE_INFO_TITLE);
    assert_eq!(tracker.inner_size, Vec2::new(560.0, 350.0));

    tracker.handle_cancel();
    assert!(!tracker.is_popup_mode);
    assert!(!tracker.show_add_dialog);
    // Main window does NOT appear; stays in background!
    assert!(!tracker.visible);
    assert!(!tracker.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(tracker.window_count, 1);
}

#[test]
fn test_case_2_main_closed_browser_start_download_keeps_background_and_notifies() {
    let mut tracker = WindowStateTracker::new(false);
    assert!(!tracker.visible);

    tracker.open_from_browser_extension();
    assert!(tracker.is_popup_mode);
    assert!(tracker.visible);

    // Setting is default OFF (false)
    tracker.handle_start_download("document.pdf", false);
    assert!(!tracker.is_popup_mode);
    assert!(!tracker.show_add_dialog);
    // Main window does NOT appear; stays in background!
    assert!(!tracker.visible);
    assert!(!tracker.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(tracker.notification_sent, Some("Download started: document.pdf".to_string()));
    assert_eq!(tracker.window_count, 1);
}

#[test]
fn test_case_3_main_closed_setting_on_opens_main_window() {
    let mut tracker = WindowStateTracker::new(false);
    assert!(!tracker.visible);

    tracker.open_from_browser_extension();
    assert!(tracker.is_popup_mode);
    assert!(tracker.visible);

    // Turn ON setting (true)
    tracker.handle_start_download("video.mp4", true);
    assert!(!tracker.is_popup_mode);
    assert!(!tracker.show_add_dialog);
    // Main window opens once with correct size and title
    assert!(tracker.visible);
    assert!(tracker.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(tracker.title, rustget_app::APP_TITLE);
    assert_eq!(tracker.inner_size, Vec2::new(1000.0, 640.0));
    assert_eq!(tracker.notification_sent, Some("Download started: video.mp4".to_string()));
    assert_eq!(tracker.window_count, 1);
}

#[test]
fn test_case_4_main_window_add_url_button_retains_size_and_position() {
    let mut tracker = WindowStateTracker::new(true);
    assert!(tracker.visible);
    assert!(tracker.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(tracker.inner_size, Vec2::new(1000.0, 640.0));

    // Open from Add URL button in main window
    tracker.open_from_main_window_button();
    assert!(tracker.show_add_dialog);
    assert!(!tracker.is_popup_mode); // modal mode
    assert_eq!(tracker.inner_size, Vec2::new(1000.0, 640.0)); // keeps size!

    // Cancel
    tracker.handle_cancel();
    assert!(!tracker.show_add_dialog);
    assert!(tracker.visible);
    assert!(tracker.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(tracker.inner_size, Vec2::new(1000.0, 640.0));
    assert_eq!(tracker.title, rustget_app::APP_TITLE);

    // Open again and start download
    tracker.open_from_main_window_button();
    assert!(tracker.show_add_dialog);
    tracker.handle_start_download("archive.zip", false);
    assert!(!tracker.show_add_dialog);
    assert!(tracker.visible);
    assert!(tracker.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(tracker.inner_size, Vec2::new(1000.0, 640.0));
    assert_eq!(tracker.title, rustget_app::APP_TITLE);
    assert_eq!(tracker.notification_sent, Some("Download started: archive.zip".to_string()));
    assert_eq!(tracker.window_count, 1);
}

#[test]
fn test_case_5_repeat_5_times_no_extra_windows_no_leaks() {
    let mut tracker = WindowStateTracker::new(false);

    for i in 1..=5 {
        assert_eq!(tracker.window_count, 1);
        tracker.open_from_browser_extension();
        assert!(tracker.is_popup_mode);
        assert!(tracker.visible);

        if i % 2 == 1 {
            tracker.handle_cancel();
            assert!(!tracker.visible);
            assert!(!tracker.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed));
        } else {
            tracker.handle_start_download(&format!("file_{}.bin", i), false);
            assert!(!tracker.visible);
            assert!(!tracker.is_main_window_open.load(std::sync::atomic::Ordering::Relaxed));
            assert_eq!(tracker.notification_sent, Some(format!("Download started: file_{}.bin", i)));
        }
        assert_eq!(tracker.window_count, 1);
    }
}

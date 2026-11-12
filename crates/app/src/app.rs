use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, Margin, Stroke, Vec2};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tray_icon::TrayIcon;

use rustget_core::{
    categorize,
    category::{Category, CategoryFilter},
    db::{Database, DownloadRecord},
    downloader::{DownloadOptions, DownloadProgress},
    queue::{QueueEvent, QueueManager},
    BridgeServer, ClipboardMonitor, ExtensionDownloadRequest,
};

use crate::components::*;
use crate::dialogs::*;
use crate::platform;
use crate::theme::{self, shapes};

pub const ICON_RGBA: &[u8] = include_bytes!("icon_32.rgba");
pub const APP_TITLE: &str = "Cosmo Downloader";

pub fn compute_category_counts(downloads: &[DownloadRecord]) -> CategoryCounts {
    let mut counts = CategoryCounts {
        all: downloads.len(),
        programs: 0,
        video: 0,
        music: 0,
        documents: 0,
        archives: 0,
    };
    for d in downloads {
        if CategoryFilter::Programs.matches(&d.category) {
            counts.programs += 1;
        } else if CategoryFilter::Video.matches(&d.category) {
            counts.video += 1;
        } else if CategoryFilter::Music.matches(&d.category) {
            counts.music += 1;
        } else if CategoryFilter::Documents.matches(&d.category) {
            counts.documents += 1;
        } else if CategoryFilter::Compressed.matches(&d.category) {
            counts.archives += 1;
        }
    }
    counts
}

pub struct CosmoDownloaderApp {
    pub queue: Arc<QueueManager>,
    pub downloads: Vec<DownloadRecord>,
    pub active_progress: HashMap<String, DownloadProgress>,
    pub ema_speeds: HashMap<String, f64>,
    pub selected_category: CategoryFilter,
    pub selected_ids: HashSet<String>,
    pub is_dark: bool,
    pub logo_texture: Option<egui::TextureHandle>,

    // Dialogs & Popup Mode
    pub is_popup_mode: bool,
    pub is_standalone_popup: bool,
    pub standalone_popup_file: Option<String>,
    pub popup_opened_at: Option<std::time::Instant>,
    pub show_add_dialog: bool,
    pub show_settings_dialog: bool,
    pub show_about_dialog: bool,
    pub completed_dialog_record: Option<DownloadRecord>,
    pub is_completed_popup_mode: bool,
    pub active_popup_task_id: Option<String>,
    pub active_popup_filename: String,
    pub active_popup_url: String,
    pub active_popup_target_path: String,
    pub is_progress_popup_mode: bool,
    pub pending_task_rx: Option<tokio::sync::mpsc::Receiver<String>>,

    // Add dialog fields
    pub add_url: String,
    pub add_filename: String,
    pub add_category: String,
    pub add_save_dir: String,
    pub add_connections: usize,
    pub add_speed_limit_kb: u64,
    pub add_checksum: String,
    pub add_cookies: Option<String>,
    pub add_referer: Option<String>,
    pub add_file_size: Option<u64>,
    pub add_description: String,
    pub add_remember_category: bool,
    pub show_notes_expanded: bool,
    pub size_probe_rx: Option<mpsc::Receiver<(Option<u64>, Option<String>)>>,

    // Settings fields
    pub setting_max_concurrent: usize,
    pub setting_global_speed_kb: u64,
    pub setting_clipboard_monitor: bool,
    pub setting_minimize_to_tray: bool,
    pub setting_show_tray_icon: bool,
    pub setting_autostart: bool,
    pub setting_start_minimized: bool,
    pub setting_dont_show_complete_dialog: bool,

    // System tray
    pub tray_icon: Option<TrayIcon>,
    pub tray_tx: std::sync::mpsc::Sender<String>,
    pub tray_event_receiver: Option<std::sync::mpsc::Receiver<String>>,

    // Async event receivers
    pub event_rx: mpsc::Receiver<QueueEvent>,
    pub clipboard_rx: mpsc::Receiver<String>,
    pub browser_dl_rx: mpsc::Receiver<ExtensionDownloadRequest>,
    pub show_rx: mpsc::Receiver<()>,
    pub quit_rx: mpsc::Receiver<()>,
    pub shutting_down: bool,
    pub initial_minimized_handled: bool,
}

impl CosmoDownloaderApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut is_standalone_popup = false;
        let mut standalone_popup_file = None;
        let mut standalone_req = None;
        let mut initial_url_arg: Option<String> = None;

        let mut args_iter = std::env::args().skip(1);
        while let Some(arg) = args_iter.next() {
            if arg == "--popup-file" {
                if let Some(file_path) = args_iter.next() {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        if let Ok(req) = serde_json::from_str::<ExtensionDownloadRequest>(&content) {
                            standalone_req = Some(req);
                            is_standalone_popup = true;
                            standalone_popup_file = Some(file_path);
                        }
                    }
                }
            } else if arg.starts_with("--popup-file=") {
                let file_path = arg.trim_start_matches("--popup-file=").to_string();
                if let Ok(content) = std::fs::read_to_string(&file_path) {
                    if let Ok(req) = serde_json::from_str::<ExtensionDownloadRequest>(&content) {
                        standalone_req = Some(req);
                        is_standalone_popup = true;
                        standalone_popup_file = Some(file_path);
                    }
                }
            } else if (arg.starts_with("http://") || arg.starts_with("https://")) && initial_url_arg.is_none() {
                initial_url_arg = Some(arg);
            }
        }

        #[cfg(windows)]
        platform::windows::init_shutdown_listener(&cc.egui_ctx);

        let is_popup_arg = std::env::args().any(|a| a == "--popup") || is_standalone_popup;

        let db = Database::open_default().unwrap_or_else(|_| Database::open_in_memory().unwrap());
        let (event_tx, event_rx) = mpsc::channel(128);
        let queue = Arc::new(QueueManager::new(db.clone(), 3, 0, event_tx));

        platform::ensure_native_host_registered();

        let (bridge_tx, bridge_rx) = mpsc::channel(32);
        let (show_tx, show_rx) = mpsc::channel(16);
        let (quit_tx, quit_rx) = mpsc::channel(4);
        let daemon_online = std::net::TcpStream::connect("127.0.0.1:42424").is_ok();

        if !daemon_online {
            platform::log_daemon("Starting queue worker and bridge server on port 42424");
            let q_worker = Arc::clone(&queue);
            let q_cancel = CancellationToken::new();
            tokio::spawn(async move {
                q_worker.run_queue_worker(q_cancel).await;
            });

            let ctx_wake = cc.egui_ctx.clone();
            let ctx_wake_dl = cc.egui_ctx.clone();
            let ctx_wake_quit = cc.egui_ctx.clone();
            let bridge_server = BridgeServer::new(Arc::clone(&queue), 42424)
                .with_notifier(bridge_tx)
                .with_show_notifier(show_tx)
                .with_quit_notifier(quit_tx)
                .with_wake_callback(move || {
                    #[cfg(windows)]
                    platform::show_main_window();
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                })
                .with_download_wake_callback(move || {
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(460.0, 390.0)));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Title("New Download".to_string()));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                    #[cfg(windows)]
                    platform::make_popup_topmost();
                });

            let bridge_cancel = CancellationToken::new();
            tokio::spawn(async move {
                let _ = bridge_server.run(bridge_cancel).await;
                ctx_wake_quit.request_repaint();
            });
        }

        let (clip_tx, clip_rx) = mpsc::channel(32);
        let clip_cancel = CancellationToken::new();
        let clip_monitor = ClipboardMonitor::new(clip_cancel);
        clip_monitor.start(clip_tx);

        let (autostart, reg_start_minimized) = platform::get_autostart_info();
        let (saved_clip, saved_min_tray, saved_show_tray, saved_start_min, saved_theme) = {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let db = queue.db();
                let clip = db.get_setting("clipboard_monitor_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(false);
                let min_tray = db.get_setting("minimize_to_tray_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(true);
                let show_tray = db.get_setting("show_tray_icon_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(true);
                let start_min = db.get_setting("start_minimized_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(reg_start_minimized);
                let ui_theme = db.get_setting("ui_theme").await.ok().flatten();
                (clip, min_tray, show_tray, start_min, ui_theme)
            })
        };

        let is_dark = match saved_theme.as_deref() {
            Some("light") => false,
            Some("dark") => true,
            _ => platform::system_prefers_dark(),
        };
        theme::configure_visuals(&cc.egui_ctx, is_dark);

        let color_image = egui::ColorImage::from_rgba_unmultiplied([32, 32], ICON_RGBA);
        let logo_texture = Some(cc.egui_ctx.load_texture("cosmo_logo", color_image, egui::TextureOptions::LINEAR));

        let (tray_tx, tray_event_receiver) = std::sync::mpsc::channel();
        // Always initialize tray icon on the primary single-instance process (even if launched via --popup-file)
        let tray_icon = if !daemon_online && saved_show_tray {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                platform::build_system_tray(&tray_tx, cc.egui_ctx.clone(), ICON_RGBA, APP_TITLE)
            }))
            .ok()
            .flatten()
        } else {
            None
        };

        let initial_downloads = {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                queue.db().list_downloads(None).await.unwrap_or_default()
            })
        };

        let initial_category = {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                match queue.db().get_setting("ui_selected_category").await.unwrap_or(None).as_deref() {
                    Some("video") => CategoryFilter::Video,
                    Some("music") => CategoryFilter::Music,
                    Some("documents") => CategoryFilter::Documents,
                    Some("compressed") => CategoryFilter::Compressed,
                    Some("programs") => CategoryFilter::Programs,
                    Some("other") => CategoryFilter::Other,
                    _ => CategoryFilter::All,
                }
            })
        };

        let mut app = Self {
            queue,
            downloads: initial_downloads,
            active_progress: HashMap::new(),
            ema_speeds: HashMap::new(),
            selected_category: initial_category,
            selected_ids: HashSet::new(),
            is_dark,
            logo_texture,

            is_popup_mode: is_popup_arg,
            is_standalone_popup,
            standalone_popup_file,
            popup_opened_at: if is_popup_arg { Some(std::time::Instant::now()) } else { None },
            show_add_dialog: is_popup_arg,
            show_settings_dialog: false,
            show_about_dialog: false,
            completed_dialog_record: None,
            is_completed_popup_mode: false,
            active_popup_task_id: None,
            active_popup_filename: String::new(),
            active_popup_url: String::new(),
            active_popup_target_path: String::new(),
            is_progress_popup_mode: false,
            pending_task_rx: None,

            add_url: String::new(),
            add_filename: String::new(),
            add_category: "General".to_string(),
            add_save_dir: Category::General.default_dir().to_string_lossy().to_string(),
            add_connections: 8,
            add_speed_limit_kb: 0,
            add_checksum: String::new(),
            add_cookies: None,
            add_referer: None,
            add_file_size: None,
            add_description: String::new(),
            add_remember_category: false,
            show_notes_expanded: false,
            size_probe_rx: None,

            setting_max_concurrent: 3,
            setting_global_speed_kb: 0,
            setting_clipboard_monitor: saved_clip,
            setting_minimize_to_tray: saved_min_tray,
            setting_show_tray_icon: saved_show_tray,
            setting_autostart: autostart,
            setting_start_minimized: saved_start_min,
            setting_dont_show_complete_dialog: false,

            tray_icon,
            tray_tx,
            tray_event_receiver: Some(tray_event_receiver),

            event_rx,
            clipboard_rx: clip_rx,
            browser_dl_rx: bridge_rx,
            show_rx,
            quit_rx,
            shutting_down: false,
            initial_minimized_handled: false,
        };

        if let Some(req) = standalone_req {
            app.prepare_download_dialog(&req.url, req.filename, req.cookies, req.referer, req.num_segments);
        } else if let Some(url_arg) = initial_url_arg {
            app.prepare_download_dialog(&url_arg, None, None, None, None);
        }

        app
    }

    /// Cancels active tasks cleanly (preserving them as `'queued'` for auto-resume),
    /// flushes SQLite WAL, removes tray icon & temp popup files, and exits cleanly.
    pub fn perform_graceful_shutdown(&mut self) -> ! {
        if !self.shutting_down {
            self.shutting_down = true;
            platform::log_daemon("Graceful shutdown requested; saving active downloads and DB state...");
            self.tray_icon = None;
            if let Some(ref p) = self.standalone_popup_file {
                let _ = std::fs::remove_file(p);
            }
            let q = Arc::clone(&self.queue);
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async move {
                q.shutdown_and_save().await;
            });
            platform::log_daemon("Graceful shutdown complete.");
        }
        std::process::exit(0);
    }

    pub fn refresh_downloads(&mut self) {
        let q = Arc::clone(&self.queue);
        let downloads = {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                q.db().list_downloads(None).await.unwrap_or_default()
            })
        };
        let valid_ids: HashSet<&str> = downloads.iter().map(|d| d.id.as_str()).collect();
        self.selected_ids.retain(|id| valid_ids.contains(id.as_str()));
        self.downloads = downloads;
    }

    pub fn prepare_download_dialog(
        &mut self,
        url: &str,
        filename: Option<String>,
        cookies: Option<String>,
        referer: Option<String>,
        num_segments: Option<usize>,
    ) {
        self.add_url = url.trim().to_string();
        self.add_cookies = cookies;
        self.add_referer = referer;
        self.add_connections = num_segments.unwrap_or(8);
        self.add_description.clear();
        self.add_file_size = None;
        self.add_remember_category = true;

        if let Some(f) = filename {
            if !f.trim().is_empty() {
                self.add_filename = rustget_core::probe::sanitize_filename(&f);
            }
        } else if let Ok(parsed) = url::Url::parse(&self.add_url) {
            let name = rustget_core::probe::extract_filename(&reqwest::header::HeaderMap::new(), &parsed);
            self.add_filename = rustget_core::probe::sanitize_filename(&name);
        }

        let cat = if rustget_core::youtube::is_streaming_site(&self.add_url) {
            if self.add_filename.is_empty() || self.add_filename.eq_ignore_ascii_case("watch") {
                self.add_filename = "YouTube_Video.mp4".to_string();
            } else if !self.add_filename.contains('.') {
                self.add_filename.push_str(".mp4");
            }
            Category::Video
        } else {
            categorize(Some(&self.add_filename), None)
        };
        self.add_category = cat.name().to_string();
        self.add_save_dir = cat.default_dir().to_string_lossy().to_string();
        self.show_notes_expanded = false;
        self.show_add_dialog = true;

        if !rustget_core::youtube::is_streaming_site(&self.add_url) {
            let (tx, rx) = mpsc::channel(1);
            self.size_probe_rx = Some(rx);
            if let Ok(parsed_url) = url::Url::parse(&self.add_url) {
                let cookies_clone = self.add_cookies.clone();
                let referer_clone = self.add_referer.clone();
                tokio::spawn(async move {
                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(3))
                        .build();
                    if let Ok(c) = client {
                        let mut headers = reqwest::header::HeaderMap::new();
                        if let Some(ref r) = referer_clone {
                            if let Ok(hv) = reqwest::header::HeaderValue::from_str(r) {
                                headers.insert(reqwest::header::REFERER, hv);
                            }
                        }
                        if let Some(ref ck) = cookies_clone {
                            if let Ok(hv) = reqwest::header::HeaderValue::from_str(ck) {
                                headers.insert(reqwest::header::COOKIE, hv);
                            }
                        }
                        if let Ok(info) = rustget_core::probe::probe_url(&c, &parsed_url, &headers).await {
                            let _ = tx.send((info.content_length, Some(info.suggested_filename))).await;
                        }
                    }
                });
            }
        }
    }

    pub fn trigger_download(&mut self, start_paused: bool) {
        let url = self.add_url.trim().to_string();
        if url.is_empty() {
            return;
        }

        self.active_popup_filename = self.add_filename.clone();
        self.active_popup_url = url.clone();

        let mut opt = DownloadOptions {
            url,
            num_segments: self.add_connections,
            expected_sha256: if self.add_checksum.trim().is_empty() {
                None
            } else {
                Some(self.add_checksum.trim().to_string())
            },
            ..Default::default()
        };

        let mut out_path = if !self.add_save_dir.trim().is_empty() {
            PathBuf::from(self.add_save_dir.trim())
        } else {
            Category::General.default_dir()
        };
        if !self.add_filename.trim().is_empty() {
            out_path.push(self.add_filename.trim());
        }
        self.active_popup_target_path = out_path.to_string_lossy().to_string();
        opt.output_path = Some(out_path);

        if let Some(c) = self.add_cookies.take() {
            opt.cookie = Some(c);
        }
        if let Some(r) = self.add_referer.take() {
            opt.referer = Some(r);
        }

        let q = Arc::clone(&self.queue);
        let speed_bps = self.add_speed_limit_kb * 1024;
        let (id_tx, id_rx) = mpsc::channel(1);
        self.pending_task_rx = Some(id_rx);

        let is_standalone = self.is_standalone_popup;
        tokio::spawn(async move {
            let res = if start_paused {
                if let Ok(id) = q.add_download(opt, None, speed_bps).await {
                    let _ = q.pause(&id).await;
                    Ok(id)
                } else {
                    Err(())
                }
            } else if is_standalone {
                q.start_download_immediately(opt, None, speed_bps).await.map_err(|_| ())
            } else {
                q.add_download(opt, None, speed_bps).await.map_err(|_| ())
            };

            if let Ok(id) = res {
                let _ = id_tx.send(id).await;
            }
        });

        self.add_url.clear();
        self.add_filename.clear();
        self.add_checksum.clear();
        self.add_description.clear();
        self.show_add_dialog = false;
    }

    fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.is_dark = !self.is_dark;
        theme::configure_visuals(ctx, self.is_dark);
        let db = self.queue.db().clone();
        let val = if self.is_dark { "dark" } else { "light" };
        tokio::spawn(async move {
            let _ = db.set_setting("ui_theme", val).await;
        });
    }

    fn restore_main_window_chrome(&self, ctx: &egui::Context) {
        platform::remove_popup_topmost();
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(920.0, 580.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(APP_TITLE.to_string()));
    }

    fn resume_selected(&self) {
        let q = Arc::clone(&self.queue);
        let ids: Vec<String> = self
            .downloads
            .iter()
            .filter(|d| self.selected_ids.contains(&d.id) && (d.status == "paused" || d.status == "failed"))
            .map(|d| d.id.clone())
            .collect();
        tokio::spawn(async move {
            for id in ids {
                let _ = q.resume(&id).await;
            }
        });
    }

    fn pause_selected(&self) {
        let q = Arc::clone(&self.queue);
        let ids: Vec<String> = self
            .downloads
            .iter()
            .filter(|d| self.selected_ids.contains(&d.id) && (d.status == "downloading" || d.status == "queued"))
            .map(|d| d.id.clone())
            .collect();
        tokio::spawn(async move {
            for id in ids {
                let _ = q.pause(&id).await;
            }
        });
    }

    fn delete_selected(&mut self) {
        let q = Arc::clone(&self.queue);
        let ids: Vec<String> = self.selected_ids.drain().collect();
        if ids.is_empty() {
            return;
        }
        tokio::spawn(async move {
            for id in ids {
                let _ = q.remove(&id, false).await;
            }
        });
    }

    fn resume_all(&self) {
        let q = Arc::clone(&self.queue);
        let ids: Vec<String> = self
            .downloads
            .iter()
            .filter(|d| d.status == "paused" || d.status == "failed")
            .map(|d| d.id.clone())
            .collect();
        tokio::spawn(async move {
            for id in ids {
                let _ = q.resume(&id).await;
            }
        });
    }

    fn pause_all(&self) {
        let q = Arc::clone(&self.queue);
        let ids: Vec<String> = self
            .downloads
            .iter()
            .filter(|d| d.status == "downloading" || d.status == "queued")
            .map(|d| d.id.clone())
            .collect();
        tokio::spawn(async move {
            for id in ids {
                let _ = q.pause(&id).await;
            }
        });
    }

    fn handle_borderless_resize(&self, ctx: &egui::Context) {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        if is_maximized {
            return;
        }
        let Some(rect) = ctx.input(|i| i.viewport().inner_rect) else {
            return;
        };
        let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
            return;
        };

        let edge = 5.0;
        let left = pos.x <= edge;
        let right = pos.x >= rect.width() - edge;
        let top = pos.y <= edge;
        let bottom = pos.y >= rect.height() - edge;

        let dir = match (left, right, top, bottom) {
            (true, false, true, false) => Some(egui::ResizeDirection::NorthWest),
            (false, true, true, false) => Some(egui::ResizeDirection::NorthEast),
            (true, false, false, true) => Some(egui::ResizeDirection::SouthWest),
            (false, true, false, true) => Some(egui::ResizeDirection::SouthEast),
            (true, false, false, false) => Some(egui::ResizeDirection::West),
            (false, true, false, false) => Some(egui::ResizeDirection::East),
            (false, false, true, false) => Some(egui::ResizeDirection::North),
            (false, false, false, true) => Some(egui::ResizeDirection::South),
            _ => None,
        };

        if let Some(d) = dir {
            let icon = match d {
                egui::ResizeDirection::North | egui::ResizeDirection::South => egui::CursorIcon::ResizeVertical,
                egui::ResizeDirection::East | egui::ResizeDirection::West => egui::CursorIcon::ResizeHorizontal,
                egui::ResizeDirection::NorthWest | egui::ResizeDirection::SouthEast => egui::CursorIcon::ResizeNwSe,
                egui::ResizeDirection::NorthEast | egui::ResizeDirection::SouthWest => egui::CursorIcon::ResizeNeSw,
            };
            ctx.set_cursor_icon(icon);
            if ctx.input(|i| i.pointer.primary_pressed()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(d));
            }
        }
    }
}

impl eframe::App for CosmoDownloaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(windows)]
        platform::windows::init_shutdown_listener(ctx);

        let external_quit = self.quit_rx.try_recv().is_ok()
            || {
                #[cfg(windows)]
                { platform::windows::is_external_shutdown_requested() }
                #[cfg(not(windows))]
                { false }
            };
        if external_quit {
            self.perform_graceful_shutdown();
        }

        if !self.initial_minimized_handled {
            self.initial_minimized_handled = true;
            let start_minimized = std::env::args().any(|a| a == "--minimized");
            if start_minimized {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                if self.setting_minimize_to_tray && self.tray_icon.is_some() {
                    platform::hide_main_window();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
            }
        }

        ctx.request_repaint_after(Duration::from_millis(100));

        let pal = theme::palette(self.is_dark);

        // Window close handler (honor OS/installer quit first, otherwise tray only if tray icon exists)
        if ctx.input(|i| i.viewport().close_requested()) {
            #[cfg(windows)]
            if platform::windows::is_external_shutdown_requested() {
                self.perform_graceful_shutdown();
            }

            if self.is_progress_popup_mode {
                self.is_progress_popup_mode = false;
                self.active_popup_task_id = None;
                self.restore_main_window_chrome(ctx);
                if self.tray_icon.is_some() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    platform::hide_main_window();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                } else {
                    self.perform_graceful_shutdown();
                }
                return;
            }
            if self.is_completed_popup_mode {
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                self.is_popup_mode = false;
                self.is_progress_popup_mode = false;
                self.active_popup_task_id = None;
                self.popup_opened_at = None;
                self.restore_main_window_chrome(ctx);
                if self.tray_icon.is_some() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    platform::hide_main_window();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                } else {
                    self.perform_graceful_shutdown();
                }
                return;
            }
            if self.is_popup_mode {
                if let Some(opened) = self.popup_opened_at {
                    if opened.elapsed() < Duration::from_millis(600) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                        return;
                    }
                }
                self.is_popup_mode = false;
                self.is_progress_popup_mode = false;
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                self.active_popup_task_id = None;
                self.popup_opened_at = None;
                self.show_add_dialog = false;
                self.add_url.clear();
                self.add_filename.clear();
                self.restore_main_window_chrome(ctx);
                if self.tray_icon.is_some() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    platform::hide_main_window();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                } else {
                    self.perform_graceful_shutdown();
                }
                return;
            }

            if self.setting_minimize_to_tray && self.tray_icon.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                platform::remove_popup_topmost();
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                platform::hide_main_window();
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                return;
            } else {
                self.perform_graceful_shutdown();
            }
        }

        // Receive task ID
        if let Some(ref mut rx) = self.pending_task_rx {
            if let Ok(id) = rx.try_recv() {
                self.active_popup_task_id = Some(id);
            }
        }

        // Size probe receiver
        if let Some(ref mut rx) = self.size_probe_rx {
            if let Ok((sz, suggested_name)) = rx.try_recv() {
                self.add_file_size = sz;
                if let Some(name) = suggested_name {
                    let cur_name = self.add_filename.trim().to_ascii_lowercase();
                    if cur_name == "download"
                        || cur_name == "download.bin"
                        || !self.add_filename.contains('.')
                        || (name.contains('.') && !cur_name.contains('.'))
                    {
                        self.add_filename = name.clone();
                        let cat = categorize(Some(&name), None);
                        self.add_category = cat.name().to_string();
                        self.add_save_dir = cat.default_dir().to_string_lossy().to_string();
                    }
                }
            }
        }

        // Event processing
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                QueueEvent::TaskCompleted(id) => {
                    self.refresh_downloads();
                    if self.is_progress_popup_mode {
                        self.is_progress_popup_mode = false;
                    }
                    if !self.setting_dont_show_complete_dialog {
                        let record = self.downloads.iter().find(|d| d.id == id).cloned().unwrap_or_else(|| {
                            let mut resolved_target = self.active_popup_target_path.clone();
                            let p = Path::new(&resolved_target);
                            if !p.exists() {
                                if let Some(parent) = p.parent() {
                                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                                        if let Ok(entries) = std::fs::read_dir(parent) {
                                            for entry in entries.flatten() {
                                                let ep = entry.path();
                                                if ep.is_file() {
                                                    if let Some(name) = ep.file_name().and_then(|n| n.to_str()) {
                                                        if name.starts_with(stem) && !name.ends_with(".part") && !name.ends_with(".ytdl") {
                                                            resolved_target = ep.to_string_lossy().to_string();
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            let (size, bytes) = if let Ok(meta) = std::fs::metadata(&resolved_target) {
                                (Some(meta.len()), meta.len())
                            } else {
                                (None, 0)
                            };
                            let cat_str = categorize(Some(&resolved_target), None).name().to_string();
                            DownloadRecord {
                                id: id.clone(),
                                url: self.active_popup_url.clone(),
                                filename: Path::new(&resolved_target)
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(&self.active_popup_filename)
                                    .to_string(),
                                target_path: resolved_target,
                                part_path: String::new(),
                                category: cat_str,
                                total_size: size,
                                downloaded_bytes: bytes,
                                status: "completed".to_string(),
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
                                created_at: chrono::Utc::now().to_rfc3339(),
                                updated_at: chrono::Utc::now().to_rfc3339(),
                            }
                        });
                        self.completed_dialog_record = Some(record);
                        self.is_completed_popup_mode = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(560.0, 260.0)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Download complete".to_string()));
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                        #[cfg(windows)]
                        platform::make_popup_topmost();
                    }
                }
                QueueEvent::TaskAdded(_) | QueueEvent::TaskFailed { .. } => {
                    self.refresh_downloads();
                }
                QueueEvent::StatusChanged { id, status } => {
                    if status == "completed" || status == "paused" || status == "failed" {
                        self.active_progress.remove(&id);
                        self.ema_speeds.remove(&id);
                    }
                    self.refresh_downloads();
                }
                QueueEvent::Progress { id, progress } => {
                    let current_speed = progress.speed_bps;
                    let smoothed = if let Some(&prev) = self.ema_speeds.get(&id) {
                        0.3 * current_speed + 0.7 * prev
                    } else {
                        current_speed
                    };
                    self.ema_speeds.insert(id.clone(), smoothed);
                    self.active_progress.insert(id, progress);
                }
            }
        }

        // External show requests
        while let Ok(_) = self.show_rx.try_recv() {
            if self.is_popup_mode || self.is_progress_popup_mode || self.is_completed_popup_mode {
                continue;
            }
            self.is_popup_mode = false;
            self.is_progress_popup_mode = false;
            self.is_completed_popup_mode = false;
            self.restore_main_window_chrome(ctx);
            platform::show_main_window();
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
        }

        // Browser extension download requests
        while let Ok(req) = self.browser_dl_rx.try_recv() {
            self.prepare_download_dialog(&req.url, req.filename, req.cookies, req.referer, req.num_segments);
            self.is_popup_mode = true;
            self.popup_opened_at = Some(std::time::Instant::now());
            self.is_progress_popup_mode = false;
            self.is_completed_popup_mode = false;
            self.completed_dialog_record = None;
            self.active_popup_task_id = None;
            self.show_add_dialog = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(560.0, 310.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::Title("Download File Info".to_string()));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
            #[cfg(windows)]
            platform::make_popup_topmost();
        }

        // Clipboard events
        while let Ok(url) = self.clipboard_rx.try_recv() {
            if self.setting_clipboard_monitor {
                self.prepare_download_dialog(&url, None, None, None, None);
                self.is_popup_mode = true;
                self.popup_opened_at = Some(std::time::Instant::now());
                self.is_progress_popup_mode = false;
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                self.active_popup_task_id = None;
                self.show_add_dialog = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(560.0, 310.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::Title("Download File Info".to_string()));
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                #[cfg(windows)]
                platform::make_popup_topmost();
            }
        }

        // Tray actions
        if let Some(ref rx) = self.tray_event_receiver {
            while let Ok(action) = rx.try_recv() {
                match action.as_str() {
                    "show" => {
                        self.is_popup_mode = false;
                        self.is_progress_popup_mode = false;
                        self.is_completed_popup_mode = false;
                        self.restore_main_window_chrome(ctx);
                        platform::show_main_window();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                    }
                    "add" => {
                        self.is_popup_mode = false;
                        self.is_progress_popup_mode = false;
                        self.is_completed_popup_mode = false;
                        self.restore_main_window_chrome(ctx);
                        platform::show_main_window();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        self.show_add_dialog = true;
                    }
                    "exit" => {
                        self.perform_graceful_shutdown();
                    }
                    _ => {}
                }
            }
        }

        // Standalone Progress Mode
        if self.is_progress_popup_mode {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(pal.surface).inner_margin(Margin::same(16.0)))
                .show(ctx, |ui| {
                    let task_id = self.active_popup_task_id.clone();
                    let fname = if !self.active_popup_filename.is_empty() {
                        self.active_popup_filename.clone()
                    } else {
                        "Downloading file...".to_string()
                    };
                    let prog = task_id.as_ref().and_then(|id| self.active_progress.get(id));
                    let is_paused = task_id.as_ref().map_or(false, |id| {
                        self.downloads.iter().any(|d| &d.id == id && d.status == "paused")
                    });

                    let outcome = render_progress_dialog(ui, pal, &fname, &self.active_popup_target_path, prog, is_paused);
                    if outcome.toggle_pause {
                        if let Some(ref id) = task_id {
                            let id_c = id.clone();
                            let q_c = Arc::clone(&self.queue);
                            tokio::spawn(async move {
                                if is_paused {
                                    let _ = q_c.resume(&id_c).await;
                                } else {
                                    let _ = q_c.pause(&id_c).await;
                                }
                            });
                        }
                    }
                    if outcome.cancel {
                        if let Some(ref id) = task_id {
                            let id_c = id.clone();
                            let q_c = Arc::clone(&self.queue);
                            tokio::spawn(async move {
                                let _ = q_c.remove(&id_c, false).await;
                            });
                        }
                        self.is_progress_popup_mode = false;
                        self.active_popup_task_id = None;
                        if self.is_standalone_popup {
                            self.restore_main_window_chrome(ctx);
                            platform::hide_main_window();
                            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        }
                    }
                    if outcome.close {
                        self.is_progress_popup_mode = false;
                        if self.is_standalone_popup {
                            self.restore_main_window_chrome(ctx);
                            platform::hide_main_window();
                            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        }
                    }
                });
            return;
        }

        // Standalone Complete Mode
        if self.is_completed_popup_mode && self.completed_dialog_record.is_some() {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(pal.surface).inner_margin(Margin::same(16.0)))
                .show(ctx, |ui| {
                    if let Some(ref record) = self.completed_dialog_record {
                        let outcome = render_complete_dialog(ui, pal, record);
                        if outcome.close {
                            if self.is_standalone_popup {
                                if let Some(ref f) = self.standalone_popup_file {
                                    let _ = std::fs::remove_file(f);
                                }
                                std::process::exit(0);
                            } else {
                                self.is_completed_popup_mode = false;
                                self.completed_dialog_record = None;
                                self.restore_main_window_chrome(ctx);
                                platform::hide_main_window();
                                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            }
                        }
                    }
                });
            return;
        }

        // Standalone Popup File Info Mode
        if self.is_popup_mode {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(pal.surface).inner_margin(Margin::same(20.0)))
                .show(ctx, |ui| {
                    let outcome = render_add_download_dialog(
                        ui,
                        pal,
                        &mut self.add_url,
                        &mut self.add_filename,
                        &mut self.add_category,
                        &mut self.add_save_dir,
                        &mut self.add_remember_category,
                        &mut self.add_description,
                        self.add_file_size,
                        &mut self.show_notes_expanded,
                    );

                    if outcome.open_settings {
                        self.show_settings_dialog = true;
                    }
                    if outcome.download_later {
                        self.trigger_download(true);
                        self.is_popup_mode = false;
                        self.popup_opened_at = None;
                        if self.is_standalone_popup {
                            self.restore_main_window_chrome(ctx);
                            if self.tray_icon.is_some() {
                                platform::hide_main_window();
                                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            } else {
                                self.perform_graceful_shutdown();
                            }
                        }
                    }
                    if outcome.start_now {
                        self.trigger_download(false);
                        self.is_popup_mode = false;
                        self.popup_opened_at = None;
                        self.is_progress_popup_mode = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(480.0, 240.0)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Downloading - Cosmo Downloader".to_string()));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                        #[cfg(windows)]
                        platform::make_popup_topmost();
                    }
                    if outcome.cancel {
                        self.show_add_dialog = false;
                        self.add_url.clear();
                        self.add_filename.clear();
                        self.is_popup_mode = false;
                        self.popup_opened_at = None;
                        if self.is_standalone_popup {
                            self.restore_main_window_chrome(ctx);
                            if self.tray_icon.is_some() {
                                platform::hide_main_window();
                                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            } else {
                                self.perform_graceful_shutdown();
                            }
                        }
                    }
                });
            return;
        }

        // ====================================================================
        // Full Dashboard View (Matches User's HTML UI)
        // ====================================================================
        self.handle_borderless_resize(ctx);

        let prev_cat = self.selected_category;
        let cat_counts = compute_category_counts(&self.downloads);

        let filtered_ids: Vec<String> = self
            .downloads
            .iter()
            .filter(|d| self.selected_category.matches(&d.category))
            .map(|d| d.id.clone())
            .collect();
        let filtered_count = filtered_ids.len();

        let total_speed: f64 = self.active_progress.values().map(|p| p.speed_bps).sum();

        // 1. Top Title Bar (.title)
        egui::TopBottomPanel::top("cosmo_title_bar")
            .exact_height(36.0)
            .frame(egui::Frame::none().fill(pal.raise).inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                let act = title_bar::render_title_bar(ui, ctx, pal, self.logo_texture.as_ref(), APP_TITLE);
                if act.minimize {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                if act.toggle_maximize {
                    let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                }
                if act.close {
                    if self.setting_minimize_to_tray && self.tray_icon.is_some() {
                        platform::hide_main_window();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    } else {
                        self.perform_graceful_shutdown();
                    }
                }
            });

        // 2. Menu Bar (.menubar)
        egui::TopBottomPanel::top("cosmo_menu_bar")
            .exact_height(36.0)
            .frame(egui::Frame::none().fill(pal.surface).inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                let m_act = menu_bar::render_menu_bar(
                    ui,
                    pal,
                    !self.selected_ids.is_empty(),
                    self.setting_clipboard_monitor,
                    self.selected_category,
                );
                if m_act.add_download {
                    self.show_add_dialog = true;
                }
                if m_act.exit_app {
                    self.perform_graceful_shutdown();
                }
                if m_act.resume_selected {
                    self.resume_selected();
                }
                if m_act.pause_selected {
                    self.pause_selected();
                }
                if m_act.delete_selected {
                    self.delete_selected();
                }
                if m_act.resume_all {
                    self.resume_all();
                }
                if m_act.pause_all {
                    self.pause_all();
                }
                if m_act.select_all {
                    for id in &filtered_ids {
                        self.selected_ids.insert(id.clone());
                    }
                }
                if m_act.clear_selection {
                    self.selected_ids.clear();
                }
                if let Some(cat) = m_act.set_category {
                    self.selected_category = cat;
                }
                if m_act.toggle_theme {
                    self.toggle_theme(ctx);
                }
                if m_act.open_options {
                    self.show_settings_dialog = true;
                }
                if m_act.toggle_clipboard {
                    self.setting_clipboard_monitor = !self.setting_clipboard_monitor;
                    let db = self.queue.db().clone();
                    let val = if self.setting_clipboard_monitor { "true" } else { "false" };
                    tokio::spawn(async move {
                        let _ = db.set_setting("clipboard_monitor_enabled", val).await;
                    });
                }
                if m_act.open_downloads_dir {
                    let dir = Category::General.default_dir().to_string_lossy().to_string();
                    platform::shell_open_folder_and_select(&dir);
                }
                if m_act.open_about {
                    self.show_about_dialog = true;
                }
            });

        // 3. Toolbar (.toolbar)
        egui::TopBottomPanel::top("cosmo_toolbar")
            .exact_height(60.0)
            .frame(egui::Frame::none().fill(pal.surface).inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                let tb = toolbar::render_toolbar(ui, pal, !self.selected_ids.is_empty());
                if tb.add_clicked {
                    self.show_add_dialog = true;
                }
                if tb.resume_clicked {
                    self.resume_selected();
                }
                if tb.pause_clicked {
                    self.pause_selected();
                }
                if tb.delete_clicked {
                    self.delete_selected();
                }
                if tb.options_clicked {
                    self.show_settings_dialog = true;
                }
            });

        // 4. Bottom Status Bar (.status)
        egui::TopBottomPanel::bottom("cosmo_status_bar")
            .exact_height(28.0)
            .frame(egui::Frame::none().fill(pal.raise).inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                status_bar::render_status_bar(
                    ui,
                    pal,
                    filtered_count,
                    total_speed,
                    env!("CARGO_PKG_VERSION"),
                );
            });

        // 5. Left Sidebar (.side)
        let pal_refresh = theme::palette(self.is_dark);
        egui::SidePanel::left("cosmo_sidebar")
            .resizable(false)
            .exact_width(200.0)
            .frame(
                egui::Frame::none()
                    .fill(pal_refresh.bg)
                    .inner_margin(Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                // Right 1px divider line
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [egui::pos2(r.right() + 12.0, r.top() - 12.0), egui::pos2(r.right() + 12.0, r.bottom() + 12.0)],
                    Stroke::new(1.0_f32, pal_refresh.line),
                );
                nav_rail::render_category_sidebar(ui, pal_refresh, &mut self.selected_category, &cat_counts);
            });

        if self.selected_category != prev_cat {
            let cat_str = match self.selected_category {
                CategoryFilter::All => "all",
                CategoryFilter::Video => "video",
                CategoryFilter::Music => "music",
                CategoryFilter::Documents => "documents",
                CategoryFilter::Compressed => "compressed",
                CategoryFilter::Programs => "programs",
                CategoryFilter::Other => "other",
            };
            let q = Arc::clone(&self.queue);
            tokio::spawn(async move {
                let _ = q.db().set_setting("ui_selected_category", cat_str).await;
            });
        }

        // 6. Central Downloads Table (.tbl)
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(pal_refresh.surface).inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                let filtered_downloads: Vec<&DownloadRecord> = self
                    .downloads
                    .iter()
                    .filter(|d| self.selected_category.matches(&d.category))
                    .collect();
                download_table::render_download_table(
                    ui,
                    pal_refresh,
                    &filtered_downloads,
                    &self.active_progress,
                    &self.ema_speeds,
                    &mut self.selected_ids,
                    &self.queue,
                );
            });

        // Add Download Modal Dialog
        if self.show_add_dialog && !self.is_popup_mode {
            egui::Window::new("New Download")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(
                    egui::Frame::none()
                        .fill(pal_refresh.surface)
                        .stroke(Stroke::new(1.0_f32, pal_refresh.line))
                        .inner_margin(Margin::same(20.0))
                        .rounding(shapes::R),
                )
                .show(ctx, |ui| {
                    ui.set_width(440.0);
                    let outcome = render_add_download_dialog(
                        ui,
                        pal_refresh,
                        &mut self.add_url,
                        &mut self.add_filename,
                        &mut self.add_category,
                        &mut self.add_save_dir,
                        &mut self.add_remember_category,
                        &mut self.add_description,
                        self.add_file_size,
                        &mut self.show_notes_expanded,
                    );
                    if outcome.open_settings {
                        self.show_settings_dialog = true;
                    }
                    if outcome.download_later {
                        self.trigger_download(true);
                        self.show_add_dialog = false;
                    }
                    if outcome.start_now {
                        self.trigger_download(false);
                        self.show_add_dialog = false;
                    }
                    if outcome.cancel {
                        self.show_add_dialog = false;
                        self.add_url.clear();
                        self.add_filename.clear();
                    }
                });
        }

        // Complete Modal Dialog
        let mut close_complete_dialog = false;
        if let Some(ref record) = self.completed_dialog_record {
            if !self.is_completed_popup_mode {
                egui::Window::new("Download Complete")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                    .frame(
                        egui::Frame::none()
                            .fill(pal_refresh.surface)
                            .stroke(Stroke::new(1.0_f32, pal_refresh.line))
                            .inner_margin(Margin::same(18.0))
                            .rounding(shapes::R),
                    )
                    .show(ctx, |ui| {
                        ui.set_width(520.0);
                        let outcome = render_complete_dialog(ui, pal_refresh, record);
                        if outcome.close {
                            close_complete_dialog = true;
                        }
                    });
            }
        }
        if close_complete_dialog {
            self.completed_dialog_record = None;
        }

        // Options / Settings Modal Dialog
        if self.show_settings_dialog {
            egui::Window::new("Options")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(
                    egui::Frame::none()
                        .fill(pal_refresh.surface)
                        .stroke(Stroke::new(1.0_f32, pal_refresh.line))
                        .inner_margin(Margin::same(18.0))
                        .rounding(shapes::R),
                )
                .show(ctx, |ui| {
                    ui.set_width(420.0);
                    let prev_tray = self.setting_show_tray_icon;
                    let outcome = render_settings_dialog(
                        ui,
                        pal_refresh,
                        &mut self.setting_max_concurrent,
                        &mut self.setting_global_speed_kb,
                        &mut self.setting_clipboard_monitor,
                        &mut self.setting_minimize_to_tray,
                        &mut self.setting_show_tray_icon,
                        &mut self.setting_autostart,
                        &mut self.setting_start_minimized,
                    );

                    if prev_tray != self.setting_show_tray_icon {
                        if self.setting_show_tray_icon {
                            self.tray_icon = platform::build_system_tray(&self.tray_tx, ctx.clone(), ICON_RGBA, APP_TITLE);
                        } else {
                            self.tray_icon = None;
                        }
                    }

                    if outcome.save_and_close {
                        self.show_settings_dialog = false;
                        let q = Arc::clone(&self.queue);
                        let db = self.queue.db().clone();
                        let max_c = self.setting_max_concurrent;
                        let speed_bps = self.setting_global_speed_kb * 1024;
                        let clip = self.setting_clipboard_monitor;
                        let min_tray = self.setting_minimize_to_tray;
                        let show_tray = self.setting_show_tray_icon;
                        let autostart = self.setting_autostart;
                        let start_min = self.setting_start_minimized;

                        tokio::spawn(async move {
                            q.set_max_concurrent(max_c).await;
                            q.set_global_speed_limit(speed_bps).await;
                            let _ = db.set_setting("clipboard_monitor_enabled", if clip { "true" } else { "false" }).await;
                            let _ = db.set_setting("minimize_to_tray_enabled", if min_tray { "true" } else { "false" }).await;
                            let _ = db.set_setting("show_tray_icon_enabled", if show_tray { "true" } else { "false" }).await;
                            let _ = db.set_setting("start_minimized_enabled", if start_min { "true" } else { "false" }).await;
                        });
                        platform::set_autostart_enabled(autostart, start_min);
                    }
                });
        }

        // About Modal Dialog
        if self.show_about_dialog {
            egui::Window::new("About Cosmo Downloader")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(
                    egui::Frame::none()
                        .fill(pal_refresh.surface)
                        .stroke(Stroke::new(1.0_f32, pal_refresh.line))
                        .inner_margin(Margin::same(20.0))
                        .rounding(shapes::R),
                )
                .show(ctx, |ui| {
                    ui.set_width(340.0);
                    ui.horizontal(|ui| {
                        if let Some(tex) = &self.logo_texture {
                            ui.image((tex.id(), Vec2::splat(28.0)));
                            ui.add_space(6.0);
                        }
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("Cosmo Downloader")
                                    .size(16.0)
                                    .strong()
                                    .color(pal_refresh.text),
                            );
                            ui.label(
                                egui::RichText::new(format!("Version {} · © 2026 Mahedi", env!("CARGO_PKG_VERSION")))
                                    .size(12.5)
                                    .color(pal_refresh.muted),
                            );
                        });
                    });
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("Fast multi-segment download manager for Windows with seamless browser integration.")
                            .size(13.0)
                            .color(pal_refresh.text),
                    );
                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ok_btn = egui::Button::new(
                            egui::RichText::new("Close").size(13.0).color(pal_refresh.on_accent),
                        )
                        .fill(pal_refresh.accent)
                        .min_size(Vec2::new(80.0, shapes::CONTROL_HEIGHT))
                        .rounding(shapes::R_SM);
                        if ui.add(ok_btn).clicked() {
                            self.show_about_dialog = false;
                        }
                    });
                });
        }
    }
}

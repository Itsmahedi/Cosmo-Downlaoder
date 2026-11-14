
pub mod app;
pub mod components;
pub mod dialogs;
pub mod icons;
pub mod platform;
pub mod theme;

use std::sync::Arc;
use eframe::egui;

pub use app::{CosmoDownloaderApp, APP_TITLE, DOWNLOAD_FILE_INFO_TITLE, ICON_RGBA};

fn send_http_to_bridge(req: &str) -> bool {
    for _ in 0..10 {
        if let Ok(mut stream) = std::net::TcpStream::connect("127.0.0.1:42424") {
            let _ = std::io::Write::write_all(&mut stream, req.as_bytes());
            let _ = std::io::Write::flush(&mut stream);
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}

fn handle_quit_command() -> ! {
    #[cfg(windows)]
    {
        let signaled = platform::windows::signal_existing_instance_shutdown();
        let token = rustget_core::server::get_or_create_bridge_token();
        let quit_req = format!(
            "POST /quit HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Cosmo-Token: {}\r\nConnection: close\r\n\r\n",
            token
        );
        let bridged = send_http_to_bridge(&quit_req);
        if signaled || bridged {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while std::time::Instant::now() < deadline {
                if std::net::TcpStream::connect("127.0.0.1:42424").is_err() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
    std::process::exit(0);
}

pub fn run() -> eframe::Result<()> {
    if std::env::args().any(|a| a == "--quit") {
        handle_quit_command();
    }

    std::panic::set_hook(Box::new(|info| {
        platform::log_daemon(&format!("PANIC: {:?}", info));
    }));
    tracing_subscriber::fmt().init();

    platform::log_daemon(&format!("main() started with args: {:?}", std::env::args().collect::<Vec<_>>()));

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    // Parse optional CLI URL or --popup-file argument
    let mut cli_popup_file: Option<String> = None;
    let mut cli_url: Option<String> = None;
    {
        let mut args = std::env::args().skip(1);
        while let Some(a) = args.next() {
            let clean = a.trim_matches('"').trim_matches('\'');
            if clean == "--popup-file" {
                cli_popup_file = args.next().map(|s| s.trim_matches('"').trim_matches('\'').to_string());
            } else if clean.starts_with("--popup-file=") {
                cli_popup_file = Some(clean.trim_start_matches("--popup-file=").trim_matches('"').trim_matches('\'').to_string());
            } else if clean.starts_with("http://") || clean.starts_with("https://") {
                cli_url = Some(clean.to_string());
            }
        }
    }

    // Single-instance guard
    let is_screenshot_mode = std::env::args().any(|a| {
        let clean = a.trim_matches('"').trim_matches('\'');
        clean.starts_with("--screenshot")
    });
    #[cfg(windows)]
    let _instance_mutex_guard: *mut std::ffi::c_void;
    #[cfg(windows)]
    unsafe {
        let mutex_handle = platform::windows::CreateMutexA(
            std::ptr::null_mut(),
            1,
            c"CosmoDownloader_v1_0_SingleInstance_Mutex".as_ptr().cast(),
        );
        let last_err = platform::windows::GetLastError();
        platform::log_daemon(&format!("Mutex check: last_error={}", last_err));
        if !is_screenshot_mode && last_err == 183 {
            if let Some(ref file_path) = cli_popup_file {
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    let clean = content.trim_start_matches('\u{feff}');
                    let req = format!(
                        "POST /download HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        clean.len(),
                        clean
                    );
                    if send_http_to_bridge(&req) {
                        let _ = std::fs::remove_file(file_path);
                        std::process::exit(0);
                    }
                }
            } else if let Some(ref url_arg) = cli_url {
                let body = serde_json::json!({ "url": url_arg }).to_string();
                let req = format!(
                    "POST /download HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                if send_http_to_bridge(&req) {
                    std::process::exit(0);
                }
            } else {
                let is_bg = std::env::args().any(|a| a == "--minimized" || a == "--daemon");
                if !is_bg {
                    let token = rustget_core::server::get_or_create_bridge_token();
                    let show_req = format!(
                        "POST /show HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Cosmo-Token: {}\r\nConnection: close\r\n\r\n",
                        token
                    );
                    if send_http_to_bridge(&show_req) {
                        std::process::exit(0);
                    }
                } else {
                    std::process::exit(0);
                }
            }
        }
        _instance_mutex_guard = mutex_handle;
    }

    #[cfg(not(windows))]
    let _unix_instance_guard = {
        if let Ok(mut stream) = std::net::TcpStream::connect("127.0.0.1:42424") {
            let token = rustget_core::server::get_or_create_bridge_token();
            let show_req = format!(
                "POST /show HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Cosmo-Token: {}\r\nConnection: close\r\n\r\n",
                token
            );
            let _ = std::io::Write::write_all(&mut stream, show_req.as_bytes());
            std::process::exit(0);
        }
    };

    let window_icon = egui::IconData {
        rgba: ICON_RGBA.to_vec(),
        width: 32,
        height: 32,
    };

    let is_popup_arg = std::env::args().any(|a| {
        let clean = a.trim_matches('"');
        clean == "--popup" || clean == "--popup-file" || clean.starts_with("--popup-file=")
    });
    let start_minimized = std::env::args().any(|a| a == "--minimized" || a == "--daemon");

    let custom_scale: Option<f32> = std::env::args().find_map(|a| {
        let clean = a.trim_matches('"').trim_matches('\'');
        if clean.starts_with("--scale=") {
            clean.trim_start_matches("--scale=").parse().ok()
        } else {
            None
        }
    });

    let mut viewport = egui::ViewportBuilder::default()
        .with_icon(Arc::new(window_icon));

    if is_popup_arg {
        let (w, h) = if let Some(s) = custom_scale {
            (560.0 * s, 350.0 * s)
        } else {
            (560.0, 350.0)
        };
        viewport = viewport
            .with_app_id("cosmo-dialog")
            .with_title(DOWNLOAD_FILE_INFO_TITLE)
            .with_inner_size([w, h])
            .with_min_inner_size([w, h])
            .with_max_inner_size([w, h])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_minimize_button(false)
            .with_close_button(true)
            .with_decorations(false)
            .with_visible(true)
            .with_active(true)
            .with_always_on_top();
    } else if start_minimized && !is_screenshot_mode {
        viewport = viewport
            .with_app_id("cosmo-downloader")
            .with_title(APP_TITLE)
            .with_decorations(false)
            .with_inner_size([1000.0, 640.0])
            .with_min_inner_size([640.0, 620.0])
            .with_resizable(true)
            .with_visible(false)
            .with_active(false);
    } else {
        viewport = viewport
            .with_app_id("cosmo-downloader")
            .with_title(APP_TITLE)
            .with_decorations(false)
            .with_inner_size([1000.0, 640.0])
            .with_min_inner_size([640.0, 620.0])
            .with_resizable(true)
            .with_visible(true)
            .with_active(true);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    platform::log_daemon("Calling eframe::run_native...");
    let res = eframe::run_native(
        "Cosmo Downloader",
        native_options,
        Box::new(|cc| {
            platform::log_daemon("Inside eframe creator closure...");
            Ok(Box::new(CosmoDownloaderApp::new(cc)))
        }),
    );
    if let Err(ref e) = res {
        platform::log_daemon(&format!("eframe::run_native error: {:?}", e));
    } else {
        platform::log_daemon("eframe::run_native exited cleanly");
    }
    res
}

#![windows_subsystem = "windows"]

pub mod app;
pub mod components;
pub mod dialogs;
pub mod icons;
pub mod platform;
pub mod theme;

use std::sync::Arc;
use eframe::egui;

pub use app::{CosmoDownloaderApp, APP_TITLE, ICON_RGBA};

fn main() -> eframe::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        platform::log_daemon(&format!("PANIC: {:?}", info));
    }));
    tracing_subscriber::fmt().init();

    platform::log_daemon(&format!("main() started with args: {:?}", std::env::args().collect::<Vec<_>>()));

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    // Single-instance guard
    #[cfg(windows)]
    let _instance_mutex_guard: *mut std::ffi::c_void;
    #[cfg(windows)]
    unsafe {
        let mutex_handle = platform::windows::CreateMutexA(
            std::ptr::null_mut(),
            1,
            b"CosmoDownloader_v1_0_SingleInstance_Mutex\0".as_ptr(),
        );
        if platform::windows::GetLastError() == 183 {
            if let Ok(mut stream) = std::net::TcpStream::connect("127.0.0.1:42424") {
                let mut args = std::env::args().skip(1);
                let mut popup_file = None;
                while let Some(a) = args.next() {
                    if a == "--popup-file" {
                        popup_file = args.next();
                    } else if a.starts_with("--popup-file=") {
                        popup_file = Some(a.trim_start_matches("--popup-file=").to_string());
                    }
                }

                if let Some(file_path) = popup_file {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        let req = format!(
                            "POST /download HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            content.len(),
                            content
                        );
                        let _ = std::io::Write::write_all(&mut stream, req.as_bytes());
                        let _ = std::fs::remove_file(file_path);
                    }
                } else {
                    let is_bg = std::env::args().any(|a| a == "--minimized" || a == "--daemon");
                    if !is_bg {
                        platform::windows::show_main_window();
                        let token = rustget_core::server::get_or_create_bridge_token();
                        let show_req = format!(
                            "POST /show HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Cosmo-Token: {}\r\nConnection: close\r\n\r\n",
                            token
                        );
                        let _ = std::io::Write::write_all(&mut stream, show_req.as_bytes());
                    }
                }
                std::process::exit(0);
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

    let is_popup_arg = std::env::args().any(|a| a == "--popup" || a == "--popup-file" || a.starts_with("--popup-file="));
    let start_minimized = std::env::args().any(|a| a == "--minimized" || a == "--daemon");

    let mut viewport = egui::ViewportBuilder::default()
        .with_icon(Arc::new(window_icon));

    if is_popup_arg {
        viewport = viewport
            .with_app_id("cosmo-dialog")
            .with_title("New Download")
            .with_inner_size([460.0, 390.0])
            .with_min_inner_size([460.0, 390.0])
            .with_max_inner_size([460.0, 390.0])
            .with_resizable(false)
            .with_visible(true)
            .with_active(true)
            .with_always_on_top();
    } else if start_minimized {
        viewport = viewport
            .with_app_id("cosmo-downloader")
            .with_title(APP_TITLE)
            .with_inner_size([880.0, 560.0])
            .with_min_inner_size([640.0, 440.0])
            .with_resizable(true)
            .with_visible(false)
            .with_active(false);
    } else {
        viewport = viewport
            .with_app_id("cosmo-downloader")
            .with_title(APP_TITLE)
            .with_inner_size([880.0, 560.0])
            .with_min_inner_size([640.0, 440.0])
            .with_resizable(true)
            .with_visible(true)
            .with_active(true);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Cosmo Downloader",
        native_options,
        Box::new(|cc| Ok(Box::new(CosmoDownloaderApp::new(cc)))),
    )
}

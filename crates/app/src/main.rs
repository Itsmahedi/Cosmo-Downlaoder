#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, Color32, Margin, Rounding, Stroke, Vec2};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tray_icon::{
    menu::{Menu, MenuItem},
    TrayIcon, TrayIconBuilder,
};

use rustget_core::{
    category::Category,
    db::{Database, DownloadRecord},
    downloader::{DownloadOptions, DownloadProgress},
    queue::{QueueEvent, QueueManager},
    speed::SpeedTracker,
    BridgeServer, ClipboardMonitor, ExtensionDownloadRequest,
};

const ICON_RGBA: &[u8] = include_bytes!("icon_32.rgba");

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn run_reg_command(args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut cmd = std::process::Command::new("reg");
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.output()
}

fn get_autostart_info() -> (bool, bool) {
    let output = run_reg_command(&["query", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "Cosmo Downloader"]);
    if let Ok(ref out) = output {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            let is_min = s.contains("--minimized");
            return (true, is_min);
        }
    }
    let output2 = run_reg_command(&["query", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "CosmoDownloader"]);
    if let Ok(out) = output2 {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            let is_min = s.contains("--minimized");
            return (true, is_min);
        }
    }
    (false, false)
}

fn set_autostart_enabled(enable: bool, start_minimized: bool) {
    std::thread::spawn(move || {
        if enable {
            if let Ok(exe) = std::env::current_exe() {
                let cmd = if start_minimized {
                    format!("\"{}\" --minimized", exe.display())
                } else {
                    format!("\"{}\"", exe.display())
                };
                let _ = run_reg_command(&["add", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "Cosmo Downloader", "/t", "REG_SZ", "/d", &cmd, "/f"]);
            }
        } else {
            let _ = run_reg_command(&["delete", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "Cosmo Downloader", "/f"]);
            let _ = run_reg_command(&["delete", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "Cosmo Space Downloader", "/f"]);
            let _ = run_reg_command(&["delete", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "CosmoDownloader", "/f"]);
            let _ = run_reg_command(&["delete", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "RustGet", "/f"]);
        }
    });
}

#[cfg(windows)]
extern "system" {
    fn CreateMutexA(lpMutexAttributes: *mut std::ffi::c_void, bInitialOwner: i32, lpName: *const u8) -> *mut std::ffi::c_void;
    fn GetLastError() -> u32;
    fn GetCurrentProcessId() -> u32;
    fn GetWindowThreadProcessId(hWnd: *mut std::ffi::c_void, lpdwProcessId: *mut u32) -> u32;
    fn EnumWindows(lpEnumFunc: Option<unsafe extern "system" fn(*mut std::ffi::c_void, isize) -> i32>, lParam: isize) -> i32;
    fn GetWindowLongW(hWnd: *mut std::ffi::c_void, nIndex: i32) -> i32;
    fn GetWindowTextLengthW(hWnd: *mut std::ffi::c_void) -> i32;
    fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> *mut std::ffi::c_void;
    fn ShowWindow(hWnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
    fn SetForegroundWindow(hWnd: *mut std::ffi::c_void) -> i32;
    fn SetWindowPos(hWnd: *mut std::ffi::c_void, hWndInsertAfter: *mut std::ffi::c_void, X: i32, Y: i32, cx: i32, cy: i32, uFlags: u32) -> i32;
    fn GetSystemMetrics(nIndex: i32) -> i32;
}

#[cfg(windows)]
const HWND_TOPMOST: *mut std::ffi::c_void = (-1isize) as *mut std::ffi::c_void;
#[cfg(windows)]
const HWND_NOTOPMOST: *mut std::ffi::c_void = (-2isize) as *mut std::ffi::c_void;

#[cfg(windows)]
pub fn get_process_window() -> *mut std::ffi::c_void {
    unsafe {
        let current_pid = GetCurrentProcessId();
        struct EnumData {
            pid: u32,
            hwnd: *mut std::ffi::c_void,
        }
        let mut data = EnumData {
            pid: current_pid,
            hwnd: std::ptr::null_mut(),
        };

        unsafe extern "system" fn enum_proc(hwnd: *mut std::ffi::c_void, lparam: isize) -> i32 {
            let data = &mut *(lparam as *mut EnumData);
            let mut wnd_pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut wnd_pid);
            if wnd_pid == data.pid {
                let style = GetWindowLongW(hwnd, -16 /* GWL_STYLE */) as u32;
                // Exclude child windows (WS_CHILD = 0x40000000)
                if (style & 0x40000000) == 0 {
                    let text_len = GetWindowTextLengthW(hwnd);
                    if text_len > 0 {
                        data.hwnd = hwnd;
                        return 0; // stop enumeration, found real titled window!
                    } else if data.hwnd.is_null() {
                        data.hwnd = hwnd;
                    }
                }
            }
            1 // continue
        }

        EnumWindows(Some(enum_proc), &mut data as *mut _ as isize);

        if !data.hwnd.is_null() {
            return data.hwnd;
        }

        // Fallback: title search
        let titles = [
            "Download File Info\0",
            "? Download File Info\0",
            "Download File Info\0",
            "Downloading - Cosmo Downloader\0",
            "Download complete\0",
            "? Download complete\0",
            "Download Complete\0",
            "? Cosmo Downloader v1.10\0",
            "Cosmo Downloader v1.10\0",
            "Cosmo Downloader v1.10\0",
            "? Cosmo Downloader v1.8\0",
            "✦ Cosmo Downloader v1.8\0",
            "Cosmo Downloader v1.8\0",
            "? Cosmo Downloader v1.3\0",
            "✦ Cosmo Downloader v1.3\0",
            "Cosmo Downloader v1.3\0",
            "? Cosmo Downloader v1.2\0",
            "✦ Cosmo Downloader v1.2\0",
            "Cosmo Downloader v1.2\0",
            "? Cosmo Downloader\0",
            "✦ Cosmo Downloader\0",
            "Cosmo Downloader\0",
            "CosmoDownloader\0",
        ];
        for t in titles {
            let encoded: Vec<u16> = t.encode_utf16().collect();
            let hwnd = FindWindowW(std::ptr::null(), encoded.as_ptr());
            if !hwnd.is_null() {
                return hwnd;
            }
        }

        std::ptr::null_mut()
    }
}

#[cfg(windows)]
pub fn show_download_popup_window() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            let screen_w = GetSystemMetrics(0 /* SM_CXSCREEN */);
            let screen_h = GetSystemMetrics(1 /* SM_CYSCREEN */);
            let w = 560;
            let h = 310;
            let x = if screen_w > w { (screen_w - w) / 2 } else { 100 };
            let y = if screen_h > h { (screen_h - h) / 2 } else { 100 };
            // Instantly position at 560x310 centered, make topmost, restore and show
            SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, 0x0040 /* SWP_SHOWWINDOW */);
            ShowWindow(hwnd, 9); // SW_RESTORE
            ShowWindow(hwnd, 5); // SW_SHOW
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(windows)]
pub fn show_main_window() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, 9); // SW_RESTORE
            ShowWindow(hwnd, 5); // SW_SHOW
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(windows)]
pub fn make_popup_topmost() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, 9); // SW_RESTORE
            ShowWindow(hwnd, 5); // SW_SHOW
            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0040);
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(windows)]
pub fn remove_popup_topmost() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, 0x0001 | 0x0002);
        }
    }
}

#[cfg(windows)]
pub fn hide_main_window() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, 0); // SW_HIDE removes window completely from taskbar
        }
    }
}

#[cfg(windows)]
#[repr(C)]
#[allow(non_snake_case)]
struct SHELLEXECUTEINFOW {
    cbSize: u32,
    fMask: u32,
    hwnd: *mut std::ffi::c_void,
    lpVerb: *const u16,
    lpFile: *const u16,
    lpParameters: *const u16,
    lpDirectory: *const u16,
    nShow: i32,
    hInstApp: *mut std::ffi::c_void,
    lpIDList: *mut std::ffi::c_void,
    lpClass: *const u16,
    hkeyClass: *mut std::ffi::c_void,
    dwHotKey: u32,
    hIconOrMonitor: *mut std::ffi::c_void,
    hProcess: *mut std::ffi::c_void,
}

#[cfg(windows)]
#[link(name = "shell32")]
#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(pvReserved: *mut std::ffi::c_void, dwCoInit: u32) -> i32;
    fn CoUninitialize();
    fn ShellExecuteW(
        hwnd: *mut std::ffi::c_void,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> isize;
    fn ShellExecuteExW(pExecInfo: *mut SHELLEXECUTEINFOW) -> i32;
}

#[cfg(windows)]
pub fn shell_open_file(path: &str) {
    let target = path.to_string();
    std::thread::spawn(move || {
        let p = std::path::Path::new(&target);
        let actual_target = if !p.exists() {
            let mut resolved = target.clone();
            if let Some(parent) = p.parent() {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            let entry_path = entry.path();
                            if entry_path.is_file() {
                                if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                                    if name.starts_with(stem) && !name.ends_with(".part") && !name.ends_with(".ytdl") {
                                        resolved = entry_path.to_string_lossy().to_string();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            resolved
        } else {
            target
        };

        let wide: Vec<u16> = actual_target.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let res = ShellExecuteW(
                std::ptr::null_mut(),
                std::ptr::null(), // default verb ("open")
                wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            );
            if (res as isize) <= 32 {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", "", &actual_target])
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();
            }
        }
    });
}

#[cfg(windows)]
pub fn shell_open_with(path: &str) {
    let target = path.to_string();
    std::thread::spawn(move || {
        let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
        let openas_verb: Vec<u16> = "openas\0".encode_utf16().collect();

        unsafe {
            let _ = CoInitializeEx(std::ptr::null_mut(), 2 /* COINIT_APARTMENTTHREADED */);

            // Attempt 1: Native ShellExecuteExW with verb "openas" and SEE_MASK_INVOKEIDLIST
            let mut exec_info: SHELLEXECUTEINFOW = std::mem::zeroed();
            exec_info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            exec_info.fMask = 0x0000000C; // SEE_MASK_INVOKEIDLIST
            exec_info.hwnd = get_process_window();
            exec_info.lpVerb = openas_verb.as_ptr();
            exec_info.lpFile = wide.as_ptr();
            exec_info.nShow = 1; // SW_SHOWNORMAL

            let success = ShellExecuteExW(&mut exec_info);
            if success == 0 {
                // Attempt 2: rundll32 shell32.dll,OpenAs_RunDLL <target>
                let _ = std::process::Command::new("rundll32.exe")
                    .arg("shell32.dll,OpenAs_RunDLL")
                    .arg(&target)
                    .spawn();
            }

            CoUninitialize();
        }
    });
}

#[cfg(windows)]
pub fn shell_open_folder_and_select(path: &str) {
    let target = path.to_string();
    std::thread::spawn(move || {
        let p = std::path::Path::new(&target);

        // 1. If path exists and is a directory, open that directory directly
        if p.is_dir() {
            let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                let res = ShellExecuteW(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    wide.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    1, // SW_SHOWNORMAL
                );
                if (res as isize) > 32 {
                    return;
                }
            }
            let _ = std::process::Command::new("explorer").arg(&target).spawn();
            return;
        }

        // 2. If path exists and is a file, select it in explorer
        if p.is_file() {
            let win_path = target.replace('/', "\\");
            let mut cmd = std::process::Command::new("explorer");
            cmd.raw_arg(format!(r#"/select,"{}""#, win_path));
            if cmd.spawn().is_ok() {
                return;
            }
        }

        // 3. If file doesn't exist at exact target path, search parent directory for matching stem
        let parent_dir = p.parent().and_then(|parent| {
            if parent.exists() && parent.is_dir() {
                Some(parent.to_path_buf())
            } else {
                None
            }
        });

        if let Some(parent) = parent_dir {
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                if let Ok(entries) = std::fs::read_dir(&parent) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.is_file() {
                            if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                                if name.starts_with(stem) && !name.ends_with(".part") && !name.ends_with(".ytdl") {
                                    let win_path = entry_path.to_string_lossy().replace('/', "\\");
                                    let mut cmd = std::process::Command::new("explorer");
                                    cmd.raw_arg(format!(r#"/select,"{}""#, win_path));
                                    if cmd.spawn().is_ok() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Fallback: open parent folder directly
            let parent_str = parent.to_string_lossy().to_string();
            let wide: Vec<u16> = parent_str.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                let res = ShellExecuteW(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    wide.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    1,
                );
                if (res as isize) > 32 {
                    return;
                }
            }
            let _ = std::process::Command::new("explorer").arg(&parent_str).spawn();
            return;
        }

        // 4. Fallback to general Downloads / Videos / C:\
        let fallback = Category::General.default_dir();
        let fallback_str = fallback.to_string_lossy().to_string();
        let wide: Vec<u16> = fallback_str.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let _ = ShellExecuteW(
                std::ptr::null_mut(),
                std::ptr::null(),
                wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            );
        }
    });
}

#[cfg(not(windows))]
pub fn show_download_popup_window() {}

#[cfg(not(windows))]
pub fn show_main_window() {}

#[cfg(not(windows))]
pub fn make_popup_topmost() {}

#[cfg(not(windows))]
pub fn remove_popup_topmost() {}

#[cfg(not(windows))]
pub fn hide_main_window() {}

#[cfg(not(windows))]
pub fn shell_open_file(path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

#[cfg(not(windows))]
pub fn shell_open_with(path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

#[cfg(not(windows))]
pub fn shell_open_folder_and_select(path: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
    }
}

fn build_system_tray(tray_tx: &std::sync::mpsc::Sender<String>, egui_ctx: egui::Context) -> Option<TrayIcon> {
    let tray_menu = Menu::new();
    let show_item = MenuItem::new("Show Cosmo Downloader", true, None);
    let add_item = MenuItem::new("Add Download URL...", true, None);
    let exit_item = MenuItem::new("Exit", true, None);
    let _ = tray_menu.append(&show_item);
    let _ = tray_menu.append(&add_item);
    let _ = tray_menu.append(&exit_item);

    let show_id = show_item.id().clone();
    let add_id = add_item.id().clone();
    let exit_id = exit_item.id().clone();

    let tx = tray_tx.clone();
    let ctx_menu = egui_ctx.clone();
    tray_icon::menu::MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
        if event.id == show_id {
            show_main_window();
            let _ = tx.send("show".to_string());
            ctx_menu.request_repaint();
        } else if event.id == add_id {
            show_main_window();
            let _ = tx.send("add".to_string());
            ctx_menu.request_repaint();
        } else if event.id == exit_id {
            std::process::exit(0);
        }
    }));

    let tx_tray = tray_tx.clone();
    let ctx_tray = egui_ctx.clone();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event: tray_icon::TrayIconEvent| {
        match event {
            tray_icon::TrayIconEvent::Click { button: tray_icon::MouseButton::Left, .. }
            | tray_icon::TrayIconEvent::DoubleClick { button: tray_icon::MouseButton::Left, .. } => {
                show_main_window();
                let _ = tx_tray.send("show".to_string());
                ctx_tray.request_repaint();
            }
            _ => {}
        }
    }));

    let tray_icon_img = tray_icon::Icon::from_rgba(ICON_RGBA.to_vec(), 32, 32).ok();
    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Cosmo Downloader v1.10");
    if let Some(ref icon) = tray_icon_img {
        builder = builder.with_icon(icon.clone());
    }
    builder.build().ok()
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum FilterTab {
    All,
    Downloading,
    Completed,
    Paused,
}

struct CosmoDownloaderApp {
    queue: Arc<QueueManager>,
    downloads: Vec<DownloadRecord>,
    active_progress: HashMap<String, DownloadProgress>,
    selected_tab: FilterTab,

    // Dialogs & Popup Mode
    is_popup_mode: bool,
    is_standalone_popup: bool,
    standalone_popup_file: Option<String>,
    popup_opened_at: Option<std::time::Instant>,
    show_add_dialog: bool,
    show_settings_dialog: bool,
    completed_dialog_record: Option<DownloadRecord>,
    is_completed_popup_mode: bool,
    active_popup_task_id: Option<String>,
    active_popup_filename: String,
    active_popup_url: String,
    active_popup_target_path: String,
    is_progress_popup_mode: bool,
    pending_task_rx: Option<tokio::sync::mpsc::Receiver<String>>,

    // Add dialog fields
    add_url: String,
    add_filename: String,
    add_category: String,
    add_save_dir: String,
    add_connections: usize,
    add_speed_limit_kb: u64,
    add_checksum: String,
    add_cookies: Option<String>,
    add_referer: Option<String>,
    add_file_size: Option<u64>,
    add_description: String,
    add_remember_category: bool,
    size_probe_rx: Option<mpsc::Receiver<(Option<u64>, Option<String>)>>,

    // Settings fields
    setting_max_concurrent: usize,
    setting_global_speed_kb: u64,
    setting_clipboard_monitor: bool,
    setting_minimize_to_tray: bool,
    setting_show_tray_icon: bool,
    setting_autostart: bool,
    setting_start_minimized: bool,
    setting_dont_show_complete_dialog: bool,

    // System tray
    tray_icon: Option<TrayIcon>,
    tray_tx: std::sync::mpsc::Sender<String>,
    tray_event_receiver: Option<std::sync::mpsc::Receiver<String>>,

    // Async event receivers
    event_rx: mpsc::Receiver<QueueEvent>,
    clipboard_rx: mpsc::Receiver<String>,
    browser_dl_rx: mpsc::Receiver<ExtensionDownloadRequest>,
    show_rx: mpsc::Receiver<()>,
    initial_minimized_handled: bool,
    last_db_refresh: std::time::Instant,
}

impl CosmoDownloaderApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Setup Cosmic Dark Theme
        let mut visuals = egui::Visuals::dark();
        visuals.window_rounding = Rounding::same(12.0);
        visuals.window_fill = Color32::from_rgb(13, 14, 27);
        visuals.panel_fill = Color32::from_rgb(18, 19, 36);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 26, 48);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(32, 35, 62);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(45, 48, 86);
        visuals.widgets.active.bg_fill = Color32::from_rgb(0, 229, 255);
        cc.egui_ctx.set_visuals(visuals);

        let mut is_standalone_popup = false;
        let mut standalone_popup_file = None;
        let mut standalone_req = None;

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
            }
        }

        let is_popup_arg = std::env::args().any(|a| a == "--popup") || is_standalone_popup;

        let db = Database::open_default().unwrap_or_else(|_| Database::open_in_memory().unwrap());
        let (event_tx, event_rx) = mpsc::channel(128);
        let queue = Arc::new(QueueManager::new(db.clone(), 3, 0, event_tx));

        ensure_native_host_registered();

        // Bridge server & notifier channels
        let (bridge_tx, bridge_rx) = mpsc::channel(32);
        let (show_tx, show_rx) = mpsc::channel(16);
        let daemon_online = std::net::TcpStream::connect("127.0.0.1:42424").is_ok();

        // Start background queue processing and bridge server if not handled by another daemon
        if !daemon_online {
            log_daemon("Starting queue worker and bridge server on port 42424");
            let q_worker = Arc::clone(&queue);
            let q_cancel = CancellationToken::new();
            tokio::spawn(async move {
                q_worker.run_queue_worker(q_cancel).await;
            });

            let ctx_wake = cc.egui_ctx.clone();
            let ctx_wake_dl = cc.egui_ctx.clone();
            let bridge_server = BridgeServer::new(Arc::clone(&queue), 42424)
                .with_notifier(bridge_tx)
                .with_show_notifier(show_tx)
                .with_wake_callback(move || {
                    #[cfg(windows)]
                    {
                        show_main_window();
                    }
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx_wake.request_repaint();
                })
                .with_download_wake_callback(move || {
                    log_daemon("Bridge server download wake callback triggered");
                    #[cfg(windows)]
                    {
                        show_download_popup_window();
                    }
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(560.0, 310.0)));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                    ctx_wake_dl.request_repaint();
                });
            let bridge_cancel = CancellationToken::new();
            tokio::spawn(async move {
                bridge_server.run(bridge_cancel).await;
            });
        }

        // Clipboard monitor
        let (clip_tx, clip_rx) = mpsc::channel(16);
        let saved_clip_monitor = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.get_setting("clipboard_monitor_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(true)
            })
        });

        if !is_standalone_popup && !daemon_online && saved_clip_monitor {
            let clip_cancel = CancellationToken::new();
            let ctx_wake_clip = cc.egui_ctx.clone();
            let clip_monitor = ClipboardMonitor::new(clip_cancel)
                .with_wake_callback(move || {
                    #[cfg(windows)]
                    {
                        show_download_popup_window();
                    }
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(560.0, 310.0)));
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                    ctx_wake_clip.request_repaint();
                });
            clip_monitor.start(clip_tx);
        }

        // System tray setup
        let (tray_tx, tray_rx) = std::sync::mpsc::channel();

        // Load saved settings from database
        let saved_minimize_to_tray = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.get_setting("minimize_to_tray_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(true)
            })
        });
        let saved_show_tray_icon = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.get_setting("show_tray_icon_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(true)
            })
        });

        let (saved_autostart, reg_start_minimized) = get_autostart_info();
        let saved_start_min = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.get_setting("start_minimized_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(reg_start_minimized)
            })
        });

        let egui_ctx = cc.egui_ctx.clone();
        let tray_icon = if !is_standalone_popup && !daemon_online && saved_show_tray_icon {
            build_system_tray(&tray_tx, egui_ctx)
        } else {
            None
        };

        let initial_records = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.list_downloads(None).await.unwrap_or_default()
            })
        });

        let mut app = Self {
            queue,
            downloads: initial_records,
            active_progress: HashMap::new(),
            selected_tab: FilterTab::All,
            is_popup_mode: is_popup_arg,
            is_standalone_popup,
            standalone_popup_file,
            popup_opened_at: if is_popup_arg { Some(std::time::Instant::now()) } else { None },
            show_add_dialog: is_popup_arg,
            show_settings_dialog: false,
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
            add_remember_category: true,
            size_probe_rx: None,
            setting_max_concurrent: 3,
            setting_global_speed_kb: 0,
            setting_clipboard_monitor: saved_clip_monitor,
            setting_minimize_to_tray: saved_minimize_to_tray,
            setting_show_tray_icon: saved_show_tray_icon,
            setting_autostart: saved_autostart,
            setting_start_minimized: saved_start_min,
            setting_dont_show_complete_dialog: false,
            tray_icon,
            tray_tx,
            tray_event_receiver: Some(tray_rx),
            event_rx,
            clipboard_rx: clip_rx,
            browser_dl_rx: bridge_rx,
            show_rx,
            initial_minimized_handled: false,
            last_db_refresh: std::time::Instant::now(),
        };

        if let Some(req) = standalone_req {
            app.prepare_download_dialog(&req.url, req.filename, req.cookies, req.referer, req.num_segments);
        }

        app
    }

    fn refresh_downloads(&mut self) {
        let db = self.queue.db().clone();
        let records = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                db.list_downloads(None).await.unwrap_or_default()
            })
        });
        self.downloads = records;
    }

    fn prepare_download_dialog(&mut self, url: &str, filename: Option<String>, cookies: Option<String>, referer: Option<String>, num_segments: Option<usize>) {
        self.add_url = url.trim().to_string();
        self.add_cookies = cookies.clone();
        self.add_referer = referer.clone();
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

        if rustget_core::youtube::is_streaming_site(&self.add_url) {
            if self.add_filename.is_empty() || self.add_filename.eq_ignore_ascii_case("watch") {
                self.add_filename = "YouTube_Video.mp4".to_string();
            } else if !self.add_filename.contains('.') {
                self.add_filename.push_str(".mp4");
            }
            self.add_category = "Video".to_string();
            self.add_save_dir = Category::Video.default_dir().to_string_lossy().to_string();
        } else {
            let cat = Category::detect_from_filename(&self.add_filename);
            self.add_category = cat.name().to_string();
            self.add_save_dir = cat.default_dir().to_string_lossy().to_string();
        }
        self.show_add_dialog = true;

        // Async probe for content-length and headers (only for direct files, streaming sites have dynamic adaptive streams)
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

    fn trigger_download(&mut self, start_paused: bool) {
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

    fn render_download_file_info(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, is_standalone: bool) {
        // IDM-styled Download File Info dialog matching user screenshot
        ui.horizontal(|ui| {
            // Left main form (approx 400px width)
            ui.vertical(|ui| {
                ui.set_width(400.0);

                // Row 1: URL
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("URL:").strong().size(12.0));
                    ui.add_space(28.0);
                    if ui.add(egui::TextEdit::singleline(&mut self.add_url).desired_width(330.0)).changed() {
                        if let Ok(parsed) = url::Url::parse(self.add_url.trim()) {
                            let name = rustget_core::probe::extract_filename(&reqwest::header::HeaderMap::new(), &parsed);
                            self.add_filename = rustget_core::probe::sanitize_filename(&name);
                            if rustget_core::youtube::is_streaming_site(&self.add_url) {
                                if self.add_filename.is_empty() || self.add_filename.eq_ignore_ascii_case("watch") {
                                    self.add_filename = "YouTube_Video.mp4".to_string();
                                } else if !self.add_filename.contains('.') {
                                    self.add_filename.push_str(".mp4");
                                }
                                self.add_category = "Video".to_string();
                                self.add_save_dir = Category::Video.default_dir().to_string_lossy().to_string();
                            } else {
                                let cat = Category::detect_from_filename(&self.add_filename);
                                self.add_category = cat.name().to_string();
                                self.add_save_dir = cat.default_dir().to_string_lossy().to_string();
                            }
                        }
                    }
                });
                ui.add_space(4.0);

                // Row 2: Category Dropdown + [+] button
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Category:").strong().size(12.0));
                    ui.add_space(2.0);
                    egui::ComboBox::from_id_salt("cat_select")
                        .selected_text(&self.add_category)
                        .width(185.0)
                        .show_ui(ui, |ui| {
                            for cat in ["Compressed", "Programs", "Video", "Music", "Documents", "General"] {
                                if ui.selectable_value(&mut self.add_category, cat.to_string(), cat).clicked() {
                                    if let Some(c) = Category::from_name(cat) {
                                        self.add_save_dir = c.default_dir().to_string_lossy().to_string();
                                    }
                                }
                            }
                        });
                    if ui.button(egui::RichText::new("+").strong()).on_hover_text("Add or configure categories").clicked() {
                        self.show_settings_dialog = true;
                    }
                });
                ui.add_space(4.0);

                // Row 3: Save As (Target path + Browse)
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Save As:").strong().size(12.0));
                    ui.add_space(8.0);
                    let mut full_target_path = PathBuf::from(&self.add_save_dir);
                    full_target_path.push(&self.add_filename);
                    let mut path_display = full_target_path.to_string_lossy().to_string();
                    if ui.add(egui::TextEdit::singleline(&mut path_display).desired_width(280.0)).changed() {
                        let p = Path::new(&path_display);
                        if let Some(parent) = p.parent() {
                            self.add_save_dir = parent.to_string_lossy().to_string();
                        }
                        if let Some(fname) = p.file_name() {
                            self.add_filename = fname.to_string_lossy().to_string();
                        }
                    }
                    if ui.button(egui::RichText::new("...").strong()).on_hover_text("Choose save destination").clicked() {
                        let start_dir = if !self.add_save_dir.trim().is_empty() && Path::new(self.add_save_dir.trim()).exists() {
                            PathBuf::from(self.add_save_dir.trim())
                        } else {
                            PathBuf::from("C:\\")
                        };
                        if let Some(file_path) = rfd::FileDialog::new()
                            .set_title("Save As")
                            .set_directory(&start_dir)
                            .set_file_name(&self.add_filename)
                            .save_file()
                        {
                            if let Some(parent) = file_path.parent() {
                                self.add_save_dir = parent.to_string_lossy().to_string();
                            }
                            if let Some(fname) = file_path.file_name() {
                                self.add_filename = fname.to_string_lossy().to_string();
                            }
                        }
                    }
                });
                ui.add_space(4.0);

                // Row 4: Remember this path checkbox
                let remember_label = format!("Remember this path for \"{}\" category", self.add_category);
                ui.checkbox(&mut self.add_remember_category, egui::RichText::new(remember_label).size(11.0).color(Color32::LIGHT_GRAY));
                ui.add_space(2.0);

                // Row 5: Path preview frame
                let frame = egui::Frame::none().fill(Color32::from_rgb(26, 28, 48)).inner_margin(Margin::symmetric(6.0, 4.0)).rounding(Rounding::same(4.0));
                frame.show(ui, |ui| {
                    ui.set_width(380.0);
                    let mut dir_display = self.add_save_dir.clone();
                    if !dir_display.ends_with('\\') && !dir_display.ends_with('/') {
                        dir_display.push('\\');
                    }
                    ui.label(egui::RichText::new(dir_display).size(11.0).color(Color32::from_rgb(140, 150, 180)));
                });
                ui.add_space(4.0);

                // Row 6: Description
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Description:").size(11.0).color(Color32::LIGHT_GRAY));
                    ui.add(egui::TextEdit::singleline(&mut self.add_description).desired_width(290.0));
                });
            });

            ui.add_space(10.0);

            // Right side: Icon & File Size preview
            ui.vertical(|ui| {
                ui.set_width(110.0);
                ui.add_space(10.0);

                let cat_icon = match self.add_category.as_str() {
                "Video" => "▶",
                "Music" => "♫",
                "Documents" => "≡",
                "Compressed" => "◆",
                "Programs" => "⚡",
                _ => "●",
                };
                ui.label(egui::RichText::new(cat_icon).size(42.0));
                ui.add_space(4.0);

                if rustget_core::youtube::is_streaming_site(&self.add_url) {
                    ui.label(egui::RichText::new("Adaptive Stream").strong().size(12.0).color(Color32::from_rgb(0, 240, 255)));
                } else if let Some(sz) = self.add_file_size {
                    ui.label(egui::RichText::new(SpeedTracker::format_bytes(sz)).strong().size(13.0).color(Color32::from_rgb(0, 240, 255)));
                } else {
                    ui.label(egui::RichText::new("Probing size...").size(11.0).color(Color32::GRAY));
                }

                ui.add_space(8.0);
                let preview_btn = egui::Button::new(
                    egui::RichText::new("Preview").size(11.0)
                ).min_size(Vec2::new(76.0, 24.0));
                ui.add_enabled(false, preview_btn).on_disabled_hover_text("Preview available during or after download");
            });
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        // Action Buttons Row (Download Later, Start Download, Cancel)
        ui.horizontal(|ui| {
            let later_btn = egui::Button::new(
                egui::RichText::new("Download Later").size(12.0),
            )
            .min_size(Vec2::new(120.0, 32.0));

            if ui.add(later_btn).clicked() {
                self.trigger_download(true);
                self.popup_opened_at = None;
                self.is_popup_mode = false;
                self.is_progress_popup_mode = false;
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                self.active_popup_task_id = None;
                if is_standalone {
                    remove_popup_topmost();
                    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                    hide_main_window();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title("Cosmo Downloader v1.10".to_string()));
                }
            }

            ui.add_space(8.0);

            let start_btn = egui::Button::new(
                egui::RichText::new("Start Download")
                    .strong()
                    .size(13.0)
                    .color(Color32::WHITE),
            )
            .fill(Color32::from_rgb(0, 140, 255))
            .min_size(Vec2::new(140.0, 32.0));

            if ui.add(start_btn).clicked() {
                self.trigger_download(false);
                self.is_popup_mode = false;
                self.popup_opened_at = None;
                self.is_progress_popup_mode = true;
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(520.0, 250.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::Title("Downloading - Cosmo Downloader".to_string()));
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                #[cfg(windows)]
                make_popup_topmost();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new("Cancel").min_size(Vec2::new(80.0, 32.0))).clicked() {
                    self.show_add_dialog = false;
                    self.add_url.clear();
                    self.add_filename.clear();
                    self.popup_opened_at = None;
                    self.is_popup_mode = false;
                    self.is_progress_popup_mode = false;
                    self.is_completed_popup_mode = false;
                    self.completed_dialog_record = None;
                    self.active_popup_task_id = None;
                    if is_standalone {
                        remove_popup_topmost();
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                        hide_main_window();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Cosmo Downloader v1.10".to_string()));
                    }
                }
            });
        });
    }

    fn render_download_progress(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, is_standalone: bool) {
        let task_id = self.active_popup_task_id.clone();
        let fname = if !self.active_popup_filename.is_empty() {
            self.active_popup_filename.clone()
        } else {
            "Downloading file...".to_string()
        };

        let prog = task_id.as_ref().and_then(|id| self.active_progress.get(id));
        let (downloaded, total, speed, eta, ratio) = if let Some(p) = prog {
            let total = p.total_bytes.unwrap_or(0);
            let r = if total > 0 {
                (p.downloaded_bytes as f32 / total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (
                SpeedTracker::format_bytes(p.downloaded_bytes),
                if total > 0 { SpeedTracker::format_bytes(total) } else { "Unknown".to_string() },
                SpeedTracker::format_speed(p.speed_bps),
                SpeedTracker::format_eta(p.eta_seconds),
                r,
            )
        } else {
            ("0 B".to_string(), "Connecting...".to_string(), "0 B/s".to_string(), "--:--".to_string(), 0.0)
        };

        // Header: Icon + Filename + Speed/ETA
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("↓").size(32.0).color(Color32::from_rgb(0, 240, 255)));
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(&fname).strong().size(13.5).color(Color32::WHITE));
                ui.add_space(2.0);
                ui.label(egui::RichText::new(format!("Transfer rate: {}  |  Time left: {}", speed, eta)).size(11.0).color(Color32::from_rgb(0, 240, 255)));
            });
        });

        ui.add_space(10.0);

        // Progress Bar
        let pct_text = format!("{:.1}%  ({} / {})", ratio * 100.0, downloaded, total);
        ui.add(
            egui::ProgressBar::new(ratio)
                .show_percentage()
                .text(pct_text)
                .animate(true)
        );

        ui.add_space(12.0);

        // Details grid: Status, File path
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Status:").size(11.0).color(Color32::LIGHT_GRAY));
            let status_text = if ratio >= 1.0 { "Finishing download..." } else if ratio > 0.0 { "Downloading..." } else { "Connecting to server..." };
            ui.label(egui::RichText::new(status_text).size(11.0).color(Color32::WHITE));
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Save path:").size(11.0).color(Color32::LIGHT_GRAY));
            ui.label(egui::RichText::new(&self.active_popup_target_path).size(11.0).color(Color32::from_rgb(140, 150, 180)));
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(10.0);

        // Controls Row: Pause/Resume, Cancel, Close
        ui.horizontal(|ui| {
            let q = Arc::clone(&self.queue);

            let is_paused = task_id.as_ref().map_or(false, |id| {
                self.downloads.iter().any(|d| &d.id == id && d.status == "paused")
            });

            let pause_label = if is_paused { "▶ Resume" } else { "⏸ Pause" };
            if ui.add(egui::Button::new(pause_label).min_size(Vec2::new(95.0, 30.0))).clicked() {
                if let Some(ref id) = task_id {
                    let id_c = id.clone();
                    let q_c = Arc::clone(&q);
                    tokio::spawn(async move {
                        if is_paused {
                            let _ = q_c.resume(&id_c).await;
                        } else {
                            let _ = q_c.pause(&id_c).await;
                        }
                    });
                }
            }

            ui.add_space(8.0);

            if ui.add(egui::Button::new("Cancel").min_size(Vec2::new(80.0, 30.0))).clicked() {
                if let Some(ref id) = task_id {
                    let id_c = id.clone();
                    let q_c = Arc::clone(&q);
                    tokio::spawn(async move {
                        let _ = q_c.remove(&id_c, false).await;
                    });
                }
                self.is_progress_popup_mode = false;
                self.active_popup_task_id = None;
                if is_standalone {
                    remove_popup_topmost();
                    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                    hide_main_window();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title("Cosmo Downloader v1.10".to_string()));
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Close button: hides progress dialog while download continues in background
                let close_btn = egui::Button::new(
                    egui::RichText::new("Close").strong().color(Color32::WHITE)
                )
                .fill(Color32::from_rgb(45, 48, 86))
                .min_size(Vec2::new(90.0, 30.0));

                if ui.add(close_btn).clicked() {
                    self.is_progress_popup_mode = false;
                    if is_standalone {
                        remove_popup_topmost();
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                        hide_main_window();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Cosmo Downloader v1.10".to_string()));
                    }
                }
            });
        });
    }

    fn render_download_complete(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, is_standalone: bool) {
        if let Some(record) = self.completed_dialog_record.clone() {
            let (sz_formatted, bytes_num) = if let Some(sz) = record.total_size {
                (SpeedTracker::format_bytes(sz), sz)
            } else if let Ok(meta) = std::fs::metadata(&record.target_path) {
                let len = meta.len();
                (SpeedTracker::format_bytes(len), len)
            } else {
                ("Unknown".to_string(), 0)
            };

            // Top section: Folder Icon + "Download complete" + size in bytes
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("✓").size(36.0).color(Color32::from_rgb(34, 197, 94)));
                ui.add_space(8.0);

                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Download complete")
                            .strong()
                            .size(13.0)
                            .color(Color32::WHITE),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(format!("Downloaded {} ({} Bytes)", sz_formatted, bytes_num))
                            .size(11.5)
                            .color(Color32::from_rgb(205, 210, 220)),
                    );
                });
            });

            ui.add_space(8.0);

            // "Address" label + Full width textedit
            ui.label(egui::RichText::new("Address").size(11.5).color(Color32::from_rgb(220, 220, 220)));
            ui.add_space(2.0);
            let mut url_disp = record.url.clone();
            ui.add(
                egui::TextEdit::singleline(&mut url_disp)
                    .desired_width(ui.available_width())
            );

            ui.add_space(6.0);

            // "The file saved as" label with quick Copy Path button
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("The file saved as").size(11.5).color(Color32::from_rgb(220, 220, 220)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Copy").size(10.5).color(Color32::from_rgb(0, 229, 255))).clicked() {
                        ui.output_mut(|o| o.copied_text = record.target_path.clone());
                    }
                });
            });
            ui.add_space(2.0);
            let mut path_disp = record.target_path.clone();
            ui.add(
                egui::TextEdit::singleline(&mut path_disp)
                    .id_source("complete_dialog_target_path")
                    .desired_width(ui.available_width())
            ).on_hover_text(format!("Full Path:\n{}", record.target_path));

            ui.add_space(12.0);

            // Row of 4 buttons matching IDM: [ Open ]  [ Open with... ]  [ Open folder ]  [ Close ]
            ui.horizontal(|ui| {
                let btn_size = Vec2::new(115.0, 26.0);

                let open_btn = egui::Button::new(
                    egui::RichText::new("Open").size(12.0).color(Color32::WHITE)
                ).min_size(btn_size);

                if ui.add(open_btn).clicked() {
                    shell_open_file(&record.target_path);
                }

                ui.add_space(6.0);

                let open_with_btn = egui::Button::new(
                    egui::RichText::new("Open with...").size(12.0).color(Color32::WHITE)
                ).min_size(btn_size);

                if ui.add(open_with_btn).clicked() {
                    shell_open_with(&record.target_path);
                }

                ui.add_space(6.0);

                let folder_btn = egui::Button::new(
                    egui::RichText::new("Open folder").size(12.0).color(Color32::WHITE)
                ).min_size(btn_size);

                if ui.add(folder_btn).clicked() {
                    shell_open_folder_and_select(&record.target_path);
                }

                ui.add_space(6.0);

                let close_btn = egui::Button::new(
                    egui::RichText::new("Close").size(12.0).color(Color32::WHITE)
                ).min_size(btn_size);

                if ui.add(close_btn).clicked() {
                    if self.is_standalone_popup {
                        if let Some(ref f) = self.standalone_popup_file {
                            let _ = std::fs::remove_file(f);
                        }
                        std::process::exit(0);
                    }
                    self.completed_dialog_record = None;
                    self.is_popup_mode = false;
                    self.is_progress_popup_mode = false;
                    self.is_completed_popup_mode = false;
                    self.active_popup_task_id = None;
                    self.popup_opened_at = None;
                    if is_standalone {
                        remove_popup_topmost();
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                        hide_main_window();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Cosmo Downloader v1.10".to_string()));
                    }
                }
            });

            ui.add_space(8.0);

            // Bottom row: [ ] Don't show this dialog again
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.setting_dont_show_complete_dialog, egui::RichText::new("Don't show this dialog again").size(11.0).color(Color32::LIGHT_GRAY));
            });
        }
    }
}

impl eframe::App for CosmoDownloaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.initial_minimized_handled {
            self.initial_minimized_handled = true;
            let start_minimized = std::env::args().any(|a| a == "--minimized");
            if start_minimized {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                if self.setting_minimize_to_tray && self.tray_icon.is_some() {
                    hide_main_window();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
            }
        }

        // Continuous redraw when downloads are active
        ctx.request_repaint_after(Duration::from_millis(100));

        // Intercept window close when minimize_to_tray is enabled and tray icon is active
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.is_progress_popup_mode {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.is_progress_popup_mode = false;
                self.active_popup_task_id = None;
                remove_popup_topmost();
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                hide_main_window();
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::Title("Cosmo Downloader v1.10".to_string()));
                return;
            }
            if self.is_completed_popup_mode {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                self.is_popup_mode = false;
                self.is_progress_popup_mode = false;
                self.active_popup_task_id = None;
                self.popup_opened_at = None;
                remove_popup_topmost();
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                hide_main_window();
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::Title("Cosmo Downloader v1.10".to_string()));
                return;
            }
            if self.is_popup_mode {
                if let Some(opened) = self.popup_opened_at {
                    if opened.elapsed() < Duration::from_millis(600) {
                        // Prevent spurious close events triggered immediately during window unhide/focus transition
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                        return;
                    }
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.is_popup_mode = false;
                self.is_progress_popup_mode = false;
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                self.active_popup_task_id = None;
                self.popup_opened_at = None;
                self.show_add_dialog = false;
                self.add_url.clear();
                self.add_filename.clear();
                remove_popup_topmost();
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                hide_main_window();
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::Title("Cosmo Downloader v1.10".to_string()));
                return;
            }
            // Main window [X] close: hide to background / tray without killing bridge listener
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            remove_popup_topmost();
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
            hide_main_window();
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            return;
        }

        // Receive pending task id for progress popup
        if let Some(ref mut rx) = self.pending_task_rx {
            if let Ok(id) = rx.try_recv() {
                self.active_popup_task_id = Some(id);
            }
        }

        // Receive probed file size and suggested filename if available
        if let Some(ref mut rx) = self.size_probe_rx {
            if let Ok((sz, suggested_name)) = rx.try_recv() {
                self.add_file_size = sz;
                if let Some(name) = suggested_name {
                    let cur_name = self.add_filename.trim().to_ascii_lowercase();
                    // If current name is generic, missing extension, or probe found a better filename with extension
                    if cur_name == "download" || cur_name == "download.bin" || !self.add_filename.contains('.') || (name.contains('.') && !cur_name.contains('.')) {
                        self.add_filename = name.clone();
                        let cat = Category::detect_from_filename(&name);
                        self.add_category = cat.name().to_string();
                        self.add_save_dir = cat.default_dir().to_string_lossy().to_string();
                    }
                }
            }
        }

        // Process queue events
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
                            let p = std::path::Path::new(&resolved_target);
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
                            let cat_str = Category::detect_from_filename(&resolved_target).name().to_string();
                            DownloadRecord {
                                id: id.clone(),
                                url: self.active_popup_url.clone(),
                                filename: std::path::Path::new(&resolved_target).file_name().and_then(|s| s.to_str()).unwrap_or(&self.active_popup_filename).to_string(),
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
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(560.0, 260.0)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Download complete".to_string()));
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                        #[cfg(windows)]
                        make_popup_topmost();
                    }
                }
                QueueEvent::TaskAdded(_) | QueueEvent::TaskFailed { .. } => {
                    self.refresh_downloads();
                }
                QueueEvent::StatusChanged { id, status } => {
                    if status == "completed" || status == "paused" || status == "failed" {
                        self.active_progress.remove(&id);
                    }
                    self.refresh_downloads();
                }
                QueueEvent::Progress { id, progress } => {
                    self.active_progress.insert(id, progress);
                }
            }
        }

        // Process external show requests (from second instance launch or HTTP /show)
        while let Ok(_) = self.show_rx.try_recv() {
            if self.is_popup_mode || self.is_progress_popup_mode || self.is_completed_popup_mode {
                // If any download popup or progress popup is active, NEVER destroy it or resize to 880x560!
                continue;
            }
            self.is_popup_mode = false;
            self.is_progress_popup_mode = false;
            self.is_completed_popup_mode = false;
            remove_popup_topmost();
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::Title("Cosmo Downloader v1.10".to_string()));
            show_main_window();
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
        }

        // Process browser extension download requests (IDM popup dialog - compact standalone)
        while let Ok(req) = self.browser_dl_rx.try_recv() {
            self.prepare_download_dialog(&req.url, req.filename, req.cookies, req.referer, req.num_segments);
            self.is_popup_mode = true;
            self.popup_opened_at = Some(std::time::Instant::now());
            self.is_progress_popup_mode = false;
            self.is_completed_popup_mode = false;
            self.completed_dialog_record = None;
            self.active_popup_task_id = None;
            self.show_add_dialog = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(560.0, 310.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::Title("Download File Info".to_string()));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
            #[cfg(windows)]
            make_popup_topmost();
        }

        // Process clipboard events
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
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(560.0, 310.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::Title("Download File Info".to_string()));
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                #[cfg(windows)]
                make_popup_topmost();
            }
        }

        // Process tray menu clicks
        if let Some(ref rx) = self.tray_event_receiver {
            while let Ok(action) = rx.try_recv() {
                match action.as_str() {
                    "show" => {
                        self.is_popup_mode = false;
                        self.is_progress_popup_mode = false;
                        self.is_completed_popup_mode = false;
                        remove_popup_topmost();
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Cosmo Downloader v1.10".to_string()));
                        show_main_window();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                        #[cfg(windows)]
                        make_popup_topmost();
                    }
                    "add" => {
                        self.is_popup_mode = false;
                        self.is_progress_popup_mode = false;
                        self.is_completed_popup_mode = false;
                        remove_popup_topmost();
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(880.0, 560.0)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title("Cosmo Downloader v1.10".to_string()));
                        show_main_window();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        self.show_add_dialog = true;
                    }
                    "exit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
        }

        // Standalone Compact Progress Popup Mode
        if self.is_progress_popup_mode {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(Color32::from_rgb(20, 22, 38)).inner_margin(Margin::same(16.0)))
                .show(ctx, |ui| {
                    self.render_download_progress(ui, ctx, true);
                });
            return;
        }

        // Standalone Compact Completion Popup Mode
        if self.is_completed_popup_mode && self.completed_dialog_record.is_some() {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(Color32::from_rgb(20, 22, 38)).inner_margin(Margin::same(16.0)))
                .show(ctx, |ui| {
                    self.render_download_complete(ui, ctx, true);
                });
            return;
        }

        // Standalone Compact Popup Mode (renders ONLY the Download File Info interface)
        if self.is_popup_mode {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(Color32::from_rgb(20, 22, 38)).inner_margin(Margin::same(16.0)))
                .show(ctx, |ui| {
                    self.render_download_file_info(ui, ctx, true);
                });
            return;
        }

        // Top Toolbar
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::none().fill(Color32::from_rgb(18, 19, 36)).inner_margin(Margin::same(12.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("Cosmo Downloader v1.10")
                            .strong()
                            .color(Color32::from_rgb(0, 240, 255)),
                    );
                    ui.add_space(20.0);

                    if ui.add(egui::Button::new("+ Add URL").min_size(Vec2::new(100.0, 30.0))).clicked() {
                        self.show_add_dialog = true;
                    }

                    if ui.add(egui::Button::new("⏸ Pause All").min_size(Vec2::new(90.0, 30.0))).clicked() {
                        let q = Arc::clone(&self.queue);
                        let ids: Vec<String> = self.downloads.iter()
                            .filter(|d| d.status == "downloading" || d.status == "queued")
                            .map(|d| d.id.clone())
                            .collect();
                        tokio::spawn(async move {
                            for id in ids {
                                let _ = q.pause(&id).await;
                            }
                        });
                    }

                    if ui.add(egui::Button::new("▶ Resume All").min_size(Vec2::new(100.0, 30.0))).clicked() {
                        let q = Arc::clone(&self.queue);
                        let ids: Vec<String> = self.downloads.iter()
                            .filter(|d| d.status == "paused" || d.status == "failed")
                            .map(|d| d.id.clone())
                            .collect();
                        tokio::spawn(async move {
                            for id in ids {
                                let _ = q.resume(&id).await;
                            }
                        });
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("Settings").min_size(Vec2::new(80.0, 30.0))).clicked() {
                            self.show_settings_dialog = true;
                        }

                        // Aggregate live speed calculation
                        let total_speed: f64 = self.active_progress.values().map(|p| p.speed_bps).sum();
                        if total_speed > 0.0 {
                            ui.label(
                                egui::RichText::new(format!("Speed: {}", SpeedTracker::format_speed(total_speed)))
                                    .color(Color32::from_rgb(0, 240, 255))
                                    .strong(),
                            );
                        }
                    });
                });
            });

        // Bottom Status & Credits Bar
        egui::TopBottomPanel::bottom("bottom_bar")
            .frame(egui::Frame::none().fill(Color32::from_rgb(14, 15, 29)).inner_margin(Margin::same(8.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Cosmo Downloader v1.10  |  Lead Architect & Developer: Mahedi  |  32-Segment Warp Engine Active")
                            .size(11.0)
                            .color(Color32::from_rgb(147, 197, 253)),
                    );
                });
            });

        // Left Navigation Bar (Categories & Filters)
        egui::SidePanel::left("side_panel")
            .resizable(false)
            .default_width(180.0)
            .frame(egui::Frame::none().fill(Color32::from_rgb(20, 22, 42)).inner_margin(Margin::same(12.0)))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("STATUS").size(11.0).color(Color32::from_rgb(168, 85, 247)).strong());
                ui.add_space(6.0);

                ui.selectable_value(&mut self.selected_tab, FilterTab::All, "All Downloads");
                ui.selectable_value(&mut self.selected_tab, FilterTab::Downloading, "Downloading");
                ui.selectable_value(&mut self.selected_tab, FilterTab::Completed, "Completed");
                ui.selectable_value(&mut self.selected_tab, FilterTab::Paused, "Paused");

                ui.add_space(20.0);
                ui.label(egui::RichText::new("CATEGORIES").size(11.0).color(Color32::from_rgb(168, 85, 247)).strong());
                ui.add_space(6.0);

                ui.label("Video");
                ui.label("Music");
                ui.label("Documents");
                ui.label("Compressed");
                ui.label("Programs");
            });

        // Main Content Area (Download Table)
        egui::CentralPanel::default().show(ctx, |ui| {
            let filtered_downloads: Vec<&DownloadRecord> = self.downloads.iter().filter(|d| {
                match self.selected_tab {
                    FilterTab::All => true,
                    FilterTab::Downloading => d.status == "downloading" || d.status == "queued",
                    FilterTab::Completed => d.status == "completed",
                    FilterTab::Paused => d.status == "paused",
                }
            }).collect();

            if filtered_downloads.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new("No downloads in this view").size(16.0).color(Color32::GRAY));
                    ui.label("Downloads from browser or clipboard appear here automatically.");
                });
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for record in filtered_downloads {
                        render_download_card(ui, record, &self.active_progress, &self.queue);
                        ui.add_space(6.0);
                    }
                });
            }
        });

        // Download File Info Modal Dialog (IDM Style)
        if self.show_add_dialog && !self.is_popup_mode {
            egui::Window::new("Download File Info")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(540.0);
                    self.render_download_file_info(ui, ctx, false);
                });
        }

        // Download Complete Modal Dialog (IDM Style, when main window is open)
        if self.completed_dialog_record.is_some() && !self.is_completed_popup_mode {
            egui::Window::new("Download Complete")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(520.0);
                    self.render_download_complete(ui, ctx, false);
                });
        }

        // Settings Modal Dialog
        if self.show_settings_dialog {
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(420.0);

                    ui.heading(egui::RichText::new("CosmoDownloader Preferences").size(15.0).color(Color32::from_rgb(0, 240, 255)));
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Max Concurrent Downloads:");
                        ui.add(egui::Slider::new(&mut self.setting_max_concurrent, 1..=10));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Global Speed Limit (KB/s, 0=unlimited):");
                        ui.add(egui::DragValue::new(&mut self.setting_global_speed_kb));
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.checkbox(&mut self.setting_clipboard_monitor, "Clipboard Monitor (Auto-detect copied URLs)");
                    ui.checkbox(&mut self.setting_minimize_to_tray, "Minimize to System Tray on Close [X]");
                    
                    let prev_tray = self.setting_show_tray_icon;
                    ui.checkbox(&mut self.setting_show_tray_icon, "Show Icon in Windows System Tray");
                    if prev_tray != self.setting_show_tray_icon {
                        if self.setting_show_tray_icon {
                            self.tray_icon = build_system_tray(&self.tray_tx, ctx.clone());
                        } else {
                            self.tray_icon = None;
                        }
                    }

                    ui.checkbox(&mut self.setting_autostart, "Start CosmoDownloader on Windows Boot");
                    if self.setting_autostart {
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            ui.checkbox(&mut self.setting_start_minimized, "Start Minimized (Silent in System Tray)");
                        });
                    }

                    ui.add_space(16.0);
                    if ui.add(egui::Button::new("Save & Close").min_size(Vec2::new(120.0, 30.0))).clicked() {
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
                        set_autostart_enabled(autostart, start_min);
                    }
                });
        }
    }
}

fn render_download_card(
    ui: &mut egui::Ui,
    record: &DownloadRecord,
    progress_map: &HashMap<String, DownloadProgress>,
    queue: &Arc<QueueManager>,
) {
    let card_frame = egui::Frame::none()
        .fill(Color32::from_rgb(26, 28, 54))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 48, 86)))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(10.0));

    card_frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // Category Icon & Filename
            let cat_icon = match record.category.as_str() {
                "Video" => "▶",
                "Music" => "♫",
                "Documents" => "≡",
                "Compressed" => "◆",
                "Programs" => "⚡",
                _ => "●",
            };
            ui.label(egui::RichText::new(cat_icon).size(18.0));
            ui.label(egui::RichText::new(&record.filename).strong().size(14.0));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Action Buttons
                let q_id = record.id.clone();
                let q = Arc::clone(queue);

                if ui.button("✕").on_hover_text("Delete Download").clicked() {
                    tokio::spawn(async move {
                        let _ = q.remove(&q_id, false).await;
                    });
                }

                if record.status == "completed" {
                    if ui.button("Open").on_hover_text("Show in Explorer").clicked() {
                        shell_open_folder_and_select(&record.target_path);
                    }
                } else if record.status == "downloading" || record.status == "queued" {
                    let q_id_pause = record.id.clone();
                    let q_pause = Arc::clone(queue);
                    if ui.button("⏸").on_hover_text("Pause").clicked() {
                        tokio::spawn(async move {
                            let _ = q_pause.pause(&q_id_pause).await;
                        });
                    }
                } else if record.status == "paused" || record.status == "failed" {
                    let q_id_res = record.id.clone();
                    let q_res = Arc::clone(queue);
                    if ui.button("▶").on_hover_text("Resume").clicked() {
                        tokio::spawn(async move {
                            let _ = q_res.resume(&q_id_res).await;
                        });
                    }
                }

                // Status Badge
                let (badge_text, badge_color) = match record.status.as_str() {
                    "downloading" => ("DOWNLOADING", Color32::from_rgb(0, 229, 255)),
                    "completed" => ("COMPLETED", Color32::from_rgb(34, 197, 94)),
                    "paused" => ("PAUSED", Color32::from_rgb(234, 179, 8)),
                    "failed" => ("FAILED", Color32::from_rgb(239, 68, 68)),
                    _ => ("QUEUED", Color32::from_rgb(156, 163, 175)),
                };

                ui.label(egui::RichText::new(badge_text).size(11.0).color(badge_color).strong());
            });
        });

        ui.add_space(4.0);

        // Progress metrics & Visualizer
        let live_prog = progress_map.get(&record.id);
        let downloaded = live_prog.map(|p| p.downloaded_bytes).unwrap_or(record.downloaded_bytes);
        let total = live_prog.and_then(|p| p.total_bytes).or(record.total_size);

        let pct = match total {
            Some(t) if t > 0 => ((downloaded as f64) / (t as f64) * 100.0).clamp(0.0, 100.0),
            _ => if record.status == "completed" { 100.0 } else { 0.0 },
        };

        // Multi-segment progress visualizer bar
        if let Some(prog) = live_prog {
            if !prog.segments.is_empty() && prog.segments.len() > 1 {
                ui.horizontal(|ui| {
                    let total_width = ui.available_width();
                    let num_segs = prog.segments.len() as f32;
                    let seg_width = (total_width / num_segs - 2.0).max(4.0);

                    for seg in &prog.segments {
                        let seg_pct = (seg.progress_percent() / 100.0) as f32;
                        let color = if seg.complete {
                            Color32::from_rgb(34, 197, 94)
                        } else if seg_pct > 0.0 {
                            Color32::from_rgb(0, 229, 255)
                        } else {
                            Color32::from_rgb(45, 48, 86)
                        };

                        ui.add(
                            egui::ProgressBar::new(seg_pct)
                                .desired_width(seg_width)
                                .fill(color),
                        );
                    }
                });
            } else {
                ui.add(egui::ProgressBar::new((pct / 100.0) as f32).fill(Color32::from_rgb(0, 229, 255)));
            }
        } else {
            let color = if record.status == "completed" {
                Color32::from_rgb(34, 197, 94)
            } else {
                Color32::from_rgb(0, 229, 255)
            };
            ui.add(egui::ProgressBar::new((pct / 100.0) as f32).fill(color));
        }

        ui.add_space(2.0);

        // Stats line: Size | Speed | ETA
        ui.horizontal(|ui| {
            let dl_str = SpeedTracker::format_bytes(downloaded);
            let tot_str = total.map(SpeedTracker::format_bytes).unwrap_or_else(|| "Unknown".to_string());
            ui.label(egui::RichText::new(format!("{:.1}% ({} / {})", pct, dl_str, tot_str)).size(11.0).color(Color32::LIGHT_GRAY));

            if let Some(prog) = live_prog {
                if prog.speed_bps > 0.0 {
                    ui.label(egui::RichText::new(format!("Speed: {}", SpeedTracker::format_speed(prog.speed_bps))).size(11.0).color(Color32::from_rgb(0, 240, 255)));
                    ui.label(egui::RichText::new(format!("ETA: {}", SpeedTracker::format_eta(prog.eta_seconds))).size(11.0).color(Color32::LIGHT_GRAY));
                }
            }
        });
    });
}

fn log_daemon(msg: &str) {
    if let Ok(mut dir) = std::env::current_exe() {
        dir.pop();
        let log_file = dir.join("cosmodownloader.log");
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!("[{}] {}\n", timestamp, msg);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
    }
}

fn ensure_native_host_registered() {
    #[cfg(windows)]
    std::thread::spawn(|| {
        if let Ok(mut exe) = std::env::current_exe() {
            exe.pop();
            let host_exe = exe.join("cosmodownloader-host.exe");
            let chrome_json = exe.join("com.cosmodownloader.downloader.chrome.json");
            if host_exe.exists() && chrome_json.exists() {
                let _ = std::process::Command::new(host_exe)
                    .arg("--register")
                    .arg(&chrome_json)
                    .creation_flags(CREATE_NO_WINDOW)
                    .status();
            }
        }
    });
}

fn main() -> eframe::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        log_daemon(&format!("PANIC: {:?}", info));
    }));
    tracing_subscriber::fmt().init();

    log_daemon(&format!("main() started with args: {:?}", std::env::args().collect::<Vec<_>>()));

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    // Single-instance guard so launching twice never opens two windows
    #[cfg(windows)]
    let _instance_mutex_guard: *mut std::ffi::c_void;
    #[cfg(windows)]
    unsafe {
        let mutex_handle = CreateMutexA(std::ptr::null_mut(), 1, b"CosmoDownloader_v1_0_SingleInstance_Mutex\0".as_ptr());
        if GetLastError() == 183 { // ERROR_ALREADY_EXISTS = 183
            // Check if existing instance is ACTUALLY running and listening on bridge port 42424
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
                        show_main_window();
                        let token = rustget_core::server::get_or_create_bridge_token();
                        let show_req = format!(
                            "POST /show HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Cosmo-Token: {}\r\nConnection: close\r\n\r\n",
                            token
                        );
                        let _ = std::io::Write::write_all(&mut stream, show_req.as_bytes());
                    }
                }
                // An active GUI instance exists, exit this duplicate safely
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
        .with_inner_size(if is_popup_arg { [560.0, 310.0] } else { [880.0, 560.0] })
        .with_min_inner_size([540.0, 300.0])
        .with_title(if is_popup_arg { "Download File Info" } else { "Cosmo Downloader v1.10" })
        .with_icon(Arc::new(window_icon));

    if is_popup_arg {
        viewport = viewport
            .with_visible(true)
            .with_active(true)
            .with_always_on_top();
    } else if start_minimized {
        viewport = viewport
            .with_visible(false)
            .with_active(false);
    } else {
        viewport = viewport
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
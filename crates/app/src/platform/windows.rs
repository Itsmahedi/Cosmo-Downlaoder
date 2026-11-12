#![cfg(windows)]

use std::os::windows::process::CommandExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);
static SUBCLASS_INSTALLED: AtomicBool = AtomicBool::new(false);
static PREV_WNDPROC: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
static EGUI_CTX_FOR_SHUTDOWN: OnceLock<eframe::egui::Context> = OnceLock::new();

pub const SHUTDOWN_EVENT_NAME: &[u8] = b"Local\\CosmoDownloader_Shutdown_Event\0";

#[repr(C)]
#[allow(non_snake_case)]
pub struct SHELLEXECUTEINFOW {
    pub cbSize: u32,
    pub fMask: u32,
    pub hwnd: *mut std::ffi::c_void,
    pub lpVerb: *const u16,
    pub lpFile: *const u16,
    pub lpParameters: *const u16,
    pub lpDirectory: *const u16,
    pub nShow: i32,
    pub hInstApp: *mut std::ffi::c_void,
    pub lpIDList: *mut std::ffi::c_void,
    pub lpClass: *const u16,
    pub hkeyClass: *mut std::ffi::c_void,
    pub dwHotKey: u32,
    pub hIconOrMonitor: *mut std::ffi::c_void,
    pub hProcess: *mut std::ffi::c_void,
}

#[link(name = "shell32")]
#[link(name = "ole32")]
extern "system" {
    pub fn CoInitializeEx(pvReserved: *mut std::ffi::c_void, dwCoInit: u32) -> i32;
    pub fn CoUninitialize();
    pub fn ShellExecuteW(
        hwnd: *mut std::ffi::c_void,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> isize;
    pub fn ShellExecuteExW(pExecInfo: *mut SHELLEXECUTEINFOW) -> i32;
}

extern "system" {
    pub fn CreateMutexA(lpMutexAttributes: *mut std::ffi::c_void, bInitialOwner: i32, lpName: *const u8) -> *mut std::ffi::c_void;
    pub fn CreateEventA(lpEventAttributes: *mut std::ffi::c_void, bManualReset: i32, bInitialState: i32, lpName: *const u8) -> *mut std::ffi::c_void;
    pub fn OpenEventA(dwDesiredAccess: u32, bInheritHandle: i32, lpName: *const u8) -> *mut std::ffi::c_void;
    pub fn SetEvent(hEvent: *mut std::ffi::c_void) -> i32;
    pub fn WaitForSingleObject(hHandle: *mut std::ffi::c_void, dwMilliseconds: u32) -> u32;
    pub fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
    pub fn GetLastError() -> u32;
    pub fn GetCurrentProcessId() -> u32;
    pub fn GetWindowThreadProcessId(hWnd: *mut std::ffi::c_void, lpdwProcessId: *mut u32) -> u32;
    pub fn EnumWindows(lpEnumFunc: Option<unsafe extern "system" fn(*mut std::ffi::c_void, isize) -> i32>, lParam: isize) -> i32;
    pub fn GetWindowLongW(hWnd: *mut std::ffi::c_void, nIndex: i32) -> i32;
    pub fn GetWindowTextLengthW(hWnd: *mut std::ffi::c_void) -> i32;
    pub fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> *mut std::ffi::c_void;
    pub fn ShowWindow(hWnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
    pub fn SetForegroundWindow(hWnd: *mut std::ffi::c_void) -> i32;
    pub fn SetWindowPos(hWnd: *mut std::ffi::c_void, hWndInsertAfter: *mut std::ffi::c_void, X: i32, Y: i32, cx: i32, cy: i32, uFlags: u32) -> i32;
    pub fn GetSystemMetrics(nIndex: i32) -> i32;
    pub fn SetWindowLongPtrW(hWnd: *mut std::ffi::c_void, nIndex: i32, dwNewLong: isize) -> isize;
    pub fn CallWindowProcW(lpPrevWndFunc: isize, hWnd: *mut std::ffi::c_void, Msg: u32, wParam: usize, lParam: isize) -> isize;
}

unsafe extern "system" fn shutdown_wndproc(
    hwnd: *mut std::ffi::c_void,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    const WM_QUERYENDSESSION: u32 = 0x0011;
    const WM_ENDSESSION: u32 = 0x0016;
    if msg == WM_QUERYENDSESSION {
        SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
        if let Some(ctx) = EGUI_CTX_FOR_SHUTDOWN.get() {
            ctx.request_repaint();
        }
        return 1;
    }
    if msg == WM_ENDSESSION && wparam != 0 {
        SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
        if let Some(ctx) = EGUI_CTX_FOR_SHUTDOWN.get() {
            ctx.request_repaint();
        }
    }
    let prev = PREV_WNDPROC.load(Ordering::SeqCst);
    if prev != 0 {
        CallWindowProcW(prev, hwnd, msg, wparam, lparam)
    } else {
        0
    }
}

/// Subclasses the app's HWND once to intercept Windows RestartManager / session end messages.
pub fn init_shutdown_listener(ctx: &eframe::egui::Context) {
    let _ = EGUI_CTX_FOR_SHUTDOWN.set(ctx.clone());
    if !SUBCLASS_INSTALLED.swap(true, Ordering::SeqCst) {
        // Spawn zero-CPU waiter on named Win32 shutdown event signaled by installer or `--quit`
        std::thread::spawn(|| unsafe {
            let h = CreateEventA(std::ptr::null_mut(), 1, 0, SHUTDOWN_EVENT_NAME.as_ptr());
            if !h.is_null() {
                WaitForSingleObject(h, u32::MAX);
                SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
                if let Some(c) = EGUI_CTX_FOR_SHUTDOWN.get() {
                    c.request_repaint();
                }
                CloseHandle(h);
            }
        });
    }

    if PREV_WNDPROC.load(Ordering::SeqCst) == 0 {
        let hwnd = get_process_window();
        if !hwnd.is_null() {
            unsafe {
                let prev = SetWindowLongPtrW(hwnd, -4, shutdown_wndproc as *const () as isize);
                if prev != 0 {
                    PREV_WNDPROC.store(prev, Ordering::SeqCst);
                }
            }
        }
    }
}

/// Returns true if OS RestartManager, Windows logoff/shutdown, or `--quit`/installer requested exit.
pub fn is_external_shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::SeqCst)
}

/// Signals an already-running instance to gracefully save and exit. Returns true if event existed.
pub fn signal_existing_instance_shutdown() -> bool {
    unsafe {
        // EVENT_MODIFY_STATE = 0x0002
        let h = OpenEventA(0x0002, 0, SHUTDOWN_EVENT_NAME.as_ptr());
        if !h.is_null() {
            let ok = SetEvent(h);
            CloseHandle(h);
            return ok != 0;
        }
        false
    }
}

pub const HWND_TOPMOST: *mut std::ffi::c_void = (-1isize) as *mut std::ffi::c_void;
pub const HWND_NOTOPMOST: *mut std::ffi::c_void = (-2isize) as *mut std::ffi::c_void;

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
                let style = GetWindowLongW(hwnd, -16) as u32;
                if (style & 0x40000000) == 0 {
                    let text_len = GetWindowTextLengthW(hwnd);
                    if text_len > 0 {
                        data.hwnd = hwnd;
                        return 0;
                    } else if data.hwnd.is_null() {
                        data.hwnd = hwnd;
                    }
                }
            }
            1
        }

        EnumWindows(Some(enum_proc), &mut data as *mut _ as isize);
        if !data.hwnd.is_null() {
            return data.hwnd;
        }

        let titles = [
            "Download File Info\0",
            "Downloading - Cosmo Downloader\0",
            "Download complete\0",
            "Download Complete\0",
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

pub fn show_download_popup_window() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            let screen_w = GetSystemMetrics(0);
            let screen_h = GetSystemMetrics(1);
            let w = 560;
            let h = 310;
            let x = if screen_w > w { (screen_w - w) / 2 } else { 100 };
            let y = if screen_h > h { (screen_h - h) / 2 } else { 100 };
            SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, 0x0040);
            ShowWindow(hwnd, 9);
            ShowWindow(hwnd, 5);
            SetForegroundWindow(hwnd);
        }
    }
}

pub fn show_main_window() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, 9);
            ShowWindow(hwnd, 5);
            SetForegroundWindow(hwnd);
        }
    }
}

pub fn make_popup_topmost() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, 9);
            ShowWindow(hwnd, 5);
            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0040);
            SetForegroundWindow(hwnd);
        }
    }
}

pub fn remove_popup_topmost() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, 0x0001 | 0x0002);
        }
    }
}

pub fn hide_main_window() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, 0);
        }
    }
}

pub fn shell_open_file(path: &str) {
    let target = path.to_string();
    std::thread::spawn(move || {
        let p = Path::new(&target);
        let actual_target = if !p.exists() {
            let mut resolved = target.clone();
            if let Some(parent) = p.parent() {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            let ep = entry.path();
                            if ep.is_file() {
                                if let Some(name) = ep.file_name().and_then(|n| n.to_str()) {
                                    if name.starts_with(stem) && !name.ends_with(".part") && !name.ends_with(".ytdl") {
                                        resolved = ep.to_string_lossy().to_string();
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
                std::ptr::null(),
                wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
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

pub fn shell_open_with(path: &str) {
    let target = path.to_string();
    std::thread::spawn(move || {
        let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
        let openas_verb: Vec<u16> = "openas\0".encode_utf16().collect();
        unsafe {
            let _ = CoInitializeEx(std::ptr::null_mut(), 2);
            let mut exec_info: SHELLEXECUTEINFOW = std::mem::zeroed();
            exec_info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            exec_info.fMask = 0x0000000C;
            exec_info.hwnd = get_process_window();
            exec_info.lpVerb = openas_verb.as_ptr();
            exec_info.lpFile = wide.as_ptr();
            exec_info.nShow = 1;

            let success = ShellExecuteExW(&mut exec_info);
            if success == 0 {
                let _ = std::process::Command::new("rundll32.exe")
                    .arg("shell32.dll,OpenAs_RunDLL")
                    .arg(&target)
                    .spawn();
            }
            CoUninitialize();
        }
    });
}

pub fn shell_open_folder_and_select(path: &str) {
    let target = path.to_string();
    std::thread::spawn(move || {
        let p = Path::new(&target);
        if p.is_dir() {
            let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
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
            let _ = std::process::Command::new("explorer").arg(&target).spawn();
            return;
        }

        if p.is_file() {
            let win_path = target.replace('/', "\\");
            let mut cmd = std::process::Command::new("explorer");
            cmd.raw_arg(format!(r#"/select,"{}""#, win_path));
            if cmd.spawn().is_ok() {
                return;
            }
        }

        if let Some(parent) = p.parent() {
            if parent.exists() && parent.is_dir() {
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
            }
        }
    });
}

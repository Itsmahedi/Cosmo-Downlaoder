#[cfg(windows)]
pub mod windows;
#[cfg(not(windows))]
pub mod unix;
pub mod tray;

#[cfg(windows)]
pub use windows::*;
#[cfg(not(windows))]
pub use unix::*;
pub use tray::*;

pub fn log_daemon(msg: &str) {
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

pub fn get_autostart_info() -> (bool, bool) {
    #[cfg(windows)]
    {
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
    }
    (false, false)
}

pub fn set_autostart_enabled(enable: bool, start_minimized: bool) {
    #[cfg(windows)]
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
    #[cfg(not(windows))]
    let _ = (enable, start_minimized);
}

#[cfg(windows)]
fn run_reg_command(args: &[&str]) -> std::io::Result<std::process::Output> {
    use std::os::windows::process::CommandExt;
    let mut cmd = std::process::Command::new("reg");
    cmd.args(args);
    cmd.creation_flags(windows::CREATE_NO_WINDOW);
    cmd.output()
}

pub fn system_prefers_dark() -> bool {
    #[cfg(windows)]
    {
        if let Ok(out) = run_reg_command(&[
            "query",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
            "/v",
            "AppsUseLightTheme",
        ]) {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                if s.contains("0x1") {
                    return false;
                }
                if s.contains("0x0") {
                    return true;
                }
            }
        }
    }
    true
}

pub fn ensure_native_host_registered() {
    #[cfg(windows)]
    std::thread::spawn(|| {
        use std::os::windows::process::CommandExt;
        if let Ok(mut exe) = std::env::current_exe() {
            exe.pop();
            let host_exe = exe.join("cosmodownloader-host.exe");
            let chrome_json = exe.join("com.cosmodownloader.downloader.chrome.json");
            if host_exe.exists() && chrome_json.exists() {
                let _ = std::process::Command::new(host_exe)
                    .arg("--register")
                    .arg(&chrome_json)
                    .creation_flags(windows::CREATE_NO_WINDOW)
                    .status();
            }
        }
    });
}

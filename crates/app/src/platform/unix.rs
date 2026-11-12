#![cfg(not(windows))]

use std::path::Path;

pub fn show_download_popup_window() {}
pub fn show_main_window() {}
pub fn make_popup_topmost() {}
pub fn remove_popup_topmost() {}
pub fn hide_main_window() {}

pub fn shell_open_file(path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

pub fn shell_open_with(path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

pub fn shell_open_folder_and_select(path: &str) {
    if let Some(parent) = Path::new(path).parent() {
        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
    }
}

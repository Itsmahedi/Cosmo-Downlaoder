use eframe::egui;
use tray_icon::{
    menu::{Menu, MenuItem},
    TrayIcon, TrayIconBuilder,
};

pub fn build_system_tray(
    tray_tx: &std::sync::mpsc::Sender<String>,
    egui_ctx: egui::Context,
    icon_rgba: &[u8],
    app_title: &str,
) -> Option<TrayIcon> {
    #[cfg(target_os = "linux")]
    {
        if gtk::init().is_err() {
            return None;
        }
    }

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
            #[cfg(windows)]
            super::windows::show_main_window();
            let _ = tx.send("show".to_string());
            ctx_menu.request_repaint();
        } else if event.id == add_id {
            #[cfg(windows)]
            super::windows::show_main_window();
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
                #[cfg(windows)]
                super::windows::show_main_window();
                let _ = tx_tray.send("show".to_string());
                ctx_tray.request_repaint();
            }
            _ => {}
        }
    }));

    let tray_icon_img = tray_icon::Icon::from_rgba(icon_rgba.to_vec(), 32, 32).ok();
    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip(app_title);
    if let Some(ref icon) = tray_icon_img {
        builder = builder.with_icon(icon.clone());
    }
    builder.build().ok()
}

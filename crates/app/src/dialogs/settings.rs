use std::path::Path;
use eframe::egui::{self, Vec2};
use crate::theme::{shapes, ThemeColors};
use crate::icons;

pub struct SettingsDialogOutcome {
    pub save_and_close: bool,
}

pub struct SettingsViewState<'a> {
    pub max_concurrent: &'a mut usize,
    pub global_speed_kb: &'a mut u64,
    pub default_connections: &'a mut usize,
    pub default_save_dir: &'a mut String,
    pub proxy_url: &'a mut String,
    pub clipboard_monitor: &'a mut bool,
    pub notifications_enabled: &'a mut bool,
    pub open_main_on_download_start: &'a mut bool,
    pub dont_show_complete_dialog: &'a mut bool,
    pub minimize_to_tray: &'a mut bool,
    pub show_tray_icon: &'a mut bool,
    pub autostart: &'a mut bool,
    pub start_minimized: &'a mut bool,
}

pub fn render_settings_dialog(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    state: &mut SettingsViewState<'_>,
) -> SettingsDialogOutcome {
    let mut outcome = SettingsDialogOutcome { save_and_close: false };

    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::hover());
        icons::draw_settings(ui.painter(), icon_rect, pal.accent, pal.surface);

        ui.add_space(shapes::S2);
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Options")
                    .strong()
                    .size(16.0)
                    .color(pal.text),
            );
            ui.label(
                egui::RichText::new(format!("Cosmo Downloader v{}", env!("CARGO_PKG_VERSION")))
                    .size(12.0)
                    .color(pal.muted),
            );
        });
    });

    ui.add_space(shapes::S3);

    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
        ui.label(egui::RichText::new("Max concurrent downloads").size(12.0).color(pal.muted));
        ui.add_space(2.0);
        ui.add(egui::Slider::new(state.max_concurrent, 1..=10).text("tasks"));

        ui.add_space(shapes::S2);

        ui.label(egui::RichText::new("Default connections per download").size(12.0).color(pal.muted));
        ui.add_space(2.0);
        ui.add(egui::Slider::new(state.default_connections, 1..=32).text("threads"));

        ui.add_space(shapes::S2);

        ui.label(egui::RichText::new("Speed limit (KB/s, 0 = unlimited)").size(12.0).color(pal.muted));
        ui.add_space(2.0);
        ui.add(egui::DragValue::new(state.global_speed_kb).speed(50.0).range(0..=1_000_000));

        ui.add_space(shapes::S2);

        ui.label(egui::RichText::new("Default save folder (blank = auto by category)").size(12.0).color(pal.muted));
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 74.0).max(120.0);
            ui.add(
                egui::TextEdit::singleline(state.default_save_dir)
                    .hint_text("Category default (Downloads/...)")
                    .desired_width(w),
            );
            if ui.button("Browse").clicked() {
                let mut picker = rfd::FileDialog::new().set_title("Select Default Download Folder");
                if !state.default_save_dir.trim().is_empty() && Path::new(state.default_save_dir.trim()).exists() {
                    picker = picker.set_directory(state.default_save_dir.trim());
                }
                if let Some(dir) = picker.pick_folder() {
                    *state.default_save_dir = dir.to_string_lossy().to_string();
                }
            }
        });

        ui.add_space(shapes::S2);

        ui.label(egui::RichText::new("Proxy URL (optional, e.g. http://127.0.0.1:8080)").size(12.0).color(pal.muted));
        ui.add(
            egui::TextEdit::singleline(state.proxy_url)
                .hint_text("Direct connection (no proxy)")
                .desired_width(ui.available_width()),
        );

        ui.add_space(shapes::S3);

        let div_rect = ui.allocate_space(Vec2::new(ui.available_width(), 1.0)).1;
        ui.painter().rect_filled(div_rect, 0.0, pal.line);

        ui.add_space(shapes::S2);

        ui.checkbox(state.clipboard_monitor, "Catch links from clipboard automatically");
        ui.add_space(shapes::S1);
        ui.checkbox(state.notifications_enabled, "Show desktop notifications on completion or error");
        ui.add_space(shapes::S1);
        ui.checkbox(state.open_main_on_download_start, "Open the main window when a download starts");
        ui.add_space(shapes::S1);
        ui.checkbox(state.dont_show_complete_dialog, "Hide 'Download Complete' popup window");
        ui.add_space(shapes::S1);
        ui.checkbox(state.minimize_to_tray, "Minimize to system tray on close");
        ui.add_space(shapes::S1);
        ui.checkbox(state.show_tray_icon, "Show icon in system tray");
        ui.add_space(shapes::S1);
        ui.checkbox(state.autostart, "Launch Cosmo on system startup");

        if *state.autostart {
            ui.horizontal(|ui| {
                ui.add_space(18.0);
                ui.checkbox(state.start_minimized, "Start minimized in tray");
            });
        }
    });

    ui.add_space(shapes::S3);

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let save_btn = egui::Button::new(
            egui::RichText::new("Save & Close").size(13.0).color(pal.on_accent),
        )
        .fill(pal.accent)
        .min_size(Vec2::new(110.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(save_btn).clicked() {
            outcome.save_and_close = true;
        }
    });

    outcome
}

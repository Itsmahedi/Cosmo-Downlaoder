use eframe::egui::{self, Vec2};
use crate::theme::{colors, shapes};
use crate::icons;

pub struct SettingsDialogOutcome {
    pub save_and_close: bool,
}

pub fn render_settings_dialog(
    ui: &mut egui::Ui,
    max_concurrent: &mut usize,
    global_speed_kb: &mut u64,
    clipboard_monitor: &mut bool,
    minimize_to_tray: &mut bool,
    show_tray_icon: &mut bool,
    autostart: &mut bool,
    start_minimized: &mut bool,
) -> SettingsDialogOutcome {
    let mut outcome = SettingsDialogOutcome { save_and_close: false };

    // Header: Settings title + version
    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::hover());
        icons::draw_settings(ui.painter(), icon_rect, colors::ACCENT);

        ui.add_space(shapes::S2);
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Preferences")
                    .strong()
                    .size(16.0)
                    .color(colors::TEXT),
            );
            ui.label(
                egui::RichText::new(format!("Cosmo Downloader v{}", env!("CARGO_PKG_VERSION")))
                    .size(12.0)
                    .color(colors::MUTED),
            );
        });
    });

    ui.add_space(shapes::S3);

    // Concurrency Slider
    ui.label(egui::RichText::new("Max concurrent downloads").size(12.0).color(colors::MUTED));
    ui.add_space(2.0);
    ui.add(egui::Slider::new(max_concurrent, 1..=10).text("tasks"));

    ui.add_space(shapes::S3);

    // Global Speed Limit
    ui.label(egui::RichText::new("Speed limit (KB/s, 0 = unlimited)").size(12.0).color(colors::MUTED));
    ui.add_space(2.0);
    ui.add(egui::DragValue::new(global_speed_kb).speed(50.0).range(0..=1_000_000));

    ui.add_space(shapes::S3);

    // Divider
    let div_rect = ui.allocate_space(Vec2::new(ui.available_width(), 1.0)).1;
    ui.painter().rect_filled(div_rect, 0.0, colors::LINE);

    ui.add_space(shapes::S3);

    // Checkboxes / Switches
    ui.checkbox(clipboard_monitor, "Catch links from clipboard automatically");
    ui.add_space(shapes::S1);
    ui.checkbox(minimize_to_tray, "Minimize to system tray on close");
    ui.add_space(shapes::S1);
    ui.checkbox(show_tray_icon, "Show icon in system tray");
    ui.add_space(shapes::S1);
    ui.checkbox(autostart, "Launch Cosmo on system startup");

    if *autostart {
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            ui.checkbox(start_minimized, "Start minimized in tray");
        });
    }

    ui.add_space(shapes::S4);

    // Action Buttons
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let save_btn = egui::Button::new(
            egui::RichText::new("Save & Close").size(13.0).color(colors::ON_ACCENT),
        )
        .fill(colors::ACCENT)
        .min_size(Vec2::new(110.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(save_btn).clicked() {
            outcome.save_and_close = true;
        }
    });

    outcome
}

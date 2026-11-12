use eframe::egui::{self, Color32, Stroke, Vec2};
use crate::theme::{colors, shapes};
use crate::icons;
use rustget_core::{
    downloader::DownloadProgress,
    speed::SpeedTracker,
};

pub struct ProgressDialogOutcome {
    pub toggle_pause: bool,
    pub cancel: bool,
    pub close: bool,
}

pub fn render_progress_dialog(
    ui: &mut egui::Ui,
    filename: &str,
    target_path: &str,
    progress: Option<&DownloadProgress>,
    is_paused: bool,
) -> ProgressDialogOutcome {
    let mut outcome = ProgressDialogOutcome {
        toggle_pause: false,
        cancel: false,
        close: false,
    };

    let (downloaded, total, speed, eta, ratio) = if let Some(p) = progress {
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

    // Header: Download icon + Filename
    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
        icons::draw_download(ui.painter(), icon_rect, colors::ACCENT);

        ui.add_space(shapes::S2);
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(filename)
                    .strong()
                    .size(14.0)
                    .color(colors::TEXT),
            );
            ui.label(
                egui::RichText::new(format!("{} of {}  •  {}, {} left", downloaded, total, speed, eta))
                    .size(12.0)
                    .color(colors::MUTED)
                    .monospace(),
            );
        });
    });

    ui.add_space(shapes::S3);

    // 3px Progress Bar
    let track_rect = ui.allocate_space(Vec2::new(ui.available_width(), 4.0)).1;
    ui.painter().rect_filled(track_rect, shapes::R_SM, colors::RAISE);
    let fill_w = (track_rect.width() * ratio).max(if ratio > 0.0 { 3.0 } else { 0.0 });
    let fill_rect = eframe::egui::Rect::from_min_size(track_rect.min, Vec2::new(fill_w, 4.0));
    let fill_col = if is_paused { colors::MUTED } else { colors::ACCENT };
    ui.painter().rect_filled(fill_rect, shapes::R_SM, fill_col);

    ui.add_space(shapes::S2);

    ui.label(
        egui::RichText::new(format!("Saving to: {}", target_path))
            .size(11.5)
            .color(colors::MUTED),
    );

    ui.add_space(shapes::S3);

    // Divider
    let div_rect = ui.allocate_space(Vec2::new(ui.available_width(), 1.0)).1;
    ui.painter().rect_filled(div_rect, 0.0, colors::LINE);

    ui.add_space(shapes::S3);

    // Controls Row
    ui.horizontal(|ui| {
        let pause_label = if is_paused { "Resume" } else { "Pause" };
        let pause_btn = egui::Button::new(
            egui::RichText::new(pause_label).size(13.0).color(colors::TEXT),
        )
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, colors::LINE))
        .min_size(Vec2::new(90.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(pause_btn).clicked() {
            outcome.toggle_pause = true;
        }

        ui.add_space(shapes::S2);

        let cancel_btn = egui::Button::new(
            egui::RichText::new("Cancel").size(13.0).color(colors::MUTED),
        )
        .fill(Color32::TRANSPARENT)
        .min_size(Vec2::new(70.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(cancel_btn).clicked() {
            outcome.cancel = true;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close_btn = egui::Button::new(
                egui::RichText::new("Hide to background").size(13.0).color(colors::ON_ACCENT),
            )
            .fill(colors::ACCENT)
            .min_size(Vec2::new(140.0, shapes::CONTROL_HEIGHT))
            .rounding(shapes::R_SM);

            if ui.add(close_btn).clicked() {
                outcome.close = true;
            }
        });
    });

    outcome
}

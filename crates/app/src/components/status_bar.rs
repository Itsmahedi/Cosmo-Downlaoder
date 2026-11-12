use eframe::egui::{self, pos2, Pos2, Rect, Vec2};
use rustget_core::speed::SpeedTracker;
use crate::theme::ThemeColors;

pub fn render_status_bar(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    shown_count: usize,
    total_speed_bps: f64,
    version: &str,
) {
    let bar_rect = ui.allocate_space(Vec2::new(ui.available_width(), 28.0)).1;
    ui.painter().rect_filled(bar_rect, 0.0, pal.raise);
    ui.painter().rect_filled(
        Rect::from_min_max(bar_rect.min, Pos2::new(bar_rect.right(), bar_rect.top() + 1.0)),
        0.0,
        pal.line,
    );

    let speed_text = if total_speed_bps > 0.0 {
        SpeedTracker::format_speed(total_speed_bps)
    } else {
        "0.0 MB/s".to_string()
    };
    let left_text = format!("{} items · {}", shown_count, speed_text);
    let right_text = format!("Cosmo Downloader {} · © 2026 Mahedi", version);

    let mid_y = bar_rect.center().y;
    let font = egui::FontId::proportional(12.0);
    ui.painter().text(
        pos2(bar_rect.left() + 16.0, mid_y),
        egui::Align2::LEFT_CENTER,
        left_text,
        font.clone(),
        pal.muted,
    );
    ui.painter().text(
        pos2(bar_rect.right() - 16.0, mid_y),
        egui::Align2::RIGHT_CENTER,
        right_text,
        font,
        pal.muted,
    );
}

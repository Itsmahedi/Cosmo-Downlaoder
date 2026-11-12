use eframe::egui::{self, Color32, Rect, Rounding, Vec2};
use crate::theme::colors;
use rustget_core::downloader::DownloadProgress;

// ============================================================================
// Clean Minimal Progress Bar (4px Height, Crisp High-Contrast Track)
// ============================================================================

pub fn render_m3_progress_bar(
    ui: &mut egui::Ui,
    progress: Option<&DownloadProgress>,
    status: &str,
    ratio: f32,
) {
    let bar_height = 4.0;
    let total_width = ui.available_width();
    let is_paused = status == "paused";

    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_width, bar_height), egui::Sense::hover());

    // High contrast track
    ui.painter().rect_filled(rect, Rounding::same(2.0), Color32::from_rgb(0x36, 0x3C, 0x47));

    let fill_color = if is_paused {
        Color32::from_rgb(0xFA, 0xB3, 0x66)
    } else {
        colors::ACCENT
    };

    if let Some(prog) = progress {
        if !prog.segments.is_empty() && prog.segments.len() > 1 && status != "completed" {
            let num_segs = prog.segments.len() as f32;
            let gap = 1.5;
            let seg_w = ((total_width - (num_segs - 1.0) * gap) / num_segs).max(2.0);

            for (i, seg) in prog.segments.iter().enumerate() {
                let seg_x = rect.left() + (i as f32) * (seg_w + gap);
                let seg_pct = (seg.progress_percent() / 100.0) as f32;
                let active_w = (seg_w * seg_pct).clamp(0.0, seg_w);

                if active_w > 0.0 {
                    let seg_rect = Rect::from_min_size(
                        eframe::egui::pos2(seg_x, rect.top()),
                        Vec2::new(active_w, bar_height),
                    );
                    ui.painter().rect_filled(seg_rect, Rounding::same(2.0), fill_color);
                }
            }
            return;
        }
    }

    let fill_w = (total_width * ratio.clamp(0.0, 1.0)).max(if ratio > 0.0 { 2.0 } else { 0.0 });
    let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, bar_height));
    ui.painter().rect_filled(fill_rect, Rounding::same(2.0), fill_color);
}

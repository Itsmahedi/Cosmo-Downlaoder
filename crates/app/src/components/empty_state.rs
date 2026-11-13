use eframe::egui::{self, Vec2};
use crate::theme::{shapes, ThemeColors};
use crate::icons;

// ============================================================================
// Clean Centered Empty State View
// ============================================================================

pub fn render_empty_state(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    title: &str,
    subtitle: &str,
    action_label: &str,
) -> bool {
    let mut clicked = false;

    let avail_height = ui.available_height();
    let content_height = 160.0;
    if avail_height > content_height {
        ui.add_space((avail_height - content_height) * 0.4);
    }

    ui.vertical_centered(|ui| {
        // 36px icon tile / icon
        let (circle_rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
        ui.painter().circle_filled(circle_rect.center(), 18.0, pal.raise);
        icons::draw_download(ui.painter(), circle_rect.shrink(9.0), pal.muted);

        ui.add_space(shapes::S3);

        ui.label(
            egui::RichText::new(title)
                .size(16.0)
                .color(pal.text)
                .strong(),
        );

        if !subtitle.is_empty() {
            ui.add_space(shapes::S1);
            ui.label(
                egui::RichText::new(subtitle)
                    .size(12.0)
                    .color(pal.muted),
            );
        }

        ui.add_space(shapes::S4);

        let btn = egui::Button::new(
            egui::RichText::new(action_label)
                .size(13.5)
                .color(pal.on_accent),
        )
        .fill(pal.accent)
        .min_size(Vec2::new(140.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(btn).clicked() {
            clicked = true;
        }
    });

    clicked
}

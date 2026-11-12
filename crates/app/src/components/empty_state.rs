use eframe::egui::{self, Vec2};
use crate::theme::{colors, shapes};
use crate::icons;

// ============================================================================
// Clean Centered Empty State View
// ============================================================================

pub fn render_empty_state(
    ui: &mut egui::Ui,
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
        // Subtle wireframe circle + icon
        let (circle_rect, _) = ui.allocate_exact_size(Vec2::splat(48.0), egui::Sense::hover());
        ui.painter().circle_filled(circle_rect.center(), 24.0, colors::RAISE);
        icons::draw_download(ui.painter(), circle_rect.shrink(14.0), colors::MUTED);

        ui.add_space(shapes::S3);

        ui.label(
            egui::RichText::new(title)
                .size(15.0)
                .color(colors::TEXT)
                .strong(),
        );

        if !subtitle.is_empty() {
            ui.add_space(shapes::S1);
            ui.label(
                egui::RichText::new(subtitle)
                    .size(12.0)
                    .color(colors::MUTED),
            );
        }

        ui.add_space(shapes::S4);

        let btn = egui::Button::new(
            egui::RichText::new(action_label)
                .size(13.0)
                .color(colors::ON_ACCENT),
        )
        .fill(colors::ACCENT)
        .min_size(Vec2::new(140.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(btn).clicked() {
            clicked = true;
        }
    });

    clicked
}

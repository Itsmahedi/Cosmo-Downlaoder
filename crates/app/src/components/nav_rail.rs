use eframe::egui::{self, pos2, Color32, Painter, Rect, Vec2};
use crate::theme::{colors, shapes};
use crate::icons;

// ============================================================================
// Clean Minimalist Navigation Sidebar Components
// ============================================================================

pub fn render_brand(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
        icons::draw_logo(ui.painter(), icon_rect, colors::ACCENT);
        ui.add_space(shapes::S2);
        ui.label(
            egui::RichText::new("Cosmo")
                .size(16.0)
                .color(colors::TEXT)
                .strong(),
        );
    });
}

pub fn render_new_download_button(ui: &mut egui::Ui) -> bool {
    let btn_rect = ui.allocate_space(Vec2::new(ui.available_width(), shapes::CONTROL_HEIGHT)).1;
    let response = ui.interact(btn_rect, ui.id().with("new_download_btn"), egui::Sense::click());

    let bg_color = if response.hovered() {
        Color32::from_rgb(
            colors::ACCENT.r().saturating_add(15),
            colors::ACCENT.g().saturating_add(15),
            colors::ACCENT.b().saturating_add(15),
        )
    } else {
        colors::ACCENT
    };

    ui.painter().rect_filled(btn_rect, shapes::R_SM, bg_color);

    // Draw Plus Icon & Text
    let mid_y = btn_rect.center().y;
    let icon_rect = Rect::from_center_size(pos2(btn_rect.left() + 18.0, mid_y), Vec2::splat(14.0));
    icons::draw_add(ui.painter(), icon_rect, colors::ON_ACCENT);

    let text_pos = pos2(btn_rect.left() + 34.0, mid_y - 7.5);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_TOP,
        "New download",
        egui::FontId::proportional(14.0),
        colors::ON_ACCENT,
    );

    response.clicked()
}

pub fn render_nav_item(
    ui: &mut egui::Ui,
    is_selected: bool,
    name: &str,
    count: usize,
    draw_icon: impl FnOnce(&Painter, Rect, Color32),
) -> bool {
    let row_height = shapes::CONTROL_HEIGHT;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_height), egui::Sense::click());

    let (bg_color, text_color, icon_color, count_color) = if is_selected {
        (
            Color32::from_rgb(0x28, 0x36, 0x4D), // Solid subtle dark blue container
            Color32::from_rgb(0xA6, 0xC6, 0xFF), // Bright accent text
            Color32::from_rgb(0xA6, 0xC6, 0xFF),
            Color32::from_rgb(0xA6, 0xC6, 0xFF),
        )
    } else if response.hovered() {
        (
            colors::RAISE,
            Color32::WHITE,
            Color32::WHITE,
            Color32::from_rgb(0xD8, 0xDE, 0xE8),
        )
    } else {
        (
            Color32::TRANSPARENT,
            Color32::from_rgb(0xEB, 0xEF, 0xF5), // Bright, clearly legible white text
            Color32::from_rgb(0xB8, 0xC0, 0xCC), // Clear icon
            Color32::from_rgb(0xB8, 0xC0, 0xCC), // Clear tabular count
        )
    };

    if bg_color != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, shapes::R_SM, bg_color);
    }

    let mid_y = rect.center().y;
    let icon_rect = Rect::from_center_size(pos2(rect.left() + 14.0, mid_y), Vec2::splat(15.0));
    draw_icon(ui.painter(), icon_rect, icon_color);

    let text_pos = pos2(rect.left() + 32.0, mid_y - 7.5);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(13.5),
        text_color,
    );

    // Right-aligned clean count
    let count_str = count.to_string();
    let count_pos = pos2(rect.right() - 10.0, mid_y - 7.5);
    ui.painter().text(
        count_pos,
        egui::Align2::RIGHT_TOP,
        count_str,
        egui::FontId::monospace(12.5),
        count_color,
    );

    response.clicked()
}

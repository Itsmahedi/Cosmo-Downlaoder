use eframe::egui::{self, Color32, Vec2};
use crate::theme::{colors, shapes};
use rustget_core::category::CategoryFilter;

// ============================================================================
// Clean Category Filter Chips (Single-Row, Scrollable, Crisp Contrast)
// ============================================================================

pub fn render_filter_chips(
    ui: &mut egui::Ui,
    selected_category: &mut CategoryFilter,
) {
    egui::ScrollArea::horizontal()
        .id_salt("cat_chips_scroll")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = shapes::S1;

                category_chip(ui, selected_category, CategoryFilter::All, "All");
                category_chip(ui, selected_category, CategoryFilter::Video, "Video");
                category_chip(ui, selected_category, CategoryFilter::Music, "Music");
                category_chip(ui, selected_category, CategoryFilter::Documents, "Documents");
                category_chip(ui, selected_category, CategoryFilter::Compressed, "Archives");
                category_chip(ui, selected_category, CategoryFilter::Programs, "Programs");
                category_chip(ui, selected_category, CategoryFilter::Other, "Other");
            });
        });
}

fn category_chip(
    ui: &mut egui::Ui,
    current: &mut CategoryFilter,
    target: CategoryFilter,
    label: &str,
) {
    let is_selected = *current == target;
    let font = egui::FontId::proportional(13.0);

    let text_w = ui.painter().layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE).size().x;
    let chip_width = text_w + 22.0;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(chip_width, shapes::CHIP_HEIGHT), egui::Sense::click());
    if response.clicked() {
        *current = target;
    }

    let (bg, text_color) = if is_selected {
        (Color32::from_rgb(0x35, 0x3C, 0x48), Color32::WHITE)
    } else if response.hovered() {
        (colors::RAISE, Color32::WHITE)
    } else {
        (Color32::TRANSPARENT, Color32::from_rgb(0xC4, 0xCB, 0xD6))
    };

    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, shapes::R_SM, bg);
    }

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font,
        text_color,
    );
}

use eframe::egui::{self, pos2, Color32, Painter, Rect, Vec2};
use rustget_core::category::CategoryFilter;
use crate::icons;
use crate::theme::{shapes, ThemeColors};

pub struct CategoryCounts {
    pub all: usize,
    pub programs: usize,
    pub video: usize,
    pub music: usize,
    pub documents: usize,
    pub archives: usize,
}

pub fn render_category_sidebar(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    selected: &mut CategoryFilter,
    counts: &CategoryCounts,
) {
    ui.painter().text(
        pos2(ui.min_rect().left() + 8.0, ui.cursor().top() + 8.0),
        egui::Align2::LEFT_TOP,
        "Categories",
        egui::FontId::proportional(12.0),
        pal.muted,
    );
    ui.add_space(28.0);

    let items: [(CategoryFilter, &str, usize, fn(&Painter, Rect, Color32)); 6] = [
        (CategoryFilter::All, "All downloads", counts.all, icons::draw_all),
        (CategoryFilter::Programs, "Programs", counts.programs, icons::draw_programs),
        (CategoryFilter::Video, "Video", counts.video, icons::draw_video),
        (CategoryFilter::Music, "Music", counts.music, icons::draw_music),
        (CategoryFilter::Documents, "Documents", counts.documents, icons::draw_documents),
        (CategoryFilter::Compressed, "Archives", counts.archives, icons::draw_compressed),
    ];

    for (cat, label, count, icon_fn) in items {
        if render_nav_item(ui, pal, *selected == cat, label, count, icon_fn) {
            *selected = cat;
        }
        ui.add_space(2.0);
    }
}

pub fn render_nav_item(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    is_selected: bool,
    name: &str,
    count: usize,
    draw_icon: impl FnOnce(&Painter, Rect, Color32),
) -> bool {
    let row_height = shapes::CONTROL_HEIGHT; // 34px
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_height), egui::Sense::click());

    let (bg_color, text_color, icon_color, count_color) = if is_selected {
        (pal.accent_soft, pal.accent, pal.accent, pal.accent)
    } else if response.hovered() {
        (pal.raise, pal.text, pal.text, pal.text)
    } else {
        (Color32::TRANSPARENT, pal.muted, pal.muted, pal.muted)
    };

    if bg_color != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, shapes::R_SM, bg_color);
    }

    let mid_y = rect.center().y;
    let icon_rect = Rect::from_center_size(pos2(rect.left() + 8.0 + 9.0, mid_y), Vec2::splat(17.0));
    draw_icon(ui.painter(), icon_rect, icon_color);

    ui.painter().text(
        pos2(rect.left() + 8.0 + 18.0 + 12.0, mid_y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(13.5),
        text_color,
    );

    ui.painter().text(
        pos2(rect.right() - 8.0, mid_y),
        egui::Align2::RIGHT_CENTER,
        count.to_string(),
        egui::FontId::proportional(12.0),
        count_color,
    );

    response.clicked()
}

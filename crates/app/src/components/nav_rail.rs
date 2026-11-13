use eframe::egui::{self, pos2, Color32, Painter, Rect, Vec2};
use rustget_core::category::CategoryFilter;
use crate::icons;
use crate::theme::{shapes, ThemeColors};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    All,
    Downloading,
    Completed,
    Paused,
}

pub struct StatusCounts {
    pub all: usize,
    pub downloading: usize,
    pub completed: usize,
    pub paused: usize,
}

pub struct CategoryCounts {
    pub all: usize,
    pub video: usize,
    pub music: usize,
    pub documents: usize,
    pub compressed: usize,
    pub programs: usize,
    pub other: usize,
}

pub fn render_category_sidebar(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    selected_status: &mut StatusFilter,
    status_counts: &StatusCounts,
    selected_category: &mut CategoryFilter,
    cat_counts: &CategoryCounts,
) {
    // 1. Status Section
    ui.painter().text(
        pos2(ui.min_rect().left() + 8.0, ui.cursor().top() + 4.0),
        egui::Align2::LEFT_TOP,
        "Status",
        egui::FontId::proportional(12.0),
        pal.muted,
    );
    ui.add_space(24.0);

    type NavStatusItem = (StatusFilter, &'static str, usize, fn(&Painter, Rect, Color32));
    let status_items: [NavStatusItem; 4] = [
        (StatusFilter::All, "All Downloads", status_counts.all, icons::draw_all),
        (StatusFilter::Downloading, "Downloading", status_counts.downloading, icons::draw_download),
        (StatusFilter::Completed, "Completed", status_counts.completed, icons::draw_check),
        (StatusFilter::Paused, "Paused", status_counts.paused, icons::draw_pause),
    ];

    for (st, label, count, icon_fn) in status_items {
        if render_nav_item(ui, pal, *selected_status == st, label, count, icon_fn) {
            *selected_status = st;
        }
        ui.add_space(2.0);
    }

    ui.add_space(16.0);

    // 2. Categories Section
    ui.painter().text(
        pos2(ui.min_rect().left() + 8.0, ui.cursor().top() + 4.0),
        egui::Align2::LEFT_TOP,
        "Categories",
        egui::FontId::proportional(12.0),
        pal.muted,
    );
    ui.add_space(24.0);

    type NavCategoryItem = (CategoryFilter, &'static str, usize, fn(&Painter, Rect, Color32));
    let cat_items: [NavCategoryItem; 7] = [
        (CategoryFilter::All, "All Categories", cat_counts.all, icons::draw_all),
        (CategoryFilter::Video, "Video", cat_counts.video, icons::draw_video),
        (CategoryFilter::Music, "Music", cat_counts.music, icons::draw_music),
        (CategoryFilter::Documents, "Documents", cat_counts.documents, icons::draw_documents),
        (CategoryFilter::Compressed, "Compressed", cat_counts.compressed, icons::draw_compressed),
        (CategoryFilter::Programs, "Programs", cat_counts.programs, icons::draw_programs),
        (CategoryFilter::Other, "Other", cat_counts.other, icons::draw_other),
    ];

    for (cat, label, count, icon_fn) in cat_items {
        if render_nav_item(ui, pal, *selected_category == cat, label, count, icon_fn) {
            *selected_category = cat;
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
    let row_height = 32.0; // 32px per reference
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
    let icon_rect = Rect::from_center_size(pos2(rect.left() + 8.0 + 9.0, mid_y), Vec2::splat(16.0));
    draw_icon(ui.painter(), icon_rect, icon_color);

    ui.painter().text(
        pos2(rect.left() + 8.0 + 18.0 + 10.0, mid_y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(13.0),
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

use eframe::egui::{self, pos2, Color32, Pos2, Rect, Stroke, Vec2};
use rustget_core::category::CategoryFilter;
use crate::icons;
use crate::theme::{shapes, ThemeColors};

#[derive(Default)]
pub struct MenuBarAction {
    pub add_download: bool,
    pub exit_app: bool,
    pub resume_selected: bool,
    pub pause_selected: bool,
    pub delete_selected: bool,
    pub resume_all: bool,
    pub pause_all: bool,
    pub select_all: bool,
    pub clear_selection: bool,
    pub set_category: Option<CategoryFilter>,
    pub toggle_theme: bool,
    pub open_options: bool,
    pub toggle_clipboard: bool,
    pub open_downloads_dir: bool,
    pub open_about: bool,
}

pub fn render_menu_bar(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    has_selection: bool,
    clipboard_enabled: bool,
    selected_category: CategoryFilter,
) -> MenuBarAction {
    let mut action = MenuBarAction::default();

    let bar_rect = ui.allocate_space(Vec2::new(ui.available_width(), 36.0)).1;
    ui.painter().rect_filled(bar_rect, 0.0, pal.surface);
    ui.painter().rect_filled(
        Rect::from_min_max(Pos2::new(bar_rect.left(), bar_rect.bottom() - 1.0), bar_rect.max),
        0.0,
        pal.line,
    );

    let mut inner = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(bar_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    inner.set_clip_rect(bar_rect);

    // Flatten menu button styling to match .menubar .m (transparent, hover --raise)
    {
        let vis = inner.visuals_mut();
        vis.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        vis.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        vis.widgets.inactive.bg_stroke = Stroke::NONE;
        vis.widgets.hovered.bg_fill = pal.raise;
        vis.widgets.hovered.weak_bg_fill = pal.raise;
        vis.widgets.hovered.bg_stroke = Stroke::NONE;
        vis.widgets.open.bg_fill = pal.raise;
        vis.widgets.open.weak_bg_fill = pal.raise;
        vis.widgets.open.bg_stroke = Stroke::NONE;
        vis.button_frame = false;
    }
    inner.spacing_mut().button_padding = Vec2::new(12.0, 5.0);
    inner.spacing_mut().item_spacing.x = 2.0;

    inner.add_space(8.0);

    inner.menu_button(egui::RichText::new("File").size(13.5).color(pal.text), |ui| {
        ui.set_min_width(170.0);
        if ui.button("New download...").clicked() {
            action.add_download = true;
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Exit").clicked() {
            action.exit_app = true;
            ui.close_menu();
        }
    });

    inner.menu_button(egui::RichText::new("Downloads").size(13.5).color(pal.text), |ui| {
        ui.set_min_width(180.0);
        if ui.add_enabled(has_selection, egui::Button::new("Resume selected")).clicked() {
            action.resume_selected = true;
            ui.close_menu();
        }
        if ui.add_enabled(has_selection, egui::Button::new("Pause selected")).clicked() {
            action.pause_selected = true;
            ui.close_menu();
        }
        if ui.add_enabled(has_selection, egui::Button::new("Delete selected")).clicked() {
            action.delete_selected = true;
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Resume all").clicked() {
            action.resume_all = true;
            ui.close_menu();
        }
        if ui.button("Pause all").clicked() {
            action.pause_all = true;
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Select all").clicked() {
            action.select_all = true;
            ui.close_menu();
        }
        if ui.add_enabled(has_selection, egui::Button::new("Clear selection")).clicked() {
            action.clear_selection = true;
            ui.close_menu();
        }
    });

    inner.menu_button(egui::RichText::new("View").size(13.5).color(pal.text), |ui| {
        ui.set_min_width(175.0);
        for (cat, label) in [
            (CategoryFilter::All, "All downloads"),
            (CategoryFilter::Programs, "Programs"),
            (CategoryFilter::Video, "Video"),
            (CategoryFilter::Music, "Music"),
            (CategoryFilter::Documents, "Documents"),
            (CategoryFilter::Compressed, "Archives"),
        ] {
            if ui.selectable_label(selected_category == cat, label).clicked() {
                action.set_category = Some(cat);
                ui.close_menu();
            }
        }
        ui.separator();
        let theme_lbl = if pal.is_dark { "Switch to Light mode" } else { "Switch to Dark mode" };
        if ui.button(theme_lbl).clicked() {
            action.toggle_theme = true;
            ui.close_menu();
        }
    });

    inner.menu_button(egui::RichText::new("Tools").size(13.5).color(pal.text), |ui| {
        ui.set_min_width(190.0);
        if ui.button("Options...").clicked() {
            action.open_options = true;
            ui.close_menu();
        }
        let mut clip = clipboard_enabled;
        if ui.checkbox(&mut clip, "Monitor clipboard URLs").changed() {
            action.toggle_clipboard = true;
            ui.close_menu();
        }
    });

    inner.menu_button(egui::RichText::new("Help").size(13.5).color(pal.text), |ui| {
        ui.set_min_width(190.0);
        if ui.button("Open Downloads folder").clicked() {
            action.open_downloads_dir = true;
            ui.close_menu();
        }
        if ui.button("About Cosmo Downloader").clicked() {
            action.open_about = true;
            ui.close_menu();
        }
    });

    // Right-aligned Dark/Light mode toggle button
    let label = if pal.is_dark { "Light mode" } else { "Dark mode" };
    let font = egui::FontId::proportional(13.5);
    let text_w = inner.painter().layout_no_wrap(label.to_string(), font.clone(), pal.text).size().x;
    let btn_w = text_w + 18.0 + 8.0 + 24.0;
    let btn_rect = Rect::from_min_size(
        pos2(bar_rect.right() - 8.0 - btn_w, bar_rect.center().y - 15.0),
        Vec2::new(btn_w, 30.0),
    );
    let resp = inner.interact(btn_rect, inner.id().with("theme_toggle_btn"), egui::Sense::click());
    if resp.hovered() {
        inner.painter().rect_filled(btn_rect, shapes::R_SM, pal.raise);
    }
    let icon_rect = Rect::from_center_size(pos2(btn_rect.left() + 12.0 + 9.0, btn_rect.center().y), Vec2::splat(17.0));
    if pal.is_dark {
        icons::draw_sun(inner.painter(), icon_rect, pal.text);
    } else {
        icons::draw_moon(inner.painter(), icon_rect, pal.text);
    }
    inner.painter().text(
        pos2(btn_rect.left() + 12.0 + 18.0 + 8.0, btn_rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        pal.text,
    );
    if resp.clicked() {
        action.toggle_theme = true;
    }

    action
}

use eframe::egui::{self, pos2, Color32, Painter, Pos2, Rect, Vec2};
use crate::icons;
use crate::theme::{shapes, ThemeColors};

pub struct ToolbarAction {
    pub add_clicked: bool,
    pub resume_clicked: bool,
    pub pause_clicked: bool,
    pub delete_clicked: bool,
    pub options_clicked: bool,
}

pub fn render_toolbar(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    has_selection: bool,
    search_query: &mut String,
) -> ToolbarAction {
    let mut action = ToolbarAction {
        add_clicked: false,
        resume_clicked: false,
        pause_clicked: false,
        delete_clicked: false,
        options_clicked: false,
    };

    // Toolbar height matches padding (8+8) + content (~44) = 60px
    let bar_rect = ui.allocate_space(Vec2::new(ui.available_width(), 60.0)).1;
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
    inner.spacing_mut().item_spacing.x = 4.0;

    inner.add_space(12.0);

    if toolbar_btn(&mut inner, pal, "tb_add", "Add", true, |p, r, c| icons::draw_add(p, r, c)) {
        action.add_clicked = true;
    }

    toolbar_sep(&mut inner, pal);

    if toolbar_btn(&mut inner, pal, "tb_resume", "Resume", has_selection, |p, r, c| icons::draw_play(p, r, c)) {
        action.resume_clicked = true;
    }
    if toolbar_btn(&mut inner, pal, "tb_pause", "Pause", has_selection, |p, r, c| icons::draw_pause(p, r, c)) {
        action.pause_clicked = true;
    }
    if toolbar_btn(&mut inner, pal, "tb_del", "Delete", has_selection, |p, r, c| icons::draw_trash(p, r, c)) {
        action.delete_clicked = true;
    }

    toolbar_sep(&mut inner, pal);

    if toolbar_btn(&mut inner, pal, "tb_opts", "Options", true, |p, r, c| icons::draw_settings(p, r, c, pal.surface)) {
        action.options_clicked = true;
    }

    // Right-aligned search box when width permits
    if inner.available_width() > 140.0 {
        inner.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(16.0);
            if !search_query.is_empty() {
                let clr = egui::Button::new(egui::RichText::new("×").size(13.0).color(pal.muted))
                    .fill(Color32::TRANSPARENT)
                    .min_size(Vec2::new(22.0, shapes::CONTROL_HEIGHT))
                    .rounding(shapes::R_SM);
                if ui.add(clr).on_hover_text("Clear search").clicked() {
                    search_query.clear();
                }
            }
            let search_w = ui.available_width().clamp(120.0, 220.0);
            ui.add(
                egui::TextEdit::singleline(search_query)
                    .hint_text("Search downloads...")
                    .desired_width(search_w)
                    .min_size(Vec2::new(search_w, 30.0))
                    .margin(egui::Margin::symmetric(10.0, 5.0)),
            );
        });
    }

    action
}

fn toolbar_sep(ui: &mut egui::Ui, pal: &ThemeColors) {
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 32.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, pal.line);
    ui.add_space(4.0);
}

fn toolbar_btn(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    id_salt: &str,
    label: &str,
    enabled: bool,
    draw_icon: impl FnOnce(&Painter, Rect, Color32),
) -> bool {
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(66.0, 44.0), sense);

    if enabled && resp.hovered() {
        ui.painter().rect_filled(rect, shapes::R_SM, pal.raise);
    }

    let icon_color = if enabled {
        pal.accent
    } else {
        Color32::from_rgba_unmultiplied(pal.accent.r(), pal.accent.g(), pal.accent.b(), 102)
    };
    let text_color = if enabled {
        pal.text
    } else {
        Color32::from_rgba_unmultiplied(pal.text.r(), pal.text.g(), pal.text.b(), 102)
    };

    let _ = id_salt;
    let icon_rect = Rect::from_center_size(pos2(rect.center().x, rect.top() + 15.0), Vec2::splat(18.0));
    draw_icon(ui.painter(), icon_rect, icon_color);

    ui.painter().text(
        pos2(rect.center().x, rect.bottom() - 10.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(12.0),
        text_color,
    );

    enabled && resp.clicked()
}

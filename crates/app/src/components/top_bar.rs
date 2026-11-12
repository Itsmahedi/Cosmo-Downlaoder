use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use crate::theme::{colors, shapes};
use crate::icons;
use crate::components::filter_chips::render_filter_chips;
use rustget_core::category::CategoryFilter;
use rustget_core::speed::SpeedTracker;

// ============================================================================
// Clean Quiet Top Control Bar (Category Chips + Action Buttons)
// ============================================================================

pub struct TopBarAction {
    pub pause_all_clicked: bool,
    pub resume_all_clicked: bool,
    pub settings_clicked: bool,
}

pub fn render_top_bar(
    ui: &mut egui::Ui,
    selected_category: &mut CategoryFilter,
    has_active_downloads: bool,
    has_paused_downloads: bool,
    total_speed: f64,
) -> TopBarAction {
    let mut action = TopBarAction {
        pause_all_clicked: false,
        resume_all_clicked: false,
        settings_clicked: false,
    };

    let bar_rect = ui.allocate_space(Vec2::new(ui.available_width(), 44.0)).1;
    // Bottom border line
    let border_rect = Rect::from_min_max(
        Pos2::new(bar_rect.left(), bar_rect.bottom() - 1.0),
        Pos2::new(bar_rect.right(), bar_rect.bottom()),
    );
    ui.painter().rect_filled(border_rect, 0.0, colors::LINE);

    let mut inner_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(bar_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    inner_ui.set_clip_rect(bar_rect);

    inner_ui.add_space(shapes::S3);

    // Left: Category Filter Chips
    inner_ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        render_filter_chips(ui, selected_category);
    });

    // Right: Speed + Toggle Button + Settings Button
    let right_w = 160.0;
    let right_rect = Rect::from_min_max(
        Pos2::new(bar_rect.right() - right_w, bar_rect.top()),
        bar_rect.max,
    );
    let mut right_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(right_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    right_ui.add_space(shapes::S3);

    // Settings icon button
    let (s_rect, s_resp) = right_ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::click());
    let s_bg = if s_resp.hovered() { colors::RAISE } else { Color32::TRANSPARENT };
    if s_bg != Color32::TRANSPARENT {
        right_ui.painter().rect_filled(s_rect, shapes::R_SM, s_bg);
    }
    let s_color = if s_resp.hovered() { Color32::WHITE } else { Color32::from_rgb(0xC4, 0xCB, 0xD6) };
    icons::draw_settings(right_ui.painter(), s_rect.shrink(6.5), s_color);
    if s_resp.on_hover_text("Settings").clicked() {
        action.settings_clicked = true;
    }

    right_ui.add_space(shapes::S1);

    // Pause / Resume All Toggle Button
    if has_active_downloads {
        let (p_rect, p_resp) = right_ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::click());
        let p_bg = if p_resp.hovered() { colors::RAISE } else { Color32::TRANSPARENT };
        if p_bg != Color32::TRANSPARENT {
            right_ui.painter().rect_filled(p_rect, shapes::R_SM, p_bg);
        }
        let p_color = if p_resp.hovered() { Color32::WHITE } else { Color32::from_rgb(0xC4, 0xCB, 0xD6) };
        icons::draw_pause(right_ui.painter(), p_rect.shrink(6.5), p_color);
        if p_resp.on_hover_text("Pause all").clicked() {
            action.pause_all_clicked = true;
        }
    } else if has_paused_downloads {
        let (p_rect, p_resp) = right_ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::click());
        let p_bg = if p_resp.hovered() { colors::RAISE } else { Color32::TRANSPARENT };
        if p_bg != Color32::TRANSPARENT {
            right_ui.painter().rect_filled(p_rect, shapes::R_SM, p_bg);
        }
        let p_color = if p_resp.hovered() { Color32::WHITE } else { Color32::from_rgb(0xC4, 0xCB, 0xD6) };
        icons::draw_play(right_ui.painter(), p_rect.shrink(6.5), p_color);
        if p_resp.on_hover_text("Resume all").clicked() {
            action.resume_all_clicked = true;
        }
    }

    // Quiet speed label if downloading
    if total_speed > 0.0 {
        right_ui.add_space(shapes::S2);
        let speed_str = SpeedTracker::format_speed(total_speed);
        right_ui.label(
            egui::RichText::new(speed_str)
                .size(12.5)
                .color(colors::ACCENT)
                .monospace(),
        );
    }

    action
}

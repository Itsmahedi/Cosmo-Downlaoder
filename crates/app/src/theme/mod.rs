pub mod colors;
pub mod shapes;

pub use colors::*;
pub use shapes::*;

use eframe::egui::{self, FontFamily, FontId, Stroke, TextStyle};

pub fn configure_m3_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_rounding = shapes::R;
    visuals.window_fill = colors::SURFACE;
    visuals.panel_fill = colors::SURFACE;
    visuals.extreme_bg_color = colors::BG;
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.popup_shadow = egui::Shadow::NONE;
    visuals.override_text_color = None;

    // Flat surfaces with 2 elevation levels (Surface + Raise)
    visuals.widgets.noninteractive.bg_fill = colors::SURFACE;
    visuals.widgets.noninteractive.rounding = shapes::R_SM;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, colors::TEXT);

    visuals.widgets.inactive.bg_fill = colors::RAISE;
    visuals.widgets.inactive.rounding = shapes::R_SM;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, colors::TEXT);

    visuals.widgets.hovered.bg_fill = colors::RAISE;
    visuals.widgets.hovered.rounding = shapes::R_SM;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, colors::TEXT);

    visuals.widgets.active.bg_fill = colors::ACCENT_SOFT;
    visuals.widgets.active.rounding = shapes::R_SM;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, colors::ACCENT);

    visuals.widgets.open.bg_fill = colors::RAISE;
    visuals.widgets.open.rounding = shapes::R_SM;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, colors::TEXT);

    visuals.selection.bg_fill = colors::ACCENT_SOFT;
    visuals.selection.stroke = Stroke::new(1.0_f32, colors::ACCENT);

    // Font family styles with legible sizing
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Name("title".into()), FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
    ].into();

    ctx.set_style(style);
    ctx.set_visuals(visuals);
}

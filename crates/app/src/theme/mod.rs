pub mod colors;
pub mod shapes;

pub use colors::*;
pub use shapes::*;

use eframe::egui::{self, FontFamily, FontId, Stroke, TextStyle};

pub fn configure_visuals(ctx: &egui::Context, is_dark: bool) {
    let pal = colors::palette(is_dark);
    let mut visuals = if is_dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.window_rounding = shapes::R;
    visuals.window_fill = pal.surface;
    visuals.window_stroke = Stroke::new(1.0_f32, pal.line);
    visuals.panel_fill = pal.surface;
    visuals.extreme_bg_color = pal.bg;
    visuals.faint_bg_color = pal.raise;
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.popup_shadow = egui::Shadow::NONE;
    visuals.override_text_color = None;

    visuals.widgets.noninteractive.bg_fill = pal.surface;
    visuals.widgets.noninteractive.weak_bg_fill = pal.surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, pal.line);
    visuals.widgets.noninteractive.rounding = shapes::R_SM;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, pal.text);

    visuals.widgets.inactive.bg_fill = pal.raise;
    visuals.widgets.inactive.weak_bg_fill = pal.raise;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, pal.line);
    visuals.widgets.inactive.rounding = shapes::R_SM;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, pal.text);

    visuals.widgets.hovered.bg_fill = pal.raise;
    visuals.widgets.hovered.weak_bg_fill = pal.raise;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, pal.accent);
    visuals.widgets.hovered.rounding = shapes::R_SM;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, pal.text);

    visuals.widgets.active.bg_fill = pal.accent_soft;
    visuals.widgets.active.weak_bg_fill = pal.accent_soft;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, pal.accent);
    visuals.widgets.active.rounding = shapes::R_SM;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, pal.accent);

    visuals.widgets.open.bg_fill = pal.raise;
    visuals.widgets.open.weak_bg_fill = pal.raise;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, pal.line);
    visuals.widgets.open.rounding = shapes::R_SM;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, pal.text);

    visuals.selection.bg_fill = pal.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0_f32, pal.accent);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Name("title".into()), FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ]
    .into();

    ctx.set_style(style);
    ctx.set_visuals(visuals);
}

pub fn configure_m3_visuals(ctx: &egui::Context) {
    configure_visuals(ctx, true);
}

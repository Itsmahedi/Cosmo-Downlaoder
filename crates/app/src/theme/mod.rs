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

/// Configures UI font definitions with required fallback fonts for non-Latin scripts (Bengali,
/// Hindi, Arabic, Japanese, Korean, Chinese, Emoji, Symbols).
/// Keeps the app font first, then in strict order:
/// "Segoe UI", "Nirmala UI", "Segoe UI Historic", "Microsoft YaHei UI", "Yu Gothic UI",
/// "Malgun Gothic", "Segoe UI Emoji", "Segoe UI Symbol", sans-serif.
pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    #[cfg(windows)]
    let font_specs: &[(&str, &[&str])] = &[
        ("Segoe UI", &["segoeui.ttf"]),
        ("Nirmala UI", &["Nirmala.ttc", "NirmalaB.ttc", "NirmalaS.ttc"]),
        ("Segoe UI Historic", &["seguihis.ttf"]),
        ("Microsoft YaHei UI", &["msyh.ttc", "msyhbd.ttc", "msyhl.ttc"]),
        ("Yu Gothic UI", &["YuGothR.ttc", "YuGothM.ttc", "YuGothB.ttc", "yugoth.ttc"]),
        ("Malgun Gothic", &["malgun.ttf", "malgunbd.ttf"]),
        ("Segoe UI Emoji", &["seguiemj.ttf"]),
        ("Segoe UI Symbol", &["seguisym.ttf"]),
    ];

    #[cfg(windows)]
    {
        let windir = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        let fonts_dir = std::path::Path::new(&windir).join("Fonts");

        for (family_name, file_candidates) in font_specs {
            for filename in *file_candidates {
                let font_path = fonts_dir.join(filename);
                if font_path.exists() {
                    if let Ok(data) = std::fs::read(&font_path) {
                        fonts.font_data.insert(
                            family_name.to_string(),
                            egui::FontData::from_owned(data),
                        );
                        if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                            prop.push(family_name.to_string());
                        }
                        if let Some(mono) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                            mono.push(family_name.to_string());
                        }
                        break;
                    }
                }
            }
        }
    }

    ctx.set_fonts(fonts);
}

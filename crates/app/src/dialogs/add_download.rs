use std::path::{Path, PathBuf};
use eframe::egui::{self, Color32, Margin, Pos2, Rect, Stroke, Vec2};
use crate::theme::{shapes, ThemeColors};
use crate::icons;
use rustget_core::{
    categorize,
    category::Category,
    speed::SpeedTracker,
};

pub struct AddDialogState<'a> {
    pub url: &'a mut String,
    pub filename: &'a mut String,
    pub category: &'a mut String,
    pub save_dir: &'a mut String,
    pub connections: &'a mut usize,
    pub speed_limit_kb: &'a mut u64,
    pub checksum: &'a mut String,
    pub remember_category: &'a mut bool,
    pub description: &'a mut String,
    pub file_size: Option<u64>,
    pub is_probing: bool,
    pub error_message: Option<&'a str>,
    pub show_notes_expanded: &'a mut bool,
    pub show_advanced_expanded: &'a mut bool,
}

pub struct AddDialogOutcome {
    pub start_now: bool,
    pub download_later: bool,
    pub cancel: bool,
    pub open_settings: bool,
    pub url_changed: bool,
    pub category_changed: bool,
}

pub fn render_add_download_dialog(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    state: &mut AddDialogState<'_>,
) -> AddDialogOutcome {
    let mut outcome = AddDialogOutcome {
        start_now: false,
        download_later: false,
        cancel: false,
        open_settings: false,
        url_changed: false,
        category_changed: false,
    };

    ui.spacing_mut().item_spacing.y = shapes::S2;

    ui.label(
        egui::RichText::new("New download")
            .size(16.0)
            .strong()
            .color(pal.text),
    );

    ui.add_space(shapes::S1);

    // 1. Link Field
    ui.label(egui::RichText::new("Link").size(12.0).color(pal.muted));
    let url_edit = egui::TextEdit::singleline(state.url)
        .hint_text("https://example.com/file.zip")
        .desired_width(ui.available_width())
        .min_size(Vec2::new(ui.available_width(), shapes::CONTROL_HEIGHT))
        .margin(Margin::symmetric(10.0, 7.0));

    if ui.add(url_edit).changed() {
        outcome.url_changed = true;
        if let Ok(parsed) = url::Url::parse(state.url.trim()) {
            let name = rustget_core::probe::extract_filename(&reqwest::header::HeaderMap::new(), &parsed);
            *state.filename = rustget_core::probe::sanitize_filename(&name);
            if rustget_core::youtube::is_streaming_site(state.url) {
                if state.filename.is_empty() || state.filename.eq_ignore_ascii_case("watch") {
                    *state.filename = "YouTube_Video.mp4".to_string();
                } else if !state.filename.contains('.') {
                    state.filename.push_str(".mp4");
                }
                *state.category = Category::Video.name().to_string();
                *state.save_dir = Category::Video.default_dir().to_string_lossy().to_string();
            } else {
                let cat = categorize(Some(state.filename), None);
                *state.category = cat.name().to_string();
                *state.save_dir = cat.default_dir().to_string_lossy().to_string();
            }
        }
    }

    if let Some(err) = state.error_message {
        ui.label(
            egui::RichText::new(err)
                .size(11.5)
                .color(pal.danger),
        );
    }

    ui.add_space(shapes::S1);

    // 2. File Card Preview
    let card_w = ui.available_width();
    let card_rect = ui.allocate_space(Vec2::new(card_w, 44.0)).1;

    ui.painter().line_segment(
        [card_rect.left_top(), card_rect.right_top()],
        Stroke::new(1.0_f32, pal.line),
    );
    ui.painter().line_segment(
        [card_rect.left_bottom(), card_rect.right_bottom()],
        Stroke::new(1.0_f32, pal.line),
    );

    let mid_y = card_rect.center().y;
    let icon_rect = Rect::from_center_size(Pos2::new(card_rect.left() + 16.0, mid_y), Vec2::splat(18.0));
    icons::draw_category_symbol(ui.painter(), icon_rect, state.category, pal.muted);

    let text_x = card_rect.left() + 38.0;
    let display_name = if state.filename.trim().is_empty() {
        "File will be resolved from link"
    } else {
        state.filename.as_str()
    };
    ui.painter().text(
        Pos2::new(text_x, mid_y - 10.0),
        egui::Align2::LEFT_TOP,
        display_name,
        egui::FontId::proportional(13.0),
        pal.text,
    );

    let meta_text = if rustget_core::youtube::is_streaming_site(state.url) {
        format!("{}, Adaptive stream", state.category)
    } else if let Some(sz) = state.file_size {
        format!("{}, {}", state.category, SpeedTracker::format_bytes(sz))
    } else if state.is_probing {
        format!("{}, Checking size...", state.category)
    } else if state.url.trim().is_empty() {
        format!("{}, Enter URL", state.category)
    } else {
        format!("{}, Unknown size", state.category)
    };
    ui.painter().text(
        Pos2::new(text_x, mid_y + 4.0),
        egui::Align2::LEFT_TOP,
        meta_text,
        egui::FontId::proportional(11.5),
        pal.muted,
    );

    ui.add_space(shapes::S1);

    // 3. Category Field
    ui.label(egui::RichText::new("Category").size(12.0).color(pal.muted));
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("cat_select_clean")
            .selected_text(&*state.category)
            .width(ui.available_width() - 40.0)
            .show_ui(ui, |ui| {
                for cat in ["Compressed", "Video", "Music", "Documents", "Programs", "General"] {
                    if ui.selectable_value(state.category, cat.to_string(), cat).clicked() {
                        if let Some(c) = Category::from_name(cat) {
                            *state.save_dir = c.default_dir().to_string_lossy().to_string();
                        }
                        outcome.category_changed = true;
                    }
                }
            });

        let plus_btn = egui::Button::new("+")
            .min_size(Vec2::new(32.0, shapes::CONTROL_HEIGHT))
            .rounding(shapes::R_SM);
        if ui.add(plus_btn).on_hover_text("Configure categories in Options").clicked() {
            outcome.open_settings = true;
        }
    });

    ui.add_space(shapes::S1);

    // 4. Save to Field
    ui.label(egui::RichText::new("Save to").size(12.0).color(pal.muted));
    ui.horizontal(|ui| {
        let input_w = (ui.available_width() - 72.0).max(100.0);
        let mut full_target = PathBuf::from(&*state.save_dir);
        full_target.push(&*state.filename);
        let mut path_display = full_target.to_string_lossy().to_string();

        let path_edit = egui::TextEdit::singleline(&mut path_display)
            .desired_width(input_w)
            .min_size(Vec2::new(input_w, shapes::CONTROL_HEIGHT))
            .margin(Margin::symmetric(10.0, 7.0));

        if ui.add(path_edit).changed() {
            let p = Path::new(&path_display);
            if let Some(parent) = p.parent() {
                *state.save_dir = parent.to_string_lossy().to_string();
            }
            if let Some(fname) = p.file_name() {
                *state.filename = fname.to_string_lossy().to_string();
            }
        }

        let browse_btn = egui::Button::new("Browse")
            .min_size(Vec2::new(64.0, shapes::CONTROL_HEIGHT))
            .rounding(shapes::R_SM);

        if ui.add(browse_btn).clicked() {
            let start_dir = if !state.save_dir.trim().is_empty() && Path::new(state.save_dir.trim()).exists() {
                PathBuf::from(state.save_dir.trim())
            } else {
                Category::General.default_dir()
            };
            if let Some(file_path) = rfd::FileDialog::new()
                .set_title("Save As")
                .set_directory(&start_dir)
                .set_file_name(&*state.filename)
                .save_file()
            {
                if let Some(parent) = file_path.parent() {
                    *state.save_dir = parent.to_string_lossy().to_string();
                }
                if let Some(fname) = file_path.file_name() {
                    *state.filename = fname.to_string_lossy().to_string();
                }
            }
        }
    });

    ui.add_space(shapes::S1);

    let remember_label = format!("Use this folder for {} next time", state.category.to_lowercase());
    ui.checkbox(
        state.remember_category,
        egui::RichText::new(remember_label).size(12.0).color(pal.muted),
    );

    ui.horizontal(|ui| {
        let adv_label = if *state.show_advanced_expanded {
            "Hide advanced"
        } else {
            "Advanced options"
        };
        if ui.button(egui::RichText::new(adv_label).size(12.0).color(pal.accent)).clicked() {
            *state.show_advanced_expanded = !*state.show_advanced_expanded;
        }
        if !*state.show_notes_expanded && state.description.trim().is_empty() {
            ui.add_space(shapes::S2);
            if ui.button(egui::RichText::new("Add a note").size(12.0).color(pal.accent)).clicked() {
                *state.show_notes_expanded = true;
            }
        }
    });

    if *state.show_advanced_expanded {
        ui.add_space(shapes::S1);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Connections:").size(12.0).color(pal.muted));
            ui.add(
                egui::DragValue::new(state.connections)
                    .range(1..=32)
                    .speed(0.2),
            );
            ui.add_space(shapes::S2);
            ui.label(egui::RichText::new("Speed limit (KB/s):").size(12.0).color(pal.muted));
            ui.add(
                egui::DragValue::new(state.speed_limit_kb)
                    .range(0..=1_000_000)
                    .speed(10),
            );
        });
        ui.label(egui::RichText::new("Expected SHA-256 (optional)").size(12.0).color(pal.muted));
        ui.add(
            egui::TextEdit::singleline(state.checksum)
                .hint_text("64-char hex checksum")
                .desired_width(ui.available_width())
                .min_size(Vec2::new(ui.available_width(), shapes::CONTROL_HEIGHT))
                .margin(Margin::symmetric(10.0, 7.0)),
        );
    }

    if *state.show_notes_expanded || !state.description.trim().is_empty() {
        ui.add_space(shapes::S1);
        ui.label(egui::RichText::new("Note").size(12.0).color(pal.muted));
        ui.add(
            egui::TextEdit::singleline(state.description)
                .desired_width(ui.available_width())
                .min_size(Vec2::new(ui.available_width(), shapes::CONTROL_HEIGHT))
                .margin(Margin::symmetric(10.0, 7.0)),
        );
    }

    ui.add_space(shapes::S3);

    ui.horizontal(|ui| {
        let start_btn = egui::Button::new(
            egui::RichText::new("Start download").size(13.0).color(pal.on_accent),
        )
        .fill(pal.accent)
        .min_size(Vec2::new(125.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(start_btn).clicked() {
            outcome.start_now = true;
        }

        ui.add_space(shapes::S2);

        let later_btn = egui::Button::new(
            egui::RichText::new("Download later").size(13.0).color(pal.text),
        )
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, pal.line))
        .min_size(Vec2::new(115.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(later_btn).clicked() {
            outcome.download_later = true;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let cancel_btn = egui::Button::new(
                egui::RichText::new("Cancel").size(13.0).color(pal.muted),
            )
            .fill(Color32::TRANSPARENT)
            .min_size(Vec2::new(60.0, shapes::CONTROL_HEIGHT))
            .rounding(shapes::R_SM);

            if ui.add(cancel_btn).clicked() {
                outcome.cancel = true;
            }
        });
    });

    outcome
}

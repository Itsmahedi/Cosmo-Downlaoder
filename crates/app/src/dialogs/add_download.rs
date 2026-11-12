use std::path::{Path, PathBuf};
use eframe::egui::{self, Color32, Margin, Pos2, Rect, Stroke, Vec2};
use crate::theme::{colors, shapes};
use crate::icons;
use rustget_core::{
    categorize,
    category::Category,
    speed::SpeedTracker,
};

pub struct AddDialogOutcome {
    pub start_now: bool,
    pub download_later: bool,
    pub cancel: bool,
    pub open_settings: bool,
}

pub fn render_add_download_dialog(
    ui: &mut egui::Ui,
    url: &mut String,
    filename: &mut String,
    category: &mut String,
    save_dir: &mut String,
    remember_category: &mut bool,
    description: &mut String,
    file_size: Option<u64>,
    show_notes_expanded: &mut bool,
) -> AddDialogOutcome {
    let mut outcome = AddDialogOutcome {
        start_now: false,
        download_later: false,
        cancel: false,
        open_settings: false,
    };

    ui.spacing_mut().item_spacing.y = shapes::S2;

    // Header: Title
    ui.label(
        egui::RichText::new("New download")
            .size(16.0)
            .strong()
            .color(colors::TEXT),
    );

    ui.add_space(shapes::S1);

    // 1. Link Field
    ui.label(egui::RichText::new("Link").size(12.0).color(colors::MUTED));
    let url_edit = egui::TextEdit::singleline(url)
        .desired_width(ui.available_width())
        .min_size(Vec2::new(ui.available_width(), shapes::CONTROL_HEIGHT))
        .margin(Margin::symmetric(10.0, 7.0));

    if ui.add(url_edit).changed() {
        if let Ok(parsed) = url::Url::parse(url.trim()) {
            let name = rustget_core::probe::extract_filename(&reqwest::header::HeaderMap::new(), &parsed);
            *filename = rustget_core::probe::sanitize_filename(&name);
            if rustget_core::youtube::is_streaming_site(url) {
                if filename.is_empty() || filename.eq_ignore_ascii_case("watch") {
                    *filename = "YouTube_Video.mp4".to_string();
                } else if !filename.contains('.') {
                    filename.push_str(".mp4");
                }
                *category = "Video".to_string();
                *save_dir = Category::Video.default_dir().to_string_lossy().to_string();
            } else {
                let cat = categorize(Some(filename), None);
                *category = cat.name().to_string();
                *save_dir = cat.default_dir().to_string_lossy().to_string();
            }
        }
    }

    ui.add_space(shapes::S1);

    // 2. File Card Preview (Subtle top and bottom border)
    let card_w = ui.available_width();
    let card_rect = ui.allocate_space(Vec2::new(card_w, 44.0)).1;

    // Top border
    ui.painter().line_segment(
        [card_rect.left_top(), card_rect.right_top()],
        Stroke::new(1.0_f32, colors::LINE),
    );
    // Bottom border
    ui.painter().line_segment(
        [card_rect.left_bottom(), card_rect.right_bottom()],
        Stroke::new(1.0_f32, colors::LINE),
    );

    let mid_y = card_rect.center().y;
    let icon_rect = Rect::from_center_size(Pos2::new(card_rect.left() + 16.0, mid_y), Vec2::splat(18.0));
    icons::draw_category_symbol(ui.painter(), icon_rect, category);

    let text_x = card_rect.left() + 38.0;
    let display_name = if filename.trim().is_empty() {
        "File will be resolved from link"
    } else {
        filename.as_str()
    };
    ui.painter().text(
        Pos2::new(text_x, mid_y - 10.0),
        egui::Align2::LEFT_TOP,
        display_name,
        egui::FontId::proportional(13.0),
        colors::TEXT,
    );

    // Size / Status
    let meta_text = if rustget_core::youtube::is_streaming_site(url) {
        format!("{}, Adaptive stream", category)
    } else if let Some(sz) = file_size {
        format!("{}, {}", category, SpeedTracker::format_bytes(sz))
    } else {
        format!("{}, Checking size...", category)
    };
    ui.painter().text(
        Pos2::new(text_x, mid_y + 4.0),
        egui::Align2::LEFT_TOP,
        meta_text,
        egui::FontId::proportional(11.5),
        colors::MUTED,
    );

    ui.add_space(shapes::S1);

    // 3. Category Field (Split ComboBox + Plus)
    ui.label(egui::RichText::new("Category").size(12.0).color(colors::MUTED));
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("cat_select_clean")
            .selected_text(&*category)
            .width(ui.available_width() - 40.0)
            .show_ui(ui, |ui| {
                for cat in ["Archives", "Video", "Music", "Documents", "Programs", "General"] {
                    if ui.selectable_value(category, cat.to_string(), cat).clicked() {
                        if let Some(c) = Category::from_name(cat) {
                            *save_dir = c.default_dir().to_string_lossy().to_string();
                        }
                    }
                }
            });

        let plus_btn = egui::Button::new("+")
            .min_size(Vec2::new(32.0, shapes::CONTROL_HEIGHT))
            .rounding(shapes::R_SM);
        if ui.add(plus_btn).on_hover_text("Configure categories in Settings").clicked() {
            outcome.open_settings = true;
        }
    });

    ui.add_space(shapes::S1);

    // 4. Save to Field (Split Input + Browse Button)
    ui.label(egui::RichText::new("Save to").size(12.0).color(colors::MUTED));
    ui.horizontal(|ui| {
        let input_w = (ui.available_width() - 72.0).max(100.0);
        let mut full_target = PathBuf::from(&*save_dir);
        full_target.push(&*filename);
        let mut path_display = full_target.to_string_lossy().to_string();

        let path_edit = egui::TextEdit::singleline(&mut path_display)
            .desired_width(input_w)
            .min_size(Vec2::new(input_w, shapes::CONTROL_HEIGHT))
            .margin(Margin::symmetric(10.0, 7.0));

        if ui.add(path_edit).changed() {
            let p = Path::new(&path_display);
            if let Some(parent) = p.parent() {
                *save_dir = parent.to_string_lossy().to_string();
            }
            if let Some(fname) = p.file_name() {
                *filename = fname.to_string_lossy().to_string();
            }
        }

        let browse_btn = egui::Button::new("Browse")
            .min_size(Vec2::new(64.0, shapes::CONTROL_HEIGHT))
            .rounding(shapes::R_SM);

        if ui.add(browse_btn).clicked() {
            let start_dir = if !save_dir.trim().is_empty() && Path::new(save_dir.trim()).exists() {
                PathBuf::from(save_dir.trim())
            } else {
                Category::General.default_dir()
            };
            if let Some(file_path) = rfd::FileDialog::new()
                .set_title("Save As")
                .set_directory(&start_dir)
                .set_file_name(&*filename)
                .save_file()
            {
                if let Some(parent) = file_path.parent() {
                    *save_dir = parent.to_string_lossy().to_string();
                }
                if let Some(fname) = file_path.file_name() {
                    *filename = fname.to_string_lossy().to_string();
                }
            }
        }
    });

    ui.add_space(shapes::S1);

    // 5. Remember Checkbox
    let remember_label = format!("Use this folder for {} next time", category.to_lowercase());
    ui.checkbox(
        remember_category,
        egui::RichText::new(remember_label).size(12.0).color(colors::MUTED),
    );

    // 6. Optional Note link
    if !*show_notes_expanded && description.trim().is_empty() {
        if ui.button(egui::RichText::new("Add a note").size(12.0).color(colors::ACCENT)).clicked() {
            *show_notes_expanded = true;
        }
    } else {
        ui.add_space(shapes::S1);
        ui.label(egui::RichText::new("Note").size(12.0).color(colors::MUTED));
        ui.add(
            egui::TextEdit::singleline(description)
                .desired_width(ui.available_width())
                .min_size(Vec2::new(ui.available_width(), shapes::CONTROL_HEIGHT))
                .margin(Margin::symmetric(10.0, 7.0)),
        );
    }

    ui.add_space(shapes::S3);

    // 7. Footer: Start Download (Primary), Download Later (Outline), Spacer, Cancel (Ghost)
    ui.horizontal(|ui| {
        // Start download (Primary accent button)
        let start_btn = egui::Button::new(
            egui::RichText::new("Start download").size(13.0).color(colors::ON_ACCENT),
        )
        .fill(colors::ACCENT)
        .min_size(Vec2::new(125.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(start_btn).clicked() {
            outcome.start_now = true;
        }

        ui.add_space(shapes::S2);

        // Download later (Outline button)
        let later_btn = egui::Button::new(
            egui::RichText::new("Download later").size(13.0).color(colors::TEXT),
        )
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, colors::LINE))
        .min_size(Vec2::new(115.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(later_btn).clicked() {
            outcome.download_later = true;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Cancel (Ghost button)
            let cancel_btn = egui::Button::new(
                egui::RichText::new("Cancel").size(13.0).color(colors::MUTED),
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

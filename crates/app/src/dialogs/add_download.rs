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
    _logo_tex: Option<&egui::TextureHandle>,
) -> AddDialogOutcome {
    let mut outcome = AddDialogOutcome {
        start_now: false,
        download_later: false,
        cancel: false,
        open_settings: false,
        url_changed: false,
        category_changed: false,
    };

    ui.spacing_mut().item_spacing.y = shapes::S4;

    // 1. Link Field (Title bar "New download" serves as the sole header)
    ui.label(egui::RichText::new("Link").size(12.0).color(pal.muted));
    let url_edit = egui::TextEdit::singleline(state.url)
        .hint_text("https://example.com/file.zip")
        .desired_width(ui.available_width())
        .min_size(Vec2::new(ui.available_width(), shapes::CONTROL_HEIGHT))
        .margin(Margin::symmetric(10.0, 7.0));

    if ui.add(url_edit).changed() {
        outcome.url_changed = true;
        let normalized = rustget_core::youtube::normalize_streaming_url(state.url);
        if normalized != *state.url {
            *state.url = normalized;
        }
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
    let card_rect = ui.allocate_space(Vec2::new(card_w, 52.0)).1;

    ui.painter().line_segment(
        [card_rect.left_bottom(), card_rect.right_bottom()],
        Stroke::new(1.0_f32, pal.line),
    );

    let mid_y = card_rect.center().y;
    let icon_box = Rect::from_center_size(Pos2::new(card_rect.left() + 20.0, mid_y), Vec2::splat(36.0));
    ui.painter().rect_filled(icon_box, shapes::R_SM, pal.accent_soft);
    let icon_rect = Rect::from_center_size(icon_box.center(), Vec2::splat(18.0));
    icons::draw_category_symbol(ui.painter(), icon_rect, state.category, pal.accent);

    let display_name = if state.filename.trim().is_empty() {
        "File will be resolved from link"
    } else {
        state.filename.as_str()
    };

    let meta_text = if rustget_core::youtube::is_streaming_site(state.url) {
        format!("{} · YouTube", state.category)
    } else if let Ok(parsed) = url::Url::parse(state.url.trim()) {
        let host = parsed.host_str().unwrap_or("link").trim_start_matches("www.");
        format!("{} · {}", state.category, host)
    } else if let Some(sz) = state.file_size {
        format!("{} · {}", state.category, SpeedTracker::format_bytes(sz))
    } else if state.is_probing {
        format!("{} · Checking size...", state.category)
    } else {
        format!("{} · File", state.category)
    };

    let text_rect = Rect::from_min_max(
        Pos2::new(card_rect.left() + 46.0, card_rect.top() + 8.0),
        Pos2::new(card_rect.right() - 8.0, card_rect.bottom() - 8.0),
    );

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(text_rect), |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        ui.add(
            egui::Label::new(
                egui::RichText::new(display_name)
                    .size(13.5)
                    .strong()
                    .color(pal.text),
            )
            .truncate(),
        );
        ui.add(
            egui::Label::new(
                egui::RichText::new(meta_text)
                    .size(12.0)
                    .color(pal.muted),
            )
            .truncate(),
        );
    });

    ui.add_space(shapes::S2);

    // 3. Category & Save to on the SAME ROW (matches Image 1 & Download_popup.html .sv)
    let total_w = ui.available_width();
    let cat_w = 140.0_f32;
    let col_gap = 12.0_f32;
    let save_w = (total_w - cat_w - col_gap).max(60.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = col_gap;

        // Column 1: Category (fixed width ~140px, 34px height)
        ui.allocate_ui_with_layout(
            Vec2::new(cat_w, shapes::CONTROL_HEIGHT + 22.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.label(egui::RichText::new("Category").size(12.0).color(pal.muted));
                ui.scope(|ui| {
                    ui.spacing_mut().interact_size.y = shapes::CONTROL_HEIGHT;
                    ui.spacing_mut().button_padding = egui::vec2(10.0, 7.0);
                    egui::ComboBox::from_id_salt("cat_select_clean")
                        .selected_text(&*state.category)
                        .width(cat_w - 6.0)
                        .show_ui(ui, |ui| {
                            for cat in ["Video", "Music", "Documents", "Compressed", "Programs", "General"] {
                                let is_sel = state.category.as_str() == cat;
                                if ui.selectable_label(is_sel, cat).clicked() {
                                    *state.category = cat.to_string();
                                    if let Some(c) = Category::from_name(cat) {
                                        *state.save_dir = c.default_dir().to_string_lossy().to_string();
                                    }
                                    outcome.category_changed = true;
                                }
                            }
                        });
                });
            },
        );

        // Column 2: Save to (takes the rest, folder button flex: none 34px fixed)
        ui.allocate_ui_with_layout(
            Vec2::new(save_w, shapes::CONTROL_HEIGHT + 22.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.label(egui::RichText::new("Save to").size(12.0).color(pal.muted));
                ui.horizontal(|ui| {
                    let browse_w = shapes::CONTROL_HEIGHT; // fixed 34px size
                    let inner_gap = 8.0_f32;
                    ui.spacing_mut().item_spacing.x = inner_gap;
                    let input_w = (save_w - browse_w - inner_gap).max(40.0);

                    let mut full_target = PathBuf::from(&*state.save_dir);
                    full_target.push(&*state.filename);
                    let mut path_display = full_target.to_string_lossy().to_string();
                    let path_tooltip = path_display.clone();

                    let path_edit = egui::TextEdit::singleline(&mut path_display)
                        .desired_width(input_w)
                        .min_size(Vec2::new(input_w, shapes::CONTROL_HEIGHT))
                        .margin(Margin::symmetric(10.0, 7.0));

                    let path_resp = ui.add(path_edit).on_hover_text(path_tooltip);

                    if path_resp.changed() {
                        let p = Path::new(&path_display);
                        if let Some(parent) = p.parent() {
                            *state.save_dir = parent.to_string_lossy().to_string();
                        }
                        if let Some(fname) = p.file_name() {
                            *state.filename = fname.to_string_lossy().to_string();
                        }
                    }

                    // Folder icon button (draw_folder) - fixed 34px size
                    let (btn_rect, btn_resp) = ui.allocate_exact_size(
                        Vec2::splat(browse_w),
                        egui::Sense::click(),
                    );
                    let btn_bg = if btn_resp.hovered() { pal.raise } else { Color32::TRANSPARENT };
                    ui.painter().rect(btn_rect, shapes::R_SM, btn_bg, Stroke::new(1.0_f32, pal.line));
                    let folder_rect = Rect::from_center_size(btn_rect.center(), Vec2::splat(16.0));
                    icons::draw_folder(ui.painter(), folder_rect, if btn_resp.hovered() { pal.text } else { pal.muted });

                    if btn_resp.on_hover_text("Change folder").clicked() {
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
            },
        );
    });

    ui.add_space(shapes::S1);

    // Checkbox (custom square checkbox matching Image 1 & Download_popup.html)
    let remember_label = format!("Use this folder for {} next time", state.category.to_lowercase());
    ui.horizontal(|ui| {
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::click());
        if resp.clicked() {
            *state.remember_category = !*state.remember_category;
        }
        let bg = if *state.remember_category { pal.accent } else { pal.bg };
        let border = if *state.remember_category { pal.accent } else { pal.line };
        ui.painter().rect(rect, eframe::egui::Rounding::same(3.0), bg, Stroke::new(1.0_f32, border));
        if *state.remember_category {
            let p1 = Pos2::new(rect.left() + 3.5, rect.center().y);
            let p2 = Pos2::new(rect.left() + 6.0, rect.bottom() - 4.0);
            let p3 = Pos2::new(rect.right() - 3.5, rect.top() + 4.5);
            ui.painter().line_segment([p1, p2], Stroke::new(1.8_f32, pal.on_accent));
            ui.painter().line_segment([p2, p3], Stroke::new(1.8_f32, pal.on_accent));
        }
        let text_resp = ui.add(
            egui::Label::new(egui::RichText::new(remember_label).size(12.5).color(pal.muted))
                .sense(egui::Sense::click()),
        );
        if text_resp.clicked() {
            *state.remember_category = !*state.remember_category;
        }
    });

    if !state.description.trim().is_empty() {
        ui.add_space(shapes::S1);
        ui.label(egui::RichText::new("Note").size(12.0).color(pal.muted));
        ui.add(
            egui::TextEdit::singleline(state.description)
                .desired_width(ui.available_width())
                .min_size(Vec2::new(ui.available_width(), shapes::CONTROL_HEIGHT))
                .margin(Margin::symmetric(10.0, 7.0)),
        );
    }

    ui.add_space(shapes::S2);

    // Border line above action buttons (matching .dlf border-top: 1px solid var(--line))
    let sep_w = ui.available_width();
    let sep_rect = ui.allocate_space(Vec2::new(sep_w, 1.0)).1;
    ui.painter().line_segment(
        [sep_rect.left_top(), sep_rect.right_top()],
        Stroke::new(1.0_f32, pal.line),
    );

    ui.add_space(shapes::S2);

    let is_empty_link = state.url.trim().is_empty();

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        // Start download (primary)
        let (btn_rect, btn_resp) = ui.allocate_exact_size(
            Vec2::new(138.0, shapes::CONTROL_HEIGHT),
            if is_empty_link { egui::Sense::hover() } else { egui::Sense::click() },
        );
        let bg = if is_empty_link {
            pal.line
        } else if btn_resp.hovered() {
            pal.accent.gamma_multiply(1.08)
        } else {
            pal.accent
        };
        let text_color = if is_empty_link {
            pal.muted
        } else {
            pal.on_accent
        };
        ui.painter().rect_filled(btn_rect, shapes::R_SM, bg);
        let icon_rect = Rect::from_center_size(Pos2::new(btn_rect.left() + 18.0, btn_rect.center().y), Vec2::splat(14.0));
        icons::draw_download(ui.painter(), icon_rect, text_color);
        ui.painter().text(
            Pos2::new(btn_rect.left() + 32.0, btn_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Start download",
            egui::FontId::proportional(13.0),
            text_color,
        );

        if !is_empty_link && btn_resp.clicked() {
            outcome.start_now = true;
        }

        ui.add_space(shapes::S2);

        // Cancel (outline)
        let cancel_btn = egui::Button::new(
            egui::RichText::new("Cancel").size(13.0).color(pal.text),
        )
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, pal.line))
        .min_size(Vec2::new(72.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(cancel_btn).clicked() {
            outcome.cancel = true;
        }

        // Keyboard shortcuts: Esc = Cancel, Enter = Start download
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            outcome.cancel = true;
        }
        if !is_empty_link && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            outcome.start_now = true;
        }
    });

    outcome
}

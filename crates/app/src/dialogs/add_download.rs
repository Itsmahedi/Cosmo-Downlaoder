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

pub fn format_path_smart_measured(ui: &egui::Ui, dir: &str, filename: &str, max_px: f32) -> String {
    if max_px <= 20.0 || (dir.is_empty() && filename.is_empty()) {
        return filename.to_string();
    }

    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let measure = |s: &str| -> f32 {
        ui.fonts(|f| f.layout_no_wrap(s.to_string(), font_id.clone(), Color32::WHITE).size().x)
    };

    let clean_dir = dir.trim_end_matches(['\\', '/']);
    let full = if clean_dir.is_empty() {
        filename.to_string()
    } else {
        format!("{}\\{}", clean_dir, filename)
    };

    // If full path fits, return it immediately without any shortening
    if measure(&full) <= max_px {
        return full;
    }

    let dir_prefix = if clean_dir.is_empty() {
        String::new()
    } else {
        format!("{}\\", clean_dir)
    };

    let (stem, ext) = if let Some(dot_idx) = filename.rfind('.') {
        if dot_idx > 0 && dot_idx + 6 >= filename.len() {
            (&filename[..dot_idx], &filename[dot_idx..])
        } else {
            (filename, "")
        }
    } else {
        (filename, "")
    };

    let stem_chars: Vec<char> = stem.chars().collect();
    let tail_len = if stem.ends_with("Video)") {
        6
    } else {
        10.min(stem_chars.len())
    };
    let tail: String = stem_chars[stem_chars.len().saturating_sub(tail_len)..].iter().collect();
    let tail_with_ext = format!("{}{}", tail, ext);

    // Priority 1: keep drive + folder ("C:\Users\steam\Videos\") fully visible
    let min_case1 = format!("{}…{}", dir_prefix, tail_with_ext);
    if measure(&min_case1) <= max_px {
        let mut best = min_case1;
        for head_len in 1..=stem_chars.len().saturating_sub(tail_len) {
            let head: String = stem_chars.iter().take(head_len).collect();
            let candidate = format!("{}{}…{}", dir_prefix, head, tail_with_ext);
            if measure(&candidate) <= max_px {
                best = candidate;
            } else {
                break;
            }
        }
        return best;
    }

    // Priority 2: If folder does not fit, fall back to "C:\...\Videos\...Video).mp4"
    let drive = if let Some(idx) = clean_dir.find(':') {
        &clean_dir[..=idx]
    } else {
        "C:"
    };
    let last_folder = std::path::Path::new(clean_dir)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Videos");
    let short_dir = format!("{}\\...\\{}\\", drive, last_folder);
    let min_case2 = format!("{}…{}", short_dir, tail_with_ext);
    if measure(&min_case2) <= max_px {
        let mut best = min_case2;
        for head_len in 1..=stem_chars.len().saturating_sub(tail_len) {
            let head: String = stem_chars.iter().take(head_len).collect();
            let candidate = format!("{}{}…{}", short_dir, head, tail_with_ext);
            if measure(&candidate) <= max_px {
                best = candidate;
            } else {
                break;
            }
        }
        return best;
    }

    // Priority 3: Fall back to "C:\...\...Video).mp4"
    let min_dir = format!("{}\\...\\", drive);
    let mut best = format!("{}…{}", min_dir, tail_with_ext);
    for head_len in 1..=stem_chars.len().saturating_sub(tail_len) {
        let head: String = stem_chars.iter().take(head_len).collect();
        let candidate = format!("{}{}…{}", min_dir, head, tail_with_ext);
        if measure(&candidate) <= max_px {
            best = candidate;
        } else {
            break;
        }
    }
    best
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

    let body_h = 255.0_f32; // Strictly 255px: 350px total - 36px title bar - 59px footer

    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), body_h),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            egui::Frame::none()
                .inner_margin(Margin { left: 24.0, right: 24.0, top: 20.0, bottom: 16.0 })
                .show(ui, |ui| {
                    render_add_download_body(ui, pal, state, &mut outcome);
                });
        },
    );

    egui::Frame::none()
        .inner_margin(Margin { left: 24.0, right: 24.0, top: 0.0, bottom: 0.0 })
        .show(ui, |ui| {
            render_add_download_footer(ui, pal, state, &mut outcome);
        });

    outcome
}

pub fn render_add_download_body(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    state: &mut AddDialogState<'_>,
    outcome: &mut AddDialogOutcome,
) {
    ui.spacing_mut().item_spacing.y = 0.0;

    // 1. Link Field: label 16 + gap 4 + field 34 = 54
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), 16.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.label(egui::RichText::new("Link").size(12.0).color(pal.muted));
        },
    );
    ui.add_space(4.0);

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

    // gap 12
    ui.add_space(12.0);

    // 2. File summary row: 48 + 1px bottom line = 49
    let card_w = ui.available_width();
    let card_rect = ui.allocate_space(Vec2::new(card_w, 48.0)).1;

    ui.painter().line_segment(
        [card_rect.left_bottom(), card_rect.right_bottom()],
        Stroke::new(1.0_f32, pal.line),
    );

    let mid_y = card_rect.center().y;
    let icon_box = Rect::from_center_size(Pos2::new(card_rect.left() + 18.0, mid_y), Vec2::splat(34.0));
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
        Pos2::new(card_rect.left() + 44.0, card_rect.top() + 4.0),
        Pos2::new(card_rect.right() - 8.0, card_rect.bottom() - 4.0),
    );

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(text_rect), |ui| {
        ui.set_clip_rect(text_rect);
        ui.set_max_width(text_rect.width());
        ui.spacing_mut().item_spacing.y = 2.0;
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

    // gap 16
    ui.add_space(16.0);

    // 3. Category + Save to 2-column layout:
    // Column 1 (Category): label "Category" + select (140px wide)
    // Column 2 (Save to): label "Save to" + path field + folder button (fills the rest)
    // 12px gap between the columns. Each label is 12px, muted, 4px gap above control, left-aligned to its own control.
    let row_w = ui.available_width();
    let cat_w = 140.0_f32;
    let gap_col = 12.0_f32;
    let folder_w = 34.0_f32;
    let gap_path_folder = 8.0_f32;
    let col2_w = (row_w - cat_w - gap_col).max(0.0);
    let path_w = (col2_w - gap_path_folder - folder_w).max(0.0);
    let col_h = 16.0 + 4.0 + shapes::CONTROL_HEIGHT;

    ui.allocate_ui_with_layout(
        Vec2::new(row_w, col_h),
        egui::Layout::left_to_right(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.spacing_mut().item_spacing.y = 0.0;

            // Column 1: Category (140px wide, vertical stack)
            ui.allocate_ui_with_layout(
                Vec2::new(cat_w, col_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(cat_w);
                    ui.spacing_mut().item_spacing.y = 0.0;

                    // Label "Category" (height 16px, left-aligned)
                    ui.allocate_ui_with_layout(
                        Vec2::new(cat_w, 16.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label(egui::RichText::new("Category").size(12.0).color(pal.muted));
                        },
                    );

                    ui.add_space(4.0); // 4px gap between label and control

                    // Select (140px wide, 34px high)
                    ui.spacing_mut().interact_size.x = cat_w;
                    ui.spacing_mut().interact_size.y = shapes::CONTROL_HEIGHT;
                    ui.spacing_mut().button_padding = egui::vec2(10.0, 7.0);
                    let display_cat = if state.category.as_str() == "General" { "Other" } else { &*state.category };
                    egui::ComboBox::from_id_salt("cat_select_clean")
                        .selected_text(display_cat)
                        .width(cat_w)
                        .show_ui(ui, |ui| {
                            for cat in ["Video", "Music", "Documents", "Compressed", "Programs", "Other"] {
                                let is_sel = state.category.as_str() == cat || (cat == "Other" && state.category.as_str() == "General");
                                if ui.selectable_label(is_sel, cat).clicked() {
                                    *state.category = cat.to_string();
                                    if let Some(c) = Category::from_name(cat) {
                                        *state.save_dir = c.default_dir().to_string_lossy().to_string();
                                    }
                                    outcome.category_changed = true;
                                }
                            }
                        });
                },
            );

            // 12px gap between Column 1 and Column 2
            ui.add_space(gap_col);

            // Column 2: Save to (fills remaining width, vertical stack)
            ui.allocate_ui_with_layout(
                Vec2::new(col2_w, col_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(col2_w);
                    ui.spacing_mut().item_spacing.y = 0.0;

                    // Label "Save to" (height 16px, left-aligned to path field's left edge)
                    ui.allocate_ui_with_layout(
                        Vec2::new(col2_w, 16.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label(egui::RichText::new("Save to").size(12.0).color(pal.muted));
                        },
                    );

                    ui.add_space(4.0); // 4px gap between label and control

                    // Controls row: Path field + Folder button
                    ui.allocate_ui_with_layout(
                        Vec2::new(col2_w, shapes::CONTROL_HEIGHT),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.spacing_mut().item_spacing.y = 0.0;

                            let mut full_target = PathBuf::from(&*state.save_dir);
                            full_target.push(&*state.filename);
                            let full_path_str = full_target.to_string_lossy().to_string();
                            let path_tooltip = full_path_str.clone();

                            let path_id = ui.make_persistent_id("dl_path_input");
                            let is_focused = ui.memory(|m| m.has_focus(path_id));

                            let inner_path_w = (path_w - 20.0).max(0.0);
                            let mut path_display = if is_focused {
                                full_path_str.clone()
                            } else {
                                format_path_smart_measured(ui, &state.save_dir, &state.filename, inner_path_w - 4.0)
                            };

                            let path_edit = egui::TextEdit::singleline(&mut path_display)
                                .id(path_id)
                                .desired_width(inner_path_w)
                                .margin(Margin::symmetric(10.0, 7.0));

                            let path_resp = ui.add(path_edit).on_hover_text(&path_tooltip);

                            if !is_focused {
                                let char_count = path_display.chars().count();
                                let mut t_state = egui::text_edit::TextEditState::load(ui.ctx(), path_id).unwrap_or_default();
                                t_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(char_count))));
                                t_state.store(ui.ctx(), path_id);
                            }

                            if is_focused && path_resp.changed() {
                                let p = Path::new(&path_display);
                                if let Some(parent) = p.parent() {
                                    *state.save_dir = parent.to_string_lossy().to_string();
                                }
                                if let Some(fname) = p.file_name() {
                                    *state.filename = fname.to_string_lossy().to_string();
                                }
                            }

                            // 8px gap between path field and folder button
                            ui.add_space(gap_path_folder);

                            // Folder button (fixed 34 wide, 34 high)
                            let (btn_rect, btn_resp) = ui.allocate_exact_size(
                                Vec2::splat(folder_w),
                                egui::Sense::click(),
                            );
                            let btn_resp = btn_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
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
                        },
                    );
                },
            );
        },
    );

    // gap 12
    ui.add_space(12.0);

    // 4. Checkbox row: 20
    let remember_label = format!("Use this folder for {} next time", state.category.to_lowercase());
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), 20.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
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
            ui.add_space(8.0);
            let text_resp = ui.add(
                egui::Label::new(egui::RichText::new(remember_label).size(12.5).color(pal.muted))
                    .sense(egui::Sense::click()),
            );
            if text_resp.clicked() {
                *state.remember_category = !*state.remember_category;
            }
            resp.union(text_resp).on_hover_cursor(egui::CursorIcon::PointingHand);
        },
    );

    if let Some(err) = state.error_message {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(err)
                .size(11.5)
                .color(pal.danger),
        );
    }
}

pub fn render_add_download_footer(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    state: &AddDialogState<'_>,
    outcome: &mut AddDialogOutcome,
) {
    ui.spacing_mut().item_spacing.y = 0.0;

    // Fixed height 59 = 1px top border + 12 padding + 34 buttons + 12 padding
    // 1px top border directly above buttons across the footer width
    let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    let left_x = sep_rect.left() - 24.0;
    let right_x = sep_rect.right() + 24.0;
    ui.painter().line_segment(
        [Pos2::new(left_x, sep_rect.top()), Pos2::new(right_x, sep_rect.top())],
        Stroke::new(1.0_f32, pal.line),
    );

    ui.add_space(12.0); // 12px padding directly above the buttons

    let is_empty_link = state.url.trim().is_empty();

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.spacing_mut().item_spacing.y = 0.0;

        // Start download (primary) 34 height
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

        let mut btn_resp = btn_resp;
        if !is_empty_link {
            btn_resp = btn_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if btn_resp.clicked() {
                outcome.start_now = true;
            }
        }

        ui.add_space(shapes::S2); // 8px gap between buttons

        // Cancel (outline) exactly 72.0 wide, 34.0 height
        let (cancel_rect, cancel_resp) = ui.allocate_exact_size(
            Vec2::new(72.0, shapes::CONTROL_HEIGHT),
            egui::Sense::click(),
        );
        let cancel_resp = cancel_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        let cancel_bg = if cancel_resp.hovered() { pal.raise } else { Color32::TRANSPARENT };
        let cancel_border_rect = cancel_rect.shrink(0.5);
        ui.painter().rect(cancel_border_rect, shapes::R_SM, cancel_bg, Stroke::new(1.0_f32, pal.line));
        ui.painter().text(
            cancel_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Cancel",
            egui::FontId::proportional(13.0),
            pal.text,
        );

        if cancel_resp.clicked() {
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

    ui.add_space(12.0); // 12px padding below buttons
}

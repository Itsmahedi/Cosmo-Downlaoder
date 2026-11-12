use std::collections::HashMap;
use std::sync::Arc;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use crate::theme::{colors, shapes};
use crate::icons;
use crate::platform;
use super::progress_bar::render_m3_progress_bar;
use rustget_core::{
    db::DownloadRecord,
    downloader::DownloadProgress,
    queue::QueueManager,
    speed::SpeedTracker,
};

// ============================================================================
// Clean Quiet Download Item Row (Grid Layout, Crisp Legible Typography)
// ============================================================================

pub fn render_download_card(
    ui: &mut egui::Ui,
    record: &DownloadRecord,
    progress_map: &HashMap<String, DownloadProgress>,
    ema_speeds: &HashMap<String, f64>,
    queue: &Arc<QueueManager>,
    confirm_delete_id: &mut Option<String>,
) {
    let row_height = 54.0;
    let (row_rect, row_resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), row_height),
        egui::Sense::hover(),
    );

    let is_hovered = row_resp.hovered();
    let bg_color = if is_hovered { colors::RAISE } else { Color32::TRANSPARENT };
    if bg_color != Color32::TRANSPARENT {
        ui.painter().rect_filled(row_rect, shapes::R_SM, bg_color);
    }

    let live_prog = progress_map.get(&record.id);
    let downloaded = live_prog.map(|p| p.downloaded_bytes).unwrap_or(record.downloaded_bytes);
    let total = live_prog.and_then(|p| p.total_bytes).or(record.total_size);

    let pct = match total {
        Some(t) if t > 0 => ((downloaded as f64) / (t as f64) * 100.0).clamp(0.0, 100.0),
        _ => if record.status == "completed" { 100.0 } else { 0.0 },
    };

    let mid_y = row_rect.center().y;

    // 1. Icon (32px column, centered at x = left + 20)
    let icon_rect = Rect::from_center_size(Pos2::new(row_rect.left() + 20.0, mid_y), Vec2::splat(18.0));
    icons::draw_category_symbol(ui.painter(), icon_rect, &record.category);

    // 2. Col 2: Name + Meta size
    let actions_w = 72.0;
    let state_w = 190.0;
    let name_w = (row_rect.width() - 44.0 - state_w - actions_w).max(120.0);

    let name_rect = Rect::from_min_size(
        Pos2::new(row_rect.left() + 42.0, row_rect.top() + 8.0),
        Vec2::new(name_w, 38.0),
    );

    let mut name_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(name_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    name_ui.set_clip_rect(name_rect);

    name_ui.add(
        egui::Label::new(
            egui::RichText::new(&record.filename)
                .size(14.0)
                .strong()
                .color(colors::TEXT),
        )
        .truncate(),
    );

    let dl_str = SpeedTracker::format_bytes(downloaded);
    let tot_str = total.map(SpeedTracker::format_bytes).unwrap_or_else(|| "".to_string());
    let meta_str = if tot_str.is_empty() {
        dl_str
    } else {
        format!("{} of {}", dl_str, tot_str)
    };
    name_ui.label(
        egui::RichText::new(meta_str)
            .size(12.0)
            .color(colors::MUTED)
            .monospace(),
    );

    // 3. Col 3: State / Progress Bar
    let state_x = row_rect.right() - actions_w - state_w;
    let state_rect = Rect::from_min_size(
        Pos2::new(state_x, row_rect.top() + 10.0),
        Vec2::new(state_w - 12.0, 34.0),
    );

    let mut state_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(state_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    state_ui.set_clip_rect(state_rect);

    if record.status == "downloading" || record.status == "queued" {
        render_m3_progress_bar(&mut state_ui, live_prog, &record.status, (pct / 100.0) as f32);
        state_ui.add_space(4.0);

        let speed = live_prog.map(|p| ema_speeds.get(&record.id).copied().unwrap_or(p.speed_bps)).unwrap_or(0.0);
        let speed_str = SpeedTracker::format_speed(speed);
        let eta_str = live_prog.map(|p| SpeedTracker::format_eta(p.eta_seconds)).unwrap_or_default();
        let stat_text = if eta_str.is_empty() {
            speed_str
        } else {
            format!("{}, {} left", speed_str, eta_str)
        };
        state_ui.label(
            egui::RichText::new(stat_text)
                .size(12.0)
                .color(colors::ACCENT)
                .monospace(),
        );
    } else if record.status == "paused" {
        render_m3_progress_bar(&mut state_ui, live_prog, &record.status, (pct / 100.0) as f32);
        state_ui.add_space(4.0);
        state_ui.label(
            egui::RichText::new("Paused")
                .size(12.0)
                .strong()
                .color(Color32::from_rgb(0xFA, 0xB3, 0x66)),
        );
    } else if record.status == "completed" {
        state_ui.add_space(8.0);
        state_ui.label(
            egui::RichText::new("Completed")
                .size(12.5)
                .strong()
                .color(Color32::from_rgb(0x76, 0xD4, 0x92)),
        );
    } else {
        state_ui.add_space(8.0);
        state_ui.label(
            egui::RichText::new("Failed")
                .size(12.5)
                .strong()
                .color(colors::ERROR),
        );
    }

    // 4. Col 4: Action Buttons (Reveal on hover / focus)
    let acts_rect = Rect::from_min_size(
        Pos2::new(row_rect.right() - actions_w, row_rect.top() + 12.0),
        Vec2::new(actions_w, 30.0),
    );

    let is_confirming = confirm_delete_id.as_deref() == Some(&record.id);

    if is_hovered || is_confirming {
        let mut acts_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(acts_rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );

        if is_confirming {
            let confirm_btn = egui::Button::new(
                egui::RichText::new("Delete?").size(11.5).color(colors::ERROR),
            )
            .fill(Color32::TRANSPARENT)
            .min_size(Vec2::new(48.0, 26.0))
            .rounding(shapes::R_SM);

            if acts_ui.add(confirm_btn).clicked() {
                let q_id = record.id.clone();
                let q = Arc::clone(queue);
                tokio::spawn(async move {
                    let _ = q.remove(&q_id, false).await;
                });
                *confirm_delete_id = None;
            }
        } else {
            // Delete / Close button
            let (d_rect, d_resp) = acts_ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::click());
            let d_bg = if d_resp.hovered() { colors::RAISE } else { Color32::TRANSPARENT };
            if d_bg != Color32::TRANSPARENT {
                acts_ui.painter().rect_filled(d_rect, shapes::R_SM, d_bg);
            }
            let d_color = if d_resp.hovered() { colors::ERROR } else { Color32::from_rgb(0xB8, 0xC0, 0xCC) };
            icons::draw_close(acts_ui.painter(), d_rect.shrink(6.5), d_color);
            if d_resp.on_hover_text("Delete").clicked() {
                *confirm_delete_id = Some(record.id.clone());
            }

            acts_ui.add_space(2.0);

            // Primary action per state (Pause, Resume, or Open Folder)
            if record.status == "downloading" || record.status == "queued" {
                let (p_rect, p_resp) = acts_ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::click());
                let p_bg = if p_resp.hovered() { colors::RAISE } else { Color32::TRANSPARENT };
                if p_bg != Color32::TRANSPARENT {
                    acts_ui.painter().rect_filled(p_rect, shapes::R_SM, p_bg);
                }
                let p_color = if p_resp.hovered() { Color32::WHITE } else { Color32::from_rgb(0xB8, 0xC0, 0xCC) };
                icons::draw_pause(acts_ui.painter(), p_rect.shrink(6.0), p_color);
                if p_resp.on_hover_text("Pause").clicked() {
                    let q_id = record.id.clone();
                    let q = Arc::clone(queue);
                    tokio::spawn(async move {
                        let _ = q.pause(&q_id).await;
                    });
                }
            } else if record.status == "paused" || record.status == "failed" {
                let (r_rect, r_resp) = acts_ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::click());
                let r_bg = if r_resp.hovered() { colors::RAISE } else { Color32::TRANSPARENT };
                if r_bg != Color32::TRANSPARENT {
                    acts_ui.painter().rect_filled(r_rect, shapes::R_SM, r_bg);
                }
                let r_color = if r_resp.hovered() { Color32::WHITE } else { Color32::from_rgb(0xB8, 0xC0, 0xCC) };
                icons::draw_play(acts_ui.painter(), r_rect.shrink(6.0), r_color);
                if r_resp.on_hover_text("Resume").clicked() {
                    let q_id = record.id.clone();
                    let q = Arc::clone(queue);
                    tokio::spawn(async move {
                        let _ = q.resume(&q_id).await;
                    });
                }
            } else if record.status == "completed" {
                let (f_rect, f_resp) = acts_ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::click());
                let f_bg = if f_resp.hovered() { colors::RAISE } else { Color32::TRANSPARENT };
                if f_bg != Color32::TRANSPARENT {
                    acts_ui.painter().rect_filled(f_rect, shapes::R_SM, f_bg);
                }
                let f_color = if f_resp.hovered() { Color32::WHITE } else { Color32::from_rgb(0xB8, 0xC0, 0xCC) };
                icons::draw_folder(acts_ui.painter(), f_rect.shrink(5.5), f_color);
                if f_resp.on_hover_text("Show in folder").clicked() {
                    platform::shell_open_folder_and_select(&record.target_path);
                }
            }
        }
    }
}

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use eframe::egui::{self, pos2, Pos2, Rect, Rounding, Stroke, Vec2};
use rustget_core::{
    db::DownloadRecord,
    downloader::DownloadProgress,
    queue::QueueManager,
    speed::SpeedTracker,
};
use crate::icons;
use crate::platform;
use crate::theme::ThemeColors;

pub fn render_download_table(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    rows: &[&DownloadRecord],
    progress_map: &HashMap<String, DownloadProgress>,
    ema_speeds: &HashMap<String, f64>,
    selected_ids: &mut HashSet<String>,
    queue: &Arc<QueueManager>,
) {
    let avail_w = ui.available_width();
    let compact = avail_w < 580.0;

    // Header (.th, 34px)
    let th_rect = ui.allocate_space(Vec2::new(avail_w, 34.0)).1;
    ui.painter().rect_filled(th_rect, 0.0, pal.surface);
    ui.painter().rect_filled(
        Rect::from_min_max(Pos2::new(th_rect.left(), th_rect.bottom() - 1.0), th_rect.max),
        0.0,
        pal.line,
    );

    let cols = compute_columns(th_rect, compact);

    // Header select-all checkbox
    let mut all_selected = !rows.is_empty() && rows.iter().all(|r| selected_ids.contains(&r.id));
    let cb_rect = Rect::from_center_size(pos2(cols.cb_x + 8.0, th_rect.center().y), Vec2::splat(16.0));
    if draw_checkbox(ui, ui.id().with("hdr_cb"), cb_rect, pal, all_selected) {
        all_selected = !all_selected;
        if all_selected {
            for r in rows {
                selected_ids.insert(r.id.clone());
            }
        } else {
            for r in rows {
                selected_ids.remove(&r.id);
            }
        }
    }

    let hdr_font = egui::FontId::proportional(12.0);
    let mid_y = th_rect.center().y;
    ui.painter().text(pos2(cols.name_x, mid_y), egui::Align2::LEFT_CENTER, "File name", hdr_font.clone(), pal.muted);
    if !compact {
        ui.painter().text(pos2(cols.size_x, mid_y), egui::Align2::LEFT_CENTER, "Size", hdr_font.clone(), pal.muted);
    }
    ui.painter().text(pos2(cols.prog_x, mid_y), egui::Align2::LEFT_CENTER, "Progress", hdr_font.clone(), pal.muted);
    if !compact {
        ui.painter().text(pos2(cols.speed_x, mid_y), egui::Align2::LEFT_CENTER, "Speed", hdr_font.clone(), pal.muted);
    }
    ui.painter().text(pos2(cols.status_x, mid_y), egui::Align2::LEFT_CENTER, "Status", hdr_font, pal.muted);

    // Rows area (.rows)
    if rows.is_empty() {
        ui.add_space(32.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("No downloads in this category")
                    .size(14.0)
                    .color(pal.muted),
            );
        });
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("download_table_rows")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for record in rows {
                render_table_row(ui, pal, record, progress_map, ema_speeds, selected_ids, queue, compact);
            }
        });
}

struct ColLayout {
    cb_x: f32,
    name_x: f32,
    name_w: f32,
    size_x: f32,
    prog_x: f32,
    prog_w: f32,
    speed_x: f32,
    status_x: f32,
}

fn compute_columns(row_rect: Rect, compact: bool) -> ColLayout {
    let pad = 16.0;
    let gap = 12.0;
    let left = row_rect.left() + pad;
    let right = row_rect.right() - pad;

    let cb_w = 36.0;
    let size_w = if compact { 0.0 } else { 92.0 };
    let prog_w = if compact { 150.0 } else { 170.0 };
    let speed_w = if compact { 0.0 } else { 100.0 };
    let status_w = if compact { 96.0 } else { 108.0 };

    let status_x = right - status_w;
    let speed_x = if compact { status_x } else { status_x - gap - speed_w };
    let prog_x = speed_x - gap - prog_w;
    let size_x = if compact { prog_x } else { prog_x - gap - size_w };

    let cb_x = left;
    let name_x = cb_x + cb_w + gap;
    let name_w = (size_x - gap - name_x).max(80.0);

    ColLayout {
        cb_x,
        name_x,
        name_w,
        size_x,
        prog_x,
        prog_w,
        speed_x,
        status_x,
    }
}

fn render_table_row(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    record: &DownloadRecord,
    progress_map: &HashMap<String, DownloadProgress>,
    ema_speeds: &HashMap<String, f64>,
    selected_ids: &mut HashSet<String>,
    queue: &Arc<QueueManager>,
    compact: bool,
) {
    let row_rect = ui.allocate_space(Vec2::new(ui.available_width(), 48.0)).1;
    let cols = compute_columns(row_rect, compact);

    let is_selected = selected_ids.contains(&record.id);
    let cb_rect = Rect::from_center_size(pos2(cols.cb_x + 8.0, row_rect.center().y), Vec2::splat(16.0));

    let row_resp = ui.interact(row_rect, ui.id().with(("dl_row", &record.id)), egui::Sense::click());
    let hovered = row_resp.hovered();

    let bg = if is_selected {
        pal.accent_soft
    } else if hovered {
        pal.raise
    } else {
        pal.surface
    };
    ui.painter().rect_filled(row_rect, 0.0, bg);
    ui.painter().rect_filled(
        Rect::from_min_max(Pos2::new(row_rect.left(), row_rect.bottom() - 1.0), row_rect.max),
        0.0,
        pal.line,
    );

    // Checkbox (visually drawn; clicking anywhere on row toggles selection)
    let _cb_clicked = draw_checkbox_visual(ui.painter(), cb_rect, pal, is_selected);

    if row_resp.clicked() {
        if is_selected {
            selected_ids.remove(&record.id);
        } else {
            selected_ids.insert(record.id.clone());
        }
    }

    if row_resp.double_clicked() {
        if record.status == "completed" {
            platform::shell_open_file(&record.target_path);
        } else if record.status == "paused" || record.status == "failed" {
            let q_id = record.id.clone();
            let q = Arc::clone(queue);
            tokio::spawn(async move {
                let _ = q.resume(&q_id).await;
            });
        }
    }

    row_resp.context_menu(|ui| {
        ui.set_min_width(160.0);
        if record.status == "completed" {
            if ui.button("Open file").clicked() {
                platform::shell_open_file(&record.target_path);
                ui.close_menu();
            }
            if ui.button("Show in folder").clicked() {
                platform::shell_open_folder_and_select(&record.target_path);
                ui.close_menu();
            }
            ui.separator();
        } else if record.status == "downloading" || record.status == "queued" {
            if ui.button("Pause").clicked() {
                let q_id = record.id.clone();
                let q = Arc::clone(queue);
                tokio::spawn(async move {
                    let _ = q.pause(&q_id).await;
                });
                ui.close_menu();
            }
        } else if record.status == "paused" || record.status == "failed" {
            if ui.button("Resume").clicked() {
                let q_id = record.id.clone();
                let q = Arc::clone(queue);
                tokio::spawn(async move {
                    let _ = q.resume(&q_id).await;
                });
                ui.close_menu();
            }
        }
        if ui.button("Copy download link").clicked() {
            ui.output_mut(|o| o.copied_text = record.url.clone());
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Delete").clicked() {
            let q_id = record.id.clone();
            let q = Arc::clone(queue);
            selected_ids.remove(&record.id);
            tokio::spawn(async move {
                let _ = q.remove(&q_id, false).await;
            });
            ui.close_menu();
        }
    });

    let live_prog = progress_map.get(&record.id);
    let downloaded = live_prog.map(|p| p.downloaded_bytes).unwrap_or(record.downloaded_bytes);
    let total = live_prog.and_then(|p| p.total_bytes).or(record.total_size);
    let pct = match total {
        Some(t) if t > 0 => ((downloaded as f64) / (t as f64) * 100.0).clamp(0.0, 100.0),
        _ => {
            if record.status == "completed" {
                100.0
            } else {
                0.0
            }
        }
    };

    let mid_y = row_rect.center().y;

    // Col 2: Category icon (18x18) + Filename (truncated)
    let icon_rect = Rect::from_center_size(pos2(cols.name_x + 9.0, mid_y), Vec2::splat(18.0));
    icons::draw_category_symbol(ui.painter(), icon_rect, &record.category, pal.muted);

    let text_rect = Rect::from_min_max(
        pos2(cols.name_x + 18.0 + 12.0, row_rect.top()),
        pos2(cols.name_x + cols.name_w, row_rect.bottom()),
    );
    let mut name_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(text_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    name_ui.set_clip_rect(text_rect);
    name_ui.add(
        egui::Label::new(
            egui::RichText::new(&record.filename)
                .size(14.0)
                .color(pal.text),
        )
        .truncate()
        .selectable(false),
    );

    // Col 3: Size
    if !compact {
        let size_str = total
            .map(SpeedTracker::format_bytes)
            .unwrap_or_else(|| {
                if downloaded > 0 {
                    SpeedTracker::format_bytes(downloaded)
                } else {
                    "—".to_string()
                }
            });
        ui.painter().text(
            pos2(cols.size_x, mid_y),
            egui::Align2::LEFT_CENTER,
            size_str,
            egui::FontId::proportional(12.0),
            pal.muted,
        );
    }

    // Col 4: Progress bar (3px) + 5px gap + pct
    let bar_y = row_rect.top() + 13.0;
    let bar_rect = Rect::from_min_size(pos2(cols.prog_x, bar_y), Vec2::new(cols.prog_w, 3.0));
    let track_color = if hovered { pal.line } else { pal.raise };
    ui.painter().rect_filled(bar_rect, Rounding::same(2.0), track_color);

    let is_paused = record.status == "paused";
    let fill_color = if is_paused { pal.muted } else { pal.accent };
    let ratio = (pct / 100.0).clamp(0.0, 1.0) as f32;
    let fill_w = (cols.prog_w * ratio).max(if ratio > 0.0 { 2.0 } else { 0.0 });
    if fill_w > 0.0 {
        ui.painter().rect_filled(
            Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, 3.0)),
            Rounding::same(2.0),
            fill_color,
        );
    }

    ui.painter().text(
        pos2(cols.prog_x, bar_y + 3.0 + 5.0),
        egui::Align2::LEFT_TOP,
        format!("{}%", pct.round() as u32),
        egui::FontId::proportional(12.0),
        pal.muted,
    );

    // Col 5: Speed
    if !compact {
        let speed_str = if record.status == "downloading" {
            let speed = live_prog
                .map(|p| ema_speeds.get(&record.id).copied().unwrap_or(p.speed_bps))
                .unwrap_or(0.0);
            SpeedTracker::format_speed(speed)
        } else {
            "—".to_string()
        };
        ui.painter().text(
            pos2(cols.speed_x, mid_y),
            egui::Align2::LEFT_CENTER,
            speed_str,
            egui::FontId::proportional(12.0),
            pal.muted,
        );
    }

    // Col 6: Status
    let status_label = match record.status.as_str() {
        "downloading" => "Downloading",
        "paused" => "Paused",
        "completed" => "Completed",
        "queued" => "Queued",
        "failed" => "Failed",
        other => other,
    };
    ui.painter().text(
        pos2(cols.status_x, mid_y),
        egui::Align2::LEFT_CENTER,
        status_label,
        egui::FontId::proportional(12.0),
        pal.muted,
    );
}

fn draw_checkbox(
    ui: &mut egui::Ui,
    id: egui::Id,
    rect: Rect,
    pal: &ThemeColors,
    checked: bool,
) -> bool {
    let resp = ui.interact(rect.expand(4.0), id, egui::Sense::click());
    draw_checkbox_visual(ui.painter(), rect, pal, checked);
    resp.clicked()
}

fn draw_checkbox_visual(
    painter: &egui::Painter,
    rect: Rect,
    pal: &ThemeColors,
    checked: bool,
) {
    let r = Rounding::same(3.5);
    if checked {
        painter.rect_filled(rect, r, pal.accent);
        let s = Stroke::new(1.8_f32, pal.on_accent);
        let p1 = pos2(rect.left() + 3.5, rect.center().y);
        let p2 = pos2(rect.left() + 6.5, rect.bottom() - 4.0);
        let p3 = pos2(rect.right() - 3.0, rect.top() + 4.0);
        painter.line_segment([p1, p2], s);
        painter.line_segment([p2, p3], s);
    } else {
        painter.rect_filled(rect, r, pal.surface);
        painter.rect_stroke(rect, r, Stroke::new(1.2_f32, pal.muted));
    }
}

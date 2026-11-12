use eframe::egui::{pos2, vec2, Color32, Painter, Rect, Rounding, Stroke};
use crate::theme::colors;

// ============================================================================
// Cosmo Downloader Vector Icons (Clean 1.6-weight Line Icons & Custom Logo Mark)
// ============================================================================

/// Custom Minimalist Logo Mark:
/// 2 horizontal rounded bars (top full-width, bottom half-width with 55% opacity)
pub fn draw_logo(painter: &Painter, rect: Rect, color: Color32) {
    let w = rect.width();
    let h = rect.height();
    let bar_h = (h * 0.22).max(2.0);
    let r = Rounding::same(bar_h * 0.5);

    // Top bar: width ~81%, y ~19%
    let top_rect = Rect::from_min_size(
        pos2(rect.left() + w * 0.09, rect.top() + h * 0.19),
        vec2(w * 0.82, bar_h),
    );
    painter.rect_filled(top_rect, r, color);

    // Bottom bar: width ~44%, y ~59%, 55% opacity
    let bot_color = Color32::from_rgba_premultiplied(
        ((color.r() as f32) * 0.55) as u8,
        ((color.g() as f32) * 0.55) as u8,
        ((color.b() as f32) * 0.55) as u8,
        ((color.a() as f32) * 0.55) as u8,
    );
    let bot_rect = Rect::from_min_size(
        pos2(rect.left() + w * 0.09, rect.top() + h * 0.59),
        vec2(w * 0.44, bar_h),
    );
    painter.rect_filled(bot_rect, r, bot_color);
}

pub fn draw_add(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.6_f32, color);
    let mid_x = rect.center().x;
    let mid_y = rect.center().y;
    let span = rect.width() * 0.32;
    painter.line_segment([pos2(mid_x, mid_y - span), pos2(mid_x, mid_y + span)], stroke);
    painter.line_segment([pos2(mid_x - span, mid_y), pos2(mid_x + span, mid_y)], stroke);
}

pub fn draw_download(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.6_f32, color);
    let mid_x = rect.center().x;
    let top_y = rect.top() + rect.height() * 0.16;
    let arrow_y = rect.top() + rect.height() * 0.62;
    let bot_y = rect.bottom() - rect.height() * 0.16;
    let wing = rect.width() * 0.22;

    // Stem
    painter.line_segment([pos2(mid_x, top_y), pos2(mid_x, arrow_y)], stroke);
    // Arrow wings
    painter.line_segment([pos2(mid_x - wing, arrow_y - wing * 0.75), pos2(mid_x, arrow_y)], stroke);
    painter.line_segment([pos2(mid_x + wing, arrow_y - wing * 0.75), pos2(mid_x, arrow_y)], stroke);

    // Shelf base
    let shelf_span = rect.width() * 0.32;
    painter.line_segment([pos2(mid_x - shelf_span, bot_y), pos2(mid_x + shelf_span, bot_y)], stroke);
}

pub fn draw_play(painter: &Painter, rect: Rect, color: Color32) {
    let p1 = pos2(rect.left() + rect.width() * 0.30, rect.top() + rect.height() * 0.22);
    let p2 = pos2(rect.left() + rect.width() * 0.76, rect.center().y);
    let p3 = pos2(rect.left() + rect.width() * 0.30, rect.bottom() - rect.height() * 0.22);
    painter.add(eframe::egui::epaint::PathShape::convex_polygon(vec![p1, p2, p3], color, Stroke::NONE));
}

pub fn draw_pause(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.6_f32, color);
    let x1 = rect.left() + rect.width() * 0.36;
    let x2 = rect.right() - rect.width() * 0.36;
    let y1 = rect.top() + rect.height() * 0.25;
    let y2 = rect.bottom() - rect.height() * 0.25;
    painter.line_segment([pos2(x1, y1), pos2(x1, y2)], stroke);
    painter.line_segment([pos2(x2, y1), pos2(x2, y2)], stroke);
}

pub fn draw_check(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.6_f32, color);
    let p1 = pos2(rect.left() + rect.width() * 0.22, rect.top() + rect.height() * 0.52);
    let p2 = pos2(rect.left() + rect.width() * 0.42, rect.top() + rect.height() * 0.72);
    let p3 = pos2(rect.left() + rect.width() * 0.80, rect.top() + rect.height() * 0.30);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
}

pub fn draw_folder(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.6_f32, color);
    let l = rect.left() + 2.0;
    let r = rect.right() - 2.0;
    let t = rect.top() + 3.0;
    let b = rect.bottom() - 3.0;
    let tab_w = (r - l) * 0.42;
    let tab_h = (b - t) * 0.24;

    // Folder outline
    let pts = vec![
        pos2(l, t + tab_h),
        pos2(l, b),
        pos2(r, b),
        pos2(r, t + tab_h),
        pos2(l + tab_w + 3.0, t + tab_h),
        pos2(l + tab_w, t),
        pos2(l + 2.0, t),
        pos2(l, t + tab_h),
    ];
    for i in 0..pts.len() - 1 {
        painter.line_segment([pts[i], pts[i + 1]], stroke);
    }
}

pub fn draw_video(painter: &Painter, rect: Rect, color: Color32) {
    let frame_rect = rect.shrink(2.0);
    painter.rect_stroke(frame_rect, Rounding::same(2.5), Stroke::new(1.5_f32, color));
    let p1 = pos2(rect.left() + rect.width() * 0.42, rect.top() + rect.height() * 0.35);
    let p2 = pos2(rect.left() + rect.width() * 0.68, rect.center().y);
    let p3 = pos2(rect.left() + rect.width() * 0.42, rect.bottom() - rect.height() * 0.35);
    painter.add(eframe::egui::epaint::PathShape::convex_polygon(vec![p1, p2, p3], color, Stroke::NONE));
}

pub fn draw_music(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let r = (rect.width() * 0.12).max(1.8);
    let c1 = pos2(rect.left() + rect.width() * 0.30, rect.bottom() - rect.height() * 0.26);
    let c2 = pos2(rect.right() - rect.width() * 0.26, rect.bottom() - rect.height() * 0.36);
    painter.circle_stroke(c1, r, stroke);
    painter.circle_stroke(c2, r, stroke);

    let stem1_top = pos2(c1.x + r, rect.top() + rect.height() * 0.24);
    let stem2_top = pos2(c2.x + r, rect.top() + rect.height() * 0.16);
    painter.line_segment([pos2(c1.x + r, c1.y), stem1_top], stroke);
    painter.line_segment([pos2(c2.x + r, c2.y), stem2_top], stroke);
    painter.line_segment([stem1_top, stem2_top], stroke);
}

pub fn draw_documents(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let doc_rect = Rect::from_min_max(pos2(rect.left() + 3.0, rect.top() + 2.0), pos2(rect.right() - 3.0, rect.bottom() - 2.0));
    painter.rect_stroke(doc_rect, Rounding::same(2.0), stroke);
    let pad = 3.5;
    let y1 = doc_rect.top() + doc_rect.height() * 0.40;
    let y2 = doc_rect.top() + doc_rect.height() * 0.65;
    painter.line_segment([pos2(doc_rect.left() + pad, y1), pos2(doc_rect.right() - pad, y1)], stroke);
    painter.line_segment([pos2(doc_rect.left() + pad, y2), pos2(doc_rect.left() + doc_rect.width() * 0.55, y2)], stroke);
}

pub fn draw_compressed(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let top_box = Rect::from_min_max(pos2(rect.left() + 2.0, rect.top() + 2.0), pos2(rect.right() - 2.0, rect.top() + rect.height() * 0.40));
    let bot_box = Rect::from_min_max(pos2(rect.left() + 2.8, rect.top() + rect.height() * 0.38), pos2(rect.right() - 2.8, rect.bottom() - 2.0));
    painter.rect_stroke(top_box, Rounding::same(1.5), stroke);
    painter.rect_stroke(bot_box, Rounding::same(1.5), stroke);
    let mid_x = bot_box.center().x;
    let seam_y = bot_box.top() + bot_box.height() * 0.45;
    painter.line_segment([pos2(mid_x - 3.0, seam_y), pos2(mid_x + 3.0, seam_y)], stroke);
}

pub fn draw_programs(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let frame_rect = rect.shrink(2.0);
    painter.rect_stroke(frame_rect, Rounding::same(2.5), stroke);
    let p1 = pos2(rect.left() + 4.5, rect.top() + rect.height() * 0.38);
    let p2 = pos2(rect.left() + 8.0, rect.center().y);
    let p3 = pos2(rect.left() + 4.5, rect.bottom() - rect.height() * 0.38);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
    let cursor_y = rect.bottom() - rect.height() * 0.38;
    painter.line_segment([pos2(rect.left() + 10.0, cursor_y), pos2(rect.left() + 14.0, cursor_y)], stroke);
}

pub fn draw_settings(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let y1 = rect.top() + rect.height() * 0.32;
    let y2 = rect.top() + rect.height() * 0.68;
    let l = rect.left() + 2.0;
    let r = rect.right() - 2.0;
    let c1 = pos2(l + (r - l) * 0.35, y1);
    let c2 = pos2(l + (r - l) * 0.65, y2);

    painter.line_segment([pos2(l, y1), pos2(r, y1)], stroke);
    painter.line_segment([pos2(l, y2), pos2(r, y2)], stroke);
    painter.circle_filled(c1, 2.5, colors::SURFACE);
    painter.circle_stroke(c1, 2.5, stroke);
    painter.circle_filled(c2, 2.5, colors::SURFACE);
    painter.circle_stroke(c2, 2.5, stroke);
}

pub fn draw_more(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let dx = rect.width() * 0.22;
    painter.circle_filled(pos2(c.x - dx, c.y), 1.6, color);
    painter.circle_filled(c, 1.6, color);
    painter.circle_filled(pos2(c.x + dx, c.y), 1.6, color);
}

pub fn draw_close(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let span = rect.width() * 0.28;
    let c = rect.center();
    painter.line_segment([pos2(c.x - span, c.y - span), pos2(c.x + span, c.y + span)], stroke);
    painter.line_segment([pos2(c.x + span, c.y - span), pos2(c.x - span, c.y + span)], stroke);
}

pub fn draw_all(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.4_f32, color);
    let off = rect.width() * 0.22;
    let c = rect.center();
    let sq_size = (rect.width() * 0.26).max(3.5);
    for &(dx, dy) in &[(-off, -off), (off, -off), (-off, off), (off, off)] {
        let r = Rect::from_center_size(pos2(c.x + dx, c.y + dy), vec2(sq_size, sq_size));
        painter.rect_stroke(r, Rounding::same(1.2), stroke);
    }
}

pub fn draw_other(painter: &Painter, rect: Rect, color: Color32) {
    painter.circle_stroke(rect.center(), rect.width() * 0.32, Stroke::new(1.4_f32, color));
}

pub fn draw_category_symbol(painter: &Painter, rect: Rect, category: &str) {
    let cat_lower = category.trim().to_ascii_lowercase();
    let icon_color = Color32::from_rgb(0xD0, 0xD6, 0xE2);
    match cat_lower.as_str() {
        "video" | "videos" => draw_video(painter, rect, icon_color),
        "music" => draw_music(painter, rect, icon_color),
        "documents" | "document" => draw_documents(painter, rect, icon_color),
        "compressed" | "archive" | "archives" => draw_compressed(painter, rect, icon_color),
        "programs" | "program" | "app" | "apps" => draw_programs(painter, rect, icon_color),
        _ => draw_other(painter, rect, icon_color),
    }
}

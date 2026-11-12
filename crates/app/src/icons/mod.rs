use eframe::egui::{pos2, vec2, Color32, Painter, Rect, Rounding, Stroke};

// ============================================================================
// Cosmo Downloader Vector Icons (1.6-weight Line Icons matching HTML symbols)
// ============================================================================

/// Custom Minimalist Logo Mark (for dialogs/tray)
pub fn draw_logo(painter: &Painter, rect: Rect, color: Color32) {
    let w = rect.width();
    let h = rect.height();
    let bar_h = (h * 0.22).max(2.0);
    let r = Rounding::same(bar_h * 0.5);

    let top_rect = Rect::from_min_size(
        pos2(rect.left() + w * 0.09, rect.top() + h * 0.19),
        vec2(w * 0.82, bar_h),
    );
    painter.rect_filled(top_rect, r, color);

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

#[inline]
fn pt(rect: Rect, nx: f32, ny: f32) -> eframe::egui::Pos2 {
    pos2(rect.left() + rect.width() * nx, rect.top() + rect.height() * ny)
}

pub fn draw_all(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let r = Rounding::same(1.5);
    let s = vec2(rect.width() * 0.25, rect.height() * 0.25);
    painter.rect_stroke(Rect::from_min_size(pt(rect, 4.0 / 24.0, 4.0 / 24.0), s), r, stroke);
    painter.rect_stroke(Rect::from_min_size(pt(rect, 14.0 / 24.0, 4.0 / 24.0), s), r, stroke);
    painter.rect_stroke(Rect::from_min_size(pt(rect, 4.0 / 24.0, 14.0 / 24.0), s), r, stroke);
    painter.rect_stroke(Rect::from_min_size(pt(rect, 14.0 / 24.0, 14.0 / 24.0), s), r, stroke);
}

pub fn draw_video(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let frame = Rect::from_min_max(pt(rect, 3.5 / 24.0, 5.0 / 24.0), pt(rect, 20.5 / 24.0, 19.0 / 24.0));
    painter.rect_stroke(frame, Rounding::same(2.2), stroke);
    let p1 = pt(rect, 10.5 / 24.0, 9.5 / 24.0);
    let p2 = pt(rect, 14.5 / 24.0, 12.0 / 24.0);
    let p3 = pt(rect, 10.5 / 24.0, 14.5 / 24.0);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
    painter.line_segment([p3, p1], stroke);
}

pub fn draw_music(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let rad = (rect.width() * 2.0 / 24.0).max(1.8);
    let c1 = pt(rect, 7.0 / 24.0, 18.0 / 24.0);
    let c2 = pt(rect, 17.0 / 24.0, 16.0 / 24.0);
    painter.circle_stroke(c1, rad, stroke);
    painter.circle_stroke(c2, rad, stroke);
    painter.line_segment([pt(rect, 9.0 / 24.0, 18.0 / 24.0), pt(rect, 9.0 / 24.0, 6.0 / 24.0)], stroke);
    painter.line_segment([pt(rect, 9.0 / 24.0, 6.0 / 24.0), pt(rect, 19.0 / 24.0, 4.0 / 24.0)], stroke);
    painter.line_segment([pt(rect, 19.0 / 24.0, 4.0 / 24.0), pt(rect, 19.0 / 24.0, 16.0 / 24.0)], stroke);
}

pub fn draw_documents(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let pts = [
        pt(rect, 6.0 / 24.0, 3.5 / 24.0),
        pt(rect, 14.0 / 24.0, 3.5 / 24.0),
        pt(rect, 18.0 / 24.0, 7.5 / 24.0),
        pt(rect, 18.0 / 24.0, 20.5 / 24.0),
        pt(rect, 6.0 / 24.0, 20.5 / 24.0),
        pt(rect, 6.0 / 24.0, 3.5 / 24.0),
    ];
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }
    painter.line_segment([pt(rect, 14.0 / 24.0, 3.5 / 24.0), pt(rect, 14.0 / 24.0, 7.5 / 24.0)], stroke);
    painter.line_segment([pt(rect, 14.0 / 24.0, 7.5 / 24.0), pt(rect, 18.0 / 24.0, 7.5 / 24.0)], stroke);
    painter.line_segment([pt(rect, 9.0 / 24.0, 13.0 / 24.0), pt(rect, 15.0 / 24.0, 13.0 / 24.0)], stroke);
    painter.line_segment([pt(rect, 9.0 / 24.0, 16.5 / 24.0), pt(rect, 15.0 / 24.0, 16.5 / 24.0)], stroke);
}

pub fn draw_compressed(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let top = Rect::from_min_max(pt(rect, 4.0 / 24.0, 4.0 / 24.0), pt(rect, 20.0 / 24.0, 9.0 / 24.0));
    painter.rect_stroke(top, Rounding::same(1.4), stroke);
    let bl = pt(rect, 5.5 / 24.0, 9.0 / 24.0);
    let br = pt(rect, 18.5 / 24.0, 9.0 / 24.0);
    let bbl = pt(rect, 5.5 / 24.0, 19.5 / 24.0);
    let bbr = pt(rect, 18.5 / 24.0, 19.5 / 24.0);
    painter.line_segment([bl, bbl], stroke);
    painter.line_segment([bbl, bbr], stroke);
    painter.line_segment([bbr, br], stroke);
    painter.line_segment([pt(rect, 10.0 / 24.0, 13.0 / 24.0), pt(rect, 14.0 / 24.0, 13.0 / 24.0)], stroke);
}

pub fn draw_programs(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let frame = Rect::from_min_max(pt(rect, 3.5 / 24.0, 4.5 / 24.0), pt(rect, 20.5 / 24.0, 19.5 / 24.0));
    painter.rect_stroke(frame, Rounding::same(2.2), stroke);
    painter.line_segment([pt(rect, 8.0 / 24.0, 10.0 / 24.0), pt(rect, 11.0 / 24.0, 12.5 / 24.0)], stroke);
    painter.line_segment([pt(rect, 11.0 / 24.0, 12.5 / 24.0), pt(rect, 8.0 / 24.0, 15.0 / 24.0)], stroke);
    painter.line_segment([pt(rect, 13.0 / 24.0, 15.0 / 24.0), pt(rect, 16.5 / 24.0, 15.0 / 24.0)], stroke);
}

pub fn draw_add(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.6_f32, color);
    painter.line_segment([pt(rect, 0.5, 5.0 / 24.0), pt(rect, 0.5, 19.0 / 24.0)], stroke);
    painter.line_segment([pt(rect, 5.0 / 24.0, 0.5), pt(rect, 19.0 / 24.0, 0.5)], stroke);
}

pub fn draw_play(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.6_f32, color);
    let p1 = pt(rect, 8.0 / 24.0, 5.5 / 24.0);
    let p2 = pt(rect, 18.0 / 24.0, 12.0 / 24.0);
    let p3 = pt(rect, 8.0 / 24.0, 18.5 / 24.0);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
    painter.line_segment([p3, p1], stroke);
}

pub fn draw_pause(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.6_f32, color);
    painter.line_segment([pt(rect, 9.0 / 24.0, 6.0 / 24.0), pt(rect, 9.0 / 24.0, 18.0 / 24.0)], stroke);
    painter.line_segment([pt(rect, 15.0 / 24.0, 6.0 / 24.0), pt(rect, 15.0 / 24.0, 18.0 / 24.0)], stroke);
}

pub fn draw_trash(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    // Top bar
    painter.line_segment([pt(rect, 5.0 / 24.0, 7.0 / 24.0), pt(rect, 19.0 / 24.0, 7.0 / 24.0)], stroke);
    // Handle
    painter.line_segment([pt(rect, 10.0 / 24.0, 7.0 / 24.0), pt(rect, 10.0 / 24.0, 4.5 / 24.0)], stroke);
    painter.line_segment([pt(rect, 10.0 / 24.0, 4.5 / 24.0), pt(rect, 14.0 / 24.0, 4.5 / 24.0)], stroke);
    painter.line_segment([pt(rect, 14.0 / 24.0, 4.5 / 24.0), pt(rect, 14.0 / 24.0, 7.0 / 24.0)], stroke);
    // Basket
    painter.line_segment([pt(rect, 7.0 / 24.0, 7.0 / 24.0), pt(rect, 8.0 / 24.0, 19.5 / 24.0)], stroke);
    painter.line_segment([pt(rect, 8.0 / 24.0, 19.5 / 24.0), pt(rect, 16.0 / 24.0, 19.5 / 24.0)], stroke);
    painter.line_segment([pt(rect, 16.0 / 24.0, 19.5 / 24.0), pt(rect, 17.0 / 24.0, 7.0 / 24.0)], stroke);
    // Inner ribs
    painter.line_segment([pt(rect, 10.5 / 24.0, 11.0 / 24.0), pt(rect, 10.5 / 24.0, 16.0 / 24.0)], stroke);
    painter.line_segment([pt(rect, 13.5 / 24.0, 11.0 / 24.0), pt(rect, 13.5 / 24.0, 16.0 / 24.0)], stroke);
}

pub fn draw_settings(painter: &Painter, rect: Rect, color: Color32, bg_fill: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let y1 = 7.0 / 24.0;
    let y2 = 17.0 / 24.0;
    let c1 = pt(rect, 16.0 / 24.0, y1);
    let c2 = pt(rect, 8.0 / 24.0, y2);
    let rad = (rect.width() * 2.0 / 24.0).max(2.0);

    painter.line_segment([pt(rect, 4.0 / 24.0, y1), pt(rect, 14.0 / 24.0, y1)], stroke);
    painter.line_segment([pt(rect, 18.0 / 24.0, y1), pt(rect, 20.0 / 24.0, y1)], stroke);
    painter.line_segment([pt(rect, 4.0 / 24.0, y2), pt(rect, 6.0 / 24.0, y2)], stroke);
    painter.line_segment([pt(rect, 10.0 / 24.0, y2), pt(rect, 20.0 / 24.0, y2)], stroke);
    painter.circle_filled(c1, rad, bg_fill);
    painter.circle_stroke(c1, rad, stroke);
    painter.circle_filled(c2, rad, bg_fill);
    painter.circle_stroke(c2, rad, stroke);
}

pub fn draw_min(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    painter.line_segment([pt(rect, 5.0 / 24.0, 0.5), pt(rect, 19.0 / 24.0, 0.5)], stroke);
}

pub fn draw_max(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let frame = Rect::from_min_max(pt(rect, 5.5 / 24.0, 5.5 / 24.0), pt(rect, 18.5 / 24.0, 18.5 / 24.0));
    painter.rect_stroke(frame, Rounding::same(1.5), stroke);
}

pub fn draw_close(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    painter.line_segment([pt(rect, 6.0 / 24.0, 6.0 / 24.0), pt(rect, 18.0 / 24.0, 18.0 / 24.0)], stroke);
    painter.line_segment([pt(rect, 18.0 / 24.0, 6.0 / 24.0), pt(rect, 6.0 / 24.0, 18.0 / 24.0)], stroke);
}

pub fn draw_moon(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    // Crescent moon arc approximated cleanly with 20 points
    let c = pt(rect, 0.5, 0.5);
    let r = rect.width() * 0.34;
    let mut pts = Vec::with_capacity(24);
    for i in 0..=16 {
        let a = std::f32::consts::FRAC_PI_2 * 0.2 + (i as f32 / 16.0) * std::f32::consts::PI * 1.45;
        pts.push(pos2(c.x + r * a.cos(), c.y + r * a.sin()));
    }
    for i in (0..=16).rev() {
        let t = i as f32 / 16.0;
        let a = std::f32::consts::FRAC_PI_2 * 0.2 + t * std::f32::consts::PI * 1.45;
        let inner = r * 0.72;
        pts.push(pos2(c.x + r * 0.32 + inner * a.cos(), c.y - r * 0.22 + inner * a.sin()));
    }
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }
    if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
        painter.line_segment([*last, *first], stroke);
    }
}

pub fn draw_sun(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let c = pt(rect, 0.5, 0.5);
    let r = rect.width() * (4.0 / 24.0);
    painter.circle_stroke(c, r, stroke);
    let r1 = rect.width() * (7.0 / 24.0);
    let r2 = rect.width() * (9.0 / 24.0);
    for i in 0..8 {
        let a = (i as f32) * std::f32::consts::FRAC_PI_4;
        painter.line_segment(
            [pos2(c.x + r1 * a.cos(), c.y + r1 * a.sin()), pos2(c.x + r2 * a.cos(), c.y + r2 * a.sin())],
            stroke,
        );
    }
}

pub fn draw_download(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.6_f32, color);
    let mid_x = rect.center().x;
    let top_y = rect.top() + rect.height() * 0.16;
    let arrow_y = rect.top() + rect.height() * 0.62;
    let bot_y = rect.bottom() - rect.height() * 0.16;
    let wing = rect.width() * 0.22;

    painter.line_segment([pos2(mid_x, top_y), pos2(mid_x, arrow_y)], stroke);
    painter.line_segment([pos2(mid_x - wing, arrow_y - wing * 0.75), pos2(mid_x, arrow_y)], stroke);
    painter.line_segment([pos2(mid_x + wing, arrow_y - wing * 0.75), pos2(mid_x, arrow_y)], stroke);

    let shelf_span = rect.width() * 0.32;
    painter.line_segment([pos2(mid_x - shelf_span, bot_y), pos2(mid_x + shelf_span, bot_y)], stroke);
}

pub fn draw_check(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.6_f32, color);
    let p1 = pt(rect, 0.22, 0.52);
    let p2 = pt(rect, 0.42, 0.72);
    let p3 = pt(rect, 0.80, 0.30);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
}

pub fn draw_folder(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let l = rect.left() + 2.0;
    let r = rect.right() - 2.0;
    let t = rect.top() + 3.0;
    let b = rect.bottom() - 3.0;
    let tab_w = (r - l) * 0.42;
    let tab_h = (b - t) * 0.24;

    let pts = [
        pos2(l, t + tab_h),
        pos2(l, b),
        pos2(r, b),
        pos2(r, t + tab_h),
        pos2(l + tab_w + 3.0, t + tab_h),
        pos2(l + tab_w, t),
        pos2(l + 2.0, t),
        pos2(l, t + tab_h),
    ];
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }
}

pub fn draw_other(painter: &Painter, rect: Rect, color: Color32) {
    draw_documents(painter, rect, color);
}

pub fn draw_category_symbol(painter: &Painter, rect: Rect, category: &str, color: Color32) {
    let cat_lower = category.trim().to_ascii_lowercase();
    match cat_lower.as_str() {
        "video" | "videos" => draw_video(painter, rect, color),
        "music" | "audio" => draw_music(painter, rect, color),
        "documents" | "document" | "docs" => draw_documents(painter, rect, color),
        "compressed" | "archive" | "archives" => draw_compressed(painter, rect, color),
        "programs" | "program" | "app" | "apps" => draw_programs(painter, rect, color),
        _ => draw_other(painter, rect, color),
    }
}

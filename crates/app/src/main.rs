#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, pos2, Color32, Margin, Painter, Rect, Rounding, Stroke, Vec2};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tray_icon::{
    menu::{Menu, MenuItem},
    TrayIcon, TrayIconBuilder,
};

use rustget_core::{
    categorize,
    category::{Category, CategoryFilter},
    db::{Database, DownloadRecord},
    downloader::{DownloadOptions, DownloadProgress},
    queue::{QueueEvent, QueueManager},
    speed::SpeedTracker,
    BridgeServer, ClipboardMonitor, ExtensionDownloadRequest,
};

const ICON_RGBA: &[u8] = include_bytes!("icon_32.rgba");
const APP_TITLE: &str = concat!("Cosmo Downloader v", env!("CARGO_PKG_VERSION"));

pub mod theme {
    use eframe::egui::Color32;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ThemeMode { Light, Dark }

    impl ThemeMode {
        pub fn palette(self) -> &'static Palette { match self { Self::Light => &LIGHT, Self::Dark => &DARK } }
        pub fn toggle(self) -> Self { match self { Self::Light => Self::Dark, Self::Dark => Self::Light } }
        pub fn label(self) -> &'static str { match self { Self::Light => "light", Self::Dark => "dark" } }
        pub fn from_label(s: &str) -> Self { if s == "light" { Self::Light } else { Self::Dark } }
    }

    #[derive(Clone, Copy)]
    pub struct Palette {
        pub bg: Color32,
        pub surface: Color32,
        pub raise: Color32,
        pub line: Color32,
        pub text: Color32,
        pub muted: Color32,
        pub accent: Color32,
        pub on_accent: Color32,
        pub accent_soft: Color32,
        pub danger: Color32,
    }

    const fn rgba_const(r: u8, g: u8, b: u8, a: u8) -> Color32 {
        Color32::from_rgba_premultiplied(
            ((r as u16 * a as u16) / 255) as u8,
            ((g as u16 * a as u16) / 255) as u8,
            ((b as u16 * a as u16) / 255) as u8,
            a,
        )
    }

    pub const LIGHT: Palette = Palette {
        bg:          Color32::from_rgb(0xf3, 0xf4, 0xf6),
        surface:     Color32::from_rgb(0xff, 0xff, 0xff),
        raise:       Color32::from_rgb(0xec, 0xee, 0xf1),
        line:        rgba_const(20, 24, 30, 26),
        text:        Color32::from_rgb(0x1b, 0x1d, 0x21),
        muted:       Color32::from_rgb(0x64, 0x6b, 0x75),
        accent:      Color32::from_rgb(0x44, 0x54, 0x6a),
        on_accent:   Color32::from_rgb(0xff, 0xff, 0xff),
        accent_soft: rgba_const(68, 84, 106, 31),
        danger:      Color32::from_rgb(0xd9, 0x30, 0x25),
    };

    pub const DARK: Palette = Palette {
        bg:          Color32::from_rgb(0x1a, 0x1c, 0x20),
        surface:     Color32::from_rgb(0x22, 0x25, 0x2a),
        raise:       Color32::from_rgb(0x2b, 0x2f, 0x35),
        line:        rgba_const(255, 255, 255, 20),
        text:        Color32::from_rgb(0xe6, 0xe8, 0xeb),
        muted:       Color32::from_rgb(0x90, 0x97, 0xa1),
        accent:      Color32::from_rgb(0x8f, 0xa6, 0xc4),
        on_accent:   Color32::from_rgb(0x0f, 0x1a, 0x2a),
        accent_soft: rgba_const(143, 166, 196, 41),
        danger:      Color32::from_rgb(0xd9, 0x30, 0x25),
    };

    pub const MAIN_WIDTH: f32 = 900.0;
    pub const MAIN_HEIGHT: f32 = 600.0;

    pub fn brighten(c: Color32, factor: f32) -> Color32 {
        let [r, g, b, a] = c.to_array();
        Color32::from_rgba_unmultiplied(
            ((r as f32 * factor).clamp(0.0, 255.0)) as u8,
            ((g as f32 * factor).clamp(0.0, 255.0)) as u8,
            ((b as f32 * factor).clamp(0.0, 255.0)) as u8,
            a,
        )
    }

    pub fn hover_raise(mode: ThemeMode) -> Color32 {
        match mode {
            ThemeMode::Light => Color32::from_rgb(0xe3, 0xe6, 0xeb),
            ThemeMode::Dark => Color32::from_rgb(0x32, 0x37, 0x3f),
        }
    }

    // Compatibility aliases routing to DARK theme by default
    pub const BG_APP: Color32 = DARK.bg;
    pub const BG_PANEL: Color32 = DARK.surface;
    pub const BG_CARD: Color32 = DARK.surface;
    pub const BG_CARD_HOVER: Color32 = DARK.raise;
    pub const BORDER: Color32 = DARK.line;
    pub const BORDER_FOCUS: Color32 = DARK.accent;

    pub const CYAN_ACCENT: Color32 = DARK.accent;
    pub const PURPLE_ACCENT: Color32 = DARK.accent;
    pub const EMERALD_SUCCESS: Color32 = Color32::from_rgb(0x22, 0xC5, 0x5E);
    pub const AMBER_WARNING: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);
    pub const RED_ERROR: Color32 = DARK.danger;

    pub const TEXT_PRIMARY: Color32 = DARK.text;
    pub const TEXT_SECONDARY: Color32 = DARK.muted;
    pub const TEXT_MUTED: Color32 = DARK.muted;
    pub const TEXT_CYAN: Color32 = DARK.accent;
}

mod icons {
    use eframe::egui::{pos2, Color32, Painter, Rect, Rounding, Stroke};

    pub fn draw_play(painter: &Painter, rect: Rect, color: Color32) {
        let p1 = pos2(rect.left() + rect.width() * 0.25, rect.top() + rect.height() * 0.15);
        let p2 = pos2(rect.left() + rect.width() * 0.85, rect.top() + rect.height() * 0.5);
        let p3 = pos2(rect.left() + rect.width() * 0.25, rect.top() + rect.height() * 0.85);
        painter.add(egui::epaint::PathShape::convex_polygon(vec![p1, p2, p3], color, Stroke::NONE));
    }

    pub fn draw_pause(painter: &Painter, rect: Rect, color: Color32) {
        let bar_w = (rect.width() * 0.22).max(2.0);
        let bar_h = rect.height() * 0.7;
        let y = rect.top() + rect.height() * 0.15;
        let r1 = Rect::from_min_size(pos2(rect.left() + rect.width() * 0.2, y), eframe::egui::vec2(bar_w, bar_h));
        let r2 = Rect::from_min_size(pos2(rect.right() - rect.width() * 0.2 - bar_w, y), eframe::egui::vec2(bar_w, bar_h));
        painter.rect_filled(r1, Rounding::same(1.5), color);
        painter.rect_filled(r2, Rounding::same(1.5), color);
    }

    pub fn draw_folder(painter: &Painter, rect: Rect, color: Color32) {
        let tab_w = rect.width() * 0.45;
        let tab_h = rect.height() * 0.25;
        painter.rect_filled(Rect::from_min_size(pos2(rect.left() + 1.0, rect.top() + 2.0), eframe::egui::vec2(tab_w, tab_h)), Rounding::same(1.5), color);
        let body_rect = Rect::from_min_max(pos2(rect.left() + 1.0, rect.top() + rect.height() * 0.22), pos2(rect.right() - 1.0, rect.bottom() - 2.0));
        painter.rect_filled(body_rect, Rounding::same(2.0), color);
    }

    pub fn draw_video(painter: &Painter, rect: Rect, color: Color32) {
        painter.rect_stroke(rect.shrink(1.0), Rounding::same(3.0), Stroke::new(1.2_f32, color));
        let p1 = pos2(rect.left() + rect.width() * 0.38, rect.top() + rect.height() * 0.28);
        let p2 = pos2(rect.left() + rect.width() * 0.72, rect.top() + rect.height() * 0.5);
        let p3 = pos2(rect.left() + rect.width() * 0.38, rect.top() + rect.height() * 0.72);
        painter.add(egui::epaint::PathShape::convex_polygon(vec![p1, p2, p3], color, Stroke::NONE));
    }

    pub fn draw_music(painter: &Painter, rect: Rect, color: Color32) {
        let r = (rect.width() * 0.15).max(1.8);
        let head1 = pos2(rect.left() + rect.width() * 0.3, rect.bottom() - rect.height() * 0.25);
        let head2 = pos2(rect.right() - rect.width() * 0.3, rect.bottom() - rect.height() * 0.35);
        painter.circle_filled(head1, r, color);
        painter.circle_filled(head2, r, color);
        let stem1_top = pos2(head1.x + r - 0.5, rect.top() + rect.height() * 0.2);
        let stem2_top = pos2(head2.x + r - 0.5, rect.top() + rect.height() * 0.1);
        painter.line_segment([pos2(head1.x + r - 0.5, head1.y), stem1_top], Stroke::new(1.4_f32, color));
        painter.line_segment([pos2(head2.x + r - 0.5, head2.y), stem2_top], Stroke::new(1.4_f32, color));
        painter.line_segment([stem1_top, stem2_top], Stroke::new(2.0_f32, color));
    }

    pub fn draw_documents(painter: &Painter, rect: Rect, color: Color32) {
        let doc_rect = Rect::from_min_max(pos2(rect.left() + 2.5, rect.top() + 1.0), pos2(rect.right() - 2.5, rect.bottom() - 1.0));
        painter.rect_stroke(doc_rect, Rounding::same(1.5), Stroke::new(1.2_f32, color));
        let line_pad = 4.0;
        let y1 = doc_rect.top() + doc_rect.height() * 0.32;
        let y2 = doc_rect.top() + doc_rect.height() * 0.52;
        let y3 = doc_rect.top() + doc_rect.height() * 0.72;
        painter.line_segment([pos2(doc_rect.left() + line_pad, y1), pos2(doc_rect.right() - line_pad, y1)], Stroke::new(1.0_f32, color));
        painter.line_segment([pos2(doc_rect.left() + line_pad, y2), pos2(doc_rect.right() - line_pad, y2)], Stroke::new(1.0_f32, color));
        painter.line_segment([pos2(doc_rect.left() + line_pad, y3), pos2(doc_rect.left() + doc_rect.width() * 0.55, y3)], Stroke::new(1.0_f32, color));
    }

    pub fn draw_compressed(painter: &Painter, rect: Rect, color: Color32) {
        let box_rect = Rect::from_min_max(pos2(rect.left() + 2.0, rect.top() + 2.0), pos2(rect.right() - 2.0, rect.bottom() - 2.0));
        painter.rect_stroke(box_rect, Rounding::same(2.0), Stroke::new(1.2_f32, color));
        let mid_x = box_rect.center().x;
        painter.line_segment([pos2(mid_x, box_rect.top()), pos2(mid_x, box_rect.bottom())], Stroke::new(1.0_f32, color));
        for y_pct in [0.25, 0.45, 0.65] {
            let y = box_rect.top() + box_rect.height() * y_pct;
            painter.line_segment([pos2(mid_x - 2.0, y), pos2(mid_x + 2.0, y)], Stroke::new(1.2_f32, color));
        }
    }

    pub fn draw_programs(painter: &Painter, rect: Rect, color: Color32) {
        let p1 = pos2(rect.center().x + 1.0, rect.top() + 1.0);
        let p2 = pos2(rect.left() + 3.0, rect.center().y + 1.0);
        let p3 = pos2(rect.center().x, rect.center().y + 1.0);
        let p4 = pos2(rect.center().x - 1.0, rect.bottom() - 1.0);
        let p5 = pos2(rect.right() - 3.0, rect.center().y - 1.0);
        let p6 = pos2(rect.center().x, rect.center().y - 1.0);
        painter.add(egui::epaint::PathShape::convex_polygon(vec![p1, p2, p3, p4, p5, p6], color, Stroke::NONE));
    }

    pub fn draw_other(painter: &Painter, rect: Rect, color: Color32) {
        painter.circle_stroke(rect.center(), rect.width() * 0.38, Stroke::new(1.2_f32, color));
        painter.circle_filled(rect.center(), 2.0, color);
    }

    pub fn draw_all(painter: &Painter, rect: Rect, color: Color32) {
        let off = rect.width() * 0.22;
        let c = rect.center();
        let r = 2.0;
        painter.circle_filled(pos2(c.x - off, c.y - off), r, color);
        painter.circle_filled(pos2(c.x + off, c.y - off), r, color);
        painter.circle_filled(pos2(c.x - off, c.y + off), r, color);
        painter.circle_filled(pos2(c.x + off, c.y + off), r, color);
    }

    pub fn draw_sparkle(painter: &Painter, rect: Rect, color: Color32) {
        let c = rect.center();
        let rx = rect.width() * 0.45;
        let ry = rect.height() * 0.45;
        let pts = vec![
            pos2(c.x, c.y - ry),
            pos2(c.x + rx * 0.25, c.y - ry * 0.25),
            pos2(c.x + rx, c.y),
            pos2(c.x + rx * 0.25, c.y + ry * 0.25),
            pos2(c.x, c.y + ry),
            pos2(c.x - rx * 0.25, c.y + ry * 0.25),
            pos2(c.x - rx, c.y),
            pos2(c.x - rx * 0.25, c.y - ry * 0.25),
        ];
        painter.add(egui::epaint::PathShape::convex_polygon(pts, color, Stroke::NONE));
    }

    pub fn draw_down(painter: &Painter, rect: Rect, color: Color32) {
        let c = rect.center();
        let stroke = Stroke::new(1.8_f32, color);
        let top_y = rect.top() + rect.height() * 0.16;
        let tip_y = rect.bottom() - rect.height() * 0.28;
        let head_w = rect.width() * 0.24;
        painter.line_segment([pos2(c.x, top_y), pos2(c.x, tip_y)], stroke);
        painter.line_segment([pos2(c.x - head_w, tip_y - head_w), pos2(c.x, tip_y)], stroke);
        painter.line_segment([pos2(c.x + head_w, tip_y - head_w), pos2(c.x, tip_y)], stroke);
        let tray_y = rect.bottom() - rect.height() * 0.14;
        painter.line_segment(
            [pos2(c.x - rect.width() * 0.30, tray_y), pos2(c.x + rect.width() * 0.30, tray_y)],
            stroke,
        );
    }

    pub fn draw_win_min(painter: &Painter, center: eframe::egui::Pos2, color: Color32) {
        painter.line_segment(
            [pos2(center.x - 5.0, center.y), pos2(center.x + 5.0, center.y)],
            Stroke::new(1.2_f32, color),
        );
    }

    pub fn draw_win_max(painter: &Painter, center: eframe::egui::Pos2, color: Color32, maximized: bool) {
        let stroke = Stroke::new(1.2_f32, color);
        if maximized {
            let back = Rect::from_center_size(pos2(center.x + 1.5, center.y - 1.5), eframe::egui::vec2(7.0, 7.0));
            painter.line_segment([pos2(back.left() + 1.5, back.top()), back.right_top()], stroke);
            painter.line_segment([back.right_top(), pos2(back.right(), back.bottom() - 1.5)], stroke);
            let front = Rect::from_center_size(pos2(center.x - 1.0, center.y + 1.0), eframe::egui::vec2(7.0, 7.0));
            painter.rect_stroke(front, Rounding::ZERO, stroke);
        } else {
            let box_rect = Rect::from_center_size(center, eframe::egui::vec2(9.0, 9.0));
            painter.rect_stroke(box_rect, Rounding::ZERO, stroke);
        }
    }

    pub fn draw_win_close(painter: &Painter, center: eframe::egui::Pos2, color: Color32) {
        let stroke = Stroke::new(1.3_f32, color);
        painter.line_segment(
            [pos2(center.x - 4.5, center.y - 4.5), pos2(center.x + 4.5, center.y + 4.5)],
            stroke,
        );
        painter.line_segment(
            [pos2(center.x + 4.5, center.y - 4.5), pos2(center.x - 4.5, center.y + 4.5)],
            stroke,
        );
    }
}

fn draw_category_icon(painter: &Painter, rect: Rect, category: &str, color: Color32) {
    let cat_lower = category.trim().to_ascii_lowercase();
    match cat_lower.as_str() {
        "video" => icons::draw_video(painter, rect, color),
        "music" => icons::draw_music(painter, rect, color),
        "documents" => icons::draw_documents(painter, rect, color),
        "compressed" => icons::draw_compressed(painter, rect, color),
        "programs" => icons::draw_programs(painter, rect, color),
        _ => icons::draw_other(painter, rect, color),
    }
}

fn sidebar_row<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    current: &mut T,
    val: T,
    name: &str,
    count: usize,
    palette: &theme::Palette,
    draw_icon: impl FnOnce(&Painter, Rect, Color32),
) {
    let is_selected = *current == val;
    let row_height = 28.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_height), egui::Sense::click());
    if response.clicked() {
        *current = val;
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let bg = if is_selected {
        palette.accent_soft
    } else if response.hovered() {
        palette.raise
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(6.0), bg);

    let fg_color = if is_selected {
        palette.accent
    } else if response.hovered() {
        palette.text
    } else {
        palette.muted
    };

    // Draw icon at left
    let icon_rect = Rect::from_center_size(pos2(rect.left() + 16.0, rect.center().y), Vec2::splat(13.0));
    draw_icon(ui.painter(), icon_rect, fg_color);

    // Text label
    let text_galley = ui.painter().layout_no_wrap(
        name.to_string(),
        egui::FontId::proportional(13.0),
        fg_color,
    );
    ui.painter().galley(
        pos2(rect.left() + 30.0, rect.center().y - text_galley.size().y / 2.0),
        text_galley,
        fg_color,
    );

    // Count at right
    let badge_color = if is_selected {
        palette.accent
    } else if response.hovered() {
        palette.text
    } else {
        palette.muted
    };
    let galley = ui.painter().layout_no_wrap(count.to_string(), egui::FontId::proportional(12.0), badge_color);
    ui.painter().galley(pos2(rect.right() - galley.size().x - 10.0, rect.center().y - galley.size().y / 2.0), galley, badge_color);
}

fn primary_button(
    ui: &mut egui::Ui,
    palette: &theme::Palette,
    label: &str,
    min_size: Vec2,
    enabled: bool,
) -> egui::Response {
    let font = egui::FontId::proportional(12.5);
    let text_galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), palette.on_accent);
    let desired_w = (text_galley.size().x + 28.0).max(min_size.x);
    let desired_h = min_size.y.max(28.0);
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(desired_w, desired_h), sense);

    let (fill, fg) = if !enabled {
        (palette.accent.gamma_multiply(0.45), palette.on_accent.gamma_multiply(0.45))
    } else if response.hovered() || response.is_pointer_button_down_on() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        (theme::brighten(palette.accent, 1.08), palette.on_accent)
    } else {
        (palette.accent, palette.on_accent)
    };

    ui.painter().rect_filled(rect, Rounding::same(6.0), fill);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, fg);
    ui.painter().galley(
        pos2(rect.center().x - galley.size().x / 2.0, rect.center().y - galley.size().y / 2.0),
        galley,
        fg,
    );
    response
}

fn secondary_button(
    ui: &mut egui::Ui,
    mode: theme::ThemeMode,
    palette: &theme::Palette,
    label: &str,
    min_size: Vec2,
    enabled: bool,
) -> egui::Response {
    let font = egui::FontId::proportional(12.0);
    let measure = ui.painter().layout_no_wrap(label.to_string(), font.clone(), palette.text);
    let desired_w = (measure.size().x + 22.0).max(min_size.x);
    let desired_h = min_size.y.max(28.0);
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(desired_w, desired_h), sense);

    let (fill, fg, stroke_col) = if !enabled {
        (
            palette.raise.gamma_multiply(0.45),
            palette.text.gamma_multiply(0.45),
            palette.line.gamma_multiply(0.45),
        )
    } else if response.hovered() || response.is_pointer_button_down_on() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        (theme::hover_raise(mode), palette.text, palette.line)
    } else {
        (palette.raise, palette.text, palette.line)
    };

    ui.painter().rect(rect, Rounding::same(6.0), fill, Stroke::new(1.0_f32, stroke_col));
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, fg);
    ui.painter().galley(
        pos2(rect.center().x - galley.size().x / 2.0, rect.center().y - galley.size().y / 2.0),
        galley,
        fg,
    );
    response
}

fn restore_main_viewport(ctx: &egui::Context, visible: bool) {
    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
    ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(theme::MAIN_WIDTH, theme::MAIN_HEIGHT)));
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(APP_TITLE.to_string()));
    if !visible {
        hide_main_window();
    }
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(visible));
    if visible {
        show_main_window();
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
    }
}

fn open_popup_viewport(ctx: &egui::Context, size: Vec2, title: &str) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.to_string()));
    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
    #[cfg(windows)]
    make_popup_topmost();
}

const GITHUB_RELEASES_LATEST_API: &str = "https://api.github.com/repos/Itsmahedi/Cosmo-Downlaoder/releases/latest";
const GITHUB_REPO_URL: &str = "https://github.com/Itsmahedi/Cosmo-Downlaoder";

fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    let core = trimmed.split(['-', '+']).next().unwrap_or(trimmed);
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    Some((major, minor, patch))
}

fn is_newer_version(candidate: &str, current: &str) -> bool {
    match (parse_semver(candidate), parse_semver(current)) {
        (Some(cand), Some(curr)) => cand > curr,
        _ => false,
    }
}

fn open_url_in_browser(url: &str) {
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new("rundll32");
        cmd.args(["url.dll,FileProtocolHandler", url]);
        let _ = cmd.spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
}

#[derive(serde::Deserialize)]
struct GithubLatestRelease {
    tag_name: String,
    html_url: String,
}

fn spawn_update_check(
    db: Database,
    ctx: egui::Context,
) -> tokio::sync::mpsc::Receiver<Option<(String, String)>> {
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = db.set_setting("last_update_check_ts", &now_secs.to_string()).await;

        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent(concat!("CosmoDownloader/", env!("CARGO_PKG_VERSION")))
            .build()
        {
            Ok(c) => c,
            Err(_) => {
                let _ = tx.send(None).await;
                ctx.request_repaint();
                return;
            }
        };

        let result = match client.get(GITHUB_RELEASES_LATEST_API).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<GithubLatestRelease>().await {
                    Ok(rel) => {
                        let clean_ver = rel.tag_name.trim().trim_start_matches(['v', 'V']).to_string();
                        if is_newer_version(&clean_ver, env!("CARGO_PKG_VERSION")) {
                            let _ = db.set_setting("latest_release_version", &clean_ver).await;
                            let _ = db.set_setting("latest_release_url", &rel.html_url).await;
                            Some((clean_ver, rel.html_url))
                        } else {
                            let _ = db.set_setting("latest_release_version", "").await;
                            let _ = db.set_setting("latest_release_url", "").await;
                            None
                        }
                    }
                    Err(_) => None,
                }
            }
            _ => None,
        };
        let _ = tx.send(result).await;
        ctx.request_repaint();
    });
    rx
}

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn run_reg_command(args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut cmd = std::process::Command::new("reg");
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.output()
}

fn get_autostart_info() -> (bool, bool) {
    let output = run_reg_command(&["query", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "Cosmo Downloader"]);
    if let Ok(ref out) = output {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            let is_min = s.contains("--minimized");
            return (true, is_min);
        }
    }
    let output2 = run_reg_command(&["query", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "CosmoDownloader"]);
    if let Ok(out) = output2 {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            let is_min = s.contains("--minimized");
            return (true, is_min);
        }
    }
    (false, false)
}

fn set_autostart_enabled(enable: bool, start_minimized: bool) {
    std::thread::spawn(move || {
        if enable {
            if let Ok(exe) = std::env::current_exe() {
                let cmd = if start_minimized {
                    format!("\"{}\" --minimized", exe.display())
                } else {
                    format!("\"{}\"", exe.display())
                };
                let _ = run_reg_command(&["add", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "Cosmo Downloader", "/t", "REG_SZ", "/d", &cmd, "/f"]);
            }
        } else {
            let _ = run_reg_command(&["delete", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "Cosmo Downloader", "/f"]);
            let _ = run_reg_command(&["delete", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "Cosmo Space Downloader", "/f"]);
            let _ = run_reg_command(&["delete", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "CosmoDownloader", "/f"]);
            let _ = run_reg_command(&["delete", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", "RustGet", "/f"]);
        }
    });
}

#[cfg(windows)]
extern "system" {
    fn CreateMutexA(lpMutexAttributes: *mut std::ffi::c_void, bInitialOwner: i32, lpName: *const u8) -> *mut std::ffi::c_void;
    fn GetLastError() -> u32;
    fn GetCurrentProcessId() -> u32;
    fn GetWindowThreadProcessId(hWnd: *mut std::ffi::c_void, lpdwProcessId: *mut u32) -> u32;
    fn EnumWindows(lpEnumFunc: Option<unsafe extern "system" fn(*mut std::ffi::c_void, isize) -> i32>, lParam: isize) -> i32;
    fn GetWindowLongW(hWnd: *mut std::ffi::c_void, nIndex: i32) -> i32;
    fn GetWindowTextLengthW(hWnd: *mut std::ffi::c_void) -> i32;
    fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> *mut std::ffi::c_void;
    fn ShowWindow(hWnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
    fn SetForegroundWindow(hWnd: *mut std::ffi::c_void) -> i32;
    fn SetWindowPos(hWnd: *mut std::ffi::c_void, hWndInsertAfter: *mut std::ffi::c_void, X: i32, Y: i32, cx: i32, cy: i32, uFlags: u32) -> i32;
    fn GetSystemMetrics(nIndex: i32) -> i32;
}

#[cfg(windows)]
const HWND_TOPMOST: *mut std::ffi::c_void = (-1isize) as *mut std::ffi::c_void;
#[cfg(windows)]
const HWND_NOTOPMOST: *mut std::ffi::c_void = (-2isize) as *mut std::ffi::c_void;

#[cfg(windows)]
pub fn get_process_window() -> *mut std::ffi::c_void {
    unsafe {
        let current_pid = GetCurrentProcessId();
        struct EnumData {
            pid: u32,
            hwnd: *mut std::ffi::c_void,
        }
        let mut data = EnumData {
            pid: current_pid,
            hwnd: std::ptr::null_mut(),
        };

        unsafe extern "system" fn enum_proc(hwnd: *mut std::ffi::c_void, lparam: isize) -> i32 {
            let data = &mut *(lparam as *mut EnumData);
            let mut wnd_pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut wnd_pid);
            if wnd_pid == data.pid {
                let style = GetWindowLongW(hwnd, -16 /* GWL_STYLE */) as u32;
                // Exclude child windows (WS_CHILD = 0x40000000)
                if (style & 0x40000000) == 0 {
                    let text_len = GetWindowTextLengthW(hwnd);
                    if text_len > 0 {
                        data.hwnd = hwnd;
                        return 0; // stop enumeration, found real titled window!
                    } else if data.hwnd.is_null() {
                        data.hwnd = hwnd;
                    }
                }
            }
            1 // continue
        }

        EnumWindows(Some(enum_proc), &mut data as *mut _ as isize);

        if !data.hwnd.is_null() {
            return data.hwnd;
        }

        // Fallback: title search
        let titles = [
            concat!("Cosmo Downloader v", env!("CARGO_PKG_VERSION"), "\0"),
            "Download File Info\0",
            "Downloading - Cosmo Downloader\0",
            "Download complete\0",
            "Download Complete\0",
            "Cosmo Downloader\0",
            "CosmoDownloader\0",
        ];
        for t in titles {
            let encoded: Vec<u16> = t.encode_utf16().collect();
            let hwnd = FindWindowW(std::ptr::null(), encoded.as_ptr());
            if !hwnd.is_null() {
                return hwnd;
            }
        }

        std::ptr::null_mut()
    }
}

#[cfg(windows)]
pub fn show_download_popup_window() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            let screen_w = GetSystemMetrics(0 /* SM_CXSCREEN */);
            let screen_h = GetSystemMetrics(1 /* SM_CYSCREEN */);
            let w = 560;
            let h = 310;
            let x = if screen_w > w { (screen_w - w) / 2 } else { 100 };
            let y = if screen_h > h { (screen_h - h) / 2 } else { 100 };
            // Instantly position at 560x310 centered, make topmost, restore and show
            SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, 0x0040 /* SWP_SHOWWINDOW */);
            ShowWindow(hwnd, 9); // SW_RESTORE
            ShowWindow(hwnd, 5); // SW_SHOW
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(windows)]
pub fn show_main_window() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, 9); // SW_RESTORE
            ShowWindow(hwnd, 5); // SW_SHOW
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(windows)]
pub fn make_popup_topmost() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, 9); // SW_RESTORE
            ShowWindow(hwnd, 5); // SW_SHOW
            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0040);
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(windows)]
pub fn remove_popup_topmost() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, 0x0001 | 0x0002);
        }
    }
}

#[cfg(windows)]
pub fn hide_main_window() {
    let hwnd = get_process_window();
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, 0); // SW_HIDE removes window completely from taskbar
        }
    }
}

#[cfg(windows)]
#[repr(C)]
#[allow(non_snake_case)]
struct SHELLEXECUTEINFOW {
    cbSize: u32,
    fMask: u32,
    hwnd: *mut std::ffi::c_void,
    lpVerb: *const u16,
    lpFile: *const u16,
    lpParameters: *const u16,
    lpDirectory: *const u16,
    nShow: i32,
    hInstApp: *mut std::ffi::c_void,
    lpIDList: *mut std::ffi::c_void,
    lpClass: *const u16,
    hkeyClass: *mut std::ffi::c_void,
    dwHotKey: u32,
    hIconOrMonitor: *mut std::ffi::c_void,
    hProcess: *mut std::ffi::c_void,
}

#[cfg(windows)]
#[link(name = "shell32")]
#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(pvReserved: *mut std::ffi::c_void, dwCoInit: u32) -> i32;
    fn CoUninitialize();
    fn ShellExecuteW(
        hwnd: *mut std::ffi::c_void,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> isize;
    fn ShellExecuteExW(pExecInfo: *mut SHELLEXECUTEINFOW) -> i32;
}

#[cfg(windows)]
pub fn shell_open_file(path: &str) {
    let target = path.to_string();
    std::thread::spawn(move || {
        let p = std::path::Path::new(&target);
        let actual_target = if !p.exists() {
            let mut resolved = target.clone();
            if let Some(parent) = p.parent() {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            let entry_path = entry.path();
                            if entry_path.is_file() {
                                if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                                    if name.starts_with(stem) && !name.ends_with(".part") && !name.ends_with(".ytdl") {
                                        resolved = entry_path.to_string_lossy().to_string();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            resolved
        } else {
            target
        };

        let wide: Vec<u16> = actual_target.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let res = ShellExecuteW(
                std::ptr::null_mut(),
                std::ptr::null(), // default verb ("open")
                wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            );
            if (res as isize) <= 32 {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", "", &actual_target])
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();
            }
        }
    });
}

#[cfg(windows)]
pub fn shell_open_with(path: &str) {
    let target = path.to_string();
    std::thread::spawn(move || {
        let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
        let openas_verb: Vec<u16> = "openas\0".encode_utf16().collect();

        unsafe {
            let _ = CoInitializeEx(std::ptr::null_mut(), 2 /* COINIT_APARTMENTTHREADED */);

            // Attempt 1: Native ShellExecuteExW with verb "openas" and SEE_MASK_INVOKEIDLIST
            let mut exec_info: SHELLEXECUTEINFOW = std::mem::zeroed();
            exec_info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            exec_info.fMask = 0x0000000C; // SEE_MASK_INVOKEIDLIST
            exec_info.hwnd = get_process_window();
            exec_info.lpVerb = openas_verb.as_ptr();
            exec_info.lpFile = wide.as_ptr();
            exec_info.nShow = 1; // SW_SHOWNORMAL

            let success = ShellExecuteExW(&mut exec_info);
            if success == 0 {
                // Attempt 2: rundll32 shell32.dll,OpenAs_RunDLL <target>
                let _ = std::process::Command::new("rundll32.exe")
                    .arg("shell32.dll,OpenAs_RunDLL")
                    .arg(&target)
                    .spawn();
            }

            CoUninitialize();
        }
    });
}

#[cfg(windows)]
pub fn shell_open_folder_and_select(path: &str) {
    let target = path.to_string();
    std::thread::spawn(move || {
        let p = std::path::Path::new(&target);

        // 1. If path exists and is a directory, open that directory directly
        if p.is_dir() {
            let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                let res = ShellExecuteW(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    wide.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    1, // SW_SHOWNORMAL
                );
                if (res as isize) > 32 {
                    return;
                }
            }
            let _ = std::process::Command::new("explorer").arg(&target).spawn();
            return;
        }

        // 2. If path exists and is a file, select it in explorer
        if p.is_file() {
            let win_path = target.replace('/', "\\");
            let mut cmd = std::process::Command::new("explorer");
            cmd.raw_arg(format!(r#"/select,"{}""#, win_path));
            if cmd.spawn().is_ok() {
                return;
            }
        }

        // 3. If file doesn't exist at exact target path, search parent directory for matching stem
        let parent_dir = p.parent().and_then(|parent| {
            if parent.exists() && parent.is_dir() {
                Some(parent.to_path_buf())
            } else {
                None
            }
        });

        if let Some(parent) = parent_dir {
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                if let Ok(entries) = std::fs::read_dir(&parent) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.is_file() {
                            if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                                if name.starts_with(stem) && !name.ends_with(".part") && !name.ends_with(".ytdl") {
                                    let win_path = entry_path.to_string_lossy().replace('/', "\\");
                                    let mut cmd = std::process::Command::new("explorer");
                                    cmd.raw_arg(format!(r#"/select,"{}""#, win_path));
                                    if cmd.spawn().is_ok() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Fallback: open parent folder directly
            let parent_str = parent.to_string_lossy().to_string();
            let wide: Vec<u16> = parent_str.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                let res = ShellExecuteW(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    wide.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    1,
                );
                if (res as isize) > 32 {
                    return;
                }
            }
            let _ = std::process::Command::new("explorer").arg(&parent_str).spawn();
            return;
        }

        // 4. Fallback to general Downloads / Videos / C:\
        let fallback = Category::General.default_dir();
        let fallback_str = fallback.to_string_lossy().to_string();
        let wide: Vec<u16> = fallback_str.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let _ = ShellExecuteW(
                std::ptr::null_mut(),
                std::ptr::null(),
                wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            );
        }
    });
}

#[cfg(not(windows))]
pub fn show_download_popup_window() {}

#[cfg(not(windows))]
pub fn show_main_window() {}

#[cfg(not(windows))]
pub fn make_popup_topmost() {}

#[cfg(not(windows))]
pub fn remove_popup_topmost() {}

#[cfg(not(windows))]
pub fn hide_main_window() {}

#[cfg(not(windows))]
pub fn shell_open_file(path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

#[cfg(not(windows))]
pub fn shell_open_with(path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

#[cfg(not(windows))]
pub fn shell_open_folder_and_select(path: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
    }
}

fn build_system_tray(tray_tx: &std::sync::mpsc::Sender<String>, egui_ctx: egui::Context) -> Option<TrayIcon> {
    #[cfg(target_os = "linux")]
    {
        if gtk::init().is_err() {
            tracing::warn!("GTK initialization failed; system tray icon disabled.");
            return None;
        }
    }

    let tray_menu = Menu::new();
    let show_item = MenuItem::new("Show Cosmo Downloader", true, None);
    let add_item = MenuItem::new("Add Download URL...", true, None);
    let exit_item = MenuItem::new("Exit", true, None);
    let _ = tray_menu.append(&show_item);
    let _ = tray_menu.append(&add_item);
    let _ = tray_menu.append(&exit_item);

    let show_id = show_item.id().clone();
    let add_id = add_item.id().clone();
    let exit_id = exit_item.id().clone();

    let tx = tray_tx.clone();
    let ctx_menu = egui_ctx.clone();
    tray_icon::menu::MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
        if event.id == show_id {
            show_main_window();
            let _ = tx.send("show".to_string());
            ctx_menu.request_repaint();
        } else if event.id == add_id {
            show_main_window();
            let _ = tx.send("add".to_string());
            ctx_menu.request_repaint();
        } else if event.id == exit_id {
            std::process::exit(0);
        }
    }));

    let tx_tray = tray_tx.clone();
    let ctx_tray = egui_ctx.clone();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event: tray_icon::TrayIconEvent| {
        match event {
            tray_icon::TrayIconEvent::Click { button: tray_icon::MouseButton::Left, .. }
            | tray_icon::TrayIconEvent::DoubleClick { button: tray_icon::MouseButton::Left, .. } => {
                show_main_window();
                let _ = tx_tray.send("show".to_string());
                ctx_tray.request_repaint();
            }
            _ => {}
        }
    }));

    let tray_icon_img = tray_icon::Icon::from_rgba(ICON_RGBA.to_vec(), 32, 32).ok();
    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip(APP_TITLE);
    if let Some(ref icon) = tray_icon_img {
        builder = builder.with_icon(icon.clone());
    }
    builder.build().ok()
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum FilterTab {
    All,
    Downloading,
    Completed,
    Paused,
}



#[derive(Debug, Clone, Default)]
struct AppCounts {
    status_all: usize,
    status_downloading: usize,
    status_completed: usize,
    status_paused: usize,

    cat_all: usize,
    cat_video: usize,
    cat_music: usize,
    cat_documents: usize,
    cat_compressed: usize,
    cat_programs: usize,
    cat_other: usize,
}

fn compute_app_counts(downloads: &[DownloadRecord]) -> AppCounts {
    let mut counts = AppCounts::default();
    counts.status_all = downloads.len();
    counts.cat_all = downloads.len();

    for d in downloads {
        match d.status.as_str() {
            "downloading" | "queued" => counts.status_downloading += 1,
            "completed" => counts.status_completed += 1,
            "paused" => counts.status_paused += 1,
            _ => {}
        }

        let cat = d.category.trim().to_ascii_lowercase();
        match cat.as_str() {
            "video" => counts.cat_video += 1,
            "music" => counts.cat_music += 1,
            "documents" => counts.cat_documents += 1,
            "compressed" => counts.cat_compressed += 1,
            "programs" => counts.cat_programs += 1,
            _ => counts.cat_other += 1,
        }
    }

    counts
}

struct CosmoDownloaderApp {
    queue: Arc<QueueManager>,
    downloads: Vec<DownloadRecord>,
    active_progress: HashMap<String, DownloadProgress>,
    ema_speeds: HashMap<String, f64>,
    selected_tab: FilterTab,
    selected_category: CategoryFilter,
    counts: AppCounts,
    confirm_delete_id: Option<String>,
    theme_mode: theme::ThemeMode,
    selected_row_ids: std::collections::HashSet<String>,
    logo_texture: egui::TextureHandle,

    // Dialogs & Popup Mode
    is_popup_mode: bool,
    is_standalone_popup: bool,
    standalone_popup_file: Option<String>,
    popup_opened_at: Option<std::time::Instant>,
    show_add_dialog: bool,
    show_settings_dialog: bool,
    show_about_dialog: bool,
    update_available: Option<(String, String)>,
    update_check_rx: Option<tokio::sync::mpsc::Receiver<Option<(String, String)>>>,
    completed_dialog_record: Option<DownloadRecord>,
    is_completed_popup_mode: bool,
    active_popup_task_id: Option<String>,
    active_popup_filename: String,
    active_popup_url: String,
    active_popup_target_path: String,
    is_progress_popup_mode: bool,
    pending_task_rx: Option<tokio::sync::mpsc::Receiver<String>>,

    // Add dialog fields
    add_url: String,
    add_filename: String,
    add_category: String,
    add_save_dir: String,
    add_connections: usize,
    add_speed_limit_kb: u64,
    add_checksum: String,
    add_cookies: Option<String>,
    add_referer: Option<String>,
    add_file_size: Option<u64>,
    add_description: String,
    add_remember_category: bool,
    size_probe_rx: Option<mpsc::Receiver<(Option<u64>, Option<String>)>>,

    // Settings fields
    setting_max_concurrent: usize,
    setting_global_speed_kb: u64,
    setting_clipboard_monitor: bool,
    setting_minimize_to_tray: bool,
    setting_show_tray_icon: bool,
    setting_autostart: bool,
    setting_start_minimized: bool,
    setting_dont_show_complete_dialog: bool,

    // System tray
    tray_icon: Option<TrayIcon>,
    tray_tx: std::sync::mpsc::Sender<String>,
    tray_event_receiver: Option<std::sync::mpsc::Receiver<String>>,

    // Async event receivers
    event_rx: mpsc::Receiver<QueueEvent>,
    clipboard_rx: mpsc::Receiver<String>,
    browser_dl_rx: mpsc::Receiver<ExtensionDownloadRequest>,
    show_rx: mpsc::Receiver<()>,
    initial_minimized_handled: bool,
}

impl CosmoDownloaderApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let db = Database::open_default().unwrap_or_else(|_| Database::open_in_memory().unwrap());
        let saved_theme = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.get_setting("ui_theme_mode").await.ok().flatten().map(|v| theme::ThemeMode::from_label(&v)).unwrap_or(theme::ThemeMode::Dark)
            })
        });

        // Setup Visuals according to saved theme tokens
        let p = saved_theme.palette();
        let mut visuals = if saved_theme == theme::ThemeMode::Dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.window_rounding = Rounding::same(10.0);
        visuals.window_fill = p.surface;
        visuals.panel_fill = p.bg;
        visuals.widgets.noninteractive.bg_fill = p.surface;
        visuals.widgets.inactive.bg_fill = p.raise;
        visuals.widgets.hovered.bg_fill = theme::hover_raise(saved_theme);
        visuals.widgets.active.bg_fill = p.accent_soft;
        cc.egui_ctx.set_visuals(visuals);

        let logo_image = egui::ColorImage::from_rgba_unmultiplied([32, 32], ICON_RGBA);
        let logo_texture = cc.egui_ctx.load_texture("cosmo_app_logo", logo_image, egui::TextureOptions::LINEAR);

        let mut is_standalone_popup = false;
        let mut standalone_popup_file = None;
        let mut standalone_req = None;

        let mut args_iter = std::env::args().skip(1);
        while let Some(arg) = args_iter.next() {
            if arg == "--popup-file" {
                if let Some(file_path) = args_iter.next() {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        if let Ok(req) = serde_json::from_str::<ExtensionDownloadRequest>(&content) {
                            standalone_req = Some(req);
                            is_standalone_popup = true;
                            standalone_popup_file = Some(file_path);
                        }
                    }
                }
            } else if arg.starts_with("--popup-file=") {
                let file_path = arg.trim_start_matches("--popup-file=").to_string();
                if let Ok(content) = std::fs::read_to_string(&file_path) {
                    if let Ok(req) = serde_json::from_str::<ExtensionDownloadRequest>(&content) {
                        standalone_req = Some(req);
                        is_standalone_popup = true;
                        standalone_popup_file = Some(file_path);
                    }
                }
            }
        }

        let is_popup_arg = std::env::args().any(|a| a == "--popup") || is_standalone_popup;

        let (event_tx, event_rx) = mpsc::channel(128);
        let queue = Arc::new(QueueManager::new(db.clone(), 3, 0, event_tx));

        ensure_native_host_registered();

        // Bridge server & notifier channels
        let (bridge_tx, bridge_rx) = mpsc::channel(32);
        let (show_tx, show_rx) = mpsc::channel(16);
        let daemon_online = std::net::TcpStream::connect("127.0.0.1:42424").is_ok();

        // Start background queue processing and bridge server if not handled by another daemon
        if !daemon_online {
            log_daemon("Starting queue worker and bridge server on port 42424");
            let q_worker = Arc::clone(&queue);
            let q_cancel = CancellationToken::new();
            tokio::spawn(async move {
                q_worker.run_queue_worker(q_cancel).await;
            });

            let ctx_wake = cc.egui_ctx.clone();
            let ctx_wake_dl = cc.egui_ctx.clone();
            let bridge_server = BridgeServer::new(Arc::clone(&queue), 42424)
                .with_notifier(bridge_tx)
                .with_show_notifier(show_tx)
                .with_wake_callback(move || {
                    #[cfg(windows)]
                    {
                        show_main_window();
                    }
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(theme::MAIN_WIDTH, theme::MAIN_HEIGHT)));
                    ctx_wake.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx_wake.request_repaint();
                })
                .with_download_wake_callback(move || {
                    log_daemon("Bridge server download wake callback triggered");
                    #[cfg(windows)]
                    {
                        show_download_popup_window();
                    }
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(560.0, 310.0)));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx_wake_dl.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                    ctx_wake_dl.request_repaint();
                });
            let bridge_cancel = CancellationToken::new();
            tokio::spawn(async move {
                bridge_server.run(bridge_cancel).await;
            });
        }

        // Clipboard monitor
        let (clip_tx, clip_rx) = mpsc::channel(16);
        let saved_clip_monitor = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.get_setting("clipboard_monitor_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(true)
            })
        });

        if !is_standalone_popup && !daemon_online && saved_clip_monitor {
            let clip_cancel = CancellationToken::new();
            let ctx_wake_clip = cc.egui_ctx.clone();
            let clip_monitor = ClipboardMonitor::new(clip_cancel)
                .with_wake_callback(move || {
                    #[cfg(windows)]
                    {
                        show_download_popup_window();
                    }
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(560.0, 310.0)));
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx_wake_clip.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                    ctx_wake_clip.request_repaint();
                });
            clip_monitor.start(clip_tx);
        }

        // System tray setup
        let (tray_tx, tray_rx) = std::sync::mpsc::channel();

        // Load saved settings from database
        let saved_minimize_to_tray = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.get_setting("minimize_to_tray_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(true)
            })
        });
        let saved_show_tray_icon = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.get_setting("show_tray_icon_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(true)
            })
        });

        let (saved_autostart, reg_start_minimized) = get_autostart_info();
        let saved_start_min = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.get_setting("start_minimized_enabled").await.ok().flatten().map(|v| v == "true").unwrap_or(reg_start_minimized)
            })
        });

        let egui_ctx = cc.egui_ctx.clone();
        let tray_icon = if !is_standalone_popup && !daemon_online && saved_show_tray_icon {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                build_system_tray(&tray_tx, egui_ctx)
            }))
            .ok()
            .flatten()
        } else {
            None
        };

        let initial_records = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.list_downloads(None).await.unwrap_or_default()
            })
        });
        let initial_counts = compute_app_counts(&initial_records);

        let saved_tab = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.get_setting("ui_selected_tab").await.ok().flatten().and_then(|v| {
                    match v.as_str() {
                        "all" => Some(FilterTab::All),
                        "downloading" => Some(FilterTab::Downloading),
                        "completed" => Some(FilterTab::Completed),
                        "paused" => Some(FilterTab::Paused),
                        _ => None,
                    }
                })
            })
        }).unwrap_or(FilterTab::All);

        let saved_category = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.get_setting("ui_selected_category").await.ok().flatten().and_then(|v| {
                    match v.as_str() {
                        "all" => Some(CategoryFilter::All),
                        "video" => Some(CategoryFilter::Video),
                        "music" => Some(CategoryFilter::Music),
                        "documents" => Some(CategoryFilter::Documents),
                        "compressed" => Some(CategoryFilter::Compressed),
                        "programs" => Some(CategoryFilter::Programs),
                        "other" => Some(CategoryFilter::Other),
                        _ => None,
                    }
                })
            })
        }).unwrap_or(CategoryFilter::All);

        // Load cached update info & schedule background GitHub check (at most once per 24h)
        let (initial_update_available, initial_update_rx) = if !is_standalone_popup {
            let db_upd = db.clone();
            let (cached_upd, should_check) = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let cached_ver = db_upd.get_setting("latest_release_version").await.ok().flatten().unwrap_or_default();
                    let cached_url = db_upd.get_setting("latest_release_url").await.ok().flatten().unwrap_or_default();
                    let last_ts: u64 = db_upd.get_setting("last_update_check_ts").await.ok().flatten()
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(0);
                    let now_secs = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let cached = if !cached_ver.is_empty() && !cached_url.is_empty() && is_newer_version(&cached_ver, env!("CARGO_PKG_VERSION")) {
                        Some((cached_ver, cached_url))
                    } else {
                        None
                    };
                    let due = now_secs.saturating_sub(last_ts) >= 86_400;
                    (cached, due)
                })
            });
            let rx = if should_check {
                Some(spawn_update_check(db.clone(), cc.egui_ctx.clone()))
            } else {
                None
            };
            (cached_upd, rx)
        } else {
            (None, None)
        };

        let mut app = Self {
            queue,
            downloads: initial_records,
            active_progress: HashMap::new(),
            ema_speeds: HashMap::new(),
            selected_tab: saved_tab,
            selected_category: saved_category,
            counts: initial_counts,
            confirm_delete_id: None,
            theme_mode: saved_theme,
            selected_row_ids: std::collections::HashSet::new(),
            logo_texture,
            is_popup_mode: is_popup_arg,
            is_standalone_popup,
            standalone_popup_file,
            popup_opened_at: if is_popup_arg { Some(std::time::Instant::now()) } else { None },
            show_add_dialog: is_popup_arg,
            show_settings_dialog: false,
            show_about_dialog: false,
            update_available: initial_update_available,
            update_check_rx: initial_update_rx,
            completed_dialog_record: None,
            is_completed_popup_mode: false,
            active_popup_task_id: None,
            active_popup_filename: String::new(),
            active_popup_url: String::new(),
            active_popup_target_path: String::new(),
            is_progress_popup_mode: false,
            pending_task_rx: None,
            add_url: String::new(),
            add_filename: String::new(),
            add_category: "General".to_string(),
            add_save_dir: Category::General.default_dir().to_string_lossy().to_string(),
            add_connections: 8,
            add_speed_limit_kb: 0,
            add_checksum: String::new(),
            add_cookies: None,
            add_referer: None,
            add_file_size: None,
            add_description: String::new(),
            add_remember_category: true,
            size_probe_rx: None,
            setting_max_concurrent: 3,
            setting_global_speed_kb: 0,
            setting_clipboard_monitor: saved_clip_monitor,
            setting_minimize_to_tray: saved_minimize_to_tray,
            setting_show_tray_icon: saved_show_tray_icon,
            setting_autostart: saved_autostart,
            setting_start_minimized: saved_start_min,
            setting_dont_show_complete_dialog: false,
            tray_icon,
            tray_tx,
            tray_event_receiver: Some(tray_rx),
            event_rx,
            clipboard_rx: clip_rx,
            browser_dl_rx: bridge_rx,
            show_rx,
            initial_minimized_handled: false,
        };

        if let Some(req) = standalone_req {
            app.prepare_download_dialog(&req.url, req.filename, req.cookies, req.referer, req.num_segments);
        }

        app
    }

    fn refresh_downloads(&mut self) {
        let db = self.queue.db().clone();
        let records = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                db.list_downloads(None).await.unwrap_or_default()
            })
        });
        self.counts = compute_app_counts(&records);
        self.downloads = records;
    }

    fn prepare_download_dialog(&mut self, url: &str, filename: Option<String>, cookies: Option<String>, referer: Option<String>, num_segments: Option<usize>) {
        self.add_url = url.trim().to_string();
        self.add_cookies = cookies.clone();
        self.add_referer = referer.clone();
        self.add_connections = num_segments.unwrap_or(8);
        self.add_description.clear();
        self.add_file_size = None;
        self.add_remember_category = true;

        if let Some(f) = filename {
            if !f.trim().is_empty() {
                self.add_filename = rustget_core::probe::sanitize_filename(&f);
            }
        } else if let Ok(parsed) = url::Url::parse(&self.add_url) {
            let name = rustget_core::probe::extract_filename(&reqwest::header::HeaderMap::new(), &parsed);
            self.add_filename = rustget_core::probe::sanitize_filename(&name);
        }

        if rustget_core::youtube::is_streaming_site(&self.add_url) {
            if self.add_filename.is_empty() || self.add_filename.eq_ignore_ascii_case("watch") {
                self.add_filename = "YouTube_Video.mp4".to_string();
            } else if !self.add_filename.contains('.') {
                self.add_filename.push_str(".mp4");
            }
            self.add_category = "Video".to_string();
            self.add_save_dir = Category::Video.default_dir().to_string_lossy().to_string();
        } else {
            let cat = categorize(Some(&self.add_filename), None);
            self.add_category = cat.name().to_string();
            self.add_save_dir = cat.default_dir().to_string_lossy().to_string();
        }
        self.show_add_dialog = true;

        // Async probe for content-length and headers (only for direct files, streaming sites have dynamic adaptive streams)
        if !rustget_core::youtube::is_streaming_site(&self.add_url) {
            let (tx, rx) = mpsc::channel(1);
            self.size_probe_rx = Some(rx);
            if let Ok(parsed_url) = url::Url::parse(&self.add_url) {
                let cookies_clone = self.add_cookies.clone();
                let referer_clone = self.add_referer.clone();
                tokio::spawn(async move {
                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(3))
                        .build();
                    if let Ok(c) = client {
                        let mut headers = reqwest::header::HeaderMap::new();
                        if let Some(ref r) = referer_clone {
                            if let Ok(hv) = reqwest::header::HeaderValue::from_str(r) {
                                headers.insert(reqwest::header::REFERER, hv);
                            }
                        }
                        if let Some(ref ck) = cookies_clone {
                            if let Ok(hv) = reqwest::header::HeaderValue::from_str(ck) {
                                headers.insert(reqwest::header::COOKIE, hv);
                            }
                        }
                        if let Ok(info) = rustget_core::probe::probe_url(&c, &parsed_url, &headers).await {
                            let _ = tx.send((info.content_length, Some(info.suggested_filename))).await;
                        }
                    }
                });
            }
        }
    }

    fn trigger_download(&mut self, start_paused: bool) {
        let url = self.add_url.trim().to_string();
        if url.is_empty() {
            return;
        }

        self.active_popup_filename = self.add_filename.clone();
        self.active_popup_url = url.clone();

        let mut opt = DownloadOptions {
            url,
            num_segments: self.add_connections,
            expected_sha256: if self.add_checksum.trim().is_empty() {
                None
            } else {
                Some(self.add_checksum.trim().to_string())
            },
            ..Default::default()
        };

        let mut out_path = if !self.add_save_dir.trim().is_empty() {
            PathBuf::from(self.add_save_dir.trim())
        } else {
            Category::General.default_dir()
        };
        if !self.add_filename.trim().is_empty() {
            out_path.push(self.add_filename.trim());
        }
        self.active_popup_target_path = out_path.to_string_lossy().to_string();
        opt.output_path = Some(out_path);

        if let Some(c) = self.add_cookies.take() {
            opt.cookie = Some(c);
        }
        if let Some(r) = self.add_referer.take() {
            opt.referer = Some(r);
        }

        let q = Arc::clone(&self.queue);
        let speed_bps = self.add_speed_limit_kb * 1024;
        let (id_tx, id_rx) = mpsc::channel(1);
        self.pending_task_rx = Some(id_rx);

        let is_standalone = self.is_standalone_popup;
        tokio::spawn(async move {
            let res = if start_paused {
                if let Ok(id) = q.add_download(opt, None, speed_bps).await {
                    let _ = q.pause(&id).await;
                    Ok(id)
                } else {
                    Err(())
                }
            } else if is_standalone {
                q.start_download_immediately(opt, None, speed_bps).await.map_err(|_| ())
            } else {
                q.add_download(opt, None, speed_bps).await.map_err(|_| ())
            };

            if let Ok(id) = res {
                let _ = id_tx.send(id).await;
            }
        });

        self.add_url.clear();
        self.add_filename.clear();
        self.add_checksum.clear();
        self.add_description.clear();
        self.show_add_dialog = false;
    }

    fn render_download_file_info(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, is_standalone: bool) {
        // IDM-styled Download File Info dialog matching user screenshot
        ui.horizontal(|ui| {
            // Left main form (approx 400px width)
            ui.vertical(|ui| {
                ui.set_width(400.0);

                // Row 1: URL
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("URL:").strong().size(12.0));
                    ui.add_space(28.0);
                    if ui.add(egui::TextEdit::singleline(&mut self.add_url).desired_width(330.0)).changed() {
                        if let Ok(parsed) = url::Url::parse(self.add_url.trim()) {
                            let name = rustget_core::probe::extract_filename(&reqwest::header::HeaderMap::new(), &parsed);
                            self.add_filename = rustget_core::probe::sanitize_filename(&name);
                            if rustget_core::youtube::is_streaming_site(&self.add_url) {
                                if self.add_filename.is_empty() || self.add_filename.eq_ignore_ascii_case("watch") {
                                    self.add_filename = "YouTube_Video.mp4".to_string();
                                } else if !self.add_filename.contains('.') {
                                    self.add_filename.push_str(".mp4");
                                }
                                self.add_category = "Video".to_string();
                                self.add_save_dir = Category::Video.default_dir().to_string_lossy().to_string();
                            } else {
                                let cat = categorize(Some(&self.add_filename), None);
                                self.add_category = cat.name().to_string();
                                self.add_save_dir = cat.default_dir().to_string_lossy().to_string();
                            }
                        }
                    }
                });
                ui.add_space(4.0);

                // Row 2: Category Dropdown + [+] button
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Category:").strong().size(12.0));
                    ui.add_space(2.0);
                    egui::ComboBox::from_id_salt("cat_select")
                        .selected_text(&self.add_category)
                        .width(185.0)
                        .show_ui(ui, |ui| {
                            for cat in ["Compressed", "Programs", "Video", "Music", "Documents", "General"] {
                                if ui.selectable_value(&mut self.add_category, cat.to_string(), cat).clicked() {
                                    if let Some(c) = Category::from_name(cat) {
                                        self.add_save_dir = c.default_dir().to_string_lossy().to_string();
                                    }
                                }
                            }
                        });
                    if ui.button(egui::RichText::new("+").strong()).on_hover_text("Add or configure categories").clicked() {
                        self.show_settings_dialog = true;
                    }
                });
                ui.add_space(4.0);

                // Row 3: Save As (Target path + Browse)
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Save As:").strong().size(12.0));
                    ui.add_space(8.0);
                    let mut full_target_path = PathBuf::from(&self.add_save_dir);
                    full_target_path.push(&self.add_filename);
                    let mut path_display = full_target_path.to_string_lossy().to_string();
                    if ui.add(egui::TextEdit::singleline(&mut path_display).desired_width(280.0)).changed() {
                        let p = Path::new(&path_display);
                        if let Some(parent) = p.parent() {
                            self.add_save_dir = parent.to_string_lossy().to_string();
                        }
                        if let Some(fname) = p.file_name() {
                            self.add_filename = fname.to_string_lossy().to_string();
                        }
                    }
                    if ui.button(egui::RichText::new("...").strong()).on_hover_text("Choose save destination").clicked() {
                        let start_dir = if !self.add_save_dir.trim().is_empty() && Path::new(self.add_save_dir.trim()).exists() {
                            PathBuf::from(self.add_save_dir.trim())
                        } else {
                            PathBuf::from("C:\\")
                        };
                        if let Some(file_path) = rfd::FileDialog::new()
                            .set_title("Save As")
                            .set_directory(&start_dir)
                            .set_file_name(&self.add_filename)
                            .save_file()
                        {
                            if let Some(parent) = file_path.parent() {
                                self.add_save_dir = parent.to_string_lossy().to_string();
                            }
                            if let Some(fname) = file_path.file_name() {
                                self.add_filename = fname.to_string_lossy().to_string();
                            }
                        }
                    }
                });
                ui.add_space(4.0);

                // Row 4: Remember this path checkbox
                let remember_label = format!("Remember this path for \"{}\" category", self.add_category);
                ui.checkbox(&mut self.add_remember_category, egui::RichText::new(remember_label).size(11.0).color(Color32::LIGHT_GRAY));
                ui.add_space(2.0);

                // Row 5: Path preview frame
                let frame = egui::Frame::none().fill(Color32::from_rgb(26, 28, 48)).inner_margin(Margin::symmetric(6.0, 4.0)).rounding(Rounding::same(4.0));
                frame.show(ui, |ui| {
                    ui.set_width(380.0);
                    let mut dir_display = self.add_save_dir.clone();
                    if !dir_display.ends_with('\\') && !dir_display.ends_with('/') {
                        dir_display.push('\\');
                    }
                    ui.label(egui::RichText::new(dir_display).size(11.0).color(Color32::from_rgb(140, 150, 180)));
                });
                ui.add_space(4.0);

                // Row 6: Description
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Description:").size(11.0).color(Color32::LIGHT_GRAY));
                    ui.add(egui::TextEdit::singleline(&mut self.add_description).desired_width(290.0));
                });
            });

            ui.add_space(10.0);

            // Right side: Icon & File Size preview
            ui.vertical(|ui| {
                ui.set_width(110.0);
                ui.add_space(10.0);

                let cat_icon = match self.add_category.as_str() {
                "Video" => "▶",
                "Music" => "♫",
                "Documents" => "≡",
                "Compressed" => "◆",
                "Programs" => "⚡",
                _ => "●",
                };
                ui.label(egui::RichText::new(cat_icon).size(42.0));
                ui.add_space(4.0);

                let palette = *self.theme_mode.palette();
                if rustget_core::youtube::is_streaming_site(&self.add_url) {
                    ui.label(egui::RichText::new("Adaptive Stream").strong().size(12.0).color(palette.accent));
                } else if let Some(sz) = self.add_file_size {
                    ui.label(egui::RichText::new(SpeedTracker::format_bytes(sz)).strong().size(13.0).color(palette.accent));
                } else {
                    ui.label(egui::RichText::new("Probing size...").size(11.0).color(palette.muted));
                }

                ui.add_space(8.0);
                let preview_btn = egui::Button::new(
                    egui::RichText::new("Preview").size(11.0)
                ).min_size(Vec2::new(76.0, 24.0));
                ui.add_enabled(false, preview_btn).on_disabled_hover_text("Preview available during or after download");
            });
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        let palette = *self.theme_mode.palette();
        // Action Buttons Row (Download Later, Start Download, Cancel)
        ui.horizontal(|ui| {
            if secondary_button(ui, self.theme_mode, &palette, "Download Later", Vec2::new(120.0, 32.0), true).clicked() {
                self.trigger_download(true);
                self.popup_opened_at = None;
                self.is_popup_mode = false;
                self.is_progress_popup_mode = false;
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                self.active_popup_task_id = None;
                if is_standalone {
                    remove_popup_topmost();
                    restore_main_viewport(ctx, false);
                }
            }

            ui.add_space(8.0);

            if primary_button(ui, &palette, "Start Download", Vec2::new(140.0, 32.0), true).clicked() {
                self.trigger_download(false);
                self.is_popup_mode = false;
                self.popup_opened_at = None;
                self.is_progress_popup_mode = true;
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                open_popup_viewport(ctx, Vec2::new(520.0, 250.0), "Downloading - Cosmo Downloader");
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if secondary_button(ui, self.theme_mode, &palette, "Cancel", Vec2::new(80.0, 32.0), true).clicked() {
                    self.show_add_dialog = false;
                    self.add_url.clear();
                    self.add_filename.clear();
                    self.popup_opened_at = None;
                    self.is_popup_mode = false;
                    self.is_progress_popup_mode = false;
                    self.is_completed_popup_mode = false;
                    self.completed_dialog_record = None;
                    self.active_popup_task_id = None;
                    if is_standalone {
                        remove_popup_topmost();
                        restore_main_viewport(ctx, false);
                    }
                }
            });
        });
    }

    fn render_download_progress(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, is_standalone: bool) {
        let task_id = self.active_popup_task_id.clone();
        let fname = if !self.active_popup_filename.is_empty() {
            self.active_popup_filename.clone()
        } else {
            "Downloading file...".to_string()
        };

        let prog = task_id.as_ref().and_then(|id| self.active_progress.get(id));
        let (downloaded, total, speed, eta, ratio) = if let Some(p) = prog {
            let total = p.total_bytes.unwrap_or(0);
            let r = if total > 0 {
                (p.downloaded_bytes as f32 / total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (
                SpeedTracker::format_bytes(p.downloaded_bytes),
                if total > 0 { SpeedTracker::format_bytes(total) } else { "Unknown".to_string() },
                SpeedTracker::format_speed(p.speed_bps),
                SpeedTracker::format_eta(p.eta_seconds),
                r,
            )
        } else {
            ("0 B".to_string(), "Connecting...".to_string(), "0 B/s".to_string(), "--:--".to_string(), 0.0)
        };

        let palette = *self.theme_mode.palette();
        // Header: Icon + Filename + Speed/ETA
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("↓").size(32.0).color(palette.accent));
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(&fname).strong().size(13.5).color(palette.text));
                ui.add_space(2.0);
                ui.label(egui::RichText::new(format!("Transfer rate: {}  |  Time left: {}", speed, eta)).size(11.0).color(palette.accent));
            });
        });

        ui.add_space(10.0);

        // Progress Bar
        let pct_text = format!("{:.1}%  ({} / {})", ratio * 100.0, downloaded, total);
        ui.add(
            egui::ProgressBar::new(ratio)
                .show_percentage()
                .text(pct_text)
                .animate(true)
        );

        ui.add_space(12.0);

        // Details grid: Status, File path
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Status:").size(11.0).color(palette.muted));
            let status_text = if ratio >= 1.0 { "Finishing download..." } else if ratio > 0.0 { "Downloading..." } else { "Connecting to server..." };
            ui.label(egui::RichText::new(status_text).size(11.0).color(palette.text));
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Save path:").size(11.0).color(Color32::LIGHT_GRAY));
            ui.label(egui::RichText::new(&self.active_popup_target_path).size(11.0).color(Color32::from_rgb(140, 150, 180)));
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(10.0);

        // Controls Row: Pause/Resume, Cancel, Close
        ui.horizontal(|ui| {
            let q = Arc::clone(&self.queue);

            let is_paused = task_id.as_ref().map_or(false, |id| {
                self.downloads.iter().any(|d| &d.id == id && d.status == "paused")
            });

            let pause_label = if is_paused { "▶ Resume" } else { "⏸ Pause" };
            if ui.add(egui::Button::new(pause_label).min_size(Vec2::new(95.0, 30.0))).clicked() {
                if let Some(ref id) = task_id {
                    let id_c = id.clone();
                    let q_c = Arc::clone(&q);
                    tokio::spawn(async move {
                        if is_paused {
                            let _ = q_c.resume(&id_c).await;
                        } else {
                            let _ = q_c.pause(&id_c).await;
                        }
                    });
                }
            }

            ui.add_space(8.0);

            if ui.add(egui::Button::new("Cancel").min_size(Vec2::new(80.0, 30.0))).clicked() {
                if let Some(ref id) = task_id {
                    let id_c = id.clone();
                    let q_c = Arc::clone(&q);
                    tokio::spawn(async move {
                        let _ = q_c.remove(&id_c, false).await;
                    });
                }
                self.is_progress_popup_mode = false;
                self.active_popup_task_id = None;
                if is_standalone {
                    remove_popup_topmost();
                    restore_main_viewport(ctx, false);
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Close button: hides progress dialog while download continues in background
                if primary_button(ui, &palette, "Close", Vec2::new(90.0, 30.0), true).clicked() {
                    self.is_progress_popup_mode = false;
                    if is_standalone {
                        remove_popup_topmost();
                        restore_main_viewport(ctx, false);
                    }
                }
            });
        });
    }

    fn render_download_complete(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, is_standalone: bool) {
        if let Some(record) = self.completed_dialog_record.clone() {
            let (sz_formatted, bytes_num) = if let Some(sz) = record.total_size {
                (SpeedTracker::format_bytes(sz), sz)
            } else if let Ok(meta) = std::fs::metadata(&record.target_path) {
                let len = meta.len();
                (SpeedTracker::format_bytes(len), len)
            } else {
                ("Unknown".to_string(), 0)
            };

            let palette = *self.theme_mode.palette();
            // Top section: Folder Icon + "Download complete" + size in bytes
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("✓").size(36.0).color(theme::EMERALD_SUCCESS));
                ui.add_space(8.0);

                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Download complete")
                            .strong()
                            .size(13.0)
                            .color(palette.text),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(format!("Downloaded {} ({} Bytes)", sz_formatted, bytes_num))
                            .size(11.5)
                            .color(palette.muted),
                    );
                });
            });

            ui.add_space(8.0);

            // "Address" label + Full width textedit
            ui.label(egui::RichText::new("Address").size(11.5).color(palette.text));
            ui.add_space(2.0);
            let mut url_disp = record.url.clone();
            ui.add(
                egui::TextEdit::singleline(&mut url_disp)
                    .desired_width(ui.available_width())
            );

            ui.add_space(6.0);

            // "The file saved as" label with quick Copy Path button
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("The file saved as").size(11.5).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Copy").size(10.5).color(palette.accent)).clicked() {
                        ui.output_mut(|o| o.copied_text = record.target_path.clone());
                    }
                });
            });
            ui.add_space(2.0);
            let mut path_disp = record.target_path.clone();
            ui.add(
                egui::TextEdit::singleline(&mut path_disp)
                    .id_source("complete_dialog_target_path")
                    .desired_width(ui.available_width())
            ).on_hover_text(format!("Full Path:\n{}", record.target_path));

            ui.add_space(12.0);

            // Row of 4 buttons: [ Open ]  [ Open with... ]  [ Open folder ]  [ Close ]
            ui.horizontal(|ui| {
                let btn_size = Vec2::new(115.0, 26.0);

                if primary_button(ui, &palette, "Open", btn_size, true).clicked() {
                    shell_open_file(&record.target_path);
                }

                ui.add_space(6.0);

                if secondary_button(ui, self.theme_mode, &palette, "Open with...", btn_size, true).clicked() {
                    shell_open_with(&record.target_path);
                }

                ui.add_space(6.0);

                if secondary_button(ui, self.theme_mode, &palette, "Open folder", btn_size, true).clicked() {
                    shell_open_folder_and_select(&record.target_path);
                }

                ui.add_space(6.0);

                if secondary_button(ui, self.theme_mode, &palette, "Close", btn_size, true).clicked() {
                    if self.is_standalone_popup {
                        if let Some(ref f) = self.standalone_popup_file {
                            let _ = std::fs::remove_file(f);
                        }
                        std::process::exit(0);
                    }
                    self.completed_dialog_record = None;
                    self.is_popup_mode = false;
                    self.is_progress_popup_mode = false;
                    self.is_completed_popup_mode = false;
                    self.active_popup_task_id = None;
                    self.popup_opened_at = None;
                    if is_standalone {
                        remove_popup_topmost();
                        restore_main_viewport(ctx, false);
                    }
                }
            });

            ui.add_space(8.0);

            // Bottom row: [ ] Don't show this dialog again
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.setting_dont_show_complete_dialog, egui::RichText::new("Don't show this dialog again").size(11.0).color(palette.muted));
            });
        }
    }
}

impl eframe::App for CosmoDownloaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.initial_minimized_handled {
            self.initial_minimized_handled = true;
            let start_minimized = std::env::args().any(|a| a == "--minimized");
            if start_minimized {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                if self.setting_minimize_to_tray && self.tray_icon.is_some() {
                    hide_main_window();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
            }
        }

        // Continuous redraw when downloads are active
        ctx.request_repaint_after(Duration::from_millis(100));

        // Poll non-blocking GitHub release update checker
        if let Some(ref mut rx) = self.update_check_rx {
            if let Ok(result) = rx.try_recv() {
                if result.is_some() {
                    self.update_available = result;
                }
                self.update_check_rx = None;
            }
        }

        // Intercept window close when minimize_to_tray is enabled and tray icon is active
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.is_progress_popup_mode {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.is_progress_popup_mode = false;
                self.active_popup_task_id = None;
                remove_popup_topmost();
                restore_main_viewport(ctx, false);
                return;
            }
            if self.is_completed_popup_mode {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                self.is_popup_mode = false;
                self.is_progress_popup_mode = false;
                self.active_popup_task_id = None;
                self.popup_opened_at = None;
                remove_popup_topmost();
                restore_main_viewport(ctx, false);
                return;
            }
            if self.is_popup_mode {
                if let Some(opened) = self.popup_opened_at {
                    if opened.elapsed() < Duration::from_millis(600) {
                        // Prevent spurious close events triggered immediately during window unhide/focus transition
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                        return;
                    }
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.is_popup_mode = false;
                self.is_progress_popup_mode = false;
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                self.active_popup_task_id = None;
                self.popup_opened_at = None;
                self.show_add_dialog = false;
                self.add_url.clear();
                self.add_filename.clear();
                remove_popup_topmost();
                restore_main_viewport(ctx, false);
                return;
            }
            // Main window [X] close: hide to background / tray without killing bridge listener
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            remove_popup_topmost();
            restore_main_viewport(ctx, false);
            return;
        }

        // Receive pending task id for progress popup
        if let Some(ref mut rx) = self.pending_task_rx {
            if let Ok(id) = rx.try_recv() {
                self.active_popup_task_id = Some(id);
            }
        }

        // Receive probed file size and suggested filename if available
        if let Some(ref mut rx) = self.size_probe_rx {
            if let Ok((sz, suggested_name)) = rx.try_recv() {
                self.add_file_size = sz;
                if let Some(name) = suggested_name {
                    let cur_name = self.add_filename.trim().to_ascii_lowercase();
                    // If current name is generic, missing extension, or probe found a better filename with extension
                    if cur_name == "download" || cur_name == "download.bin" || !self.add_filename.contains('.') || (name.contains('.') && !cur_name.contains('.')) {
                        self.add_filename = name.clone();
                        let cat = categorize(Some(&name), None);
                        self.add_category = cat.name().to_string();
                        self.add_save_dir = cat.default_dir().to_string_lossy().to_string();
                    }
                }
            }
        }

        // Process queue events
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                QueueEvent::TaskCompleted(id) => {
                    self.refresh_downloads();
                    if self.is_progress_popup_mode {
                        self.is_progress_popup_mode = false;
                    }
                    if !self.setting_dont_show_complete_dialog {
                        let record = self.downloads.iter().find(|d| d.id == id).cloned().unwrap_or_else(|| {
                            let mut resolved_target = self.active_popup_target_path.clone();
                            let p = std::path::Path::new(&resolved_target);
                            if !p.exists() {
                                if let Some(parent) = p.parent() {
                                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                                        if let Ok(entries) = std::fs::read_dir(parent) {
                                            for entry in entries.flatten() {
                                                let ep = entry.path();
                                                if ep.is_file() {
                                                    if let Some(name) = ep.file_name().and_then(|n| n.to_str()) {
                                                        if name.starts_with(stem) && !name.ends_with(".part") && !name.ends_with(".ytdl") {
                                                            resolved_target = ep.to_string_lossy().to_string();
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            let (size, bytes) = if let Ok(meta) = std::fs::metadata(&resolved_target) {
                                (Some(meta.len()), meta.len())
                            } else {
                                (None, 0)
                            };
                            let cat_str = categorize(Some(&resolved_target), None).name().to_string();
                            DownloadRecord {
                                id: id.clone(),
                                url: self.active_popup_url.clone(),
                                filename: std::path::Path::new(&resolved_target).file_name().and_then(|s| s.to_str()).unwrap_or(&self.active_popup_filename).to_string(),
                                target_path: resolved_target,
                                part_path: String::new(),
                                category: cat_str,
                                total_size: size,
                                downloaded_bytes: bytes,
                                status: "completed".to_string(),
                                num_segments: 8,
                                expected_sha256: None,
                                custom_headers: None,
                                user_agent: None,
                                referer: None,
                                cookie: None,
                                max_speed_bps: 0,
                                retry_count: 0,
                                max_retries: 3,
                                error_message: None,
                                created_at: chrono::Utc::now().to_rfc3339(),
                                updated_at: chrono::Utc::now().to_rfc3339(),
                            }
                        });
                        self.completed_dialog_record = Some(record);
                        self.is_completed_popup_mode = true;
                        open_popup_viewport(ctx, Vec2::new(560.0, 260.0), "Download complete");
                    }
                }
                QueueEvent::TaskAdded(_) | QueueEvent::TaskFailed { .. } => {
                    self.refresh_downloads();
                }
                QueueEvent::StatusChanged { id, status } => {
                    if status == "completed" || status == "paused" || status == "failed" {
                        self.active_progress.remove(&id);
                        self.ema_speeds.remove(&id);
                    }
                    self.refresh_downloads();
                }
                QueueEvent::Progress { id, progress } => {
                    let current_speed = progress.speed_bps;
                    let smoothed = if let Some(&prev) = self.ema_speeds.get(&id) {
                        0.3 * current_speed + 0.7 * prev
                    } else {
                        current_speed
                    };
                    self.ema_speeds.insert(id.clone(), smoothed);
                    self.active_progress.insert(id, progress);
                }
            }
        }

        // Process external show requests (from second instance launch or HTTP /show)
        while let Ok(_) = self.show_rx.try_recv() {
            if self.is_popup_mode || self.is_progress_popup_mode || self.is_completed_popup_mode {
                // If any download popup or progress popup is active, NEVER destroy it or resize to main size!
                continue;
            }
            self.is_popup_mode = false;
            self.is_progress_popup_mode = false;
            self.is_completed_popup_mode = false;
            remove_popup_topmost();
            restore_main_viewport(ctx, true);
        }

        // Process browser extension download requests (IDM popup dialog - compact standalone)
        while let Ok(req) = self.browser_dl_rx.try_recv() {
            self.prepare_download_dialog(&req.url, req.filename, req.cookies, req.referer, req.num_segments);
            self.is_popup_mode = true;
            self.popup_opened_at = Some(std::time::Instant::now());
            self.is_progress_popup_mode = false;
            self.is_completed_popup_mode = false;
            self.completed_dialog_record = None;
            self.active_popup_task_id = None;
            self.show_add_dialog = true;
            open_popup_viewport(ctx, Vec2::new(560.0, 310.0), "Download File Info");
        }

        // Process clipboard events
        while let Ok(url) = self.clipboard_rx.try_recv() {
            if self.setting_clipboard_monitor {
                self.prepare_download_dialog(&url, None, None, None, None);
                self.is_popup_mode = true;
                self.popup_opened_at = Some(std::time::Instant::now());
                self.is_progress_popup_mode = false;
                self.is_completed_popup_mode = false;
                self.completed_dialog_record = None;
                self.active_popup_task_id = None;
                self.show_add_dialog = true;
                open_popup_viewport(ctx, Vec2::new(560.0, 310.0), "Download File Info");
            }
        }

        // Process tray menu clicks
        if let Some(ref rx) = self.tray_event_receiver {
            while let Ok(action) = rx.try_recv() {
                match action.as_str() {
                    "show" => {
                        self.is_popup_mode = false;
                        self.is_progress_popup_mode = false;
                        self.is_completed_popup_mode = false;
                        remove_popup_topmost();
                        restore_main_viewport(ctx, true);
                    }
                    "add" => {
                        self.is_popup_mode = false;
                        self.is_progress_popup_mode = false;
                        self.is_completed_popup_mode = false;
                        remove_popup_topmost();
                        restore_main_viewport(ctx, true);
                        self.show_add_dialog = true;
                    }
                    "exit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
        }

        // Dynamic theme synchronization
        let palette = *self.theme_mode.palette();
        let mut visuals = if self.theme_mode == theme::ThemeMode::Dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.window_rounding = Rounding::same(10.0);
        visuals.window_fill = palette.surface;
        visuals.panel_fill = palette.surface;
        visuals.override_text_color = Some(palette.text);
        visuals.widgets.noninteractive.bg_fill = palette.surface;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, palette.text);
        visuals.widgets.inactive.bg_fill = palette.raise;
        visuals.widgets.inactive.weak_bg_fill = palette.raise;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, palette.line);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, palette.text);
        visuals.widgets.hovered.bg_fill = theme::hover_raise(self.theme_mode);
        visuals.widgets.hovered.weak_bg_fill = theme::hover_raise(self.theme_mode);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, palette.line);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, palette.text);
        visuals.widgets.active.bg_fill = palette.accent_soft;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, palette.accent);
        ctx.set_visuals(visuals);

        // Standalone Compact Progress Popup Mode
        if self.is_progress_popup_mode {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(palette.surface).inner_margin(Margin::same(16.0)))
                .show(ctx, |ui| {
                    self.render_download_progress(ui, ctx, true);
                });
            return;
        }

        // Standalone Compact Completion Popup Mode
        if self.is_completed_popup_mode && self.completed_dialog_record.is_some() {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(palette.surface).inner_margin(Margin::same(16.0)))
                .show(ctx, |ui| {
                    self.render_download_complete(ui, ctx, true);
                });
            return;
        }

        // Standalone Compact Popup Mode (renders ONLY the Download File Info interface)
        if self.is_popup_mode {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(palette.surface).inner_margin(Margin::same(16.0)))
                .show(ctx, |ui| {
                    self.render_download_file_info(ui, ctx, true);
                });
            return;
        }

        // Top Titlebar & Menu & Toolbar
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::none().fill(palette.surface))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;

                // 1. Custom Frameless Window Title Bar (36px high, 12px left padding)
                let (titlebar_rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 36.0),
                    egui::Sense::hover(),
                );
                let win_btn_w = 46.0;
                let controls_total_w = win_btn_w * 3.0;
                let close_rect = Rect::from_min_size(
                    pos2(titlebar_rect.right() - win_btn_w, titlebar_rect.top()),
                    Vec2::new(win_btn_w, 36.0),
                );
                let max_rect = Rect::from_min_size(
                    pos2(titlebar_rect.right() - win_btn_w * 2.0, titlebar_rect.top()),
                    Vec2::new(win_btn_w, 36.0),
                );
                let min_rect = Rect::from_min_size(
                    pos2(titlebar_rect.right() - controls_total_w, titlebar_rect.top()),
                    Vec2::new(win_btn_w, 36.0),
                );
                let drag_rect = Rect::from_min_max(
                    titlebar_rect.min,
                    pos2(min_rect.left(), titlebar_rect.bottom()),
                );
                let titlebar_drag = ui.interact(drag_rect, ui.id().with("win_title_drag"), egui::Sense::click_and_drag());

                // Icon (icon.ico texture) + Title
                let icon_rect = Rect::from_center_size(
                    pos2(titlebar_rect.left() + 12.0 + 9.0, titlebar_rect.center().y),
                    Vec2::splat(18.0),
                );
                ui.painter().image(
                    self.logo_texture.id(),
                    icon_rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                let title_str = format!("Cosmo Downloader v{}", env!("CARGO_PKG_VERSION"));
                let title_galley = ui.painter().layout_no_wrap(
                    title_str,
                    egui::FontId::proportional(12.5),
                    palette.text,
                );
                ui.painter().galley(
                    pos2(icon_rect.right() + 8.0, titlebar_rect.center().y - title_galley.size().y / 2.0),
                    title_galley,
                    palette.text,
                );

                // Window Control Buttons (no-drag interactive zones)
                let min_resp = ui.interact(min_rect, ui.id().with("win_min_btn"), egui::Sense::click());
                let max_resp = ui.interact(max_rect, ui.id().with("win_max_btn"), egui::Sense::click());
                let close_resp = ui.interact(close_rect, ui.id().with("win_close_btn"), egui::Sense::click());

                if min_resp.hovered() {
                    ui.painter().rect_filled(min_rect, Rounding::ZERO, palette.raise);
                }
                icons::draw_win_min(ui.painter(), min_rect.center(), palette.muted);
                if min_resp.clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }

                let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                if max_resp.hovered() {
                    ui.painter().rect_filled(max_rect, Rounding::ZERO, palette.raise);
                }
                icons::draw_win_max(ui.painter(), max_rect.center(), palette.muted, is_maximized);
                if max_resp.clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                }

                if close_resp.hovered() {
                    ui.painter().rect_filled(close_rect, Rounding::ZERO, palette.danger);
                }
                icons::draw_win_close(
                    ui.painter(),
                    close_rect.center(),
                    if close_resp.hovered() { Color32::WHITE } else { palette.muted },
                );
                if close_resp.clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                // Drag & double-click maximize on title bar outside control buttons
                if titlebar_drag.double_clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                } else if titlebar_drag.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                // Hairline separator below title bar
                let sep1_y = titlebar_rect.bottom();
                ui.painter().line_segment(
                    [pos2(titlebar_rect.left(), sep1_y), pos2(titlebar_rect.right(), sep1_y)],
                    Stroke::new(1.0_f32, palette.line),
                );

                // 2. Menu bar: height 36px, horizontal padding 8px
                egui::Frame::none()
                    .inner_margin(Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        ui.set_min_height(28.0);
                        ui.spacing_mut().item_spacing.x = 2.0;
                        ui.horizontal_centered(|ui| {
                            ui.scope(|ui| {
                                ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                                ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
                                ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
                                ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(1.0_f32, palette.muted);
                                ui.visuals_mut().widgets.hovered.weak_bg_fill = palette.raise;
                                ui.visuals_mut().widgets.hovered.bg_fill = palette.raise;
                                ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::NONE;
                                ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(1.0_f32, palette.text);
                                ui.visuals_mut().widgets.open.weak_bg_fill = palette.raise;
                                ui.visuals_mut().widgets.open.bg_fill = palette.raise;
                                ui.visuals_mut().widgets.open.fg_stroke = Stroke::new(1.0_f32, palette.text);

                                ui.menu_button(egui::RichText::new("File").size(12.5), |ui| {
                                    if ui.button("Add Download URL...").clicked() {
                                        self.show_add_dialog = true;
                                        ui.close_menu();
                                    }
                                    ui.separator();
                                    if ui.button("Exit").clicked() {
                                        std::process::exit(0);
                                    }
                                });

                                ui.menu_button(egui::RichText::new("Downloads").size(12.5), |ui| {
                                    if ui.button("Pause All").clicked() {
                                        let q = Arc::clone(&self.queue);
                                        let ids: Vec<String> = self.downloads.iter()
                                            .filter(|d| d.status == "downloading" || d.status == "queued")
                                            .map(|d| d.id.clone())
                                            .collect();
                                        tokio::spawn(async move {
                                            for id in ids { let _ = q.pause(&id).await; }
                                        });
                                        ui.close_menu();
                                    }
                                    if ui.button("Resume All").clicked() {
                                        let q = Arc::clone(&self.queue);
                                        let ids: Vec<String> = self.downloads.iter()
                                            .filter(|d| d.status == "paused" || d.status == "failed")
                                            .map(|d| d.id.clone())
                                            .collect();
                                        tokio::spawn(async move {
                                            for id in ids { let _ = q.resume(&id).await; }
                                        });
                                        ui.close_menu();
                                    }
                                });

                                ui.menu_button(egui::RichText::new("View").size(12.5), |ui| {
                                    let label = if self.theme_mode == theme::ThemeMode::Dark {
                                        "Switch to Light mode"
                                    } else {
                                        "Switch to Dark mode"
                                    };
                                    if ui.button(label).clicked() {
                                        self.theme_mode = self.theme_mode.toggle();
                                        let q = Arc::clone(&self.queue);
                                        let t_str = self.theme_mode.label();
                                        tokio::spawn(async move {
                                            let _ = q.db().set_setting("ui_theme_mode", t_str).await;
                                        });
                                        ui.close_menu();
                                    }
                                });

                                ui.menu_button(egui::RichText::new("Tools").size(12.5), |ui| {
                                    if ui.button("Settings...").clicked() {
                                        self.show_settings_dialog = true;
                                        ui.close_menu();
                                    }
                                });

                                ui.menu_button(egui::RichText::new("Help").size(12.5), |ui| {
                                    if ui.button("Check for updates").clicked() {
                                        self.update_check_rx = Some(spawn_update_check(self.queue.db().clone(), ctx.clone()));
                                        ui.close_menu();
                                    }
                                    if ui.button("About Cosmo Downloader").clicked() {
                                        self.show_about_dialog = true;
                                        ui.close_menu();
                                    }
                                });
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let theme_label = if self.theme_mode == theme::ThemeMode::Dark {
                                    "☀ Light mode"
                                } else {
                                    "☾ Dark mode"
                                };
                                if secondary_button(ui, self.theme_mode, &palette, theme_label, Vec2::new(96.0, 26.0), true).clicked() {
                                    self.theme_mode = self.theme_mode.toggle();
                                    let q = Arc::clone(&self.queue);
                                    let t_str = self.theme_mode.label();
                                    tokio::spawn(async move {
                                        let _ = q.db().set_setting("ui_theme_mode", t_str).await;
                                    });
                                }
                            });
                        });
                    });

                // Hairline separator below menu bar
                let menu_bottom = ui.min_rect().bottom();
                ui.painter().line_segment(
                    [pos2(titlebar_rect.left(), menu_bottom), pos2(titlebar_rect.right(), menu_bottom)],
                    Stroke::new(1.0_f32, palette.line),
                );

                // 3. Toolbar: padding 8px 12px
                egui::Frame::none()
                    .inner_margin(Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.horizontal(|ui| {
                            let btn_size = Vec2::new(82.0, 30.0);

                            if secondary_button(ui, self.theme_mode, &palette, "+ Add URL", btn_size, true).clicked() {
                                self.show_add_dialog = true;
                            }

                            ui.add_space(2.0);
                            ui.separator();
                            ui.add_space(2.0);

                            if secondary_button(ui, self.theme_mode, &palette, "⏸ Pause All", btn_size, true).clicked() {
                                let q = Arc::clone(&self.queue);
                                let ids: Vec<String> = self.downloads.iter()
                                    .filter(|d| d.status == "downloading" || d.status == "queued")
                                    .map(|d| d.id.clone())
                                    .collect();
                                tokio::spawn(async move {
                                    for id in ids {
                                        let _ = q.pause(&id).await;
                                    }
                                });
                            }

                            if secondary_button(ui, self.theme_mode, &palette, "▶ Resume All", Vec2::new(94.0, 30.0), true).clicked() {
                                let q = Arc::clone(&self.queue);
                                let ids: Vec<String> = self.downloads.iter()
                                    .filter(|d| d.status == "paused" || d.status == "failed")
                                    .map(|d| d.id.clone())
                                    .collect();
                                tokio::spawn(async move {
                                    for id in ids {
                                        let _ = q.resume(&id).await;
                                    }
                                });
                            }

                            let has_selection = !self.selected_row_ids.is_empty();
                            if secondary_button(ui, self.theme_mode, &palette, "🗑 Delete", btn_size, has_selection).clicked() {
                                let q = Arc::clone(&self.queue);
                                let ids: Vec<String> = self.selected_row_ids.iter().cloned().collect();
                                tokio::spawn(async move {
                                    for id in ids {
                                        let _ = q.remove(&id, false).await;
                                    }
                                });
                                self.selected_row_ids.clear();
                            }

                            ui.add_space(2.0);
                            ui.separator();
                            ui.add_space(2.0);

                            if secondary_button(ui, self.theme_mode, &palette, "⚙ Settings", btn_size, true).clicked() {
                                self.show_settings_dialog = true;
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let total_speed: f64 = self.active_progress.values().map(|p| p.speed_bps).sum();
                                if total_speed > 0.0 {
                                    ui.label(
                                        egui::RichText::new(format!("Speed: {}", SpeedTracker::format_speed(total_speed)))
                                            .color(palette.accent)
                                            .strong()
                                            .size(11.5),
                                    );
                                }
                            });
                        });
                    });

                let tb_bottom = ui.min_rect().bottom();
                ui.painter().line_segment(
                    [pos2(titlebar_rect.left(), tb_bottom), pos2(titlebar_rect.right(), tb_bottom)],
                    Stroke::new(1.0_f32, palette.line),
                );
            });

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("bottom_bar")
            .frame(egui::Frame::none().fill(palette.raise).stroke(Stroke::new(1.0_f32, palette.line)).inner_margin(Margin::symmetric(14.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let total_speed: f64 = self.active_progress.values().map(|p| p.speed_bps).sum();
                    let raw_mb = total_speed / (1024.0 * 1024.0);
                    let speed_mb = if raw_mb > 0.0 { raw_mb } else { 0.0 };
                    let count = self.downloads.len();
                    let item_word = if count == 1 { "item" } else { "items" };
                    ui.label(
                        egui::RichText::new(format!("{} {} · {:.1} MB/s · Warp Engine active", count, item_word, speed_mb))
                            .size(11.5)
                            .color(palette.muted),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("Cosmo Downloader v{} · © 2026 Mahedi", env!("CARGO_PKG_VERSION")))
                                .size(11.5)
                                .color(palette.muted),
                        );

                        if let Some((ref new_ver, ref rel_url)) = self.update_available.clone() {
                            ui.add_space(12.0);
                            if ui.add(
                                egui::Button::new(
                                    egui::RichText::new("View release")
                                        .size(11.0)
                                        .color(palette.accent),
                                )
                                .frame(false),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                            {
                                open_url_in_browser(rel_url);
                            }
                            ui.label(
                                egui::RichText::new(format!("Update available v{}", new_ver))
                                    .size(11.5)
                                    .color(palette.accent),
                            );
                        }
                    });
                });
            });

        let prev_tab = self.selected_tab;
        let prev_cat = self.selected_category;

        // Left Navigation Bar (Categories & Filters)
        egui::SidePanel::left("side_panel")
            .resizable(false)
            .default_width(204.0)
            .frame(egui::Frame::none().fill(palette.bg).stroke(Stroke::new(1.0_f32, palette.line)).inner_margin(Margin::symmetric(10.0, 10.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;

                    ui.label(egui::RichText::new("Status").size(11.5).color(palette.muted).strong());
                    ui.add_space(4.0);

                    sidebar_row(ui, &mut self.selected_tab, FilterTab::All, "All Downloads", self.counts.status_all, &palette, icons::draw_all);
                    sidebar_row(ui, &mut self.selected_tab, FilterTab::Downloading, "Downloading", self.counts.status_downloading, &palette, icons::draw_play);
                    sidebar_row(ui, &mut self.selected_tab, FilterTab::Completed, "Completed", self.counts.status_completed, &palette, icons::draw_folder);
                    sidebar_row(ui, &mut self.selected_tab, FilterTab::Paused, "Paused", self.counts.status_paused, &palette, icons::draw_pause);

                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("Categories").size(11.5).color(palette.muted).strong());
                    ui.add_space(4.0);

                    sidebar_row(ui, &mut self.selected_category, CategoryFilter::All, "All Categories", self.counts.cat_all, &palette, icons::draw_all);
                    sidebar_row(ui, &mut self.selected_category, CategoryFilter::Video, "Video", self.counts.cat_video, &palette, icons::draw_video);
                    sidebar_row(ui, &mut self.selected_category, CategoryFilter::Music, "Music", self.counts.cat_music, &palette, icons::draw_music);
                    sidebar_row(ui, &mut self.selected_category, CategoryFilter::Documents, "Documents", self.counts.cat_documents, &palette, icons::draw_documents);
                    sidebar_row(ui, &mut self.selected_category, CategoryFilter::Compressed, "Compressed", self.counts.cat_compressed, &palette, icons::draw_compressed);
                    sidebar_row(ui, &mut self.selected_category, CategoryFilter::Programs, "Programs", self.counts.cat_programs, &palette, icons::draw_programs);
                    sidebar_row(ui, &mut self.selected_category, CategoryFilter::Other, "Other", self.counts.cat_other, &palette, icons::draw_other);
                });
            });

        // Persist tab & category when changed
        if self.selected_tab != prev_tab {
            let tab_str = match self.selected_tab {
                FilterTab::All => "all",
                FilterTab::Downloading => "downloading",
                FilterTab::Completed => "completed",
                FilterTab::Paused => "paused",
            };
            let q = Arc::clone(&self.queue);
            tokio::spawn(async move {
                let _ = q.db().set_setting("ui_selected_tab", tab_str).await;
            });
        }
        if self.selected_category != prev_cat {
            let cat_str = match self.selected_category {
                CategoryFilter::All => "all",
                CategoryFilter::Video => "video",
                CategoryFilter::Music => "music",
                CategoryFilter::Documents => "documents",
                CategoryFilter::Compressed => "compressed",
                CategoryFilter::Programs => "programs",
                CategoryFilter::Other => "other",
            };
            let q = Arc::clone(&self.queue);
            tokio::spawn(async move {
                let _ = q.db().set_setting("ui_selected_category", cat_str).await;
            });
        }

        // Main Content Area (Columnar Download Table)
        egui::CentralPanel::default().show(ctx, |ui| {
            let filtered_downloads: Vec<&DownloadRecord> = self.downloads.iter().filter(|d| {
                let status_match = match self.selected_tab {
                    FilterTab::All => true,
                    FilterTab::Downloading => d.status == "downloading" || d.status == "queued",
                    FilterTab::Completed => d.status == "completed",
                    FilterTab::Paused => d.status == "paused",
                };
                let cat_match = self.selected_category.matches(&d.category);
                status_match && cat_match
            }).collect();

            if filtered_downloads.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(36.0, 36.0), egui::Sense::hover());
                    icons::draw_down(ui.painter(), icon_rect, palette.muted);
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new("No downloads in this view")
                            .size(16.0)
                            .color(palette.text)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Downloads captured from browser extensions or clipboard appear here.")
                            .size(12.0)
                            .color(palette.muted),
                    );
                    ui.add_space(16.0);
                    if primary_button(ui, &palette, "+ Add Download URL", Vec2::new(168.0, 34.0), true).clicked() {
                        self.show_add_dialog = true;
                    }
                });
            } else {
                // Table Header
                let row_width = ui.available_width();
                let col_check_w = 36.0;
                let col_size_w = 85.0;
                let col_prog_w = 160.0;
                let col_speed_w = 95.0;
                let col_status_w = 100.0;
                let col_name_w = (row_width - (col_check_w + col_size_w + col_prog_w + col_speed_w + col_status_w + 32.0)).max(120.0);

                let all_selected = !filtered_downloads.is_empty() && filtered_downloads.iter().all(|d| self.selected_row_ids.contains(&d.id));
                let mut header_check = all_selected;

                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    if ui.add_sized(Vec2::new(col_check_w, 24.0), egui::Checkbox::without_text(&mut header_check)).changed() {
                        if header_check {
                            for d in &filtered_downloads {
                                self.selected_row_ids.insert(d.id.clone());
                            }
                        } else {
                            self.selected_row_ids.clear();
                        }
                    }

                    ui.add_sized(Vec2::new(col_name_w, 24.0), egui::Label::new(egui::RichText::new("File name").size(12.0).color(palette.muted)));
                    ui.add_sized(Vec2::new(col_size_w, 24.0), egui::Label::new(egui::RichText::new("Size").size(12.0).color(palette.muted)));
                    ui.add_sized(Vec2::new(col_prog_w, 24.0), egui::Label::new(egui::RichText::new("Progress").size(12.0).color(palette.muted)));
                    ui.add_sized(Vec2::new(col_speed_w, 24.0), egui::Label::new(egui::RichText::new("Speed").size(12.0).color(palette.muted)));
                    ui.add_sized(Vec2::new(col_status_w, 24.0), egui::Label::new(egui::RichText::new("Status").size(12.0).color(palette.muted)));
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for record in filtered_downloads {
                        let is_sel = self.selected_row_ids.contains(&record.id);
                        let live_prog = self.active_progress.get(&record.id);
                        let downloaded = live_prog.map(|p| p.downloaded_bytes).unwrap_or(record.downloaded_bytes);
                        let total = live_prog.and_then(|p| p.total_bytes).or(record.total_size);

                        let pct = match total {
                            Some(t) if t > 0 => ((downloaded as f64) / (t as f64) * 100.0).clamp(0.0, 100.0),
                            _ => if record.status == "completed" { 100.0 } else { 0.0 },
                        };

                        let row_height = 44.0;
                        let (row_rect, row_resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_height), egui::Sense::click());

                        let row_bg = if is_sel {
                            palette.accent_soft
                        } else if row_resp.hovered() {
                            palette.raise
                        } else {
                            palette.surface
                        };

                        ui.painter().rect_filled(row_rect, Rounding::ZERO, row_bg);
                        ui.painter().line_segment(
                            [pos2(row_rect.left(), row_rect.bottom()), pos2(row_rect.right(), row_rect.bottom())],
                            Stroke::new(1.0_f32, palette.line),
                        );

                        // Checkbox
                        let check_rect = Rect::from_center_size(pos2(row_rect.left() + 20.0, row_rect.center().y), Vec2::splat(16.0));
                        let mut item_checked = is_sel;
                        if ui.put(check_rect, egui::Checkbox::without_text(&mut item_checked)).changed() {
                            if item_checked {
                                self.selected_row_ids.insert(record.id.clone());
                            } else {
                                self.selected_row_ids.remove(&record.id);
                            }
                        }

                        // Icon + Filename
                        let name_start_x = row_rect.left() + col_check_w + 8.0;
                        let icon_rect = Rect::from_center_size(pos2(name_start_x + 8.0, row_rect.center().y), Vec2::splat(14.0));
                        draw_category_icon(ui.painter(), icon_rect, &record.category, palette.muted);

                        let name_rect = Rect::from_min_size(
                            pos2(name_start_x + 22.0, row_rect.center().y - 8.0),
                            Vec2::new(col_name_w - 26.0, 18.0),
                        );
                        ui.painter().text(
                            name_rect.left_top(),
                            egui::Align2::LEFT_TOP,
                            &record.filename,
                            egui::FontId::proportional(13.0),
                            palette.text,
                        );

                        // Size
                        let size_start_x = name_start_x + col_name_w + 4.0;
                        let total_str = total.map(SpeedTracker::format_bytes).unwrap_or_else(|| "—".to_string());
                        ui.painter().text(
                            pos2(size_start_x, row_rect.center().y - 8.0),
                            egui::Align2::LEFT_TOP,
                            total_str,
                            egui::FontId::proportional(12.0),
                            palette.muted,
                        );

                        // Progress: Thin bar + %
                        let prog_start_x = size_start_x + col_size_w + 4.0;
                        let bar_w = col_prog_w - 45.0;
                        let bar_rect = Rect::from_min_size(pos2(prog_start_x, row_rect.center().y - 3.0), Vec2::new(bar_w, 6.0));
                        ui.painter().rect_filled(bar_rect, Rounding::same(3.0), palette.raise);

                        let fill_w = bar_w * (pct as f32 / 100.0);
                        if fill_w > 0.0 {
                            let bar_color = if record.status == "paused" { palette.muted } else { palette.accent };
                            let fill_rect = Rect::from_min_size(bar_rect.left_top(), Vec2::new(fill_w, 6.0));
                            ui.painter().rect_filled(fill_rect, Rounding::same(3.0), bar_color);
                        }

                        let pct_str = format!("{:.0}%", pct);
                        ui.painter().text(
                            pos2(prog_start_x + bar_w + 6.0, row_rect.center().y - 7.0),
                            egui::Align2::LEFT_TOP,
                            pct_str,
                            egui::FontId::proportional(11.5),
                            palette.muted,
                        );

                        // Speed
                        let speed_start_x = prog_start_x + col_prog_w + 4.0;
                        let speed_str = if record.status == "downloading" || record.status == "queued" {
                            if let Some(prog) = live_prog {
                                let sp = self.ema_speeds.get(&record.id).copied().unwrap_or(prog.speed_bps);
                                SpeedTracker::format_speed(sp)
                            } else {
                                "—".to_string()
                            }
                        } else {
                            "—".to_string()
                        };
                        ui.painter().text(
                            pos2(speed_start_x, row_rect.center().y - 8.0),
                            egui::Align2::LEFT_TOP,
                            speed_str,
                            egui::FontId::proportional(12.0),
                            palette.muted,
                        );

                        // Status
                        let status_start_x = speed_start_x + col_speed_w + 4.0;
                        let status_display = match record.status.as_str() {
                            "downloading" => "Downloading",
                            "completed" => "Completed",
                            "paused" => "Paused",
                            "failed" => "Failed",
                            _ => "Queued",
                        };
                        ui.painter().text(
                            pos2(status_start_x, row_rect.center().y - 8.0),
                            egui::Align2::LEFT_TOP,
                            status_display,
                            egui::FontId::proportional(12.0),
                            palette.muted,
                        );
                    }
                });
            }
        });

        // Download File Info Modal Dialog (IDM Style)
        if self.show_add_dialog && !self.is_popup_mode {
            egui::Window::new("Download File Info")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(540.0);
                    self.render_download_file_info(ui, ctx, false);
                });
        }

        // Download Complete Modal Dialog (IDM Style, when main window is open)
        if self.completed_dialog_record.is_some() && !self.is_completed_popup_mode {
            egui::Window::new("Download Complete")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(520.0);
                    self.render_download_complete(ui, ctx, false);
                });
        }

        // Settings Modal Dialog
        if self.show_settings_dialog {
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(420.0);

                    ui.heading(egui::RichText::new("CosmoDownloader Preferences").size(15.0).color(palette.accent));
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Max Concurrent Downloads:");
                        ui.add(egui::Slider::new(&mut self.setting_max_concurrent, 1..=10));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Global Speed Limit (KB/s, 0=unlimited):");
                        ui.add(egui::DragValue::new(&mut self.setting_global_speed_kb));
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.checkbox(&mut self.setting_clipboard_monitor, "Clipboard Monitor (Auto-detect copied URLs)");
                    ui.checkbox(&mut self.setting_minimize_to_tray, "Minimize to System Tray on Close [X]");
                    
                    let prev_tray = self.setting_show_tray_icon;
                    ui.checkbox(&mut self.setting_show_tray_icon, "Show Icon in Windows System Tray");
                    if prev_tray != self.setting_show_tray_icon {
                        if self.setting_show_tray_icon {
                            self.tray_icon = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                build_system_tray(&self.tray_tx, ctx.clone())
                            }))
                            .ok()
                            .flatten();
                        } else {
                            self.tray_icon = None;
                        }
                    }

                    ui.checkbox(&mut self.setting_autostart, "Start CosmoDownloader on Windows Boot");
                    if self.setting_autostart {
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            ui.checkbox(&mut self.setting_start_minimized, "Start Minimized (Silent in System Tray)");
                        });
                    }

                    ui.add_space(16.0);
                    if primary_button(ui, &palette, "Save & Close", Vec2::new(120.0, 30.0), true).clicked() {
                        self.show_settings_dialog = false;

                        let q = Arc::clone(&self.queue);
                        let db = self.queue.db().clone();
                        let max_c = self.setting_max_concurrent;
                        let speed_bps = self.setting_global_speed_kb * 1024;
                        let clip = self.setting_clipboard_monitor;
                        let min_tray = self.setting_minimize_to_tray;
                        let show_tray = self.setting_show_tray_icon;
                        let autostart = self.setting_autostart;
                        let start_min = self.setting_start_minimized;

                        tokio::spawn(async move {
                            q.set_max_concurrent(max_c).await;
                            q.set_global_speed_limit(speed_bps).await;
                            let _ = db.set_setting("clipboard_monitor_enabled", if clip { "true" } else { "false" }).await;
                            let _ = db.set_setting("minimize_to_tray_enabled", if min_tray { "true" } else { "false" }).await;
                            let _ = db.set_setting("show_tray_icon_enabled", if show_tray { "true" } else { "false" }).await;
                            let _ = db.set_setting("start_minimized_enabled", if start_min { "true" } else { "false" }).await;
                        });
                        set_autostart_enabled(autostart, start_min);
                    }
                });
        }

        // About Modal Dialog
        if self.show_about_dialog {
            egui::Window::new("About Cosmo Downloader")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(360.0);
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.image((self.logo_texture.id(), Vec2::splat(36.0)));
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Cosmo Downloader v{}", env!("CARGO_PKG_VERSION")))
                                    .size(15.0)
                                    .strong()
                                    .color(palette.text),
                            );
                            ui.label(
                                egui::RichText::new("High-speed multi-segment download manager")
                                    .size(12.0)
                                    .color(palette.muted),
                            );
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("© 2026 Mahedi · Warp Engine").size(12.0).color(palette.muted));

                    if let Some((ref new_ver, ref rel_url)) = self.update_available.clone() {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Update available: v{}", new_ver))
                                    .size(12.0)
                                    .color(palette.accent),
                            );
                            if secondary_button(ui, self.theme_mode, &palette, "View release", Vec2::new(96.0, 24.0), true).clicked() {
                                open_url_in_browser(rel_url);
                            }
                        });
                    }

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if secondary_button(ui, self.theme_mode, &palette, "GitHub", Vec2::new(90.0, 28.0), true).clicked() {
                            open_url_in_browser(GITHUB_REPO_URL);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if primary_button(ui, &palette, "Close", Vec2::new(80.0, 28.0), true).clicked() {
                                self.show_about_dialog = false;
                            }
                        });
                    });
                });
        }
    }
}

fn render_download_card(
    ui: &mut egui::Ui,
    record: &DownloadRecord,
    progress_map: &HashMap<String, DownloadProgress>,
    ema_speeds: &HashMap<String, f64>,
    queue: &Arc<QueueManager>,
    confirm_delete_id: &mut Option<String>,
) {
    let is_active = record.status == "downloading" || record.status == "queued";
    let border_color = if is_active {
        Color32::from_rgba_unmultiplied(0, 240, 255, 120) // subtle cyan glow
    } else {
        theme::BORDER
    };

    let card_frame = egui::Frame::none()
        .fill(theme::BG_CARD)
        .stroke(Stroke::new(1.0_f32, border_color))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(10.0));

    card_frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // Category Icon & Filename
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), egui::Sense::hover());
            draw_category_icon(ui.painter(), icon_rect, &record.category, theme::DARK.accent);
            ui.add_space(4.0);

            ui.add(
                egui::Label::new(
                    egui::RichText::new(&record.filename)
                        .strong()
                        .size(13.5)
                        .color(theme::TEXT_PRIMARY)
                ).truncate()
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Delete button with click-to-confirm (NOT hover-confirm)
                let is_confirming = confirm_delete_id.as_deref() == Some(&record.id);
                if is_confirming {
                    let confirm_btn = egui::Button::new(
                        egui::RichText::new("Confirm?").size(11.0).color(Color32::WHITE).strong()
                    ).fill(theme::RED_ERROR).min_size(Vec2::new(65.0, 24.0));

                    if ui.add(confirm_btn).clicked() {
                        let q_id = record.id.clone();
                        let q = Arc::clone(queue);
                        tokio::spawn(async move {
                            let _ = q.remove(&q_id, false).await;
                        });
                        *confirm_delete_id = None;
                    }

                    if ui.add(egui::Button::new("Cancel").min_size(Vec2::new(50.0, 24.0))).clicked() {
                        *confirm_delete_id = None;
                    }
                } else {
                    let del_btn = egui::Button::new(
                        egui::RichText::new("✕").size(11.0).color(theme::TEXT_SECONDARY)
                    ).min_size(Vec2::new(26.0, 24.0));
                    if ui.add(del_btn).on_hover_text("Delete Download").clicked() {
                        *confirm_delete_id = Some(record.id.clone());
                    }
                }

                ui.add_space(4.0);

                // Action buttons
                if record.status == "completed" {
                    if ui.add(egui::Button::new("Folder").min_size(Vec2::new(55.0, 24.0))).on_hover_text("Show in Explorer").clicked() {
                        shell_open_folder_and_select(&record.target_path);
                    }
                    ui.add_space(2.0);
                    if ui.add(egui::Button::new("Open").min_size(Vec2::new(48.0, 24.0))).on_hover_text("Open file").clicked() {
                        shell_open_file(&record.target_path);
                    }
                } else if record.status == "downloading" || record.status == "queued" {
                    let q_id_pause = record.id.clone();
                    let q_pause = Arc::clone(queue);
                    if ui.add(egui::Button::new("Pause").min_size(Vec2::new(52.0, 24.0))).on_hover_text("Pause").clicked() {
                        tokio::spawn(async move {
                            let _ = q_pause.pause(&q_id_pause).await;
                        });
                    }
                } else if record.status == "paused" || record.status == "failed" {
                    let q_id_res = record.id.clone();
                    let q_res = Arc::clone(queue);
                    if ui.add(egui::Button::new("Resume").min_size(Vec2::new(60.0, 24.0))).on_hover_text("Resume").clicked() {
                        tokio::spawn(async move {
                            let _ = q_res.resume(&q_id_res).await;
                        });
                    }
                }

                ui.add_space(6.0);

                // Status Badge
                let (badge_text, badge_color) = match record.status.as_str() {
                    "downloading" => ("DOWNLOADING", theme::CYAN_ACCENT),
                    "completed" => ("COMPLETED", theme::EMERALD_SUCCESS),
                    "paused" => ("PAUSED", theme::AMBER_WARNING),
                    "failed" => ("FAILED", theme::RED_ERROR),
                    _ => ("QUEUED", theme::TEXT_MUTED),
                };

                ui.label(egui::RichText::new(badge_text).size(10.5).color(badge_color).strong());
            });
        });

        ui.add_space(6.0);

        // Progress metrics & Visualizer
        let live_prog = progress_map.get(&record.id);
        let downloaded = live_prog.map(|p| p.downloaded_bytes).unwrap_or(record.downloaded_bytes);
        let total = live_prog.and_then(|p| p.total_bytes).or(record.total_size);

        let pct = match total {
            Some(t) if t > 0 => ((downloaded as f64) / (t as f64) * 100.0).clamp(0.0, 100.0),
            _ => if record.status == "completed" { 100.0 } else { 0.0 },
        };

        // Multi-segment progress visualizer bar (segmented: cyan active, emerald complete)
        if let Some(prog) = live_prog {
            if !prog.segments.is_empty() && prog.segments.len() > 1 {
                ui.horizontal(|ui| {
                    let total_width = ui.available_width();
                    let num_segs = prog.segments.len() as f32;
                    let seg_width = ((total_width - (num_segs - 1.0) * 2.0) / num_segs).max(3.0);

                    for seg in &prog.segments {
                        let seg_pct = (seg.progress_percent() / 100.0) as f32;
                        let color = if seg.complete {
                            theme::EMERALD_SUCCESS
                        } else if seg_pct > 0.0 {
                            theme::CYAN_ACCENT
                        } else {
                            theme::BORDER
                        };

                        ui.add(
                            egui::ProgressBar::new(seg_pct)
                                .desired_width(seg_width)
                                .fill(color),
                        );
                    }
                });
            } else {
                let bar_color = if record.status == "completed" {
                    theme::EMERALD_SUCCESS
                } else {
                    theme::CYAN_ACCENT
                };
                ui.add(egui::ProgressBar::new((pct / 100.0) as f32).fill(bar_color));
            }
        } else {
            let color = if record.status == "completed" {
                theme::EMERALD_SUCCESS
            } else {
                theme::CYAN_ACCENT
            };
            ui.add(egui::ProgressBar::new((pct / 100.0) as f32).fill(color));
        }

        ui.add_space(4.0);

        // Stats line: Size | Speed (EMA-smoothed) | ETA
        ui.horizontal(|ui| {
            let dl_str = SpeedTracker::format_bytes(downloaded);
            let tot_str = total.map(SpeedTracker::format_bytes).unwrap_or_else(|| "Unknown".to_string());
            ui.label(
                egui::RichText::new(format!("{:.1}%  ({} / {})", pct, dl_str, tot_str))
                    .size(11.0)
                    .color(theme::TEXT_SECONDARY),
            );

            if let Some(prog) = live_prog {
                let speed = ema_speeds.get(&record.id).copied().unwrap_or(prog.speed_bps);
                if speed > 0.0 {
                    ui.label(
                        egui::RichText::new(format!("Speed: {}", SpeedTracker::format_speed(speed)))
                            .size(11.0)
                            .color(theme::TEXT_CYAN)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(format!("ETA: {}", SpeedTracker::format_eta(prog.eta_seconds)))
                            .size(11.0)
                            .color(theme::TEXT_MUTED),
                    );
                }
            }
        });
    });
}

fn log_daemon(msg: &str) {
    if let Ok(mut dir) = std::env::current_exe() {
        dir.pop();
        let log_file = dir.join("cosmodownloader.log");
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!("[{}] {}\n", timestamp, msg);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
    }
}

fn ensure_native_host_registered() {
    #[cfg(windows)]
    std::thread::spawn(|| {
        if let Ok(mut exe) = std::env::current_exe() {
            exe.pop();
            let host_exe = exe.join("cosmodownloader-host.exe");
            let chrome_json = exe.join("com.cosmodownloader.downloader.chrome.json");
            if host_exe.exists() && chrome_json.exists() {
                let _ = std::process::Command::new(host_exe)
                    .arg("--register")
                    .arg(&chrome_json)
                    .creation_flags(CREATE_NO_WINDOW)
                    .status();
            }
        }
    });
}

fn main() -> eframe::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        log_daemon(&format!("PANIC: {:?}", info));
    }));
    tracing_subscriber::fmt().init();

    log_daemon(&format!("main() started with args: {:?}", std::env::args().collect::<Vec<_>>()));

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    // Single-instance guard so launching twice never opens two windows
    #[cfg(windows)]
    let _instance_mutex_guard: *mut std::ffi::c_void;
    #[cfg(windows)]
    unsafe {
        let mutex_handle = CreateMutexA(std::ptr::null_mut(), 1, b"CosmoDownloader_v1_0_SingleInstance_Mutex\0".as_ptr());
        if GetLastError() == 183 { // ERROR_ALREADY_EXISTS = 183
            // Check if existing instance is ACTUALLY running and listening on bridge port 42424
            if let Ok(mut stream) = std::net::TcpStream::connect("127.0.0.1:42424") {
                let mut args = std::env::args().skip(1);
                let mut popup_file = None;
                while let Some(a) = args.next() {
                    if a == "--popup-file" {
                        popup_file = args.next();
                    } else if a.starts_with("--popup-file=") {
                        popup_file = Some(a.trim_start_matches("--popup-file=").to_string());
                    }
                }

                if let Some(file_path) = popup_file {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        let req = format!(
                            "POST /download HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            content.len(),
                            content
                        );
                        let _ = std::io::Write::write_all(&mut stream, req.as_bytes());
                        let _ = std::fs::remove_file(file_path);
                    }
                } else {
                    let is_bg = std::env::args().any(|a| a == "--minimized" || a == "--daemon");
                    if !is_bg {
                        show_main_window();
                        let token = rustget_core::server::get_or_create_bridge_token();
                        let show_req = format!(
                            "POST /show HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Cosmo-Token: {}\r\nConnection: close\r\n\r\n",
                            token
                        );
                        let _ = std::io::Write::write_all(&mut stream, show_req.as_bytes());
                    }
                }
                // An active GUI instance exists, exit this duplicate safely
                std::process::exit(0);
            }
        }
        _instance_mutex_guard = mutex_handle;
    }

    #[cfg(not(windows))]
    let _unix_instance_guard = {
        if let Ok(mut stream) = std::net::TcpStream::connect("127.0.0.1:42424") {
            let token = rustget_core::server::get_or_create_bridge_token();
            let show_req = format!(
                "POST /show HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Cosmo-Token: {}\r\nConnection: close\r\n\r\n",
                token
            );
            let _ = std::io::Write::write_all(&mut stream, show_req.as_bytes());
            std::process::exit(0);
        }
    };

    let window_icon = egui::IconData {
        rgba: ICON_RGBA.to_vec(),
        width: 32,
        height: 32,
    };

    let is_popup_arg = std::env::args().any(|a| a == "--popup" || a == "--popup-file" || a.starts_with("--popup-file="));
    let start_minimized = std::env::args().any(|a| a == "--minimized" || a == "--daemon");

    let mut viewport = egui::ViewportBuilder::default()
        .with_decorations(is_popup_arg)
        .with_inner_size(if is_popup_arg { [560.0, 310.0] } else { [theme::MAIN_WIDTH, theme::MAIN_HEIGHT] })
        .with_min_inner_size(if is_popup_arg { [540.0, 300.0] } else { [760.0, 520.0] })
        .with_title(if is_popup_arg { "Download File Info" } else { APP_TITLE })
        .with_icon(Arc::new(window_icon));

    if is_popup_arg {
        viewport = viewport
            .with_visible(true)
            .with_active(true)
            .with_always_on_top();
    } else if start_minimized {
        viewport = viewport
            .with_visible(false)
            .with_active(false);
    } else {
        viewport = viewport
            .with_visible(true)
            .with_active(true);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Cosmo Downloader",
        native_options,
        Box::new(|cc| Ok(Box::new(CosmoDownloaderApp::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::{is_newer_version, parse_semver};

    #[test]
    fn semver_ordering_handles_multi_digit_minor() {
        assert_eq!(parse_semver("v1.13.0"), Some((1, 13, 0)));
        assert_eq!(parse_semver("1.9.0"), Some((1, 9, 0)));
        assert!(is_newer_version("1.12.0", "1.9.0"));
        assert!(!is_newer_version("1.9.0", "1.12.0"));
        assert!(is_newer_version("v1.14.0", "1.13.0"));
        assert!(!is_newer_version("1.13.0", "1.13.0"));
    }
}

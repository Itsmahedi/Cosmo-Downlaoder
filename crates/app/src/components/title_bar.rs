use eframe::egui::{self, pos2, Color32, Pos2, Rect, TextureHandle, Vec2};
use crate::icons;
use crate::theme::ThemeColors;

pub struct TitleBarAction {
    pub minimize: bool,
    pub toggle_maximize: bool,
    pub close: bool,
}

pub fn render_title_bar(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    pal: &ThemeColors,
    logo_tex: Option<&TextureHandle>,
    title: &str,
) -> TitleBarAction {
    let mut action = TitleBarAction {
        minimize: false,
        toggle_maximize: false,
        close: false,
    };

    let bar_rect = ui.allocate_space(Vec2::new(ui.available_width(), 36.0)).1;
    ui.painter().rect_filled(bar_rect, 0.0, pal.raise);
    ui.painter().rect_filled(
        Rect::from_min_max(Pos2::new(bar_rect.left(), bar_rect.bottom() - 1.0), bar_rect.max),
        0.0,
        pal.line,
    );

    let wc_width = 46.0 * 3.0;
    let drag_rect = Rect::from_min_max(
        bar_rect.min,
        Pos2::new((bar_rect.right() - wc_width).max(bar_rect.left()), bar_rect.bottom()),
    );
    let drag_resp = ui.interact(drag_rect, ui.id().with("title_drag"), egui::Sense::click_and_drag());
    if drag_resp.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if drag_resp.double_clicked() {
        action.toggle_maximize = true;
    }

    // Logo (18x18) + Title (12px medium)
    let mid_y = bar_rect.center().y;
    let logo_rect = Rect::from_center_size(pos2(bar_rect.left() + 12.0 + 9.0, mid_y), Vec2::splat(18.0));
    if let Some(tex) = logo_tex {
        ui.painter().image(
            tex.id(),
            logo_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        icons::draw_logo(ui.painter(), logo_rect, pal.accent);
    }

    ui.painter().text(
        pos2(bar_rect.left() + 38.0, mid_y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(12.5),
        pal.text,
    );

    // Window control buttons (46x36 each)
    let mut x = bar_rect.right() - wc_width;
    for (idx, kind) in ["min", "max", "close"].iter().enumerate() {
        let btn_rect = Rect::from_min_size(pos2(x, bar_rect.top()), Vec2::new(46.0, 36.0));
        let resp = ui.interact(btn_rect, ui.id().with(("wc_btn", idx)), egui::Sense::click());
        let hovered = resp.hovered();
        if *kind == "close" {
            if hovered {
                ui.painter().rect_filled(btn_rect, 0.0, pal.danger);
            }
            let icon_color = if hovered { Color32::WHITE } else { pal.muted };
            icons::draw_close(ui.painter(), Rect::from_center_size(btn_rect.center(), Vec2::splat(14.0)), icon_color);
            if resp.clicked() {
                action.close = true;
            }
        } else {
            if hovered {
                ui.painter().rect_filled(btn_rect, 0.0, pal.line);
            }
            let icon_color = if hovered { pal.text } else { pal.muted };
            let icon_rect = Rect::from_center_size(btn_rect.center(), Vec2::splat(14.0));
            if *kind == "min" {
                icons::draw_min(ui.painter(), icon_rect, icon_color);
                if resp.clicked() {
                    action.minimize = true;
                }
            } else {
                icons::draw_max(ui.painter(), icon_rect, icon_color);
                if resp.clicked() {
                    action.toggle_maximize = true;
                }
            }
        }
        x += 46.0;
    }

    action
}

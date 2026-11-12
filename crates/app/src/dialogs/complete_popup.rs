use eframe::egui::{self, Color32, Margin, Stroke, Vec2};
use crate::theme::{shapes, ThemeColors};
use crate::icons;
use crate::platform;
use rustget_core::{
    db::DownloadRecord,
    speed::SpeedTracker,
};

pub struct CompleteDialogOutcome {
    pub close: bool,
}

pub fn render_complete_dialog(
    ui: &mut egui::Ui,
    pal: &ThemeColors,
    record: &DownloadRecord,
) -> CompleteDialogOutcome {
    let mut outcome = CompleteDialogOutcome { close: false };

    let sz_formatted = if let Some(sz) = record.total_size {
        SpeedTracker::format_bytes(sz)
    } else if let Ok(meta) = std::fs::metadata(&record.target_path) {
        SpeedTracker::format_bytes(meta.len())
    } else {
        "Unknown".to_string()
    };

    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
        ui.painter().rect_filled(icon_rect, shapes::R_SM, pal.raise);
        icons::draw_check(ui.painter(), icon_rect.shrink(7.0), pal.accent);

        ui.add_space(shapes::S2);
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Download complete")
                    .strong()
                    .size(15.0)
                    .color(pal.text),
            );
            ui.label(
                egui::RichText::new(format!("Saved {}", sz_formatted))
                    .size(12.0)
                    .color(pal.muted),
            );
        });
    });

    ui.add_space(shapes::S3);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Saved file").size(12.0).color(pal.muted));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(egui::RichText::new("Copy path").size(11.5).color(pal.accent)).clicked() {
                ui.output_mut(|o| o.copied_text = record.target_path.clone());
            }
        });
    });

    ui.add_space(2.0);
    let mut path_disp = record.target_path.clone();
    ui.add(
        egui::TextEdit::singleline(&mut path_disp)
            .desired_width(ui.available_width())
            .margin(Margin::symmetric(10.0, 7.0)),
    );

    ui.add_space(shapes::S3);

    let div_rect = ui.allocate_space(Vec2::new(ui.available_width(), 1.0)).1;
    ui.painter().rect_filled(div_rect, 0.0, pal.line);

    ui.add_space(shapes::S3);

    ui.horizontal(|ui| {
        let open_btn = egui::Button::new(
            egui::RichText::new("Open file").size(13.0).color(pal.on_accent),
        )
        .fill(pal.accent)
        .min_size(Vec2::new(100.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(open_btn).clicked() {
            platform::shell_open_file(&record.target_path);
        }

        ui.add_space(shapes::S2);

        let folder_btn = egui::Button::new(
            egui::RichText::new("Show in folder").size(13.0).color(pal.text),
        )
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, pal.line))
        .min_size(Vec2::new(120.0, shapes::CONTROL_HEIGHT))
        .rounding(shapes::R_SM);

        if ui.add(folder_btn).clicked() {
            platform::shell_open_folder_and_select(&record.target_path);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close_btn = egui::Button::new(
                egui::RichText::new("Close").size(13.0).color(pal.muted),
            )
            .fill(Color32::TRANSPARENT)
            .min_size(Vec2::new(60.0, shapes::CONTROL_HEIGHT))
            .rounding(shapes::R_SM);

            if ui.add(close_btn).clicked() {
                outcome.close = true;
            }
        });
    });

    outcome
}

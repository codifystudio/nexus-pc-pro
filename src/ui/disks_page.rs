//! Диски: список накопителей, занятое и свободное пространство.

use crate::app::NexusApp;
use crate::system::monitor::fmt_bytes;
use crate::ui::widgets::progress_bar;

pub fn show(app: &mut NexusApp, ui: &mut egui::Ui) {
    let t = app.theme;
    let disks: Vec<(String, bool, u64, u64, String)> = app
        .sampler
        .disks()
        .iter()
        .map(|d| {
            (
                d.mount_point().display().to_string(),
                d.is_removable(),
                d.total_space(),
                d.available_space(),
                d.file_system().to_string_lossy().into_owned(),
            )
        })
        .collect();

    egui::ScrollArea::vertical().show(ui, |ui| {
        if disks.is_empty() {
            crate::theme::card_frame(&t).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Накопители не обнаружены")
                        .color(t.text_dim),
                );
            });
            return;
        }

        ui.columns(3, |cols| {
            let total: u64 = disks.iter().map(|d| d.2).sum();
            let free: u64 = disks.iter().map(|d| d.3).sum();
            crate::theme::card_frame(&t).show(&mut cols[0], |ui| {
                ui.label(egui::RichText::new("ВСЕГО").small().color(t.text_faint));
                ui.label(
                    egui::RichText::new(fmt_bytes(total))
                        .text_style(egui::TextStyle::Name("Value".into()))
                        .color(t.text),
                );
            });
            crate::theme::card_frame(&t).show(&mut cols[1], |ui| {
                ui.label(egui::RichText::new("СВОБОДНО").small().color(t.text_faint));
                ui.label(
                    egui::RichText::new(fmt_bytes(free))
                        .text_style(egui::TextStyle::Name("Value".into()))
                        .color(t.good),
                );
            });
            crate::theme::card_frame(&t).show(&mut cols[2], |ui| {
                ui.label(egui::RichText::new("ТОМОВ").small().color(t.text_faint));
                ui.label(
                    egui::RichText::new(disks.len().to_string())
                        .text_style(egui::TextStyle::Name("Value".into()))
                        .color(t.text),
                );
            });
        });

        ui.add_space(8.0);

        for (mount, removable, total, free, fs) in &disks {
            let total = *total;
            let free = *free;
            let frac = if total > 0 { 1.0 - free as f32 / total as f32 } else { 0.0 };
            crate::theme::card_frame(&t).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(mount)
                            .text_style(egui::TextStyle::Name("Title".into()))
                            .color(t.text),
                    );
                    if *removable {
                        ui.label(
                            egui::RichText::new("съёмный")
                                .small()
                                .color(t.warn),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} / {}",
                                fmt_bytes(total - free),
                                fmt_bytes(total)
                            ))
                            .color(t.text_dim),
                        );
                    });
                });
                ui.add_space(4.0);
                progress_bar(ui, &t, frac, t.level(frac));
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(format!(
                        "файловая система: {fs} · свободно {}",
                        fmt_bytes(free)
                    ))
                    .small()
                    .color(t.text_faint),
                );
            });
            ui.add_space(6.0);
        }
    });
}

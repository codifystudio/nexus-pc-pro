//! Видеокарта: адаптеры, обнаруженные через wgpu.

use crate::app::NexusApp;
use crate::theme::card_frame;

pub fn show(app: &mut NexusApp, ui: &mut egui::Ui) {
    let t = app.theme;

    egui::ScrollArea::vertical().show(ui, |ui| {
        if app.gpu_rx.is_some() {
            card_frame(&t).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        egui::RichText::new("Опрос графических адаптеров…")
                            .color(t.text_dim),
                    );
                });
            });
            return;
        }

        let adapters = app.gpu_cache.clone().unwrap_or_default();
        if adapters.is_empty() {
            card_frame(&t).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Адаптеры не обнаружены")
                        .text_style(egui::TextStyle::Name("Title".into()))
                        .color(t.text),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "Система не сообщила ни одного графического устройства.\n\
                         На серверных конфигурациях и в виртуальных машинах это ожидаемо.",
                    )
                    .small()
                    .color(t.text_dim),
                );
            });
            return;
        }

        ui.columns(
            adapters.len().min(3).max(1),
            |cols| {
                for (i, a) in adapters.iter().enumerate() {
                    let col = &mut cols[i % cols.len()];
                    card_frame(&t).show(col, |ui| {
                        ui.label(egui::RichText::new("АДАПТЕР").small().color(t.text_faint));
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(&a.name)
                                .text_style(egui::TextStyle::Name("ValueSmall".into()))
                                .color(t.text),
                        );
                        ui.add_space(4.0);
                        egui::Grid::new(format!("gpu{i}"))
                            .num_columns(2)
                            .spacing([14.0, 4.0])
                            .show(ui, |ui| {
                                row(ui, &t, "Бэкенд", &a.backend);
                                row(ui, &t, "Тип", &a.device_type);
                                row(ui, &t, "Драйвер", &a.driver);
                                row(ui, &t, "Версия", &a.driver_info);
                            });
                    });
                }
            },
        );

        ui.add_space(8.0);

        card_frame(&t).show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Список формируется на основе перечисления адаптеров графического стека. \
                     Для отображения занимает несколько секунд при первом открытии раздела.",
                )
                .small()
                .color(t.text_dim),
            );
        });
    });
}

fn row(ui: &mut egui::Ui, t: &crate::theme::Theme, k: &str, v: &str) {
    ui.label(egui::RichText::new(k).color(t.text_dim));
    ui.label(egui::RichText::new(v).color(t.text));
    ui.end_row();
}

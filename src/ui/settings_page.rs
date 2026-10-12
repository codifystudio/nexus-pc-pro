//! Настройки: тема, период обновления, параметры сканера, подтверждения.

use crate::app::NexusApp;
use crate::settings::{
    OUTPUT_CAP_MAX, OUTPUT_CAP_MIN, REFRESH_MAX_MS, REFRESH_MIN_MS, SCAN_LIMIT_MAX, SCAN_LIMIT_MIN,
};
use crate::theme::{card_frame, Theme};

pub fn show(app: &mut NexusApp, ui: &mut egui::Ui) {
    let t = app.theme;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Темы.
        card_frame(&t).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new("Оформление")
                    .text_style(egui::TextStyle::Name("Title".into()))
                    .color(t.text),
            );
            ui.add_space(8.0);
            ui.columns(Theme::COUNT, |cols| {
                for idx in 0..Theme::COUNT {
                    let preview = Theme::by_index(idx);
                    let selected = app.settings.theme_idx == idx;
                    let stroke = if selected {
                        egui::Stroke::new(2.0_f32, preview.accent)
                    } else {
                        egui::Stroke::new(1.0_f32, t.card_stroke)
                    };
                    let resp = egui::Frame::none()
                        .fill(preview.bg)
                        .stroke(stroke)
                        .rounding(egui::Rounding::same(10.0))
                        .inner_margin(egui::Margin::same(12.0))
                        .show(&mut cols[idx], |ui| {
                            ui.label(
                                egui::RichText::new(preview.name)
                                    .color(preview.text),
                            );
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                dot(ui, preview.accent);
                                dot(ui, preview.good);
                                dot(ui, preview.warn);
                                dot(ui, preview.bad);
                            });
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(if selected { "активна" } else { "выбрать" })
                                    .small()
                                    .color(preview.text_dim),
                            );
                        })
                        .response
                        .interact(egui::Sense::click());
                    if resp.clicked() {
                        app.set_theme_idx(idx);
                    }
                }
            });
        });

        ui.add_space(8.0);

        // Производительность.
        card_frame(&t).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new("Обновление и сканирование")
                    .text_style(egui::TextStyle::Name("Title".into()))
                    .color(t.text),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Период опроса метрик").color(t.text_dim));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut refresh = app.settings.refresh_ms;
                    let changed = ui
                        .add(
                            egui::Slider::new(&mut refresh, REFRESH_MIN_MS..=REFRESH_MAX_MS)
                                .suffix(" мс")
                                .step_by(50.0),
                        )
                        .changed();
                    if changed {
                        app.settings.refresh_ms = refresh;
                        let _ = app.settings.save();
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Лимит объектов сканера").color(t.text_dim));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut limit = app.settings.scan_limit;
                    let changed = ui
                        .add(
                            egui::Slider::new(&mut limit, SCAN_LIMIT_MIN..=SCAN_LIMIT_MAX)
                                .suffix(" шт."),
                        )
                        .changed();
                    if changed {
                        app.settings.scan_limit = limit;
                        let _ = app.settings.save();
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Ёмкость журнала сканера").color(t.text_dim));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut cap = app.settings.output_cap as f32;
                    let changed = ui
                        .add(
                            egui::Slider::new(
                                &mut cap,
                                (OUTPUT_CAP_MIN as f32)..=(OUTPUT_CAP_MAX as f32),
                            )
                            .suffix(" Б"),
                        )
                        .changed();
                    if changed {
                        app.settings.output_cap = cap as usize;
                        let _ = app.settings.save();
                    }
                });
            });
        });

        ui.add_space(8.0);

        // Безопасность.
        card_frame(&t).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new("Безопасность")
                    .text_style(egui::TextStyle::Name("Title".into()))
                    .color(t.text),
            );
            ui.add_space(4.0);
            let mut confirm = app.settings.confirm_actions;
            if ui
                .checkbox(&mut confirm, "Требовать подтверждение опасных действий")
                .changed()
            {
                app.settings.confirm_actions = confirm;
                let _ = app.settings.save();
            }
            ui.label(
                egui::RichText::new(
                    "Завершение процессов, удаление файлов и запуск утилит всегда проходят \
                     дополнительные проверки: белый список путей, двойную сверку pid/времени \
                     старта и запрет командной оболочки. Диалог подтверждения — последний рубеж.",
                )
                .small()
                .color(t.text_faint),
            );
        });

        ui.add_space(8.0);

        // Файл конфигурации.
        card_frame(&t).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Файл настроек")
                            .text_style(egui::TextStyle::Name("Title".into()))
                            .color(t.text),
                    );
                    ui.label(
                        egui::RichText::new(
                            crate::settings::Settings::file_path()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| "недоступно".into()),
                        )
                        .small()
                        .color(t.text_dim),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Сбросить").clicked() {
                        app.settings = crate::settings::Settings::default();
                        app.set_theme_idx(app.settings.theme_idx);
                        let _ = app.settings.save();
                        app.notify("Настройки сброшены", false);
                    }
                });
            });
        });
    });
}

fn dot(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    ui.painter_at(rect)
        .circle_filled(rect.center(), 4.5, color);
}

//! Очистка: категорийные итоги последнего скана и удаление с подтверждением.

use crate::app::{NexusApp, ScanState};
use crate::system::monitor::fmt_bytes;
use crate::system::scanner::Category;
use crate::theme::card_frame;
use crate::ui::scanner_page::{totals, color32_bg};

pub fn show(app: &mut NexusApp, ui: &mut egui::Ui) {
    let t = app.theme;

    card_frame(&t).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            egui::RichText::new("Очистка дискового пространства")
                .text_style(egui::TextStyle::Name("Title".into()))
                .color(t.text),
        );
        ui.label(
            egui::RichText::new(
                "Очистка работает по результатам сканера: удаляются только объекты из белого списка каталогов.",
            )
            .small()
            .color(t.text_dim),
        );
    });

    ui.add_space(8.0);

    if matches!(app.scan_state, ScanState::Idle) {
        card_frame(&t).show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Сначала выполните сканирование в разделе «Сканер».",
                )
                .color(t.text_dim),
            );
        });
        return;
    }

    let totals = totals(app);
    if totals.is_empty() {
        card_frame(&t).show(ui, |ui| {
            ui.label(
                egui::RichText::new("Найденные объекты уже обработаны либо список пуст.")
                    .color(t.text_dim),
            );
        });
        return;
    }

    // Карточки категорий.
    let cats: Vec<(Category, usize, u64)> = totals
        .iter()
        .map(|(c, (n, b))| (*c, *n, *b))
        .collect();
    ui.columns(cats.len().min(3).max(1), |cols| {
        for (i, (cat, n, bytes)) in cats.iter().enumerate() {
            let col = &mut cols[i % cols.len()];
            card_frame(&t).show(col, |ui| {
                ui.label(
                    egui::RichText::new(cat.label().to_uppercase())
                        .small()
                        .color(t.text_faint),
                );
                ui.label(
                    egui::RichText::new(fmt_bytes(*bytes))
                        .text_style(egui::TextStyle::Name("Value".into()))
                        .color(t.text),
                );
                ui.label(
                    egui::RichText::new(format!("{n} объектов"))
                        .small()
                        .color(t.text_dim),
                );
                ui.add_space(6.0);
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("Очистить").color(color32_bg(&t)))
                            .fill(t.accent),
                    )
                    .clicked()
                {
                    app.request_clean(Some(*cat));
                }
            });
        }
    });

    ui.add_space(8.0);

    card_frame(&t).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Все категории сразу:").color(t.text_dim));
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Очистить всё")
                            .color(egui::Color32::from_rgb(0xFF, 0xFF, 0xFF)),
                    )
                    .fill(t.bad),
                )
                .clicked()
            {
                app.request_clean(None);
            }
        });
    });

    // Отчёт о последней очистке.
    if let Some(report) = &app.last_clean {
        ui.add_space(8.0);
        card_frame(&t).show(ui, |ui| {
            ui.label(
                egui::RichText::new("ОТЧЁТ ПОСЛЕДНЕЙ ОЧИСТКИ")
                    .small()
                    .color(t.text_faint),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!(
                    "удалено объектов: {} · освобождено: {} · ошибок: {}",
                    report.removed.len(),
                    fmt_bytes(report.freed),
                    report.failed.len()
                ))
                .color(t.text),
            );
            if !report.failed.is_empty() {
                ui.add_space(2.0);
                egui::ScrollArea::vertical()
                    .max_height(120.0)
                    .show(ui, |ui| {
                        for (path, why) in report.failed.iter().take(50) {
                            ui.label(
                                egui::RichText::new(format!("{} — {why}", path.display()))
                                    .small()
                                    .color(t.warn),
                            );
                        }
                    });
            }
        });
    }
}

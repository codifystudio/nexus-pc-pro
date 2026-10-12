//! Сканер: поиск временных файлов, кэша и журналов в белом списке корней.

use crate::app::{NexusApp, ScanState};
use crate::system::monitor::fmt_bytes;
use crate::system::scanner::{allowed_roots, canonical_root, Category};
use crate::theme::card_frame;
use std::collections::BTreeMap;

pub fn show(app: &mut NexusApp, ui: &mut egui::Ui) {
    let t = app.theme;

    // Верхняя панель управления.
    card_frame(&t).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Сканирование системных каталогов")
                        .text_style(egui::TextStyle::Name("Title".into()))
                        .color(t.text),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "Область поиска: только белый список корней ({}) · лимит {} объектов",
                        roots_label(),
                        app.settings.scan_limit
                    ))
                    .small()
                    .color(t.text_dim),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match &app.scan_state {
                    ScanState::Running { .. } => {
                        if ui.button("Отменить").clicked() {
                            app.cancel_scan();
                        }
                    }
                    _ => {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Сканировать")
                                        .color(color32_bg(&t)),
                                )
                                .fill(t.accent),
                            )
                            .clicked()
                        {
                            app.start_scan();
                        }
                    }
                }
            });
        });
        if let ScanState::Running { files, bytes } = &app.scan_state {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    egui::RichText::new(format!(
                        "найдено объектов: {files} · суммарный объём: {}",
                        fmt_bytes(*bytes)
                    ))
                    .color(t.text_dim),
                );
            });
        }
    });

    ui.add_space(8.0);

    match &app.scan_state {
        ScanState::Idle => {
            card_frame(&t).show(ui, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Сканирование ещё не запускалось.\nНажмите «Сканировать», чтобы найти временные файлы, кэш и журналы.",
                    )
                    .color(t.text_dim),
                );
            });
        }
        ScanState::Finished { files, bytes, truncated } => {
            summary(app, ui, *files, *bytes, *truncated);
        }
        ScanState::Running { .. } => {
            // Живой список найденного.
            entries_list(app, ui, 200);
        }
    }

    // Журнал сканирования.
    ui.add_space(8.0);
    card_frame(&t).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.toggle_value(&mut app.show_log, "Журнал сканирования");
            if app.scan_log.dropped > 0 {
                ui.label(
                    egui::RichText::new(format!(
                        "усечено байт журнала: {}",
                        app.scan_log.dropped
                    ))
                    .small()
                    .color(t.warn),
                );
            }
        });
        if app.show_log {
            let text = app.scan_log.text();
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut text.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_width(ui.available_width())
                            .interactive(false),
                    );
                });
        }
    });
}

fn roots_label() -> String {
    allowed_roots()
        .iter()
        .filter_map(|r| canonical_root(r).ok())
        .map(|r| r.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn summary(app: &mut NexusApp, ui: &mut egui::Ui, files: usize, bytes: u64, truncated: bool) {
    let t = &app.theme;
    card_frame(t).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("НАЙДЕНО").small().color(t.text_faint));
                ui.label(
                    egui::RichText::new(format!("{files} объектов"))
                        .text_style(egui::TextStyle::Name("Value".into()))
                        .color(t.text),
                );
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("ОБЪЁМ").small().color(t.text_faint));
                ui.label(
                    egui::RichText::new(fmt_bytes(bytes))
                        .text_style(egui::TextStyle::Name("Value".into()))
                        .color(t.accent),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("Перейти к очистке").color(color32_bg(t)))
                            .fill(t.accent),
                    )
                    .clicked()
                {
                    app.page = crate::app::Page::Cleaner;
                }
            });
        });
        if truncated {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Обход остановлен по лимиту объектов — увеличьте лимит в настройках для полного сканирования.")
                    .small()
                    .color(t.warn),
            );
        }
    });

    ui.add_space(6.0);
    entries_list(app, ui, 400);
}

fn entries_list(app: &NexusApp, ui: &mut egui::Ui, max: usize) {
    let t = &app.theme;
    let entries: Vec<&crate::system::scanner::JunkEntry> =
        app.scan_entries.iter().take(max).collect();
    if entries.is_empty() {
        return;
    }
    card_frame(t).show(ui, |ui| {
        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
            egui::Grid::new("junk_grid")
                .num_columns(3)
                .spacing([16.0, 3.0])
                .show(ui, |ui| {
                    for e in entries {
                        ui.label(
                            egui::RichText::new(e.path.display().to_string())
                                .small()
                                .color(t.text),
                        );
                        ui.label(
                            egui::RichText::new(e.category.label())
                                .small()
                                .color(t.text_dim),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(fmt_bytes(e.size))
                                    .small()
                                    .color(t.text_dim),
                            );
                        });
                        ui.end_row();
                    }
                });
        });
        if app.scan_entries.len() > max {
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(format!(
                    "показаны первые {max} из {} объектов",
                    app.scan_entries.len()
                ))
                .small()
                .color(t.text_faint),
            );
        }
    });
}

/// Текст на акцентной кнопке — контрастный к акценту.
pub fn color32_bg(t: &crate::theme::Theme) -> egui::Color32 {
    let (r, g, b) = (t.accent.r(), t.accent.g(), t.accent.b());
    let lum = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
    if lum > 140.0 {
        egui::Color32::from_black_alpha(235)
    } else {
        egui::Color32::from_white_alpha(240)
    }
}

/// Категорийные итоги последнего скана (используется страницей очистки).
pub fn totals(app: &NexusApp) -> BTreeMap<Category, (usize, u64)> {
    let mut m: BTreeMap<Category, (usize, u64)> = BTreeMap::new();
    for e in &app.scan_entries {
        let slot = m.entry(e.category).or_insert((0, 0));
        slot.0 += 1;
        slot.1 += e.size;
    }
    m
}

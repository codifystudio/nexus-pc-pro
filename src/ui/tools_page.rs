//! Инструменты: карточки штатных утилит ОС с подтверждением запуска.

use crate::app::NexusApp;
use crate::system::tools::{available, resolve, TOOLS};
use crate::theme::card_frame;

pub fn show(app: &mut NexusApp, ui: &mut egui::Ui) {
    let t = app.theme;

    egui::ScrollArea::vertical().show(ui, |ui| {
        card_frame(&t).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new("Штатные утилиты системы")
                    .text_style(egui::TextStyle::Name("Title".into()))
                    .color(t.text),
            );
            ui.label(
                egui::RichText::new(
                    "Запуск выполняется напрямую, без командной оболочки, только по фиксированному списку программ.",
                )
                .small()
                .color(t.text_dim),
            );
        });

        ui.add_space(8.0);

        for spec in TOOLS.iter() {
            let resolvable = resolve(spec.tool).is_ok();
            let usable = available(spec.tool) && resolvable;
            card_frame(&t).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(spec.title)
                                .text_style(egui::TextStyle::Name("Title".into()))
                                .color(if usable { t.text } else { t.text_faint }),
                        );
                        ui.label(
                            egui::RichText::new(spec.desc)
                                .small()
                                .color(t.text_dim),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !available(spec.tool) {
                            ui.label(
                                egui::RichText::new("недоступно на этой ОС")
                                    .small()
                                    .color(t.text_faint),
                            );
                        } else if !resolvable {
                            ui.label(
                                egui::RichText::new("не найдено в системе")
                                    .small()
                                    .color(t.warn),
                            );
                        } else if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Запустить").color(crate::ui::scanner_page::color32_bg(&t)),
                                )
                                .fill(t.accent),
                            )
                            .clicked()
                        {
                            app.request_launch(spec.tool);
                        }
                    });
                });
            });
            ui.add_space(6.0);
        }
    });
}

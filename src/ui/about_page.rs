//! О программе: версия, стек, платформы, лицензия.

use crate::app::NexusApp;
use crate::theme::card_frame;

pub fn show(app: &mut NexusApp, ui: &mut egui::Ui) {
    let t = app.theme;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Бренд-блок.
        card_frame(&t).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui
                    .allocate_exact_size(egui::vec2(64.0, 64.0), egui::Sense::hover());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, egui::Rounding::same(16.0), t.accent);
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "N",
                    egui::FontId::new(34.0, egui::FontFamily::Name("Display".into())),
                    egui::Color32::from_black_alpha(230),
                );
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(crate::APP_NAME)
                            .text_style(egui::TextStyle::Heading)
                            .color(t.text),
                    );
                    ui.label(
                        egui::RichText::new(crate::APP_TAGLINE).color(t.text_dim),
                    );
                    ui.label(
                        egui::RichText::new(format!("версия {}", crate::APP_VERSION))
                            .small()
                            .color(t.text_faint),
                    );
                });
            });
        });

        ui.add_space(8.0);

        card_frame(&t).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new("ОПИСАНИЕ")
                    .small()
                    .color(t.text_faint),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Настольная утилита для мониторинга ресурсов и обслуживания ПК: \
                     обзорные показатели, графики нагрузки, управление процессами, \
                     сведения о видеокарте, сканер временных файлов и безопасная очистка. \
                     Работает на Windows, macOS и Linux.",
                )
                .color(t.text_dim),
            );
        });

        ui.add_space(8.0);

        ui.columns(2, |cols| {
            card_frame(&t).show(&mut cols[0], |ui| {
                ui.label(egui::RichText::new("ТЕХНОЛОГИИ").small().color(t.text_faint));
                ui.add_space(4.0);
                for line in [
                    "Язык — Rust",
                    "Интерфейс — egui / eframe",
                    "Метрики системы — sysinfo",
                    "Графические адаптеры — wgpu",
                    "Сборка — единый исполняемый файл",
                ] {
                    ui.label(egui::RichText::new(format!("• {line}")).color(t.text_dim));
                }
            });
            card_frame(&t).show(&mut cols[1], |ui| {
                ui.label(egui::RichText::new("ЛИЦЕНЗИЯ").small().color(t.text_faint));
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("MIT License")
                        .color(t.text),
                );
                ui.label(
                    egui::RichText::new(
                        "Программное обеспечение поставляется «как есть», без каких-либо гарантий. \
                         Полный текст лицензии — в файле LICENSE репозитория.",
                    )
                    .small()
                    .color(t.text_dim),
                );
            });
        });

        ui.add_space(8.0);

        card_frame(&t).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(egui::RichText::new("ИСХОДНЫЙ КОД").small().color(t.text_faint));
            ui.add_space(4.0);
            ui.hyperlink_to(
                egui::RichText::new("github.com/codifystudio/nexus-pc-pro").color(t.accent),
                "https://github.com/codifystudio/nexus-pc-pro",
            );
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(
                    "Готовые сборки для Windows, macOS и Linux публикуются в разделе Releases.",
                )
                .small()
                .color(t.text_faint),
            );
        });
    });
}

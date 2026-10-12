//! Обзор: карточки ключевых метрик и сведения о системе.

use crate::app::NexusApp;
use crate::system::monitor::fmt_bytes;
use crate::ui::widgets::{info_row, progress_bar, stat_card, status_dot};

pub fn show(app: &mut NexusApp, ui: &mut egui::Ui) {
    let t = app.theme;
    let s = app.sample.clone();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Первый ряд карточек.
        ui.columns(4, |cols| {
            stat_card(&mut cols[0], &t, "Процессор", &format!("{:.1}%", s.cpu_total),
                &format!("{} ядер · {}", app.sampler.core_count(), app.sampler.cpu_name()), s.cpu_total / 100.0);
            let mem_frac = if s.mem_total > 0 { s.mem_used as f32 / s.mem_total as f32 } else { 0.0 };
            stat_card(&mut cols[1], &t, "Память", &fmt_bytes(s.mem_used),
                &format!("из {} доступно", fmt_bytes(s.mem_total)), mem_frac);
            let swap_frac = if s.swap_total > 0 { s.swap_used as f32 / s.swap_total as f32 } else { 0.0 };
            stat_card(&mut cols[2], &t, "Swap", &fmt_bytes(s.swap_used),
                &format!("из {} занято", fmt_bytes(s.swap_total)), swap_frac);
            stat_card(&mut cols[3], &t, "Сеть", &format!("↓ {:.1} КиБ/с", s.net.down_kib),
                &format!("↑ {:.1} КиБ/с", s.net.up_kib), (s.net.down_kib / 2048.0).min(1.0));
        });

        ui.add_space(6.0);

        // Диски (первый диск) + процессы + температура.
        let root_disk = app
            .sampler
            .disks()
            .first()
            .map(|d| (
                d.mount_point().display().to_string(),
                d.total_space(),
                d.available_space(),
            ));
        let root_disk_ref = root_disk.clone();
        ui.columns(3, |cols| {
            if let Some((mount, total, free)) = root_disk_ref.clone() {
                let frac = if total > 0 {
                    1.0 - free as f32 / total as f32
                } else {
                    0.0
                };
                stat_card(&mut cols[0], &t, "Диск", &fmt_bytes(total - free),
                    &format!("{} · свободно {}", mount, fmt_bytes(free)), frac);
            } else {
                stat_card(&mut cols[0], &t, "Диски", "—", "Накопители не обнаружены", 0.0);
            }
            let n = app.procs.len();
            let heavy = app.procs.iter().filter(|p| p.cpu > 25.0).count();
            stat_card(&mut cols[1], &t, "Процессы", &n.to_string(),
                &format!("активных (>25% ЦП): {heavy}"), 0.0);
            let temp = app
                .sampler
                .temps
                .values()
                .cloned()
                .reduce(f32::max);
            match temp {
                Some(c) => stat_card(&mut cols[2], &t, "Температура", &format!("{c:.0} °C"),
                    "пиковое значение датчиков", (c / 100.0).min(1.0)),
                None => stat_card(&mut cols[2], &t, "Температура", "—",
                    "датчики недоступны", 0.0),
            }
        });

        ui.add_space(10.0);

        // Сведения о системе.
        crate::theme::card_frame(&t).show(ui, |ui| {
            ui.label(
                egui::RichText::new("СИСТЕМА")
                    .small()
                    .color(t.text_faint),
            );
            ui.add_space(6.0);
            egui::Grid::new("sysinfo")
                .num_columns(2)
                .min_col_width(160.0)
                .spacing([16.0, 5.0])
                .show(ui, |ui| {
                    info_row(ui, &t, "Устройство", &app.sampler.host_name());
                    info_row(ui, &t, "ОС", &app.sampler.os_name());
                    info_row(ui, &t, "Ядро", &app.sampler.kernel());
                    info_row(ui, &t, "Процессор", &app.sampler.cpu_name());
                    info_row(ui, &t, "Частота", &format!("{} МГц", app.sampler.cpu_freq_mhz()));
                    info_row(ui, &t, "Логических ядер", &app.sampler.core_count().to_string());
                });
        });

        ui.add_space(10.0);

        // Состояние компонентов.
        crate::theme::card_frame(&t).show(ui, |ui| {
            ui.label(
                egui::RichText::new("СОСТОЯНИЕ")
                    .small()
                    .color(t.text_faint),
            );
            ui.add_space(6.0);
            let rows: [(String, f32, &str); 3] = [
                (
                    "Процессор".into(),
                    s.cpu_total / 100.0,
                    if s.cpu_total > 85.0 { "высокая нагрузка" } else { "норма" },
                ),
                (
                    "Память".into(),
                    if s.mem_total > 0 { s.mem_used as f32 / s.mem_total as f32 } else { 0.0 },
                    if s.mem_total > 0 && s.mem_used as f32 / s.mem_total as f32 > 0.9 {
                        "нехватка памяти"
                    } else {
                        "норма"
                    },
                ),
                (
                    "Корневой диск".into(),
                    root_disk_ref.map(|(_, total, free)| if total > 0 {
                        1.0 - free as f32 / total as f32
                    } else {
                        0.0
                    }).unwrap_or(0.0),
                    "использование тома",
                ),
            ];
            for (name, frac, note) in rows {
                ui.horizontal(|ui| {
                    status_dot(ui, &t, t.level(frac));
                    ui.label(egui::RichText::new(name.clone()).color(t.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(note).small().color(t.text_dim));
                    });
                });
                progress_bar(ui, &t, frac, t.level(frac));
                ui.add_space(4.0);
            }
        });
    });
}

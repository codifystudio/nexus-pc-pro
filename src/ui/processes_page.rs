//! Процессы: таблица с поиском, сортировкой по ЦП и безопасным
//! завершением (SEC-03: подтверждение + проверка пары pid/время старта).

use crate::app::NexusApp;
use crate::system::monitor::fmt_bytes;
use crate::system::processes::{filter_rows, ProcRow};

pub fn show(app: &mut NexusApp, ui: &mut egui::Ui) {
    let t = app.theme;

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Поиск:").color(t.text_dim));
        ui.add(
            egui::TextEdit::singleline(&mut app.proc_query)
                .desired_width(260.0)
                .hint_text("имя процесса…"),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("всего: {}", app.procs.len()))
                    .small()
                    .color(t.text_faint),
            );
        });
    });

    ui.add_space(4.0);

    let rows: Vec<ProcRow> = filter_rows(&app.procs, &app.proc_query);
    let mut sorted = rows;
    sorted.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("proc_grid")
            .num_columns(6)
            .spacing([14.0, 4.0])
            .min_col_width(80.0)
            .striped(false)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("ИМЯ").small().color(t.text_faint));
                ui.label(egui::RichText::new("PID").small().color(t.text_faint));
                ui.label(egui::RichText::new("ЦП").small().color(t.text_faint));
                ui.label(egui::RichText::new("ПАМЯТЬ").small().color(t.text_faint));
                ui.label(egui::RichText::new("ВРЕМЯ РАБОТЫ").small().color(t.text_faint));
                ui.label(egui::RichText::new("").small());
                ui.end_row();

                for row in sorted.iter().take(600) {
                    ui.label(egui::RichText::new(&row.name).color(t.text));
                    ui.label(egui::RichText::new(row.pid.to_string()).color(t.text_dim));
                    let cpu_col = t.level(row.cpu / 100.0);
                    ui.label(
                        egui::RichText::new(format!("{:.1}%", row.cpu)).color(cpu_col),
                    );
                    ui.label(egui::RichText::new(fmt_bytes(row.mem_bytes)).color(t.text));
                    ui.label(
                        egui::RichText::new(format_run_time(row.run_time)).color(t.text_dim),
                    );
                    if ui
                        .button(egui::RichText::new("Завершить").small().color(t.bad))
                        .clicked()
                    {
                        app.request_kill(row);
                    }
                    ui.end_row();
                }
            });
    });
}

fn format_run_time(secs: u64) -> String {
    if secs < 60 {
        format!("{secs} с")
    } else if secs < 3600 {
        format!("{} мин", secs / 60)
    } else if secs < 86_400 {
        format!("{} ч {} мин", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{} д {} ч", secs / 86_400, (secs % 86_400) / 3600)
    }
}

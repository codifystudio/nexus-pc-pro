//! Мониторинг: графики ЦП, памяти и сети + загрузка по ядрам.

use crate::app::NexusApp;
use crate::system::monitor::fmt_bytes;
use crate::theme::{sunken_frame, Theme};
use egui_plot::{Legend, Line, Plot, PlotPoints};

fn history_line(name: &str, data: &std::collections::VecDeque<f32>, color: egui::Color32) -> Line {
    let points: PlotPoints = data
        .iter()
        .enumerate()
        .map(|(i, v)| [i as f64, *v as f64])
        .collect();
    Line::new(points).name(name).color(color).width(1.8_f32)
}

pub fn show(app: &mut NexusApp, ui: &mut egui::Ui) {
    let t: Theme = app.theme;
    let cpu: Vec<f32> = app.sampler.cpu_history.iter().cloned().collect();
    let mem: Vec<f32> = app.sampler.mem_history.iter().cloned().collect();
    let down: Vec<f32> = app.sampler.down_history.iter().cloned().collect();
    let up: Vec<f32> = app.sampler.up_history.iter().cloned().collect();
    let n = cpu.len().max(2) as f64;

    ui.columns(2, |cols| {
        sunken_frame(&t).show(&mut cols[0], |ui| {
            ui.label(egui::RichText::new("Процессор, %").color(t.text_dim));
            Plot::new("cpu_plot")
                .legend(Legend::default())
                .allow_drag(false)
                .allow_scroll(false)
                .allow_zoom(false)
                .allow_boxed_zoom(false)
                .y_axis_width(34)
                .include_y(0.0)
                .include_y(100.0)
                .set_margin_fraction(egui::Vec2::new(0.02, 0.08))
                .height(160.0)
                .show(ui, |p| {
                    p.line(history_line("ЦП", &cpu.into(), t.accent));
                });
            let _ = n;
        });
        sunken_frame(&t).show(&mut cols[1], |ui| {
            ui.label(egui::RichText::new("Память, %").color(t.text_dim));
            Plot::new("mem_plot")
                .legend(Legend::default())
                .allow_drag(false)
                .allow_scroll(false)
                .allow_zoom(false)
                .allow_boxed_zoom(false)
                .y_axis_width(34)
                .include_y(0.0)
                .include_y(100.0)
                .set_margin_fraction(egui::Vec2::new(0.02, 0.08))
                .height(160.0)
                .show(ui, |p| {
                    p.line(history_line("ОЗУ", &mem.into(), t.good));
                });
        });
    });

    ui.add_space(6.0);

    sunken_frame(&t).show(ui, |ui| {
        ui.label(egui::RichText::new("Сеть, КиБ/с").color(t.text_dim));
        Plot::new("net_plot")
            .legend(Legend::default())
            .allow_drag(false)
            .allow_scroll(false)
            .allow_zoom(false)
            .allow_boxed_zoom(false)
            .y_axis_width(34)
            .set_margin_fraction(egui::Vec2::new(0.02, 0.08))
            .height(140.0)
            .show(ui, |p| {
                p.line(history_line("Загрузка", &down.into(), t.accent));
                p.line(history_line("Отдача", &up.into(), t.warn));
            });
    });

    ui.add_space(6.0);

    // Загрузка по ядрам.
    crate::theme::card_frame(&t).show(ui, |ui| {
        ui.label(
            egui::RichText::new(format!("ЯДРА ({})", app.sample.cpu_cores.len()))
                .small()
                .color(t.text_faint),
        );
        ui.add_space(4.0);
        let cores = app.sample.cpu_cores.clone();
        let per_row = 4;
        let chunks: Vec<&[f32]> = cores.chunks(per_row).collect();
        for chunk in chunks {
            ui.columns(per_row, |cols| {
                for (i, col) in cols.iter_mut().enumerate() {
                    if let Some(v) = chunk.get(i) {
                        col.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Ядро {}", i + 1))
                                    .small()
                                    .color(t.text_dim),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(format!("{v:.0}%"))
                                        .small()
                                        .color(t.level(v / 100.0)),
                                );
                            });
                        });
                        crate::ui::widgets::progress_bar(col, &t, v / 100.0, t.level(v / 100.0));
                        col.add_space(2.0);
                    }
                }
            });
        }
    });

    ui.add_space(6.0);

    // Память в числах.
    crate::theme::card_frame(&t).show(ui, |ui| {
        ui.columns(3, |cols| {
            cols[0].label(
                egui::RichText::new(fmt_bytes(app.sample.mem_used))
                    .text_style(egui::TextStyle::Name("ValueSmall".into()))
                    .color(t.text),
            );
            cols[0].label(egui::RichText::new("использовано").small().color(t.text_dim));
            cols[1].label(
                egui::RichText::new(fmt_bytes(app.sample.mem_total - app.sample.mem_used))
                    .text_style(egui::TextStyle::Name("ValueSmall".into()))
                    .color(t.text),
            );
            cols[1].label(egui::RichText::new("свободно").small().color(t.text_dim));
            cols[2].label(
                egui::RichText::new(fmt_bytes(app.sample.swap_used))
                    .text_style(egui::TextStyle::Name("ValueSmall".into()))
                    .color(t.text),
            );
            cols[2].label(egui::RichText::new("swap занято").small().color(t.text_dim));
        });
    });
}

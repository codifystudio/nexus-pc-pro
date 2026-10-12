//! Дизайн-система: боковая панель навигации, верхняя панель и базовые
//! виджеты (карточки, индикаторы, кнопки). Вся отрисовка опирается на
//! токены темы, литеральные цвета запрещены.

use crate::app::{NexusApp, Page};
use crate::settings::REFRESH_MAX_MS;
use crate::system::monitor::fmt_bytes;
use crate::theme::{card_frame, mix};
use egui::{Align, Align2, Color32, Context, Frame, Label, Layout, Margin, Rounding, Sense, Ui, Vec2};

// --- Боковая панель ------------------------------------------------------

pub fn sidebar(app: &mut NexusApp, ctx: &Context) {
    egui::SidePanel::left("nav")
        .exact_width(228.0)
        .resizable(false)
        .frame(Frame::none().fill(app.theme.panel))
        .show(ctx, |ui| {
            ui.add_space(18.0);

            // Логотип.
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(38.0, 38.0), Sense::hover());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, Rounding::same(10.0), app.theme.accent);
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "N",
                    egui::FontId::new(21.0, egui::FontFamily::Name("Display".into())),
                    Color32::from_black_alpha(230),
                );
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("NEXUS PC PRO")
                            .text_style(egui::TextStyle::Name("Title".into()))
                            .color(app.theme.text),
                    );
                    ui.label(
                        egui::RichText::new(format!("v{}", crate::APP_VERSION))
                            .small()
                            .color(app.theme.text_faint),
                    );
                });
            });

            ui.add_space(18.0);

            // Группы разделов.
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (pages, group) in Page::all() {
                    ui.add_space(6.0);
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new(group.to_uppercase())
                                .small()
                                .color(app.theme.text_faint),
                        );
                    });
                    ui.add_space(2.0);
                    for page in pages {
                        nav_item(app, ui, page);
                    }
                }

                ui.add_space(16.0);

                // Переключатель тем.
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new("ТЕМА").small().color(app.theme.text_faint));
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    for idx in 0..crate::theme::Theme::COUNT {
                        let t = crate::theme::Theme::by_index(idx);
                        let selected = app.settings.theme_idx == idx;
                        let (rect, resp) = ui.allocate_exact_size(
                            Vec2::new(26.0, 26.0),
                            Sense::click(),
                        );
                        let painter = ui.painter_at(rect);
                        let fill = if selected {
                            t.accent
                        } else {
                            mix(t.bg, t.accent, 0.45)
                        };
                        painter.rect_filled(rect.shrink(4.0), Rounding::same(8.0), fill);
                        if selected {
                            painter.rect_stroke(rect, Rounding::same(8.0), egui::Stroke::new(1.5_f32, t.accent));
                        } else if resp.hovered() {
                            painter.rect_stroke(rect, Rounding::same(8.0), egui::Stroke::new(1.0_f32, t.card_stroke));
                        }
                        if resp.clicked() {
                            app.set_theme_idx(idx);
                        }
                    }
                });
            });

            ui.add_space(8.0);
        });
}

fn nav_item(app: &mut NexusApp, ui: &mut Ui, page: Page) {
    let active = app.page == page;
    let t = app.theme;
    let desired = Vec2::new(ui.available_width() - 16.0, 30.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    let bg = if active {
        t.accent_soft
    } else if resp.hovered() {
        mix(t.panel, t.text, 0.05)
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(8.0), bg);
    if active {
        let bar = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(0.0, 4.0),
            Vec2::new(3.0, rect.height() - 8.0),
        );
        painter.rect_filled(bar, Rounding::same(2.0), t.accent);
    }
    painter.text(
        egui::pos2(rect.left() + 16.0, rect.center().y),
        Align2::LEFT_CENTER,
        page.title(),
        egui::FontId::proportional(14.0),
        if active { t.text } else { t.text_dim },
    );
    if resp.clicked() {
        app.page = page;
    }
    ui.add_space(2.0);
}

// --- Верхняя панель ------------------------------------------------------

pub fn topbar(app: &mut NexusApp, ctx: &Context) {
    egui::TopBottomPanel::top("topbar")
        .exact_height(58.0)
        .resizable(false)
        .frame(Frame::none().fill(app.theme.panel))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(app.page.title())
                            .text_style(egui::TextStyle::Heading)
                            .color(app.theme.text),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    let s = &app.sample;
                    let mem_frac = if s.mem_total > 0 {
                        s.mem_used as f32 / s.mem_total as f32
                    } else {
                        0.0
                    };
                    chip(ui, &app.theme, &format!("ЦП {:.0}%", s.cpu_total), app.theme.level(s.cpu_total / 100.0));
                    chip(ui, &app.theme, &format!("ОЗУ {:.0}%", mem_frac * 100.0), app.theme.level(mem_frac));
                    chip(
                        ui,
                        &app.theme,
                        &format!("↓ {:.1} КиБ/с", s.net.down_kib),
                        app.theme.text_dim,
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "обновление каждые {} мс",
                            app.settings.refresh_ms
                        ))
                        .small()
                        .color(app.theme.text_faint),
                    );
                    let _ = REFRESH_MAX_MS;
                });
            });
            ui.add_space(4.0);
        });
}

/// Небольшая плашка-статус.
pub fn chip(ui: &mut Ui, t: &crate::theme::Theme, text: &str, dot: Color32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("●").color(dot).small());
        ui.label(egui::RichText::new(text).small().color(t.text_dim));
    });
}

// --- Карточки ------------------------------------------------------------

/// Крупная статистическая карточка.
pub fn stat_card(
    ui: &mut Ui,
    t: &crate::theme::Theme,
    label: &str,
    value: &str,
    sub: &str,
    frac: f32,
) {
    card_frame(t).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(egui::RichText::new(label.to_uppercase()).small().color(t.text_faint));
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(value)
                .text_style(egui::TextStyle::Name("Value".into()))
                .color(t.text),
        );
        ui.add_space(2.0);
        ui.label(egui::RichText::new(sub).small().color(t.text_dim));
        ui.add_space(6.0);
        progress_bar(ui, t, frac, t.level(frac));
    });
}

/// Тонкий индикатор заполнения.
pub fn progress_bar(ui: &mut Ui, t: &crate::theme::Theme, frac: f32, color: Color32) {
    let frac = frac.clamp(0.0, 1.0);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 6.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(3.0), t.sunken);
    if frac > 0.0 {
        let fill = egui::Rect::from_min_size(
            rect.left_top(),
            Vec2::new(rect.width() * frac, rect.height()),
        );
        painter.rect_filled(fill, Rounding::same(3.0), color);
    }
}

/// Строка "ключ — значение" внутри карточек.
pub fn info_row(ui: &mut Ui, t: &crate::theme::Theme, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(key).color(t.text_dim));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(egui::RichText::new(value).color(t.text));
        });
    });
}

/// Карточка-секция с заголовком.
pub fn section_card(ui: &mut Ui, t: &crate::theme::Theme, title: &str, add: impl FnOnce(&mut Ui)) {
    card_frame(t).show(ui, |ui| {
        ui.label(
            egui::RichText::new(title)
                .text_style(egui::TextStyle::Name("Title".into()))
                .color(t.text),
        );
        ui.add_space(6.0);
        add(ui);
    });
}

/// Круглый индикатор состояния.
pub fn status_dot(ui: &mut Ui, t: &crate::theme::Theme, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), Sense::hover());
    ui.painter_at(rect)
        .circle_filled(rect.center(), 4.0, color);
    let _ = t;
}

/// Кнопка с предупреждением.
pub fn danger_button(ui: &mut Ui, t: &crate::theme::Theme, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).color(Color32::from_rgb(0xFF, 0xFF, 0xFF)))
            .fill(t.bad)
            .rounding(Rounding::same(8.0)),
    )
}

/// Заглушка от неиспользуемых импортов при платформенных ветках.
pub fn _unused(_: &str) {}

pub fn label_small(ui: &mut Ui, t: &crate::theme::Theme, text: &str) {
    ui.add(
        Label::new(egui::RichText::new(text).small().color(t.text_dim)).wrap(true),
    );
}

pub fn margin_same(v: f32) -> Margin {
    Margin::same(v)
}

pub fn fmt_size(bytes: u64) -> String {
    fmt_bytes(bytes)
}

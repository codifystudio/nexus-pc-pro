//! Визуальная подсистема: три тёмные темы, типографика Inter и общий
//! стиль интерфейса. Все цвета задаёт `Theme`; UI-код никогда не
//! использует литеральные цвета напрямую.

use egui::{
    epaint::Vertex, Color32, Context, FontData, FontDefinitions, FontFamily, Frame, Mesh, Rect,
    Rounding, Style, TextStyle, Vec2, Visuals,
};

/// Полный набор дизайн-токенов одной темы.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    /// Фон рабочего окна (за карточками — градиент обоев).
    pub bg: Color32,
    /// Градиент обоев: верх и низ рабочего поля.
    pub grad_top: Color32,
    pub grad_bottom: Color32,
    /// Боковая панель (почти непрозрачная, слегка тонируется обоями).
    pub panel: Color32,
    /// Карточки и панели инструментов.
    pub card: Color32,
    pub card_hover: Color32,
    pub card_stroke: Color32,
    /// Вложенные поля (графики, ввод, таблицы).
    pub sunken: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub accent: Color32,
    /// Акцент с прозрачностью для мягких заливок.
    pub accent_soft: Color32,
    pub good: Color32,
    pub warn: Color32,
    pub bad: Color32,
}

/// Линейная интерполяция цветов.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t).round() as u8 };
    Color32::from_rgba_unmultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

impl Theme {
    pub const COUNT: usize = 3;

    pub fn by_index(idx: usize) -> Self {
        match idx % Self::COUNT {
            1 => Self::carbon(),
            2 => Self::aurora(),
            _ => Self::midnight(),
        }
    }

    /// Midnight — глубокий сине-чёрный, электрик.
    pub fn midnight() -> Self {
        Self {
            name: "Midnight",
            bg: Color32::from_rgb(0x0B, 0x0E, 0x14),
            grad_top: Color32::from_rgb(0x0D, 0x13, 0x1E),
            grad_bottom: Color32::from_rgb(0x08, 0x0A, 0x10),
            panel: Color32::from_rgba_unmultiplied(0x0E, 0x12, 0x1A, 235),
            card: Color32::from_rgb(0x15, 0x1B, 0x26),
            card_hover: Color32::from_rgb(0x1B, 0x22, 0x2F),
            card_stroke: Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 14),
            sunken: Color32::from_rgb(0x0F, 0x14, 0x1D),
            text: Color32::from_rgb(0xE7, 0xEB, 0xF3),
            text_dim: Color32::from_rgb(0x9A, 0xA4, 0xB5),
            text_faint: Color32::from_rgb(0x63, 0x6D, 0x7F),
            accent: Color32::from_rgb(0x5B, 0x8C, 0xFF),
            accent_soft: Color32::from_rgba_unmultiplied(0x5B, 0x8C, 0xFF, 30),
            good: Color32::from_rgb(0x3D, 0xD6, 0x8C),
            warn: Color32::from_rgb(0xF5, 0xB5, 0x4A),
            bad: Color32::from_rgb(0xF0, 0x56, 0x4E),
        }
    }

    /// Carbon — графитовый минимализм, янтарный акцент.
    pub fn carbon() -> Self {
        Self {
            name: "Carbon",
            bg: Color32::from_rgb(0x0E, 0x0F, 0x11),
            grad_top: Color32::from_rgb(0x12, 0x13, 0x16),
            grad_bottom: Color32::from_rgb(0x0A, 0x0B, 0x0D),
            panel: Color32::from_rgba_unmultiplied(0x13, 0x14, 0x17, 235),
            card: Color32::from_rgb(0x1A, 0x1C, 0x20),
            card_hover: Color32::from_rgb(0x22, 0x25, 0x2A),
            card_stroke: Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 12),
            sunken: Color32::from_rgb(0x12, 0x13, 0x16),
            text: Color32::from_rgb(0xEC, 0xEE, 0xF1),
            text_dim: Color32::from_rgb(0xA2, 0xA7, 0xAF),
            text_faint: Color32::from_rgb(0x6B, 0x70, 0x78),
            accent: Color32::from_rgb(0xF5, 0xA5, 0x24),
            accent_soft: Color32::from_rgba_unmultiplied(0xF5, 0xA5, 0x24, 28),
            good: Color32::from_rgb(0x4A, 0xCE, 0x8A),
            warn: Color32::from_rgb(0xF5, 0xA5, 0x24),
            bad: Color32::from_rgb(0xE5, 0x48, 0x4D),
        }
    }

    /// Aurora — глубокий тёмно-зелёный, бирюзовый акцент.
    pub fn aurora() -> Self {
        Self {
            name: "Aurora",
            bg: Color32::from_rgb(0x0A, 0x14, 0x12),
            grad_top: Color32::from_rgb(0x0C, 0x1A, 0x17),
            grad_bottom: Color32::from_rgb(0x07, 0x0F, 0x0E),
            panel: Color32::from_rgba_unmultiplied(0x0D, 0x19, 0x16, 235),
            card: Color32::from_rgb(0x12, 0x22, 0x1D),
            card_hover: Color32::from_rgb(0x18, 0x2B, 0x25),
            card_stroke: Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 12),
            sunken: Color32::from_rgb(0x0C, 0x18, 0x15),
            text: Color32::from_rgb(0xE4, 0xF0, 0xEC),
            text_dim: Color32::from_rgb(0x93, 0xA8, 0xA1),
            text_faint: Color32::from_rgb(0x5D, 0x70, 0x6A),
            accent: Color32::from_rgb(0x2D, 0xD4, 0xBF),
            accent_soft: Color32::from_rgba_unmultiplied(0x2D, 0xD4, 0xBF, 28),
            good: Color32::from_rgb(0x3D, 0xD6, 0x8C),
            warn: Color32::from_rgb(0xF5, 0xB5, 0x4A),
            bad: Color32::from_rgb(0xF0, 0x56, 0x4E),
        }
    }

    /// Цвет уровня загрузки: норма / внимание / опасность.
    pub fn level(&self, frac: f32) -> Color32 {
        if frac >= 0.9 {
            self.bad
        } else if frac >= 0.75 {
            self.warn
        } else {
            self.accent
        }
    }

    /// Применить тему и общий стиль к контексту egui.
    pub fn apply(&self, ctx: &Context) {
        let mut style = (*ctx.style()).clone();
        self.apply_fonts(ctx);
        self.apply_visuals(&mut style);
        self.apply_text_styles(&mut style);
        ctx.set_style(style);
    }

    fn apply_text_styles(&self, style: &mut Style) {
        let display = FontFamily::Name("Display".into());
        let mut ts = std::collections::BTreeMap::new();
        ts.insert(TextStyle::Heading, egui::FontId::new(21.0, display.clone()));
        ts.insert(TextStyle::Body, egui::FontId::proportional(14.5));
        ts.insert(TextStyle::Button, egui::FontId::proportional(14.0));
        ts.insert(TextStyle::Small, egui::FontId::proportional(12.0));
        ts.insert(TextStyle::Monospace, egui::FontId::monospace(13.0));
        ts.insert(TextStyle::Name("Value".into()), egui::FontId::new(27.0, display.clone()));
        ts.insert(TextStyle::Name("ValueSmall".into()), egui::FontId::new(19.0, display.clone()));
        ts.insert(TextStyle::Name("Title".into()), egui::FontId::new(15.0, display.clone()));
        style.text_styles = ts;
    }

    fn apply_fonts(&self, ctx: &Context) {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "inter_body".into(),
            FontData::from_owned(include_bytes!("../assets/fonts/Inter-Regular.ttf").to_vec()),
        );
        fonts.font_data.insert(
            "inter_display".into(),
            FontData::from_owned(
                include_bytes!("../assets/fonts/InterDisplay-SemiBold.ttf").to_vec(),
            ),
        );
        // Inter первым в пропорциональном семействе — латиница/кириллица
        // рендерится им, недостающие символы (эмодзи и пр.) уходят в fallback.
        if let Some(list) = fonts.families.get_mut(&FontFamily::Proportional) {
            list.insert(0, "inter_body".into());
        }
        fonts
            .families
            .insert(FontFamily::Name("Display".into()), vec!["inter_display".into()]);
        ctx.set_fonts(fonts);
    }

    fn apply_visuals(&self, style: &mut Style) {
        let st = self.card_stroke;
        let mut v = Visuals::dark();
        v.panel_fill = self.panel;
        v.window_fill = self.card;
        v.extreme_bg_color = self.sunken;
        v.faint_bg_color = self.accent_soft;
        v.code_bg_color = self.sunken;
        v.hyperlink_color = self.accent;
        v.selection.bg_fill = self.accent.gamma_multiply(0.45);
        v.selection.stroke = egui::Stroke::new(1.0_f32, self.accent);
        v.window_rounding = Rounding::same(14.0);
        v.window_stroke = egui::Stroke::new(1.0_f32, st);
        v.window_shadow = egui::epaint::Shadow {
            offset: Vec2::new(0.0, 12.0),
            blur: 36.0,
            spread: 0.0,
            color: Color32::from_black_alpha(120),
        };
        v.popup_shadow = egui::epaint::Shadow {
            offset: Vec2::new(0.0, 6.0),
            blur: 18.0,
            spread: 0.0,
            color: Color32::from_black_alpha(90),
        };

        let rounding = Rounding::same(8.0);
        v.widgets.noninteractive.rounding = rounding;
        v.widgets.inactive.rounding = rounding;
        v.widgets.hovered.rounding = rounding;
        v.widgets.active.rounding = rounding;
        v.widgets.open.rounding = rounding;

        v.widgets.noninteractive.bg_fill = self.sunken;
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, st);
        v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, self.text_dim);

        v.widgets.inactive.bg_fill = self.card;
        v.widgets.inactive.weak_bg_fill = self.card;
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, st);
        v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, self.text);

        v.widgets.hovered.bg_fill = self.card_hover;
        v.widgets.hovered.weak_bg_fill = self.card_hover;
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, self.accent.gamma_multiply(0.55));
        v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, self.text);

        v.widgets.active.bg_fill = self.accent_soft;
        v.widgets.active.weak_bg_fill = self.accent_soft;
        v.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, self.accent);
        v.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, self.text);

        v.widgets.open.bg_fill = self.card_hover;
        v.widgets.open.weak_bg_fill = self.card_hover;

        style.visuals = v;
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(14.0, 7.0);
        style.spacing.menu_margin = egui::Margin::same(8.0);
        style.spacing.scroll = egui::style::ScrollStyle::solid();
        style.spacing.indent = 18.0;
    }

}

/// Вертикальный градиент (обои рабочего поля) через mesh — рисуется GPU,
/// в отличие от оверлеев на CPU.
pub fn paint_gradient(painter: egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    let vertex = |pos: egui::Pos2, color: Color32| Vertex {
        pos,
        uv: egui::epaint::WHITE_UV,
        color,
    };
    mesh.vertices = vec![
        vertex(rect.left_top(), top),
        vertex(rect.right_top(), top),
        vertex(rect.right_bottom(), bottom),
        vertex(rect.left_bottom(), bottom),
    ];
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    painter.add(egui::Shape::mesh(mesh));
}

/// Стандартная карточка: скругление 12, тонкий штрих, мягкий фон.
pub fn card_frame(t: &Theme) -> Frame {
    Frame::none()
        .fill(t.card)
        .stroke(egui::Stroke::new(1.0_f32, t.card_stroke))
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::same(14.0))
        .outer_margin(egui::Margin::same(0.0))
}

/// Затемнённая "колодец"-панель для графиков и таблиц.
pub fn sunken_frame(t: &Theme) -> Frame {
    Frame::none()
        .fill(t.sunken)
        .stroke(egui::Stroke::new(1.0_f32, t.card_stroke))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::same(10.0))
}

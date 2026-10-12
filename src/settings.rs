//! Настройки приложения: JSON-файл в каталоге конфигурации + жёсткая
//! нормализация (clamp) всех значений при загрузке. Атаки через
//! подменённый/повреждённый конфиг невозможны: любые значения вне
//! допустимых диапазонов приводятся к границам.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Индекс темы: 0 — Midnight, 1 — Carbon, 2 — Aurora.
pub const THEME_MIN: usize = 0;
pub const THEME_MAX: usize = 2;

/// Период опроса системных метрик, мс.
pub const REFRESH_MIN_MS: u64 = 250;
pub const REFRESH_MAX_MS: u64 = 5_000;

/// Максимальное число объектов, проходящих скан (защита от бесконечных ФС).
pub const SCAN_LIMIT_MIN: u32 = 1_000;
pub const SCAN_LIMIT_MAX: u32 = 200_000;
pub const SCAN_LIMIT_DEFAULT: u32 = 50_000;

/// Ёмкость журнала сканера (SEC-05: троттлинг вывода фоновых задач), байт.
pub const OUTPUT_CAP_MIN: usize = 8 * 1024;
pub const OUTPUT_CAP_MAX: usize = 1024 * 1024;
pub const OUTPUT_CAP_DEFAULT: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub theme_idx: usize,
    pub refresh_ms: u64,
    pub scan_limit: u32,
    pub output_cap: usize,
    /// Требовать подтверждение для опасных действий (завершение процессов,
    /// удаление файлов, запуск системных утилит).
    pub confirm_actions: bool,
    pub show_grid: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme_idx: 0,
            refresh_ms: 1_000,
            scan_limit: SCAN_LIMIT_DEFAULT,
            output_cap: OUTPUT_CAP_DEFAULT,
            confirm_actions: true,
            show_grid: false,
        }
    }
}

impl Settings {
    /// Нормализация значений. Вызывается всегда после десериализации.
    pub fn clamped(mut self) -> Self {
        self.theme_idx = self.theme_idx.clamp(THEME_MIN, THEME_MAX);
        self.refresh_ms = self.refresh_ms.clamp(REFRESH_MIN_MS, REFRESH_MAX_MS);
        self.scan_limit = self.scan_limit.clamp(SCAN_LIMIT_MIN, SCAN_LIMIT_MAX);
        self.output_cap = self.output_cap.clamp(OUTPUT_CAP_MIN, OUTPUT_CAP_MAX);
        self
    }

    /// Путь к файлу настроек: `<config_dir>/nexus-pc-pro/settings.json`.
    pub fn file_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("nexus-pc-pro").join("settings.json"))
    }

    pub fn load() -> Self {
        let path = match Self::file_path() {
            Some(p) => p,
            None => return Self::default().clamped(),
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::default().clamped();
        };
        match serde_json::from_str::<Settings>(&text) {
            Ok(s) => s.clamped(),
            Err(_) => Self::default().clamped(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::file_path() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        // Запись через временный файл: конфиг не может остаться битым.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

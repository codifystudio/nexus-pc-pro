//! NEXUS PC PRO — системный монитор и утилита обслуживания ПК.
//!
//! Библиотечная часть: вся логика приложения собрана здесь, чтобы
//! интеграционные тесты (`tests/security_tests.rs`) работали с тем же
//! кодом, что и бинарник.

pub mod app;
pub mod settings;
pub mod system;
pub mod theme;
pub mod ui;

pub const APP_NAME: &str = "NEXUS PC PRO";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_TAGLINE: &str = "Мониторинг и обслуживание системы";

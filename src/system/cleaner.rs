//! Очистка найденного мусора. Каждый путь перед удалением повторно
//! канонизируется и проверяется на принадлежность одному из корней
//! белого списка (SEC-01). Удаляются только регулярные файлы;
//! символические ссылки игнорируются, каталоги не рекурсируются.

use std::fs;
use std::path::{Path, PathBuf};

use super::scanner::{canonical_root, allowed_roots, JunkEntry};

#[derive(Debug)]
pub enum CleanError {
    /// Путь вне белого списка — удаление запрещено.
    OutsideWhitelist,
    /// Путь не найден (уже удалён) — не считается ошибкой в итогах,
    /// но возвращается вызывающему для прозрачности.
    NotFound,
    Io(std::io::Error),
}

/// SEC-01: проверить, что путь находится строго внутри одного из
/// канонизированных корней белого списка.
pub fn path_in_whitelist(path: &Path, roots: &[PathBuf]) -> bool {
    let Ok(canon) = fs::canonicalize(path) else {
        return false;
    };
    roots.iter().any(|root| canon.starts_with(root))
}

/// Итог очистки.
#[derive(Debug, Default, PartialEq)]
pub struct CleanReport {
    pub removed: Vec<PathBuf>,
    pub failed: Vec<(PathBuf, String)>,
    pub freed: u64,
}

/// Удалить набор записей. `entry.path` перепроверяется на каждый вызов:
/// между сканом и очисткой путь мог быть подменён символической ссылкой —
/// canonicalize разворачивает её, и проверка отсекает выход из корня.
pub fn clean(entries: &[JunkEntry]) -> Result<CleanReport, CleanError> {
    let roots = allowed_roots()
        .iter()
        .filter_map(|r| canonical_root(r).ok())
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return Err(CleanError::OutsideWhitelist);
    }
    let mut report = CleanReport::default();
    for entry in entries {
        if !path_in_whitelist(&entry.path, &roots) {
            report
                .failed
                .push((entry.path.clone(), "вне белого списка".into()));
            continue;
        }
        // Повторная канонизация после проверки: удаляем именно тот путь,
        // который проверили (отдельный вызов исключает гонку между
        // starts_with и remove_file по неканонизированному пути).
        let Ok(canon) = fs::canonicalize(&entry.path) else {
            report.failed.push((entry.path.clone(), "не найден".into()));
            continue;
        };
        let meta = match fs::symlink_metadata(&canon) {
            Ok(m) => m,
            Err(_) => {
                report.failed.push((entry.path.clone(), "не найден".into()));
                continue;
            }
        };
        if meta.is_symlink() || meta.is_dir() {
            report
                .failed
                .push((entry.path.clone(), "не регулярный файл".into()));
            continue;
        }
        match fs::remove_file(&canon) {
            Ok(()) => {
                report.freed += meta.len();
                report.removed.push(canon);
            }
            Err(e) => report.failed.push((entry.path.clone(), e.to_string())),
        }
    }
    Ok(report)
}

/// Подсчёт суммарного объёма по категориям.
pub fn total_bytes(entries: &[JunkEntry]) -> u64 {
    entries.iter().map(|e| e.size).sum()
}

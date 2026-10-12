//! Сканер мусорных файлов. Безопасность (SEC-01): сканируются только
//! корни из статического белого списка, канонизированные при старте.
//! Канонизация обязательна и fail-closed: если корень не существует или
//! не читается — он пропускается, и никакой "откат на текущий каталог"
//! не выполняется. Символические ссылки никогда не обходятся.

use super::monitor::fmt_bytes;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Ёмкость журнала (SEC-05): фоновая задача не может заполнить память
/// бесконечным выводом. Всё сверх лимита — отбрасывается со счётчиком.
#[derive(Debug, Default)]
pub struct OutputBuffer {
    pub bytes: Vec<u8>,
    pub dropped: u64,
}

impl OutputBuffer {
    pub fn push(&mut self, data: &[u8], cap: usize) {
        if self.bytes.len() + data.len() > cap {
            let free = cap.saturating_sub(self.bytes.len());
            self.bytes.extend_from_slice(&data[..free.min(data.len())]);
            self.dropped += (data.len() - free.min(data.len())) as u64;
        } else {
            self.bytes.extend_from_slice(data);
        }
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Category {
    Temp,
    Cache,
    Logs,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Temp => "Временные",
            Category::Cache => "Кэш",
            Category::Logs => "Журналы",
        }
    }
}

#[derive(Debug, Clone)]
pub struct JunkEntry {
    pub path: PathBuf,
    pub size: u64,
    pub category: Category,
}

/// Сообщения фонового потока сканирования.
pub enum ScanMsg {
    Entry(JunkEntry),
    LogChunk(String),
    Done { files: usize, bytes: u64, truncated: bool },
    Error(String),
}

/// Статический белый список корней сканирования/очистки.
pub fn allowed_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    roots.push(std::env::temp_dir());
    if let Some(cache) = dirs::cache_dir() {
        roots.push(cache);
    }
    roots
}

/// SEC-01: канонизация корня с проверкой существования. Никаких
/// подстановок по умолчанию: некорректный корень = отказ.
pub fn canonical_root(path: &Path) -> std::io::Result<PathBuf> {
    if !path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "корень не существует",
        ));
    }
    let canon = fs::canonicalize(path)?;
    if !canon.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "канонизированный путь не является каталогом",
        ));
    }
    Ok(canon)
}

fn classify(name: &str, is_dir: bool) -> Option<Category> {
    let lower = name.to_lowercase();
    if is_dir {
        for marker in ["cache", "temp", "tmp"] {
            if lower.contains(marker) {
                return Some(Category::Cache);
            }
        }
        return None;
    }
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "tmp" | "temp" | "old" | "bak" => Some(Category::Temp),
        "log" => Some(Category::Logs),
        _ => {
            if lower.contains(".cache") || lower.starts_with("cache_") {
                Some(Category::Cache)
            } else {
                None
            }
        }
    }
}

pub struct ScanResult {
    pub files: usize,
    pub bytes: u64,
    /// true, если достигнут лимит объектов и обход остановлен раньше конца.
    pub truncated: bool,
}

/// Синхронный обход одного канонизированного корня. Используется в
/// фоновом потоке; `cancel` позволяет прервать обход.
pub fn scan_root(
    root: &Path,
    limit: u32,
    cancel: &AtomicBool,
    tx: &std::sync::mpsc::Sender<ScanMsg>,
    log: &mut OutputBuffer,
    log_cap: usize,
) -> ScanResult {
    let mut visited: u32 = 0;
    let mut files = 0usize;
    let mut bytes = 0u64;
    let mut truncated = false;
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            truncated = true;
            log.push("[отменено пользователем]\n".as_bytes(), log_cap);
            break;
        }
        if visited >= limit {
            truncated = true;
            log.push(
                format!("[лимит {} объектов достигнут, обход остановлен]\n", limit).as_bytes(),
                log_cap,
            );
            break;
        }
        let Ok(read) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            visited += 1;
            if visited > limit {
                truncated = true;
                break;
            }
            let Ok(ft) = entry.file_type() else { continue };
            // Символические ссылки не переходим: исключает выход за
            // пределы белого списка и петли.
            if ft.is_symlink() {
                continue;
            }
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if ft.is_dir() {
                if let Some(cat) = classify(&name, true) {
                    // Категорийный каталог целиком: считаем его размер лениво,
                    // заносим как одну запись.
                    if let Ok(size) = dir_size(&path, limit - visited.min(limit), cancel) {
                        files += 1;
                        bytes += size;
                        log.push(
                            format!("{}  {}  [{}]\n", path.display(), fmt_bytes(size), cat.label())
                                .as_bytes(),
                            log_cap,
                        );
                        let _ = tx.send(ScanMsg::Entry(JunkEntry { path, size, category: cat }));
                    }
                } else {
                    stack.push(path);
                }
            } else if ft.is_file() {
                if let Some(cat) = classify(&name, false) {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    files += 1;
                    bytes += size;
                    log.push(
                        format!("{}  {}  [{}]\n", path.display(), fmt_bytes(size), cat.label())
                            .as_bytes(),
                        log_cap,
                    );
                    let _ = tx.send(ScanMsg::Entry(JunkEntry { path, size, category: cat }));
                }
            }
        }
    }
    ScanResult { files, bytes, truncated }
}

fn dir_size(dir: &Path, limit: u32, cancel: &AtomicBool) -> std::io::Result<u64> {
    let mut total = 0u64;
    let mut visited = 0u32;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        if cancel.load(Ordering::Relaxed) || visited > limit {
            break;
        }
        let Ok(read) = fs::read_dir(&d) else { continue };
        for entry in read.flatten() {
            visited += 1;
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                stack.push(entry.path());
            } else {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    Ok(total)
}

/// Удобная обёртка для тестов: скан с журналом в памяти.
pub fn scan_for_test(root: &Path, limit: u32, cap: usize) -> (ScanResult, OutputBuffer) {
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel();
    let mut log = OutputBuffer::default();
    let res = scan_root(root, limit, &cancel, &tx, &mut log, cap);
    while rx.try_recv().is_ok() {}
    (res, log)
}

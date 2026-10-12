//! Запуск системных утилит (SEC-02): ни одной строки-команды, ни одного
//! shell. Набор инструментов — замкнутое перечисление; путь к исполняемому
//! файлу резолвится по фиксированным правилам (стандартные каталоги Windows
//! или поиск в PATH с проверкой канонизации и бита исполнения).
//! Пользовательский ввод не участвует в формировании команды.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    TaskManager,
    DeviceManager,
    SystemInfo,
    DiskCleanup,
    Terminal,
    Files,
}

pub struct ToolSpec {
    pub tool: Tool,
    pub title: &'static str,
    pub desc: &'static str,
}

/// Отображаемые карточки инструментов (безопасные описания).
pub const TOOLS: [ToolSpec; 6] = [
    ToolSpec { tool: Tool::TaskManager, title: "Диспетчер задач", desc: "Запуск стандартного диспетчера задач ОС" },
    ToolSpec { tool: Tool::DeviceManager, title: "Диспетчер устройств", desc: "Оборудование и драйверы" },
    ToolSpec { tool: Tool::SystemInfo, title: "Сведения о системе", desc: "Полная конфигурация оборудования" },
    ToolSpec { tool: Tool::DiskCleanup, title: "Очистка диска", desc: "Встроенная утилита очистки ОС" },
    ToolSpec { tool: Tool::Terminal, title: "Терминал", desc: "Командная строка системы" },
    ToolSpec { tool: Tool::Files, title: "Файловый менеджер", desc: "Обозреватель файлов ОС" },
];

impl Tool {
    /// (программа, аргументы) для каждой платформы. Все элементы —
    /// константы времени компиляции.
    pub fn command(self) -> Option<(&'static str, &'static [&'static str])> {
        match self {
            Tool::TaskManager => windows_or_unix(
                ("taskmgr.exe", &[] as &[&str]),
                Some(("open", &["-a", "Activity Monitor"])),
                None,
            ),
            Tool::DeviceManager => windows_or_unix(
                ("mmc.exe", &["devmgmt.msc"]),
                None,
                None,
            ),
            Tool::SystemInfo => windows_or_unix(
                ("msinfo32.exe", &[]),
                Some(("open", &["-a", "System Information"])),
                Some(("gnome-system-monitor", &[])),
            ),
            Tool::DiskCleanup => windows_or_unix(
                ("cleanmgr.exe", &[]),
                None,
                None,
            ),
            Tool::Terminal => windows_or_unix(
                ("cmd.exe", &[]),
                Some(("open", &["-a", "Terminal"])),
                None,
            ),
            Tool::Files => windows_or_unix(
                ("explorer.exe", &[]),
                Some(("open", &[])),
                Some(("xdg-open", &["."])),
            ),
        }
    }
}

/// Разбор по платформам во время компиляции.
#[allow(unused_assignments)]
fn windows_or_unix(
    win: (&'static str, &'static [&'static str]),
    mac: Option<(&'static str, &'static [&'static str])>,
    linux: Option<(&'static str, &'static [&'static str])>,
) -> Option<(&'static str, &'static [&'static str])> {
    if cfg!(target_os = "windows") {
        Some(win)
    } else if cfg!(target_os = "macos") {
        mac
    } else {
        linux
    }
}

#[derive(Debug, PartialEq)]
pub enum ResolveError {
    /// Инструмент не предусмотрен на этой платформе.
    Unsupported,
    /// Исполняемый файл не найден в стандартных местах.
    NotFound,
}

/// Резолв пути к исполняемому файлу без запуска.
/// - Windows: поиск в %SystemRoot%, %SystemRoot%\System32, %PATH%;
/// - macOS/Linux: поиск в PATH с проверкойexecutable-бита и канонизацией.
pub fn resolve(tool: Tool) -> Result<PathBuf, ResolveError> {
    let Some((program, _)) = tool.command() else {
        return Err(ResolveError::Unsupported);
    };
    find_program(program).ok_or(ResolveError::NotFound)
}

fn find_program(program: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if cfg!(target_os = "windows") {
        if let Some(windir) = std::env::var_os("SystemRoot") {
            let w = PathBuf::from(&windir);
            candidates.push(w.join("System32").join(program));
            candidates.push(w.join(program));
        }
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            candidates.push(dir.join(program));
        }
    }

    for c in candidates {
        // canonicalize обязателен: каталоги в PATH могут содержать ссылки.
        let Ok(canon) = std::fs::canonicalize(&c) else { continue };
        if !canon.is_file() {
            continue;
        }
        let Ok(meta) = std::fs::metadata(&canon) else { continue };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if meta.permissions().mode() & 0o111 == 0 {
                continue;
            }
        }
        let _ = meta;
        return Some(canon);
    }
    None
}

#[derive(Debug)]
pub enum LaunchError {
    Resolve(ResolveError),
    Spawn(std::io::Error),
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchError::Resolve(ResolveError::Unsupported) => {
                write!(f, "Инструмент недоступен на этой платформе")
            }
            LaunchError::Resolve(ResolveError::NotFound) => {
                write!(f, "Исполняемый файл не найден")
            }
            LaunchError::Spawn(e) => write!(f, "Не удалось запустить: {e}"),
        }
    }
}

/// Запуск утилиты: без shell, строго по enum, с канонизированным путём.
/// Потоки ввода-вывода отключены — дочерний процесс полностью отвязан.
pub fn launch(tool: Tool) -> Result<(), LaunchError> {
    let path = resolve(tool).map_err(LaunchError::Resolve)?;
    let (_program, args) = tool.command().ok_or(LaunchError::Resolve(ResolveError::Unsupported))?;
    let mut cmd = Command::new(&path);
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
        cmd.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    cmd.spawn().map_err(LaunchError::Spawn)?;
    Ok(())
}

/// Видимость инструмента на текущей платформе: используется для того,
/// чтобы не показывать карточки, которых на этой ОС не существует.
pub fn available(tool: Tool) -> bool {
    tool.command().is_some()
}

/// Проверка, что путь программы — абсолютный (после канонизации).
pub fn is_absolute_program(path: &Path) -> bool {
    path.is_absolute()
}

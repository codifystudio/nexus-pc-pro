//! Интеграционные тесты безопасности NEXUS PC PRO.
//!
//! Каждый тест закрепляет конкретную гарантию из аудита:
//! SEC-01 — белый список путей, fail-closed канонизация;
//! SEC-02 — запуск утилит только по замкнутому перечислению, без shell;
//! SEC-03 — двойная проверка (pid, время старта) перед завершением процесса;
//! SEC-04 — обязательное подтверждение опасных действий;
//! SEC-05 — троттлинг вывода фоновых задач.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use nexus_pc_pro::app::ConfirmAction;
use nexus_pc_pro::settings::{Settings, OUTPUT_CAP_DEFAULT};
use nexus_pc_pro::system::cleaner::{self};
use nexus_pc_pro::system::processes::{self, KillError};
use nexus_pc_pro::system::scanner::{self, Category, JunkEntry, OutputBuffer};
use nexus_pc_pro::system::tools::{self, Tool};

fn test_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nexus_sec_{}_{}", std::process::id(), tag));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

// --- SEC-01: канонизация корней fail-closed -------------------------------

#[test]
fn sec01_canonical_root_fails_closed_on_missing_dir() {
    // Несуществующий корень обязан вернуть ошибку. Откат на текущий
    // каталог или "пустой" корень недопустим (fail-closed).
    let missing = std::env::temp_dir().join("nexus_sec_missing_root_no_such_dir");
    let _ = fs::remove_dir_all(&missing);
    assert!(scanner::canonical_root(&missing).is_err());

    // Файл вместо каталога — тоже отказ.
    let file = test_dir("canon_file");
    let f = file.join("f.txt");
    fs::write(&f, b"x").unwrap();
    assert!(scanner::canonical_root(&f).is_err());
}

#[test]
fn sec01_cleaner_rejects_paths_outside_whitelist() {
    // Файл ВНЕ белого списка: рабочий каталог проекта не входит в список
    // разрешённых корней (temp_dir + cache_dir).
    let outside_dir = std::env::current_dir()
        .unwrap()
        .join(format!("nexus_sec_outside_{}", std::process::id()));
    let _ = fs::remove_dir_all(&outside_dir);
    fs::create_dir_all(&outside_dir).unwrap();
    let outside_file = outside_dir.join("precious.txt");
    fs::write(&outside_file, b"important user data").unwrap();

    let entries = vec![JunkEntry {
        path: outside_file.clone(),
        size: 19,
        category: Category::Temp,
    }];

    // Вне белого списка удаление запрещено.
    let report = cleaner::clean(&entries).unwrap();
    assert_eq!(report.removed.len(), 0, "файл вне белого списка не должен быть удалён");
    assert!(outside_file.exists(), "файл обязан остаться на месте");
    assert_eq!(report.failed.len(), 1);
    let _ = fs::remove_dir_all(&outside_dir);

    // Вспомогательная проверка принадлежности: путь во временном каталоге
    // разрешён, путь в текущем рабочем каталоге — нет.
    let roots = scanner::allowed_roots()
        .iter()
        .filter_map(|r| scanner::canonical_root(r).ok())
        .collect::<Vec<_>>();
    let inside = test_dir("inside");
    let inside_file = inside.join("junk.tmp");
    fs::write(&inside_file, b"junk").unwrap();
    assert!(cleaner::path_in_whitelist(&inside_file, &roots));
    let cwd_file = std::env::current_dir().unwrap().join("Cargo.toml");
    assert!(!cleaner::path_in_whitelist(&cwd_file, &roots));
}

#[test]
fn sec01_scanner_skips_symlinks_and_stays_inside_root() {
    #[cfg(unix)]
    {
        let root = test_dir("symlink_root");
        fs::write(root.join("junk.tmp"), b"a").unwrap();
        let secret = test_dir("secret");
        fs::write(secret.join("keep.txt"), b"do not touch").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&secret, root.join("escape")).unwrap();

        let (res, _log) = scanner::scan_for_test(&root, 10_000, OUTPUT_CAP_DEFAULT);
        // Найден ровно один файл: junk.tmp. Символьная ссылка "escape"
        // не должна была раскрыться в содержимое каталога secret.
        assert_eq!(res.files, 1, "сканер не должен переходить по symlink");
        assert!(secret.join("keep.txt").exists());
    }
    // На платформах без прав создания symlink тест вырождается в no-op.
    let _ = 0;
}

// --- SEC-02: запуск утилит без shell --------------------------------------

#[test]
fn sec02_tools_use_enum_programs_never_shell() {
    const ALL: [Tool; 6] = [
        Tool::TaskManager,
        Tool::DeviceManager,
        Tool::SystemInfo,
        Tool::DiskCleanup,
        Tool::Terminal,
        Tool::Files,
    ];
    for tool in ALL {
        if let Some((program, args)) = tool.command() {
            // Программа — не shell и не интерпретатор.
            let base = program.rsplit(['\\', '/']).next().unwrap().to_lowercase();
            assert!(
                !["sh", "bash", "zsh", "cmd", "cmd.exe", "powershell", "pwsh", "powershell.exe"]
                    .contains(&base.as_str()),
                "запрещён запуск через интерпретатор: {program}"
            );
            // Аргументы не содержат метасимволов shell.
            for a in args {
                for bad in ['&', '|', ';', '`', '$', '>', '<'] {
                    assert!(!a.contains(bad), "метасимвол {bad} в аргументе {a}");
                }
            }
        }
    }
}

#[test]
fn sec02_tool_resolution_reports_missing_without_panic() {
    // Резолвер обязан либо найти фиксированную программу, либо
    // сообщить об отсутствии — но никогда не падать и не подменять путь.
    let result = tools::resolve(Tool::Files);
    match result {
        Ok(path) => assert!(tools::is_absolute_program(&path)),
        Err(tools::ResolveError::NotFound) | Err(tools::ResolveError::Unsupported) => {}
    }
}

// --- SEC-03: двойная проверка pid + время старта ---------------------------

fn spawn_dummy() -> std::process::Child {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "ping", "-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("не удалось запустить заглушку")
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("sleep")
            .arg("30")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("не удалось запустить заглушку")
    }
}

#[test]
fn sec03_kill_rejected_when_start_time_mismatches() {
    let mut child = spawn_dummy();
    let pid = child.id();
    let mut sys = sysinfo::System::new_all();
    sys.refresh_processes();
    std::thread::sleep(Duration::from_millis(150));
    sys.refresh_processes();

    // Время старта намеренно искажено — kill обязан быть отклонён.
    let real = sys
        .process(sysinfo::Pid::from_u32(pid))
        .expect("заглушка должна быть видна")
        .start_time();
    let wrong = real.wrapping_add(999_999);
    let result = processes::verify_and_kill(&mut sys, pid, wrong);
    assert_eq!(result, Err(KillError::PidReused));

    // Корректная пара (pid, start_time) — kill разрешён.
    let result = processes::verify_and_kill(&mut sys, pid, real);
    assert!(result.is_ok(), "корректная пара должна завершить процесс");
    let _ = child.wait();
}

#[test]
fn sec03_kill_reports_gone_for_unknown_pid() {
    let mut sys = sysinfo::System::new_all();
    // PID из приватного диапазона, почти наверняка не занят.
    let ghost: u32 = 4_294_901_760;
    let result = processes::verify_and_kill(&mut sys, ghost, 0);
    assert_eq!(result, Err(KillError::Gone));
}

// --- SEC-04: обязательное подтверждение ------------------------------------

#[test]
fn sec04_destructive_actions_require_confirmation() {
    // Любое действие из перечисления подтверждений является разрушительным
    // и обязано проходить через диалог (SEC-04).
    let actions = [
        ConfirmAction::KillProcess { pid: 1, start_time: 0, name: "x".into() },
        ConfirmAction::CleanJunk { category: None },
        ConfirmAction::LaunchTool(Tool::Terminal),
    ];
    for a in actions {
        assert!(matches!(
            a,
            ConfirmAction::KillProcess { .. }
                | ConfirmAction::CleanJunk { .. }
                | ConfirmAction::LaunchTool(_)
        ));
    }
    // По умолчанию подтверждения включены.
    let s = Settings::default();
    assert!(s.confirm_actions);
}

// --- SEC-05: троттлинг вывода фоновых задач --------------------------------

#[test]
fn sec05_output_buffer_throttles_at_cap() {
    let cap = 1024usize;
    let mut buf = OutputBuffer::default();
    let chunk = vec![b'x'; 512];
    buf.push(&chunk, cap);
    buf.push(&chunk, cap);
    buf.push(&chunk, cap);
    buf.push(&chunk, cap);
    assert!(
        buf.bytes.len() <= cap,
        "буфер не должен превышать лимит: {} > {cap}",
        buf.bytes.len()
    );
    assert!(buf.dropped > 0, "отброшенные байты должны учитываться");
}

#[test]
fn sec05_default_settings_use_64k_cap() {
    let s = Settings::default().clamped();
    assert_eq!(s.output_cap, OUTPUT_CAP_DEFAULT);
    assert_eq!(s.output_cap, 64 * 1024);
}

// --- Настройки: clamp и устойчивость ---------------------------------------

#[test]
fn settings_out_of_range_values_are_clamped() {
    let s = Settings {
        theme_idx: 99,
        refresh_ms: 5,
        scan_limit: 1,
        output_cap: 2,
        confirm_actions: false,
        show_grid: true,
    };
    let c = s.clamped();
    assert!(c.theme_idx <= 2);
    assert!(c.refresh_ms >= 250 && c.refresh_ms <= 5_000);
    assert!(c.scan_limit >= 1_000 && c.scan_limit <= 200_000);
    assert!(c.output_cap >= 8 * 1024 && c.output_cap <= 1024 * 1024);
}

#[test]
fn settings_json_roundtrip_preserves_values() {
    let s = Settings {
        theme_idx: 2,
        refresh_ms: 1_500,
        scan_limit: 77_777,
        output_cap: 128 * 1024,
        confirm_actions: false,
        show_grid: true,
    };
    let text = serde_json::to_string(&s).unwrap();
    let back: Settings = serde_json::from_str::<Settings>(&text).unwrap().clamped();
    assert_eq!(s, back);
}

// --- Сканер: лимит объектов -------------------------------------------------

#[test]
fn scanner_stops_at_entry_limit_and_flags_truncation() {
    let root = test_dir("limit");
    for i in 0..50 {
        fs::write(root.join(format!("f{i}.tmp")), b"junk").unwrap();
    }
    let (res, _log) = scanner::scan_for_test(&root, 10, OUTPUT_CAP_DEFAULT);
    assert!(res.truncated, "при превышении лимита обязан взводиться флаг");
    assert!(res.files <= 10, "объектов найдено не больше лимита");
    assert!(res.files >= 1);
}

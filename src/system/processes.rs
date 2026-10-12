//! Работа с процессами. Завершение процессов защищено двойной проверкой
//! (SEC-03): решение принимается по паре (pid, время старта). Если процесс
//! умер и его PID занял новый процесс, пара не совпадёт — kill будет
//! отклонён. Это устраняет TOCTOU-гонку и reuse-PID атаку.

use sysinfo::{Pid, System};

/// Снимок процесса из таблицы.
#[derive(Debug, Clone)]
pub struct ProcRow {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub mem_bytes: u64,
    pub start_time: u64,
    pub run_time: u64,
}

/// Текущий список процессов.
pub fn snapshot(sys: &System) -> Vec<ProcRow> {
    sys.processes()
        .values()
        .map(|p| ProcRow {
            pid: p.pid().as_u32(),
            name: p.name().to_owned(),
            cpu: p.cpu_usage(),
            mem_bytes: p.memory(),
            start_time: p.start_time(),
            run_time: p.run_time(),
        })
        .collect()
}

#[derive(Debug, PartialEq)]
pub enum KillError {
    /// Процесс с таким PID больше не существует.
    Gone,
    /// PID существует, но время старта не совпадает: после снятия снимка
    /// процесс завершился, а PID был переиспользован. Kill отклонён.
    PidReused,
}

/// SEC-03: завершить процесс можно только при совпадении (pid, start_time).
/// Возвращает Ok(имя процесса) при успешном kill.
pub fn verify_and_kill(sys: &mut System, pid: u32, expected_start_time: u64) -> Result<String, KillError> {
    let pid = Pid::from_u32(pid);
    // Перечитываем таблицу процессов непосредственно перед kill:
    // минимизирует окно TOCTOU.
    sys.refresh_processes();
    let Some(proc_now) = sys.process(pid) else {
        return Err(KillError::Gone);
    };
    if proc_now.start_time() != expected_start_time {
        return Err(KillError::PidReused);
    }
    let name = proc_now.name().to_owned();
    let _ = proc_now.kill();
    Ok(name)
}

/// Фильтрация списка по подстроке имени.
pub fn filter_rows(rows: &[ProcRow], query: &str) -> Vec<ProcRow> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return rows.to_vec();
    }
    rows.iter()
        .filter(|r| r.name.to_lowercase().contains(&q))
        .cloned()
        .collect()
}

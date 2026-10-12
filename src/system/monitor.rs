//! Периодический сборщик системных метрик: CPU (по ядрам), RAM, swap,
//! диски, сеть, датчики температуры. Хранит кольцевую историю для графиков.

use std::collections::BTreeMap;
use std::time::Instant;
use sysinfo::{Components, Disks, Networks, System};

pub const HISTORY_LEN: usize = 180;

#[derive(Debug, Clone, Copy, Default)]
pub struct NetRate {
    pub down_kib: f32,
    pub up_kib: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Sample {
    pub cpu_total: f32,
    pub cpu_cores: Vec<f32>,
    pub mem_used: u64,
    pub mem_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub net: NetRate,
}

pub struct Sampler {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    pub cpu_history: std::collections::VecDeque<f32>,
    pub mem_history: std::collections::VecDeque<f32>,
    pub down_history: std::collections::VecDeque<f32>,
    pub up_history: std::collections::VecDeque<f32>,
    pub temps: BTreeMap<String, f32>,
    last_refresh: Option<Instant>,
}

impl Sampler {
    pub fn new() -> Self {
        let mut s = Self {
            sys: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            cpu_history: Default::default(),
            mem_history: Default::default(),
            down_history: Default::default(),
            up_history: Default::default(),
            temps: Default::default(),
            last_refresh: None,
        };
        // Первый замер сразу после старта: нулевые дельты сети не должны
        // портить историю.
        s.cpu_history.push_back(s.sys.global_cpu_info().cpu_usage());
        s.mem_history.push_back(mem_frac(&s.sys) * 100.0);
        s
    }

    /// Обновить метрики. Вызывается раз в `refresh_ms` из UI-потока.
    pub fn tick(&mut self) -> Sample {
        let now = Instant::now();
        let dt = self
            .last_refresh
            .map(|t| now.duration_since(t).as_secs_f32())
            .unwrap_or(1.0)
            .max(0.001);

        self.sys.refresh_cpu();
        self.sys.refresh_memory();
        self.networks.refresh_list();
        self.components.refresh_list();

        let mut down = 0u64;
        let mut up = 0u64;
        for (_iface, data) in self.networks.iter() {
            down += data.received();
            up += data.transmitted();
        }
        let net = NetRate {
            down_kib: down as f32 / 1024.0 / dt,
            up_kib: up as f32 / 1024.0 / dt,
        };

        self.temps.clear();
        for c in self.components.list() {
            let t = c.temperature();
            if t > 0.0 && t < 150.0 {
                self.temps.insert(c.label().to_string(), t);
            }
        }

        let cpu_total = self.sys.global_cpu_info().cpu_usage();
        let cores = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        let sample = Sample {
            cpu_total,
            cpu_cores: cores,
            mem_used: self.sys.used_memory(),
            mem_total: self.sys.total_memory(),
            swap_used: self.sys.used_swap(),
            swap_total: self.sys.total_swap(),
            net,
        };

        self.cpu_history.push_back(cpu_total);
        self.mem_history.push_back(mem_frac(&self.sys) * 100.0);
        self.down_history.push_back(net.down_kib);
        self.up_history.push_back(net.up_kib);
        while self.cpu_history.len() > HISTORY_LEN {
            self.cpu_history.pop_front();
        }
        while self.mem_history.len() > HISTORY_LEN {
            self.mem_history.pop_front();
        }
        while self.down_history.len() > HISTORY_LEN {
            self.down_history.pop_front();
        }
        while self.up_history.len() > HISTORY_LEN {
            self.up_history.pop_front();
        }

        self.last_refresh = Some(now);
        sample
    }

    pub fn disks(&self) -> &[sysinfo::Disk] {
        self.disks.list()
    }

    /// Доступ к внутреннему System для работы с процессами.
    pub fn sys(&self) -> &System {
        &self.sys
    }

    pub fn sys_mut(&mut self) -> &mut System {
        &mut self.sys
    }

    pub fn cpu_name(&self) -> String {
        self.sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Процессор".into())
    }

    pub fn cpu_freq_mhz(&self) -> u64 {
        self.sys.cpus().first().map(|c| c.frequency()).unwrap_or(0)
    }

    pub fn core_count(&self) -> usize {
        self.sys.cpus().len()
    }

    pub fn os_name(&self) -> String {
        System::long_os_version().unwrap_or_else(|| "Неизвестная ОС".into())
    }

    pub fn kernel(&self) -> String {
        System::kernel_version().unwrap_or_else(|| "-".into())
    }

    pub fn host_name(&self) -> String {
        System::host_name().unwrap_or_else(|| "-".into())
    }
}

fn mem_frac(sys: &System) -> f32 {
    let total = sys.total_memory();
    if total == 0 {
        return 0.0;
    }
    (sys.used_memory() as f32 / total as f32).clamp(0.0, 1.0)
}

/// Форматирование объёмов: KiB/MiB/GiB/TiB.
pub fn fmt_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["Б", "КиБ", "МиБ", "ГиБ", "ТиБ"];
    let mut v = bytes as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} Б")
    } else {
        format!("{v:.1} {}", UNITS[unit])
    }
}

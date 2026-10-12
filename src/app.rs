//! Состояние приложения: навигация, периодические замеры, фоновые
//! задачи (сканер, очистка, GPU), подтверждения опасных действий и
//! всплывающие уведомления.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use crate::settings::Settings;
use crate::system::cleaner::{self, CleanReport, CleanError};
use crate::system::gpu::{self, GpuAdapter};
use crate::system::monitor::{Sample, Sampler};
use crate::system::processes::{self, ProcRow};
use crate::system::scanner::{self, JunkEntry, OutputBuffer, ScanMsg};
use crate::system::tools::Tool;
use crate::theme::{paint_gradient, Theme};
use crate::ui;

/// Разделы приложения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Monitor,
    Disks,
    Processes,
    Gpu,
    Scanner,
    Cleaner,
    Tools,
    SettingsPage,
    About,
}

impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Page::Dashboard => "Обзор",
            Page::Monitor => "Мониторинг",
            Page::Disks => "Диски",
            Page::Processes => "Процессы",
            Page::Gpu => "Видеокарта",
            Page::Scanner => "Сканер",
            Page::Cleaner => "Очистка",
            Page::Tools => "Инструменты",
            Page::SettingsPage => "Настройки",
            Page::About => "О программе",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Page::Dashboard => "Сводка о состоянии системы",
            Page::Monitor => "Процессор, память и сеть в реальном времени",
            Page::Disks => "Подключённые накопители и занятое пространство",
            Page::Processes => "Активные процессы и потребление ресурсов",
            Page::Gpu => "Обнаруженные графические адаптеры",
            Page::Scanner => "Поиск временных файлов, кэша и журналов",
            Page::Cleaner => "Освобождение дискового пространства",
            Page::Tools => "Запуск штатных утилит операционной системы",
            Page::SettingsPage => "Параметры приложения",
            Page::About => "Версия, платформа и лицензия",
        }
    }

    /// Метки разделов в боковой панели, сгруппированные по смыслу.
    pub fn all() -> [(Vec<Page>, &'static str); 3] {
        vec![
            (
                vec![
                    Page::Dashboard,
                    Page::Monitor,
                    Page::Disks,
                    Page::Processes,
                    Page::Gpu,
                ],
                "Мониторинг",
            ),
            (
                vec![Page::Scanner, Page::Cleaner, Page::Tools],
                "Обслуживание",
            ),
            (vec![Page::SettingsPage, Page::About], "Прочее"),
        ]
        .try_into()
        .expect("фиксированные группы")
    }
}

/// Действие, ожидающее подтверждения (SEC-04): опасные операции
/// выполняются только после явного щелчка по диалогу.
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    KillProcess { pid: u32, start_time: u64, name: String },
    CleanJunk { category: Option<scanner::Category> },
    LaunchTool(Tool),
}

#[derive(Debug, Clone)]
pub enum ScanState {
    Idle,
    Running { files: usize, bytes: u64 },
    Finished { files: usize, bytes: u64, truncated: bool },
}

pub struct NexusApp {
    pub settings: Settings,
    pub theme: Theme,
    pub page: Page,

    pub sampler: Sampler,
    pub sample: Sample,
    pub procs: Vec<ProcRow>,

    pub proc_query: String,

    pub gpu_cache: Option<Vec<GpuAdapter>>,
    pub gpu_rx: Option<mpsc::Receiver<Vec<GpuAdapter>>>,

    pub scan_state: ScanState,
    pub scan_entries: Vec<JunkEntry>,
    pub scan_log: OutputBuffer,
    pub scan_rx: Option<mpsc::Receiver<ScanMsg>>,
    pub scan_cancel: Option<Arc<AtomicBool>>,
    pub clean_rx: Option<mpsc::Receiver<Result<CleanReport, CleanError>>>,
    pub last_clean: Option<CleanReport>,

    pub confirm: Option<ConfirmAction>,
    pub toast: Option<(String, bool, Instant)>,
    pub show_log: bool,

    style_applied: bool,
    last_tick: Instant,
}

impl NexusApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let settings = Settings::load();
        let theme = Theme::by_index(settings.theme_idx);
        theme.apply(&cc.egui_ctx);
        let mut sampler = Sampler::new();
        let sample = sampler.tick();
        sampler.sys_mut().refresh_processes();
        let procs = processes::snapshot(sampler.sys());
        Self {
            settings,
            theme,
            page: Page::Dashboard,
            sampler,
            sample,
            procs,
            proc_query: String::new(),
            gpu_cache: None,
            gpu_rx: None,
            scan_state: ScanState::Idle,
            scan_entries: Vec::new(),
            scan_log: OutputBuffer::default(),
            scan_rx: None,
            scan_cancel: None,
            clean_rx: None,
            last_clean: None,
            confirm: None,
            toast: None,
            show_log: false,
            style_applied: true,
            last_tick: Instant::now(),
        }
    }

    /// Смена темы: перечитать токены и пометить стиль как требующий
    /// повторного применения.
    pub fn set_theme_idx(&mut self, idx: usize) {
        self.settings.theme_idx = idx.clamp(0, Theme::COUNT - 1);
        self.theme = Theme::by_index(self.settings.theme_idx);
        self.style_applied = false;
        let _ = self.settings.save();
    }

    fn ensure_style(&mut self, ctx: &egui::Context) {
        if !self.style_applied {
            self.theme.apply(ctx);
            self.style_applied = true;
        }
    }

    /// Периодический замер метрик и списка процессов.
    fn tick(&mut self) {
        self.sample = self.sampler.tick();
        self.sampler.sys_mut().refresh_processes();
        self.procs = processes::snapshot(self.sampler.sys());
        self.last_tick = Instant::now();
    }

    // --- Сканер ----------------------------------------------------------

    pub fn start_scan(&mut self) {
        if matches!(self.scan_state, ScanState::Running { .. }) {
            return;
        }
        // SEC-01: корни канонизируются в UI-потоке, fail-closed.
        let roots: Vec<std::path::PathBuf> = scanner::allowed_roots()
            .iter()
            .filter_map(|r| scanner::canonical_root(r).ok())
            .collect();
        if roots.is_empty() {
            self.notify("Белый список корней пуст — сканирование отменено", true);
            return;
        }
        let limit = self.settings.scan_limit;
        let cap = self.settings.output_cap;
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let cancel_thread = Arc::clone(&cancel);
        std::thread::spawn(move || {
            let mut log = OutputBuffer::default();
            let mut files = 0usize;
            let mut bytes = 0u64;
            let mut truncated = false;
            for root in roots {
                let res = scanner::scan_root(&root, limit, &cancel_thread, &tx, &mut log, cap);
                files += res.files;
                bytes += res.bytes;
                truncated |= res.truncated;
            }
            let _ = tx.send(ScanMsg::LogChunk(log.text()));
            let _ = tx.send(ScanMsg::Done { files, bytes, truncated });
        });
        self.scan_rx = Some(rx);
        self.scan_cancel = Some(cancel);
        self.scan_state = ScanState::Running { files: 0, bytes: 0 };
        self.scan_entries.clear();
        self.scan_log = OutputBuffer::default();
        self.last_clean = None;
    }

    pub fn cancel_scan(&mut self) {
        if let Some(c) = &self.scan_cancel {
            c.store(true, Ordering::Relaxed);
        }
    }

    fn poll_scan(&mut self) {
        let mut msgs: Vec<ScanMsg> = Vec::new();
        if let Some(rx) = &self.scan_rx {
            while let Ok(msg) = rx.try_recv() {
                msgs.push(msg);
            }
        }
        let mut log_chunk: Option<String> = None;
        let mut done: Option<(usize, u64, bool)> = None;
        for msg in msgs {
            match msg {
                ScanMsg::Entry(e) => {
                    if let ScanState::Running { files, bytes } = &mut self.scan_state {
                        *files += 1;
                        *bytes += e.size;
                    }
                    self.scan_entries.push(e);
                }
                ScanMsg::LogChunk(text) => log_chunk = Some(text),
                ScanMsg::Done { files, bytes, truncated } => {
                    done = Some((files, bytes, truncated));
                }
                ScanMsg::Error(text) => self.notify(&text, true),
            }
        }
        if let Some(text) = log_chunk {
            self.scan_log.push(text.as_bytes(), self.settings.output_cap);
        }
        if let Some((files, bytes, truncated)) = done {
            self.scan_state = ScanState::Finished { files, bytes, truncated };
            self.scan_rx = None;
            self.scan_cancel = None;
        }
    }

    /// Запрос очистки найденного (после подтверждения).
    pub fn request_clean(&mut self, category: Option<scanner::Category>) {
        self.confirm = Some(ConfirmAction::CleanJunk { category });
    }

    fn run_clean(&mut self, category: Option<scanner::Category>) {
        let entries: Vec<JunkEntry> = self
            .scan_entries
            .iter()
            .filter(|e| category.map_or(true, |c| e.category == c))
            .cloned()
            .collect();
        if entries.is_empty() {
            self.notify("Нет объектов для очистки", false);
            return;
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let report = cleaner::clean(&entries);
            let _ = tx.send(report);
        });
        self.clean_rx = Some(rx);
    }

    fn poll_clean(&mut self) {
        let mut results: Vec<Result<CleanReport, CleanError>> = Vec::new();
        if let Some(rx) = &self.clean_rx {
            while let Ok(r) = rx.try_recv() {
                results.push(r);
            }
        }
        for r in results {
            self.clean_rx = None;
            match r {
                Ok(report) => {
                    let n = report.removed.len();
                    let freed = report.freed;
                    let removed = report.removed.clone();
                    self.last_clean = Some(report);
                    self.notify(
                        &format!("Удалено объектов: {n}, освобождено {freed} байт"),
                        false,
                    );
                    self.scan_entries
                        .retain(|e| !removed.iter().any(|p| p == &e.path));
                }
                Err(e) => self.notify(&format!("Очистка отклонена: {e:?}"), true),
            }
        }
    }

    // --- GPU -------------------------------------------------------------

    fn poll_gpu(&mut self) {
        if self.page == Page::Gpu && self.gpu_cache.is_none() && self.gpu_rx.is_none() {
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(gpu::enumerate());
            });
            self.gpu_rx = Some(rx);
        }
        let mut adapters: Option<Vec<GpuAdapter>> = None;
        if let Some(rx) = &self.gpu_rx {
            if let Ok(list) = rx.try_recv() {
                adapters = Some(list);
            }
        }
        if let Some(list) = adapters {
            self.gpu_cache = Some(list);
            self.gpu_rx = None;
        }
    }

    // --- Прочее ----------------------------------------------------------

    pub fn notify(&mut self, text: &str, is_error: bool) {
        self.toast = Some((text.to_string(), is_error, Instant::now() + Duration::from_secs(3)));
    }

    pub fn request_kill(&mut self, row: &ProcRow) {
        self.confirm = Some(ConfirmAction::KillProcess {
            pid: row.pid,
            start_time: row.start_time,
            name: row.name.clone(),
        });
    }

    pub fn request_launch(&mut self, tool: Tool) {
        self.confirm = Some(ConfirmAction::LaunchTool(tool));
    }

    /// Диалог подтверждения опасных действий (SEC-04).
    fn draw_confirm(&mut self, ctx: &egui::Context) {
        let Some(action) = self.confirm.clone() else { return };
        let mut execute = false;
        egui::Window::new("Подтверждение")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let t = &self.theme;
                ui.add_space(2.0);
                match &action {
                    ConfirmAction::KillProcess { pid, name, .. } => {
                        ui.label(
                            egui::RichText::new(format!(
                                "Завершить процесс «{name}» (PID {pid})?"
                            ))
                            .size(15.0),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(
                                "Несохранённые данные процесса будут потеряны.",
                            )
                            .small()
                            .color(t.text_dim),
                        );
                    }
                    ConfirmAction::CleanJunk { category } => {
                        let label = category
                            .map(|c| format!("категории «{}»", c.label()))
                            .unwrap_or_else(|| "все найденные объекты".into());
                        ui.label(
                            egui::RichText::new(format!("Удалить {label}?"))
                                .size(15.0),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(
                                "Удаление выполняется только в пределах белого списка каталогов.",
                            )
                            .small()
                            .color(t.text_dim),
                        );
                    }
                    ConfirmAction::LaunchTool(tool) => {
                        ui.label(
                            egui::RichText::new(format!(
                                "Запустить «{}»?",
                                crate::system::tools::TOOLS
                                    .iter()
                                    .find(|s| s.tool == *tool)
                                    .map(|s| s.title)
                                    .unwrap_or("инструмент")
                            ))
                            .size(15.0),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(
                                "Будет запущена штатная утилита операционной системы.",
                            )
                            .small()
                            .color(t.text_dim),
                        );
                    }
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("Отмена")).clicked() {
                        self.confirm = None;
                    }
                    if ui
                        .button(egui::RichText::new("Подтвердить").color(t.accent))
                        .clicked()
                    {
                        execute = true;
                    }
                });
            });
        if execute {
            self.confirm = None;
            match action {
                ConfirmAction::KillProcess { pid, start_time, name } => {
                    let sys = self.sampler.sys_mut();
                    match processes::verify_and_kill(sys, pid, start_time) {
                        Ok(n) => self.notify(&format!("Процесс «{n}» завершён"), false),
                        Err(processes::KillError::Gone) => {
                            self.notify("Процесс уже завершился", true)
                        }
                        Err(processes::KillError::PidReused) => self.notify(
                            "PID переиспользован другим процессом — kill отклонён",
                            true,
                        ),
                    }
                    let _ = name;
                }
                ConfirmAction::CleanJunk { category } => self.run_clean(category),
                ConfirmAction::LaunchTool(tool) => match crate::system::tools::launch(tool) {
                    Ok(()) => self.notify("Утилита запущена", false),
                    Err(e) => self.notify(&e.to_string(), true),
                },
            }
        }
    }

    /// Всплывающее уведомление в правом нижнем углу.
    fn draw_toast(&mut self, ctx: &egui::Context) {
        let expired = match &self.toast {
            None => true,
            Some((_, _, until)) => Instant::now() > *until,
        };
        if expired {
            self.toast = None;
            return;
        }
        let Some((text, is_error, _)) = self.toast.clone() else { return };
        let t = self.theme.clone();
        let (dot, main) = if is_error { (t.bad, t.bad) } else { (t.good, t.text) };
        egui::Window::new("toast")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, [-18.0, -18.0])
            .frame(
                egui::Frame::none()
                    .fill(t.card)
                    .stroke(egui::Stroke::new(1.0_f32, t.card_stroke))
                    .rounding(egui::Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("●").color(dot));
                    ui.label(egui::RichText::new(text).color(main));
                });
            });
    }
}

impl eframe::App for NexusApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ensure_style(ctx);

        // Обои: вертикальный градиент фоновым слоем (рисуется GPU).
        paint_gradient(
            ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("wallpaper"),
            )),
            ctx.screen_rect(),
            self.theme.grad_top,
            self.theme.grad_bottom,
        );

        if self.last_tick.elapsed() >= Duration::from_millis(self.settings.refresh_ms) {
            self.tick();
        }
        self.poll_scan();
        self.poll_clean();
        self.poll_gpu();

        ui::widgets::sidebar(self, ctx);
        ui::widgets::topbar(self, ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(egui::Margin::same(16.0)),
            )
            .show(ctx, |ui| {
                match self.page {
                    Page::Dashboard => ui::dashboard::show(self, ui),
                    Page::Monitor => ui::monitor_page::show(self, ui),
                    Page::Disks => ui::disks_page::show(self, ui),
                    Page::Processes => ui::processes_page::show(self, ui),
                    Page::Gpu => ui::gpu_page::show(self, ui),
                    Page::Scanner => ui::scanner_page::show(self, ui),
                    Page::Cleaner => ui::cleaner_page::show(self, ui),
                    Page::Tools => ui::tools_page::show(self, ui),
                    Page::SettingsPage => ui::settings_page::show(self, ui),
                    Page::About => ui::about_page::show(self, ui),
                }
            });

        self.draw_confirm(ctx);
        self.draw_toast(ctx);

        ctx.request_repaint_after(Duration::from_millis(self.settings.refresh_ms));
    }
}

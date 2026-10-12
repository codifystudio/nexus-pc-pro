//! Определение видеокарт через wgpu: список адаптеров с именем,
//! бэкендом, типом устройства и версией драйвера. Кросс-платформенно
//! и без привязки к конкретному вендору.

#[derive(Debug, Clone)]
pub struct GpuAdapter {
    pub name: String,
    pub backend: String,
    pub device_type: String,
    pub driver: String,
    pub driver_info: String,
}

/// Перечислить адаптеры. Может занять до пары секунд при первом вызове —
/// результат кешируется вызывающей стороной.
pub fn enumerate() -> Vec<GpuAdapter> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    instance
        .enumerate_adapters(wgpu::Backends::all())
        .into_iter()
        .map(|a| {
            let info = a.get_info();
            GpuAdapter {
                name: if info.name.trim().is_empty() {
                    "Неизвестный адаптер".into()
                } else {
                    info.name.trim().to_string()
                },
                backend: format!("{:?}", info.backend),
                device_type: match info.device_type {
                    wgpu::DeviceType::IntegratedGpu => "Интегрированная".into(),
                    wgpu::DeviceType::DiscreteGpu => "Дискретная".into(),
                    wgpu::DeviceType::VirtualGpu => "Виртуальная".into(),
                    wgpu::DeviceType::Cpu => "Программный рендеринг".into(),
                    wgpu::DeviceType::Other => "Другое".into(),
                },
                driver: info.driver,
                driver_info: info.driver_info,
            }
        })
        .collect()
}

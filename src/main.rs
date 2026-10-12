//! Точка входа NEXUS PC PRO.

use nexus_pc_pro::app::NexusApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NEXUS PC PRO")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "NEXUS PC PRO",
        options,
        Box::new(|cc| Box::new(NexusApp::new(cc))),
    )
}

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RavomIO Workbench",
        options,
        Box::new(|_cc| Ok(Box::new(RavomApp::default()))),
    )
}

#[derive(Default)]
struct RavomApp;

impl eframe::App for RavomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("RavomIO Workbench v{}", ravom_core::version()));
            ui.label("egui 桌面端骨架已就绪。");
        });
    }
}

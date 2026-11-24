use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "FerroIO Workbench",
        options,
        Box::new(|_cc| Ok(Box::new(FerroApp::default()))),
    )
}

#[derive(Default)]
struct FerroApp;

impl eframe::App for FerroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("FerroIO Workbench v{}", ferro_core::version()));
            ui.label("egui 桌面端骨架已就绪。");
        });
    }
}

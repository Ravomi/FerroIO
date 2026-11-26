//! RavomIO desktop workbench (egui).

mod app;
mod board_index;
mod console;
mod icons;
mod panels;
mod plotter;
mod tasks;
mod theme;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 760.0])
            .with_icon(theme::app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "RavomIO Workbench",
        options,
        Box::new(|cc| {
            theme::apply_shadcn_theme(&cc.egui_ctx);
            Ok(Box::new(app::RavomApp::new()))
        }),
    )
}

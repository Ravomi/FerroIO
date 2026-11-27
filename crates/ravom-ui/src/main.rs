//! Ravomi — native monochrome embedded development workbench (egui).

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
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Ravomi")
        .with_inner_size([1320.0, 820.0])
        .with_min_inner_size([960.0, 620.0])
        .with_icon(theme::app_icon());

    #[cfg(target_os = "macos")]
    {
        viewport = viewport
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false);
    }

    #[cfg(not(target_os = "macos"))]
    {
        viewport = viewport.with_decorations(false);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Ravomi",
        options,
        Box::new(|cc| {
            theme::apply_shadcn_theme(&cc.egui_ctx);
            Ok(Box::new(app::RavomApp::new()))
        }),
    )
}

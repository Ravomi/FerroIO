use crate::app::{DrawerTab, RavomApp};
use crate::icons::{self, ButtonKind, Icon};
use crate::theme;
use eframe::egui::{self, Color32, Layout, RichText};

/// Render the collapsible bottom Output Drawer (Console | Serial | Wave).
pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        for (tab, label) in [
            (DrawerTab::Console, "Console"),
            (DrawerTab::Serial, "Serial"),
            (DrawerTab::Plotter, "Wave"),
        ] {
            let active = app.drawer_tab == tab;
            if theme::underline_tab(ui, label, active).clicked() {
                app.drawer_tab = tab;
            }
        }

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            let (close_rect, close_resp) =
                ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
            let close_resp = close_resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Hide Output Drawer (Cmd/Ctrl+J)");
            if ui.is_rect_visible(close_rect) {
                let fg = if close_resp.hovered() {
                    ui.painter().rect_filled(close_rect, 5.0, theme::BG_HOVER);
                    theme::FG_PRIMARY
                } else {
                    theme::FG_MUTED
                };
                let gal = ui.painter().layout_no_wrap(
                    "×".to_string(),
                    egui::FontId::proportional(14.0),
                    fg,
                );
                ui.painter().galley(
                    egui::pos2(
                        close_rect.center().x - gal.size().x * 0.5,
                        close_rect.center().y - gal.size().y * 0.5,
                    ),
                    gal,
                    fg,
                );
            }
            if close_resp.clicked() {
                app.drawer_open = false;
            }
            if matches!(app.drawer_tab, DrawerTab::Console | DrawerTab::Serial)
                && icons::button(ui, Icon::Trash, "", ButtonKind::Ghost, true)
                    .on_hover_text("Clear")
                    .clicked()
            {
                match app.drawer_tab {
                    DrawerTab::Console => app.console.clear(),
                    DrawerTab::Serial => app.serial_log.clear(),
                    DrawerTab::Plotter => {}
                }
            }
        });
    });

    theme::hairline(ui);
    ui.add_space(6.0);

    match app.drawer_tab {
        DrawerTab::Console => render_log_well(ui, "drawer_console_scroll", &app.console),
        DrawerTab::Serial => render_log_well(ui, "drawer_serial_scroll", &app.serial_log),
        DrawerTab::Plotter => {
            let h = (ui.available_height() - 34.0).max(88.0);
            app.plotter.ui(ui, h);
        }
    }
}

fn render_log_well(ui: &mut egui::Ui, id: &str, console: &crate::console::Console) {
    let h = (ui.available_height() - 6.0).max(72.0);
    egui::ScrollArea::vertical()
        .id_salt(id)
        .stick_to_bottom(true)
        .max_height(h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for line in console.lines() {
                let (color, is_err) = classify_log_color(line);
                let mut rt = RichText::new(line).size(11.5).monospace().color(color);
                if is_err {
                    rt = rt.strong();
                }
                ui.label(rt);
            }
        });
}

fn classify_log_color(line: &str) -> (Color32, bool) {
    let lower = line.to_ascii_lowercase();
    if lower.contains("error") || lower.contains("failed") {
        (theme::FG_PRIMARY, true)
    } else if lower.contains("warning") || lower.starts_with("$ ") {
        (theme::FG_SECONDARY, false)
    } else if line.starts_with("Build OK:")
        || line.starts_with("Flash OK:")
        || line.starts_with("Doctor OK:")
    {
        (theme::FG_PRIMARY, false)
    } else {
        (theme::FG_MUTED, false)
    }
}

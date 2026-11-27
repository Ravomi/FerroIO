use crate::app::{NavTab, RavomApp};
use crate::theme;
use eframe::egui::{self, Layout, RichText};

/// Render the 28px anchored bottom IDE status bar.
pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;

        // Left zone: active board & architecture info
        let board_opt = app.boards.find_by_qualified(&app.selected_board);
        let dot_color = if app.loaded_config.is_some() {
            theme::FG_PRIMARY
        } else {
            theme::FG_FAINT
        };
        theme::status_dot(ui, dot_color);

        ui.label(
            RichText::new(&app.selected_board)
                .size(11.0)
                .monospace()
                .color(theme::FG_PRIMARY),
        );
        if let Some(b) = board_opt {
            ui.label(
                RichText::new(format!(
                    "{} · {}",
                    b.mcu,
                    b.rust_target.as_deref().unwrap_or("custom-target")
                ))
                .size(11.0)
                .color(theme::FG_MUTED),
            );
        }

        // Right zone: Output drawer toggle, build profile, USB serial port, and active job status
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            let console_label = if app.drawer_open {
                "Console ▼"
            } else {
                "Console ▲"
            };
            let console_btn = ui
                .add(
                    egui::Button::new(RichText::new(console_label).size(10.5).monospace().color(
                        if app.drawer_open {
                            theme::FG_PRIMARY
                        } else {
                            theme::FG_MUTED
                        },
                    ))
                    .frame(false),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Toggle Output Drawer (Cmd/Ctrl+J)");
            if console_btn.clicked() {
                app.drawer_open = !app.drawer_open;
            }

            theme::v_divider(ui, 12.0);

            let profile_label = if app.release_mode { "RELEASE" } else { "DEBUG" };
            ui.label(
                RichText::new(profile_label)
                    .font(egui::FontId::monospace(10.5))
                    .color(if app.release_mode {
                        theme::FG_PRIMARY
                    } else {
                        theme::FG_MUTED
                    }),
            );

            if let Some(task) = app.tasks.running() {
                ui.label(
                    RichText::new(format!("{task}..."))
                        .size(11.0)
                        .color(theme::FG_PRIMARY),
                );
            }

            let usb_ports = app.visible_serial_ports();
            let port_summary = if let Some(sess) = &app.serial_session {
                (
                    format!("Serial · {} · {}", sess.port_name(), sess.baud()),
                    theme::FG_PRIMARY,
                )
            } else if let Some(first) = usb_ports.first() {
                (format!("USB · {}", first.path), theme::FG_SECONDARY)
            } else {
                ("No USB".to_string(), theme::FG_FAINT)
            };

            let usb_btn = ui
                .add(
                    egui::Button::new(
                        RichText::new(port_summary.0)
                            .size(11.0)
                            .monospace()
                            .color(port_summary.1),
                    )
                    .frame(false),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Open Serial Studio");
            if usb_btn.clicked() {
                app.active_tab = NavTab::Monitor;
            }
        });
    });
}

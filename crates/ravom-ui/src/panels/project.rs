use crate::app::RavomApp;
use crate::theme;
use eframe::egui::{self, Layout, RichText};

const CONSOLE_HEIGHT: f32 = 175.0;

pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    let top_height = (ui.available_height() - CONSOLE_HEIGHT - 12.0).max(200.0);

    // Upper scrollable cards area
    egui::ScrollArea::vertical()
        .id_salt("workspace_cards_scroll")
        .max_height(top_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Card 1: Project Configuration
            theme::card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    theme::section_title(ui, "Project Configuration");
                    if let Some(cfg) = &app.loaded_config {
                        let name = cfg.project.name.as_deref().unwrap_or("unnamed");
                        theme::badge(ui, name, theme::FG_PRIMARY);
                    }
                });
                ui.add_space(6.0);

                if let Some(cfg) = &app.loaded_config {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Active Env")
                                .size(12.0)
                                .color(theme::FG_MUTED),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut app.selected_env)
                                .desired_width(180.0)
                                .hint_text("default"),
                        );
                    });
                    ui.add_space(6.0);

                    for (name, env) in &cfg.env {
                        theme::inset_frame().show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(format!("[env.{name}]"))
                                        .size(12.0)
                                        .strong()
                                        .monospace()
                                        .color(theme::FG_PRIMARY),
                                );
                                theme::badge(
                                    ui,
                                    format!("platform: {}", env.platform),
                                    theme::FG_MUTED,
                                );
                                theme::badge(ui, format!("board: {}", env.board), theme::FG_MUTED);
                                theme::badge(
                                    ui,
                                    format!("framework: {}", env.framework),
                                    theme::FG_SECONDARY,
                                );
                            });
                        });
                        ui.add_space(4.0);
                    }
                } else {
                    ui.label(
                        RichText::new(
                            "No ravom.toml loaded. Select a board from the sidebar and click Init, \
                             or enter an existing workspace path above.",
                        )
                        .size(12.5)
                        .color(theme::FG_MUTED),
                    );
                }
            });

            // Card 2: Zephyr RTOS (conditional)
            let is_zephyr = app
                .loaded_config
                .as_ref()
                .and_then(|cfg| cfg.resolve_env(app.env_arg().as_deref()).ok())
                .is_some_and(|(_, env)| ravom_core::zephyr::is_zephyr_framework(&env.framework));

            if is_zephyr {
                ui.add_space(10.0);
                show_zephyr_section(app, ui);
            }

            // Card 3: Signal Plotter
            ui.add_space(10.0);
            theme::card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                app.plotter.ui(ui);
            });
        });

    ui.add_space(12.0);

    // Bottom docked Console card
    theme::card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            theme::section_title(ui, "Console");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(theme::secondary_button("Clear")).clicked() {
                    app.console.clear();
                }
            });
        });
        ui.add_space(6.0);

        theme::inset_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::ScrollArea::vertical()
                .id_salt("console_scroll")
                .stick_to_bottom(true)
                .max_height((CONSOLE_HEIGHT - 64.0).max(80.0))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for line in app.console.lines() {
                        ui.label(
                            RichText::new(line)
                                .size(11.5)
                                .monospace()
                                .color(theme::FG_SECONDARY),
                        );
                    }
                });
        });
    });
}

fn show_zephyr_section(app: &mut RavomApp, ui: &mut egui::Ui) {
    let active_kconfig = app
        .loaded_config
        .as_ref()
        .and_then(|c| c.resolve_env(app.env_arg().as_deref()).ok())
        .and_then(|(_, e)| e.zephyr.as_ref())
        .map(|z| z.kconfig.clone())
        .unwrap_or_default();

    let zenv = ravom_core::zephyr::detect_zephyr_env(None);
    theme::card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            theme::section_title(ui, "Zephyr RTOS");
            theme::badge(
                ui,
                if zenv.zephyr_base.is_some() {
                    "ZEPHYR_BASE: OK"
                } else {
                    "ZEPHYR_BASE: missing"
                },
                if zenv.zephyr_base.is_some() {
                    theme::ACCENT_OK
                } else {
                    theme::FG_MUTED
                },
            );
            theme::badge(
                ui,
                if zenv.sdk_dir.is_some() {
                    "SDK: OK"
                } else {
                    "SDK: missing"
                },
                if zenv.sdk_dir.is_some() {
                    theme::ACCENT_OK
                } else {
                    theme::FG_MUTED
                },
            );
        });
        ui.add_space(6.0);

        ui.horizontal_wrapped(|ui| {
            for (label, key) in [
                ("RTT", "CONFIG_USE_SEGGER_RTT"),
                ("Log", "CONFIG_LOG"),
                ("BLE", "CONFIG_BT"),
                ("Shell", "CONFIG_SHELL"),
                ("Rust", "CONFIG_RUST"),
            ] {
                let mut checked = active_kconfig
                    .get(key)
                    .is_some_and(|v| v.eq_ignore_ascii_case("y"));
                if ui.checkbox(&mut checked, label).changed() {
                    app.toggle_zephyr_kconfig(key);
                }
            }
        });
        ui.add_space(6.0);

        egui::CollapsingHeader::new(
            RichText::new("app.overlay (DeviceTree)")
                .size(12.5)
                .color(theme::FG_SECONDARY),
        )
        .default_open(false)
        .show(ui, |ui| {
            let resp = ui.add(
                egui::TextEdit::multiline(&mut app.overlay_draft)
                    .code_editor()
                    .desired_rows(5)
                    .desired_width(f32::INFINITY),
            );
            if resp.changed() {
                app.overlay_dirty = true;
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(app.overlay_dirty, theme::primary_button("Save Overlay"))
                    .clicked()
                {
                    app.save_overlay();
                }
            });
        });
    });
}

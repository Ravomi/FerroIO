use crate::app::{NavTab, RavomApp};
use crate::icons::{self, ButtonKind, Icon};
use crate::theme;
use eframe::egui::{self, Layout, RichText, Stroke};

/// Render the calm, Rikyu-inspired architectural Workspace & Build Pipeline view.
pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    let dir = app.project_path();
    let project_title = app
        .loaded_config
        .as_ref()
        .and_then(|c| c.project.name.as_deref())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if app.loaded_config.is_some() {
                "Active Project".to_string()
            } else {
                "Workspace".to_string()
            }
        });
    let eyebrow_text = if app.loaded_config.is_some() {
        "Active Project"
    } else {
        "Workbench"
    };
    let steps = app.workflow_steps();

    // Open architectural Hero Band directly on canvas
    let avail_w = ui.available_width();
    if avail_w >= 820.0 {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                theme::eyebrow(ui, eyebrow_text);
                theme::hero_title(ui, &project_title);
                ui.add(
                    egui::Label::new(
                        RichText::new(dir.display().to_string())
                            .size(11.0)
                            .monospace()
                            .color(theme::FG_MUTED),
                    )
                    .truncate(),
                );
            });
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                theme::workflow_stepper(ui, &steps);
            });
        });
    } else {
        theme::eyebrow(ui, eyebrow_text);
        theme::hero_title(ui, &project_title);
        ui.add(
            egui::Label::new(
                RichText::new(dir.display().to_string())
                    .size(11.0)
                    .monospace()
                    .color(theme::FG_MUTED),
            )
            .truncate(),
        );
        ui.add_space(10.0);
        theme::workflow_stepper(ui, &steps);
    }

    ui.add_space(14.0);
    theme::hairline(ui);
    ui.add_space(16.0);

    egui::ScrollArea::vertical()
        .id_salt("workspace_main_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if app.loaded_config.is_some() {
                show_loaded_dashboard(app, ui);
            } else {
                show_empty_onboarding(app, ui);
            }
        });
}

fn show_empty_onboarding(app: &mut RavomApp, ui: &mut egui::Ui) {
    let dir = app.project_path();
    let has_pio = dir.join("platformio.ini").exists();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Select an existing workspace or bootstrap a new embedded project.")
                .size(12.0)
                .color(theme::FG_MUTED),
        );
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if icons::button(ui, Icon::Folder, "Browse", ButtonKind::Outline, true)
                .on_hover_text("Browse workspace folder")
                .clicked()
            {
                app.open_folder_dialog();
            }
        });
    });

    ui.add_space(18.0);

    let wide = ui.available_width() >= 760.0;
    if wide {
        ui.columns(2, |cols| {
            cols[0].set_width((cols[0].available_width() - 10.0).max(200.0));
            render_onboarding_init_section(app, &mut cols[0], !has_pio);
            render_onboarding_migrate_section(app, &mut cols[1], has_pio);
        });
    } else {
        render_onboarding_migrate_section(app, ui, has_pio);
        ui.add_space(18.0);
        render_onboarding_init_section(app, ui, !has_pio);
    }

    ui.add_space(22.0);
    render_usb_hardware_section(app, ui);
}

fn render_onboarding_init_section(app: &mut RavomApp, ui: &mut egui::Ui, primary: bool) {
    theme::eyebrow(ui, "01 · Initialize");
    ui.add_space(4.0);
    theme::section_title(ui, "New Project");
    ui.add_space(8.0);
    theme::hairline(ui);
    ui.add_space(12.0);

    theme::kv_row(ui, "Board", &app.selected_board, true);
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        theme::field_label(ui, 116.0, 26.0, "Framework");
        let fw_label = match app.init_framework.as_str() {
            "" => "Default",
            "bare-metal" => "Bare-Metal Rust",
            "embassy" => "Embassy",
            "mirui" => "Mirui",
            "zephyr" => "Zephyr (C)",
            "zephyr-rust" => "Zephyr (Rust)",
            other => other,
        };
        egui::ComboBox::from_id_salt("onboarding_framework")
            .selected_text(fw_label)
            .width(155.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.init_framework, String::new(), "Default");
                ui.selectable_value(
                    &mut app.init_framework,
                    "bare-metal".to_string(),
                    "Bare-Metal Rust",
                );
                ui.selectable_value(&mut app.init_framework, "embassy".to_string(), "Embassy");
                ui.selectable_value(&mut app.init_framework, "mirui".to_string(), "Mirui");
                ui.selectable_value(&mut app.init_framework, "zephyr".to_string(), "Zephyr (C)");
                ui.selectable_value(
                    &mut app.init_framework,
                    "zephyr-rust".to_string(),
                    "Zephyr (Rust)",
                );
            });
    });

    ui.add_space(16.0);
    ui.horizontal_wrapped(|ui| {
        let kind = if primary {
            ButtonKind::Primary
        } else {
            ButtonKind::Outline
        };
        if icons::button(ui, Icon::Plus, "Create", kind, true).clicked() {
            let fw = (!app.init_framework.is_empty()).then(|| app.init_framework.clone());
            let board = app.selected_board.clone();
            app.run_init_here(&board, fw.as_deref());
        }
        if icons::button(ui, Icon::Cpu, "Boards", ButtonKind::Ghost, true).clicked() {
            app.active_tab = NavTab::Boards;
        }
    });
}

fn render_onboarding_migrate_section(app: &mut RavomApp, ui: &mut egui::Ui, has_pio: bool) {
    ui.horizontal(|ui| {
        theme::eyebrow(ui, "02 · Import");
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(if has_pio { "DETECTED" } else { "NONE" })
                    .font(egui::FontId::monospace(9.5))
                    .color(if has_pio {
                        theme::FG_PRIMARY
                    } else {
                        theme::FG_FAINT
                    }),
            );
        });
    });
    ui.add_space(4.0);
    theme::section_title(ui, "PlatformIO Import");
    ui.add_space(8.0);
    theme::hairline(ui);
    ui.add_space(12.0);

    theme::kv_row(
        ui,
        "Source",
        if has_pio { "platformio.ini" } else { "—" },
        true,
    );
    ui.add_space(16.0);
    let kind = if has_pio {
        ButtonKind::Primary
    } else {
        ButtonKind::Outline
    };
    if icons::button(ui, Icon::ArrowRight, "Migrate", kind, has_pio)
        .on_hover_text("Convert platformio.ini into ravom.toml")
        .clicked()
    {
        app.run_migrate();
    }
}

fn show_loaded_dashboard(app: &mut RavomApp, ui: &mut egui::Ui) {
    // 1. Swiss Hairline Spec Ledger (4 columns separated by 1px vertical hairlines)
    let board_opt = app.boards.find_by_qualified(&app.selected_board).cloned();
    let board_name = board_opt
        .as_ref()
        .map(|b| {
            if b.name.is_empty() {
                b.id.clone()
            } else {
                b.name.clone()
            }
        })
        .unwrap_or_else(|| app.selected_board.clone());
    let mcu_str = board_opt
        .as_ref()
        .map(|b| b.mcu.clone())
        .unwrap_or_else(|| "—".to_string());
    let mem_str = board_opt
        .as_ref()
        .and_then(|b| {
            b.flash_bytes
                .map(|f| format!("{}K / {}K", f / 1024, b.ram_bytes.unwrap_or(0) / 1024))
        })
        .unwrap_or_else(|| "—".to_string());
    let libs_str = format!("{} installed", app.installed_packages.len());

    let avail_w = ui.available_width();
    let cols = if avail_w >= 760.0 {
        4
    } else if avail_w >= 440.0 {
        2
    } else {
        1
    };
    let div_w = 10.0_f32;
    let cell_w = ((avail_w - div_w * (cols as f32 - 1.0)) / cols as f32).floor();

    let cells = [
        ("Board", board_name, true, Some(NavTab::Boards)),
        ("MCU", mcu_str, false, Some(NavTab::Boards)),
        ("Flash / SRAM", mem_str, false, None),
        (
            "Libraries",
            libs_str,
            !app.installed_packages.is_empty(),
            Some(NavTab::Libraries),
        ),
    ];

    for chunk in cells.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, (label, val, accent, nav)) in chunk.iter().enumerate() {
                if i > 0 {
                    theme::v_divider(ui, 38.0);
                }
                let resp = theme::ledger_cell(ui, label, val, *accent, cell_w);
                if let Some(target_tab) = nav {
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if resp.clicked() {
                        if *target_tab == NavTab::Libraries {
                            app.refresh_installed_packages();
                        }
                        app.active_tab = *target_tab;
                    }
                }
            }
        });
    }

    ui.add_space(14.0);
    theme::hairline(ui);
    ui.add_space(18.0);

    // 2. Asymmetric 2-Column Stage (Environments 60% | USB Hardware 40%)
    let stage_w = ui.available_width();
    if stage_w >= 820.0 {
        let left_w = (stage_w * 0.58).floor();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, ui.available_height().max(220.0)),
                Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(left_w);
                    render_manifest_column(app, ui);
                },
            );
            ui.separator();
            let right_w = ui.available_width().max(220.0);
            ui.allocate_ui_with_layout(
                egui::vec2(right_w, ui.available_height().max(220.0)),
                Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(right_w);
                    render_usb_hardware_section(app, ui);
                },
            );
        });
    } else {
        render_manifest_column(app, ui);
        ui.add_space(20.0);
        render_usb_hardware_section(app, ui);
    }
}

fn render_manifest_column(app: &mut RavomApp, ui: &mut egui::Ui) {
    let Some(cfg) = app.loaded_config.clone() else {
        return;
    };

    ui.horizontal(|ui| {
        theme::eyebrow(ui, "Environments");
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if icons::button(ui, Icon::Sliders, "", ButtonKind::Ghost, true)
                .on_hover_text("Inspect dry-run build/flash plan")
                .clicked()
            {
                app.run_dry_run_plan();
            }

            let mut env_changed = false;
            egui::ComboBox::from_id_salt("workspace_active_env")
                .selected_text(if app.selected_env.is_empty() {
                    "default".to_string()
                } else {
                    app.selected_env.clone()
                })
                .width(146.0)
                .show_ui(ui, |ui| {
                    for name in cfg.env.keys() {
                        if ui
                            .selectable_value(&mut app.selected_env, name.clone(), name)
                            .changed()
                        {
                            env_changed = true;
                            if let Some(env) = cfg.env.get(name) {
                                if let Ok(board) = ravom_core::registry::registry()
                                    .resolve(&env.board, Some(&env.platform))
                                {
                                    app.selected_board = board.qualified_id();
                                }
                            }
                        }
                    }
                });
            if env_changed {
                app.refresh_installed_packages();
            }
        });
    });

    ui.add_space(8.0);
    theme::hairline(ui);

    let mut picked_env: Option<(String, String, String)> = None;
    for (name, env) in &cfg.env {
        let active_env = name == &app.selected_env;
        let row_id = ui.make_persistent_id(("ws_env_row", name));
        let prev_hov = ui
            .ctx()
            .data(|d| d.get_temp::<bool>(row_id.with("hov")))
            .unwrap_or(false);
        let hov_t = theme::animate_spring(ui.ctx(), row_id.with("hov_s"), prev_hov);
        let fill = if active_env {
            theme::BG_SURFACE
        } else {
            theme::lerp_color(theme::BG_CANVAS, theme::BG_SURFACE, hov_t)
        };

        let frame = egui::Frame::new()
            .fill(fill)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).size(12.5).strong().monospace().color(
                        if active_env {
                            theme::FG_PRIMARY
                        } else {
                            theme::FG_SECONDARY
                        },
                    ));
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{} · {}", env.framework, env.board))
                                    .size(11.0)
                                    .monospace()
                                    .color(theme::FG_MUTED),
                            )
                            .truncate(),
                        );
                    });
                });
            });

        let r = frame.response.rect;
        ui.painter().line_segment(
            [
                egui::pos2(r.left(), r.bottom()),
                egui::pos2(r.right(), r.bottom()),
            ],
            Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
        );
        if active_env {
            let bar = egui::Rect::from_center_size(
                egui::pos2(r.left() + 1.0, r.center().y),
                egui::vec2(2.0, (r.height() - 12.0).max(8.0)),
            );
            ui.painter().rect_filled(bar, 1.0, theme::FG_PRIMARY);
        }

        let resp = frame
            .response
            .interact(egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let now_hov = resp.hovered();
        if now_hov != prev_hov {
            ui.ctx()
                .data_mut(|d| d.insert_temp(row_id.with("hov"), now_hov));
            ui.ctx().request_repaint();
        }
        if resp.clicked() {
            picked_env = Some((name.clone(), env.board.clone(), env.platform.clone()));
        }
    }

    if let Some((name, board, platform)) = picked_env {
        app.selected_env = name;
        if let Ok(resolved) = ravom_core::registry::registry().resolve(&board, Some(&platform)) {
            app.selected_board = resolved.qualified_id();
        }
        app.refresh_installed_packages();
    }

    let is_zephyr = cfg
        .resolve_env(app.env_arg().as_deref())
        .ok()
        .is_some_and(|(_, env)| ravom_core::zephyr::is_zephyr_framework(&env.framework));

    if is_zephyr {
        ui.add_space(18.0);
        show_zephyr_section(app, &cfg, ui);
    }
}

fn render_usb_hardware_section(app: &mut RavomApp, ui: &mut egui::Ui) {
    let usb_ports = app
        .visible_serial_ports()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();

    ui.horizontal(|ui| {
        theme::eyebrow(ui, "USB Hardware");
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if icons::button(ui, Icon::Refresh, "", ButtonKind::Ghost, true)
                .on_hover_text("Rescan USB ports")
                .clicked()
            {
                app.refresh_ports();
            }
            if icons::button(ui, Icon::Activity, "Doctor", ButtonKind::Ghost, true).clicked() {
                app.run_doctor();
                app.active_tab = NavTab::Doctor;
            }
            if icons::button(ui, Icon::Terminal, "Studio →", ButtonKind::Ghost, true).clicked() {
                app.active_tab = NavTab::Monitor;
            }
        });
    });

    ui.add_space(8.0);
    theme::hairline(ui);

    if usb_ports.is_empty() {
        ui.add_space(12.0);
        ui.label(
            RichText::new("No USB serial devices detected")
                .size(12.0)
                .color(theme::FG_FAINT),
        );
    } else {
        let mut picked_port = None;
        for p in &usb_ports {
            let row_id = ui.make_persistent_id(("usb_port_row", &p.path));
            let prev_hov = ui
                .ctx()
                .data(|d| d.get_temp::<bool>(row_id.with("hov")))
                .unwrap_or(false);
            let hov_t = theme::animate_spring(ui.ctx(), row_id.with("hov_s"), prev_hov);
            let fill = theme::lerp_color(theme::BG_CANVAS, theme::BG_SURFACE, hov_t);

            let frame = egui::Frame::new()
                .fill(fill)
                .inner_margin(egui::Margin::symmetric(8, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        theme::status_dot(ui, theme::FG_PRIMARY);
                        ui.label(
                            RichText::new(&p.path)
                                .size(12.0)
                                .monospace()
                                .color(theme::FG_PRIMARY),
                        );
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&p.description)
                                        .size(11.0)
                                        .color(theme::FG_MUTED),
                                )
                                .truncate(),
                            );
                        });
                    });
                });

            let r = frame.response.rect;
            ui.painter().line_segment(
                [
                    egui::pos2(r.left(), r.bottom()),
                    egui::pos2(r.right(), r.bottom()),
                ],
                Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
            );

            let resp = frame
                .response
                .interact(egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Open in Serial Studio");
            let now_hov = resp.hovered();
            if now_hov != prev_hov {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(row_id.with("hov"), now_hov));
                ui.ctx().request_repaint();
            }
            if resp.clicked() {
                picked_port = Some(p.path.clone());
            }
        }
        if let Some(path) = picked_port {
            app.selected_port = path;
            app.active_tab = NavTab::Monitor;
        }
    }
}

fn show_zephyr_section(
    app: &mut RavomApp,
    cfg: &ravom_core::config::RavomConfig,
    ui: &mut egui::Ui,
) {
    let active_kconfig = cfg
        .resolve_env(app.env_arg().as_deref())
        .ok()
        .and_then(|(_, e)| e.zephyr.as_ref())
        .map(|z| z.kconfig.clone())
        .unwrap_or_default();

    let zenv = ravom_core::zephyr::detect_zephyr_env(None);
    ui.horizontal_wrapped(|ui| {
        theme::eyebrow(ui, "Zephyr Kconfig");
        ui.add_space(10.0);
        ui.label(
            RichText::new(if zenv.zephyr_base.is_some() {
                "BASE ✓"
            } else {
                "BASE —"
            })
            .font(egui::FontId::monospace(10.0))
            .color(if zenv.zephyr_base.is_some() {
                theme::FG_PRIMARY
            } else {
                theme::FG_MUTED
            }),
        );
        ui.label(
            RichText::new(if zenv.sdk_dir.is_some() {
                "SDK ✓"
            } else {
                "SDK —"
            })
            .font(egui::FontId::monospace(10.0))
            .color(if zenv.sdk_dir.is_some() {
                theme::FG_PRIMARY
            } else {
                theme::FG_MUTED
            }),
        );
    });
    ui.add_space(6.0);
    theme::hairline(ui);
    ui.add_space(8.0);

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
    ui.add_space(8.0);

    egui::CollapsingHeader::new(
        RichText::new("app.overlay")
            .size(12.0)
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
        ui.add_space(6.0);
        if icons::button(
            ui,
            Icon::Check,
            "Save",
            ButtonKind::Primary,
            app.overlay_dirty,
        )
        .clicked()
        {
            app.save_overlay();
        }
    });
}

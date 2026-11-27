//! Monochrome Rikyu-styled Toolchain & Rust Targets configuration view.

use crate::app::{DoctorSubTab, RavomApp};
use crate::icons::{self, ButtonKind, Icon};
use crate::theme;
use eframe::egui;
use ravom_core::toolchain::{tools_config_path, RustTargetStatus, ToolCategory, ToolStatus};

/// Render the sleek Toolchain Doctor and Rust Target Configuration view.
pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    if app.doctor_tools.is_empty() && app.doctor_targets.is_empty() {
        app.run_doctor();
    }

    let tools_ready = app.doctor_tools.iter().filter(|t| t.installed).count();
    let tools_total = app.doctor_tools.len();
    let targets_ready = app.doctor_targets.iter().filter(|t| t.installed).count();
    let targets_total = app.doctor_targets.len();

    ui.horizontal(|ui| {
        theme::hero_title(ui, "Doctor");
        ui.add_space(14.0);
        if theme::underline_tab(
            ui,
            &format!("Tools ({tools_ready}/{tools_total})"),
            app.doctor_subtab == DoctorSubTab::Toolchains,
        )
        .clicked()
        {
            app.doctor_subtab = DoctorSubTab::Toolchains;
        }
        if theme::underline_tab(
            ui,
            &format!("Targets ({targets_ready}/{targets_total})"),
            app.doctor_subtab == DoctorSubTab::RustTargets,
        )
        .clicked()
        {
            app.doctor_subtab = DoctorSubTab::RustTargets;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icons::button(ui, Icon::Refresh, "", ButtonKind::Ghost, true)
                .on_hover_text("Re-scan toolchains & rustup targets")
                .clicked()
            {
                app.run_doctor();
            }
            if tools_total > 0 {
                let frac = tools_ready as f32 / tools_total as f32;
                let (bar_rect, _) =
                    ui.allocate_exact_size(egui::vec2(72.0, 4.0), egui::Sense::hover());
                ui.painter().rect_filled(bar_rect, 2.0, theme::BG_ELEVATED);
                let fill = egui::Rect::from_min_size(
                    bar_rect.min,
                    egui::vec2(bar_rect.width() * frac, bar_rect.height()),
                );
                ui.painter().rect_filled(fill, 2.0, theme::FG_PRIMARY);
            }
        });
    });

    theme::hairline(ui);
    ui.add_space(14.0);

    match app.doctor_subtab {
        DoctorSubTab::Toolchains => render_toolchains_split(ui, app),
        DoctorSubTab::RustTargets => render_rust_targets(ui, app),
    }
}

fn render_toolchains_split(ui: &mut egui::Ui, app: &mut RavomApp) {
    let avail = ui.available_size();
    let left_w = (avail.x * 0.34).clamp(260.0, 340.0);
    let busy = app.tasks.running().is_some();

    let mut clicked_tool: Option<String> = None;

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, avail.y.max(240.0)),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(left_w);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
                    for &cat in ToolCategory::ALL {
                        if theme::underline_tab(ui, cat.label(), app.doctor_category == cat)
                            .clicked()
                        {
                            app.doctor_category = cat;
                        }
                    }
                });
                theme::hairline(ui);

                let filtered: Vec<&ToolStatus> = app
                    .doctor_tools
                    .iter()
                    .filter(|t| {
                        app.doctor_category == ToolCategory::All
                            || t.category == app.doctor_category
                    })
                    .collect();

                egui::ScrollArea::vertical()
                    .id_salt("doctor_tools_list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(left_w);
                        ui.spacing_mut().item_spacing.y = 0.0;
                        if filtered.is_empty() {
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new("Empty category")
                                    .size(12.0)
                                    .color(theme::FG_FAINT),
                            );
                            return;
                        }
                        for tool in filtered {
                            let selected = app.selected_doctor_tool == tool.name;
                            if render_tool_list_row(ui, tool, selected, left_w) {
                                clicked_tool = Some(tool.name.to_string());
                            }
                        }
                    });
            },
        );

        ui.separator();

        let right_w = ui.available_width().max(260.0);
        egui::ScrollArea::vertical()
            .id_salt("doctor_tool_inspector")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(right_w);
                let selected_tool = app
                    .doctor_tools
                    .iter()
                    .find(|t| t.name == app.selected_doctor_tool)
                    .cloned();

                match selected_tool {
                    Some(tool) => render_tool_inspector(ui, app, &tool, busy),
                    None => {
                        ui.label(
                            egui::RichText::new("Select a tool")
                                .size(12.0)
                                .color(theme::FG_FAINT),
                        );
                    }
                }
            });
    });

    if let Some(name) = clicked_tool {
        app.select_doctor_tool(&name);
    }
}

fn render_tool_list_row(ui: &mut egui::Ui, tool: &ToolStatus, selected: bool, row_w: f32) -> bool {
    let row_h = 46.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(row_w, row_h), egui::Sense::click());
    let hover_t = theme::animate_spring(
        ui.ctx(),
        ui.make_persistent_id(("doctor_row_hover", tool.name)),
        resp.hovered(),
    );

    let bg = if selected {
        theme::BG_SURFACE
    } else {
        theme::lerp_color(theme::BG_CANVAS, theme::BG_SURFACE, hover_t)
    };

    ui.painter().rect_filled(rect, 0.0, bg);
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.bottom()),
            egui::pos2(rect.right(), rect.bottom()),
        ],
        egui::Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
    );

    if selected {
        let accent = egui::Rect::from_min_max(
            egui::pos2(rect.min.x, rect.min.y + 8.0),
            egui::pos2(rect.min.x + 2.0, rect.max.y - 8.0),
        );
        ui.painter()
            .rect_filled(accent, egui::CornerRadius::same(1), theme::FG_PRIMARY);
    }

    let inner = rect.shrink2(egui::vec2(12.0, 6.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    child.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.set_width(inner.width());
            let (badge_label, badge_color) = tool_badge_label(tool);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(badge_label)
                        .size(10.0)
                        .monospace()
                        .color(badge_color),
                );
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let dot_color = if tool.configured_path.is_some() {
                        theme::FG_PRIMARY
                    } else if tool.installed {
                        theme::FG_SECONDARY
                    } else {
                        theme::FG_FAINT
                    };
                    theme::status_dot(ui, dot_color);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(tool.name)
                                .font(egui::FontId::monospace(13.0))
                                .color(theme::FG_PRIMARY),
                        )
                        .truncate(),
                    );
                });
            });
        });

        let sub = if let Some(ver) = &tool.version {
            ver.clone()
        } else {
            tool.category.label().to_string()
        };
        ui.add(
            egui::Label::new(egui::RichText::new(sub).size(10.5).color(theme::FG_MUTED)).truncate(),
        );
    });

    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

fn tool_badge_label(tool: &ToolStatus) -> (&'static str, egui::Color32) {
    if tool.configured_path.is_some() {
        ("CUSTOM", theme::FG_PRIMARY)
    } else if tool.installed {
        ("READY", theme::FG_SECONDARY)
    } else {
        ("MISSING", theme::FG_MUTED)
    }
}

fn render_tool_inspector(ui: &mut egui::Ui, app: &mut RavomApp, tool: &ToolStatus, busy: bool) {
    // Hero Row directly on canvas
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(cmd) = &tool.auto_install_cmd {
                let label = if tool.installed { "Update" } else { "Install" };
                if icons::button(ui, Icon::Download, label, ButtonKind::Primary, !busy)
                    .on_hover_text(cmd)
                    .clicked()
                {
                    app.spawn_doctor_install_tool(tool.name);
                }
            } else {
                theme::copy_feedback_button(
                    ui,
                    &format!("doc_copy_url_{}", tool.name),
                    "Docs URL",
                    tool.docs_url,
                );
            }

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let (status_txt, status_col) = tool_badge_label(tool);
                ui.vertical(|ui| {
                    theme::eyebrow(ui, &format!("{} · {status_txt}", tool.category.label()));
                    ui.horizontal(|ui| {
                        let dot = if tool.configured_path.is_some() {
                            theme::FG_PRIMARY
                        } else if tool.installed {
                            theme::FG_SECONDARY
                        } else {
                            theme::FG_FAINT
                        };
                        theme::status_dot(ui, dot);
                        ui.label(
                            egui::RichText::new(tool.name)
                                .font(egui::FontId::monospace(20.0))
                                .strong()
                                .color(theme::FG_PRIMARY),
                        );
                        ui.label(
                            egui::RichText::new(status_txt)
                                .font(egui::FontId::monospace(10.5))
                                .color(status_col),
                        );
                    });
                });
            });
        });
    });

    ui.add_space(12.0);
    theme::hairline(ui);
    ui.add_space(12.0);

    let effective = tool
        .path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "—".to_string());
    let custom = tool
        .configured_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "PATH".to_string());
    let version = tool.version.clone().unwrap_or_else(|| "—".to_string());
    let config_file = tools_config_path().display().to_string();

    theme::kv_row(ui, "Binary", &effective, true);
    theme::kv_row(ui, "Override", &custom, true);
    theme::kv_row(ui, "Version", &version, false);
    theme::kv_row(ui, "Store", &config_file, true);

    if let Some(cmd) = &tool.auto_install_cmd {
        ui.add_space(10.0);
        theme::inset_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    theme::copy_feedback_button(
                        ui,
                        &format!("doc_copy_cmd_{}", tool.name),
                        "",
                        cmd,
                    );
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!("$ {cmd}"))
                                    .font(egui::FontId::monospace(11.5))
                                    .color(theme::FG_SECONDARY),
                            )
                            .truncate(),
                        );
                    });
                });
            });
        });
    }

    ui.add_space(16.0);
    theme::hairline(ui);
    ui.add_space(12.0);

    theme::eyebrow(ui, "Custom Binary Path");
    ui.add_space(8.0);

    let mut submit_enter = false;
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icons::button(
                ui,
                Icon::Trash,
                "",
                ButtonKind::Ghost,
                tool.configured_path.is_some(),
            )
            .on_hover_text("Reset override to system PATH")
            .clicked()
            {
                app.clear_doctor_tool_path();
            }
            let can_save = !app.doctor_path_draft.trim().is_empty();
            if icons::button(ui, Icon::Check, "Save", ButtonKind::Outline, can_save).clicked()
                || submit_enter
            {
                app.save_doctor_tool_path();
            }
            if icons::button(ui, Icon::Folder, "", ButtonKind::Ghost, true)
                .on_hover_text("Browse executable...")
                .clicked()
            {
                app.open_tool_file_dialog();
            }

            let input_w = ui.available_width().max(120.0);
            let resp = theme::text_input(
                ui,
                egui::Id::new(("doctor_path_draft", tool.name)),
                &mut app.doctor_path_draft,
                "/path/to/binary",
                input_w,
                false,
                false,
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                submit_enter = true;
            }
        });
    });

    if submit_enter {
        app.save_doctor_tool_path();
    }

    if !app.doctor_warnings.is_empty() {
        ui.add_space(14.0);
        egui::CollapsingHeader::new(
            egui::RichText::new(format!("Host Notes ({})", app.doctor_warnings.len()))
                .size(11.5)
                .color(theme::FG_SECONDARY),
        )
        .default_open(false)
        .show(ui, |ui| {
            for warning in &app.doctor_warnings {
                ui.label(
                    egui::RichText::new(format!("• {warning}"))
                        .size(11.5)
                        .color(theme::FG_MUTED),
                );
            }
        });
    }
}

fn render_rust_targets(ui: &mut egui::Ui, app: &mut RavomApp) {
    let busy = app.tasks.running().is_some();

    let mut add_clicked = false;
    let mut enter_pressed = false;
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_add = !busy && !app.custom_rust_target.trim().is_empty();
            if icons::button(ui, Icon::Download, "Add", ButtonKind::Primary, can_add).clicked() {
                add_clicked = true;
            }
            let input_w = ui.available_width().max(160.0);
            let resp = theme::text_input(
                ui,
                egui::Id::new("doctor_custom_rust_target"),
                &mut app.custom_rust_target,
                "Add rustup target (e.g. thumbv8m.main-none-eabihf)",
                input_w,
                false,
                false,
            );
            if can_add && resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                enter_pressed = true;
            }
        });
    });

    if add_clicked || enter_pressed {
        let triple = app.custom_rust_target.trim().to_string();
        if !triple.is_empty() {
            app.spawn_doctor_install_target(&triple);
            app.custom_rust_target.clear();
        }
    }

    ui.add_space(12.0);
    theme::hairline(ui);
    ui.add_space(8.0);

    let targets = app.doctor_targets.clone();
    let avail_w = ui.available_width().max(320.0);
    let cols = if avail_w >= 680.0 { 2 } else { 1 };
    let gap = 20.0;
    let col_w = ((avail_w - gap * (cols as f32 - 1.0)) / cols as f32).floor();

    egui::ScrollArea::vertical()
        .id_salt("doctor_rust_targets_grid")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if targets.is_empty() {
                ui.label(
                    egui::RichText::new("No targets")
                        .size(12.0)
                        .color(theme::FG_FAINT),
                );
                return;
            }

            for chunk in targets.chunks(cols) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for target in chunk {
                        ui.allocate_ui_with_layout(
                            egui::vec2(col_w, 72.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(col_w);
                                render_rust_target_row(ui, app, target, busy);
                            },
                        );
                    }
                });
                ui.add_space(4.0);
            }
        });
}

fn render_rust_target_row(
    ui: &mut egui::Ui,
    app: &mut RavomApp,
    target: &RustTargetStatus,
    busy: bool,
) {
    let row = egui::Frame::new()
        .fill(theme::BG_CANVAS)
        .inner_margin(egui::Margin::symmetric(4, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    theme::copy_feedback_button(
                        ui,
                        &format!("copy_triple_{}", target.triple),
                        "",
                        target.triple,
                    );
                    if target.installed {
                        if icons::button(ui, Icon::Trash, "", ButtonKind::Ghost, !busy)
                            .on_hover_text("Remove target")
                            .clicked()
                        {
                            app.spawn_doctor_remove_target(target.triple);
                        }
                    } else if icons::button(
                        ui,
                        Icon::Download,
                        "Install",
                        ButtonKind::Primary,
                        !busy,
                    )
                    .clicked()
                    {
                        app.spawn_doctor_install_target(target.triple);
                    }

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        theme::status_dot(
                            ui,
                            if target.installed {
                                theme::FG_PRIMARY
                            } else {
                                theme::FG_FAINT
                            },
                        );
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(target.triple)
                                    .font(egui::FontId::monospace(12.5))
                                    .color(theme::FG_PRIMARY),
                            )
                            .truncate(),
                        );
                    });
                });
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(target.label)
                        .size(11.5)
                        .color(theme::FG_SECONDARY),
                );
                ui.label(
                    egui::RichText::new(format!("· {}", target.boards_hint))
                        .size(11.0)
                        .color(theme::FG_MUTED),
                );
            });
        });

    let r = row.response.rect;
    ui.painter().line_segment(
        [
            egui::pos2(r.left(), r.bottom()),
            egui::pos2(r.right(), r.bottom()),
        ],
        egui::Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
    );
}

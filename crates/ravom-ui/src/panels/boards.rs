use crate::app::{NavTab, RavomApp};
use crate::board_index::BoardFilter;
use crate::icons::{self, ButtonKind, Icon};
use crate::theme;
use eframe::egui::{self, Layout, Margin, RichText, Sense, Stroke};

const MAX_FILTERED_BOARDS: usize = 250;

/// Render the Master-Detail Board Catalog & Swiss Target Specification Sheet.
pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    app.boards.ensure_query(&app.board_query, app.board_filter);
    let total_matches = app.boards.cached_total();
    let has_query = !app.boards.cached_terms().is_empty() || app.board_filter != BoardFilter::All;
    let no_matches = has_query && total_matches == 0;

    let search_id = ui.make_persistent_id("board_search_input");
    if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
        ui.memory_mut(|m| m.request_focus(search_id));
    }

    let full_h = ui.available_height();
    let left_w = (ui.available_width() * 0.35).clamp(280.0, 360.0);

    ui.horizontal_top(|ui| {
        // Left Master List: Search + Ecosystem Underline Filter + Hairline Board Tree
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, full_h),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.horizontal(|ui| {
                    theme::hero_title(ui, "Boards");
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        let count_txt = if has_query {
                            format!("{total_matches} / {}", app.boards.len())
                        } else {
                            app.boards.len().to_string()
                        };
                        ui.label(
                            RichText::new(count_txt)
                                .font(egui::FontId::monospace(10.5))
                                .color(if no_matches {
                                    theme::FG_PRIMARY
                                } else {
                                    theme::FG_MUTED
                                }),
                        );
                    });
                });
                ui.add_space(8.0);

                // Search bar
                let search_w = ui.available_width();
                theme::text_input(
                    ui,
                    search_id,
                    &mut app.board_query,
                    "Search vendor, board, MCU... (Cmd+F)",
                    search_w,
                    true,
                    no_matches,
                );

                ui.add_space(6.0);

                // Ecosystem filter underline tabs
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 2.0);
                    for &flt in BoardFilter::ALL {
                        let active = app.board_filter == flt;
                        if theme::underline_tab(ui, flt.label(), active).clicked() {
                            app.board_filter = flt;
                        }
                    }
                });
                theme::hairline(ui);

                let list_h = (ui.available_height() - 4.0).max(160.0);
                let terms = app.boards.cached_terms().to_vec();

                egui::ScrollArea::vertical()
                    .id_salt("boards_scroll")
                    .max_height(list_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if app.boards.cached_groups().is_empty() {
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new("No matching boards for current filter.")
                                    .size(12.0)
                                    .color(theme::DANGER_FG),
                            );
                            return;
                        }

                        let mut picked = None;
                        let mut shown = 0usize;

                        for group in app.boards.cached_groups() {
                            if has_query && shown >= MAX_FILTERED_BOARDS {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Refine search to view more boards...")
                                        .size(11.0)
                                        .color(theme::FG_MUTED),
                                );
                                break;
                            }

                            let contains_selected = group
                                .indices
                                .iter()
                                .any(|&i| app.boards.get(i).qualified_id() == app.selected_board);
                            let header_text =
                                format!("{}  ({})", group.vendor, group.indices.len());
                            let header_job = theme::highlighted_job(
                                &header_text,
                                &terms,
                                12.0,
                                theme::FG_SECONDARY,
                                contains_selected,
                            );

                            let mut header = egui::CollapsingHeader::new(header_job)
                                .id_salt(("vendor_group", group.vendor.as_str()))
                                .default_open(contains_selected);

                            if has_query {
                                header = header.open(Some(true));
                            }

                            header.show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                for &idx in &group.indices {
                                    if has_query && shown >= MAX_FILTERED_BOARDS {
                                        break;
                                    }
                                    shown += 1;

                                    let b = app.boards.get(idx);
                                    let qualified = b.qualified_id();
                                    let selected = app.selected_board == qualified;
                                    let title = if b.name.is_empty() { &b.id } else { &b.name };
                                    let mut meta = format!("{} · {}", b.id, b.mcu);
                                    if b.probe_rs_chip.is_some() {
                                        meta.push_str(" · probe-rs");
                                    }

                                    let card_id = ui.make_persistent_id(("board_card", &qualified));
                                    let prev_hov = ui.ctx().data(|d| {
                                        d.get_temp::<bool>(card_id.with("is_hov")).unwrap_or(false)
                                    });
                                    let hover_t = theme::animate_spring(
                                        ui.ctx(),
                                        card_id.with("hov_spring"),
                                        prev_hov,
                                    );
                                    let sel_t = theme::animate_spring(
                                        ui.ctx(),
                                        card_id.with("sel_spring"),
                                        selected,
                                    );

                                    let base_fill = theme::lerp_color(
                                        theme::BG_CANVAS,
                                        theme::BG_SURFACE,
                                        hover_t,
                                    );
                                    let fill =
                                        theme::lerp_color(base_fill, theme::BG_SURFACE, sel_t);

                                    let title_job = theme::highlighted_job(
                                        title,
                                        &terms,
                                        12.0,
                                        theme::FG_PRIMARY,
                                        selected,
                                    );
                                    let meta_job = theme::highlighted_job(
                                        &meta,
                                        &terms,
                                        10.5,
                                        theme::FG_MUTED,
                                        selected,
                                    );

                                    let card = egui::Frame::new()
                                        .fill(fill)
                                        .inner_margin(Margin::symmetric(12, 8))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.add(egui::Label::new(title_job).truncate());
                                            ui.add(egui::Label::new(meta_job).truncate());
                                        });

                                    let r = card.response.rect;
                                    ui.painter().line_segment(
                                        [
                                            egui::pos2(r.left(), r.bottom()),
                                            egui::pos2(r.right(), r.bottom()),
                                        ],
                                        Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
                                    );
                                    if sel_t > 0.01 {
                                        let bar_h = (r.height() - 10.0).max(4.0) * sel_t;
                                        let bar = egui::Rect::from_center_size(
                                            egui::pos2(r.left() + 1.0, r.center().y),
                                            egui::vec2(2.0, bar_h),
                                        );
                                        ui.painter().rect_filled(bar, 1.0, theme::FG_PRIMARY);
                                    }

                                    let resp = card
                                        .response
                                        .interact(Sense::click())
                                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                                    let now_hov = resp.hovered();
                                    if now_hov != prev_hov {
                                        ui.ctx().data_mut(|d| {
                                            d.insert_temp(card_id.with("is_hov"), now_hov)
                                        });
                                        ui.ctx().request_repaint();
                                    }
                                    if resp.clicked() {
                                        picked = Some(qualified);
                                    }
                                }
                            });
                        }

                        if let Some(q) = picked {
                            app.selected_board = q;
                        }
                    });
            },
        );

        ui.separator();

        // Right Detail Pane: Open Canvas Specification Sheet
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), full_h),
            Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("board_detail_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let Some(board) =
                            app.boards.find_by_qualified(&app.selected_board).cloned()
                        else {
                            ui.label(
                                RichText::new("Select a board")
                                    .size(12.5)
                                    .color(theme::FG_FAINT),
                            );
                            return;
                        };

                        ui.horizontal(|ui| {
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                if let Some(chip) = &board.probe_rs_chip {
                                    ui.label(
                                        RichText::new(format!("probe-rs · {chip}"))
                                            .font(egui::FontId::monospace(10.5))
                                            .color(theme::FG_PRIMARY),
                                    );
                                }
                                if let Some(v) = &board.vendor {
                                    ui.label(RichText::new(v).size(11.0).color(theme::FG_MUTED));
                                }
                                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                                    ui.vertical(|ui| {
                                        theme::eyebrow(ui, &board.platform);
                                        theme::hero_title(
                                            ui,
                                            if board.name.is_empty() {
                                                &board.id
                                            } else {
                                                &board.name
                                            },
                                        );
                                        ui.label(
                                            RichText::new(board.qualified_id())
                                                .size(11.5)
                                                .monospace()
                                                .color(theme::FG_SECONDARY),
                                        );
                                    });
                                });
                            });
                        });

                        ui.add_space(14.0);
                        theme::hairline(ui);
                        ui.add_space(10.0);

                        // Swiss hairline 4-column specification ledger
                        let flash_str = board
                            .flash_bytes
                            .map(format_bytes_compact)
                            .unwrap_or_else(|| "—".to_string());
                        let ram_str = board
                            .ram_bytes
                            .map(format_bytes_compact)
                            .unwrap_or_else(|| "—".to_string());
                        let target_str = board
                            .rust_target
                            .as_deref()
                            .unwrap_or("non-Cargo")
                            .to_string();

                        let avail_w = ui.available_width().max(220.0);
                        let cols = if avail_w >= 500.0 { 4 } else { 2 };
                        let div_w = 10.0_f32;
                        let cell_w =
                            ((avail_w - div_w * (cols as f32 - 1.0)) / cols as f32).floor();
                        let specs = [
                            ("MCU", board.mcu.as_str(), true),
                            ("Flash", flash_str.as_str(), false),
                            ("SRAM", ram_str.as_str(), false),
                            ("Target", target_str.as_str(), false),
                        ];
                        for chunk in specs.chunks(cols) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                for (i, (lbl, val, accent)) in chunk.iter().enumerate() {
                                    if i > 0 {
                                        theme::v_divider(ui, 38.0);
                                    }
                                    let _ = theme::ledger_cell(ui, lbl, val, *accent, cell_w);
                                }
                            });
                            ui.add_space(6.0);
                        }

                        ui.add_space(6.0);
                        theme::hairline(ui);
                        ui.add_space(12.0);

                        theme::kv_row(ui, "Platform", &board.platform, true);
                        if let Some(cpu) = &board.cpu {
                            theme::kv_row(ui, "CPU", cpu, true);
                        }
                        if let Some(z) = &board.zephyr_board {
                            theme::kv_row(ui, "Zephyr", z, true);
                        }
                        theme::kv_row(ui, "Default FW", &board.default_framework, true);

                        if !board.frameworks.is_empty() {
                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                theme::field_label(ui, 116.0, 20.0, "Frameworks");
                                ui.label(
                                    RichText::new(board.frameworks.join(" · "))
                                        .size(11.5)
                                        .monospace()
                                        .color(theme::FG_SECONDARY),
                                );
                            });
                        }

                        ui.add_space(16.0);
                        theme::hairline(ui);
                        ui.add_space(14.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().interact_size.y = 30.0;
                            theme::field_label(ui, 72.0, 30.0, "Framework");
                            let fw_label = match app.init_framework.as_str() {
                                "" => "Default",
                                "zephyr" => "Zephyr (C)",
                                "zephyr-rust" => "Zephyr (Rust)",
                                "embassy" => "Embassy",
                                "bare-metal" => "Bare-Metal Rust",
                                "mirui" => "Mirui",
                                other => other,
                            };
                            egui::ComboBox::from_id_salt("board_detail_framework")
                                .selected_text(fw_label)
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut app.init_framework,
                                        String::new(),
                                        "Default",
                                    );
                                    ui.selectable_value(
                                        &mut app.init_framework,
                                        "bare-metal".to_string(),
                                        "Bare-Metal Rust",
                                    );
                                    ui.selectable_value(
                                        &mut app.init_framework,
                                        "embassy".to_string(),
                                        "Embassy",
                                    );
                                    ui.selectable_value(
                                        &mut app.init_framework,
                                        "mirui".to_string(),
                                        "Mirui",
                                    );
                                    ui.selectable_value(
                                        &mut app.init_framework,
                                        "zephyr".to_string(),
                                        "Zephyr (C)",
                                    );
                                    ui.selectable_value(
                                        &mut app.init_framework,
                                        "zephyr-rust".to_string(),
                                        "Zephyr (Rust)",
                                    );
                                });

                            if icons::button(ui, Icon::Plus, "Init", ButtonKind::Primary, true)
                                .on_hover_text(format!(
                                    "Initialize project at {}",
                                    app.project_path().display()
                                ))
                                .clicked()
                            {
                                let fw = (!app.init_framework.is_empty())
                                    .then_some(app.init_framework.as_str())
                                    .map(str::to_string);
                                let qid = board.qualified_id();
                                app.run_init_here(&qid, fw.as_deref());
                                app.active_tab = NavTab::Workspace;
                            }

                            if icons::button(ui, Icon::Check, "Use", ButtonKind::Outline, true)
                                .on_hover_text("Use this board in Workspace")
                                .clicked()
                            {
                                app.active_tab = NavTab::Workspace;
                            }
                        });
                    });
            },
        );
    });
}

fn format_bytes_compact(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{} KB", bytes / 1024)
    } else {
        format!("{bytes} B")
    }
}

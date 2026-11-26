use crate::app::RavomApp;
use crate::theme;
use eframe::egui::{self, CornerRadius, Margin, RichText, Sense, Stroke};
use ravom_core::monitor::list_serial_port_details;

const MAX_FILTERED_BOARDS: usize = 200;
const SERIAL_FOOTER_HEIGHT: f32 = 128.0;

pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    app.boards.ensure_query(&app.board_query);
    let total_matches = app.boards.cached_total();
    let has_query = !app.boards.cached_terms().is_empty();
    let no_matches = has_query && total_matches == 0;

    // Ctrl+F focuses the board search box (Firefox-style).
    let search_id = ui.make_persistent_id("board_search_input");
    if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
        ui.memory_mut(|m| m.request_focus(search_id));
    }

    // Sidebar Header
    ui.horizontal(|ui| {
        theme::section_title(ui, "Boards");
        if has_query {
            let color = if no_matches {
                theme::DANGER_FG
            } else {
                theme::MATCH_BG
            };
            theme::badge(ui, format!("{total_matches} / {}", app.boards.len()), color);
        } else {
            theme::badge(ui, app.boards.len().to_string(), theme::FG_MUTED);
        }
    });
    ui.add_space(4.0);

    // Firefox-style search bar with smooth focus/error border interpolation.
    let search_focused = ui.memory(|m| m.has_focus(search_id));
    let focus_t = theme::animate_spring(ui.ctx(), search_id.with("focus"), search_focused);
    let search_border = if no_matches {
        theme::DANGER_BORDER
    } else {
        theme::lerp_color(theme::BORDER, theme::BORDER_FOCUS, focus_t)
    };

    egui::Frame::new()
        .fill(theme::BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, search_border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let clear_w = if has_query { 22.0 } else { 0.0 };
                let edit_w = (ui.available_width() - clear_w).max(80.0);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut app.board_query)
                        .id(search_id)
                        .frame(false)
                        .hint_text("Search vendor, board, MCU... (Ctrl+F)")
                        .desired_width(edit_w),
                );
                if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    app.board_query.clear();
                }
                if has_query
                    && ui
                        .add(
                            egui::Button::new(RichText::new("×").size(13.0).color(theme::FG_MUTED))
                                .frame(false),
                        )
                        .on_hover_text("Clear search (Esc)")
                        .clicked()
                {
                    app.board_query.clear();
                }
            });
        });

    ui.add_space(4.0);

    theme::inset_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new("Selected").size(11.0).color(theme::FG_MUTED));
            ui.add(
                egui::Label::new(
                    RichText::new(&app.selected_board)
                        .size(11.5)
                        .monospace()
                        .color(theme::FG_PRIMARY),
                )
                .truncate(),
            );
        });
    });

    ui.add_space(6.0);

    // Dynamic height so the board explorer fills the sidebar cleanly above Serial Ports.
    let list_height = (ui.available_height() - SERIAL_FOOTER_HEIGHT).max(160.0);
    let terms = app.boards.cached_terms().to_vec();

    egui::ScrollArea::vertical()
        .id_salt("boards_scroll")
        .max_height(list_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if app.boards.cached_groups().is_empty() {
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Phrase not found — no matching boards.")
                        .size(12.0)
                        .color(theme::DANGER_FG),
                );
                return;
            }

            let mut picked = None;
            let mut shown_boards = 0usize;

            for group in app.boards.cached_groups() {
                if has_query && shown_boards >= MAX_FILTERED_BOARDS {
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
                let header_text = format!("{}  ({})", group.vendor, group.indices.len());
                let header_job = theme::highlighted_job(
                    &header_text,
                    &terms,
                    12.5,
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
                    ui.spacing_mut().item_spacing.y = 4.0;
                    for &idx in &group.indices {
                        if has_query && shown_boards >= MAX_FILTERED_BOARDS {
                            break;
                        }
                        shown_boards += 1;

                        let b = app.boards.get(idx);
                        let qualified = b.qualified_id();
                        let selected = app.selected_board == qualified;
                        let title = if b.name.is_empty() { &b.id } else { &b.name };
                        let mut meta = format!("{} · {}", b.id, b.mcu);
                        if b.probe_rs_chip.is_some() {
                            meta.push_str(" · probe-rs");
                        }

                        let card_id = ui.make_persistent_id(("board_card", &qualified));
                        let prev_hovered = ui
                            .ctx()
                            .data(|d| d.get_temp::<bool>(card_id.with("is_hov")).unwrap_or(false));
                        let hover_t = theme::animate_spring(
                            ui.ctx(),
                            card_id.with("hov_spring"),
                            prev_hovered,
                        );
                        let sel_t =
                            theme::animate_spring(ui.ctx(), card_id.with("sel_spring"), selected);

                        let base_fill =
                            theme::lerp_color(theme::BG_SURFACE, theme::BG_MUTED, hover_t);
                        let fill = theme::lerp_color(base_fill, theme::BG_HOVER, sel_t);
                        let base_stroke =
                            theme::lerp_color(theme::BORDER, theme::BORDER_FOCUS, hover_t * 0.6);
                        let stroke_color =
                            theme::lerp_color(base_stroke, theme::BORDER_FOCUS, sel_t);

                        let title_job = theme::highlighted_job(
                            title,
                            &terms,
                            12.0,
                            theme::FG_PRIMARY,
                            selected,
                        );
                        let meta_job =
                            theme::highlighted_job(&meta, &terms, 10.5, theme::FG_MUTED, selected);

                        let card = egui::Frame::new()
                            .fill(fill)
                            .stroke(Stroke::new(1.0_f32, stroke_color))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(9, 6))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.add(egui::Label::new(title_job).truncate());
                                ui.add(egui::Label::new(meta_job).truncate());
                            });

                        // Subtle spring-animated left accent bar on selected board.
                        if sel_t > 0.01 {
                            let r = card.response.rect;
                            let bar_h = (r.height() - 10.0).max(4.0) * sel_t;
                            let bar = egui::Rect::from_center_size(
                                egui::pos2(r.left() + 2.5, r.center().y),
                                egui::vec2(2.5, bar_h),
                            );
                            ui.painter().rect_filled(bar, 1.5, theme::ACCENT_OK);
                        }

                        let resp = card
                            .response
                            .interact(Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(format!(
                                "{title}\n{} | {} | MCU {}",
                                qualified, b.platform, b.mcu
                            ));
                        let now_hovered = resp.hovered();
                        if now_hovered != prev_hovered {
                            ui.ctx()
                                .data_mut(|d| d.insert_temp(card_id.with("is_hov"), now_hovered));
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

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    // Serial Ports Footer
    ui.horizontal(|ui| {
        theme::section_title(ui, "Serial Ports");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(theme::secondary_button("Refresh")).clicked() {
                app.serial_ports = list_serial_port_details();
            }
        });
    });
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .id_salt("serial_ports_scroll")
        .max_height(84.0)
        .show(ui, |ui| {
            if app.serial_ports.is_empty() {
                ui.label(
                    RichText::new("No serial ports detected.")
                        .size(11.5)
                        .color(theme::FG_MUTED),
                );
            } else {
                for p in &app.serial_ports {
                    ui.label(
                        RichText::new(format!("{}  {}", p.path, p.description))
                            .size(11.5)
                            .monospace()
                            .color(theme::FG_SECONDARY),
                    );
                }
            }
        });
}

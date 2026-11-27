//! Monochrome Rikyu-styled Serial Studio: `FireWater` / `JustFloat` / `RawData` engines,
//! multi-channel oscilloscope, raw/HEX terminal, telemetry gauges, and collapsible macros.

use crate::app::{MonitorViewMode, RavomApp};
use crate::icons::{self, ButtonKind, Icon};
use crate::theme;
use eframe::egui::{self, Layout, Margin, RichText, Stroke};
use ravom_core::vofa::{
    format_command_template, LineEnding, SerialMacro, SerialSlider, VofaProtocol,
};

const BAUD_RATES: &[u32] = &[
    9_600, 19_200, 38_400, 57_600, 115_200, 230_400, 460_800, 921_600, 1_500_000, 2_000_000,
];

/// Render the open-canvas, Rikyu-styled VOFA+ Serial Studio view.
pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    let connected = app.serial_session.is_some();
    let ports = app
        .visible_serial_ports()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();

    // Top Studio Header: display title + compact tabular counters + Controls toggle.
    ui.horizontal(|ui| {
        theme::hero_title(ui, "Serial Studio");
        ui.add_space(10.0);
        if connected {
            theme::status_dot(ui, theme::FG_PRIMARY);
        } else {
            theme::status_dot(ui, theme::FG_FAINT);
        }
        let mut stats = format!("{} · {}", app.serial_baud, app.serial_protocol.label());
        if app.serial_fps > 0.05 {
            stats.push_str(&format!(" · {:.1} Hz", app.serial_fps));
        }
        stats.push_str(&format!(
            " · RX {} · TX {}",
            format_byte_count(app.serial_rx_bytes),
            format_byte_count(app.serial_tx_bytes)
        ));
        ui.label(
            RichText::new(stats)
                .font(egui::FontId::monospace(10.5))
                .color(if connected {
                    theme::FG_SECONDARY
                } else {
                    theme::FG_MUTED
                }),
        );

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if theme::underline_tab(ui, "Controls", app.monitor_controls_open).clicked() {
                app.monitor_controls_open = !app.monitor_controls_open;
            }
        });
    });

    ui.add_space(10.0);
    theme::hairline(ui);
    ui.add_space(10.0);

    // Flat Hairline Connection & Engine Strip directly on canvas
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().interact_size.y = 28.0;
        for &proto in VofaProtocol::ALL {
            if theme::underline_tab(ui, proto.label(), app.serial_protocol == proto)
                .on_hover_text(proto.hint())
                .clicked()
            {
                app.set_serial_protocol(proto);
            }
        }

        theme::v_divider(ui, 18.0);

        let selected_label = if app.selected_port.is_empty() {
            "Port...".to_string()
        } else {
            ports
                .iter()
                .find(|p| p.path == app.selected_port)
                .map(|p| format!("{} ({})", p.path, p.description))
                .unwrap_or_else(|| app.selected_port.clone())
        };

        egui::ComboBox::from_id_salt("serial_port_select")
            .selected_text(selected_label)
            .width(230.0)
            .show_ui(ui, |ui| {
                if ports.is_empty() {
                    ui.label(
                        RichText::new("No USB ports")
                            .size(11.5)
                            .color(theme::FG_MUTED),
                    );
                } else {
                    for p in &ports {
                        let label = format!("{}  —  {}", p.path, p.description);
                        ui.selectable_value(&mut app.selected_port, p.path.clone(), label);
                    }
                }
            });

        egui::ComboBox::from_id_salt("serial_baud_select")
            .selected_text(format!("{}", app.serial_baud))
            .width(96.0)
            .show_ui(ui, |ui| {
                for &b in BAUD_RATES {
                    ui.selectable_value(&mut app.serial_baud, b, format!("{b}"));
                }
            });

        if icons::button(ui, Icon::Refresh, "", ButtonKind::Ghost, true)
            .on_hover_text("Refresh serial ports")
            .clicked()
        {
            app.refresh_ports();
        }

        if connected {
            if icons::button(ui, Icon::Plug, "Close", ButtonKind::Outline, true).clicked() {
                app.disconnect_serial();
            }
        } else if icons::button(
            ui,
            Icon::Plug,
            "Connect",
            ButtonKind::Primary,
            !app.selected_port.is_empty(),
        )
        .clicked()
        {
            app.connect_serial();
        }

        if icons::button(ui, Icon::Zap, "", ButtonKind::Ghost, connected)
            .on_hover_text("Pulse RTS Reset")
            .clicked()
        {
            app.reset_serial_target();
        }
    });

    ui.add_space(6.0);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().interact_size.y = 26.0;
        for (mode, label) in [
            (MonitorViewMode::Split, "Split"),
            (MonitorViewMode::Oscilloscope, "Scope"),
            (MonitorViewMode::Terminal, "Raw"),
            (MonitorViewMode::Widgets, "Deck"),
        ] {
            if theme::underline_tab(ui, label, app.monitor_view == mode).clicked() {
                app.monitor_view = mode;
            }
        }

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(&mut app.show_all_ports, "All");
            let rts_changed = ui.checkbox(&mut app.serial_rts, "RTS").changed();
            let dtr_changed = ui.checkbox(&mut app.serial_dtr, "DTR").changed();
            if dtr_changed || rts_changed {
                app.apply_serial_lines();
            }
        });
    });

    ui.add_space(8.0);
    theme::hairline(ui);
    ui.add_space(12.0);

    // Main Stage: Center Workspace + Collapsible 276px Right Controls Sidebar
    let avail = ui.available_size();
    let show_sidebar = app.monitor_controls_open && avail.x >= 760.0;
    let side_w = 276.0_f32;
    let gap = 16.0_f32;
    let center_w = if show_sidebar {
        (avail.x - side_w - gap).max(320.0)
    } else {
        avail.x.max(320.0)
    };
    let stage_h = avail.y.max(240.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(center_w, stage_h),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(center_w);
                match app.monitor_view {
                    MonitorViewMode::Split => {
                        let scope_h = ((stage_h - 175.0) * 0.48).clamp(130.0, 320.0);
                        if app.plotter.show_full(ui, scope_h) {
                            app.export_plotter_csv();
                        }
                        ui.add_space(10.0);
                        theme::hairline(ui);
                        ui.add_space(10.0);
                        let term_h = (ui.available_height() - 120.0).clamp(96.0, 320.0);
                        render_terminal_stage(ui, app, connected, term_h);
                    }
                    MonitorViewMode::Oscilloscope => {
                        let scope_h = (stage_h - 210.0).clamp(190.0, 480.0);
                        if app.plotter.show_full(ui, scope_h) {
                            app.export_plotter_csv();
                        }
                        ui.add_space(12.0);
                        theme::hairline(ui);
                        ui.add_space(10.0);
                        render_channel_stats_section(ui, app);
                    }
                    MonitorViewMode::Terminal => {
                        let term_h = (stage_h - 120.0).clamp(160.0, 580.0);
                        render_terminal_stage(ui, app, connected, term_h);
                    }
                    MonitorViewMode::Widgets => {
                        render_widgets_stage(ui, app, connected);
                    }
                }
            },
        );

        if show_sidebar {
            ui.separator();
            ui.allocate_ui_with_layout(
                egui::vec2(side_w, stage_h),
                Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(side_w);
                    render_controls_sidebar(ui, app, connected);
                },
            );
        }
    });
}

fn render_terminal_stage(ui: &mut egui::Ui, app: &mut RavomApp, connected: bool, term_h: f32) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().interact_size.y = 26.0;
        theme::text_input(
            ui,
            egui::Id::new("monitor_rx_filter"),
            &mut app.serial_filter,
            "Filter...",
            165.0,
            true,
            false,
        );
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if icons::button(ui, Icon::Trash, "", ButtonKind::Ghost, true)
                .on_hover_text("Clear terminal")
                .clicked()
            {
                app.serial_log.clear();
            }
            theme::copy_feedback_button(
                ui,
                "copy_serial_log",
                "",
                &app.serial_log.lines().collect::<Vec<_>>().join("\n"),
            );
            ui.checkbox(&mut app.serial_auto_scroll, "Tail");
            if theme::underline_tab(ui, "Pause", app.serial_pause_rx).clicked() {
                app.serial_pause_rx = !app.serial_pause_rx;
            }
            if theme::underline_tab(ui, "Time", app.serial_timestamps).clicked() {
                app.serial_timestamps = !app.serial_timestamps;
            }
            if theme::underline_tab(ui, "HEX", app.serial_hex_rx).clicked() {
                app.serial_hex_rx = !app.serial_hex_rx;
            }
        });
    });

    ui.add_space(6.0);

    let filter_term = app.serial_filter.trim().to_ascii_lowercase();
    theme::inset_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::ScrollArea::vertical()
            .id_salt("serial_studio_log_scroll")
            .stick_to_bottom(app.serial_auto_scroll)
            .max_height(term_h)
            .min_scrolled_height(term_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if app.serial_log.lines().next().is_none() {
                    ui.label(
                        RichText::new("Waiting for serial data...")
                            .size(12.0)
                            .color(theme::FG_FAINT),
                    );
                } else {
                    for line in app.serial_log.lines() {
                        if !filter_term.is_empty()
                            && !line.to_ascii_lowercase().contains(&filter_term)
                        {
                            continue;
                        }
                        let color = if line.starts_with("> ") || line.contains("] > ") {
                            theme::FG_PRIMARY
                        } else if line.starts_with("---") || line.contains("[JF]") {
                            theme::FG_MUTED
                        } else {
                            theme::FG_SECONDARY
                        };
                        ui.label(RichText::new(line).size(11.5).monospace().color(color));
                    }
                }
            });
    });

    ui.add_space(8.0);

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 30.0),
        Layout::right_to_left(egui::Align::Center),
        |ui| {
            let send_clicked =
                icons::button(ui, Icon::Send, "Send", ButtonKind::Primary, connected).clicked();
            let input_w = ui.available_width().max(120.0);
            let tx_id = ui.make_persistent_id("monitor_serial_tx_input");
            let hint = if app.serial_hex_tx {
                "HEX bytes (01 AA 80 7F)..."
            } else {
                "Command (Enter to send, Up/Down history)..."
            };
            let tx_resp = theme::text_input(
                ui,
                tx_id,
                &mut app.serial_tx_input,
                hint,
                input_w,
                false,
                false,
            );

            if tx_resp.has_focus() && !app.serial_tx_history.is_empty() {
                let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
                let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
                if up {
                    let next_idx = match app.serial_tx_history_idx {
                        Some(idx) => idx.saturating_sub(1),
                        None => app.serial_tx_history.len().saturating_sub(1),
                    };
                    app.serial_tx_history_idx = Some(next_idx);
                    if let Some(item) = app.serial_tx_history.get(next_idx) {
                        app.serial_tx_input = item.clone();
                    }
                } else if down {
                    if let Some(idx) = app.serial_tx_history_idx {
                        if idx + 1 < app.serial_tx_history.len() {
                            let next_idx = idx + 1;
                            app.serial_tx_history_idx = Some(next_idx);
                            if let Some(item) = app.serial_tx_history.get(next_idx) {
                                app.serial_tx_input = item.clone();
                            }
                        } else {
                            app.serial_tx_history_idx = None;
                            app.serial_tx_input.clear();
                        }
                    }
                }
            }

            let submit = tx_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if submit || send_clicked {
                let cmd = app.serial_tx_input.clone();
                if !app.serial_auto_send {
                    app.serial_tx_input.clear();
                }
                app.send_serial(&cmd);
                tx_resp.request_focus();
            }
        },
    );

    ui.add_space(6.0);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 2.0);
        if theme::underline_tab(ui, "ASCII", !app.serial_hex_tx).clicked() {
            app.serial_hex_tx = false;
        }
        if theme::underline_tab(ui, "HEX", app.serial_hex_tx).clicked() {
            app.serial_hex_tx = true;
        }

        theme::v_divider(ui, 16.0);

        for &le in LineEnding::ALL {
            let active = !app.serial_hex_tx && app.serial_line_ending == le;
            let resp = ui.add_enabled(
                !app.serial_hex_tx,
                egui::Button::new(RichText::new(le.label()).size(11.5).color(if active {
                    theme::FG_PRIMARY
                } else if app.serial_hex_tx {
                    theme::FG_FAINT
                } else {
                    theme::FG_MUTED
                }))
                .frame(false),
            );
            if resp.clicked() {
                app.serial_line_ending = le;
            }
        }

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_enabled(
                connected || app.serial_auto_send,
                egui::DragValue::new(&mut app.serial_auto_send_ms)
                    .range(50..=10_000)
                    .speed(25)
                    .suffix(" ms"),
            );
            ui.add_enabled(
                connected,
                egui::Checkbox::new(&mut app.serial_auto_send, "Loop"),
            );
        });
    });
}

fn render_channel_stats_section(ui: &mut egui::Ui, app: &RavomApp) {
    theme::eyebrow(ui, "Channels");
    ui.add_space(8.0);

    if app.plotter.channels.is_empty() {
        ui.label(
            RichText::new("No channels")
                .size(12.0)
                .color(theme::FG_FAINT),
        );
        return;
    }

    let max_h = ui.available_height().max(90.0);
    egui::ScrollArea::vertical()
        .id_salt("monitor_channel_stats")
        .max_height(max_h)
        .show(ui, |ui| {
            egui::Grid::new("monitor_channel_stats_grid")
                .num_columns(5)
                .spacing([22.0, 7.0])
                .show(ui, |ui| {
                    for hdr in ["Channel", "Latest", "Min", "Max", "Samples"] {
                        ui.label(RichText::new(hdr).size(11.0).color(theme::FG_MUTED));
                    }
                    ui.end_row();

                    for ch in &app.plotter.channels {
                        ui.horizontal(|ui| {
                            theme::status_dot(ui, ch.color);
                            ui.label(
                                RichText::new(&ch.name)
                                    .size(12.0)
                                    .monospace()
                                    .color(theme::FG_PRIMARY),
                            );
                        });
                        let latest = ch
                            .latest()
                            .map(|v| format!("{v:.3}"))
                            .unwrap_or_else(|| "—".to_string());
                        let min_s = if ch.min_seen.is_finite() {
                            format!("{:.3}", ch.min_seen)
                        } else {
                            "—".to_string()
                        };
                        let max_s = if ch.max_seen.is_finite() {
                            format!("{:.3}", ch.max_seen)
                        } else {
                            "—".to_string()
                        };
                        ui.label(
                            RichText::new(latest)
                                .size(12.0)
                                .monospace()
                                .color(theme::FG_PRIMARY),
                        );
                        ui.label(
                            RichText::new(min_s)
                                .size(12.0)
                                .monospace()
                                .color(theme::FG_SECONDARY),
                        );
                        ui.label(
                            RichText::new(max_s)
                                .size(12.0)
                                .monospace()
                                .color(theme::FG_SECONDARY),
                        );
                        ui.label(
                            RichText::new(ch.samples.len().to_string())
                                .size(12.0)
                                .monospace()
                                .color(theme::FG_MUTED),
                        );
                        ui.end_row();
                    }
                });
        });
}

fn render_widgets_stage(ui: &mut egui::Ui, app: &mut RavomApp, connected: bool) {
    let mut pending_sends: Vec<(String, bool)> = Vec::new();
    let mut sliders_dirty = false;

    egui::ScrollArea::vertical()
        .id_salt("monitor_widgets_stage")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Section 1: Live Digital Channel Gauges (flat ledger tiles on canvas)
            ui.horizontal(|ui| {
                theme::eyebrow(ui, "Gauges");
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut app.plotter.simulate, "Sim 3ch");
                });
            });
            ui.add_space(6.0);
            theme::hairline(ui);
            ui.add_space(10.0);

            if app.plotter.channels.is_empty() {
                ui.label(
                    RichText::new("Enable Sim or stream telemetry")
                        .size(12.0)
                        .color(theme::FG_FAINT),
                );
            } else {
                let avail_w = ui.available_width().max(260.0);
                let cols = if avail_w >= 620.0 {
                    3
                } else if avail_w >= 400.0 {
                    2
                } else {
                    1
                };
                let gap = 16.0_f32;
                let card_w = ((avail_w - gap * (cols as f32 - 1.0)) / cols as f32).floor();

                for chunk in app.plotter.channels.chunks(cols) {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for ch in chunk {
                            ui.allocate_ui_with_layout(
                                egui::vec2(card_w, 72.0),
                                Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_width(card_w);
                                    let row = egui::Frame::new()
                                        .fill(theme::BG_SURFACE)
                                        .inner_margin(Margin::symmetric(12, 8))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                theme::status_dot(ui, ch.color);
                                                ui.label(
                                                    RichText::new(&ch.name)
                                                        .size(11.0)
                                                        .monospace()
                                                        .color(theme::FG_MUTED),
                                                );
                                            });
                                            let val = ch.latest().unwrap_or(0.0);
                                            ui.label(
                                                RichText::new(format!("{val:.3}"))
                                                    .font(egui::FontId::monospace(20.0))
                                                    .color(theme::FG_PRIMARY),
                                            );
                                            let span = (ch.max_seen - ch.min_seen).abs();
                                            let norm = if ch.min_seen.is_finite()
                                                && ch.max_seen.is_finite()
                                                && span > 1e-4
                                            {
                                                ((val - ch.min_seen) / span).clamp(0.0, 1.0)
                                            } else {
                                                0.5
                                            };
                                            let (bar_rect, _) = ui.allocate_exact_size(
                                                egui::vec2(ui.available_width(), 3.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(
                                                bar_rect,
                                                1.5,
                                                theme::BG_ELEVATED,
                                            );
                                            let fill_rect = egui::Rect::from_min_size(
                                                bar_rect.min,
                                                egui::vec2(
                                                    bar_rect.width() * norm,
                                                    bar_rect.height(),
                                                ),
                                            );
                                            ui.painter().rect_filled(fill_rect, 1.5, ch.color);
                                        });
                                    let r = row.response.rect;
                                    ui.painter().line_segment(
                                        [
                                            egui::pos2(r.left(), r.bottom()),
                                            egui::pos2(r.right(), r.bottom()),
                                        ],
                                        Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
                                    );
                                },
                            );
                        }
                    });
                    ui.add_space(10.0);
                }
            }

            ui.add_space(16.0);
            theme::eyebrow(ui, "Tuning Sliders");
            ui.add_space(6.0);
            theme::hairline(ui);
            ui.add_space(10.0);

            let avail_w = ui.available_width().max(260.0);
            let cols = if avail_w >= 540.0 { 2 } else { 1 };
            let gap = 16.0_f32;
            let col_w = ((avail_w - gap * (cols as f32 - 1.0)) / cols as f32).floor();

            for chunk in app.serial_controls.sliders.chunks_mut(cols) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for slider in chunk {
                        ui.allocate_ui_with_layout(
                            egui::vec2(col_w, 64.0),
                            Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(col_w);
                                let row = egui::Frame::new()
                                    .fill(theme::BG_SURFACE)
                                    .inner_margin(Margin::symmetric(12, 8))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(&slider.name)
                                                    .size(12.0)
                                                    .strong()
                                                    .color(theme::FG_PRIMARY),
                                            );
                                            ui.with_layout(
                                                Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if icons::button(
                                                        ui,
                                                        Icon::Send,
                                                        "",
                                                        ButtonKind::Ghost,
                                                        connected,
                                                    )
                                                    .on_hover_text(&slider.template)
                                                    .clicked()
                                                    {
                                                        pending_sends.push((
                                                            format_command_template(
                                                                &slider.template,
                                                                slider.value,
                                                            ),
                                                            false,
                                                        ));
                                                    }
                                                    if ui
                                                        .checkbox(&mut slider.auto_send, "Auto")
                                                        .changed()
                                                    {
                                                        sliders_dirty = true;
                                                    }
                                                },
                                            );
                                        });
                                        let lo = slider.min.min(slider.max);
                                        let hi = slider.max.max(slider.min + 0.001);
                                        let resp = ui.add_sized(
                                            [ui.available_width(), 20.0],
                                            egui::Slider::new(&mut slider.value, lo..=hi),
                                        );
                                        if resp.changed() && slider.auto_send && connected {
                                            pending_sends.push((
                                                format_command_template(
                                                    &slider.template,
                                                    slider.value,
                                                ),
                                                false,
                                            ));
                                        }
                                        if resp.drag_stopped() {
                                            sliders_dirty = true;
                                        }
                                    });
                                let r = row.response.rect;
                                ui.painter().line_segment(
                                    [
                                        egui::pos2(r.left(), r.bottom()),
                                        egui::pos2(r.right(), r.bottom()),
                                    ],
                                    Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
                                );
                            },
                        );
                    }
                });
                ui.add_space(10.0);
            }
        });

    for (payload, hex) in pending_sends {
        app.send_serial_payload(&payload, hex);
    }
    if sliders_dirty {
        app.save_serial_controls_state();
    }
}

fn render_controls_sidebar(ui: &mut egui::Ui, app: &mut RavomApp, connected: bool) {
    let mut pending_sends: Vec<(String, bool)> = Vec::new();
    let mut remove_macro: Option<usize> = None;
    let mut remove_slider: Option<usize> = None;
    let mut controls_dirty = false;

    egui::ScrollArea::vertical()
        .id_salt("monitor_controls_sidebar")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // 1. Quick Command Macros
            theme::eyebrow(ui, "Macros");
            ui.add_space(6.0);
            theme::hairline(ui);
            ui.add_space(8.0);

            let inner_w = ui.available_width().max(160.0);
            let cols = 2usize;
            let gap = 6.0_f32;
            let cell_w = ((inner_w - gap) * 0.5).floor();

            for (row_idx, chunk) in app.serial_controls.macros.chunks(cols).enumerate() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (col_idx, m) in chunk.iter().enumerate() {
                        let idx = row_idx * cols + col_idx;
                        ui.allocate_ui_with_layout(
                            egui::vec2(cell_w, 30.0),
                            Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if tiny_remove_button(ui, ("del_macro", idx)).clicked() {
                                    remove_macro = Some(idx);
                                }
                                let btn_w = ui.available_width().max(50.0);
                                ui.allocate_ui_with_layout(
                                    egui::vec2(btn_w, 30.0),
                                    Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        let label = if m.label.len() > 11 {
                                            format!("{}…", &m.label[..10])
                                        } else {
                                            m.label.clone()
                                        };
                                        if theme::chip_button(ui, &label, false, connected)
                                            .on_hover_text(&m.payload)
                                            .clicked()
                                        {
                                            pending_sends.push((m.payload.clone(), m.hex));
                                        }
                                    },
                                );
                            },
                        );
                    }
                });
                ui.add_space(4.0);
            }

            ui.add_space(6.0);
            egui::CollapsingHeader::new(RichText::new("+ Macro").size(11.5).color(theme::FG_MUTED))
                .default_open(false)
                .show(ui, |ui| {
                    let w = ui.available_width();
                    theme::text_input(
                        ui,
                        egui::Id::new("new_macro_label"),
                        &mut app.new_macro_label,
                        "Label (e.g. CALIB)",
                        w,
                        false,
                        false,
                    );
                    ui.add_space(4.0);
                    theme::text_input(
                        ui,
                        egui::Id::new("new_macro_payload"),
                        &mut app.new_macro_payload,
                        "Payload",
                        w,
                        false,
                        false,
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut app.new_macro_hex, "HEX");
                        let can_add = !app.new_macro_label.trim().is_empty()
                            && !app.new_macro_payload.trim().is_empty();
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if icons::button(ui, Icon::Plus, "Add", ButtonKind::Primary, can_add)
                                .clicked()
                            {
                                app.serial_controls.macros.push(SerialMacro {
                                    label: app.new_macro_label.trim().to_string(),
                                    payload: app.new_macro_payload.trim().to_string(),
                                    hex: app.new_macro_hex,
                                });
                                app.new_macro_label.clear();
                                app.new_macro_payload.clear();
                                controls_dirty = true;
                            }
                        });
                    });
                });

            ui.add_space(16.0);
            theme::eyebrow(ui, "Sliders");
            ui.add_space(6.0);
            theme::hairline(ui);
            ui.add_space(8.0);

            for (idx, slider) in app.serial_controls.sliders.iter_mut().enumerate() {
                let row = egui::Frame::new()
                    .fill(theme::BG_SURFACE)
                    .inner_margin(Margin::symmetric(8, 7))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&slider.name)
                                    .size(12.0)
                                    .strong()
                                    .color(theme::FG_PRIMARY),
                            );
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                if tiny_remove_button(ui, ("del_slider", idx)).clicked() {
                                    remove_slider = Some(idx);
                                }
                                if icons::button(ui, Icon::Send, "", ButtonKind::Ghost, connected)
                                    .on_hover_text(&slider.template)
                                    .clicked()
                                {
                                    pending_sends.push((
                                        format_command_template(&slider.template, slider.value),
                                        false,
                                    ));
                                }
                                if ui.checkbox(&mut slider.auto_send, "Auto").changed() {
                                    controls_dirty = true;
                                }
                            });
                        });
                        let lo = slider.min.min(slider.max);
                        let hi = slider.max.max(slider.min + 0.001);
                        let resp = ui.add_sized(
                            [ui.available_width(), 20.0],
                            egui::Slider::new(&mut slider.value, lo..=hi),
                        );
                        if resp.changed() && slider.auto_send && connected {
                            pending_sends.push((
                                format_command_template(&slider.template, slider.value),
                                false,
                            ));
                        }
                        if resp.drag_stopped() {
                            controls_dirty = true;
                        }
                    });
                let r = row.response.rect;
                ui.painter().line_segment(
                    [
                        egui::pos2(r.left(), r.bottom()),
                        egui::pos2(r.right(), r.bottom()),
                    ],
                    Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
                );
                ui.add_space(6.0);
            }

            egui::CollapsingHeader::new(
                RichText::new("+ Slider").size(11.5).color(theme::FG_MUTED),
            )
            .default_open(false)
            .show(ui, |ui| {
                let w = ui.available_width();
                theme::text_input(
                    ui,
                    egui::Id::new("new_slider_name"),
                    &mut app.new_slider_name,
                    "Name",
                    w,
                    false,
                    false,
                );
                ui.add_space(4.0);
                theme::text_input(
                    ui,
                    egui::Id::new("new_slider_template"),
                    &mut app.new_slider_template,
                    "Template (PWM %d)",
                    w,
                    false,
                    false,
                );
                ui.add_space(4.0);
                let half_w = ((w - 8.0) * 0.5).max(40.0);
                ui.horizontal(|ui| {
                    theme::text_input(
                        ui,
                        egui::Id::new("new_slider_min"),
                        &mut app.new_slider_min,
                        "Min",
                        half_w,
                        false,
                        false,
                    );
                    theme::text_input(
                        ui,
                        egui::Id::new("new_slider_max"),
                        &mut app.new_slider_max,
                        "Max",
                        half_w,
                        false,
                        false,
                    );
                });
                ui.add_space(6.0);
                let can_add = !app.new_slider_name.trim().is_empty()
                    && !app.new_slider_template.trim().is_empty();
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::button(ui, Icon::Plus, "Add", ButtonKind::Primary, can_add).clicked()
                    {
                        let min_v = app.new_slider_min.trim().parse::<f32>().unwrap_or(0.0);
                        let max_v = app.new_slider_max.trim().parse::<f32>().unwrap_or(100.0);
                        let lo = min_v.min(max_v);
                        let hi = max_v.max(lo + 1.0);
                        app.serial_controls.sliders.push(SerialSlider {
                            name: app.new_slider_name.trim().to_string(),
                            template: app.new_slider_template.trim().to_string(),
                            min: lo,
                            max: hi,
                            value: lo,
                            auto_send: false,
                        });
                        app.new_slider_name.clear();
                        app.new_slider_template.clear();
                        controls_dirty = true;
                    }
                });
            });

            ui.add_space(16.0);
            theme::hairline(ui);
            ui.add_space(8.0);

            egui::CollapsingHeader::new(
                RichText::new(format!("{} Snippet", app.serial_protocol.label()))
                    .size(12.0)
                    .color(theme::FG_SECONDARY),
            )
            .default_open(false)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(app.serial_protocol.snippet())
                        .font(egui::FontId::monospace(10.5))
                        .color(theme::FG_SECONDARY),
                );
                ui.add_space(6.0);
                theme::copy_feedback_button(
                    ui,
                    "copy_vofa_proto_snippet",
                    "Copy",
                    app.serial_protocol.snippet(),
                );
            });
        });

    if let Some(idx) = remove_macro {
        if idx < app.serial_controls.macros.len() {
            app.serial_controls.macros.remove(idx);
            controls_dirty = true;
        }
    }
    if let Some(idx) = remove_slider {
        if idx < app.serial_controls.sliders.len() {
            app.serial_controls.sliders.remove(idx);
            controls_dirty = true;
        }
    }
    for (payload, hex) in pending_sends {
        app.send_serial_payload(&payload, hex);
    }
    if controls_dirty {
        app.save_serial_controls_state();
    }
}

fn tiny_remove_button(ui: &mut egui::Ui, id: (&str, usize)) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
    let resp = resp
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Remove");
    let _ = id;
    if ui.is_rect_visible(rect) {
        let fg = if resp.hovered() {
            ui.painter().rect(
                rect,
                4.0,
                theme::BG_HOVER,
                Stroke::NONE,
                egui::StrokeKind::Inside,
            );
            theme::FG_PRIMARY
        } else {
            theme::FG_FAINT
        };
        let gal =
            ui.painter()
                .layout_no_wrap("×".to_string(), egui::FontId::proportional(13.0), fg);
        ui.painter().galley(
            egui::pos2(
                rect.center().x - gal.size().x * 0.5,
                rect.center().y - gal.size().y * 0.5,
            ),
            gal,
            fg,
        );
    }
    resp
}

fn format_byte_count(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

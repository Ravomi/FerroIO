//! VOFA+-inspired interactive multi-channel signal oscilloscope.

use crate::icons::{self, ButtonKind, Icon};
use crate::theme;
use eframe::egui::{self, Color32, Layout, Pos2, Stroke};
use std::collections::VecDeque;

/// Maximum number of samples retained per channel in the rolling waveform buffer.
pub const CAPACITY: usize = 2000;

/// Maximum simultaneous channels tracked in the oscilloscope.
pub const MAX_CHANNELS: usize = 16;

/// Pure monochrome grayscale luminance trace ladder (`R == G == B`, zero hue).
pub const CHANNEL_COLORS: &[Color32] = &[
    Color32::from_rgb(250, 250, 250), // Ch 0: Pure White (#FAFAFA)
    Color32::from_rgb(192, 192, 192), // Ch 1: Silver (#C0C0C0)
    Color32::from_rgb(140, 140, 140), // Ch 2: Mid Gray (#8C8C8C)
    Color32::from_rgb(224, 224, 224), // Ch 3: Soft White (#E0E0E0)
    Color32::from_rgb(112, 112, 112), // Ch 4: Dark Gray (#707070)
    Color32::from_rgb(168, 168, 168), // Ch 5: Light Gray (#A8A8A8)
    Color32::from_rgb(90, 90, 90),    // Ch 6: Dim Gray (#5A5A5A)
    Color32::from_rgb(238, 238, 238), // Ch 7: Bright White (#EEEEEE)
];

/// Rolling waveform buffer and metadata for one oscilloscope trace channel.
#[derive(Debug, Clone)]
pub struct PlotChannel {
    pub name: String,
    pub color: Color32,
    pub visible: bool,
    pub samples: VecDeque<f32>,
    pub min_seen: f32,
    pub max_seen: f32,
}

impl PlotChannel {
    fn new(name: String, idx: usize) -> Self {
        let color = CHANNEL_COLORS[idx % CHANNEL_COLORS.len()];
        Self {
            name,
            color,
            visible: true,
            samples: VecDeque::with_capacity(256),
            min_seen: f32::INFINITY,
            max_seen: f32::NEG_INFINITY,
        }
    }

    /// Most recent sample stored in this channel, if any.
    pub fn latest(&self) -> Option<f32> {
        self.samples.back().copied()
    }

    fn push_sample(&mut self, value: f32) {
        if !value.is_finite() {
            return;
        }
        if self.samples.len() >= CAPACITY {
            self.samples.pop_front();
        }
        self.samples.push_back(value);
        self.min_seen = self.min_seen.min(value);
        self.max_seen = self.max_seen.max(value);
    }
}

/// Interactive multi-channel rolling oscilloscope supporting live serial streams and simulation.
pub struct Plotter {
    pub simulate: bool,
    pub paused: bool,
    pub auto_y: bool,
    pub y_min: f32,
    pub y_max: f32,
    pub window_samples: usize,
    pub pan_offset: usize,
    pub channels: Vec<PlotChannel>,
    pub total_frames: u64,
    phase: f32,
    pan_accum_px: f32,
}

impl Default for Plotter {
    fn default() -> Self {
        Self {
            simulate: false,
            paused: false,
            auto_y: true,
            y_min: -1.0,
            y_max: 1.0,
            window_samples: 240,
            pan_offset: 0,
            channels: Vec::new(),
            total_frames: 0,
            phase: 0.0,
            pan_accum_px: 0.0,
        }
    }
}

impl Plotter {
    /// Push a single scalar sample into channel `I0`.
    #[cfg(test)]
    pub fn push(&mut self, sample: f32) {
        self.push_frame(&[(None, sample)]);
    }

    /// Push one multi-channel frame into the rolling oscilloscope buffers.
    pub fn push_frame(&mut self, items: &[(Option<String>, f32)]) {
        if self.paused || items.is_empty() {
            return;
        }
        let mut pushed_any = false;

        for (idx, (opt_name, value)) in items.iter().enumerate() {
            if !value.is_finite() {
                continue;
            }
            if let Some(name) = opt_name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
                if let Some(ch) = self.channels.iter_mut().find(|c| c.name == name) {
                    ch.push_sample(*value);
                    pushed_any = true;
                } else if self.channels.len() < MAX_CHANNELS {
                    let ch_idx = self.channels.len();
                    let mut ch = PlotChannel::new(name.to_string(), ch_idx);
                    ch.push_sample(*value);
                    self.channels.push(ch);
                    pushed_any = true;
                }
            } else {
                if idx >= MAX_CHANNELS {
                    continue;
                }
                while self.channels.len() <= idx && self.channels.len() < MAX_CHANNELS {
                    let ch_idx = self.channels.len();
                    self.channels
                        .push(PlotChannel::new(format!("I{ch_idx}"), ch_idx));
                }
                if let Some(ch) = self.channels.get_mut(idx) {
                    ch.push_sample(*value);
                    pushed_any = true;
                }
            }
        }

        if pushed_any {
            self.total_frames += 1;
        }
    }

    /// Clear all recorded samples and channels.
    pub fn clear(&mut self) {
        self.channels.clear();
        self.total_frames = 0;
        self.pan_offset = 0;
        self.pan_accum_px = 0.0;
        self.phase = 0.0;
    }

    /// Latest sample from the first active channel, if any.
    pub fn latest(&self) -> Option<f32> {
        self.channels.first().and_then(PlotChannel::latest)
    }

    /// Maximum sample count across tracked channels (used by unit tests).
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.channels
            .iter()
            .map(|c| c.samples.len())
            .max()
            .unwrap_or(0)
    }

    /// Export recorded channel buffers as CSV (`sample,<ch0>,<ch1>,...`).
    pub fn export_csv(&self) -> String {
        if self.channels.is_empty() {
            return String::from("sample\n");
        }
        let max_len = self
            .channels
            .iter()
            .map(|c| c.samples.len())
            .max()
            .unwrap_or(0);
        let mut csv = String::from("sample");
        for ch in &self.channels {
            csv.push(',');
            csv.push_str(&ch.name);
        }
        csv.push('\n');

        for row in 0..max_len {
            csv.push_str(&row.to_string());
            for ch in &self.channels {
                csv.push(',');
                let offset = max_len - ch.samples.len();
                if row >= offset {
                    if let Some(val) = ch.samples.get(row - offset) {
                        csv.push_str(&format!("{val:.6}"));
                    }
                }
            }
            csv.push('\n');
        }
        csv
    }

    fn tick_simulation(&mut self) {
        self.phase += 0.10;
        let setpoint = if (self.phase / 4.0) as i32 % 2 == 0 {
            1.0_f32
        } else {
            -0.5_f32
        };
        let measured = setpoint
            + 0.32 * (self.phase * 1.35).sin() * (-((self.phase % 4.0) * 0.65)).exp()
            + 0.04 * (self.phase * 5.2).cos();
        let error = setpoint - measured;
        self.push_frame(&[
            (Some("setpoint".to_string()), setpoint),
            (Some("measured".to_string()), measured),
            (Some("error".to_string()), error),
        ]);
    }

    /// Render the compact oscilloscope inside the collapsible bottom output drawer.
    pub fn ui(&mut self, ui: &mut egui::Ui, plot_height: f32) {
        let _ = self.render_inner(ui, plot_height, false);
    }

    /// Render the full VOFA+-style interactive oscilloscope and return `true` when Export CSV is clicked.
    pub fn show_full(&mut self, ui: &mut egui::Ui, plot_height: f32) -> bool {
        self.render_inner(ui, plot_height, true)
    }

    fn render_inner(&mut self, ui: &mut egui::Ui, plot_height: f32, full_controls: bool) -> bool {
        let mut export_clicked = false;
        let paused_before = self.paused;

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 4.0);
            theme::eyebrow(ui, "Scope");
            let status_txt = match (self.latest(), self.paused, self.simulate) {
                (Some(last), true, _) => format!("{}ch · {last:.2} · PAUSED", self.channels.len()),
                (Some(last), false, true) => format!("{}ch · {last:.2} · SIM", self.channels.len()),
                (Some(last), false, false) => format!("{}ch · {last:.2}", self.channels.len()),
                (None, _, true) => "SIM".to_string(),
                (None, _, false) => "IDLE".to_string(),
            };
            ui.label(
                egui::RichText::new(status_txt)
                    .font(egui::FontId::monospace(10.5))
                    .color(if self.latest().is_some() {
                        theme::FG_PRIMARY
                    } else {
                        theme::FG_MUTED
                    }),
            );

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::button(ui, Icon::Trash, "", ButtonKind::Ghost, true)
                    .on_hover_text("Clear waveform")
                    .clicked()
                {
                    self.clear();
                }
                if full_controls
                    && icons::button(
                        ui,
                        Icon::Download,
                        "",
                        ButtonKind::Ghost,
                        !self.channels.is_empty(),
                    )
                    .on_hover_text("Export CSV")
                    .clicked()
                {
                    export_clicked = true;
                }
                if theme::underline_tab(ui, "Fit", false).clicked() {
                    self.auto_y = true;
                    self.pan_offset = 0;
                    self.pan_accum_px = 0.0;
                    self.window_samples = 240;
                }
                if theme::underline_tab(ui, "Auto Y", self.auto_y).clicked() {
                    self.auto_y = !self.auto_y;
                }
                let (pause_icon, pause_tip) = if self.paused {
                    (Icon::Play, "Resume")
                } else {
                    (Icon::Pause, "Pause")
                };
                if icons::button(ui, pause_icon, "", ButtonKind::Ghost, true)
                    .on_hover_text(pause_tip)
                    .clicked()
                {
                    self.paused = !self.paused;
                }
                ui.checkbox(&mut self.simulate, "Sim");
                if full_controls {
                    for &win in [1500_usize, 600, 240, 80].iter() {
                        if theme::underline_tab(ui, &format!("{win}"), self.window_samples == win)
                            .clicked()
                        {
                            self.window_samples = win;
                            self.pan_offset = 0;
                        }
                    }
                }
            });
        });

        ui.add_space(6.0);

        if self.simulate && !paused_before {
            self.tick_simulation();
            ui.ctx().request_repaint();
        }

        let desired = egui::vec2(ui.available_width(), plot_height.max(92.0));
        let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

        let max_len = self
            .channels
            .iter()
            .map(|c| c.samples.len())
            .max()
            .unwrap_or(0);

        if resp.hovered() {
            let scroll_y = ui.ctx().input(|i| i.raw_scroll_delta.y);
            if scroll_y.abs() > 0.1 {
                let factor = if scroll_y > 0.0 { 0.85_f32 } else { 1.15_f32 };
                self.window_samples =
                    ((self.window_samples as f32 * factor).round() as usize).clamp(40, CAPACITY);
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }

        if resp.dragged() && max_len > 20 {
            self.pan_accum_px += resp.drag_delta().x;
            let px_per_sample = (rect.width() / self.window_samples.max(1) as f32).max(0.25);
            let step = (self.pan_accum_px / px_per_sample).trunc() as isize;
            if step != 0 {
                self.pan_accum_px -= step as f32 * px_per_sample;
                let max_pan = max_len.saturating_sub(20) as isize;
                let next_pan = (self.pan_offset as isize + step).clamp(0, max_pan);
                self.pan_offset = next_pan as usize;
            }
        } else {
            self.pan_accum_px = 0.0;
        }

        if resp.double_clicked() {
            self.auto_y = true;
            self.pan_offset = 0;
            self.pan_accum_px = 0.0;
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, theme::BG_CANVAS);
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
            egui::StrokeKind::Inside,
        );

        // Gridlines.
        for frac in [0.25_f32, 0.75] {
            let y = rect.top() + rect.height() * frac;
            painter.line_segment(
                [
                    Pos2::new(rect.left() + 1.0, y),
                    Pos2::new(rect.right() - 1.0, y),
                ],
                Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
            );
        }
        for frac in [0.25_f32, 0.5, 0.75] {
            let x = rect.left() + rect.width() * frac;
            painter.line_segment(
                [
                    Pos2::new(x, rect.top() + 1.0),
                    Pos2::new(x, rect.bottom() - 1.0),
                ],
                Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
            );
        }
        painter.line_segment(
            [
                Pos2::new(rect.left() + 1.0, rect.center().y),
                Pos2::new(rect.right() - 1.0, rect.center().y),
            ],
            Stroke::new(1.0_f32, theme::BORDER),
        );

        let visible_channels: Vec<&PlotChannel> =
            self.channels.iter().filter(|c| c.visible).collect();
        let has_plottable = visible_channels.iter().any(|c| c.samples.len() >= 2);

        if has_plottable {
            if self.auto_y {
                let mut min_v = f32::INFINITY;
                let mut max_v = f32::NEG_INFINITY;
                for ch in &visible_channels {
                    let end = ch.samples.len().saturating_sub(self.pan_offset);
                    let start = end.saturating_sub(self.window_samples);
                    for i in start..end {
                        if let Some(&v) = ch.samples.get(i) {
                            min_v = min_v.min(v);
                            max_v = max_v.max(v);
                        }
                    }
                }
                if min_v.is_finite() && max_v.is_finite() {
                    if (max_v - min_v).abs() < 1e-4 {
                        min_v -= 1.0;
                        max_v += 1.0;
                    }
                    let pad = (max_v - min_v) * 0.12;
                    self.y_min = min_v - pad;
                    self.y_max = max_v + pad;
                }
            }

            let lo = self.y_min;
            let hi = self.y_max;
            let span = (hi - lo).max(1e-4);
            let inner_left = rect.left() + 6.0;
            let inner_w = (rect.width() - 12.0).max(20.0);
            let inner_top = rect.top() + 10.0;
            let inner_h = (rect.height() - 20.0).max(20.0);
            let denom = self.window_samples.saturating_sub(1).max(1) as f32;

            // Zero baseline if within Y range.
            if lo < 0.0 && hi > 0.0 {
                let zero_norm = (0.0 - lo) / span;
                let zero_y = inner_top + (1.0 - zero_norm) * inner_h;
                painter.line_segment(
                    [
                        Pos2::new(rect.left() + 1.0, zero_y),
                        Pos2::new(rect.right() - 1.0, zero_y),
                    ],
                    Stroke::new(1.0_f32, theme::BORDER_STRONG),
                );
            }

            for ch in &visible_channels {
                let end = ch.samples.len().saturating_sub(self.pan_offset);
                let start = end.saturating_sub(self.window_samples);
                let count = end - start;
                if count < 2 {
                    continue;
                }
                let slot_offset = self.window_samples.saturating_sub(count);
                let mut points = Vec::with_capacity(count);
                for (local_i, sample_i) in (start..end).enumerate() {
                    if let Some(&v) = ch.samples.get(sample_i) {
                        let slot = (slot_offset + local_i) as f32;
                        let x = inner_left + (slot / denom) * inner_w;
                        let norm = ((v - lo) / span).clamp(-0.2, 1.2);
                        let y = inner_top + (1.0 - norm) * inner_h;
                        points.push(Pos2::new(x, y));
                    }
                }
                painter.add(egui::Shape::line(points, Stroke::new(1.75_f32, ch.color)));
            }

            // Y scale labels.
            painter.text(
                Pos2::new(rect.left() + 8.0, rect.top() + 5.0),
                egui::Align2::LEFT_TOP,
                format!("{hi:.2}"),
                egui::FontId::monospace(10.0),
                theme::FG_FAINT,
            );
            painter.text(
                Pos2::new(rect.left() + 8.0, rect.bottom() - 5.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{lo:.2}"),
                egui::FontId::monospace(10.0),
                theme::FG_FAINT,
            );

            // Hover crosshair & floating HUD readout.
            if let Some(pos) = resp.hover_pos() {
                if rect.contains(pos) {
                    painter.line_segment(
                        [
                            Pos2::new(pos.x, rect.top() + 1.0),
                            Pos2::new(pos.x, rect.bottom() - 1.0),
                        ],
                        Stroke::new(1.0_f32, theme::BORDER_STRONG),
                    );
                    let frac = ((pos.x - inner_left) / inner_w).clamp(0.0, 1.0);
                    let slot = (frac * denom).round() as usize;

                    let mut hud_rows: Vec<(Color32, String)> = Vec::new();
                    for ch in visible_channels.iter().take(8) {
                        let end = ch.samples.len().saturating_sub(self.pan_offset);
                        let start = end.saturating_sub(self.window_samples);
                        let count = end - start;
                        let slot_offset = self.window_samples.saturating_sub(count);
                        if slot >= slot_offset && count > 0 {
                            let idx = start + (slot - slot_offset);
                            if let Some(&val) = ch.samples.get(idx) {
                                hud_rows.push((ch.color, format!("{}: {val:.3}", ch.name)));
                            }
                        }
                    }

                    if !hud_rows.is_empty() {
                        let hud_w = 156.0_f32;
                        let hud_h = 20.0 + hud_rows.len() as f32 * 16.0;
                        let hud_x = if pos.x + hud_w + 14.0 > rect.right() {
                            (pos.x - hud_w - 10.0).max(rect.left() + 6.0)
                        } else {
                            pos.x + 10.0
                        };
                        let hud_y = (pos.y - hud_h * 0.5).clamp(
                            rect.top() + 6.0,
                            (rect.bottom() - hud_h - 6.0).max(rect.top() + 6.0),
                        );
                        let hud_rect = egui::Rect::from_min_size(
                            Pos2::new(hud_x, hud_y),
                            egui::vec2(hud_w, hud_h),
                        );
                        painter.rect_filled(hud_rect, 6.0, theme::BG_ELEVATED);
                        painter.rect_stroke(
                            hud_rect,
                            6.0,
                            Stroke::new(1.0_f32, theme::BORDER_STRONG),
                            egui::StrokeKind::Inside,
                        );
                        painter.text(
                            Pos2::new(hud_rect.left() + 8.0, hud_rect.top() + 4.0),
                            egui::Align2::LEFT_TOP,
                            format!("slot {slot}"),
                            egui::FontId::monospace(10.0),
                            theme::FG_MUTED,
                        );
                        for (i, (color, text)) in hud_rows.iter().enumerate() {
                            let row_y = hud_rect.top() + 20.0 + i as f32 * 16.0;
                            painter.circle_filled(
                                Pos2::new(hud_rect.left() + 12.0, row_y + 5.0),
                                3.0,
                                *color,
                            );
                            painter.text(
                                Pos2::new(hud_rect.left() + 20.0, row_y),
                                egui::Align2::LEFT_TOP,
                                text,
                                egui::FontId::monospace(10.5),
                                theme::FG_PRIMARY,
                            );
                        }
                    }
                }
            }
        } else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                if self.simulate {
                    "Generating 3-channel PID wave..."
                } else {
                    "Stream FireWater / JustFloat telemetry or enable Simulate"
                },
                egui::FontId::proportional(11.5),
                theme::FG_FAINT,
            );
        }

        // Clickable channel legend strip.
        if !self.channels.is_empty() {
            ui.add_space(6.0);
            let show_bulk = self.channels.len() > 1;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                for ch in &mut self.channels {
                    let latest = ch
                        .latest()
                        .map(|v| format!("{v:.2}"))
                        .unwrap_or_else(|| "—".to_string());
                    let label = format!("{}  {}", ch.name, latest);
                    let probe = ui.painter().layout_no_wrap(
                        label.clone(),
                        egui::FontId::monospace(11.0),
                        theme::FG_PRIMARY,
                    );
                    let w = probe.size().x + 28.0;
                    let (chip_rect, chip_resp) =
                        ui.allocate_exact_size(egui::vec2(w, 24.0), egui::Sense::click());
                    let chip_resp = chip_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if ui.is_rect_visible(chip_rect) {
                        let bg = if ch.visible {
                            theme::BG_ELEVATED
                        } else {
                            theme::BG_CANVAS
                        };
                        let border = if ch.visible {
                            theme::BORDER_STRONG
                        } else {
                            theme::BORDER_SUBTLE
                        };
                        let fg = if ch.visible {
                            theme::FG_PRIMARY
                        } else {
                            theme::FG_FAINT
                        };
                        ui.painter().rect(
                            chip_rect,
                            5.0,
                            bg,
                            Stroke::new(1.0_f32, border),
                            egui::StrokeKind::Inside,
                        );
                        let dot_color = if ch.visible {
                            ch.color
                        } else {
                            theme::FG_FAINT
                        };
                        ui.painter().circle_filled(
                            Pos2::new(chip_rect.left() + 10.0, chip_rect.center().y),
                            3.5,
                            dot_color,
                        );
                        let gal =
                            ui.painter()
                                .layout_no_wrap(label, egui::FontId::monospace(11.0), fg);
                        ui.painter().galley(
                            Pos2::new(
                                chip_rect.left() + 18.0,
                                chip_rect.center().y - gal.size().y * 0.5,
                            ),
                            gal,
                            fg,
                        );
                    }
                    if chip_resp.clicked() {
                        ch.visible = !ch.visible;
                    }
                }
                if show_bulk {
                    if theme::chip_button(ui, "All", false, true).clicked() {
                        for ch in &mut self.channels {
                            ch.visible = true;
                        }
                    }
                    if theme::chip_button(ui, "None", false, true).clicked() {
                        for ch in &mut self.channels {
                            ch.visible = false;
                        }
                    }
                }
            });
        }

        export_clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_is_bounded() {
        let mut p = Plotter::default();
        for _ in 0..CAPACITY + 250 {
            p.tick_simulation();
        }
        assert_eq!(p.len(), CAPACITY);
    }

    #[test]
    fn multi_channel_push_and_csv_export() {
        let mut scalar = Plotter::default();
        scalar.push(9.5);
        assert_eq!(scalar.latest(), Some(9.5));

        let mut p = Plotter::default();
        p.push_frame(&[(Some("kp".to_string()), 1.5), (None, 2.5)]);
        p.push_frame(&[(Some("kp".to_string()), 3.5), (None, 4.5)]);
        assert_eq!(p.channels.len(), 2);
        let csv = p.export_csv();
        assert!(csv.starts_with("sample,kp,I1\n"));
        assert!(csv.contains("1.500000"));
        assert!(csv.contains("4.500000"));
    }
}

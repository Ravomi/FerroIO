//! Rolling signal plot (currently fed by a synthetic generator).

use crate::theme;
use eframe::egui::{self, Layout, Stroke};
use std::collections::VecDeque;

pub const CAPACITY: usize = 160;

#[derive(Default)]
pub struct Plotter {
    pub simulate: bool,
    phase: f32,
    samples: VecDeque<f32>,
}

impl Plotter {
    pub fn push(&mut self, sample: f32) {
        if self.samples.len() >= CAPACITY {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    fn tick_simulation(&mut self) {
        self.phase += 0.15;
        let sample = (self.phase.sin() * 0.7) + ((self.phase * 2.7).cos() * 0.25);
        self.push(sample);
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            theme::section_title(ui, "Signal Plotter");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(theme::secondary_button("Reset")).clicked() {
                    self.samples.clear();
                }
                ui.checkbox(&mut self.simulate, "Simulate");
            });
        });
        ui.add_space(6.0);

        if self.simulate {
            self.tick_simulation();
            ui.ctx().request_repaint();
        }

        let desired = egui::vec2(ui.available_width(), 110.0);
        let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 6.0, theme::BG_BASE);
        painter.rect_stroke(
            rect,
            6.0,
            Stroke::new(1.0_f32, theme::BORDER),
            egui::StrokeKind::Inside,
        );

        // Subtle horizontal midline
        painter.line_segment(
            [
                egui::pos2(rect.left() + 1.0, rect.center().y),
                egui::pos2(rect.right() - 1.0, rect.center().y),
            ],
            Stroke::new(1.0_f32, theme::BG_MUTED),
        );

        if self.samples.len() >= 2 {
            let n = self.samples.len() as f32;
            let points: Vec<egui::Pos2> = self
                .samples
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let x = rect.left() + (i as f32 / (n - 1.0).max(1.0)) * rect.width();
                    let y = rect.center().y - v * (rect.height() * 0.42);
                    egui::pos2(x, y)
                })
                .collect();
            painter.add(egui::Shape::line(
                points,
                Stroke::new(1.75_f32, theme::ACCENT_WINE_FG),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_is_bounded() {
        let mut p = Plotter::default();
        for _ in 0..CAPACITY * 2 {
            p.tick_simulation();
        }
        assert_eq!(p.len(), CAPACITY);
    }
}

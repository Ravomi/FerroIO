//! Lucide-inspired minimalist 1.5px vector icons drawn with `egui::Painter`.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Icon {
    Folder,
    Plus,
    Play,
    Zap,
    Activity,
    Cpu,
    Sliders,
    Trash,
}

/// Paint a crisp 14x14 Lucide-style stroke icon at `rect.center()`.
pub fn paint(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width().min(rect.height()) / 24.0;
    let o = rect.center() - Vec2::splat(12.0 * s);
    let p = |x: f32, y: f32| Pos2::new(o.x + x * s, o.y + y * s);
    let stroke = Stroke::new(1.5_f32, color);

    match icon {
        Icon::Folder => {
            painter.line_segment([p(3.0, 6.0), p(9.0, 6.0)], stroke);
            painter.line_segment([p(9.0, 6.0), p(11.0, 8.0)], stroke);
            painter.line_segment([p(11.0, 8.0), p(21.0, 8.0)], stroke);
            painter.line_segment([p(21.0, 8.0), p(21.0, 19.0)], stroke);
            painter.line_segment([p(21.0, 19.0), p(3.0, 19.0)], stroke);
            painter.line_segment([p(3.0, 19.0), p(3.0, 6.0)], stroke);
        }
        Icon::Plus => {
            painter.line_segment([p(12.0, 5.0), p(12.0, 19.0)], stroke);
            painter.line_segment([p(5.0, 12.0), p(19.0, 12.0)], stroke);
        }
        Icon::Play => {
            let pts = vec![p(6.0, 4.0), p(19.0, 12.0), p(6.0, 20.0)];
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
        Icon::Zap => {
            let pts = vec![
                p(13.0, 2.0),
                p(4.0, 14.0),
                p(12.0, 14.0),
                p(11.0, 22.0),
                p(20.0, 10.0),
                p(12.0, 10.0),
            ];
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
        Icon::Activity => {
            let pts = vec![
                p(2.0, 12.0),
                p(6.0, 12.0),
                p(9.0, 4.0),
                p(15.0, 20.0),
                p(18.0, 12.0),
                p(22.0, 12.0),
            ];
            painter.add(egui::Shape::line(pts, stroke));
        }
        Icon::Cpu => {
            painter.rect_stroke(
                Rect::from_min_max(p(6.0, 6.0), p(18.0, 18.0)),
                2.0 * s,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_min_max(p(10.0, 10.0), p(14.0, 14.0)),
                1.0 * s,
                stroke,
                egui::StrokeKind::Inside,
            );
            for offset in [9.0, 15.0] {
                painter.line_segment([p(offset, 2.0), p(offset, 6.0)], stroke);
                painter.line_segment([p(offset, 18.0), p(offset, 22.0)], stroke);
                painter.line_segment([p(2.0, offset), p(6.0, offset)], stroke);
                painter.line_segment([p(18.0, offset), p(22.0, offset)], stroke);
            }
        }
        Icon::Sliders => {
            painter.line_segment([p(4.0, 21.0), p(4.0, 14.0)], stroke);
            painter.line_segment([p(4.0, 10.0), p(4.0, 3.0)], stroke);
            painter.line_segment([p(12.0, 21.0), p(12.0, 12.0)], stroke);
            painter.line_segment([p(12.0, 8.0), p(12.0, 3.0)], stroke);
            painter.line_segment([p(20.0, 21.0), p(20.0, 16.0)], stroke);
            painter.line_segment([p(20.0, 12.0), p(20.0, 3.0)], stroke);
            painter.line_segment([p(2.0, 14.0), p(6.0, 14.0)], stroke);
            painter.line_segment([p(10.0, 8.0), p(14.0, 8.0)], stroke);
            painter.line_segment([p(18.0, 16.0), p(22.0, 16.0)], stroke);
        }
        Icon::Trash => {
            painter.line_segment([p(3.0, 6.0), p(21.0, 6.0)], stroke);
            painter.line_segment([p(8.0, 6.0), p(8.0, 3.0)], stroke);
            painter.line_segment([p(8.0, 3.0), p(16.0, 3.0)], stroke);
            painter.line_segment([p(16.0, 3.0), p(16.0, 6.0)], stroke);
            painter.line_segment([p(6.0, 6.0), p(7.0, 20.0)], stroke);
            painter.line_segment([p(7.0, 20.0), p(17.0, 20.0)], stroke);
            painter.line_segment([p(17.0, 20.0), p(18.0, 6.0)], stroke);
        }
    }
}

/// Render a shadcn-style button with a leading 14x14 Lucide vector icon.
pub fn button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    primary: bool,
    enabled: bool,
) -> egui::Response {
    let text_color = crate::theme::FG_PRIMARY;
    let fill = if primary {
        crate::theme::ACCENT_WINE
    } else {
        crate::theme::BG_MUTED
    };
    let border = if primary {
        crate::theme::ACCENT_WINE_HOVER
    } else {
        crate::theme::BORDER
    };

    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(13.0),
        text_color,
    );
    let icon_size = 14.0;
    let gap = 6.0;
    let pad = Vec2::new(10.0, 5.0);
    let desired = Vec2::new(
        pad.x * 2.0 + icon_size + gap + galley.size().x,
        (pad.y * 2.0 + icon_size.max(galley.size().y)).max(26.0),
    );
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(desired, sense);
    if ui.is_rect_visible(rect) {
        let hovered = enabled && response.hovered();
        let bg = if !enabled {
            fill.gamma_multiply(0.5)
        } else if hovered && !primary {
            crate::theme::BG_HOVER
        } else if hovered && primary {
            crate::theme::ACCENT_WINE_HOVER
        } else {
            fill
        };
        let fg = if enabled {
            text_color
        } else {
            text_color.gamma_multiply(0.5)
        };
        ui.painter().rect(
            rect,
            6.0,
            bg,
            Stroke::new(1.0_f32, border),
            egui::StrokeKind::Inside,
        );
        let icon_rect = Rect::from_center_size(
            Pos2::new(rect.left() + pad.x + icon_size * 0.5, rect.center().y),
            Vec2::splat(icon_size),
        );
        paint(ui.painter(), icon_rect, icon, fg);
        let text_pos = Pos2::new(
            icon_rect.right() + gap,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(text_pos, galley, fg);
    }
    response
}

//! Lucide-inspired minimalist 1.5px vector icons and IDE controls drawn with `egui::Painter`.

use crate::theme;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};

/// Uniform 1.5px Lucide-style vector icon identifiers used across the workbench.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Layers,
    Cpu,
    Terminal,
    Activity,
    Play,
    Pause,
    Zap,
    Folder,
    Plus,
    Refresh,
    Send,
    PanelBottom,
    Trash,
    Search,
    Sliders,
    ArrowRight,
    Check,
    Plug,
    Package,
    Download,
    Copy,
}

/// Visual prominence level for [`button`] following `shadcn/ui` variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Outline,
    Ghost,
}

/// Paint a crisp 1.5px Lucide-style vector icon centered inside `rect`.
pub fn paint(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width().min(rect.height()) / 24.0;
    let o = rect.center() - Vec2::splat(12.0 * s);
    let p = |x: f32, y: f32| Pos2::new(o.x + x * s, o.y + y * s);
    let stroke = Stroke::new(1.5_f32, color);

    match icon {
        Icon::Layers => {
            let top = vec![p(12.0, 3.0), p(21.0, 8.0), p(12.0, 13.0), p(3.0, 8.0)];
            painter.add(egui::Shape::closed_line(top, stroke));
            painter.add(egui::Shape::line(
                vec![p(3.0, 12.5), p(12.0, 17.5), p(21.0, 12.5)],
                stroke,
            ));
            painter.add(egui::Shape::line(
                vec![p(3.0, 17.0), p(12.0, 22.0), p(21.0, 17.0)],
                stroke,
            ));
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
        Icon::Terminal => {
            painter.rect_stroke(
                Rect::from_min_max(p(3.0, 4.0), p(21.0, 20.0)),
                2.0 * s,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.add(egui::Shape::line(
                vec![p(7.0, 9.0), p(10.5, 12.0), p(7.0, 15.0)],
                stroke,
            ));
            painter.line_segment([p(12.5, 15.0), p(17.0, 15.0)], stroke);
        }
        Icon::Activity => {
            painter.add(egui::Shape::line(
                vec![
                    p(2.0, 12.0),
                    p(6.0, 12.0),
                    p(9.0, 4.0),
                    p(15.0, 20.0),
                    p(18.0, 12.0),
                    p(22.0, 12.0),
                ],
                stroke,
            ));
        }
        Icon::Play => {
            let pts = vec![p(6.0, 4.0), p(19.0, 12.0), p(6.0, 20.0)];
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
        Icon::Pause => {
            painter.line_segment([p(8.5, 5.0), p(8.5, 19.0)], stroke);
            painter.line_segment([p(15.5, 5.0), p(15.5, 19.0)], stroke);
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
        Icon::Folder => {
            let pts = vec![
                p(3.0, 6.0),
                p(9.0, 6.0),
                p(11.0, 8.0),
                p(21.0, 8.0),
                p(21.0, 19.0),
                p(3.0, 19.0),
            ];
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
        Icon::Plus => {
            painter.line_segment([p(12.0, 5.0), p(12.0, 19.0)], stroke);
            painter.line_segment([p(5.0, 12.0), p(19.0, 12.0)], stroke);
        }
        Icon::Refresh => {
            painter.add(egui::Shape::line(
                vec![
                    p(20.0, 11.0),
                    p(18.5, 6.5),
                    p(14.0, 4.5),
                    p(9.0, 5.0),
                    p(5.5, 8.5),
                ],
                stroke,
            ));
            painter.line_segment([p(20.0, 5.0), p(20.0, 11.0)], stroke);
            painter.line_segment([p(15.0, 11.0), p(20.0, 11.0)], stroke);
            painter.add(egui::Shape::line(
                vec![
                    p(4.0, 13.0),
                    p(5.5, 17.5),
                    p(10.0, 19.5),
                    p(15.0, 19.0),
                    p(18.5, 15.5),
                ],
                stroke,
            ));
            painter.line_segment([p(4.0, 13.0), p(4.0, 19.0)], stroke);
            painter.line_segment([p(4.0, 13.0), p(9.0, 13.0)], stroke);
        }
        Icon::Send => {
            let pts = vec![p(21.0, 3.0), p(14.0, 21.0), p(11.0, 13.0), p(3.0, 10.0)];
            painter.add(egui::Shape::closed_line(pts, stroke));
            painter.line_segment([p(21.0, 3.0), p(11.0, 13.0)], stroke);
        }
        Icon::PanelBottom => {
            painter.rect_stroke(
                Rect::from_min_max(p(3.0, 4.0), p(21.0, 20.0)),
                2.0 * s,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line_segment([p(3.0, 15.0), p(21.0, 15.0)], stroke);
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
        Icon::Search => {
            painter.circle_stroke(p(11.0, 11.0), 6.5 * s, stroke);
            painter.line_segment([p(16.0, 16.0), p(21.0, 21.0)], stroke);
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
        Icon::ArrowRight => {
            painter.line_segment([p(5.0, 12.0), p(19.0, 12.0)], stroke);
            painter.add(egui::Shape::line(
                vec![p(13.0, 6.0), p(19.0, 12.0), p(13.0, 18.0)],
                stroke,
            ));
        }
        Icon::Check => {
            painter.add(egui::Shape::line(
                vec![p(5.0, 12.5), p(10.0, 17.5), p(19.5, 6.5)],
                stroke,
            ));
        }
        Icon::Plug => {
            painter.line_segment([p(9.0, 3.0), p(9.0, 7.5)], stroke);
            painter.line_segment([p(15.0, 3.0), p(15.0, 7.5)], stroke);
            painter.rect_stroke(
                Rect::from_min_max(p(6.5, 7.5), p(17.5, 14.5)),
                2.5 * s,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line_segment([p(12.0, 14.5), p(12.0, 21.0)], stroke);
        }
        Icon::Package => {
            let outer = vec![
                p(12.0, 2.5),
                p(20.5, 7.2),
                p(20.5, 16.8),
                p(12.0, 21.5),
                p(3.5, 16.8),
                p(3.5, 7.2),
            ];
            painter.add(egui::Shape::closed_line(outer, stroke));
            painter.add(egui::Shape::line(
                vec![p(3.5, 7.2), p(12.0, 12.0), p(20.5, 7.2)],
                stroke,
            ));
            painter.line_segment([p(12.0, 12.0), p(12.0, 21.5)], stroke);
            painter.line_segment([p(7.8, 4.9), p(16.2, 9.6)], stroke);
        }
        Icon::Download => {
            painter.line_segment([p(12.0, 3.0), p(12.0, 15.0)], stroke);
            painter.add(egui::Shape::line(
                vec![p(7.0, 10.0), p(12.0, 15.0), p(17.0, 10.0)],
                stroke,
            ));
            painter.add(egui::Shape::line(
                vec![p(4.0, 17.0), p(4.0, 20.5), p(20.0, 20.5), p(20.0, 17.0)],
                stroke,
            ));
        }
        Icon::Copy => {
            painter.rect_stroke(
                Rect::from_min_max(p(8.5, 8.5), p(20.0, 20.0)),
                2.0 * s,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.add(egui::Shape::line(
                vec![
                    p(15.5, 8.5),
                    p(15.5, 4.5),
                    p(4.5, 4.5),
                    p(4.5, 15.5),
                    p(8.5, 15.5),
                ],
                stroke,
            ));
        }
    }
}

/// Render a calm `shadcn/ui`-inspired IDE button with a leading 14x14 Lucide vector icon.
pub fn button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    kind: ButtonKind,
    enabled: bool,
) -> egui::Response {
    let icon_size = 14.0;
    let has_label = !label.is_empty();
    let gap = if has_label { 6.0 } else { 0.0 };
    let pad = Vec2::new(if has_label { 12.0 } else { 8.0 }, 5.0);

    let probe_galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(12.5),
        theme::FG_PRIMARY,
    );
    let desired = Vec2::new(
        pad.x * 2.0 + icon_size + gap + probe_galley.size().x,
        (pad.y * 2.0 + icon_size.max(probe_galley.size().y)).max(30.0),
    );
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(desired, sense);
    let response = if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };

    if ui.is_rect_visible(rect) {
        let hovered = enabled && response.hovered();
        let pressed = enabled && response.is_pointer_button_down_on();
        let hover_t = theme::animate_spring(ui.ctx(), response.id.with("btn_hov"), hovered);
        let press_t = theme::animate_spring(ui.ctx(), response.id.with("btn_prs"), pressed);

        let (base_fill, hover_fill, press_fill, base_border, hover_border, base_fg, hover_fg) =
            match kind {
                ButtonKind::Primary => (
                    theme::ACCENT_PRIMARY,
                    theme::ACCENT_PRIMARY_HOVER,
                    theme::ACCENT_PRIMARY_ACTIVE,
                    theme::ACCENT_PRIMARY,
                    theme::ACCENT_PRIMARY_HOVER,
                    theme::ACCENT_PRIMARY_FG,
                    theme::ACCENT_PRIMARY_FG,
                ),
                ButtonKind::Outline => (
                    theme::BG_SURFACE,
                    theme::BG_ELEVATED,
                    theme::BG_HOVER,
                    theme::BORDER,
                    theme::BORDER_STRONG,
                    theme::FG_SECONDARY,
                    theme::FG_PRIMARY,
                ),
                ButtonKind::Ghost => (
                    Color32::TRANSPARENT,
                    theme::BG_HOVER,
                    theme::BG_ACTIVE,
                    Color32::TRANSPARENT,
                    theme::BORDER_SUBTLE,
                    theme::FG_MUTED,
                    theme::FG_PRIMARY,
                ),
            };

        let bg = if !enabled {
            match kind {
                ButtonKind::Primary => theme::BG_HOVER,
                ButtonKind::Outline => theme::BG_SURFACE,
                ButtonKind::Ghost => Color32::TRANSPARENT,
            }
        } else {
            theme::lerp_color(
                theme::lerp_color(base_fill, hover_fill, hover_t),
                press_fill,
                press_t,
            )
        };

        let border_color = if !enabled {
            match kind {
                ButtonKind::Ghost => Color32::TRANSPARENT,
                _ => theme::BORDER_SUBTLE,
            }
        } else {
            theme::lerp_color(base_border, hover_border, hover_t)
        };

        let fg = if !enabled {
            theme::FG_FAINT
        } else {
            theme::lerp_color(base_fg, hover_fg, hover_t)
        };

        ui.painter().rect(
            rect,
            6.0,
            bg,
            Stroke::new(1.0_f32, border_color),
            egui::StrokeKind::Inside,
        );

        // Subtle inner top sheen on elevated Outline buttons for tactile New York depth.
        if enabled && kind == ButtonKind::Outline {
            let y = rect.top() + 1.0;
            ui.painter().line_segment(
                [
                    Pos2::new(rect.left() + 5.0, y),
                    Pos2::new(rect.right() - 5.0, y),
                ],
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 11)),
            );
        }

        let icon_rect = Rect::from_center_size(
            Pos2::new(rect.left() + pad.x + icon_size * 0.5, rect.center().y),
            Vec2::splat(icon_size),
        );
        paint(ui.painter(), icon_rect, icon, fg);
        if has_label {
            let galley = ui.painter().layout_no_wrap(
                label.to_string(),
                egui::FontId::proportional(12.5),
                fg,
            );
            let text_pos = Pos2::new(
                icon_rect.right() + gap,
                rect.center().y - galley.size().y * 0.5,
            );
            ui.painter().galley(text_pos, galley, fg);
        }
    }
    response
}

/// Render a far-left Activity Rail icon button with monochrome active indicator bar and dot.
pub fn rail_button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    active: bool,
    badge_color: Option<Color32>,
    tooltip: &str,
) -> egui::Response {
    let desired = Vec2::new(44.0, 44.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());
    let response = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tooltip);

    if ui.is_rect_visible(rect) {
        let hover_t =
            theme::animate_spring(ui.ctx(), response.id.with("rail_hov"), response.hovered());
        let active_t = theme::animate_spring(ui.ctx(), response.id.with("rail_act"), active);

        let bg = theme::lerp_color(
            theme::lerp_color(Color32::TRANSPARENT, theme::BG_HOVER, hover_t),
            theme::BG_ELEVATED,
            active_t,
        );
        let fg = theme::lerp_color(
            theme::lerp_color(theme::FG_MUTED, theme::FG_PRIMARY, hover_t),
            theme::FG_PRIMARY,
            active_t,
        );
        let border = theme::lerp_color(Color32::TRANSPARENT, theme::BORDER, active_t);

        ui.painter().rect(
            rect,
            8.0,
            bg,
            Stroke::new(1.0_f32, border),
            egui::StrokeKind::Inside,
        );

        if active_t > 0.02 {
            let bar_h = 18.0 * active_t;
            let bar = Rect::from_center_size(
                Pos2::new(rect.left() - 2.0, rect.center().y),
                Vec2::new(2.0, bar_h),
            );
            ui.painter().rect_filled(bar, 1.0, theme::FG_PRIMARY);
        }

        let icon_rect = Rect::from_center_size(
            Pos2::new(rect.center().x, rect.center().y - 7.0),
            Vec2::splat(16.0),
        );
        paint(ui.painter(), icon_rect, icon, fg);

        let galley =
            ui.painter()
                .layout_no_wrap(label.to_string(), egui::FontId::proportional(9.5), fg);
        ui.painter().galley(
            Pos2::new(
                rect.center().x - galley.size().x * 0.5,
                rect.bottom() - 14.5,
            ),
            galley,
            fg,
        );

        if let Some(dot_color) = badge_color {
            let dot_pos = Pos2::new(rect.right() - 8.0, rect.top() + 8.0);
            ui.painter().circle_filled(dot_pos, 3.0, dot_color);
        }
    }
    response
}

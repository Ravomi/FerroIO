//! Pure monochrome Black & White `shadcn/ui` IDE theme and reusable visual primitives.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Margin, RichText, Stroke,
    Vec2,
};
use std::sync::Arc;

/// 6-tier strictly achromatic (`R == G == B`) dark surface ladder inspired by `shadcn/ui`.
pub const BG_CANVAS: Color32 = Color32::from_rgb(9, 9, 9); // #090909
pub const BG_CHROME: Color32 = Color32::from_rgb(13, 13, 13); // #0D0D0D
pub const BG_SURFACE: Color32 = Color32::from_rgb(18, 18, 18); // #121212
pub const BG_ELEVATED: Color32 = Color32::from_rgb(26, 26, 26); // #1A1A1A
pub const BG_HOVER: Color32 = Color32::from_rgb(38, 38, 38); // #262626
pub const BG_ACTIVE: Color32 = Color32::from_rgb(56, 56, 56); // #383838

pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(31, 31, 31); // #1F1F1F
pub const BORDER: Color32 = Color32::from_rgb(42, 42, 42); // #2A2A2A
pub const BORDER_STRONG: Color32 = Color32::from_rgb(68, 68, 68); // #444444
pub const BORDER_FOCUS: Color32 = Color32::from_rgb(212, 212, 212); // #D4D4D4

pub const FG_PRIMARY: Color32 = Color32::from_rgb(250, 250, 250); // #FAFAFA
pub const FG_SECONDARY: Color32 = Color32::from_rgb(212, 212, 212); // #D4D4D4
pub const FG_MUTED: Color32 = Color32::from_rgb(156, 156, 156); // #9C9C9C
pub const FG_FAINT: Color32 = Color32::from_rgb(102, 102, 102); // #666666

/// Crisp white Primary CTA fill (`#FAFAFA`).
pub const ACCENT_PRIMARY: Color32 = FG_PRIMARY;
/// Hovered Primary CTA fill (`#E4E4E4`).
pub const ACCENT_PRIMARY_HOVER: Color32 = Color32::from_rgb(228, 228, 228);
/// Pressed Primary CTA fill (`#CCCCCC`).
pub const ACCENT_PRIMARY_ACTIVE: Color32 = Color32::from_rgb(204, 204, 204);
/// Foreground text and icon ink rendered inside [`ACCENT_PRIMARY`] buttons (`#090909`).
pub const ACCENT_PRIMARY_FG: Color32 = BG_CANVAS;
/// High-contrast monochrome ink for selection indicators, links, and active indicators (`#EBEBEB`).
pub const ACCENT_STRONG: Color32 = Color32::from_rgb(235, 235, 235);

/// Search match highlight tokens for monochrome dark surfaces.
pub const MATCH_BG: Color32 = BG_ACTIVE;
pub const MATCH_ACTIVE_BG: Color32 = FG_PRIMARY;
pub const MATCH_FG: Color32 = FG_PRIMARY;
pub const DANGER_BORDER: Color32 = Color32::from_rgb(120, 120, 120);
pub const DANGER_FG: Color32 = FG_PRIMARY;

fn try_insert_font(
    fonts: &mut FontDefinitions,
    name: &str,
    candidates: &[&str],
    family: FontFamily,
    prepend: bool,
) -> bool {
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.to_string(), Arc::new(FontData::from_owned(bytes)));
            let list = fonts.families.entry(family).or_default();
            if prepend {
                list.insert(0, name.to_string());
            } else {
                list.push(name.to_string());
            }
            return true;
        }
    }
    false
}

/// Configure crisp OS proportional, CJK, and monospace fonts when available on disk.
pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    try_insert_font(
        &mut fonts,
        "os_proportional",
        &[
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/HelveticaNeue.ttc",
            "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
        ],
        FontFamily::Proportional,
        true,
    );
    try_insert_font(
        &mut fonts,
        "os_mono",
        &[
            "/System/Library/Fonts/SFNSMono.ttf",
            "/System/Library/Fonts/Menlo.ttc",
            "/usr/share/fonts/truetype/jetbrains-mono/JetBrainsMono-Regular.ttf",
        ],
        FontFamily::Monospace,
        true,
    );

    // Append CJK fallback after primary proportional and monospace faces.
    if try_insert_font(
        &mut fonts,
        "os_cjk",
        &[
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ],
        FontFamily::Proportional,
        false,
    ) {
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .push("os_cjk".to_string());
    }

    ctx.set_fonts(fonts);
}

/// Apply the calm monochrome shadcn/ui IDE theme and OS fonts to the egui context.
pub fn apply_shadcn_theme(ctx: &egui::Context) {
    configure_fonts(ctx);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(10.0, 5.0);
    style.spacing.window_margin = Margin::same(0);
    style.spacing.indent = 14.0;
    style.spacing.scroll = egui::style::ScrollStyle::thin();
    style.animation_time = 0.20;

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_CANVAS;
    visuals.window_fill = BG_SURFACE;
    visuals.extreme_bg_color = BG_CANVAS;
    visuals.faint_bg_color = BG_ELEVATED;
    visuals.override_text_color = Some(FG_PRIMARY);

    let radius = CornerRadius::same(6);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = radius;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);

    visuals.selection.bg_fill = BG_HOVER;
    visuals.selection.stroke = Stroke::new(1.0_f32, BORDER_FOCUS);
    visuals.hyperlink_color = ACCENT_STRONG;

    visuals.widgets.noninteractive.bg_fill = BG_SURFACE;
    visuals.widgets.noninteractive.weak_bg_fill = BG_SURFACE;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, FG_MUTED);
    visuals.widgets.noninteractive.corner_radius = radius;

    visuals.widgets.inactive.bg_fill = BG_ELEVATED;
    visuals.widgets.inactive.weak_bg_fill = BG_ELEVATED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, FG_PRIMARY);
    visuals.widgets.inactive.corner_radius = radius;

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, BORDER_STRONG);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, FG_PRIMARY);
    visuals.widgets.hovered.corner_radius = radius;

    visuals.widgets.active.bg_fill = BG_ACTIVE;
    visuals.widgets.active.weak_bg_fill = BG_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, BORDER_FOCUS);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, FG_PRIMARY);
    visuals.widgets.active.corner_radius = radius;

    style.visuals = visuals;
    ctx.set_style(style);
}

/// Inset well container for code blocks, command snippets, and raw buffers.
pub fn inset_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 8))
}

/// Rikyu-inspired primary display heading for each view's architectural anchor.
pub fn hero_title(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(22.0).strong().color(FG_PRIMARY));
}

/// Rikyu uppercase monospace section eyebrow (`9.5pt`, muted ink).
pub fn eyebrow(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text.to_ascii_uppercase())
            .font(egui::FontId::monospace(9.5))
            .color(FG_MUTED),
    );
}

/// Crisp 1px horizontal architectural hairline rule spanning full available width.
pub fn hairline(ui: &mut egui::Ui) {
    let avail_w = ui.available_width().max(1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, 1.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.center().y),
                egui::pos2(rect.right(), rect.center().y),
            ],
            Stroke::new(1.0_f32, BORDER_SUBTLE),
        );
    }
}

/// Borderless underline tab (no card fill or box border), highlighting active state via a 1.5px white rule.
pub fn underline_tab(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let galley_probe = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(12.0),
        FG_PRIMARY,
    );
    let desired = Vec2::new((galley_probe.size().x + 12.0).max(36.0), 26.0);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let hov_t = animate_spring(ui.ctx(), resp.id.with("utab_hov"), resp.hovered());
        let act_t = animate_spring(ui.ctx(), resp.id.with("utab_act"), active);
        let fg = lerp_color(lerp_color(FG_MUTED, FG_SECONDARY, hov_t), FG_PRIMARY, act_t);
        let font = egui::FontId::proportional(12.0);
        let gal = ui.painter().layout_no_wrap(label.to_string(), font, fg);
        let pos = egui::Pos2::new(
            rect.center().x - gal.size().x * 0.5,
            rect.center().y - gal.size().y * 0.5 - 2.0,
        );
        ui.painter().galley(pos, gal, fg);

        if act_t > 0.02 {
            let line_w = (rect.width() - 8.0).max(12.0) * act_t;
            let cx = rect.center().x;
            let y = rect.bottom() - 1.5;
            ui.painter().line_segment(
                [
                    egui::pos2(cx - line_w * 0.5, y),
                    egui::pos2(cx + line_w * 0.5, y),
                ],
                Stroke::new(1.5_f32, FG_PRIMARY),
            );
        }
    }

    resp
}

/// Swiss hairline spec ledger cell showing a micro uppercase eyebrow and tabular monospace metric.
pub fn ledger_cell(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    accent: bool,
    width: f32,
) -> egui::Response {
    let w = width.max(88.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 50.0), egui::Sense::click());
    let hov_t = animate_spring(ui.ctx(), resp.id.with("ledger_hov"), resp.hovered());

    if ui.is_rect_visible(rect) {
        if hov_t > 0.02 {
            let wash = lerp_color(BG_CANVAS, BG_SURFACE, hov_t);
            ui.painter().rect_filled(rect, 4.0, wash);
        }
        let inner = rect.shrink2(Vec2::new(8.0, 4.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        child.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if accent {
                    let (d_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(7.0), egui::Sense::hover());
                    if ui.is_rect_visible(d_rect) {
                        ui.painter().circle_filled(d_rect.center(), 2.5, FG_PRIMARY);
                    }
                }
                ui.add(
                    egui::Label::new(
                        RichText::new(label.to_ascii_uppercase())
                            .font(egui::FontId::monospace(9.5))
                            .color(FG_MUTED),
                    )
                    .truncate(),
                );
            });
            ui.add(
                egui::Label::new(
                    RichText::new(value)
                        .size(16.0)
                        .strong()
                        .monospace()
                        .color(FG_PRIMARY),
                )
                .truncate(),
            );
        });
    }
    resp
}

/// Compact monochrome chip button using elevated fill + 1px white hairline border when active.
pub fn chip_button(ui: &mut egui::Ui, label: &str, active: bool, enabled: bool) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let galley_probe = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(11.5),
        FG_PRIMARY,
    );
    let desired = Vec2::new((galley_probe.size().x + 20.0).max(40.0), 26.0);
    let (rect, response) = ui.allocate_exact_size(desired, sense);
    let response = if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };

    if ui.is_rect_visible(rect) {
        let hover_t = animate_spring(
            ui.ctx(),
            response.id.with("chip_hov"),
            enabled && response.hovered(),
        );
        let active_t = animate_spring(ui.ctx(), response.id.with("chip_act"), active);

        let bg = if !enabled {
            BG_SURFACE
        } else {
            lerp_color(
                lerp_color(BG_SURFACE, BG_HOVER, hover_t),
                BG_ELEVATED,
                active_t,
            )
        };
        let border = if !enabled {
            BORDER_SUBTLE
        } else {
            lerp_color(
                lerp_color(BORDER, BORDER_STRONG, hover_t),
                FG_PRIMARY,
                active_t,
            )
        };
        let fg = if !enabled {
            FG_FAINT
        } else {
            lerp_color(
                lerp_color(FG_MUTED, FG_PRIMARY, hover_t),
                FG_PRIMARY,
                active_t,
            )
        };

        ui.painter().rect(
            rect,
            6.0,
            bg,
            Stroke::new(1.0_f32, border),
            egui::StrokeKind::Inside,
        );
        let galley =
            ui.painter()
                .layout_no_wrap(label.to_string(), egui::FontId::proportional(11.5), fg);
        let pos = egui::Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(pos, galley, fg);
    }

    response
}

/// Paint a crisp 7px status indicator dot.
pub fn status_dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().circle_filled(rect.center(), 3.5, color);
    }
}

/// Crisp section title without oversized heading margins.
pub fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(13.5).strong().color(FG_PRIMARY));
}

/// Fixed-width left-aligned, vertically centered field label for forms and spec rows.
pub fn field_label(ui: &mut egui::Ui, width: f32, height: f32, text: &str) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.add(egui::Label::new(RichText::new(text).size(12.0).color(FG_MUTED)).truncate());
        },
    );
}

/// Paint a restrained 1px vertical hairline divider of explicit height.
pub fn v_divider(ui: &mut egui::Ui, height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, height), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let x = rect.center().x;
        ui.painter().line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            Stroke::new(1.0_f32, BORDER),
        );
    }
}

/// Render a 30px-tall `shadcn/ui` single-line input or search bar with focus spring & optional clear button.
pub fn text_input(
    ui: &mut egui::Ui,
    id: egui::Id,
    value: &mut String,
    hint: &str,
    width: f32,
    search_style: bool,
    error_border: bool,
) -> egui::Response {
    let focused = ui.memory(|m| m.has_focus(id));
    let focus_t = animate_spring(ui.ctx(), id.with("focus_ring"), focused);
    let border = if error_border {
        DANGER_BORDER
    } else {
        lerp_color(BORDER, BORDER_FOCUS, focus_t)
    };
    let box_w = width.max(60.0);

    egui::Frame::new()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new((box_w - 16.0).max(40.0), 30.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if search_style {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
                        crate::icons::paint(
                            ui.painter(),
                            icon_rect,
                            crate::icons::Icon::Search,
                            FG_MUTED,
                        );
                    }
                    let clear_w = if search_style && !value.is_empty() {
                        24.0
                    } else {
                        0.0
                    };
                    let edit_w = (ui.available_width() - clear_w).max(32.0);
                    let mut resp = ui.add(
                        egui::TextEdit::singleline(value)
                            .id(id)
                            .frame(false)
                            .hint_text(hint)
                            .desired_width(edit_w),
                    );
                    let mut cleared = false;
                    if search_style
                        && resp.has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Escape))
                        && !value.is_empty()
                    {
                        value.clear();
                        cleared = true;
                    }
                    if search_style && !value.is_empty() {
                        let (clear_rect, clear_resp) =
                            ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::click());
                        let clear_resp = clear_resp
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Clear (Esc)");
                        if ui.is_rect_visible(clear_rect) {
                            let clr_fg = if clear_resp.hovered() {
                                ui.painter().rect_filled(clear_rect, 4.0, BG_HOVER);
                                FG_PRIMARY
                            } else {
                                FG_MUTED
                            };
                            let gal = ui.painter().layout_no_wrap(
                                "×".to_string(),
                                egui::FontId::proportional(13.0),
                                clr_fg,
                            );
                            ui.painter().galley(
                                egui::pos2(
                                    clear_rect.center().x - gal.size().x * 0.5,
                                    clear_rect.center().y - gal.size().y * 0.5,
                                ),
                                gal,
                                clr_fg,
                            );
                        }
                        if clear_resp.clicked() {
                            value.clear();
                            cleared = true;
                        }
                    }
                    if cleared {
                        resp.mark_changed();
                    }
                    resp
                },
            )
            .inner
        })
        .inner
}

/// Compact copy ghost button showing transient 1.5s `"Copied"` confirmation.
pub fn copy_feedback_button(ui: &mut egui::Ui, id_salt: &str, label: &str, text_to_copy: &str) {
    let mem_id = ui.make_persistent_id(("copy_feedback", id_salt));
    let now = ui.ctx().input(|i| i.time);
    let copied_at = ui.ctx().data(|d| d.get_temp::<f64>(mem_id));
    let recently_copied = copied_at.is_some_and(|t| now - t < 1.5);
    if recently_copied {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(200));
    }
    let (icon, text) = if recently_copied {
        (crate::icons::Icon::Check, "Copied")
    } else {
        (crate::icons::Icon::Copy, label)
    };
    if crate::icons::button(ui, icon, text, crate::icons::ButtonKind::Ghost, true).clicked() {
        ui.ctx().copy_text(text_to_copy.to_string());
        ui.ctx().data_mut(|d| d.insert_temp(mem_id, now));
    }
}

/// Aligned key-value row with a left-aligned, vertically centered muted key column.
pub fn kv_row(ui: &mut egui::Ui, key: &str, value: &str, mono: bool) {
    ui.horizontal(|ui| {
        field_label(ui, 116.0, 20.0, key);
        let mut rt = RichText::new(value).size(12.0).color(FG_PRIMARY);
        if mono {
            rt = rt.monospace();
        }
        ui.add(egui::Label::new(rt).truncate());
    });
}

/// Segmented 30px-tall two-option toggle pill (`Debug | Release`). Returns `true` when changed.
pub fn segmented_toggle(
    ui: &mut egui::Ui,
    id: &str,
    left: &str,
    right: &str,
    is_right: &mut bool,
) -> bool {
    let mut changed = false;
    let left_gal = ui.painter().layout_no_wrap(
        left.to_string(),
        egui::FontId::proportional(11.5),
        FG_PRIMARY,
    );
    let right_gal = ui.painter().layout_no_wrap(
        right.to_string(),
        egui::FontId::proportional(11.5),
        FG_PRIMARY,
    );
    let seg_w = (left_gal.size().x.max(right_gal.size().x) + 18.0).max(52.0);

    egui::Frame::new()
        .fill(BG_CANVAS)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.push_id(id, |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.horizontal(|ui| {
                    for (idx, text) in [left, right].into_iter().enumerate() {
                        let active = if idx == 0 { !*is_right } else { *is_right };
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::new(seg_w, 26.0), egui::Sense::click());
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                        if ui.is_rect_visible(rect) {
                            let hov_t = animate_spring(
                                ui.ctx(),
                                resp.id.with("seg_hov"),
                                !active && resp.hovered(),
                            );
                            let act_t = animate_spring(ui.ctx(), resp.id.with("seg_act"), active);
                            let bg = lerp_color(
                                lerp_color(Color32::TRANSPARENT, BG_ELEVATED, hov_t),
                                BG_HOVER,
                                act_t,
                            );
                            let border = lerp_color(Color32::TRANSPARENT, BORDER_STRONG, act_t);
                            let fg = lerp_color(
                                lerp_color(FG_MUTED, FG_SECONDARY, hov_t),
                                FG_PRIMARY,
                                act_t,
                            );
                            ui.painter().rect(
                                rect,
                                4.0,
                                bg,
                                Stroke::new(1.0_f32, border),
                                egui::StrokeKind::Inside,
                            );
                            if act_t > 0.05 {
                                let alpha = (11.0 * act_t).round() as u8;
                                let y = rect.top() + 1.0;
                                ui.painter().line_segment(
                                    [
                                        egui::pos2(rect.left() + 4.0, y),
                                        egui::pos2(rect.right() - 4.0, y),
                                    ],
                                    Stroke::new(
                                        1.0_f32,
                                        Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                                    ),
                                );
                            }
                            let gal = ui.painter().layout_no_wrap(
                                text.to_string(),
                                egui::FontId::proportional(11.5),
                                fg,
                            );
                            ui.painter().galley(
                                egui::pos2(
                                    rect.center().x - gal.size().x * 0.5,
                                    rect.center().y - gal.size().y * 0.5,
                                ),
                                gal,
                                fg,
                            );
                        }

                        if resp.clicked() && !active {
                            *is_right = idx == 1;
                            changed = true;
                        }
                    }
                });
            });
        });
    changed
}

/// Completion state for a step in [`workflow_stepper`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Done,
    Active,
    Pending,
}

/// Inline borderless architectural pipeline breadcrumb for embedded build stages.
pub fn workflow_stepper(ui: &mut egui::Ui, steps: &[(&str, StepState)]) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);
        for (idx, (label, state)) in steps.iter().enumerate() {
            let (circle_bg, circle_border, marker_fg, text_fg) = match state {
                StepState::Done => (BG_CANVAS, BORDER_STRONG, FG_PRIMARY, FG_SECONDARY),
                StepState::Active => (FG_PRIMARY, FG_PRIMARY, ACCENT_PRIMARY_FG, FG_PRIMARY),
                StepState::Pending => (BG_CANVAS, BORDER_SUBTLE, FG_FAINT, FG_FAINT),
            };

            let (circle_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
            if ui.is_rect_visible(circle_rect) {
                ui.painter().circle(
                    circle_rect.center(),
                    8.0,
                    circle_bg,
                    Stroke::new(1.0_f32, circle_border),
                );
                let marker = match state {
                    StepState::Done => "✓".to_string(),
                    _ => format!("{:02}", idx + 1),
                };
                let gal =
                    ui.painter()
                        .layout_no_wrap(marker, egui::FontId::monospace(8.5), marker_fg);
                ui.painter().galley(
                    egui::pos2(
                        circle_rect.center().x - gal.size().x * 0.5,
                        circle_rect.center().y - gal.size().y * 0.5,
                    ),
                    gal,
                    marker_fg,
                );
            }

            let mut lbl_rt = RichText::new(*label).size(11.0).color(text_fg);
            if *state == StepState::Active {
                lbl_rt = lbl_rt.strong();
            }
            ui.label(lbl_rt);

            if idx + 1 < steps.len() {
                let (line_rect, _) =
                    ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::hover());
                if ui.is_rect_visible(line_rect) {
                    let rule_col = if *state == StepState::Done {
                        BORDER_STRONG
                    } else {
                        BORDER_SUBTLE
                    };
                    ui.painter().line_segment(
                        [
                            egui::pos2(line_rect.left(), line_rect.center().y),
                            egui::pos2(line_rect.right(), line_rect.center().y),
                        ],
                        Stroke::new(1.0_f32, rule_col),
                    );
                }
            }
        }
    });
}

/// Smoothly interpolate two `Color32` values in sRGB space.
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t).round() as u8 };
    Color32::from_rgba_unmultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

/// Animate a boolean state with a critically-damped spring curve stored in egui memory.
/// Requests repaints automatically while in motion for 60/120fps smoothness.
pub fn animate_spring(ctx: &egui::Context, id: egui::Id, target: bool) -> f32 {
    let target_val = if target { 1.0_f32 } else { 0.0_f32 };
    let dt = ctx.input(|i| i.stable_dt).clamp(1.0 / 240.0, 1.0 / 30.0);

    let (mut pos, mut vel) = ctx.data_mut(|d| {
        d.get_temp::<(f32, f32)>(id)
            .unwrap_or((target_val, 0.0_f32))
    });

    let stiffness = 280.0_f32;
    let damping = 28.0_f32;
    let accel = (target_val - pos) * stiffness - vel * damping;
    vel += accel * dt;
    pos += vel * dt;

    if (target_val - pos).abs() < 0.002 && vel.abs() < 0.002 {
        pos = target_val;
        vel = 0.0;
    } else {
        ctx.request_repaint();
    }

    ctx.data_mut(|d| d.insert_temp(id, (pos, vel)));
    pos.clamp(0.0, 1.0)
}

/// Find merged, sorted byte ranges in `text` matching any lowercase term in `terms` (case-insensitive).
pub fn find_match_ranges(text: &str, terms: &[String]) -> Vec<(usize, usize)> {
    if text.is_empty() || terms.is_empty() {
        return Vec::new();
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let lower_chars: Vec<char> = chars.iter().map(|(_, c)| c.to_ascii_lowercase()).collect();

    let mut raw: Vec<(usize, usize)> = Vec::new();
    for term in terms {
        let needle: Vec<char> = term.chars().map(|c| c.to_ascii_lowercase()).collect();
        if needle.is_empty() || needle.len() > lower_chars.len() {
            continue;
        }
        let mut i = 0;
        while i + needle.len() <= lower_chars.len() {
            if lower_chars[i..i + needle.len()] == needle[..] {
                let start_byte = chars[i].0;
                let end_byte = if i + needle.len() < chars.len() {
                    chars[i + needle.len()].0
                } else {
                    text.len()
                };
                raw.push((start_byte, end_byte));
                i += needle.len();
            } else {
                i += 1;
            }
        }
    }

    if raw.is_empty() {
        return raw;
    }
    raw.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(raw.len());
    for (s, e) in raw {
        if let Some((_, last_e)) = merged.last_mut() {
            if s <= *last_e {
                *last_e = (*last_e).max(e);
                continue;
            }
        }
        merged.push((s, e));
    }
    merged
}

/// Build an `egui::text::LayoutJob` with Firefox-style inline keyword highlighting.
pub fn highlighted_job(
    text: &str,
    terms: &[String],
    size: f32,
    base_color: Color32,
    active_match: bool,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let base_fmt = egui::TextFormat {
        font_id: egui::FontId::proportional(size),
        color: base_color,
        ..Default::default()
    };
    let hit_fmt = egui::TextFormat {
        font_id: egui::FontId::proportional(size),
        color: if active_match { BG_CANVAS } else { MATCH_FG },
        background: if active_match {
            MATCH_ACTIVE_BG
        } else {
            MATCH_BG
        },
        ..Default::default()
    };

    let ranges = find_match_ranges(text, terms);
    let mut cursor = 0usize;
    for (start, end) in ranges {
        if start > cursor {
            job.append(&text[cursor..start], 0.0, base_fmt.clone());
        }
        job.append(&text[start..end], 0.0, hit_fmt.clone());
        cursor = end;
    }
    if cursor < text.len() {
        job.append(&text[cursor..], 0.0, base_fmt);
    }
    job
}

/// Pixel dimensions (width and height) of the embedded RavomIO cat icon (`assets/icon.rgba`).
pub const APP_ICON_SIZE: usize = 128;

const ICON_BYTES: &[u8] = include_bytes!("../assets/icon.rgba");

/// Embed the 128x128 RavomIO monochrome cat window icon (`assets/icon.rgba`).
pub fn app_icon() -> egui::IconData {
    egui::IconData {
        rgba: ICON_BYTES.to_vec(),
        width: APP_ICON_SIZE as u32,
        height: APP_ICON_SIZE as u32,
    }
}

/// Build an `egui::ColorImage` from the embedded 128x128 RavomIO brand icon.
pub fn brand_logo_image() -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([APP_ICON_SIZE, APP_ICON_SIZE], ICON_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_and_merges_case_insensitive_match_ranges() {
        let terms = vec!["esp32".to_string(), "32s3".to_string()];
        let ranges = find_match_ranges("Adafruit Feather ESP32S3", &terms);
        assert_eq!(ranges, vec![(17, 24)]);
        assert_eq!(&"Adafruit Feather ESP32S3"[17..24], "ESP32S3");
    }

    #[test]
    fn handles_unicode_without_panicking() {
        let terms = vec!["pico".to_string()];
        let text = "板卡 · Pico W";
        let ranges = find_match_ranges(text, &terms);
        assert_eq!(ranges.len(), 1);
        assert_eq!(&text[ranges[0].0..ranges[0].1], "Pico");
    }

    #[test]
    fn lerp_color_stays_in_bounds() {
        assert_eq!(lerp_color(BG_CANVAS, FG_PRIMARY, 0.0), BG_CANVAS);
        assert_eq!(lerp_color(BG_CANVAS, FG_PRIMARY, 1.0), FG_PRIMARY);
    }

    #[test]
    fn app_icon_has_expected_rgba_dimensions() {
        let icon = app_icon();
        assert_eq!(icon.width, APP_ICON_SIZE as u32);
        assert_eq!(icon.height, APP_ICON_SIZE as u32);
        assert_eq!(icon.rgba.len(), APP_ICON_SIZE * APP_ICON_SIZE * 4);
    }
}

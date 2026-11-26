//! Restrained, minimalist shadcn-inspired dark theme and reusable UI primitives.

use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Stroke, Vec2};

/// Zinc-based shadcn dark palette tokens.
pub const BG_BASE: Color32 = Color32::from_rgb(9, 9, 11); // #09090B
pub const BG_SURFACE: Color32 = Color32::from_rgb(15, 15, 18); // #0F0F12
pub const BG_MUTED: Color32 = Color32::from_rgb(24, 24, 27); // #18181B
pub const BG_HOVER: Color32 = Color32::from_rgb(39, 39, 42); // #27272A
pub const BG_ACTIVE: Color32 = Color32::from_rgb(63, 63, 70); // #3F3F46
pub const BORDER: Color32 = Color32::from_rgb(39, 39, 42); // #27272A
pub const BORDER_FOCUS: Color32 = Color32::from_rgb(113, 113, 122); // #71717A
pub const FG_PRIMARY: Color32 = Color32::from_rgb(250, 250, 250); // #FAFAFA
pub const FG_SECONDARY: Color32 = Color32::from_rgb(212, 212, 216); // #D4D4D8
pub const FG_MUTED: Color32 = Color32::from_rgb(161, 161, 170); // #A1A1AA
pub const ACCENT_OK: Color32 = Color32::from_rgb(52, 211, 153); // #34D399
/// Firefox-inspired match highlight tokens adapted for dark shadcn surfaces.
pub const MATCH_BG: Color32 = Color32::from_rgb(250, 204, 21); // #FACC15 (amber-400)
pub const MATCH_ACTIVE_BG: Color32 = Color32::from_rgb(52, 211, 153); // #34D399 (emerald-400)
pub const MATCH_FG: Color32 = BG_BASE; // #09090B
pub const DANGER_BORDER: Color32 = Color32::from_rgb(239, 68, 68); // #EF4444
pub const DANGER_FG: Color32 = Color32::from_rgb(248, 113, 113); // #F87171

/// Apply the shadcn-inspired dark theme to the egui context.
pub fn apply_shadcn_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(10.0, 5.0);
    style.spacing.window_margin = Margin::same(12);
    style.spacing.indent = 14.0;
    style.spacing.scroll = egui::style::ScrollStyle::thin();
    // Smooth 220ms transition for CollapsingHeader, hover, and widget state animations.
    style.animation_time = 0.22;

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_BASE;
    visuals.window_fill = BG_SURFACE;
    visuals.extreme_bg_color = BG_BASE;
    visuals.faint_bg_color = BG_MUTED;
    visuals.override_text_color = Some(FG_PRIMARY);

    let radius = CornerRadius::same(6);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = radius;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);

    visuals.selection.bg_fill = BG_HOVER;
    visuals.selection.stroke = Stroke::new(1.0_f32, FG_PRIMARY);

    visuals.widgets.noninteractive.bg_fill = BG_SURFACE;
    visuals.widgets.noninteractive.weak_bg_fill = BG_SURFACE;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, FG_MUTED);
    visuals.widgets.noninteractive.corner_radius = radius;

    visuals.widgets.inactive.bg_fill = BG_MUTED;
    visuals.widgets.inactive.weak_bg_fill = BG_MUTED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, FG_PRIMARY);
    visuals.widgets.inactive.corner_radius = radius;

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, BORDER_FOCUS);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, FG_PRIMARY);
    visuals.widgets.hovered.corner_radius = radius;

    visuals.widgets.active.bg_fill = BG_ACTIVE;
    visuals.widgets.active.weak_bg_fill = BG_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, FG_PRIMARY);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, FG_PRIMARY);
    visuals.widgets.active.corner_radius = radius;

    style.visuals = visuals;
    ctx.set_style(style);
}

/// Elevated card container matching shadcn Card.
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
}

/// Subtle inset container (e.g. console body, selected pill).
pub fn inset_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_BASE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(10, 7))
}

/// High-contrast primary button (white surface, dark text).
pub fn primary_button(label: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(label).color(BG_BASE).strong())
        .fill(FG_PRIMARY)
        .stroke(Stroke::new(1.0_f32, FG_PRIMARY))
        .corner_radius(CornerRadius::same(6))
}

/// Restrained secondary button (muted zinc surface, hairline border).
pub fn secondary_button(label: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(label).color(FG_PRIMARY))
        .fill(BG_MUTED)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(6))
}

/// Compact pill badge.
pub fn badge(ui: &mut egui::Ui, text: impl Into<String>, text_color: Color32) {
    egui::Frame::new()
        .fill(BG_MUTED)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text.into()).size(11.0).color(text_color));
        });
}

/// Crisp section title without oversized default heading margins.
pub fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(14.0).strong().color(FG_PRIMARY));
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

    // Underdamped-to-critically-damped spring (stiffness=280, damping=28 -> crisp, smooth glide).
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
        color: MATCH_FG,
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

/// Embed the 64x64 RavomIO cat window icon (`assets/icon.rgba`).
pub fn app_icon() -> egui::IconData {
    egui::IconData {
        rgba: include_bytes!("../assets/icon.rgba").to_vec(),
        width: 64,
        height: 64,
    }
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
        assert_eq!(lerp_color(BG_BASE, FG_PRIMARY, 0.0), BG_BASE);
        assert_eq!(lerp_color(BG_BASE, FG_PRIMARY, 1.0), FG_PRIMARY);
    }
}

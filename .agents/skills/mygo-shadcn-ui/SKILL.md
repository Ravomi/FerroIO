---
name: mygo-shadcn-ui
description: Design system and UI craft standard combining BanG Dream! MyGO!!!!! nocturnal minimalism with shadcn/ui component discipline for native desktop and egui interfaces. Use when designing, auditing, or de-cluttering developer tools, workbenches, dashboards, or Rust/egui applications.
---

# MyGO × shadcn/ui Design Skill (Pure Black & White Monochrome)

A calm, low-noise, Japanese atmospheric minimalism (`MyGO!!!!!`) fused with crisp structural discipline (`shadcn/ui`) in **strictly pure Black / White / Achromatic Gray (`R == G == B`)**. Designed for native desktop developer workbenches (`egui`, `eframe`, native GPUs) where **clarity, silence, luminance hierarchy, and generous whitespace** beat walls of text and color noise.

---

## 1. Visual Identity & Color Ladder (Pure Black & White)

Use pure black, neutral gray, and crisp white **exclusively** (`R == G == B` everywhere — **ZERO colored accents** such as blue, green, red, yellow, pink, or purple). Convey state, hierarchy, and focus strictly through luminance contrast, font weight, `1px` hairline borders, and `2px` pure-white (`#FAFAFA`) active selection bars.

### Surface & Border Ladder (Achromatic Dark)
- `BG_CANVAS`: `#090909` (`rgb(9, 9, 9)`) — Primary stage backdrop
- `BG_CHROME`: `#0D0D0D` (`rgb(13, 13, 13)`) — Unified titlebar, activity rail, bottom statusbar
- `BG_SURFACE`: `#121212` (`rgb(18, 18, 18)`) — Resting card / stat tile fill
- `BG_ELEVATED`: `#1A1A1A` (`rgb(26, 26, 26)`) — Active rows, popovers, selected cards
- `BG_HOVER`: `#262626` (`rgb(38, 38, 38)`) — Subtle pointer hover feedback
- `BG_ACTIVE`: `#383838` (`rgb(56, 56, 56)`) — Pressed state
- `BORDER_SUBTLE`: `#1F1F1F` (`rgb(31, 31, 31)`) — Default card hairline (`1px`)
- `BORDER`: `#2A2A2A` (`rgb(42, 42, 42)`) — Interactive control hairline
- `BORDER_STRONG`: `#444444` (`rgb(68, 68, 68)`) — Emphasized container border
- `BORDER_FOCUS`: `#D4D4D4` (`rgb(212, 212, 212)`) — Input focus hairline

### Ink & Monochrome Status / Trace Tokens
- `FG_PRIMARY`: `#FAFAFA` (`rgb(250, 250, 250)`) — Primary headings, values, primary CTA fill (`ACCENT_PRIMARY_FG = #090909`), live/connected dots, active `2px` selection bars, bold error emphasis
- `FG_SECONDARY`: `#D4D4D4` (`rgb(212, 212, 212)`) — Body labels, table text, secondary ready/paused indicators
- `FG_MUTED`: `#9C9C9C` (`rgb(156, 156, 156)`) — Micro uppercase labels, metadata, missing/idle indicators
- `FG_FAINT`: `#666666` (`rgb(102, 102, 102)`) — Placeholders, disabled text, empty states
- **Waveform / Chart Traces**: Use stepped grayscale luminance tiers (`#FAFAFA`, `#C0C0C0`, `#8C8C8C`, `#E0E0E0`, `#707070`, `#A8A8A8`, `#5A5A5A`, `#EEEEEE`) paired with hover crosshair readouts and legend chips—never rainbow hues.

---

## 2. De-Texting & 3-Second Scan Discipline

Developer tools feel bloated when every panel explains how it works in full sentences. Treat screen space like high-craft audio hardware:

1. **Ban Explanatory Paragraphs**: Delete multi-sentence subtitles and helper blocks inside cards. Move usage hints into `.on_hover_text(...)` tooltips or concise input `hint_text`.
2. **1–2 Word Labels Everywhere**: Prefer `"Scan"`, `"Init"`, `"Use"`, `"Live"`, `"Split"`, `"Scope"`, `"Raw"`, `"Deck"` over verbose phrases (`"Re-scan Environment"`, `"Initialize Project Here"`, `"Interactive Parameter Control Deck"`).
3. **Icon-First Utility Controls**: Secondary actions (`Refresh`, `Browse Folder`, `Clear Log`, `Reset Target`, `Copy Spec/Cmd`, `Export CSV`, `Pause`) must be **icon-only Ghost/Outline buttons with descriptive tooltips**, reserving text labels for Primary CTAs and tab names.
4. **Progressive Disclosure**: Keep creation forms (`+ Add Macro`, `+ Add Slider`, protocol snippets, DeviceTree raw editors, host warnings) tucked inside `CollapsingHeader` (closed by default) until needed.

---

## 3. Anti-Nesting & Spatial Serenity

1. **Zero Box-in-Box Cages**: Never nest bordered `card_frame` inside another `card_frame`. Use `16–20px` breathing whitespace, borderless inset wells, or a single `ui.separator()` hairline instead.
2. **Borderless-Until-Hover Rows**: Selectable items (toolchains, boards, packages, USB ports, environments) should rest flat on the canvas or subtle `BG_SURFACE` with zero heavy borders, transitioning via spring interpolation on hover and displaying a single `2px` `FG_PRIMARY` (`#FAFAFA`) vertical bar when selected.
3. **Visual Stat Tiles Over `kv_row` Walls**: Replace 6–8 stacked key-value text rows with a 2×2 or 4-column **Stat Tile strip** (`10pt` muted uppercase label + prominent `14–16pt` monospace value + optional `6px` status dot).
4. **Compact Chip Discipline**: Active filter/mode chips use `BG_ELEVATED` + `1px` `FG_PRIMARY` (`#FAFAFA`) hairline border (NOT solid white fills), keeping solid-white Primary CTA buttons visually sovereign.

---

## 4. `egui` / Rust Implementation Checklist

- **Right-to-Left First for Truncating Rows**: When pairing a right-aligned badge/button with left text or a `text_input`, always allocate RTL widgets first, compute `ui.available_width()`, and apply `.truncate()` on `egui::Label`.
- **Smooth Spring Feedback**: Drive hover, press, and selection states with critically-damped springs (`theme::animate_spring` + `theme::lerp_color`).
- **Explicit Float Precision**: Annotate `1.0_f32` on `egui::Stroke::new(...)` to satisfy strict Rust workspace lints.
- **Zero Borrow Traps**: Snapshot lightweight selections/flags or collect deferred click intents (`Vec<Action>`) before calling `&mut AppState` handlers.

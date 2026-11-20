//! Look and feel: a light "parchment and navy" theme close to the game's
//! own menus, with rounded cards and generous spacing.

use eframe::egui::{self, Color32, CornerRadius, Stroke, vec2};

/// Window and panel background (parchment).
pub const BACKGROUND: Color32 = Color32::from_rgb(0xEF, 0xE8, 0xD8);
/// Cards on top of the background.
pub const CARD: Color32 = Color32::from_rgb(0xFB, 0xF8, 0xF1);
/// Card borders and separators.
pub const BORDER: Color32 = Color32::from_rgb(0xDD, 0xD3, 0xBE);
/// Main text.
pub const INK: Color32 = Color32::from_rgb(0x2B, 0x2A, 0x33);
/// Secondary text (English names, labels).
pub const MUTED: Color32 = Color32::from_rgb(0x8A, 0x82, 0x70);
/// Accent: titles, selection.
pub const NAVY: Color32 = Color32::from_rgb(0x23, 0x31, 0x5A);
/// Selected row background.
pub const SELECTED: Color32 = Color32::from_rgb(0xDC, 0xE2, 0xF0);

/// Background color of a rank badge (G is plain, X is precious).
pub fn rank_color(rank: &str) -> Color32 {
    match rank {
        "G" => Color32::from_rgb(0x9A, 0x96, 0x8C),
        "F" => Color32::from_rgb(0x5E, 0x9E, 0x5A),
        "E" => Color32::from_rgb(0x3E, 0x9C, 0x9A),
        "D" => Color32::from_rgb(0x3D, 0x74, 0xC2),
        "C" => Color32::from_rgb(0x7B, 0x5C, 0xC4),
        "B" => Color32::from_rgb(0xD9, 0x82, 0x2B),
        "A" => Color32::from_rgb(0xCC, 0x3D, 0x3D),
        "S" => Color32::from_rgb(0xC9, 0x9A, 0x1E),
        "X" => Color32::from_rgb(0x2B, 0x2A, 0x33),
        _ => MUTED,
    }
}

/// Applies the theme once, at startup.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Light);

    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = CARD;
    visuals.extreme_bg_color = Color32::WHITE; // text fields
    visuals.faint_bg_color = CARD;
    visuals.override_text_color = Some(INK);
    visuals.hyperlink_color = NAVY;
    visuals.selection.bg_fill = SELECTED;
    visuals.selection.stroke = Stroke::new(1.0, NAVY);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::same(8);
    }
    visuals.widgets.inactive.weak_bg_fill = CARD;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    ctx.set_visuals(visuals);

    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = vec2(8.0, 6.0);
        style.spacing.button_padding = vec2(10.0, 5.0);
    });
}

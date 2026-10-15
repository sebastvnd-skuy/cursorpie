//! Look of the settings window: palette, geometry and the egui style applied to it.
//!
//! The look is a quiet, dense "tool" UI (think of a launcher for a suite of pro apps):
//!
//! - a graphite window with a rounded outline ([`CARD_RADIUS`]); inside it panels and controls
//!   are almost square ([`RADIUS`]) and separated by hairlines ([`BORDER`]), not by shadows;
//! - the window is cut into three bands: a top bar (brand, tabs and the buttons you confirm
//!   with), the body, and a footer (status). See `window.rs`;
//! - small monospace labels for chrome (tabs, panel titles, badges) and the normal proportional
//!   font for everything you read or edit;
//! - one accent color, chosen by the user, for the focus outline, the primary button and
//!   whatever is selected; everything else is neutral;
//! - one spacing unit ([`GAP`]) between panels and between controls, and one control height
//!   ([`CONTROL_H`]) for every button, chip, input and list row.
//!
//! Change the constants here to re-theme. Other files must not hard-code a color or a radius.

use eframe::egui::{self, Color32, CornerRadius, Rect, Stroke, Vec2};

// Settings palette (change here to re-theme). Neutral graphite with a very slight cool tint, so
// the user's accent is the only saturated color on screen. `contrast` pairs are checked by the
// tests at the bottom of this file: primary text keeps 7:1 on every surface, secondary text 4.5:1.
/// Window background.
pub const BG: Color32 = Color32::from_rgb(15, 16, 20);
/// Panels inside the window and the top bar.
pub const CARD: Color32 = Color32::from_rgb(21, 22, 27);
/// Controls at rest (buttons, tiles, chips).
pub const IDLE: Color32 = Color32::from_rgb(29, 30, 37);
/// Controls under the pointer.
pub const HOT: Color32 = Color32::from_rgb(40, 42, 51);
/// Text fields and the footer: darker than the panel they sit in.
pub const INPUT: Color32 = Color32::from_rgb(10, 11, 14);
/// Hairlines and borders.
pub const BORDER: Color32 = Color32::from_rgb(45, 47, 58);
/// Primary text.
pub const TEXT: Color32 = Color32::from_rgb(232, 233, 238);
/// Secondary text.
pub const MUTED: Color32 = Color32::from_rgb(150, 153, 168);
/// "OK" status.
pub const GOOD: Color32 = Color32::from_rgb(98, 222, 172);
/// Warning / error status.
pub const BAD: Color32 = Color32::from_rgb(255, 125, 145);
/// Text drawn on a light accent color (see [`on_accent`]).
const DARK_TEXT: Color32 = Color32::from_rgb(12, 13, 18);

// Geometry
/// Corner radius of panels and controls: just enough to take the edge off.
pub const RADIUS: CornerRadius = CornerRadius {
    nw: 2,
    ne: 2,
    sw: 2,
    se: 2,
};
/// Corner radius of the window itself, in points.
pub const CARD_RADIUS_PT: u8 = 12;
/// Corner radius of the window.
pub const CARD_RADIUS: CornerRadius = CornerRadius {
    nw: CARD_RADIUS_PT,
    ne: CARD_RADIUS_PT,
    sw: CARD_RADIUS_PT,
    se: CARD_RADIUS_PT,
};
/// Rounded top corners only: the top bar follows the window's outline.
pub const TOP_RADIUS: CornerRadius = CornerRadius {
    nw: CARD_RADIUS_PT,
    ne: CARD_RADIUS_PT,
    sw: 0,
    se: 0,
};
/// Rounded bottom corners only: the footer follows the window's outline.
pub const BOTTOM_RADIUS: CornerRadius = CornerRadius {
    nw: 0,
    ne: 0,
    sw: CARD_RADIUS_PT,
    se: CARD_RADIUS_PT,
};
/// Space between panels and between controls, in points.
pub const GAP: f32 = 8.0;
/// Height of every button, chip, input and list row, in points.
pub const CONTROL_H: f32 = 32.0;

/// Relative luminance of `c` (WCAG 2), 0 for black and 1 for white.
fn luminance(c: Color32) -> f32 {
    let lin = |c: u8| {
        let c = c as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let [r, g, b, _] = c.to_array();
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// WCAG contrast ratio of two colors: 1 (identical) to 21 (black on white).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Text color for something filled with `accent`: dark or white, whichever reads better.
/// The accent is chosen by the user, so a fixed white would be unreadable on a light one.
pub fn on_accent(accent: Color32) -> Color32 {
    if contrast(accent, DARK_TEXT) >= contrast(accent, Color32::WHITE) {
        DARK_TEXT
    } else {
        Color32::WHITE
    }
}

/// A panel frame with a hairline border.
pub(in crate::ui::settings) fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .corner_radius(RADIUS)
        .inner_margin(egui::Margin::same(12))
        .stroke(Stroke::new(1.0, BORDER))
}

/// The color of the window outline: the hairline color pulled toward `accent`, so the window
/// reads as focused without a loud frame.
pub fn window_outline_color(accent: Color32) -> Color32 {
    BORDER.lerp_to_gamma(accent, 0.45)
}

/// Draws the outline of the window just inside `rect`. It is an ordinary rounded rectangle
/// stroke, which egui anti-aliases; the previous gradient border was a hand-made triangle mesh
/// with stair-stepped edges, and it could not follow rounded corners.
pub(in crate::ui::settings) fn window_outline(
    painter: &egui::Painter,
    rect: Rect,
    accent: Color32,
) {
    painter.rect_stroke(
        rect,
        CARD_RADIUS,
        Stroke::new(1.0, window_outline_color(accent)),
        egui::StrokeKind::Inside,
    );
}

/// The shadow under the window. The window is transparent and larger than the card by
/// `WIN_MARGIN` on every side, which is where the shadow is painted.
pub(in crate::ui::settings) fn window_shadow() -> egui::Shadow {
    egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(120),
    }
}

/// Applies the dark Settings style (always dark, whatever macOS uses) with `accent` as highlight color.
pub(in crate::ui::settings) fn style_settings(ui: &mut egui::Ui, accent: Color32) {
    // force the dark theme so the card doesn't follow macOS light/dark
    *ui.visuals_mut() = egui::Visuals::dark();
    let v = ui.visuals_mut();
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.extreme_bg_color = INPUT;
    v.selection.bg_fill = accent.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, accent);
    v.hyperlink_color = accent;
    // popups (combo boxes, the profile menus) float above the card, so they keep a soft shadow
    v.window_corner_radius = RADIUS;
    v.menu_corner_radius = RADIUS;
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 14,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };
    v.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.5 };

    let w = &mut v.widgets;
    // no hover growth: a growing box looks like a layout shift
    for state in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        state.corner_radius = RADIUS;
        state.expansion = 0.0;
    }
    // separators and panel outlines; labels and button text use the primary text color
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    w.open.fg_stroke = Stroke::new(1.0, TEXT);
    // note: egui also uses this color for `.strong()` text, so it must stay light
    w.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    // at rest: grey with a hairline; under the pointer: lighter with an accent outline;
    // pressed: filled with the accent
    w.inactive.bg_fill = IDLE;
    w.inactive.weak_bg_fill = IDLE;
    w.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    w.hovered.bg_fill = HOT;
    w.hovered.weak_bg_fill = HOT;
    w.hovered.bg_stroke = Stroke::new(1.0, accent);
    w.active.bg_fill = accent;
    w.active.weak_bg_fill = accent;
    w.active.bg_stroke = Stroke::new(1.0, accent);
    w.open.bg_fill = HOT;
    w.open.weak_bg_fill = HOT;
    w.open.bg_stroke = Stroke::new(1.0, accent);

    let sp = ui.spacing_mut();
    sp.item_spacing = Vec2::splat(GAP);
    sp.button_padding = Vec2::new(12.0, 6.0);
    // one height for buttons, text fields and combo boxes
    sp.interact_size.y = CONTROL_H;
    sp.scroll = egui::style::ScrollStyle::solid();
}

#[cfg(test)]
mod tests {
    use super::*;

    const SURFACES: [(&str, Color32); 5] = [
        ("BG", BG),
        ("CARD", CARD),
        ("IDLE", IDLE),
        ("HOT", HOT),
        ("INPUT", INPUT),
    ];

    #[test]
    fn contrast_matches_the_wcag_extremes() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.05);
        assert!((contrast(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 1e-4);
        assert_eq!(
            contrast(BG, TEXT),
            contrast(TEXT, BG),
            "order does not matter"
        );
    }

    #[test]
    fn primary_text_is_readable_on_every_surface() {
        for (name, bg) in SURFACES {
            let c = contrast(TEXT, bg);
            assert!(c >= 7.0, "TEXT on {name}: {c:.1}:1");
        }
    }

    #[test]
    fn secondary_text_is_readable_on_every_surface() {
        for (name, bg) in SURFACES {
            let c = contrast(MUTED, bg);
            assert!(c >= 4.5, "MUTED on {name}: {c:.1}:1");
        }
    }

    #[test]
    fn status_colors_are_readable_on_the_footer_and_cards() {
        for (label, fg) in [("GOOD", GOOD), ("BAD", BAD)] {
            for (name, bg) in [("CARD", CARD), ("INPUT", INPUT)] {
                let c = contrast(fg, bg);
                assert!(c >= 4.5, "{label} on {name}: {c:.1}:1");
            }
        }
    }

    #[test]
    fn surfaces_get_lighter_from_input_to_hot() {
        let l = |c| luminance(c);
        assert!(l(INPUT) < l(BG) && l(BG) < l(CARD) && l(CARD) < l(IDLE) && l(IDLE) < l(HOT));
        assert!(
            contrast(BORDER, BG) > 1.3,
            "a hairline must be visible on the window"
        );
    }

    #[test]
    fn on_accent_is_readable_on_any_accent() {
        // the accent is user-chosen, so check the whole spread: saturated, light, dark, grey
        let accents = [
            Color32::from_rgb(110, 150, 255),
            Color32::from_rgb(255, 220, 60),
            Color32::from_rgb(20, 20, 20),
            Color32::from_rgb(240, 240, 240),
            Color32::from_rgb(128, 128, 128),
            Color32::from_rgb(230, 60, 60),
            Color32::from_rgb(60, 200, 120),
        ];
        for a in accents {
            let c = contrast(a, on_accent(a));
            assert!(c >= 4.0, "text on {a:?}: {c:.1}:1");
        }
        assert_eq!(on_accent(Color32::from_rgb(255, 255, 0)), DARK_TEXT);
        assert_eq!(on_accent(Color32::from_rgb(10, 10, 80)), Color32::WHITE);
    }

    #[test]
    fn the_window_outline_is_between_the_hairline_and_the_accent() {
        let accent = Color32::from_rgb(255, 0, 0);
        let c = window_outline_color(accent);
        assert!(
            c.r() > BORDER.r() && c.r() < accent.r(),
            "red moved toward the accent"
        );
        assert_eq!(window_outline_color(BORDER), BORDER, "no accent, no change");
    }

    #[test]
    fn corner_radii_fit_together() {
        assert_eq!(TOP_RADIUS.nw, CARD_RADIUS.nw);
        assert_eq!(TOP_RADIUS.sw, 0, "the bar is square where it meets the body");
        assert_eq!(BOTTOM_RADIUS.se, CARD_RADIUS.se);
        assert_eq!(BOTTOM_RADIUS.nw, 0);
        assert!(
            RADIUS.nw < CARD_RADIUS.nw,
            "controls are squarer than the window"
        );
    }
}

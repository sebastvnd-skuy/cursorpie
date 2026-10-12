//! Look of the settings window: palette, geometry and the egui style applied to it.
//!
//! The look follows tiling window managers such as Hyprland:
//!
//! - square corners everywhere ([`RADIUS`]) and flat fills inside panels;
//! - a hairline border ([`BORDER`]) around every panel and control;
//! - focus is shown with the accent color: a two-color border on the window
//!   ([`focus_border`]), an accent outline on hovered controls and selected tiles;
//! - one spacing unit ([`GAP`]) between panels and between controls, and one control height
//!   ([`CONTROL_H`]) for every button, chip, input and list row.
//!
//! Change the constants here to re-theme. Other files must not hard-code a color or a radius.

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Stroke, Vec2};

// Settings palette (change here to re-theme). Soft blue-violet darks with bright but not white
// text: easier on the eyes than pure black, and every text/background pair keeps a contrast above 7:1.
/// Window background.
pub const BG: Color32 = Color32::from_rgb(24, 25, 38);
/// Panels inside the window.
pub const CARD: Color32 = Color32::from_rgb(31, 33, 48);
/// Controls at rest (buttons, tiles, chips).
pub const IDLE: Color32 = Color32::from_rgb(41, 44, 63);
/// Controls under the pointer, and the selected sidebar entry.
pub const HOT: Color32 = Color32::from_rgb(55, 59, 84);
/// Text fields and the status bar: darker than the panel they sit in.
pub const INPUT: Color32 = Color32::from_rgb(19, 20, 31);
/// Borders.
pub const BORDER: Color32 = Color32::from_rgb(58, 62, 88);
/// Primary text.
pub const TEXT: Color32 = Color32::from_rgb(214, 220, 244);
/// Secondary text.
pub const MUTED: Color32 = Color32::from_rgb(150, 157, 186);
/// "OK" status.
pub const GOOD: Color32 = Color32::from_rgb(158, 220, 150);
/// Warning / error status.
pub const BAD: Color32 = Color32::from_rgb(240, 134, 160);
/// Text drawn on a light accent color (see [`on_accent`]).
const DARK_TEXT: Color32 = Color32::from_rgb(16, 17, 26);

// Geometry
/// Corner radius of every panel and control: none.
pub const RADIUS: CornerRadius = CornerRadius::ZERO;
/// Space between panels and between controls, in points.
pub const GAP: f32 = 8.0;
/// Height of every button, chip, input and list row, in points.
pub const CONTROL_H: f32 = 32.0;

/// Text color for something filled with `accent`: dark or white, whichever reads better.
/// The accent is chosen by the user, so a fixed white would be unreadable on a light one.
pub fn on_accent(accent: Color32) -> Color32 {
    let lin = |c: u8| {
        let c = c as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let [r, g, b, _] = accent.to_array();
    let l = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
    // WCAG contrast ratio against white and against DARK_TEXT (luminance about 0.006)
    if (l + 0.05) / 0.056 > 1.05 / (l + 0.05) {
        DARK_TEXT
    } else {
        Color32::WHITE
    }
}

/// A square panel frame with a hairline border.
pub(in crate::ui::settings) fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .corner_radius(RADIUS)
        .inner_margin(egui::Margin::same(12))
        .stroke(Stroke::new(1.0, BORDER))
}

/// The two colors of the focus border: `accent` and a greener shift of it (like Hyprland's
/// default cyan to green active border).
fn accent_pair(accent: Color32) -> (Color32, Color32) {
    let [r, g, b, _] = accent.to_array();
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * 0.5).round() as u8;
    (accent, Color32::from_rgb(m(r, 120), m(g, 255), m(b, 190)))
}

/// Draws a `width` px border just inside `rect`, fading diagonally from the accent to its
/// greener shift. egui strokes have one color, so the border is four mesh strips whose vertex
/// colors follow the diagonal.
pub(in crate::ui::settings) fn focus_border(
    painter: &egui::Painter,
    rect: Rect,
    width: f32,
    accent: Color32,
) {
    let (c0, c1) = accent_pair(accent);
    let size = rect.size().max(Vec2::splat(1.0));
    let at = |p: Pos2| -> Color32 {
        let t = ((p.x - rect.min.x) / size.x + (p.y - rect.min.y) / size.y) / 2.0;
        c0.lerp_to_gamma(c1, t.clamp(0.0, 1.0))
    };
    let (o, i) = (rect, rect.shrink(width));
    let strips = [
        [o.left_top(), o.right_top(), i.right_top(), i.left_top()],
        [
            o.right_top(),
            o.right_bottom(),
            i.right_bottom(),
            i.right_top(),
        ],
        [
            o.right_bottom(),
            o.left_bottom(),
            i.left_bottom(),
            i.right_bottom(),
        ],
        [o.left_bottom(), o.left_top(), i.left_top(), i.left_bottom()],
    ];
    let mut mesh = egui::Mesh::default();
    for quad in strips {
        let base = mesh.vertices.len() as u32;
        for p in quad {
            mesh.colored_vertex(p, at(p));
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    painter.add(egui::Shape::mesh(mesh));
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
    // popups (combo boxes, the profile menus) are flat and square too
    v.window_corner_radius = RADIUS;
    v.menu_corner_radius = RADIUS;
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    v.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.5 };

    let w = &mut v.widgets;
    // no radius and no hover growth: a growing square looks like a layout shift
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

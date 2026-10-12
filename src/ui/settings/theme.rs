//! Look of the settings window: palette and the egui style applied to it.

use eframe::egui::{self, Color32, CornerRadius, Stroke, Vec2};

// Settings palette (change here to re-theme)
/// Card background.
pub const BG: Color32 = Color32::from_rgb(15, 16, 21);
/// Panels inside the card.
pub const CARD: Color32 = Color32::from_rgb(24, 26, 33);
/// Borders.
pub const BORDER: Color32 = Color32::from_rgb(40, 43, 54);
/// Primary text.
pub const TEXT: Color32 = Color32::from_rgb(230, 232, 238);
/// Secondary text.
pub const MUTED: Color32 = Color32::from_rgb(140, 146, 160);
/// "OK" status.
pub const GOOD: Color32 = Color32::from_rgb(90, 200, 120);
/// Warning / error status.
pub const BAD: Color32 = Color32::from_rgb(235, 100, 90);

/// A rounded panel frame.
pub(in crate::ui::settings) fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(14))
        .stroke(Stroke::new(1.0, BORDER))
}

/// Applies the dark Settings style (always dark, whatever macOS uses) with `accent` as highlight color.
pub(in crate::ui::settings) fn style_settings(ui: &mut egui::Ui, accent: Color32) {
    // force the dark theme so the card doesn't follow macOS light/dark
    *ui.visuals_mut() = egui::Visuals::dark();
    let v = ui.visuals_mut();
    v.panel_fill = BG;
    v.extreme_bg_color = Color32::from_rgb(14, 15, 18);
    v.selection.bg_fill = accent.gamma_multiply(0.55);
    v.selection.stroke = Stroke::new(1.0, accent);

    let r = CornerRadius::same(10);
    let idle = Color32::from_rgb(38, 41, 49);
    let hot = Color32::from_rgb(54, 58, 70);
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.corner_radius = r;
    v.widgets.open.corner_radius = r;
    v.widgets.inactive.bg_fill = idle;
    v.widgets.inactive.weak_bg_fill = idle;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.bg_fill = hot;
    v.widgets.hovered.weak_bg_fill = hot;
    v.widgets.active.bg_fill = accent;
    v.widgets.active.weak_bg_fill = accent;

    let sp = ui.spacing_mut();
    sp.item_spacing = Vec2::new(8.0, 8.0);
    sp.button_padding = Vec2::new(14.0, 8.0);
}

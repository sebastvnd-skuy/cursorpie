//! Small widgets reused by the editors: chips, icon buttons, tiles, segmented controls.

use super::theme::*;
use crate::keys::Shortcut;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};

/// Small pill button "glyph  label". An empty label shows the glyph only.
pub(in crate::ui::settings) fn chip(
    ui: &mut egui::Ui,
    glyph: &str,
    label: &str,
    on: bool,
    accent: Color32,
) -> egui::Response {
    let text = match (glyph.is_empty(), label.is_empty()) {
        (true, _) => label.to_string(),
        (_, true) => glyph.to_string(),
        _ => format!("{glyph}  {label}"),
    };
    let b = egui::Button::new(RichText::new(text).color(if on { Color32::WHITE } else { MUTED }))
        .fill(if on {
            accent
        } else {
            Color32::from_rgb(30, 32, 40)
        })
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(9))
        .min_size(Vec2::new(0.0, 30.0));
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);
        // do not break the text letter by letter when little width is left in a horizontal_wrapped;
        // a chip that does not fit moves to the next row as a whole
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.add(b)
    })
    .inner
}

/// Square button with a single glyph and a tooltip.
pub(in crate::ui::settings) fn icon_btn(
    ui: &mut egui::Ui,
    glyph: &str,
    tip: &str,
    enabled: bool,
) -> egui::Response {
    let b = egui::Button::new(RichText::new(glyph).size(14.0)).min_size(Vec2::splat(32.0));
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = Vec2::new(4.0, 4.0);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.add_enabled(enabled, b)
    })
    .inner
    .on_hover_text(tip)
}

/// Clickable square. The caller draws its content into the returned `rect`.
pub(in crate::ui::settings) fn tile(
    ui: &mut egui::Ui,
    size: Vec2,
    selected: bool,
    accent: Color32,
) -> (egui::Rect, egui::Response) {
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let bg = if selected {
        accent.gamma_multiply(0.55)
    } else if resp.hovered() {
        Color32::from_rgb(54, 58, 70)
    } else {
        Color32::from_rgb(30, 32, 40)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(10), bg);
    if selected {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(10),
            Stroke::new(1.0, accent),
            egui::StrokeKind::Inside,
        );
    }
    (rect, resp)
}

// ------------------------------------------------------------------ layout helpers

/// Pill control "a | b | c" (replaces selectable_value / a small ComboBox).
pub(in crate::ui::settings) fn segmented(
    ui: &mut egui::Ui,
    cur: &mut usize,
    labels: &[&str],
    accent: Color32,
) -> bool {
    let mut changed = false;
    egui::Frame::new()
        .fill(Color32::from_rgb(10, 11, 14))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (i, l) in labels.iter().enumerate() {
                    let on = *cur == i;
                    let b = egui::Button::new(RichText::new(*l).strong().color(if on {
                        Color32::WHITE
                    } else {
                        MUTED
                    }))
                    .fill(if on { accent } else { Color32::TRANSPARENT })
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::same(9));
                    if ui.add(b).clicked() && !on {
                        *cur = i;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// A bold heading.
pub(in crate::ui::settings) fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(15.0).strong());
    ui.add_space(4.0);
}

/// A line of secondary text.
pub(in crate::ui::settings) fn muted(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(MUTED));
}

/// Like `segmented`, but centered in the row.
pub(in crate::ui::settings) fn segmented_centered(
    ui: &mut egui::Ui,
    cur: &mut usize,
    labels: &[&str],
    accent: Color32,
) -> bool {
    // measure the pill: text + padding of each button + spacing between buttons + Frame margin (3*2)
    let pad = ui.spacing().button_padding.x;
    let font = egui::TextStyle::Button.resolve(ui.style());
    let mut w = 6.0 + 2.0 * (labels.len().saturating_sub(1)) as f32;
    for l in labels {
        let g = ui
            .painter()
            .layout_no_wrap(l.to_string(), font.clone(), Color32::WHITE);
        w += g.size().x + pad * 2.0;
    }
    let spacing = ui.spacing().item_spacing.x;
    let indent = ((ui.available_width() - w) / 2.0 - spacing).max(0.0);

    ui.horizontal(|ui| {
        ui.add_space(indent);
        segmented(ui, cur, labels, accent)
    })
    .inner
}

/// Button showing a shortcut (or the "press any key" prompt while recording).
pub(in crate::ui::settings) fn shortcut_button(
    ui: &mut egui::Ui,
    sc: &Shortcut,
    recording: bool,
    accent: Color32,
) -> egui::Response {
    let btn = if recording {
        egui::Button::new(RichText::new("Press any key...  (Esc = cancel)").color(Color32::WHITE))
            .fill(accent)
    } else {
        egui::Button::new(RichText::new(sc.label()).monospace())
    };
    ui.add(btn.min_size(Vec2::new(210.0, 28.0)))
}

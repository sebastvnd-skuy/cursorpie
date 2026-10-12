//! Small widgets reused by the editors: chips, icon buttons, tiles, sidebar entries, panels, choices.
//!
//! All of them are square and take their colors from [`theme`](super::theme).

use super::theme::*;
use crate::keys::Shortcut;
use crate::ui::draw::ring_slice;
use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};

/// Small square button "glyph  label". An empty label shows the glyph only.
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
    // off: plain button (keeps the hover outline); on: filled with the accent
    let label_color = if on { on_accent(accent) } else { MUTED };
    let mut b = egui::Button::new(RichText::new(text).color(label_color))
        .corner_radius(RADIUS)
        .min_size(Vec2::new(0.0, CONTROL_H));
    if on {
        b = b.fill(accent).stroke(Stroke::new(1.0, accent));
    }
    // Do not wrap this in `ui.scope`: a nested Ui is not moved to the next row by
    // `horizontal_wrapped`, it just overflows and gets clipped.
    ui.add(b)
}

/// Square button with a single glyph and a tooltip.
pub(in crate::ui::settings) fn icon_btn(
    ui: &mut egui::Ui,
    glyph: &str,
    tip: &str,
    enabled: bool,
) -> egui::Response {
    let b = egui::Button::new(RichText::new(glyph).size(14.0)).min_size(Vec2::splat(CONTROL_H));
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = Vec2::new(4.0, 4.0);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.add_enabled(enabled, b)
    })
    .inner
    .on_hover_text(tip)
}

/// Clickable square. The caller draws its content into the returned `rect`.
///
/// The selected tile gets a 2 px accent border (the "focused window"), a hovered one a 1 px
/// accent outline.
pub(in crate::ui::settings) fn tile(
    ui: &mut egui::Ui,
    size: Vec2,
    selected: bool,
    accent: Color32,
) -> (egui::Rect, egui::Response) {
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let (bg, stroke) = if selected {
        (accent.gamma_multiply(0.3), Stroke::new(2.0, accent))
    } else if resp.hovered() {
        (HOT, Stroke::new(1.0, accent))
    } else {
        (IDLE, Stroke::new(1.0, BORDER))
    };
    ui.painter().rect_filled(rect, RADIUS, bg);
    ui.painter()
        .rect_stroke(rect, RADIUS, stroke, egui::StrokeKind::Inside);
    (rect, resp)
}

// ------------------------------------------------------------------ layout helpers

/// One entry of the sidebar: full width, a bar in the accent color on the left when selected.
pub(in crate::ui::settings) fn nav_item(
    ui: &mut egui::Ui,
    label: &str,
    on: bool,
    accent: Color32,
    width: f32,
) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(width, CONTROL_H + 4.0), egui::Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let bg = if on {
        HOT
    } else if resp.hovered() {
        IDLE
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter();
    painter.rect_filled(rect, RADIUS, bg);
    if on {
        let bar = egui::Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
        painter.rect_filled(bar, RADIUS, accent);
    }
    painter.text(
        egui::pos2(rect.min.x + 14.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        if on { TEXT } else { MUTED },
    );
    resp
}

/// A card heading: monospace, like the labels of a status bar.
pub(in crate::ui::settings) fn card_title(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .monospace()
            .size(13.0)
            .strong()
            .color(TEXT),
    );
}

/// A titled card that fills the width it is given, followed by one [`GAP`].
/// Every tab is a stack of these, so the spacing is the same everywhere.
pub(in crate::ui::settings) fn panel<R>(
    ui: &mut egui::Ui,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let out = card()
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            card_title(ui, title);
            add_contents(ui)
        })
        .inner;
    ui.add_space(GAP);
    out
}

/// A line of secondary text.
pub(in crate::ui::settings) fn muted(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(MUTED));
}

/// Square marker + label, the shared look of [`choice`] and [`toggle`]: a 14 px square with a
/// hairline border (accent when on or hovered). A choice shows a small filled square, a toggle
/// fills the whole marker and draws a check mark. The label keeps the primary text color so an
/// option that is off never looks disabled.
fn marker_row(
    ui: &mut egui::Ui,
    label: &str,
    on: bool,
    check: bool,
    accent: Color32,
) -> egui::Response {
    const MARKER: f32 = 14.0;
    let font = egui::TextStyle::Body.resolve(ui.style());
    let wrap = (ui.available_width() - MARKER - GAP).max(40.0);
    let galley = ui.painter().layout(label.to_string(), font, TEXT, wrap);
    let size = Vec2::new(MARKER + GAP + galley.size().x, galley.size().y.max(MARKER));
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let marker = egui::Rect::from_min_size(
        egui::pos2(
            rect.min.x,
            rect.min.y + (galley.size().y - MARKER).max(0.0) / 2.0 + 1.0,
        ),
        Vec2::splat(MARKER),
    );
    let outline = if on || resp.hovered() { accent } else { BORDER };
    let painter = ui.painter();
    if on && check {
        painter.rect_filled(marker, RADIUS, accent);
        let at = |x: f32, y: f32| Pos2::new(marker.min.x + x, marker.min.y + y);
        painter.add(egui::Shape::line(
            vec![at(3.5, 7.5), at(6.0, 10.0), at(10.5, 4.5)],
            Stroke::new(2.0, on_accent(accent)),
        ));
    } else {
        painter.rect_filled(marker, RADIUS, INPUT);
        if on {
            painter.rect_filled(marker.shrink(4.0), RADIUS, accent);
        }
    }
    painter.rect_stroke(
        marker,
        RADIUS,
        Stroke::new(1.0, outline),
        egui::StrokeKind::Inside,
    );
    painter.galley(egui::pos2(marker.right() + GAP, rect.min.y), galley, TEXT);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A square "radio": picks `value` for `cur`.
pub(in crate::ui::settings) fn choice<T: PartialEq>(
    ui: &mut egui::Ui,
    cur: &mut T,
    value: T,
    label: &str,
    accent: Color32,
) -> egui::Response {
    let on = *cur == value;
    let mut resp = marker_row(ui, label, on, false, accent);
    if resp.clicked() && !on {
        *cur = value;
        resp.mark_changed();
    }
    resp
}

/// A square checkbox: flips `value`.
pub(in crate::ui::settings) fn toggle(
    ui: &mut egui::Ui,
    value: &mut bool,
    label: &str,
    accent: Color32,
) -> egui::Response {
    let mut resp = marker_row(ui, label, *value, true, accent);
    if resp.clicked() {
        *value = !*value;
        resp.mark_changed();
    }
    resp
}

/// A single-line text field as high as every other control.
pub(in crate::ui::settings) fn text_field(text: &mut String) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(text)
        .min_size(Vec2::new(0.0, CONTROL_H))
        .vertical_align(egui::Align::Center)
        .margin(egui::Margin::symmetric(8, 4))
}

/// A row of plain text and links. Rows of text are 22 px high instead of [`CONTROL_H`], which is
/// meant for things you click.
pub(in crate::ui::settings) fn info_row<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = 22.0;
        ui.horizontal(add_contents).inner
    })
    .inner
}

/// The CursorPie logo: a five-slice pie with one slice in the accent color and a cursor in the
/// middle. Drawn with the same `ring_slice` as the real pie, so there is no image file to ship.
pub(in crate::ui::settings) fn draw_logo(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    accent: Color32,
) {
    use std::f32::consts::TAU;
    const SLICES: usize = 5;
    let step = TAU / SLICES as f32;
    let gap = 0.12; // radians between slices
    for i in 0..SLICES {
        let a0 = -TAU / 4.0 + step * i as f32 + gap / 2.0;
        let a1 = a0 + step - gap;
        let color = if i == 0 { accent } else { HOT };
        painter.add(ring_slice(center, radius * 0.42, radius, a0, a1, color));
    }
    // cursor arrow in the hub
    let p = |x: f32, y: f32| center + Vec2::new(x, y) * radius;
    painter.add(egui::Shape::convex_polygon(
        vec![p(-0.14, -0.24), p(-0.14, 0.2), p(0.18, 0.05)],
        TEXT,
        Stroke::NONE,
    ));
}

/// Button showing a shortcut (or the "press any key" prompt while recording).
pub(in crate::ui::settings) fn shortcut_button(
    ui: &mut egui::Ui,
    sc: &Shortcut,
    recording: bool,
    accent: Color32,
) -> egui::Response {
    let btn = if recording {
        egui::Button::new(
            RichText::new("Press any key...  (Esc = cancel)").color(on_accent(accent)),
        )
        .fill(accent)
    } else {
        egui::Button::new(RichText::new(sc.label()).monospace())
    };
    ui.add(btn.min_size(Vec2::new(210.0, CONTROL_H)))
}

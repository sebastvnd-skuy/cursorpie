//! Widget kecil yang dipakai ulang di editor: chip, tombol ikon, dan "tile" persegi.

use crate::gfx::*;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};

/// Tombol pil kecil "glyph  label". Label kosong = hanya glyph.
pub(in crate::settings) fn chip(
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
        .fill(if on { accent } else { Color32::from_rgb(30, 32, 40) })
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(9))
        .min_size(Vec2::new(0.0, 30.0));
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);
        // jangan pecah teks per huruf saat sisa lebar sempit di horizontal_wrapped;
        // chip yang tidak muat pindah utuh ke baris berikutnya
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.add(b)
    })
    .inner
}

/// Tombol persegi berisi satu glyph, dengan tooltip.
pub(in crate::settings) fn icon_btn(
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

/// Kotak persegi yang bisa diklik. Pemanggil menggambar isinya di `rect`.
pub(in crate::settings) fn tile(
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

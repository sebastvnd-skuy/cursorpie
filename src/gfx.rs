//! Palette, pie geometry (shared by hit-testing and drawing), and small egui helpers.

use crate::keys::Shortcut;
use core_graphics::display::CGDisplay;
use core_graphics::geometry::CGPoint;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, RichText, Shape, Stroke, Vec2,
};
use std::f32::consts::{PI, TAU};
use std::path::PathBuf;

// Settings palette (change here to re-theme)

pub const BG: Color32 = Color32::from_rgb(15, 16, 21);
pub const CARD: Color32 = Color32::from_rgb(24, 26, 33);
pub const BORDER: Color32 = Color32::from_rgb(40, 43, 54);
pub const TEXT: Color32 = Color32::from_rgb(230, 232, 238);
pub const MUTED: Color32 = Color32::from_rgb(140, 146, 160);
pub const GOOD: Color32 = Color32::from_rgb(90, 200, 120);
pub const BAD: Color32 = Color32::from_rgb(235, 100, 90);

// ------------------------------------------------------------------ geometry
pub fn pie_outer(win: f32) -> f32 {
    win / 2.0 - 10.0
}
pub fn pie_inner(win: f32) -> f32 {
    pie_outer(win) * 0.36
}
pub fn slice_start(i: usize, total: usize) -> f32 {
    let sl = TAU / total as f32;
    PI - sl / 2.0 + i as f32 * sl
}

/// Which slice is `p` over, relative to `center`? None inside the dead zone.
pub fn slice_at(center: CGPoint, p: CGPoint, total: usize, dead_zone: f32) -> Option<usize> {
    let dx = (p.x - center.x) as f32;
    let dy = (p.y - center.y) as f32;
    if dx.hypot(dy) < dead_zone || total == 0 {
        return None;
    }
    let mut angle = dy.atan2(dx);
    if angle < 0.0 {
        angle += TAU;
    }
    let slice_angle = TAU / total as f32;
    let mut adjusted = angle - slice_start(0, total);
    while adjusted < 0.0 {
        adjusted += TAU;
    }
    Some((adjusted / slice_angle).floor() as usize % total)
}

// --------------------------------------------------------------------- icons
/// If the icon string points to an image file, return its path. Otherwise it is text/emoji.
pub fn icon_path(s: &str) -> Option<PathBuf> {
    let s = s.trim();
    let lower = s.to_lowercase();
    let is_img = [".png", ".jpg", ".jpeg", ".webp"]
        .iter()
        .any(|e| lower.ends_with(e));
    if !is_img {
        return None;
    }
    if let Some(rest) = s.strip_prefix("~/") {
        return Some(crate::config::home().join(rest));
    }
    Some(PathBuf::from(s))
}

pub fn load_icon(ctx: &egui::Context, path: &PathBuf, name: &str) -> Option<egui::TextureHandle> {
    let img = image::open(path).ok()?;
    let rgba = img.thumbnail(128, 128).to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

/// Draws an icon (image or text) centered at `c` in a box of `size`.
pub fn draw_icon(
    painter: &egui::Painter,
    tex: Option<&egui::TextureHandle>,
    icon: &str,
    c: Pos2,
    size: f32,
    alpha: u8,
) {
    if let Some(t) = tex {
        let sz = t.size_vec2();
        let k = size / sz.x.max(sz.y).max(1.0);
        let rect = egui::Rect::from_center_size(c, sz * k);
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(t.id(), rect, uv, Color32::from_white_alpha(alpha));
    } else {
        // an image path that failed to load shows "?" so the problem is visible
        let text = if icon_path(icon).is_some() { "?" } else { icon };
        painter.text(
            c,
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(size * 0.8),
            Color32::from_white_alpha(alpha),
        );
    }
}

fn rect_of(b: core_graphics::geometry::CGRect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(b.origin.x as f32, b.origin.y as f32),
        egui::vec2(b.size.width as f32, b.size.height as f32),
    )
}

/// The display containing `p` (global macOS coordinates, same space as the cursor).
pub fn screen_rect_at(p: Pos2) -> Option<egui::Rect> {
    let ids = CGDisplay::active_displays().ok()?;
    let mut first = None;
    for id in ids {
        let r = rect_of(CGDisplay::new(id).bounds());
        if r.contains(p) {
            return Some(r);
        }
        first.get_or_insert(r);
    }
    first
}

// ------------------------------------------------------------------- drawing
pub fn smooth_approach(v: &mut f32, target: f32, dt: f32, speed: f32) -> bool {
    let diff = target - *v;
    if diff.abs() < 0.001 {
        *v = target;
        return false;
    }
    *v += diff * (1.0 - (-dt * speed).exp());
    true
}

/// Easing with a little overshoot at the end.
pub fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    let u = t - 1.0;
    1.0 + c3 * u * u * u + c1 * u * u
}

pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

pub fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

pub fn arc_points(c: Pos2, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let steps = (((a1 - a0).abs() / 0.06).ceil() as usize).max(2);
    (0..=steps)
        .map(|k| {
            let a = a0 + (a1 - a0) * k as f32 / steps as f32;
            c + Vec2::new(a.cos(), a.sin()) * r
        })
        .collect()
}

/// Donut slice built as a mesh (avoids non-convex polygon problems).
pub fn ring_slice(c: Pos2, r0: f32, r1: f32, a0: f32, a1: f32, color: Color32) -> Shape {
    let steps = (((a1 - a0).abs() / 0.07).ceil() as usize).max(2);
    let mut mesh = egui::Mesh::default();
    for k in 0..=steps {
        let a = a0 + (a1 - a0) * k as f32 / steps as f32;
        let d = Vec2::new(a.cos(), a.sin());
        mesh.colored_vertex(c + d * r0, color);
        mesh.colored_vertex(c + d * r1, color);
    }
    for k in 0..steps as u32 {
        let i = k * 2;
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i + 1, i + 3, i + 2);
    }
    Shape::mesh(mesh)
}

pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(14))
        .stroke(Stroke::new(1.0, BORDER))
}

/// Kontrol pill "a | b | c" (pengganti selectable_value / ComboBox kecil).
pub fn segmented(ui: &mut egui::Ui, cur: &mut usize, labels: &[&str], accent: Color32) -> bool {
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

pub fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(15.0).strong());
    ui.add_space(4.0);
}

pub fn muted(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(MUTED));
}

pub fn style_settings(ui: &mut egui::Ui, accent: Color32) {
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

pub fn shortcut_button(
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

//! Drawing helpers shared by the pie and the settings window: icons, shapes, easing, colors.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Shape, Vec2};
use std::path::PathBuf;

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

/// Loads an image file as a texture (downscaled to at most 128 px).
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

// ------------------------------------------------------------------- drawing
/// Moves `v` toward `target` exponentially (frame-rate independent). Returns `true` while still moving,
/// so the caller knows to request another repaint.
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

/// `[r, g, b]` as an egui color.
pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Linear blend of two colors; `t` = 0 gives `a`, 1 gives `b`.
pub fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

/// Points along an arc, for stroking progress arcs.
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

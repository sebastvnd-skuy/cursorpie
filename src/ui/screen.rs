//! Display lookup (global macOS coordinates, the same space as the cursor).

use core_graphics::display::CGDisplay;
use eframe::egui::{self, Pos2};

/// Converts a Core Graphics rectangle to an egui one.
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

//! Display lookup (global coordinates in logical points, the same space as the cursor).

use crate::platform::{self, ScreenRect};
use eframe::egui::{self, Pos2};

/// Converts display bounds to an egui rectangle.
fn rect_of(b: ScreenRect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(b.x as f32, b.y as f32),
        egui::vec2(b.w as f32, b.h as f32),
    )
}

/// The display containing `p`, or the first one when `p` is on none of them. `None` without displays.
pub fn pick_display(displays: &[ScreenRect], p: Pos2) -> Option<egui::Rect> {
    let mut first = None;
    for d in displays {
        let r = rect_of(*d);
        if r.contains(p) {
            return Some(r);
        }
        first.get_or_insert(r);
    }
    first
}

/// The display containing `p` (global coordinates, same space as the cursor).
pub fn screen_rect_at(p: Pos2) -> Option<egui::Rect> {
    pick_display(&platform::display_rects(), p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(x: f64, y: f64, w: f64, h: f64) -> ScreenRect {
        ScreenRect { x, y, w, h }
    }

    #[test]
    fn the_display_under_the_point_wins() {
        let v = [d(0.0, 0.0, 1440.0, 900.0), d(1440.0, 0.0, 1920.0, 1080.0)];
        let r = pick_display(&v, egui::pos2(2000.0, 500.0)).unwrap();
        assert_eq!(r.min, egui::pos2(1440.0, 0.0));
        assert_eq!(r.size(), egui::vec2(1920.0, 1080.0));
    }

    #[test]
    fn a_point_outside_every_display_uses_the_first() {
        let v = [d(0.0, 0.0, 100.0, 100.0), d(100.0, 0.0, 100.0, 100.0)];
        assert_eq!(
            pick_display(&v, egui::pos2(-50.0, -50.0)).unwrap().min,
            egui::pos2(0.0, 0.0)
        );
        assert!(pick_display(&[], egui::pos2(0.0, 0.0)).is_none());
    }

    #[test]
    fn displays_to_the_left_have_negative_coordinates() {
        let v = [d(0.0, 0.0, 1000.0, 800.0), d(-1280.0, 0.0, 1280.0, 1024.0)];
        let r = pick_display(&v, egui::pos2(-10.0, 10.0)).unwrap();
        assert_eq!(r.min.x, -1280.0);
    }
}

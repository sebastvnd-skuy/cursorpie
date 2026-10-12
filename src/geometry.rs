//! Pie geometry: sizes and slice hit-testing.
//!
//! Pure math with no drawing, shared by the event tap (which decides what is hovered) and the UI
//! (which draws it), so both always agree on where a slice is.
//!
//! Slice 0 is centered on the left (angle pi) and slices run clockwise on screen.
//!
//! ```
//! use cursorpie::geometry::{pie_inner, slice_at};
//! use core_graphics::geometry::CGPoint;
//!
//! let center = CGPoint::new(100.0, 100.0);
//! // inside the dead zone nothing is hovered
//! assert_eq!(slice_at(center, center, 4, pie_inner(200.0)), None);
//! // directly left of the center is slice 0
//! assert_eq!(slice_at(center, CGPoint::new(20.0, 100.0), 4, pie_inner(200.0)), Some(0));
//! ```

use core_graphics::geometry::CGPoint;
use std::f32::consts::{PI, TAU};

/// Outer radius of the ring for a window of size `win`.
pub fn pie_outer(win: f32) -> f32 {
    win / 2.0 - 10.0
}
/// Inner radius (the dead zone around the center disc).
pub fn pie_inner(win: f32) -> f32 {
    pie_outer(win) * 0.36
}
/// Start angle (radians) of slice `i` out of `total`.
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

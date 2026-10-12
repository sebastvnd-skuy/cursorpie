//! The gesture state machine's states.

use core_graphics::geometry::CGPoint;
use std::time::Instant;

/// What the trigger button is doing right now. Shared (behind a mutex) between the tap callback
/// and the short-lived timer threads; `id` ties a timer to the press that started it.
pub(super) enum Gesture {
    Idle,
    /// Pressed, nothing decided yet. `profile` = the pie opened on hold, `app` = the app's own
    /// profile (opened by a double-click), if it has one.
    Pending {
        id: u64,
        pos: CGPoint,
        profile: usize,
        app: Option<usize>,
    },
    /// First click released; waiting for a second press.
    WaitSecond {
        id: u64,
        pos: CGPoint,
        app: usize,
        at: Instant,
    },
    /// The pie is open and the button is still down.
    ActivePie {
        id: u64,
        center: CGPoint,
        profile: usize,
        path: Vec<usize>,
        hover: Option<usize>,
    },
    /// Settings was opened by holding; swallow the rest of the gesture.
    InSettings,
    /// The press was handed back to the app (a real drag); pass everything through until release.
    Passthrough,
}

//! Posting keys and mouse buttons with `CGEvent`.

use crate::actions::SYNTH_TAG;
use crate::geometry::Point;
use crate::keys::*;
use crate::tap::Kind;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGKeyCode, CGMouseButton, EventField,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use std::time::Duration;

/// Presses modifiers, the key, then releases everything in reverse order.
pub fn press_shortcut(sc: Shortcut) {
    let Ok(src) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else {
        return;
    };
    let none = CGEventFlags::CGEventFlagNull;
    let table: [(u8, CGKeyCode, CGEventFlags); 4] = [
        (MOD_CTRL, 59, CGEventFlags::CGEventFlagControl),
        (MOD_ALT, 58, CGEventFlags::CGEventFlagAlternate),
        (MOD_SHIFT, 56, CGEventFlags::CGEventFlagShift),
        (MOD_CMD, 55, CGEventFlags::CGEventFlagCommand),
    ];

    // (modifier keycode, cumulative flags once that modifier is down)
    let mut seq: Vec<(CGKeyCode, CGEventFlags)> = Vec::new();
    let mut acc = none;
    for (bit, code, flag) in table {
        if sc.mods & bit != 0 {
            acc |= flag;
            seq.push((code, acc));
        }
    }

    for (code, flags) in &seq {
        post_key(&src, *code, true, *flags);
    }
    post_key(&src, sc.key, true, acc);
    post_key(&src, sc.key, false, acc);
    for i in (0..seq.len()).rev() {
        let after = if i == 0 { none } else { seq[i - 1].1 };
        post_key(&src, seq[i].0, false, after);
    }
}

/// Posts one key event with the given modifier flags, tagged with [`SYNTH_TAG`].
fn post_key(src: &CGEventSource, code: CGKeyCode, down: bool, flags: CGEventFlags) {
    if let Ok(ev) = CGEvent::new_keyboard_event(src.clone(), code, down) {
        ev.set_flags(flags);
        ev.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, SYNTH_TAG);
        ev.post(CGEventTapLocation::HID);
        std::thread::sleep(Duration::from_millis(3));
    }
}

/// Re-posts a swallowed press or release of `button` at `pos` (used when a quick click must reach
/// the app after all). Only `Kind::Down` and `Kind::Up` are replayed.
pub fn post_mouse(pos: Point, kind: Kind, button: i64) {
    let ty = match kind {
        Kind::Down => CGEventType::OtherMouseDown,
        Kind::Up => CGEventType::OtherMouseUp,
        Kind::Drag => return,
    };
    let pos = CGPoint::new(pos.x, pos.y);
    if let Ok(src) = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        && let Ok(ev) = CGEvent::new_mouse_event(src, ty, pos, CGMouseButton::Center)
    {
        ev.set_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER, button);
        ev.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, 1);
        ev.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, SYNTH_TAG);
        ev.post(CGEventTapLocation::HID);
    }
}

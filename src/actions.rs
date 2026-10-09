//! Executes actions on a single worker thread so they always run in order.

use crate::config::Action;
use crate::keys::*;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGKeyCode, CGMouseButton, EventField,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use std::process::Command;
use std::sync::{OnceLock, mpsc};
use std::time::Duration;

/// Marks events we posted ourselves so the event tap ignores them.
pub const SYNTH_TAG: i64 = 0x504F_5055;

static TX: OnceLock<mpsc::Sender<Action>> = OnceLock::new();

pub fn init() {
    let (tx, rx) = mpsc::channel::<Action>();
    TX.set(tx).ok();
    std::thread::Builder::new()
        .name("cursorpie-actions".into())
        .spawn(move || {
            for action in rx {
                execute(&action);
            }
        })
        .expect("failed to spawn action worker");
}

/// Queue an action. Safe to call from the event tap callback (never blocks).
pub fn run(action: Action) {
    if matches!(action, Action::Nothing) {
        return;
    }
    if let Some(tx) = TX.get() {
        let _ = tx.send(action);
    }
}

fn execute(action: &Action) {
    match action {
        Action::Nothing => {}
        Action::Key(sc) => press(*sc),
        Action::OpenApp(app) if !app.trim().is_empty() => {
            Command::new("open").arg("-a").arg(app.trim()).spawn().ok();
        }
        Action::OpenUrl(target) if !target.trim().is_empty() => {
            Command::new("open").arg(target.trim()).spawn().ok();
        }
        Action::Command(cmd) if !cmd.trim().is_empty() => {
            Command::new("/bin/sh").arg("-c").arg(cmd).spawn().ok();
        }
        Action::Sequence(steps) => {
            for s in steps {
                execute(s);
                std::thread::sleep(Duration::from_millis(40));
            }
        }
        _ => {}
    }
}

/// Presses modifiers, the key, then releases everything in reverse order.
fn press(sc: Shortcut) {
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

fn post_key(src: &CGEventSource, code: CGKeyCode, down: bool, flags: CGEventFlags) {
    if let Ok(ev) = CGEvent::new_keyboard_event(src.clone(), code, down) {
        ev.set_flags(flags);
        ev.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, SYNTH_TAG);
        ev.post(CGEventTapLocation::HID);
        std::thread::sleep(Duration::from_millis(3));
    }
}

/// Re-posts a swallowed mouse press/release (used when a quick click must reach the app after all).
pub fn post_mouse(pos: CGPoint, ty: CGEventType, button: i64) {
    if let Ok(src) = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        && let Ok(ev) = CGEvent::new_mouse_event(src, ty, pos, CGMouseButton::Center)
    {
        ev.set_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER, button);
        ev.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, 1);
        ev.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, SYNTH_TAG);
        ev.post(CGEventTapLocation::HID);
    }
}

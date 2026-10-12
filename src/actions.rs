//! Executes [`Action`]s on a single worker thread so they always run in the order they were triggered.
//!
//! [`run_in`] only queues a job, so it is safe to call from the event-tap callback. The worker (started
//! by [`init`]) sends keys by posting `CGEvent`s, and shells out to `open` and `/bin/sh` for everything
//! else. Events posted here carry [`SYNTH_TAG`] so the event tap ignores them.

use crate::config::Action;
use crate::keys::*;
use crate::platform;
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

/// One queued unit of work.
struct Job {
    /// Bundle id of an app to bring to the front first (only done when the action sends keys).
    focus: Option<String>,
    action: Action,
}

/// Sender side of the worker queue; set once by [`init`].
static TX: OnceLock<mpsc::Sender<Job>> = OnceLock::new();

/// Starts the `cursorpie-actions` worker thread. Call once, before the first [`run_in`].
pub fn init() {
    let (tx, rx) = mpsc::channel::<Job>();
    TX.set(tx).ok();
    std::thread::Builder::new()
        .name("cursorpie-actions".into())
        .spawn(move || {
            for job in rx {
                if let Some(app) = job.focus.as_deref()
                    && sends_keys(&job.action)
                {
                    bring_to_front(app);
                }
                execute(&job.action);
            }
        })
        .expect("failed to spawn action worker");
}

/// Queue an action. Safe to call from the event tap callback (never blocks). If `focus` (a bundle id)
/// is given and the action sends keys, that app is brought to the front first, because keys always
/// go to the app in front.
pub fn run_in(focus: Option<String>, action: Action) {
    if matches!(action, Action::Nothing) {
        return;
    }
    if let Some(tx) = TX.get() {
        let _ = tx.send(Job { focus, action });
    }
}

/// Whether running `action` posts keystrokes (directly or inside a sequence).
fn sends_keys(action: &Action) -> bool {
    match action {
        Action::Key(_) => true,
        Action::Sequence(steps) => steps.iter().any(sends_keys),
        _ => false,
    }
}

/// Activates an app and waits (briefly) until macOS really has it in front: `open` returns
/// before the switch is done, and keys sent too early would reach the previous app.
fn bring_to_front(bundle: &str) {
    if platform::frontmost_bundle_id().as_deref() == Some(bundle) {
        return;
    }
    platform::spawn_reaped(Command::new("open").arg("-b").arg(bundle));
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(15));
        if platform::frontmost_bundle_id().as_deref() == Some(bundle) {
            std::thread::sleep(Duration::from_millis(40)); // let its window become the key window
            return;
        }
    }
}

/// Runs one action on the worker thread. A sequence waits 40 ms between steps.
fn execute(action: &Action) {
    match action {
        Action::Nothing => {}
        Action::Key(sc) => press(*sc),
        Action::OpenApp(app) if !app.trim().is_empty() => {
            platform::spawn_reaped(Command::new("open").arg("-a").arg(app.trim()));
        }
        Action::OpenUrl(target) if !target.trim().is_empty() => {
            platform::spawn_reaped(Command::new("open").arg(target.trim()));
        }
        Action::Command(cmd) if !cmd.trim().is_empty() => {
            platform::spawn_reaped(Command::new("/bin/sh").arg("-c").arg(cmd));
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

/// Posts one key event with the given modifier flags, tagged with [`SYNTH_TAG`].
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_key_actions_need_the_target_in_front() {
        assert!(sends_keys(&Action::Key(Shortcut::new(6, MOD_CMD))));
        assert!(sends_keys(&Action::Sequence(vec![
            Action::OpenApp("Safari".into()),
            Action::Key(Shortcut::new(6, 0)),
        ])));
        assert!(!sends_keys(&Action::Nothing));
        assert!(!sends_keys(&Action::OpenApp("Safari".into())));
        assert!(!sends_keys(&Action::Command("true".into())));
        assert!(!sends_keys(&Action::Sequence(vec![Action::OpenUrl(
            "https://example.com".into()
        )])));
    }
}

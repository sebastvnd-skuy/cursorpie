//! Global mouse/keyboard hook and the gesture state machine.
//!
//! The hook runs on its own thread (`cursorpie-tap`). Which hook depends on the OS and lives in a
//! small native module: `macos` (`CGEventTap` + `CFRunLoop`) or `windows` (`WH_MOUSE_LL` and
//! `WH_KEYBOARD_LL` + a message loop). The native layer turns each event into an `Ev` and calls
//! `dispatch`, which routes it and answers with a [`Verdict`]:
//!
//! | Event | Handled by |
//! |---|---|
//! | pause | the top of `dispatch` |
//! | key down / up | `keyboard`: only used to record a shortcut in Settings |
//! | right button | `right_click`: switches the side-button pie while a pie is open |
//! | side buttons (3, 4) | `side_buttons`: run the active mode's actions |
//! | the trigger button | `trigger`: the gesture state machine (`gesture::Gesture`) |
//!
//! `resolve` answers "what does this gesture mean": hovered slice, profile, target app, selection.
//! Button numbers are the macOS ones on every OS (0 left, 1 right, 2 middle, 3 back, 4 forward).
//!
//! **Safety rules for an active hook:** never block in the callback, never panic out of it (the
//! native layer wraps `dispatch` in `catch_unwind` and fails open), and pass events through when
//! unsure. The full list is in
//! [`guides::architecture`](crate::guides::architecture#rules-for-the-event-tap-callback).

mod gesture;
mod keyboard;
mod resolve;
mod right_click;
mod side_buttons;
mod trigger;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as native;

// Also compiled with `--features check-windows` so it can be type-checked and its pure helpers
// tested on other OSes.
#[cfg(any(windows, feature = "check-windows"))]
#[cfg_attr(not(windows), allow(dead_code))]
mod windows;
#[cfg(windows)]
use windows as native;

#[cfg(not(any(target_os = "macos", windows)))]
mod native {
    //! No hook on this OS (the Linux test stub): nothing is ever captured.
    use crate::msg::Notifier;

    /// Does nothing; `TAP_ACTIVE` stays false.
    pub fn spawn(_notifier: Notifier) {}
}

use crate::config::{cfg, lock};
use crate::geometry::Point;
use crate::msg::{Msg, Notifier};
use gesture::Gesture;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Button number of the right button.
const RIGHT_BUTTON: i64 = 1;
/// Button number of the "back" side button.
const SIDE_BACK: i64 = 3;
/// Button number of the "forward" side button.
const SIDE_FORWARD: i64 = 4;
/// Movement (px) before a press counts as a drag and is handed back to the app.
const DRAG_TOLERANCE: f64 = 6.0;
/// Same, when "flick" is enabled: the pie opens instead.
const FLICK_DISTANCE: f64 = 14.0;

/// true = the next key press is captured by the tap (recording a shortcut in settings)
pub static RECORDING: AtomicBool = AtomicBool::new(false);
/// keycode being swallowed until its key-up so it doesn't leak to apps (-1 = none)
pub static SWALLOW_KEY: AtomicI32 = AtomicI32::new(-1);
/// Pause switch from the menu bar: everything passes through while true.
pub static PAUSED: AtomicBool = AtomicBool::new(false);
/// true while the tap is installed and running.
pub static TAP_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Source of gesture ids; each press gets a new one so stale timers can recognise themselves.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// What a mouse button did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Pressed.
    Down,
    /// Released.
    Up,
    /// The pointer moved while the button was held.
    Drag,
}

/// What the hook does with an event: pass it on to the app or swallow it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Let the event through.
    Keep,
    /// Swallow it.
    Drop,
}

/// One event as seen by the portable state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Ev {
    /// A key press or release. `code` is a macOS virtual key code, `mods` a `MOD_*` bitmask.
    Key { down: bool, code: i32, mods: u8 },
    /// A mouse button event at `pos` (logical points).
    Mouse { kind: Kind, button: i64, pos: Point },
}

/// Starts the hook on its own thread (retrying while the OS refuses it).
pub fn spawn(notifier: Notifier) {
    native::spawn(notifier);
}

/// Routes one native event. `replay_drag` re-posts the original drag event to the app; it is
/// called when a press turns out to be a real drag that has to be handed back (macOS only needs it).
pub(crate) fn dispatch(
    state: &Arc<Mutex<Gesture>>,
    notifier: &Notifier,
    ev: Ev,
    replay_drag: &dyn Fn(),
) -> Verdict {
    if PAUSED.load(Ordering::SeqCst) {
        let mut g = lock(state);
        if !matches!(*g, Gesture::Idle) {
            *g = Gesture::Idle;
            drop(g);
            notifier.send(Msg::HidePie);
        }
        return Verdict::Keep;
    }
    match ev {
        Ev::Key { down, code, mods } => keyboard::handle_key(notifier, down, code, mods),
        Ev::Mouse { kind, button, pos } => {
            if button == RIGHT_BUTTON {
                return right_click::handle_right(state, notifier, kind);
            }
            if button == SIDE_BACK || button == SIDE_FORWARD {
                return side_buttons::handle_side(state, kind, button, pos);
            }
            let (trigger, flick) = {
                let c = cfg();
                (c.trigger_button, c.flick)
            };
            if button != trigger {
                return Verdict::Keep;
            }
            trigger::handle(state, notifier, kind, pos, button, flick, replay_drag)
        }
    }
}

/// Resets everything after the OS disabled the hook (a timeout, or the user): no gesture is in
/// progress any more and the pie is closed.
// Only the macOS tap calls this (the OS can disable its hook); the tests call it everywhere.
#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
pub(crate) fn reset(state: &Arc<Mutex<Gesture>>, notifier: &Notifier) {
    *lock(state) = Gesture::Idle;
    SWALLOW_KEY.store(-1, Ordering::SeqCst);
    notifier.send(Msg::HidePie);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::MOD_CMD;
    use std::sync::mpsc;
    use std::sync::{Mutex as StdMutex, MutexGuard};

    /// The statics above are process-wide, so tests that touch them run one at a time.
    static SERIAL: StdMutex<()> = StdMutex::new(());

    fn serial() -> MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn notifier() -> (Notifier, mpsc::Receiver<Msg>) {
        let (tx, rx) = mpsc::channel();
        (
            Notifier {
                tx,
                ctx: eframe::egui::Context::default(),
            },
            rx,
        )
    }

    fn idle() -> Arc<Mutex<Gesture>> {
        Arc::new(Mutex::new(Gesture::Idle))
    }

    fn key(down: bool, code: i32, mods: u8) -> Ev {
        Ev::Key { down, code, mods }
    }

    #[test]
    fn keys_pass_through_unless_recording() {
        let _s = serial();
        RECORDING.store(false, Ordering::SeqCst);
        SWALLOW_KEY.store(-1, Ordering::SeqCst);
        let (n, rx) = notifier();
        let st = idle();
        assert_eq!(
            dispatch(&st, &n, key(true, 6, MOD_CMD), &|| {}),
            Verdict::Keep
        );
        assert_eq!(
            dispatch(&st, &n, key(false, 6, MOD_CMD), &|| {}),
            Verdict::Keep
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn recording_captures_one_key_and_swallows_its_release() {
        let _s = serial();
        SWALLOW_KEY.store(-1, Ordering::SeqCst);
        RECORDING.store(true, Ordering::SeqCst);
        let (n, rx) = notifier();
        let st = idle();
        assert_eq!(
            dispatch(&st, &n, key(true, 6, MOD_CMD), &|| {}),
            Verdict::Drop
        );
        match rx.try_recv() {
            Ok(Msg::KeyCaptured(s)) => assert_eq!((s.key, s.mods), (6, MOD_CMD)),
            _ => panic!("expected the captured shortcut"),
        }
        assert!(!RECORDING.load(Ordering::SeqCst));
        // auto-repeat and the release of that key never reach the app
        assert_eq!(
            dispatch(&st, &n, key(true, 6, MOD_CMD), &|| {}),
            Verdict::Drop
        );
        assert_eq!(
            dispatch(&st, &n, key(false, 6, MOD_CMD), &|| {}),
            Verdict::Drop
        );
        assert_eq!(SWALLOW_KEY.load(Ordering::SeqCst), -1);
        assert_eq!(dispatch(&st, &n, key(true, 6, 0), &|| {}), Verdict::Keep);
    }

    #[test]
    fn plain_escape_cancels_recording() {
        let _s = serial();
        SWALLOW_KEY.store(-1, Ordering::SeqCst);
        RECORDING.store(true, Ordering::SeqCst);
        let (n, rx) = notifier();
        let st = idle();
        assert_eq!(dispatch(&st, &n, key(true, 53, 0), &|| {}), Verdict::Drop);
        assert!(matches!(rx.try_recv(), Ok(Msg::RecordCancelled)));
        SWALLOW_KEY.store(-1, Ordering::SeqCst);
    }

    #[test]
    fn pause_resets_the_gesture_and_passes_everything() {
        let _s = serial();
        PAUSED.store(true, Ordering::SeqCst);
        let (n, rx) = notifier();
        let st = Arc::new(Mutex::new(Gesture::InSettings));
        let ev = Ev::Mouse {
            kind: Kind::Down,
            button: 2,
            pos: Point::new(1.0, 1.0),
        };
        assert_eq!(dispatch(&st, &n, ev, &|| {}), Verdict::Keep);
        assert!(matches!(*lock(&st), Gesture::Idle));
        assert!(matches!(rx.try_recv(), Ok(Msg::HidePie)));
        PAUSED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn buttons_that_are_not_the_trigger_pass_through() {
        let _s = serial();
        PAUSED.store(false, Ordering::SeqCst);
        let (n, rx) = notifier();
        let st = idle();
        for button in [0, 5, 9] {
            for kind in [Kind::Down, Kind::Up, Kind::Drag] {
                let ev = Ev::Mouse {
                    kind,
                    button,
                    pos: Point::new(0.0, 0.0),
                };
                assert_eq!(
                    dispatch(&st, &n, ev, &|| {}),
                    Verdict::Keep,
                    "{button} {kind:?}"
                );
            }
        }
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn a_right_click_without_a_pie_is_a_normal_click() {
        let _s = serial();
        PAUSED.store(false, Ordering::SeqCst);
        let (n, _rx) = notifier();
        let st = idle();
        for kind in [Kind::Down, Kind::Up, Kind::Drag] {
            let ev = Ev::Mouse {
                kind,
                button: RIGHT_BUTTON,
                pos: Point::new(0.0, 0.0),
            };
            assert_eq!(dispatch(&st, &n, ev, &|| {}), Verdict::Keep);
        }
    }

    #[test]
    fn reset_closes_the_pie_and_clears_state() {
        let _s = serial();
        SWALLOW_KEY.store(12, Ordering::SeqCst);
        let (n, rx) = notifier();
        let st = Arc::new(Mutex::new(Gesture::Passthrough));
        reset(&st, &n);
        assert!(matches!(*lock(&st), Gesture::Idle));
        assert_eq!(SWALLOW_KEY.load(Ordering::SeqCst), -1);
        assert!(matches!(rx.try_recv(), Ok(Msg::HidePie)));
    }
}

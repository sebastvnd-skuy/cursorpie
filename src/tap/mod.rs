//! Global mouse/keyboard event tap and the gesture state machine.
//!
//! The tap runs on its own thread (`cursorpie-tap`) with a CoreFoundation run loop. [`spawn`] keeps
//! retrying every 2 seconds until the Accessibility permission is granted. Each event goes through
//! `handle`, which routes it:
//!
//! | Event | Handled by |
//! |---|---|
//! | tap disabled by macOS, own (synthetic) events, pause | the top of `handle` |
//! | key down / up | `keyboard`: only used to record a shortcut in Settings |
//! | right button | `right_click`: switches the side-button pie while a pie is open |
//! | side buttons (3, 4) | `side_buttons`: run the active mode's actions |
//! | the trigger button | `trigger`: the gesture state machine (`gesture::Gesture`) |
//!
//! `resolve` answers "what does this gesture mean": hovered slice, profile, target app, selection.
//!
//! **Safety rules for an active HID tap:** never block in the callback, never panic out of it (the
//! handler is wrapped in `catch_unwind` and fails open), and pass events through when unsure. The
//! full list is in [`guides::architecture`](crate::guides::architecture#rules-for-the-event-tap-callback).

mod gesture;
mod keyboard;
mod resolve;
mod right_click;
mod side_buttons;
mod trigger;

use crate::actions::SYNTH_TAG;
use crate::config::{cfg, lock};
use crate::msg::{Msg, Notifier};
use core_foundation::base::TCFType;
use core_foundation::mach_port::CFMachPortRef;
use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    CallbackResult, EventField,
};
use gesture::Gesture;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// macOS button number of the "back" side button.
const SIDE_BACK: i64 = 3;
/// macOS button number of the "forward" side button.
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
/// The tap's mach port, kept so the callback can re-enable the tap after macOS disables it.
static TAP_PORT: AtomicUsize = AtomicUsize::new(0);

// Not exposed by the `core-graphics` crate.
unsafe extern "C" {
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
}

// ------------------------------------------------------------------- startup

/// Starts the tap on its own thread and keeps retrying until the permission is granted.
pub fn spawn(notifier: Notifier) {
    std::thread::Builder::new()
        .name("cursorpie-tap".into())
        .spawn(move || {
            let state = Arc::new(Mutex::new(Gesture::Idle));
            let mut warned = false;
            loop {
                let (st, n) = (state.clone(), notifier.clone());
                let tap = CGEventTap::new(
                    CGEventTapLocation::HID,
                    CGEventTapPlacement::HeadInsertEventTap,
                    CGEventTapOptions::Default,
                    vec![
                        CGEventType::OtherMouseDown,
                        CGEventType::OtherMouseUp,
                        CGEventType::OtherMouseDragged,
                        CGEventType::RightMouseDown,
                        CGEventType::RightMouseUp,
                        CGEventType::RightMouseDragged,
                        CGEventType::KeyDown,
                        CGEventType::KeyUp,
                    ],
                    move |_proxy, etype, event| guarded(&st, &n, etype, event),
                );

                match tap {
                    Ok(tap) => {
                        let Ok(source) = tap.mach_port().create_runloop_source(0) else {
                            std::thread::sleep(Duration::from_secs(2));
                            continue;
                        };
                        TAP_PORT.store(tap.mach_port().as_concrete_TypeRef() as usize, Ordering::SeqCst);
                        CFRunLoop::get_current().add_source(&source, unsafe { kCFRunLoopCommonModes });
                        tap.enable();
                        TAP_ACTIVE.store(true, Ordering::SeqCst);
                        notifier.ctx.request_repaint();
                        CFRunLoop::run_current();
                        TAP_ACTIVE.store(false, Ordering::SeqCst);
                    }
                    Err(_) => {
                        TAP_ACTIVE.store(false, Ordering::SeqCst);
                        if !warned {
                            warned = true;
                            eprintln!(
                                "CursorPie: macOS blocked the event tap. Grant 'Accessibility' (and 'Input Monitoring') \
                                 to this app in System Settings -> Privacy & Security. Retrying every 2 s..."
                            );
                        }
                        notifier.ctx.request_repaint();
                    }
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        })
        .expect("failed to spawn event tap thread");
}

/// Runs the handler and turns a panic into "let the event through".
fn guarded(
    state: &Arc<Mutex<Gesture>>,
    n: &Notifier,
    t: CGEventType,
    e: &CGEvent,
) -> CallbackResult {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle(state, n, t, e)))
        .unwrap_or(CallbackResult::Keep)
}

/// The tap callback: decides what happens to one event (swallow = `Drop`, pass on = `Keep`).
fn handle(
    state: &Arc<Mutex<Gesture>>,
    notifier: &Notifier,
    etype: CGEventType,
    event: &CGEvent,
) -> CallbackResult {
    if matches!(
        etype,
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
    ) {
        let port = TAP_PORT.load(Ordering::SeqCst);
        if port != 0 {
            unsafe { CGEventTapEnable(port as CFMachPortRef, true) };
        }
        *lock(state) = Gesture::Idle;
        SWALLOW_KEY.store(-1, Ordering::SeqCst);
        notifier.send(Msg::HidePie);
        return CallbackResult::Keep;
    }
    if event.get_integer_value_field(EventField::EVENT_SOURCE_USER_DATA) == SYNTH_TAG {
        return CallbackResult::Keep;
    }
    if PAUSED.load(Ordering::SeqCst) {
        let mut g = lock(state);
        if !matches!(*g, Gesture::Idle) {
            *g = Gesture::Idle;
            drop(g);
            notifier.send(Msg::HidePie);
        }
        return CallbackResult::Keep;
    }
    if matches!(etype, CGEventType::KeyDown | CGEventType::KeyUp) {
        return keyboard::handle_key(notifier, etype, event);
    }

    if matches!(
        etype,
        CGEventType::RightMouseDown | CGEventType::RightMouseUp | CGEventType::RightMouseDragged
    ) {
        return right_click::handle_right(state, notifier, etype);
    }

    let button = event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
    if button == SIDE_BACK || button == SIDE_FORWARD {
        return side_buttons::handle_side(state, etype, button, event.location());
    }
    let (trigger, flick) = {
        let c = cfg();
        (c.trigger_button, c.flick)
    };
    if button != trigger {
        return CallbackResult::Keep;
    }

    trigger::handle(state, notifier, etype, event, button, flick)
}

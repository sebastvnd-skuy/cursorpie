//! The trigger button (middle button by default): press, hold, double-click, drag, release.
//!
//! [`handle`] looks at the current [`Gesture`] and hands the event to one small function per
//! transition. Every function that changes the state takes the lock guard by value, so it can
//! release the lock before posting messages or synthetic events (never do that while locked).

use super::gesture::Gesture;
use super::resolve::{eval, resolve_profiles, select};
use super::{DRAG_TOLERANCE, FLICK_DISTANCE, NEXT_ID};
use crate::actions::{SYNTH_TAG, post_mouse};
use crate::config::{cfg, lock};
use crate::msg::{Msg, Notifier};
use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CallbackResult, EventField};
use core_graphics::geometry::CGPoint;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Dispatches one event of the trigger button according to the current gesture state.
pub(super) fn handle(
    state: &Arc<Mutex<Gesture>>,
    notifier: &Notifier,
    etype: CGEventType,
    event: &CGEvent,
    button: i64,
    flick: bool,
) -> CallbackResult {
    let p = event.location();
    let g = lock(state);

    match (etype, &*g) {
        (CGEventType::OtherMouseDown, _) => on_press(state, notifier, g, p, button),
        (CGEventType::OtherMouseDragged, Gesture::Pending { .. }) => {
            on_drag_pending(notifier, g, event, p, button, flick)
        }
        (CGEventType::OtherMouseDragged, Gesture::ActivePie { .. }) => {
            on_drag_in_pie(notifier, g, p)
        }
        (CGEventType::OtherMouseUp, Gesture::Pending { .. }) => {
            on_release_pending(state, g, button)
        }
        (CGEventType::OtherMouseUp, Gesture::ActivePie { .. }) => on_release_in_pie(notifier, g, p),
        (CGEventType::OtherMouseUp, Gesture::InSettings) => {
            release_to_idle(g);
            CallbackResult::Drop
        }
        (CGEventType::OtherMouseUp, Gesture::Passthrough) => {
            release_to_idle(g);
            CallbackResult::Keep
        }
        (CGEventType::OtherMouseDragged, Gesture::Passthrough) => CallbackResult::Keep,
        (CGEventType::OtherMouseDragged, Gesture::InSettings) => CallbackResult::Drop,
        _ => CallbackResult::Keep,
    }
}

/// Resets the gesture to `Idle` (consumes the guard, releasing the lock).
fn release_to_idle(mut g: MutexGuard<'_, Gesture>) {
    *g = Gesture::Idle;
}

/// Button pressed. Either the second press of a double-click (open the app's pie at once) or a
/// fresh press (start the hold timers).
fn on_press<'a>(
    state: &'a Arc<Mutex<Gesture>>,
    notifier: &Notifier,
    mut g: MutexGuard<'a, Gesture>,
    p: CGPoint,
    button: i64,
) -> CallbackResult {
    let (hold_profile, app) = resolve_profiles(p);
    let dbl_ms = cfg().double_click_ms;

    // (A) second press inside the double-click window: open the app's own pie at once
    let prev = std::mem::replace(&mut *g, Gesture::Idle);
    let mut late: Option<CGPoint> = None;
    if let Gesture::WaitSecond {
        pos: first,
        app: ap,
        at,
        ..
    } = prev
    {
        if at.elapsed() < Duration::from_millis(dbl_ms) {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            *g = Gesture::ActivePie {
                id,
                center: p,
                profile: ap,
                path: Vec::new(),
                hover: None,
            };
            drop(g);
            notifier.send(Msg::ShowPie {
                x: p.x,
                y: p.y,
                profile: ap,
                path: Vec::new(),
            });
            return CallbackResult::Drop; // no "open Settings" timer for an app pie
        }
        late = Some(first); // too late: the first click still has to reach the app
    }
    if let Some(first) = late {
        drop(g);
        post_mouse(first, CGEventType::OtherMouseDown, button);
        post_mouse(first, CGEventType::OtherMouseUp, button);
        g = lock(state);
    }

    // (B) ordinary press: wait for the hold
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    *g = Gesture::Pending {
        id,
        pos: p,
        profile: hold_profile,
        app,
    };
    drop(g);
    spawn_hold_timer(state.clone(), notifier.clone(), id);
    CallbackResult::Drop
}

/// Timer thread for one press: opens the pie after `pie_hold_ms`, and Settings after
/// `settings_hold_ms` if the cursor is still in the center. It gives up silently as soon as the
/// gesture is no longer the one it started with (`id` differs or the state moved on).
fn spawn_hold_timer(st: Arc<Mutex<Gesture>>, n: Notifier, id: u64) {
    std::thread::spawn(move || {
        let (pie_ms, settings_ms) = {
            let c = cfg();
            (c.pie_hold_ms, c.settings_hold_ms)
        };

        std::thread::sleep(Duration::from_millis(pie_ms));
        // one lock per step (locking twice in one expression would deadlock)
        let opened = {
            let mut gg = lock(&st);
            let found = match &*gg {
                Gesture::Pending {
                    id: cur,
                    pos,
                    profile,
                    ..
                } if *cur == id => Some((*pos, *profile)),
                _ => None,
            };
            if let Some((pos, profile)) = found {
                *gg = Gesture::ActivePie {
                    id,
                    center: pos,
                    profile,
                    path: Vec::new(),
                    hover: None,
                };
            }
            found
        };
        let Some((pos, profile)) = opened else { return };
        n.send(Msg::ShowPie {
            x: pos.x,
            y: pos.y,
            profile,
            path: Vec::new(),
        });

        std::thread::sleep(Duration::from_millis(settings_ms.saturating_sub(pie_ms)));
        let open_settings = {
            let mut gg = lock(&st);
            let go = matches!(
                &*gg,
                Gesture::ActivePie { id: cur, hover: None, path, .. } if *cur == id && path.is_empty()
            );
            if go {
                *gg = Gesture::InSettings;
            }
            go
        };
        if open_settings {
            n.send(Msg::OpenSettings {
                x: pos.x,
                y: pos.y,
                about: false,
            });
        }
    });
}

/// After a quick click on an app with its own pie: if no second press arrives within `ms`, the
/// first click is replayed to the app.
fn spawn_double_timer(st: Arc<Mutex<Gesture>>, id: u64, button: i64, ms: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(ms));
        let replay = {
            let mut g = lock(&st);
            let found = match &*g {
                Gesture::WaitSecond { id: cur, pos, .. } if *cur == id => Some(*pos),
                _ => None,
            };
            if found.is_some() {
                *g = Gesture::Idle;
            }
            found
        };
        if let Some(p) = replay {
            post_mouse(p, CGEventType::OtherMouseDown, button);
            post_mouse(p, CGEventType::OtherMouseUp, button);
        }
    });
}

/// Moved while pressed, before the pie opened: either a real drag (hand it back to the app) or,
/// with "flick" enabled, a request to open the pie instantly.
fn on_drag_pending(
    notifier: &Notifier,
    mut g: MutexGuard<'_, Gesture>,
    event: &CGEvent,
    p: CGPoint,
    button: i64,
    flick: bool,
) -> CallbackResult {
    let Gesture::Pending {
        id, pos, profile, ..
    } = &*g
    else {
        return CallbackResult::Drop;
    };
    let (id, start, profile) = (*id, *pos, *profile);
    let limit = if flick {
        FLICK_DISTANCE
    } else {
        DRAG_TOLERANCE
    };
    if (p.x - start.x).hypot(p.y - start.y) > limit {
        if flick {
            // dragging right away opens the pie instantly
            *g = Gesture::ActivePie {
                id,
                center: start,
                profile,
                path: Vec::new(),
                hover: None,
            };
            drop(g);
            notifier.send(Msg::ShowPie {
                x: start.x,
                y: start.y,
                profile,
                path: Vec::new(),
            });
        } else {
            // a real drag: hand the gesture back to the app (e.g. panning)
            *g = Gesture::Passthrough;
            drop(g);
            post_mouse(start, CGEventType::OtherMouseDown, button);
            let drag = event.clone();
            drag.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, SYNTH_TAG);
            drag.post(CGEventTapLocation::HID);
        }
    }
    CallbackResult::Drop
}

/// Moved while the pie is open: update the hovered slice, or enter a submenu when the cursor
/// crosses the rim over one.
fn on_drag_in_pie(
    notifier: &Notifier,
    mut g: MutexGuard<'_, Gesture>,
    p: CGPoint,
) -> CallbackResult {
    let Gesture::ActivePie {
        center,
        profile,
        path,
        hover,
        ..
    } = &mut *g
    else {
        return CallbackResult::Drop;
    };
    let hit = eval(*profile, path, *center, p);
    if hit.enter {
        if let Some(h) = hit.hover {
            // crossed the rim over a submenu: open it, re-centered on the cursor
            path.push(h);
            *center = p;
            *hover = None;
            let (pr, pa) = (*profile, path.clone());
            drop(g);
            notifier.send(Msg::ShowPie {
                x: p.x,
                y: p.y,
                profile: pr,
                path: pa,
            });
        }
    } else if hit.hover != *hover {
        *hover = hit.hover;
        drop(g);
        notifier.send(Msg::Hover(hit.hover));
    }
    CallbackResult::Drop
}

/// Released before the pie opened: a click. Pass it on, or wait for a second click if the app
/// has its own pie.
fn on_release_pending(
    state: &Arc<Mutex<Gesture>>,
    mut g: MutexGuard<'_, Gesture>,
    button: i64,
) -> CallbackResult {
    let Gesture::Pending { id, pos, app, .. } = &*g else {
        return CallbackResult::Drop;
    };
    let (id, start, app) = (*id, *pos, *app);
    match app {
        Some(app) => {
            // this app has its own profile: wait for a possible second click
            *g = Gesture::WaitSecond {
                id,
                pos: start,
                app,
                at: Instant::now(),
            };
            drop(g);
            spawn_double_timer(state.clone(), id, button, cfg().double_click_ms);
        }
        None => {
            *g = Gesture::Idle;
            drop(g);
            post_mouse(start, CGEventType::OtherMouseDown, button);
            post_mouse(start, CGEventType::OtherMouseUp, button);
        }
    }
    CallbackResult::Drop
}

/// Released while the pie is open: run the slice under the cursor (if any) and close the pie.
fn on_release_in_pie(
    notifier: &Notifier,
    mut g: MutexGuard<'_, Gesture>,
    p: CGPoint,
) -> CallbackResult {
    let Gesture::ActivePie {
        center,
        profile,
        path,
        ..
    } = &*g
    else {
        return CallbackResult::Drop;
    };
    let (center, profile, path) = (*center, *profile, path.clone());
    *g = Gesture::Idle;
    drop(g);
    if let Some(h) = eval(profile, &path, center, p).hover {
        select(profile, &path, h);
    }
    notifier.send(Msg::HidePie);
    CallbackResult::Drop
}

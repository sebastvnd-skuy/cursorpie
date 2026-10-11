//! Global mouse/keyboard event tap and the gesture state machine.
//!
//! Safety rules for an active HID tap: never block in the callback, never panic out of it
//! (we `catch_unwind` and fail open), and keep retrying if the permission is missing.

use crate::actions::{self, SYNTH_TAG, post_mouse};
use crate::config::{
    self, Action, AppGesture, Item, ItemKind, MAX_DEPTH, TargetApp, cfg, item_at, items_at, lock,
};
use crate::gfx::{pie_inner, pie_outer, slice_at};
use crate::keys::{Shortcut, mods_from_flags};
use crate::msg::{Msg, Notifier};
use crate::side;
use crate::system;
use core_foundation::base::TCFType;
use core_foundation::mach_port::CFMachPortRef;
use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    CallbackResult, EventField,
};
use core_graphics::geometry::CGPoint;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SIDE_BACK: i64 = 3;
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

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static TAP_PORT: AtomicUsize = AtomicUsize::new(0);
/// Whether the last side-button press (back, forward) was swallowed, so its release is too.
static SIDE_SWALLOWED: [AtomicBool; 2] = [AtomicBool::new(false), AtomicBool::new(false)];
/// Whether the last right-press (used to switch the side-button pie) was swallowed, so its release is too.
static RIGHT_SWALLOWED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" {
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
}

enum Gesture {
    Idle,
    // `profile` = yang dibuka saat TAHAN, `app` = profil khusus app (untuk klik dua kali)
    Pending {
        id: u64,
        pos: CGPoint,
        profile: usize,
        app: Option<usize>,
    },
    // klik pertama sudah dilepas, menunggu tekan kedua
    WaitSecond {
        id: u64,
        pos: CGPoint,
        app: usize,
        at: Instant,
    },
    ActivePie {
        id: u64,
        center: CGPoint,
        profile: usize,
        path: Vec<usize>,
        hover: Option<usize>,
    },
    InSettings,
    Passthrough,
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

fn guarded(
    state: &Arc<Mutex<Gesture>>,
    n: &Notifier,
    t: CGEventType,
    e: &CGEvent,
) -> CallbackResult {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle(state, n, t, e)))
        .unwrap_or(CallbackResult::Keep)
}

// ------------------------------------------------------------------- helpers

struct Hit {
    hover: Option<usize>,
    /// cursor crossed the rim over a submenu slice
    enter: bool,
}

fn eval(profile: usize, path: &[usize], center: CGPoint, p: CGPoint) -> Hit {
    let c = cfg();
    let win = c.pie_size;
    let Some(items) = c
        .profiles
        .get(profile)
        .and_then(|pr| items_at(&pr.items, path))
    else {
        return Hit {
            hover: None,
            enter: false,
        };
    };
    let hover = slice_at(center, p, items.len(), pie_inner(win));
    let dist = ((p.x - center.x) as f32).hypot((p.y - center.y) as f32);
    let enter = match hover.and_then(|h| items.get(h)) {
        Some(Item {
            kind: ItemKind::Submenu(ch),
            ..
        }) => !ch.is_empty() && dist > pie_outer(win) && path.len() + 1 < MAX_DEPTH,
        _ => false,
    };
    Hit { hover, enter }
}

/// Applies the slice the user released over.
fn select(profile: usize, path: &[usize], index: usize) {
    let kind = {
        let c = cfg();
        c.profiles
            .get(profile)
            .and_then(|pr| items_at(&pr.items, path))
            .and_then(|items| items.get(index))
            .map(|i| i.kind.clone())
    };
    match kind {
        Some(ItemKind::Mode { .. }) => {
            let mut full = path.to_vec();
            full.push(index);
            config::set_active(profile, full);
            // the side buttons now follow the pie this mode was picked in
            let target = side::target();
            let c = cfg();
            if c.side_adopt {
                side::adopt(&c, target.as_deref(), profile);
            }
        }
        Some(ItemKind::Run(action)) => run_action(action, side::target()),
        _ => {}
    }
}

/// Runs an action. If the settings say so, the target app is brought to the front first, because
/// shortcuts always go to the app in front.
fn run_action(action: Action, target: Option<String>) {
    let focus = {
        let c = cfg();
        c.target_app == TargetApp::UnderCursor && c.focus_target
    };
    actions::run_in(if focus { target } else { None }, action);
}

/// The app a gesture at `p` is about: the app in front, or the app whose window is under `p`
/// (falling back to the app in front when that cannot be told).
fn target_bundle_at(p: CGPoint) -> Option<String> {
    let mode = cfg().target_app;
    match mode {
        TargetApp::Focused => system::frontmost_bundle_id(),
        TargetApp::UnderCursor => system::bundle_id_at(p).or_else(system::frontmost_bundle_id),
    }
}

/// Remembers the target app for this gesture (the UI and `select` read it back from `side`).
fn remember_target(bundle: Option<String>) {
    let forget = cfg().side_forget;
    side::set_target(bundle, forget);
}

/// (profil untuk TAHAN, profil khusus app untuk KLIK DUA KALI)
fn resolve_profiles(p: CGPoint) -> (usize, Option<usize>) {
    let bundle = target_bundle_at(p);
    remember_target(bundle.clone());
    let c = cfg();
    match c.app_gesture {
        AppGesture::Merged => (c.profile_for(bundle.as_deref()), None),
        AppGesture::DoubleClick => (c.global_profile(), c.app_profile_for(bundle.as_deref())),
    }
}

/// Right-click while the pie is open: switch which pie (general / the app's own) the side buttons
/// follow. The pie stays open and its ring changes color right away.
fn handle_right(
    state: &Arc<Mutex<Gesture>>,
    notifier: &Notifier,
    etype: CGEventType,
) -> CallbackResult {
    match etype {
        CGEventType::RightMouseDown => {
            let open = {
                let g = lock(state);
                match &*g {
                    Gesture::ActivePie { hover, .. } => Some(*hover),
                    _ => None,
                }
            };
            let Some(hover) = open else {
                return CallbackResult::Keep; // no pie open: a normal right-click
            };
            let enabled = cfg().right_click_switch;
            if !enabled {
                return CallbackResult::Keep;
            }
            RIGHT_SWALLOWED.store(true, Ordering::SeqCst);
            let target = side::target();
            let switched = {
                let c = cfg();
                side::toggle(&c, target.as_deref())
            };
            if switched.is_some() {
                notifier.send(Msg::Hover(hover)); // wakes the UI so the ring is redrawn
            }
            CallbackResult::Drop
        }
        CGEventType::RightMouseUp => {
            if RIGHT_SWALLOWED.swap(false, Ordering::SeqCst) {
                CallbackResult::Drop
            } else {
                CallbackResult::Keep
            }
        }
        _ => {
            if RIGHT_SWALLOWED.load(Ordering::SeqCst) {
                CallbackResult::Drop
            } else {
                CallbackResult::Keep
            }
        }
    }
}

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
// ------------------------------------------------------------------ handlers

fn handle_key(notifier: &Notifier, etype: CGEventType, event: &CGEvent) -> CallbackResult {
    let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as i32;
    let swallow = SWALLOW_KEY.load(Ordering::SeqCst);

    if matches!(etype, CGEventType::KeyUp) {
        if swallow == code {
            SWALLOW_KEY.store(-1, Ordering::SeqCst);
            return CallbackResult::Drop;
        }
        return CallbackResult::Keep;
    }

    // KeyDown (including auto-repeat of the key that was just recorded)
    if swallow == code {
        return CallbackResult::Drop;
    }
    if !RECORDING.load(Ordering::SeqCst) {
        return CallbackResult::Keep;
    }

    RECORDING.store(false, Ordering::SeqCst);
    SWALLOW_KEY.store(code, Ordering::SeqCst);
    let mods = mods_from_flags(event.get_flags());
    if code == 53 && mods == 0 {
        notifier.send(Msg::RecordCancelled); // plain Esc cancels
    } else {
        notifier.send(Msg::KeyCaptured(Shortcut::new(code as u16, mods)));
    }
    CallbackResult::Drop
}

/// Back/forward side buttons: send the active (or hovered) mode's actions, otherwise pass through.
/// Which pie they follow is decided by `side` (see there); `p` is where the pointer is.
fn handle_side(
    state: &Arc<Mutex<Gesture>>,
    etype: CGEventType,
    button: i64,
    p: CGPoint,
) -> CallbackResult {
    let idx = if button == SIDE_BACK { 0 } else { 1 };
    match etype {
        CGEventType::OtherMouseDown => {
            let (pie_profile, hovered) = {
                let g = lock(state);
                match &*g {
                    Gesture::ActivePie {
                        profile,
                        path,
                        hover,
                        ..
                    } => {
                        let h = hover.map(|h| {
                            let mut p = path.clone();
                            p.push(h);
                            p
                        });
                        (Some(*profile), h)
                    }
                    _ => (None, None),
                }
            };
            let in_pie = pie_profile.is_some();

            // Outside the pie, find out which app this press is about. Inside the pie it is the app
            // the pie was opened for.
            let target = if in_pie {
                side::target()
            } else {
                let bundle = target_bundle_at(p);
                remember_target(bundle.clone());
                bundle
            };

            let action = {
                let c = cfg();
                let pi = pie_profile.unwrap_or_else(|| side::owner(&c, target.as_deref()));
                let item = c.profiles.get(pi).and_then(|pr| match &hovered {
                    Some(p) => item_at(&pr.items, p),
                    None => pr.active_item(),
                });
                match item.map(|i| &i.kind) {
                    Some(ItemKind::Mode { back, forward }) => {
                        if idx == 0 {
                            back.clone()
                        } else {
                            forward.clone()
                        }
                    }
                    _ => Action::Nothing,
                }
            };

            if matches!(action, Action::Nothing) && !in_pie {
                SIDE_SWALLOWED[idx].store(false, Ordering::SeqCst);
                return CallbackResult::Keep; // native back/forward keeps working
            }
            SIDE_SWALLOWED[idx].store(true, Ordering::SeqCst);
            run_action(action, target);
            CallbackResult::Drop
        }
        CGEventType::OtherMouseUp => {
            if SIDE_SWALLOWED[idx].swap(false, Ordering::SeqCst) {
                CallbackResult::Drop
            } else {
                CallbackResult::Keep
            }
        }
        CGEventType::OtherMouseDragged => {
            if SIDE_SWALLOWED[idx].load(Ordering::SeqCst) {
                CallbackResult::Drop
            } else {
                CallbackResult::Keep
            }
        }
        _ => CallbackResult::Keep,
    }
}

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
        return handle_key(notifier, etype, event);
    }

    if matches!(
        etype,
        CGEventType::RightMouseDown | CGEventType::RightMouseUp | CGEventType::RightMouseDragged
    ) {
        return handle_right(state, notifier, etype);
    }

    let button = event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
    if button == SIDE_BACK || button == SIDE_FORWARD {
        return handle_side(state, etype, button, event.location());
    }
    let (trigger, flick) = {
        let c = cfg();
        (c.trigger_button, c.flick)
    };
    if button != trigger {
        return CallbackResult::Keep;
    }

    let p = event.location();
    let mut g = lock(state);

    match (etype, &mut *g) {
        (CGEventType::OtherMouseDown, _) => {
            let (hold_profile, app) = resolve_profiles(p);
            let dbl_ms = cfg().double_click_ms;

            // (A) tekan kedua dalam jendela waktu -> pie khusus app, langsung terbuka
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
                    return CallbackResult::Drop; // tidak ada timer "buka Settings" untuk pie app
                }
                late = Some(first); // terlambat: klik pertama tetap harus sampai ke app
            }
            if let Some(first) = late {
                drop(g);
                post_mouse(first, CGEventType::OtherMouseDown, button);
                post_mouse(first, CGEventType::OtherMouseUp, button);
                g = lock(state);
            }

            // (B) tekan biasa: kode lama, hanya `Pending` dan `profile` yang berubah
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            *g = Gesture::Pending {
                id,
                pos: p,
                profile: hold_profile,
                app,
            };
            drop(g);
            let st = state.clone();
            let n = notifier.clone();
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
            CallbackResult::Drop
        }
        (
            CGEventType::OtherMouseDragged,
            Gesture::Pending {
                id, pos, profile, ..
            },
        ) => {
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
        (
            CGEventType::OtherMouseDragged,
            Gesture::ActivePie {
                center,
                profile,
                path,
                hover,
                ..
            },
        ) => {
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
        (CGEventType::OtherMouseUp, Gesture::Pending { id, pos, app, .. }) => {
            let (id, start, app) = (*id, *pos, *app);
            match app {
                Some(app) => {
                    // app ini punya profil sendiri: tunggu kemungkinan klik kedua
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
        (
            CGEventType::OtherMouseUp,
            Gesture::ActivePie {
                center,
                profile,
                path,
                ..
            },
        ) => {
            let (center, profile, path) = (*center, *profile, path.clone());
            *g = Gesture::Idle;
            drop(g);
            if let Some(h) = eval(profile, &path, center, p).hover {
                select(profile, &path, h);
            }
            notifier.send(Msg::HidePie);
            CallbackResult::Drop
        }
        (CGEventType::OtherMouseUp, Gesture::InSettings) => {
            *g = Gesture::Idle;
            CallbackResult::Drop
        }
        (CGEventType::OtherMouseUp, Gesture::Passthrough) => {
            *g = Gesture::Idle;
            CallbackResult::Keep
        }
        (CGEventType::OtherMouseDragged, Gesture::Passthrough) => CallbackResult::Keep,
        (CGEventType::OtherMouseDragged, Gesture::InSettings) => CallbackResult::Drop,
        _ => CallbackResult::Keep,
    }
}

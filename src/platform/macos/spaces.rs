//! Apps on other Spaces: "Open app" for an app whose window is not on the Space that is shown.
//!
//! What the public API gives: `CGWindowListCopyWindowInfo` lists the windows of every Space and
//! says which are on screen, so CursorPie can tell that the app has a window, and that it is
//! somewhere else. What it does not give is a way to go there: the Accessibility API only lists the
//! windows of the current Space, and nothing switches to a given Space. Two things are done:
//!
//! * **Switch to the app's Space.** Four ways, each tried only when the one before did not bring a
//!   window of the app onto the screen:
//!   1. `open -a`, which is what a Dock click does: with the system setting *"When switching to an
//!      application, switch to a Space with open windows for the application"* (on by default)
//!      macOS goes there by itself.
//!   2. The Accessibility action `AXRaise` on the app's main window. The app element can name a
//!      window of another Space even though the window list of the Accessibility API cannot, and
//!      raising a window shows the Space it is on.
//!   3. What the user would do: Control+Right or Control+Left, one Space at a time, until a window
//!      of the app is on screen ([`walk_to_target`]). The
//!      window server tells the order of the Spaces and the Spaces of the app's windows, so the
//!      nearest one is gone to directly, to the left or right and however many Spaces away; without
//!      that the walk feels its way, right first and then left. That needs the shortcuts to be on
//!      (System Settings > Keyboard > Keyboard Shortcuts > Mission Control).
//!   4. The same walk with the keys sent through System Events (`osascript`), for a macOS that
//!      ignores keys posted by other programs. It asks for the Automation permission once.
//!
//!   Once the Space is shown the window is raised itself ([`focus_app`]): after the switch animation
//!   it is raised through Accessibility, the app is made frontmost, and the front window is checked.
//!
//!   `CURSORPIE_DEBUG=1` in the environment prints what each step saw to stderr.
//! * **Bring the window here** (experimental). Only private SkyLight calls can move a window of
//!   another app to a Space, the ones Hammerspoon and yabai use since macOS 14.5. They are looked up
//!   with `dlsym`, so a macOS that lacks them just reports "not available", and a macOS that has them
//!   but ignores them is caught by asking where the window is afterwards. Either way the answer is
//!   `false` and the caller switches to the app's Space instead.
//!
//! Everything is best effort and ends in `open -a`, the behavior from before this setting.

use super::shell::open_app;
use crate::config::DesktopMode;
use crate::platform::desktops::{
    Desktops, MAX_STEPS, Presence, WindowInfo, app_name_of_path, presence_of, same_app_path,
    signature, steps_between, walk_to_target,
};
use crate::platform::spawn_reaped;
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::geometry::CGRect;
use core_graphics::window::{
    copy_window_info, kCGNullWindowID, kCGWindowAlpha, kCGWindowBounds, kCGWindowIsOnscreen,
    kCGWindowLayer, kCGWindowListExcludeDesktopElements, kCGWindowListOptionAll,
    kCGWindowListOptionOnScreenOnly, kCGWindowNumber, kCGWindowOwnerPID,
};
use std::ffi::{c_char, c_int, c_void};
use std::hash::{Hash, Hasher};
use std::process::Command;
use std::time::{Duration, Instant};

/// How long `open -a` gets to bring a window of the app onto the screen by itself before the
/// Spaces are walked: the switch animation takes about half a second.
const OPEN_WAIT: Duration = Duration::from_millis(1200);
/// How long to wait for one Control+Arrow to take effect before taking it for "did not move": the
/// last Space answers with nothing at all.
const SETTLE: Duration = Duration::from_millis(1000);
/// How long the switch animation of a Space is given to finish before the window is raised: during
/// it the new Space is already reported as shown, but the app does not take focus yet.
const ARRIVE: Duration = Duration::from_millis(700);
/// How long a window gets to come to the front after it was raised.
const FRONT_WAIT: Duration = Duration::from_millis(500);
/// How long the animation of a Space switch is given before the next arrow key is pressed.
const STEP_PAUSE: Duration = Duration::from_millis(350);
/// How often the screen is looked at while waiting.
const POLL: Duration = Duration::from_millis(50);

/// Prints what the Space code saw when `CURSORPIE_DEBUG` is set, to find out why an app was not
/// reached. `message` is only built then.
fn trace(message: impl FnOnce() -> String) {
    if std::env::var_os("CURSORPIE_DEBUG").is_some() {
        eprintln!("CursorPie[spaces]: {}", message());
    }
}

/// The running app `app` names (bundle id, localized name or a `.app` path) that is not hidden:
/// its process id. `None` when it does not run, or is hidden, in which case `open -a` starts or
/// unhides it, which is all there is to do.
///
/// A path is compared as the system reports it (no trailing slash, any letter case), and by the
/// name in it, so `/Applications/Helium.app/` finds Helium even when the system knows the app under
/// another path (a symbolic link, a copy of the folder).
fn visible_running_pid(app: &str) -> Option<i32> {
    use objc2::rc::autoreleasepool;
    use objc2_app_kit::NSWorkspace;

    let target = app.trim();
    let by_path = target.contains('/');
    let name = if by_path {
        app_name_of_path(target)
    } else {
        target
    };
    autoreleasepool(|_| {
        // NSRunningApplication documents its properties as thread-safe.
        for a in NSWorkspace::sharedWorkspace().runningApplications().iter() {
            let named = a
                .localizedName()
                .is_some_and(|s| !name.is_empty() && s.to_string().eq_ignore_ascii_case(name));
            let hit = if by_path {
                a.bundleURL()
                    .and_then(|u| u.path())
                    .is_some_and(|p| same_app_path(&p.to_string(), target))
                    || named
            } else {
                a.bundleIdentifier()
                    .is_some_and(|s| s.to_string().eq_ignore_ascii_case(target))
                    || named
            };
            if hit {
                let pid = a.processIdentifier();
                if a.isHidden() {
                    trace(|| format!("{app:?} runs as pid {pid} but is hidden"));
                    return None;
                }
                return Some(pid);
            }
        }
        trace(|| {
            format!(
                "no running app matches {app:?} (by {})",
                if by_path { "path" } else { "name or bundle id" }
            )
        });
        None
    })
}

/// The windows of every app, front to back, as [`WindowInfo`]. With `on_screen_only` just the ones
/// on the Spaces that are shown, otherwise those of all Spaces (and minimized ones).
fn window_list(on_screen_only: bool) -> Vec<WindowInfo> {
    let scope = if on_screen_only {
        kCGWindowListOptionOnScreenOnly
    } else {
        kCGWindowListOptionAll
    };
    let Some(windows) =
        copy_window_info(scope | kCGWindowListExcludeDesktopElements, kCGNullWindowID)
    else {
        return Vec::new();
    };
    let (k_number, k_layer, k_alpha, k_pid, k_bounds, k_on_screen) = unsafe {
        (
            CFString::wrap_under_get_rule(kCGWindowNumber),
            CFString::wrap_under_get_rule(kCGWindowLayer),
            CFString::wrap_under_get_rule(kCGWindowAlpha),
            CFString::wrap_under_get_rule(kCGWindowOwnerPID),
            CFString::wrap_under_get_rule(kCGWindowBounds),
            CFString::wrap_under_get_rule(kCGWindowIsOnscreen),
        )
    };
    let mut out = Vec::new();
    for w in windows.iter() {
        let d: CFDictionary<CFString, CFType> =
            unsafe { CFDictionary::wrap_under_get_rule(*w as CFDictionaryRef) };
        let number = |k: &CFString| d.find(k).and_then(|v| v.downcast::<CFNumber>());
        let (Some(num), Some(layer), Some(pid)) = (
            number(&k_number).and_then(|n| n.to_i64()),
            number(&k_layer).and_then(|n| n.to_i64()),
            number(&k_pid).and_then(|n| n.to_i64()),
        ) else {
            continue;
        };
        let Some(bounds) = d
            .find(&k_bounds)
            .and_then(|v| v.downcast::<CFDictionary>())
            .and_then(|b| CGRect::from_dict_representation(&b))
        else {
            continue;
        };
        out.push(WindowInfo {
            number: num as u32,
            pid: pid as i32,
            layer,
            alpha: number(&k_alpha).and_then(|n| n.to_f64()).unwrap_or(1.0),
            width: bounds.size.width,
            height: bounds.size.height,
            // the key is only present for windows that are on screen
            on_screen: on_screen_only
                || d.find(&k_on_screen)
                    .and_then(|v| v.downcast::<CFBoolean>())
                    .is_some_and(bool::from),
        });
    }
    out
}

/// The row of Spaces as [`walk_to_target`] needs it, looking for the windows of the app `pid`.
struct Row {
    pid: i32,
    /// The ordinary windows of the app, on any Space: the Spaces they are on are the ones to go to.
    windows: Vec<u32>,
    /// The private calls, when this macOS has them: they tell the order of the Spaces, so the way
    /// there is known, and tell an empty Space from another.
    sky: Option<SkyLight>,
    /// Send the arrow keys through System Events instead of posting them directly.
    via_script: bool,
}

impl Row {
    fn new(pid: i32, windows: Vec<u32>, via_script: bool) -> Self {
        Row {
            pid,
            windows,
            sky: SkyLight::load(),
            via_script,
        }
    }

    /// Whether a window of the app is on a Space that is shown.
    fn app_is_here(&self) -> bool {
        presence_of(&window_list(true), self.pid) == Presence::Here
    }
}

impl Desktops for Row {
    type Id = u64;

    fn target_here(&mut self) -> bool {
        self.app_is_here()
    }

    /// The Space that is shown: its id from the window server, together with the windows on it, so
    /// that a change of either counts as a move, and an empty Space can still be told. CursorPie's
    /// own windows are left out of the windows: the pie may still be fading out.
    fn current(&mut self) -> Option<u64> {
        let me = std::process::id() as i32;
        let windows = signature(
            window_list(true)
                .iter()
                .filter(|w| w.pid != me && w.is_ordinary())
                .map(|w| w.number)
                .collect(),
        );
        let space = self.sky.as_ref().map_or(0, SkyLight::shown_space);
        if space == 0 {
            return windows;
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (space, windows).hash(&mut hasher);
        Some(hasher.finish())
    }

    /// Steps to the nearest Space with a window of the app, from the order the window server
    /// reports the Spaces in.
    fn route(&mut self) -> Option<isize> {
        let sky = self.sky.as_ref()?;
        let here = sky.shown_space();
        let order = sky.space_order(here);
        let targets: Vec<u64> = self
            .windows
            .iter()
            .flat_map(|w| sky.spaces_of(*w))
            .collect();
        let steps = steps_between(&order, &here, &targets);
        trace(|| {
            format!(
                "Spaces in order {order:?}, shown {here}, windows of the app on {targets:?}: {steps:?} steps"
            )
        });
        steps
    }

    fn step(&mut self, forward: bool) {
        let before = self.current();
        if self.via_script {
            press_space_arrow_with_script(forward);
        } else {
            super::press_space_arrow(forward);
        }
        // macOS animates the switch: wait until the answer changes, or until it is clear that
        // nothing happens (the last Space). The app being on the screen is not the end of it: during
        // the animation the windows of two Spaces are shown.
        let start = Instant::now();
        let mut moved = false;
        while start.elapsed() < SETTLE {
            std::thread::sleep(POLL);
            if self.current() != before {
                moved = true;
                break;
            }
        }
        if moved {
            // a key pressed during the animation can be lost, so the next step waits for it
            std::thread::sleep(STEP_PAUSE);
        }
        let after = self.current();
        trace(|| {
            format!(
                "step {} (script: {}): Space {} after {} ms, app here: {}",
                if forward { "right" } else { "left" },
                self.via_script,
                if after == before {
                    "did not change"
                } else {
                    "changed"
                },
                start.elapsed().as_millis(),
                self.app_is_here()
            )
        });
    }
}

/// Control+Left or Control+Right sent through System Events, which some macOS versions honor when
/// they ignore the same keys posted with `CGEvent`. Key codes 123 and 124 are the arrows. The call
/// is not waited for: the caller looks at the screen.
fn press_space_arrow_with_script(forward: bool) {
    let code = if forward { 124 } else { 123 };
    spawn_reaped(Command::new("osascript").arg("-e").arg(format!(
        "tell application \"System Events\" to key code {code} using control down"
    )));
}

/// Waits (at most `OPEN_WAIT`) until a window of the app `pid` is on a Space that is shown.
fn wait_until_here(pid: i32) -> bool {
    let row = Row::new(pid, Vec::new(), false);
    let start = Instant::now();
    while start.elapsed() < OPEN_WAIT {
        if row.app_is_here() {
            return true;
        }
        std::thread::sleep(POLL);
    }
    row.app_is_here()
}

/// Whether the frontmost ordinary window (CursorPie's own left out) belongs to the app `pid`.
fn is_frontmost(pid: i32) -> bool {
    let me = std::process::id() as i32;
    window_list(true)
        .iter()
        .find(|w| w.pid != me && w.is_ordinary())
        .is_some_and(|w| w.pid == pid)
}

/// Waits (at most `FRONT_WAIT`) until the app `pid` has the frontmost window.
fn wait_until_frontmost(pid: i32) -> bool {
    let start = Instant::now();
    while start.elapsed() < FRONT_WAIT {
        if is_frontmost(pid) {
            return true;
        }
        std::thread::sleep(POLL);
    }
    is_frontmost(pid)
}

/// Brings the window of the app `pid` to the front once its Space is shown. `open -a` alone does
/// not do it: it activates the app, but another app's window can stay in front. So the window is
/// raised itself (it is visible to Accessibility now that its Space is the current one), the app is
/// made frontmost, and the result is checked, up to three times. `wait` lets a Space animation
/// finish first.
fn focus_app(pid: i32, app: &str, wait: bool) -> bool {
    if wait {
        std::thread::sleep(ARRIVE);
    }
    for attempt in 1..=3 {
        let raised = raise_main_window(pid);
        activate(pid);
        open_app(app);
        let front = wait_until_frontmost(pid);
        trace(|| format!("focus attempt {attempt}: raised {raised}, frontmost {front}"));
        if front {
            return true;
        }
    }
    false
}

/// The "Open app" action with Space awareness (see [`DesktopMode`]): `app` is a bundle id, an app
/// name or a `.app` path, as typed in a slice. Apps that do not run, are hidden, have no window or
/// have one on the current Space are opened exactly as before.
pub fn reveal_app(app: &str, mode: DesktopMode) {
    trace(|| format!("open {app:?}, mode {mode:?}"));
    let Some(pid) = visible_running_pid(app) else {
        trace(|| "not running or hidden: plain open -a".into());
        open_app(app);
        return;
    };
    let all = window_list(false);
    let presence = presence_of(&all, pid);
    trace(|| {
        let own: Vec<_> = all.iter().filter(|w| w.pid == pid).collect();
        format!(
            "pid {pid}: {} windows in the list of all Spaces, {} of the app, {} ordinary, {} ordinary on screen; {presence:?}",
            all.len(),
            own.len(),
            own.iter().filter(|w| w.is_ordinary()).count(),
            own.iter()
                .filter(|w| w.is_ordinary() && w.on_screen)
                .count(),
        )
    });
    let windows: Vec<u32> = all
        .iter()
        .filter(|w| w.pid == pid && w.is_ordinary())
        .map(|w| w.number)
        .collect();
    let Presence::Elsewhere(window) = presence else {
        open_app(app);
        return;
    };
    if mode == DesktopMode::BringToCurrent {
        if bring_window_here(window) {
            focus_app(pid, app, false);
            return;
        }
        eprintln!(
            "CursorPie: could not move a window of {app} to this Space (private system calls are not \
             available or refused); switching to its Space instead"
        );
    }
    // 1. `open -a`: with the system setting on, this is all it takes
    open_app(app);
    if wait_until_here(pid) {
        trace(|| "open -a brought a window here".into());
        if !wait_until_frontmost(pid) {
            focus_app(pid, app, true);
        }
        return;
    }
    // 2. raise the main window through Accessibility
    if raise_main_window(pid) && wait_until_here(pid) {
        trace(|| "AXRaise brought a window here".into());
        focus_app(pid, app, true);
        return;
    }
    // 3. and 4. walk the Spaces with the arrow keys, posted and then through System Events
    for via_script in [false, true] {
        trace(|| format!("walking the Spaces (script: {via_script})"));
        if walk_to_target(&mut Row::new(pid, windows.clone(), via_script), MAX_STEPS) {
            focus_app(pid, app, true);
            return;
        }
    }
    eprintln!(
        "CursorPie: could not reach the Space of {app}; start CursorPie with CURSORPIE_DEBUG=1 to see why"
    );
}

// ----------------------------------------------------------------------------- Accessibility

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateApplication(pid: c_int) -> CFTypeRef;
    fn AXUIElementCopyAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> c_int;
    fn AXUIElementPerformAction(element: CFTypeRef, action: CFStringRef) -> c_int;
    fn AXUIElementSetAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> c_int;
}

/// Raises the main (else the focused) window of the app `pid` with the Accessibility action
/// `AXRaise`, which shows the Space the window is on. `true` when the action was accepted. Needs
/// the Accessibility permission, which CursorPie has to have anyway.
fn raise_main_window(pid: i32) -> bool {
    // SAFETY: every element that is created or copied is wrapped at once, so it is released on every
    // path; the attribute and action names are CFStrings that outlive the calls.
    unsafe {
        let app_ref = AXUIElementCreateApplication(pid);
        if app_ref.is_null() {
            return false;
        }
        let app = CFType::wrap_under_create_rule(app_ref);
        // make the app frontmost, so that the raised window gets the keyboard focus too
        let frontmost = CFString::from_static_string("AXFrontmost");
        let error = AXUIElementSetAttributeValue(
            app.as_CFTypeRef(),
            frontmost.as_concrete_TypeRef(),
            CFBoolean::true_value().as_CFTypeRef(),
        );
        trace(|| format!("AXFrontmost of pid {pid}: AX error {error}"));
        // the main and the focused window, else the windows of the app on the Space that is shown,
        // front to back: an app that is not active has no main or focused window
        let mut candidates: Vec<(&str, CFType)> = Vec::new();
        for attribute in ["AXMainWindow", "AXFocusedWindow"] {
            let value = copy_attribute(&app, attribute);
            trace(|| format!("{attribute} of pid {pid}: found {}", value.is_some()));
            candidates.extend(value.map(|w| (attribute, w)));
        }
        if let Some(list) = copy_attribute(&app, "AXWindows").and_then(|v| v.downcast::<CFArray>())
        {
            trace(|| format!("AXWindows of pid {pid}: {} windows", list.len()));
            for item in list.iter() {
                candidates.push(("AXWindows item", CFType::wrap_under_get_rule(*item)));
            }
        }
        let action = CFString::from_static_string("AXRaise");
        for (what, window) in candidates {
            let error =
                AXUIElementPerformAction(window.as_CFTypeRef(), action.as_concrete_TypeRef());
            trace(|| format!("AXRaise of the {what}: AX error {error}"));
            if error == 0 {
                for attribute in ["AXMain", "AXFocused"] {
                    let name = CFString::from_static_string(attribute);
                    AXUIElementSetAttributeValue(
                        window.as_CFTypeRef(),
                        name.as_concrete_TypeRef(),
                        CFBoolean::true_value().as_CFTypeRef(),
                    );
                }
                return true;
            }
        }
        false
    }
}

/// The value of the Accessibility attribute `attribute` of `element`, `None` when it has none.
///
/// # Safety
/// `element` must be a live accessibility element.
unsafe fn copy_attribute(element: &CFType, attribute: &str) -> Option<CFType> {
    let name = CFString::new(attribute);
    let mut value: CFTypeRef = std::ptr::null();
    // SAFETY: the caller guarantees the element; the copied value is wrapped, so it is released.
    unsafe {
        let error = AXUIElementCopyAttributeValue(
            element.as_CFTypeRef(),
            name.as_concrete_TypeRef(),
            &mut value,
        );
        (error == 0 && !value.is_null()).then(|| CFType::wrap_under_create_rule(value))
    }
}

/// Asks the system to activate the app `pid` with all its windows.
fn activate(pid: i32) {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
    if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        let sent = app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
        trace(|| format!("activate pid {pid}: {sent}"));
    }
}

// ----------------------------------------------------------------------------- SkyLight

// libSystem, which every process links, has both
unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

/// `RTLD_LAZY`.
const RTLD_LAZY: c_int = 1;
/// The private framework that holds the window server client calls.
const SKYLIGHT: &std::ffi::CStr = c"/System/Library/PrivateFrameworks/SkyLight.framework/SkyLight";
/// A made-up workspace id ("yabe"): while a Space has it, `SLSSetWindowListWorkspace` puts windows
/// on that Space. This is how windows are moved since macOS 14.5.
const COMPAT_ID: c_int = 0x7961_6265;
/// Selector of `SLSCopySpacesForWindows`: the Spaces of every kind.
const ALL_SPACES: c_int = 0x7;

type MainConnection = unsafe extern "C" fn() -> c_int;
type ActiveSpace = unsafe extern "C" fn(c_int) -> u64;
type SpacesForWindows = unsafe extern "C" fn(c_int, c_int, CFArrayRef) -> CFArrayRef;
type ManagedDisplaySpaces = unsafe extern "C" fn(c_int) -> CFArrayRef;
type SetCompatId = unsafe extern "C" fn(c_int, u64, c_int) -> c_int;
type SetWindowListWorkspace = unsafe extern "C" fn(c_int, *const u32, c_int, c_int) -> c_int;

/// The private calls, looked up when needed. A field is a function the running macOS has.
struct SkyLight {
    main_connection: MainConnection,
    active_space: ActiveSpace,
    spaces_for_windows: SpacesForWindows,
    managed_display_spaces: ManagedDisplaySpaces,
    set_compat_id: SetCompatId,
    set_window_list_workspace: SetWindowListWorkspace,
}

impl SkyLight {
    /// `None` when SkyLight or any of the calls is missing.
    fn load() -> Option<Self> {
        // SAFETY: `dlopen` and `dlsym` are called with zero terminated names. A symbol is only
        // turned into a function pointer when it is not null, and with the signature that the
        // call has in `yabai` and `Hammerspoon`, which use it the same way.
        unsafe {
            let handle = dlopen(SKYLIGHT.as_ptr(), RTLD_LAZY);
            if handle.is_null() {
                return None;
            }
            let sym = |name: &std::ffi::CStr| {
                let p = dlsym(handle, name.as_ptr());
                (!p.is_null()).then_some(p)
            };
            Some(SkyLight {
                main_connection: std::mem::transmute::<*mut c_void, MainConnection>(sym(
                    c"SLSMainConnectionID",
                )?),
                active_space: std::mem::transmute::<*mut c_void, ActiveSpace>(sym(
                    c"SLSGetActiveSpace",
                )?),
                spaces_for_windows: std::mem::transmute::<*mut c_void, SpacesForWindows>(sym(
                    c"SLSCopySpacesForWindows",
                )?),
                managed_display_spaces: std::mem::transmute::<*mut c_void, ManagedDisplaySpaces>(
                    sym(c"SLSCopyManagedDisplaySpaces")?,
                ),
                set_compat_id: std::mem::transmute::<*mut c_void, SetCompatId>(sym(
                    c"SLSSpaceSetCompatID",
                )?),
                set_window_list_workspace: std::mem::transmute::<*mut c_void, SetWindowListWorkspace>(
                    sym(c"SLSSetWindowListWorkspace")?,
                ),
            })
        }
    }

    /// The id of the Space that is shown, 0 when the call has nothing to say.
    fn shown_space(&self) -> u64 {
        // SAFETY: both are plain calls with the connection of this process.
        unsafe { (self.active_space)((self.main_connection)()) }
    }

    /// The ids of the Spaces the window `number` is on. Empty for a minimized window, or when the
    /// call has nothing to say.
    fn spaces_of(&self, number: u32) -> Vec<u64> {
        let ids = CFArray::from_CFTypes(&[CFNumber::from(i64::from(number))]);
        // SAFETY: `ids` is a live CFArray of CFNumber; the result, when not null, is a new array
        // that is released by the wrapper.
        unsafe {
            let spaces = (self.spaces_for_windows)(
                (self.main_connection)(),
                ALL_SPACES,
                ids.as_concrete_TypeRef(),
            );
            if spaces.is_null() {
                return Vec::new();
            }
            CFArray::<CFNumber>::wrap_under_create_rule(spaces)
                .iter()
                .filter_map(|n| n.to_i64())
                .map(|n| n as u64)
                .collect()
        }
    }

    /// The ids of the Spaces of the display that has the Space `here`, from left to right: the
    /// order Control+Arrow goes through. Empty when `here` is on no display of the list.
    fn space_order(&self, here: u64) -> Vec<u64> {
        let key_spaces = CFString::from_static_string("Spaces");
        let key_id = CFString::from_static_string("ManagedSpaceID");
        // SAFETY: the result, when not null, is a new array that is released by the wrapper; its
        // items are dictionaries, which are only read through the typed wrappers.
        let displays = unsafe {
            let list = (self.managed_display_spaces)((self.main_connection)());
            if list.is_null() {
                return Vec::new();
            }
            CFArray::<*const c_void>::wrap_under_create_rule(list)
        };
        for display in displays.iter() {
            let display: CFDictionary<CFString, CFType> =
                unsafe { CFDictionary::wrap_under_get_rule(*display as CFDictionaryRef) };
            let Some(spaces) = display
                .find(&key_spaces)
                .and_then(|v| v.downcast::<CFArray>())
            else {
                continue;
            };
            let ids: Vec<u64> = spaces
                .iter()
                .filter_map(|space| {
                    let space: CFDictionary<CFString, CFType> =
                        unsafe { CFDictionary::wrap_under_get_rule(*space as CFDictionaryRef) };
                    space
                        .find(&key_id)
                        .and_then(|v| v.downcast::<CFNumber>())
                        .and_then(|n| n.to_i64())
                        .map(|n| n as u64)
                })
                .collect();
            if ids.contains(&here) {
                return ids;
            }
        }
        Vec::new()
    }
}

/// Moves the window `number` onto the Space that is shown. `true` only when the window server then
/// reports the window on that Space: the calls themselves return success on macOS versions where
/// they do nothing.
fn bring_window_here(number: u32) -> bool {
    let Some(sl) = SkyLight::load() else {
        return false;
    };
    // SAFETY: the function pointers come from `SkyLight::load`; `number` outlives the call that
    // reads it through the pointer.
    unsafe {
        let connection = (sl.main_connection)();
        let here = sl.shown_space();
        if here == 0 {
            return false;
        }
        (sl.set_compat_id)(connection, here, COMPAT_ID);
        (sl.set_window_list_workspace)(connection, &number, 1, COMPAT_ID);
        (sl.set_compat_id)(connection, here, 0);
        sl.spaces_of(number).contains(&here)
    }
}

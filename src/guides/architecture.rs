//! # Architecture
//!
//! CursorPie is a small program with a strict split between *observing input*, *deciding what it
//! means*, *doing something about it* and *showing it*. This guide explains how those parts are
//! wired together and which rules keep it responsive and crash-free.
//!
//! * [Threads](#threads)
//! * [One window for everything](#one-window-for-everything)
//! * [Data flow](#data-flow)
//! * [The gesture state machine](#the-gesture-state-machine)
//! * [Shared state and lock order](#shared-state-and-lock-order)
//! * [Side-button routing](#side-button-routing)
//! * [Rules for the event-tap callback](#rules-for-the-event-tap-callback)
//! * [macOS APIs and external tools](#macos-apis-and-external-tools)
//!
//! ## Threads
//!
//! | Thread | Started by | Owns | Talks to the others through |
//! |---|---|---|---|
//! | **main / UI** | `eframe::run_native` in [`crate::run`] | the [`App`](crate::ui::app::App), every egui window, the menu bar icon, the keyboard-layout snapshot | receives [`Msg`](crate::msg::Msg) values |
//! | **`cursorpie-tap`** | [`tap::spawn`](crate::tap::spawn) | the `CGEventTap`, its `CFRunLoop`, the gesture state | sends `Msg` through a [`Notifier`](crate::msg::Notifier); enqueues actions |
//! | **`cursorpie-actions`** | [`actions::init`](crate::actions::init) | the queue of [`Action`](crate::config::Action)s to execute | receives jobs from [`actions::run_in`](crate::actions::run_in) |
//! | short-lived | on demand | hold timer, double-click timer, file chooser, config save | send `Msg` or touch shared state |
//!
//! Why separate threads?
//!
//! * The tap callback is on the input path of the whole system. If it is slow macOS disables the
//!   tap, so it only *decides* and never *does* anything slow.
//! * Actions (sending keys, spawning `open`, sleeping 40 ms between sequence steps) must run in
//!   the order the user triggered them, so they all go through one worker.
//! * egui must be driven from the main thread.
//!
//! ## One window for everything
//!
//! There is exactly **one** native window. In *pie mode* it is transparent, borderless,
//! click-through, always on top, `pie_size` pixels square and moved so its center is under the
//! cursor. In *Settings mode* the same window is resized to the card size, made interactive,
//! lowered to the normal window level and centered on the display that contains the cursor.
//! [`ui::app`](crate::ui::app) switches between the two (`show_pie`, `open_settings`,
//! `close_settings`). Because the window is click-through, the pie never steals mouse events: the
//! event tap is what sees the mouse, and the UI only draws.
//!
//! ## Data flow
//!
//! ```text
//!  mouse / keyboard
//!        |
//!        v
//!  +--------------------+  Msg::ShowPie / Hover / HidePie / OpenSettings
//!  | cursorpie-tap      |------------------------------------------------+
//!  |  tap::handle       |                                                |
//!  |  gesture machine   |--- Action ---> +-------------------+           v
//!  +--------------------+                | cursorpie-actions |   +---------------+
//!        ^   reads                       |  CGEvent / open / |   | main thread   |
//!        |                               |  /bin/sh          |   |  ui::app::App |
//!   config::cfg()  <---- writes ---------+-------------------+   |  pie / Settings|
//!   side::STATE                                                   +---------------+
//!        ^                                                              |
//!        +------------------- Save in Settings ------------------------+
//! ```
//!
//! 1. The tap thread classifies each event ([`tap`](crate::tap)): trigger button, side button,
//!    right button or key.
//! 2. When something visible happens it sends a [`Msg`](crate::msg::Msg). `Notifier::send` also
//!    calls `request_repaint`, which wakes the UI even while its window is hidden.
//! 3. [`App::logic`](crate::ui::app::App) drains the channel every frame and updates UI state;
//!    drawing happens in `App::ui`.
//! 4. When the user releases over a slice the **tap thread** (not the UI) picks the slice and
//!    queues its action. The UI is told to hide the pie but is not involved in running anything.
//!
//! ## The gesture state machine
//!
//! The trigger button (middle button by default) drives a small state machine,
//! `tap::gesture::Gesture`, held behind a mutex that the tap callback and the timer threads share.
//! Each timer carries the `id` of the press that created it and does nothing if the state moved on.
//!
//! ```text
//!                    press
//!        Idle ------------------> Pending --------------- hold pie_hold_ms ---------+
//!         ^                          |  |                                           |
//!         |                          |  +-- drag > 6 px (14 px with "flick") -------+--> ActivePie
//!         |   quick release,         |        (not flick: Passthrough, press and     |   (flick: opens
//!         |   no app profile:        |         drag are handed back to the app)      |    the pie now)
//!         |   click is replayed      |                                               |
//!         +--------------------------+                                               v
//!         |                          | quick release, app has its own profile    +----------+
//!         |                          v                                           | ActivePie|
//!         |                     WaitSecond --- 2nd press within double_click_ms -->  (app pie)
//!         |                          |                                           +----------+
//!         |   timer expires: first   |                                                |
//!         +---- click is replayed ---+        hover / cross rim (submenu) ----------+ |
//!         |                                                                          | |
//!         +<------------------- release: run the slice, Msg::HidePie ----------------+ |
//!         +<-- release (InSettings / Passthrough) <-- hold settings_hold_ms in the center
//! ```
//!
//! Notes:
//!
//! * A quick click must still reach the app, so the press is *swallowed* and, if it turns out to
//!   be a click, **replayed** with [`actions::post_mouse`](crate::actions::post_mouse). Replayed
//!   events carry [`SYNTH_TAG`](crate::actions::SYNTH_TAG) so the tap ignores them.
//! * With a per-app profile and the default `DoubleClick` mode, a plain click in that app is
//!   delivered only after the double-click window has passed (see `double_click_ms`).
//! * Settings can only be opened by *holding* in the center of the pie that the hold opened. A pie
//!   opened by flicking or by a double-click has no settings timer. (The menu bar also opens it.)
//! * Crossing the outer rim over a submenu slice pushes the slice onto `path`, re-centers the pie
//!   on the cursor and sends `Msg::ShowPie` again (the UI animates it as a new ring).
//! * While [`PAUSED`](crate::tap::PAUSED) is set, or when macOS disables the tap, the state is reset
//!   to `Idle` and a `Msg::HidePie` is sent, so nothing is ever left half-finished.
//!
//! ## Shared state and lock order
//!
//! | State | Type | Readers / writers |
//! |---|---|---|
//! | the live config | [`config::cfg()`](crate::config::cfg) (`Mutex<Config>`) | everyone reads; Settings *Save* and `set_active` write |
//! | gesture state | `Arc<Mutex<Gesture>>` | tap callback and its timer threads |
//! | side-button choices | private `Mutex` in [`side`](crate::side) | tap, UI |
//! | flags | [`RECORDING`](crate::tap::RECORDING), [`SWALLOW_KEY`](crate::tap::SWALLOW_KEY), [`PAUSED`](crate::tap::PAUSED), [`TAP_ACTIVE`](crate::tap::TAP_ACTIVE) | atomics |
//!
//! **Lock order: gesture, then config, then side state.** Never take the gesture lock while
//! holding the config guard, and never lock the config while holding the `side` lock (functions in
//! `side` take `&Config` as a parameter for exactly this reason). Always lock with
//! [`config::lock`](crate::config::lock), which recovers from poisoning, so a panic in one thread
//! can never wedge the event tap.
//!
//! Settings edits a **copy** of the config (`App::edit`) and replaces the global one on *Save*,
//! after [`Config::sanitize`](crate::config::Config::sanitize).
//!
//! ## Side-button routing
//!
//! The back/forward buttons run the actions of the *active mode* slice. An app with its own
//! profile has two pies, so something has to decide which of them the buttons follow. That
//! decision (the *owner*) lives in [`side`](crate::side) and is documented there. In short: the
//! app's pie by default, or whichever pie the user last picked a mode in, or toggled with a
//! right-click or the menu bar item. The pie ring is tinted by [`side::view_of`](crate::side::view_of).
//!
//! ## Rules for the event-tap callback
//!
//! Everything reachable from `tap::handle` must follow these rules:
//!
//! 1. **Never block for long.** No sleeping, no waiting for a process, no file I/O. Spawn a
//!    thread instead (see the hold timer, `config::set_active`).
//! 2. **Never panic out of it.** `tap::guarded` wraps the handler in `catch_unwind` and
//!    returns `CallbackResult::Keep` (the event passes through) on a panic.
//! 3. **Fail open.** When unsure, `Keep` the event. Only swallow events that this program is
//!    sure it owns (the trigger button, the side buttons while they run an action, the right
//!    button while a pie is open, a key while recording a shortcut).
//! 4. **Release the gesture lock before sending messages or posting events.** The transition
//!    functions in `tap::trigger` take the `MutexGuard` by value so they can `drop` it first.
//! 5. **Ignore your own events.** Anything posted by [`actions`](crate::actions) is tagged with
//!    `SYNTH_TAG` and skipped at the top of `handle`.
//!
//! ## macOS APIs and external tools
//!
//! | Need | How | Where |
//! |---|---|---|
//! | global mouse/keyboard events | `CGEventTap` (HID, head insert, active) | [`tap`](crate::tap) |
//! | send keys / replay clicks | `CGEvent` posted to the HID tap location | [`actions`](crate::actions) |
//! | Accessibility permission | `AXIsProcessTrusted` | [`platform`](crate::platform) |
//! | frontmost app | `NSWorkspace` | `platform::frontmost_bundle_id` |
//! | app under the pointer | `CGWindowListCopyWindowInfo` + `NSRunningApplication` | `platform::bundle_id_at` |
//! | real app icon | `NSWorkspace.iconForFile` as TIFF, decoded by the `tiff` crate | [`ui::appicon`](crate::ui::appicon) |
//! | key labels on any layout | TIS + `UCKeyTranslate`, snapshot taken on the main thread | [`platform::layout`](crate::platform::layout) |
//! | menu bar icon | `tray-icon` / `muda` | [`ui::tray`](crate::ui::tray) |
//! | no Dock icon | `NSApplicationActivationPolicyAccessory` | `platform::set_accessory_policy` |
//! | launch at login | a LaunchAgent plist | `platform::set_launch_at_login` |
//! | open apps / URLs, file chooser, bundle id of an `.app`, hover tick | `open`, `osascript`, `mdls`, `afplay` | [`platform`](crate::platform) |
//!
//! The tools in the last row ship with macOS, so the app needs nothing installed.

//! # CursorPie
//!
//! A hold-to-open **pie menu for macOS**: hold a mouse button, flick toward a slice, release.
//! A slice can send a shortcut, launch an app, open a URL, run a shell command, open a submenu,
//! or remap the mouse's back/forward buttons. Every app can have its own pie ("profile").
//!
//! This is the documentation for **developers and contributors**. End-user documentation lives
//! in the repository's `README.md`.
//!
//! The package is a library (this crate, which holds everything) plus a tiny binary
//! (`src/main.rs`) that calls [`run`]. Keeping the code in a library lets it be documented here.
//!
//! ## Start here
//!
//! | Guide | Read it to learn |
//! |---|---|
//! | [`guides::architecture`] | threads, message flow, locking rules, the gesture state machine |
//! | [`guides::config_format`] | the `config.json` schema, versioning and validation |
//! | [`guides::extending`] | step-by-step recipes: add an action, a setting, a tab, an icon... |
//! | [`guides::contributing`] | environment, build, test, style and pull-request checklist |
//!
//! ## Module map
//!
//! | Module | Runs on | Responsibility |
//! |---|---|---|
//! | [`config`] | any thread | data model, defaults, validation, `config.json`, the live global copy |
//! | [`tap`] | `cursorpie-tap` thread | global event tap and the gesture state machine |
//! | [`actions`] | `cursorpie-actions` thread | executes actions (shortcuts, `open`, shell) in order |
//! | [`side`] | any thread | which pie the back/forward buttons follow |
//! | [`geometry`] | any thread | pie sizes and slice hit-testing (shared by tap and UI) |
//! | [`keys`] | any thread | shortcut model (key code + modifiers) and its labels |
//! | [`msg`] | any thread | messages sent *to* the UI thread |
//! | [`platform`] | any thread (some main only) | macOS glue: permissions, apps, login item, keyboard layout |
//! | [`ui`] | main thread | eframe app, pie renderer, Settings window, menu bar icon |
//! | [`about`] | - | name, version, links shown in the About tab |
//!
//! ## Platform
//!
//! macOS only (11 or later). Building for another target fails with a `compile_error!`.
//! To read these docs locally run `cargo docs` (an alias for
//! `cargo doc --no-deps --document-private-items --open`, see `.cargo/config.toml`).
//! When published, docs.rs builds them for the Apple targets listed in `Cargo.toml`.

#[cfg(not(target_os = "macos"))]
compile_error!("CursorPie only supports macOS.");

pub mod about;
pub mod actions;
pub mod config;
pub mod geometry;
pub mod guides;
pub mod keys;
pub mod msg;
pub mod platform;
pub mod side;
pub mod tap;
pub mod ui;

use eframe::egui;
use msg::Notifier;
use std::sync::mpsc;
use ui::app::App;

/// Starts CursorPie and blocks until the app quits.
///
/// Order matters here:
/// 1. the action worker thread is started ([`actions::init`]);
/// 2. the eframe window is created hidden, transparent and click-through;
/// 3. inside eframe's creator (which runs on the **main thread**, inside the event loop) the
///    Dock icon is hidden, the keyboard-layout snapshot is taken (TIS needs the main thread),
///    the event-tap thread is spawned and the menu bar icon is created.
pub fn run() -> eframe::Result<()> {
    actions::init();
    let (tx, rx) = mpsc::channel();
    let win = config::cfg().pie_size;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([win, win])
            .with_decorations(false)
            .with_transparent(true)
            .with_has_shadow(false)
            .with_resizable(false)
            .with_always_on_top()
            .with_mouse_passthrough(true)
            .with_active(false)
            .with_visible(false),
        ..Default::default()
    };

    eframe::run_native(
        about::APP_NAME,
        options,
        Box::new(move |cc| {
            // runs on the main thread inside the running event loop
            platform::set_accessory_policy(); // no Dock icon
            platform::layout::init(); // keyboard-layout snapshot (TIS must run on the main thread)
            let notifier = Notifier {
                tx: tx.clone(),
                ctx: cc.egui_ctx.clone(),
            };
            tap::spawn(notifier.clone());
            let tray = ui::tray::build(&notifier);
            Ok(Box::new(App::new(rx, tx, win, tray)))
        }),
    )
}

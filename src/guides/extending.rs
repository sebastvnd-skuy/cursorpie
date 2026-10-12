//! # Extending CursorPie: recipes
//!
//! Each recipe lists the files to touch, in the order that keeps the project compiling. File
//! paths are relative to `src/`. Read [`architecture`](super::architecture) first.
//!
//! * [Add an action kind](#add-an-action-kind)
//! * [Add a setting](#add-a-setting)
//! * [Add a ready-made slice](#add-a-ready-made-slice)
//! * [Add a slice kind](#add-a-slice-kind)
//! * [Add a Settings tab](#add-a-settings-tab)
//! * [Add icons to the picker](#add-icons-to-the-picker)
//! * [Add a message to the UI thread](#add-a-message-to-the-ui-thread)
//! * [Add a menu bar item](#add-a-menu-bar-item)
//! * [Change gesture behavior](#change-gesture-behavior)
//! * [Rebrand a fork](#rebrand-a-fork)
//!
//! ## Add an action kind
//!
//! Example: "Open a file in a given app".
//!
//! 1. `config/model.rs`: add a variant to [`Action`](crate::config::Action). Variants are
//!    serialized by name, so choose it carefully: it is part of the file format.
//! 2. `actions.rs`: handle it in `execute`. If it sends keystrokes, also return `true` for it in
//!    `sends_keys` so the target app is brought to the front first.
//! 3. `config/tree.rs`: if it can run arbitrary code, make
//!    [`has_commands`](crate::config::has_commands) return `true` for it (that drives the
//!    "this config runs shell commands" warning after an import).
//! 4. `ui/settings/action_editor.rs`: add it to `KINDS`, `action_index`, `make_action` and the
//!    input row. The `Sequence` kind shows how to add a kind the editor cannot edit yet.
//! 5. Add a unit test next to the code (see `actions::tests`).
//!
//! ## Add a setting
//!
//! 1. `config/model.rs`: add a field to [`Config`](crate::config::Config) with a doc comment.
//!    The struct is `#[serde(default)]`, so older files keep loading.
//! 2. `config/defaults.rs`: set its value in `impl Default for Config`.
//! 3. `config/sanitize.rs`: clamp it if it has a range, and add a case to `config/mod.rs` tests.
//! 4. `ui/settings/tabs/behavior.rs`: add the widget (it edits `self.edit`, the *copy* of the
//!    config, which *Save* applies).
//! 5. Use it where needed with `let v = cfg().field;`. **Copy the value out and drop the guard**
//!    instead of holding the lock across slow work.
//! 6. Document it in the field table of `README.md`.
//!
//! ## Add a ready-made slice
//!
//! `config/defaults.rs` has three lists shown in the "Add a slice" panel:
//! [`presets`](crate::config::presets) (side-button modes), [`quick_actions`](crate::config::quick_actions)
//! and [`blank_items`](crate::config::blank_items). Append an `Item::mode(..)` or `Item::run(..)`.
//! Use [`Shortcut::ch`](crate::keys::Shortcut::ch) for letters and punctuation so the shortcut
//! follows the user's keyboard layout, and the constants in [`keys::key`](crate::keys::key) for
//! layout-independent keys. The first eight presets are indexed by position in
//! `Config::default()`, so append new presets at the end.
//!
//! ## Add a slice kind
//!
//! Larger than it looks, because a slice kind touches every layer:
//!
//! 1. `config/model.rs`: new [`ItemKind`](crate::config::ItemKind) variant.
//! 2. `config/tree.rs` and `config/sanitize.rs`: make `has_commands`, `first_mode` and
//!    `clean_items` aware of it (the compiler points at the exhaustive matches).
//! 3. `tap/resolve.rs`: decide in `select` what releasing over it does.
//! 4. `ui/pie/view.rs` and `ui/pie/mod.rs`: how it is drawn; `ui/settings/pie_designer.rs`
//!    for the designer.
//! 5. `ui/settings/inspector.rs`: the kind chips and the editor body.
//!
//! ## Add a Settings tab
//!
//! 1. `ui/settings/state.rs`: add a variant to `Tab`.
//! 2. `ui/settings/tabs/<name>.rs`: `impl App { pub(in crate::ui::settings) fn <name>_tab(&mut self, ui: &mut egui::Ui) }`
//!    and register the file in `tabs/mod.rs`.
//! 3. `ui/settings/window.rs`: add it to the `tabs` array and to the `match` that draws it.
//!
//! Layout rule used throughout the Settings UI: compute widths from the width the caller gives
//! you instead of `available_width()` inside nested layouts, otherwise content can spill out of
//! the card.
//!
//! ## Add icons to the picker
//!
//! Append `ic("glyph", "Name", "search keywords")` to a category in
//! `ui/settings/icon_catalog.rs`. Glyphs are drawn with egui's bundled fonts; if one renders as
//! an empty box, remove it. Users can also drop images into
//! `~/Library/Application Support/CursorPie/icons/`.
//!
//! ## Add a message to the UI thread
//!
//! 1. `msg.rs`: add a variant to [`Msg`](crate::msg::Msg).
//! 2. `ui/app/messages.rs`: handle it in `handle_msg`.
//! 3. Send it with [`Notifier::send`](crate::msg::Notifier::send) (from the tap) or
//!    `tx.send(..)` followed by `ctx.request_repaint()` (from a plain thread).
//!
//! For work that blocks (file choosers, `mdls`) spawn a thread; `ui/settings/picker.rs` has the
//! helper for file choosers.
//!
//! ## Add a menu bar item
//!
//! `ui/tray.rs` builds the menu (`MenuItem::with_id("my_id", ..)`); `ui/app/windows.rs`
//! `handle_tray` receives the id as `Msg::Tray(id)`.
//!
//! ## Change gesture behavior
//!
//! Gestures live in `tap/`. Re-read the [state machine](super::architecture#the-gesture-state-machine)
//! and the [callback rules](super::architecture#rules-for-the-event-tap-callback) first.
//!
//! * A new state goes into `tap/gesture.rs`; each transition is one function in `tap/trigger.rs`,
//!   dispatched from `trigger::handle`.
//! * Anything that waits (a timer) is a thread that re-checks the gesture `id` after waking up.
//! * Return `CallbackResult::Drop` only for events the program owns; when in doubt use `Keep`.
//! * Pure decisions (which profile, which pie follows the side buttons) belong in code that does
//!   not touch macOS APIs, so they can be unit tested; see [`side::resolve`](crate::side::resolve).
//!
//! ## Rebrand a fork
//!
//! Edit `about.rs` (name, author, links, bundle id), `Cargo.toml`, `LICENSE`, and keep
//! `BUNDLE_ID` identical in `about.rs` and `scripts/bundle.sh`.

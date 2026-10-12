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
//! * [Keep the Settings look consistent](#keep-the-settings-look-consistent)
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
//! ## Keep the Settings look consistent
//!
//! The Settings window is flat and square, like a tiling window manager (Hyprland). When you add
//! or change a widget:
//!
//! 1. Take colors, radius, spacing and height from `ui/settings/theme.rs` (`BG`, `CARD`, `IDLE`,
//!    `HOT`, `BORDER`, `RADIUS`, `GAP`, `CONTROL_H`). Do not write `Color32::from_rgb(..)`, a corner
//!    radius or a pixel height in an editor. Text on the accent color uses `on_accent(accent)`,
//!    because the user picks the accent and it can be light.
//! 2. Every button, chip, input and list row is `CONTROL_H` (32) high, and there is one gap, `GAP`
//!    (8), between panels and between controls. Do not add your own spacing numbers.
//! 3. A tab is a stack of `panel(ui, "Title", ..)` cards (two columns with `ui.columns(2, ..)` when
//!    it is long). Use the widgets in `ui/settings/widgets.rs` (`chip`, `tile`, `choice`, `toggle`,
//!    `text_field`, `nav_item`, `info_row`) instead of egui's defaults (`radio_value`, `checkbox`,
//!    `TextEdit::singleline`): they share one marker style and one height. Rows of plain text and
//!    links go in `info_row` (22 px), not in a bare `ui.horizontal` (32 px).
//! 4. Navigation and the Save / Cancel / Quit buttons live in the sidebar (`window.rs`) and
//!    messages in the status bar below the body. A new tab is one more entry in the sidebar.
//! 5. Text is `TEXT` (primary) or `MUTED` (secondary, for labels and hints). An option that is off
//!    keeps `TEXT`: dim text reads as disabled. Do not rely on `.strong()` for a color, egui takes
//!    it from `widgets.active.fg_stroke`; set the color explicitly.
//! 6. Use words, not emoji, on buttons and chips. Single glyphs are only for tiny icon buttons
//!    (`+`, `✖`, `◀`, `▶`); give those a tooltip.
//! 7. Focus is the accent color: a 2 px border for the selected tile, a 1 px outline for the
//!    hovered one. Do not add shadows or gradients inside panels.
//!
//! The pie itself stays radial: its slices are picked by angle, and that hit-testing is shared
//! with the event tap (see `geometry.rs`).
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

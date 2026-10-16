//! The settings card: a sidebar of sections (Menu, Behavior, General, About) and the page of the selected one.
//!
//! | Module | Contents |
//! |---|---|
//! | `state` | editor-only state ([`Ed`], [`Tab`], [`Slot`]) |
//! | `window` | the window, animation, top bar, sidebar, body and footer (`App::settings_ui`, `regions`) |
//! | `tabs/*` | one file per section (the [`Tab`] enum) (`menu` also has `profile_toolbar`) |
//! | `pie_designer` | the interactive pie used to pick and arrange slices |
//! | `inspector` | editor for one slice, `add_panel` for the "Add a slice" panel |
//! | `action_editor` | editor for one [`Action`](crate::config::Action) |
//! | `icon_picker`, `icon_catalog` | icon chooser and its catalog |
//! | `picker` | native file choosers run off the UI thread |
//! | `welcome` | the first-launch welcome window (`App::welcome_ui`), drawn in the same window and style |
//! | `undo` | undo / redo shortcuts and backup restore, on top of [`History`](crate::history::History) |
//! | `theme`, `widgets` | palette, geometry, egui style and small reusable widgets |
//!
//! Settings edits a *copy* of the config (`App::edit`); pressing Save sanitizes and applies it.
//! Every change to that copy can be undone and redone (`undo`), and the copy is dropped again when
//! Settings closes.
//!
//! The look is a quiet, dense tool UI: a graphite window with rounded corners, hairline borders,
//! almost square flat controls, monospace capitals for chrome and one accent color. See the `theme`
//! module for the rules and `docs/UI_DESIGN.md` for the reasoning.

mod action_editor;
mod add_panel;
mod icon_catalog;
mod icon_picker;
mod inspector;
mod picker;
mod pie_designer;
mod state;
mod tabs;
mod theme;
mod undo;
mod welcome;
mod widgets;
mod window;

pub use state::{Ed, Slot, Tab};
pub use welcome::Welcome;

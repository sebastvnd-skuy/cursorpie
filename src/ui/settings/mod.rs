//! The settings card: Menu (profiles + slices), Behavior, General and About tabs.
//!
//! | Module | Contents |
//! |---|---|
//! | `state` | editor-only state ([`Ed`], [`Tab`], [`Slot`]) |
//! | `window` | the card, animation, tab bar and footer (`App::settings_ui`) |
//! | `tabs/*` | one file per tab (`menu` also has `profile_toolbar`) |
//! | `pie_designer` | the interactive pie used to pick and arrange slices |
//! | `inspector` | editor for one slice, `add_panel` for the "Add a slice" panel |
//! | `action_editor` | editor for one [`Action`](crate::config::Action) |
//! | `icon_picker`, `icon_catalog` | icon chooser and its catalog |
//! | `picker` | native file choosers run off the UI thread |
//! | `theme`, `widgets` | palette, egui style and small reusable widgets |
//!
//! Settings edits a *copy* of the config (`App::edit`); pressing Save sanitizes and applies it.

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
mod widgets;
mod window;

pub use state::{Ed, Slot, Tab};

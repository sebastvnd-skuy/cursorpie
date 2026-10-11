//! The settings card: Menu (profiles + items), Behavior, General and About tabs.
//!
//! Layout:
//! - `state`         editor-only state (`Ed`, `Tab`, `Slot`, `Rec`)
//! - `action_editor` editor for one `Action`
//! - `items_editor`  editor for one slice + the "Add a slice" panel
//! - `icon_picker`  icon chooser (catalog in crate::icons + user icons folder)
//! - `widgets`      chip / icon button / tile helpers
//! - `tabs/*`        one file per tab
//! - `window`        the card, animation, tab bar and footer (`App::settings_ui`)

mod action_editor;
mod icon_picker;
mod items_editor;
mod pie_designer;
mod state;
mod tabs;
mod widgets;
mod window;

pub use state::{Ed, Slot, Tab};

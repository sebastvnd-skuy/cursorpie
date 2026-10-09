//! The settings card: Menu (profiles + items), Behavior, General and About tabs.
//!
//! Layout:
//! - `state`         editor-only state (`Ed`, `Tab`, `Slot`, `Rec`)
//! - `action_editor` editor for one `Action`
//! - `items_editor`  recursive editor for a ring of items / submenus
//! - `tabs/*`        one file per tab
//! - `window`        the card, animation, tab bar and footer (`App::settings_ui`)

mod action_editor;
mod items_editor;
mod state;
mod tabs;
mod window;

pub use state::{Ed, Slot, Tab};

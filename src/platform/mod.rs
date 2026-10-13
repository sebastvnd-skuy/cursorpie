//! Thin wrappers around macOS: permissions, running apps, launch-at-login, file choosers.
//!
//! All items are re-exported here (`platform::frontmost_bundle_id`, ...); the submodules only group them:
//!
//! | Submodule | Contents |
//! |---|---|
//! | `permissions` | Accessibility check and links to the System Settings panes |
//! | `apps` | frontmost app, app under the pointer, app icons, activation policy, cursor position |
//! | `memory` | returning freed memory to macOS after the UI goes to sleep |
//! | `login_item` | "Launch at login" LaunchAgent |
//! | `shell` | helpers that run macOS tools: `open`, `osascript`, `mdls`, `afplay` |
//! | [`layout`] | keyboard-layout snapshot (TIS + `UCKeyTranslate`) |

pub mod layout;

mod apps;
mod login_item;
mod memory;
mod permissions;
mod shell;

pub use apps::*;
pub use login_item::*;
pub use memory::*;
pub use permissions::*;
pub use shell::*;

//! The config data model: everything that is serialized to `config.json`.
//!
//! Enum variants are serialized by name (serde's default external tagging), so renaming one is a
//! breaking change of the file format.

use super::item_at;
use crate::keys::*;
use serde::{Deserialize, Serialize};

/// Something CursorPie can do.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
pub enum Action {
    /// Do nothing (for side buttons: let the click pass through to the app).
    #[default]
    Nothing,
    Key(Shortcut),
    /// App name ("Safari") or path ("/Applications/Safari.app"), passed to `open -a`.
    OpenApp(String),
    /// URL or file path, passed to `open`.
    OpenUrl(String),
    /// Shell command, run with `/bin/sh -c`.
    Command(String),
    Sequence(Vec<Action>),
}

/// What a slice does.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ItemKind {
    /// Selecting it makes the side buttons (back/forward) send these actions.
    Mode { back: Action, forward: Action },
    /// Runs the action when you release the button over the slice.
    Run(Action),
    /// Drag outward across the rim to open it.
    Submenu(Vec<Item>),
}

/// One slice of a ring.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Item {
    pub name: String,
    /// Text / emoji, or an image path (.png .jpg .jpeg .webp), e.g. "~/Pictures/undo.png"
    pub icon: String,
    pub kind: ItemKind,
}

/// How the pie of an app that has its own profile is opened.
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub enum AppGesture {
    /// Classic: hold opens the pie of the app in front (if it has one).
    Merged,
    /// New: hold opens the general pie, double-click opens the app's own pie.
    #[default]
    DoubleClick,
}

/// Which pie the side buttons follow in an app that has a pie of its own, until you choose another.
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub enum SideTarget {
    /// The app's own pie.
    #[default]
    App,
    /// The general pie, even inside an app that has its own.
    General,
}

/// How CursorPie decides which app a gesture is about.
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub enum TargetApp {
    /// The app in front (the one with keyboard focus).
    Focused,
    /// The app whose window is under the pointer, even if it is not in front.
    #[default]
    UnderCursor,
}

impl Item {
    /// Creates a side-button mode slice that sends the `back` / `forward` shortcuts.
    pub fn mode(name: &str, icon: &str, back: Shortcut, forward: Shortcut) -> Self {
        Self {
            name: name.into(),
            icon: icon.into(),
            kind: ItemKind::Mode {
                back: Action::Key(back),
                forward: Action::Key(forward),
            },
        }
    }
    /// Creates a slice that runs `action` when the button is released over it.
    pub fn run(name: &str, icon: &str, action: Action) -> Self {
        Self {
            name: name.into(),
            icon: icon.into(),
            kind: ItemKind::Run(action),
        }
    }
    /// Creates a slice that opens a nested ring.
    pub fn submenu(name: &str, icon: &str, items: Vec<Item>) -> Self {
        Self {
            name: name.into(),
            icon: icon.into(),
            kind: ItemKind::Submenu(items),
        }
    }
}

/// A pie: its slices, and the apps it applies to.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Profile {
    pub name: String,
    /// Bundle identifiers, e.g. "com.apple.Safari". Empty = fallback profile.
    pub apps: Vec<String>,
    pub items: Vec<Item>,
    /// Path (indices) of the item whose side-button actions are currently active.
    pub active: Vec<usize>,
}

impl Profile {
    /// The slice whose side-button actions are currently active, if `active` still points at one.
    pub fn active_item(&self) -> Option<&Item> {
        item_at(&self.items, &self.active)
    }
}

/// The whole configuration. `#[serde(default)]` keeps older files loadable: every field
/// missing from a file takes its value from [`Config::default`].
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub profiles: Vec<Profile>,
    pub pie_hold_ms: u64,
    pub settings_hold_ms: u64,
    pub pie_size: f32,
    pub accent: [u8; 3],
    /// Mouse button that opens the pie: 2 = middle, 5/6/7 = extra buttons.
    pub trigger_button: i64,
    /// Dragging right after pressing opens the pie instantly (disables middle-drag pass-through).
    pub flick: bool,
    pub sound: bool,
    pub launch_at_login: bool,
    pub app_gesture: AppGesture,
    pub double_click_ms: u64,
    /// Which app picks the pie (and gets the shortcuts): the one in front or the one under the pointer.
    pub target_app: TargetApp,
    /// With `UnderCursor`: bring that app to the front before sending it a shortcut.
    pub focus_target: bool,
    /// Which pie the side buttons follow in an app that has its own pie, until you choose another.
    pub side_default: SideTarget,
    /// Choosing a side-button mode in a pie makes the side buttons follow that pie.
    pub side_adopt: bool,
    /// Forget that choice when another app becomes the target.
    pub side_forget: bool,
    /// Right-click while the pie is open to switch which pie the side buttons follow.
    pub right_click_switch: bool,
    /// Ring color tells which pie the side buttons follow: accent = this one, `inactive_color` = the other.
    pub side_indicator: bool,
    pub inactive_color: [u8; 3],
    /// How long a pie preview stays after switching from the menu bar (0 = no preview).
    pub preview_ms: u64,
    /// The center of an app's own pie shows that app's icon (as a white silhouette) instead of the
    /// icon of the active side-button mode.
    pub center_app_icon: bool,
}

impl Config {
    /// Index of the general (fallback) profile: the first one with no apps, or 0.
    pub fn global_profile(&self) -> usize {
        self.profiles
            .iter()
            .position(|p| p.apps.is_empty())
            .unwrap_or(0)
    }

    /// Index of the profile that lists `bundle`, if any.
    pub fn app_profile_for(&self, bundle: Option<&str>) -> Option<usize> {
        let b = bundle?;
        self.profiles
            .iter()
            .position(|p| p.apps.iter().any(|a| a == b))
    }

    /// Which profile applies for the given frontmost app.
    pub fn profile_for(&self, bundle: Option<&str>) -> usize {
        self.app_profile_for(bundle)
            .unwrap_or_else(|| self.global_profile())
    }
}

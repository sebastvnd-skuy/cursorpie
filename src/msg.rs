//! Messages sent *to* the UI thread, from the event tap, the menu bar and short-lived worker threads.
//!
//! Senders use a [`Notifier`], which also wakes the UI so it handles the message even while its window
//! is hidden. The UI drains them in `App::logic` and applies each one in `ui::app::messages`.

use crate::keys::Shortcut;
use crate::ui::settings::Slot;
use eframe::egui;
use std::sync::mpsc;

/// Something the UI thread has to react to.
pub enum Msg {
    /// Open (or re-center, for submenus) the pie for `profile` at menu level `path`.
    ShowPie {
        x: f64,
        y: f64,
        profile: usize,
        path: Vec<usize>,
    },
    /// Open the Settings window centered on the screen containing (x, y); `about` selects the About tab.
    OpenSettings { x: f64, y: f64, about: bool },
    /// The hovered slice of the open pie changed.
    Hover(Option<usize>),
    /// Close the pie (also used to reset the UI after the tap was disabled or paused).
    HidePie,
    /// A shortcut was recorded while Settings was in recording mode.
    KeyCaptured(Shortcut),
    /// Recording was cancelled with a plain Esc.
    RecordCancelled,
    /// An image was chosen as the icon of slice `path` in `profile`.
    IconPicked {
        profile: usize,
        path: Vec<usize>,
        file: String,
    },
    /// An app was chosen for the "apps" list of `profile`; `bundle` is its bundle identifier.
    AppPicked { profile: usize, bundle: String },
    /// A config file to import was chosen (its path).
    ImportPicked(String),
    /// Menu bar item clicked (the menu item id).
    Tray(String),
    /// Take keyboard focus back after a native file chooser closed.
    Refocus,
    /// An app was chosen for an "Open app" action of the slice at `path` (`slot` = which action).
    ActionAppPicked {
        profile: usize,
        path: Vec<usize>,
        slot: Slot,
        app: String,
    },
}

/// A channel sender plus the egui context needed to wake the UI. Cheap to clone.
#[derive(Clone)]
pub struct Notifier {
    pub tx: mpsc::Sender<Msg>,
    pub ctx: egui::Context,
}

impl Notifier {
    /// Queues `m` and requests a repaint so the UI picks it up promptly. Never blocks.
    pub fn send(&self, m: Msg) {
        let _ = self.tx.send(m);
        self.ctx.request_repaint();
    }
}

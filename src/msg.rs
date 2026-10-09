//! Messages sent to the UI thread (from the event tap, tray menu and file-picker threads).

use crate::keys::Shortcut;
use eframe::egui;
use std::sync::mpsc;

pub enum Msg {
    /// Open (or re-center, for submenus) the pie for `profile` at menu level `path`.
    ShowPie { x: f64, y: f64, profile: usize, path: Vec<usize> },
    OpenSettings { x: f64, y: f64, about: bool },
    Hover(Option<usize>),
    HidePie,
    KeyCaptured(Shortcut),
    RecordCancelled,
    IconPicked { profile: usize, path: Vec<usize>, file: String },
    AppPicked { profile: usize, bundle: String },
    ImportPicked(String),
    /// Menu bar item clicked (the menu item id).
    Tray(String),
}

#[derive(Clone)]
pub struct Notifier {
    pub tx: mpsc::Sender<Msg>,
    pub ctx: egui::Context,
}

impl Notifier {
    pub fn send(&self, m: Msg) {
        let _ = self.tx.send(m);
        self.ctx.request_repaint();
    }
}

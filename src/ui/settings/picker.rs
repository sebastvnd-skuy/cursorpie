//! Native file choosers run off the UI thread.

use crate::msg::Msg;
use crate::platform::{self, FileKind};
use eframe::egui;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Opens the native file chooser on a worker thread (it blocks until the user answers), turns the
/// chosen path into a message with `make`, and finally asks the UI to take focus back.
///
/// `kind` says what the chooser accepts; `make` may return `None` to drop
/// the choice (for example when the file turns out to be unusable).
pub(super) fn spawn_file_picker(
    tx: Sender<Msg>,
    ctx: egui::Context,
    kind: FileKind,
    prompt: &'static str,
    make: impl FnOnce(String) -> Option<Msg> + Send + 'static,
) {
    std::thread::spawn(move || {
        if let Some(file) = platform::choose_file(kind, prompt) {
            if let Some(msg) = make(file) {
                let _ = tx.send(msg);
            }
        }
        std::thread::sleep(Duration::from_millis(120)); // give the OS time to hand focus back
        let _ = tx.send(Msg::Refocus);
        ctx.request_repaint();
    });
}

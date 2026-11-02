//! Native file choosers run off the UI thread, one at a time.
//!
//! The chooser is a window of the system, not a child of Settings, so Settings stays clickable
//! beside it. While one is open Settings is dimmed and ignores input ([`waiting_scrim`]), a second
//! chooser cannot be started, and a press of the trigger does not pull Settings over the chooser.

use super::theme::{CARD_RADIUS, pal};
use crate::msg::Msg;
use crate::platform::{self, FileKind};
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Set from the click that starts a chooser until the chooser is gone.
static CHOOSING: AtomicBool = AtomicBool::new(false);

/// Whether a native file chooser is open (or about to be).
pub fn choosing() -> bool {
    CHOOSING.load(Ordering::SeqCst)
}

/// Takes the right to open a chooser. `None` while another one is open. The right is given back
/// when the returned guard is dropped, also if the worker thread panics.
fn claim() -> Option<Release> {
    (!CHOOSING.swap(true, Ordering::SeqCst)).then_some(Release)
}

/// Gives the chooser back when dropped.
struct Release;

impl Drop for Release {
    fn drop(&mut self) {
        CHOOSING.store(false, Ordering::SeqCst);
    }
}

/// Opens the native file chooser on a worker thread (it blocks until the user answers), turns the
/// chosen path into a message with `make`, and finally asks the UI to take focus back. Does
/// nothing while another chooser is open.
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
    let Some(release) = claim() else {
        diag::log!("ui", "chooser_busy");
        return;
    };
    diag::log!("ui", "chooser_open");
    ctx.request_repaint(); // the dimming shows at once
    std::thread::spawn(move || {
        if let Some(file) = platform::choose_file(kind, prompt)
            && let Some(msg) = make(file)
        {
            let _ = tx.send(msg);
        }
        std::thread::sleep(Duration::from_millis(120)); // give the OS time to hand focus back
        drop(release);
        diag::log!("ui", "chooser_closed");
        let _ = tx.send(Msg::Refocus);
        ctx.request_repaint();
    });
}

/// While a chooser is open: dims the window over `bounds` and takes every click, so nothing
/// underneath can be changed or started a second time.
pub(super) fn waiting_scrim(ctx: &egui::Context, bounds: egui::Rect) {
    egui::Area::new(egui::Id::new("chooser_scrim"))
        .order(egui::Order::Foreground)
        .fixed_pos(bounds.min)
        .constrain(false)
        .show(ctx, |ui| {
            ui.allocate_rect(bounds, egui::Sense::click_and_drag());
            ui.painter().rect_filled(bounds, CARD_RADIUS, pal().scrim);
            // a pill under the words: the dimmed window shows through the scrim, and its text would
            // run into the words
            let galley = ui.painter().layout_no_wrap(
                "Waiting for the file chooser\u{2026}".to_string(),
                egui::FontId::proportional(15.0),
                pal().text,
            );
            let pill = egui::Rect::from_center_size(
                bounds.center(),
                galley.size() + egui::vec2(40.0, 24.0),
            );
            ui.painter()
                .rect_filled(pill, pill.height() / 2.0, pal().card);
            ui.painter()
                .galley(pill.center() - galley.size() / 2.0, galley, pal().text);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_chooser_at_a_time_and_it_is_given_back() {
        let first = claim();
        assert!(first.is_some());
        assert!(choosing());
        assert!(
            claim().is_none(),
            "a second click must not open a second chooser"
        );
        drop(first);
        assert!(!choosing());
        assert!(claim().is_some(), "free again once the first one is gone");
    }
}

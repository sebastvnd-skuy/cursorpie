//! Editor for one `Action`: a row of kind chips plus one row of input fields.

use super::widgets::chip;
use crate::config::*;
use crate::keys::{Shortcut, key};
use crate::ui::settings::widgets::*;
use eframe::egui::{self, Color32};

/// Chip labels. Order = index in `action_index`. Sequence (5) is only shown while it is active.
const KINDS: [&str; 5] = ["None", "Shortcut", "App", "URL / file", "Command"];

/// Position of the action's kind in [`KINDS`] (Sequence is 5).
fn action_index(a: &Action) -> usize {
    match a {
        Action::Nothing => 0,
        Action::Key(_) => 1,
        Action::OpenApp(_) => 2,
        Action::OpenUrl(_) => 3,
        Action::Command(_) => 4,
        Action::Sequence(_) => 5,
    }
}

/// A fresh action of kind `i` (see [`KINDS`]).
fn make_action(i: usize) -> Action {
    match i {
        1 => Action::Key(Shortcut::new(key::F13, 0)),
        2 => Action::OpenApp(String::new()),
        3 => Action::OpenUrl(String::new()),
        4 => Action::Command(String::new()),
        5 => Action::Sequence(Vec::new()),
        _ => Action::Nothing,
    }
}

/// Which buttons of the editor were clicked this frame.
#[derive(Default)]
pub(super) struct ActionClick {
    pub shortcut: bool,
    pub pick_app: bool,
}

/// Draws the editor for `a`. The caller reacts to the returned clicks (start recording, pick an app).
pub(super) fn action_editor(
    ui: &mut egui::Ui,
    _id: &str,
    a: &mut Action,
    recording: bool,
    accent: Color32,
) -> ActionClick {
    let mut out = ActionClick::default();
    let now = action_index(a);
    let mut sel = now;

    // kind: one square chip per kind, all with their label (they wrap onto a second row if needed)
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::splat(4.0);
        for (i, name) in KINDS.iter().enumerate() {
            if chip(ui, "", name, now == i, accent).clicked() {
                sel = i;
            }
        }
        if now == 5 {
            chip(ui, "", "Sequence", true, accent);
        }
    });
    if sel != now {
        *a = make_action(sel);
    }

    // input fields
    ui.horizontal(|ui| match a {
        Action::Key(sc) => {
            if shortcut_button(ui, sc, recording, accent).clicked() {
                out.shortcut = true;
            }
        }
        Action::OpenApp(s) => {
            let w = (ui.available_width() - 100.0).max(120.0);
            ui.add(
                text_field(s)
                    .desired_width(w)
                    .hint_text("Safari  or  /Applications/X.app"),
            );
            if ui.button("Pick…").clicked() {
                out.pick_app = true;
            }
        }
        Action::OpenUrl(s) => {
            ui.add(
                text_field(s)
                    .desired_width(ui.available_width())
                    .hint_text("https://…  or a file path"),
            );
        }
        Action::Command(s) => {
            ui.add(
                text_field(s)
                    .desired_width(ui.available_width())
                    .hint_text("shell command"),
            );
        }
        Action::Sequence(v) => {
            muted(
                ui,
                &format!("{} steps (edit config.json to change)", v.len()),
            );
        }
        Action::Nothing => muted(ui, "The click passes through."),
    });
    out
}

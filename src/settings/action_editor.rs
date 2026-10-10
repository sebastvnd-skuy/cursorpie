//! Editor for a single `Action` (type selector + value field).

use crate::config::*;
use crate::gfx::*;
use crate::keys::{Shortcut, key};
use eframe::egui::{self, Color32};

const ACTION_KINDS: [&str; 6] = [
    "None",
    "Shortcut",
    "Open app",
    "Open URL / file",
    "Shell command",
    "Sequence",
];

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

#[derive(Default)]
pub(super) struct ActionClick {
    pub shortcut: bool,
    pub pick_app: bool,
}

pub(super) fn action_editor(
    ui: &mut egui::Ui,
    id: &str,
    a: &mut Action,
    recording: bool,
    accent: Color32,
) -> ActionClick {
    let mut out = ActionClick::default();
    ui.horizontal(|ui| {
        let now = action_index(a);
        let mut sel = now;
        egui::ComboBox::from_id_salt(format!("{id}_kind"))
            .width(120.0)
            .selected_text(ACTION_KINDS[now])
            .show_ui(ui, |ui| {
                for (i, name) in ACTION_KINDS.iter().enumerate() {
                    ui.selectable_value(&mut sel, i, *name);
                }
            });
        if sel != now {
            *a = make_action(sel);
        }
        match a {
            Action::Key(sc) => {
                if shortcut_button(ui, sc, recording, accent).clicked() {
                    out.shortcut = true;
                }
            }
            Action::OpenApp(s) => {
                ui.add(
                    egui::TextEdit::singleline(s)
                        .desired_width(140.0)
                        .hint_text("Safari  or  /Applications/X.app"),
                );
                if ui.button("Pick app…").clicked() {
                    out.pick_app = true;
                }
            }
            Action::OpenUrl(s) => {
                ui.add(
                    egui::TextEdit::singleline(s)
                        .desired_width(200.0)
                        .hint_text("https://...  or a file path"),
                );
            }
            Action::Command(s) => {
                ui.add(
                    egui::TextEdit::singleline(s)
                        .desired_width(200.0)
                        .hint_text("shell command"),
                );
            }
            Action::Sequence(v) => {
                muted(
                    ui,
                    &format!("{} steps (edit config.json to change them)", v.len()),
                );
            }
            Action::Nothing => muted(ui, "click passes through to the app"),
        }
    });
    out
}

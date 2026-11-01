//! Editor for one `Action`: the kind in a drop-down, with the fields that kind needs beside it.

use crate::config::*;
use crate::keys::{Shortcut, key};
use crate::ui::settings::widgets::*;
use crate::ui::symbols;
use eframe::egui::{self, Color32};

/// The kinds in the drop-down. Order = index in `action_index`. Sequence (5) is only shown while it is active.
const KINDS: [(char, &str); 5] = [
    (symbols::PROHIBIT, "None"),
    (symbols::KEYBOARD, "Shortcut"),
    (symbols::APP_WINDOW, "App"),
    (symbols::LINK, "URL / file"),
    (symbols::TERMINAL_WINDOW, "Command"),
];

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

    // one line: the kind in a drop-down, and next to it what that kind needs
    let mut items: Vec<(Option<char>, &str)> = KINDS
        .iter()
        .map(|&(icon, name)| (Some(icon), name))
        .collect();
    if now == 5 {
        items.push((Some(symbols::STACK), "Sequence"));
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if let Some(i) = select_icons(ui, 156.0, &items, now)
            && i != now
        {
            *a = make_action(i);
        }
        fields(ui, a, recording, accent, &mut out);
    });
    out
}

/// What the current kind of action needs: a shortcut, an app, a link or a command.
fn fields(
    ui: &mut egui::Ui,
    a: &mut Action,
    recording: bool,
    accent: Color32,
    out: &mut ActionClick,
) {
    match a {
        Action::Key(sc) => {
            if shortcut_button(ui, sc, recording, accent).clicked() {
                out.shortcut = true;
            }
        }
        Action::OpenApp(s) => {
            let w = (ui.available_width() - 110.0).max(120.0);
            ui.add(
                text_field(s)
                    .desired_width(w)
                    .hint_text(crate::platform::OS.open_app_hint()),
            );
            if button(ui, Look::Soft, Some(symbols::FOLDER_OPEN), "Pick\u{2026}").clicked() {
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
    }
}

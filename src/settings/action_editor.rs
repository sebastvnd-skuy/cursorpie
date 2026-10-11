//! Editor untuk satu `Action`: baris chip ikon (jenis) + satu baris isian.

use super::widgets::chip;
use crate::config::*;
use crate::gfx::*;
use crate::keys::{Shortcut, key};
use eframe::egui::{self, Color32};

/// (glyph, nama). Urutan = indeks di `action_index`. Sequence (5) hanya tampil kalau sedang aktif.
const KINDS: [(&str, &str); 5] = [
    ("🚫", "None"),
    ("⌨", "Shortcut"),
    ("🚀", "App"),
    ("🌐", "URL / file"),
    (">_", "Command"),
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
    _id: &str,
    a: &mut Action,
    recording: bool,
    accent: Color32,
) -> ActionClick {
    let mut out = ActionClick::default();
    let now = action_index(a);
    let mut sel = now;

    // jenis: yang aktif menampilkan label, sisanya hanya ikon (tooltip = nama)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (i, &(glyph, name)) in KINDS.iter().enumerate() {
            let on = now == i;
            if chip(ui, glyph, if on { name } else { "" }, on, accent)
                .on_hover_text(name)
                .clicked()
            {
                sel = i;
            }
        }
        if now == 5 {
            chip(ui, "🔁", "Sequence", true, accent);
        }
    });
    if sel != now {
        *a = make_action(sel);
    }

    // isian
    ui.horizontal(|ui| match a {
        Action::Key(sc) => {
            if shortcut_button(ui, sc, recording, accent).clicked() {
                out.shortcut = true;
            }
        }
        Action::OpenApp(s) => {
            let w = (ui.available_width() - 100.0).max(120.0);
            ui.add(
                egui::TextEdit::singleline(s)
                    .desired_width(w)
                    .hint_text("Safari  or  /Applications/X.app"),
            );
            if ui.button("📂 Pick").clicked() {
                out.pick_app = true;
            }
        }
        Action::OpenUrl(s) => {
            ui.add(
                egui::TextEdit::singleline(s)
                    .desired_width(ui.available_width())
                    .hint_text("https://…  or a file path"),
            );
        }
        Action::Command(s) => {
            ui.add(
                egui::TextEdit::singleline(s)
                    .desired_width(ui.available_width())
                    .hint_text("shell command"),
            );
        }
        Action::Sequence(v) => {
            muted(ui, &format!("{} steps (edit config.json to change)", v.len()));
        }
        Action::Nothing => muted(ui, "The click passes through."),
    });
    out
}

//! Undo and redo in Settings: the keyboard shortcuts, the actions behind the sidebar buttons,
//! restoring a backup, and feeding [`History`](crate::history::History) once per frame.
//!
//! The history watches `App::edit` as a whole, so *every* change is undoable without the editors
//! knowing about it: deleting a slice or a profile, editing an action, importing a file,
//! *Reset to defaults*, restoring a backup. The only things that are not part of the history are
//! the editor's own selection (`Ed`) and the text inside a focused text field, which egui undoes
//! itself.

use crate::config::load_backup;
use crate::ui::app::App;
use eframe::egui::{self, Key, Modifiers};
use std::time::Instant;

impl App {
    /// Handles Cmd+Z, Cmd+Shift+Z and Cmd+Y, unless a text field or the shortcut recorder owns
    /// the keyboard (a text field has an undo of its own).
    pub(in crate::ui::settings) fn history_keys(&mut self, ctx: &egui::Context) {
        if self.closing.is_some() || self.ed.recording.is_some() || ctx.egui_wants_keyboard_input()
        {
            return;
        }
        let (undo, redo) = ctx.input_mut(|i| {
            // redo first: Cmd+Shift+Z would also match Cmd+Z
            let redo = i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
                || i.consume_key(Modifiers::COMMAND, Key::Y);
            let undo = !redo && i.consume_key(Modifiers::COMMAND, Key::Z);
            (undo, redo)
        });
        if undo {
            self.undo_edit(ctx);
        } else if redo {
            self.redo_edit(ctx);
        }
    }

    /// Steps back one change.
    pub(in crate::ui::settings) fn undo_edit(&mut self, ctx: &egui::Context) {
        if self.history.undo(&mut self.edit) {
            self.ed.fit_to(&self.edit);
            ctx.request_repaint();
            let left = self.history.undo_steps();
            self.say(format!(
                "Undone ({left} more can be undone). Cmd+Shift+Z redoes it."
            ));
        } else {
            self.say("Nothing to undo.");
        }
    }

    /// Steps forward one change.
    pub(in crate::ui::settings) fn redo_edit(&mut self, ctx: &egui::Context) {
        if self.history.redo(&mut self.edit) {
            self.ed.fit_to(&self.edit);
            ctx.request_repaint();
            self.say("Redone.");
        } else {
            self.say("Nothing to redo.");
        }
    }

    /// Call at the end of every Settings frame: records what the widgets changed and schedules a
    /// frame for when an open burst of edits settles.
    pub(in crate::ui::settings) fn history_observe(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.history.observe(&self.edit, now);
        if let Some(wait) = self.history.next_commit_in(now) {
            ctx.request_repaint_after(wait);
        }
    }

    /// Loads backup `n` (1 = newest) into the editor. Nothing is applied until *Save*, and *Undo*
    /// brings back what was open before.
    pub(in crate::ui::settings) fn restore_backup(&mut self, n: usize) {
        match load_backup(n) {
            Some(backup) => {
                let login = self.edit.launch_at_login;
                self.edit = backup;
                self.edit.launch_at_login = login;
                self.ed.profile = 0;
                self.ed.reset_nav();
                self.ed.stop_recording();
                self.say(
                    "Backup loaded. Review it, then press Save. Undo brings back what you had.",
                );
            }
            None => self.say("That backup could not be read."),
        }
    }
}

//! General tab: permissions status and config import/export/reset.

use crate::config::backup::age_label;
use crate::config::*;
use crate::msg::Msg;
use crate::platform;
use crate::tap::TAP_ACTIVE;
use crate::ui::app::App;
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use eframe::egui::{self, RichText};
use std::sync::atomic::Ordering;
use std::time::SystemTime;

impl App {
    /// Draws the General tab: permission status, config location, import / export / reset.
    pub(in crate::ui::settings) fn general_tab(&mut self, ui: &mut egui::Ui) {
        let title = if platform::OS.needs_permission() {
            "Permissions"
        } else {
            "Input"
        };
        panel(ui, title, |ui| {
            let needs = platform::OS.needs_permission();
            let running = TAP_ACTIVE.load(Ordering::SeqCst);
            if needs {
                let trusted = platform::is_trusted_cached();
                info_row(ui, |ui| {
                    muted(ui, "Accessibility");
                    if trusted {
                        ui.label(RichText::new("Granted").color(GOOD));
                    } else {
                        ui.label(RichText::new("Missing").color(BAD));
                    }
                });
            }
            info_row(ui, |ui| {
                muted(ui, if needs { "Event tap" } else { "Mouse hook" });
                if running {
                    ui.label(RichText::new("Running").color(GOOD));
                } else {
                    ui.label(RichText::new("Not running (retrying every 2 s)").color(BAD));
                }
            });
            if needs {
                ui.horizontal(|ui| {
                    if ui.button("Open Accessibility settings").clicked() {
                        platform::open_accessibility_settings();
                    }
                    if ui.button("Open Input Monitoring settings").clicked() {
                        platform::open_input_monitoring_settings();
                    }
                });
                muted(
                    ui,
                    "If it still says 'Not running' after granting access, quit and reopen CursorPie.",
                );
            } else {
                muted(
                    ui,
                    "No permission is needed. Windows ignores input sent to programs running as administrator unless CursorPie runs as administrator too.",
                );
            }
        });
        panel(ui, "Configuration", |ui| {
            let path = config_path();
            ui.label(
                RichText::new(path.to_string_lossy())
                    .monospace()
                    .color(MUTED),
            );
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(format!("Reveal in {}", platform::OS.file_manager()))
                    .clicked()
                {
                    platform::reveal_in_file_manager(&path);
                }
                if ui.button("Export...").clicked() {
                    match export_config(&self.edit) {
                        Ok(p) => self.say(format!("Exported to {}", p.display())),
                        Err(e) => self.say(format!("Export failed: {e}")),
                    }
                }
                if ui.button("Import...").clicked() {
                    let (tx, ctx) = (self.tx.clone(), ui.ctx().clone());
                    std::thread::spawn(move || {
                        if let Some(f) = platform::choose_file(
                            platform::FileKind::Json,
                            "Choose a CursorPie config",
                        ) {
                            let _ = tx.send(Msg::ImportPicked(f));
                            ctx.request_repaint();
                        }
                    });
                }
                if ui.button("Reset to defaults").clicked() {
                    let login = self.edit.launch_at_login;
                    self.edit = Config::default();
                    self.edit.launch_at_login = login;
                    self.ed.profile = 0;
                    self.ed.stop_recording();
                    self.say("Defaults restored. Press Save to apply.");
                }
            });
            muted(
                ui,
                "Export saves your whole setup to ~/Downloads so you can share it. Only import configs you trust: actions can run shell commands.",
            );
        });
        panel(ui, "Backups", |ui| {
            muted(
                ui,
                "Every Save keeps the version it replaces, so a setting deleted by mistake is not gone for good. Restoring loads a backup into the editor; nothing changes until you press Save, and Undo brings back what you had.",
            );
            if self.ed.backups.is_empty() {
                muted(
                    ui,
                    "No backups yet. The first one is made by your next Save.",
                );
            }
            let now = SystemTime::now();
            let mut restore = None;
            ui.horizontal_wrapped(|ui| {
                for b in &self.ed.backups {
                    let age = b
                        .modified
                        .and_then(|t| now.duration_since(t).ok())
                        .map_or_else(|| "earlier".to_string(), age_label);
                    if ui.button(format!("Restore: {age}")).clicked() {
                        restore = Some(b.n);
                    }
                }
            });
            if let Some(n) = restore {
                self.restore_backup(n);
            }
        });
    }
}

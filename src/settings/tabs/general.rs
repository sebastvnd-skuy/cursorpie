//! General tab: permissions status and config import/export/reset.

use crate::app::App;
use crate::config::*;
use crate::gfx::*;
use crate::msg::Msg;
use crate::system;
use crate::tap::TAP_ACTIVE;
use eframe::egui::{self, RichText};
use std::sync::atomic::Ordering;

impl App {
    pub(in crate::settings) fn general_tab(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Permissions");
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let trusted = system::is_trusted();
            let running = TAP_ACTIVE.load(Ordering::SeqCst);
            ui.horizontal(|ui| {
                muted(ui, "Accessibility");
                if trusted {
                    ui.label(RichText::new("Granted").color(GOOD));
                } else {
                    ui.label(RichText::new("Missing").color(BAD));
                }
            });
            ui.horizontal(|ui| {
                muted(ui, "Event tap");
                if running {
                    ui.label(RichText::new("Running").color(GOOD));
                } else {
                    ui.label(RichText::new("Not running (retrying every 2 s)").color(BAD));
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Open Accessibility settings").clicked() {
                    system::open_accessibility_settings();
                }
                if ui.button("Open Input Monitoring settings").clicked() {
                    system::open_input_monitoring_settings();
                }
            });
            muted(
                ui,
                "If it still says 'Not running' after granting access, quit and reopen CursorPie.",
            );
        });

        ui.add_space(14.0);
        section_title(ui, "Configuration");
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let path = config_path();
            ui.label(RichText::new(path.to_string_lossy()).monospace().color(MUTED));
            ui.horizontal_wrapped(|ui| {
                if ui.button("Reveal in Finder").clicked() {
                    system::reveal_in_finder(&path);
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
                        if let Some(f) = system::choose_file("public.json", "Choose a CursorPie config") {
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
            muted(ui, "Export saves your whole setup to ~/Downloads so you can share it. Only import configs you trust: actions can run shell commands.");
        });
    }
}

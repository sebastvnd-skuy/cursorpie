//! Menu tab: profile management + the item editor.

use crate::app::App;
use crate::config::*;
use crate::gfx::*;
use crate::msg::Msg;
use crate::system;
use crate::settings::items_editor::{Env, items_editor};
use eframe::egui::{self, RichText};

impl App {
    pub(in crate::settings) fn menu_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let App { edit, ed, tx, .. } = self;
        ed.profile = ed.profile.min(edit.profiles.len().saturating_sub(1));
        let pi = ed.profile;
        let accent = rgb(edit.accent);

        section_title(ui, "Profile");
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let names: Vec<String> = edit.profiles.iter().map(|p| p.name.clone()).collect();
                egui::ComboBox::from_id_salt("profile_select")
                    .width(170.0)
                    .selected_text(names[pi].clone())
                    .show_ui(ui, |ui| {
                        for (i, name) in names.iter().enumerate() {
                            ui.selectable_value(&mut ed.profile, i, name.as_str());
                        }
                    });
                if ui
                    .add_enabled(edit.profiles.len() < MAX_PROFILES, egui::Button::new("New"))
                    .clicked()
                {
                    edit.profiles.push(Profile {
                        name: "New profile".into(),
                        apps: vec![],
                        items: vec![presets().remove(0)],
                        active: vec![0],
                    });
                    ed.profile = edit.profiles.len() - 1;
                    ed.stop_recording();
                }
                if ui
                    .add_enabled(edit.profiles.len() > 1, egui::Button::new("Delete"))
                    .clicked()
                {
                    edit.profiles.remove(pi);
                    ed.profile = pi.saturating_sub(1);
                    ed.stop_recording();
                }
            });

            let Some(prof) = edit.profiles.get_mut(ed.profile) else {
                return;
            };
            ui.horizontal(|ui| {
                muted(ui, "Name");
                ui.add(egui::TextEdit::singleline(&mut prof.name).desired_width(220.0));
            });

            muted(
                ui,
                if prof.apps.is_empty() {
                    "No apps listed: used for every app that has no profile of its own."
                } else {
                    "Used while one of these apps is in front:"
                },
            );
            let mut drop_app: Option<usize> = None;
            for (i, app) in prof.apps.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(app).monospace());
                    if ui.small_button("Remove").clicked() {
                        drop_app = Some(i);
                    }
                });
            }
            if let Some(i) = drop_app {
                prof.apps.remove(i);
            }
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut ed.new_app)
                        .desired_width(220.0)
                        .hint_text("bundle id, e.g. com.apple.Safari"),
                );
                if ui.button("Add").clicked() {
                    let id = ed.new_app.trim().to_string();
                    if !id.is_empty() && !prof.apps.contains(&id) {
                        prof.apps.push(id);
                    }
                    ed.new_app.clear();
                }
                if ui.button("Pick app...").clicked() {
                    let (tx, ctx, profile) = (tx.clone(), ctx.clone(), ed.profile);
                    std::thread::spawn(move || {
                        let Some(file) = system::choose_file(
                            "com.apple.application-bundle",
                            "Choose an application",
                        ) else {
                            return;
                        };
                        if let Some(bundle) = system::bundle_id_of(&file) {
                            let _ = tx.send(Msg::AppPicked { profile, bundle });
                            ctx.request_repaint();
                        }
                    });
                }
            });
        });

        ui.add_space(14.0);
        section_title(ui, "Menu items");
        muted(
            ui,
            "Hold the trigger button, drag to a slice and release. Side buttons use the active mode.",
        );
        ui.add_space(4.0);

        let env = Env {
            ctx,
            tx,
            accent,
            profile: ed.profile,
        };
        if let Some(prof) = edit.profiles.get_mut(ed.profile) {
            let mut path = Vec::new();
            items_editor(ui, ed, &env, &mut prof.items, &mut path);
        }
    }
}

//! Menu tab: profile management + the item editor.

use crate::app::App;
use crate::config::*;
use crate::gfx::*;
use crate::msg::Msg;
use crate::settings::items_editor::{Env, add_panel, item_inspector};
use crate::settings::pie_designer::{Designer, pie_designer};
use crate::system;
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
                let before = ed.profile;
                egui::ComboBox::from_id_salt("profile_select")
                    .width(170.0)
                    .selected_text(names[pi].clone())
                    .show_ui(ui, |ui| {
                        for (i, name) in names.iter().enumerate() {
                            ui.selectable_value(&mut ed.profile, i, name.as_str());
                        }
                    });
                if ed.profile != before {
                    ed.reset_nav();
                }
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
                    ed.reset_nav();
                    ed.stop_recording();
                }
                if ui
                    .add_enabled(edit.profiles.len() > 1, egui::Button::new("Delete"))
                    .clicked()
                {
                    edit.profiles.remove(pi);
                    ed.profile = pi.saturating_sub(1);
                    ed.reset_nav();
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
                        if let Some(file) = system::choose_file(
                            "com.apple.application-bundle",
                            "Choose an application",
                        ) {
                            if let Some(bundle) = system::bundle_id_of(&file) {
                                let _ = tx.send(Msg::AppPicked { profile, bundle });
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_millis(120));
                        let _ = tx.send(Msg::Refocus);
                        ctx.request_repaint();
                    });
                }
            });
        });

        ui.add_space(14.0);
        section_title(ui, "Your pie");
        muted(
            ui,
            "Click a slice to edit · click + to add · click the center to go back.",
        );

        let accent_rgb = edit.accent;
        let Some(prof) = edit.profiles.get_mut(ed.profile) else {
            return;
        };
        if items_at(&prof.items, &ed.nav).is_none() {
            ed.reset_nav();
        } // path basi setelah hapus item
        let nav = ed.nav.clone();

        // breadcrumb: Default › Launch › ...
        ui.horizontal_wrapped(|ui| {
            let mut go: Option<usize> = None;
            if ui.link(RichText::new(&prof.name).strong()).clicked() {
                go = Some(0);
            }
            for d in 0..nav.len() {
                ui.label("›");
                let name = item_at(&prof.items, &nav[..=d])
                    .map(|i| i.name.as_str())
                    .unwrap_or("?");
                if ui.link(name).clicked() {
                    go = Some(d + 1);
                }
            }
            if let Some(k) = go {
                ed.nav.truncate(k);
                ed.sel = None;
                ed.adding = false;
            }
        });

        let res = {
            let ring = items_at(&prof.items, &nav).unwrap_or(&[]);
            ui.vertical_centered(|ui| pie_designer(ui, ed, ctx, ring, accent_rgb))
                .inner
        };
        match res {
            Designer::Enter(i) => {
                ed.nav.push(i);
                ed.sel = None;
            }
            Designer::Up => {
                ed.sel = ed.nav.pop();
            } // langsung memilih submenu asalnya
            Designer::AddClicked => {
                ed.adding = true;
                ed.sel = None;
            }
            Designer::None => {}
        }

        let nav = ed.nav.clone(); // bisa berubah barusan
        let env = Env {
            ctx,
            tx,
            accent,
            profile: ed.profile,
        };
        if let Some(ring) = ring_mut(&mut prof.items, &nav) {
            if ed.adding {
                add_panel(ui, ed, ring, nav.len());
            } else {
                item_inspector(ui, ed, &env, ring, &nav);
            }
        }
    }
}

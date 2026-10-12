//! Profile toolbar of the Menu tab: select, create, delete and rename profiles, and edit the
//! apps a profile applies to.

use crate::config::*;
use crate::msg::Msg;
use crate::platform;
use crate::ui::settings::picker::spawn_file_picker;
use crate::ui::settings::state::Ed;
use crate::ui::settings::widgets::{icon_btn, muted, text_field};
use eframe::egui::containers::menu::{MenuButton, MenuConfig};
use eframe::egui::{self, Align, Layout, PopupCloseBehavior, RichText};
use std::sync::mpsc::Sender;

/// Like `ui.menu_button`, but the popup stays open while you click and type inside it.
///
/// egui's default closes a menu on *any* click, also one inside it. A text field in such a menu
/// therefore closed the menu as soon as you clicked it (the mouse button release counts as the
/// click), so it only stayed open while the mouse button was held down. Here the popup closes on a
/// click outside it, on the button again, or on Esc.
pub(super) fn sticky_menu<R>(
    ui: &mut egui::Ui,
    title: impl Into<String>,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::Response {
    let config = MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside);
    MenuButton::new(title.into())
        .config(config)
        .ui(ui, content)
        .0
}

/// One row: profile picker, new / delete buttons, rename and "apps" popups.
pub(super) fn profile_toolbar(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    edit: &mut Config,
    ed: &mut Ed,
    tx: &Sender<Msg>,
) {
    ui.horizontal(|ui| {
        muted(ui, "Profile");
        let names: Vec<String> = edit.profiles.iter().map(|p| p.name.clone()).collect();
        let before = ed.profile;
        egui::ComboBox::from_id_salt("profile_select")
            .width(180.0)
            .selected_text(names[ed.profile].clone())
            .show_ui(ui, |ui| {
                for (i, name) in names.iter().enumerate() {
                    ui.selectable_value(&mut ed.profile, i, name.as_str());
                }
            });
        if ed.profile != before {
            ed.reset_nav();
        }
        let pi = ed.profile;

        if icon_btn(ui, "+", "New profile", edit.profiles.len() < MAX_PROFILES).clicked() {
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
        if icon_btn(ui, "✖", "Delete profile", edit.profiles.len() > 1).clicked() {
            edit.profiles.remove(pi);
            ed.profile = pi.saturating_sub(1);
            ed.reset_nav();
            ed.stop_recording();
        }

        let Some(prof) = edit.profiles.get_mut(ed.profile) else {
            return;
        };

        // rename
        sticky_menu(ui, "Rename", |ui| {
            ui.set_min_width(220.0);
            ui.add(
                text_field(&mut prof.name)
                    .desired_width(200.0)
                    .hint_text("Profile name"),
            );
        })
        .on_hover_text("Rename profile");

        // apps that use this profile
        let label = if prof.apps.is_empty() {
            "All apps".to_string()
        } else {
            format!(
                "{} app{}",
                prof.apps.len(),
                if prof.apps.len() == 1 { "" } else { "s" }
            )
        };
        let profile = ed.profile;
        sticky_menu(ui, label, |ui| {
            ui.set_min_width(320.0);
            muted(
                ui,
                if prof.apps.is_empty() {
                    "Used for every app without its own profile."
                } else {
                    "Used while one of these is in front:"
                },
            );
            let mut drop_app: Option<usize> = None;
            for (i, app) in prof.apps.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(app).monospace());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("✖").clicked() {
                            drop_app = Some(i);
                        }
                    });
                });
            }
            if let Some(i) = drop_app {
                prof.apps.remove(i);
            }
            ui.horizontal(|ui| {
                ui.add(
                    text_field(&mut ed.new_app)
                        .desired_width(170.0)
                        .hint_text("bundle id, e.g. com.apple.Safari"),
                );
                if ui.button("Add").clicked() {
                    let id = ed.new_app.trim().to_string();
                    if !id.is_empty() && !prof.apps.contains(&id) {
                        prof.apps.push(id);
                    }
                    ed.new_app.clear();
                }
                if ui.button("Pick…").clicked() {
                    spawn_file_picker(
                        tx.clone(),
                        ctx.clone(),
                        "com.apple.application-bundle",
                        "Choose an application",
                        move |file| {
                            platform::bundle_id_of(&file)
                                .map(|bundle| Msg::AppPicked { profile, bundle })
                        },
                    );
                }
            });
        })
        .on_hover_text("Apps that use this profile");
    });
}

//! Profile toolbar of the Menu tab: select, create, delete and rename profiles, edit the apps a
//! profile applies to, and share (export / import) a profile with its icons.
//!
//! There is exactly one general profile (no apps, cannot be deleted). Every other profile belongs
//! to at least one app: the "New profile" form asks for it, and the last app cannot be removed.

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

/// Shows `text` in the status bar of Settings (this file has no access to `App`).
fn notice(tx: &Sender<Msg>, ctx: &egui::Context, text: impl Into<String>) {
    let _ = tx.send(Msg::Notice(text.into()));
    ctx.request_repaint();
}

/// The "New profile" form: a name and the app the profile belongs to, both required.
fn new_profile_form(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    edit: &mut Config,
    ed: &mut Ed,
    tx: &Sender<Msg>,
) {
    ui.set_min_width(320.0);
    muted(
        ui,
        "Every profile except the general one belongs to an app.",
    );
    ui.add(
        text_field(&mut ed.new_profile_name)
            .desired_width(300.0)
            .hint_text("Profile name"),
    );
    ui.horizontal(|ui| {
        ui.add(
            text_field(&mut ed.new_profile_app)
                .desired_width(220.0)
                .hint_text("bundle id, e.g. com.apple.Safari"),
        );
        if ui.button("Pick…").clicked() {
            spawn_file_picker(
                tx.clone(),
                ctx.clone(),
                "com.apple.application-bundle",
                "Choose an application",
                |file| platform::bundle_id_of(&file).map(Msg::NewProfileAppPicked),
            );
        }
    });
    let full = edit.profiles.len() >= MAX_PROFILES;
    let new = Profile::for_app(&ed.new_profile_name, &ed.new_profile_app);
    let create = ui.add_enabled(new.is_some() && !full, egui::Button::new("Create"));
    if full {
        muted(ui, "The profile limit is reached. Delete one first.");
    }
    if create.clicked()
        && let Some(profile) = new
    {
        edit.profiles.push(profile);
        ed.profile = edit.profiles.len() - 1;
        ed.reset_nav();
        ed.stop_recording();
        ed.new_profile_name.clear();
        ed.new_profile_app.clear();
        ui.close();
    }
}

/// One row: profile picker, new / delete buttons, rename, "apps" and "share" popups.
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

        sticky_menu(ui, "New profile", |ui| {
            new_profile_form(ui, ctx, edit, ed, tx)
        })
        .on_hover_text("Create a profile for an app");
        // the general profile is the fallback for every app, so it must stay
        let can_delete = edit.profiles.len() > 1 && !edit.profiles[pi].is_general();
        if icon_btn(
            ui,
            "✖",
            "Delete profile (the general profile cannot be deleted)",
            can_delete,
        )
        .clicked()
        {
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
        let general = prof.is_general();
        let label = if general {
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
            if general {
                muted(
                    ui,
                    "The general profile: used for every app without its own profile.",
                );
                return;
            }
            muted(ui, "Used while one of these is in front:");
            let mut drop_app: Option<usize> = None;
            let last = prof.apps.len() == 1;
            for (i, app) in prof.apps.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(app).monospace());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // a profile that is not the general one keeps at least one app
                        let remove = ui.add_enabled(!last, egui::Button::new("✖").small());
                        if remove
                            .on_hover_text("A profile needs at least one app")
                            .clicked()
                        {
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

        // share: export this profile with its icons, or import one
        let (_, _) = MenuButton::new("Share").ui(ui, |ui| {
            if ui.button("Export this profile…").clicked() {
                match share::export_to_downloads(prof) {
                    Ok(path) => {
                        platform::reveal_in_finder(&path);
                        notice(tx, ctx, format!("Exported to {}", path.display()));
                    }
                    Err(e) => notice(tx, ctx, format!("Export failed: {e}")),
                }
                ui.close();
            }
            if ui.button("Import a profile…").clicked() {
                spawn_file_picker(
                    tx.clone(),
                    ctx.clone(),
                    "public.json",
                    "Choose a CursorPie profile",
                    |file| Some(Msg::ProfileImportPicked(file)),
                );
                ui.close();
            }
        });
    });
}

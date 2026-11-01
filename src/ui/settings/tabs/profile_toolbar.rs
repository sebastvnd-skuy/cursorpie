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
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use crate::ui::symbols;
use eframe::egui::{self, RichText};
use std::sync::mpsc::Sender;

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
    accent: egui::Color32,
) {
    muted(
        ui,
        "Every profile except the general one belongs to an app.",
    );
    ui.add_space(GAP / 2.0);
    ui.add(
        text_field(&mut ed.new_profile_name)
            .desired_width(ui.available_width())
            .hint_text("Profile name"),
    );
    ui.horizontal(|ui| {
        ui.add(
            text_field(&mut ed.new_profile_app)
                .desired_width(ui.available_width() - 96.0)
                .hint_text(platform::OS.app_id_hint()),
        );
        if button(ui, Look::Soft, None, "Pick\u{2026}").clicked() {
            spawn_file_picker(
                tx.clone(),
                ctx.clone(),
                platform::FileKind::App,
                "Choose an application",
                |file| platform::bundle_id_of(&file).map(Msg::NewProfileAppPicked),
            );
        }
    });
    let full = edit.profiles.len() >= MAX_PROFILES;
    let new = Profile::for_app(&ed.new_profile_name, &ed.new_profile_app);
    ui.add_space(GAP / 2.0);
    let create = button_if(
        ui,
        new.is_some() && !full,
        Look::Primary(accent),
        Some(symbols::PLUS),
        "Create profile",
    );
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

/// One row: profile picker, then new / rename / apps / share popovers and delete.
///
/// Every popup here is a [`popover`], so they open, look and close the same way.
pub(super) fn profile_toolbar(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    edit: &mut Config,
    ed: &mut Ed,
    tx: &Sender<Msg>,
    accent: egui::Color32,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let before = ed.profile;
        // names are borrowed from the config: no copy of every name on every frame
        let current = edit.profiles[ed.profile].name.as_str();
        let names: Vec<&str> = edit.profiles.iter().map(|p| p.name.as_str()).collect();
        if let Some(i) = select(ui, 200.0, current, &names, ed.profile) {
            ed.profile = i;
        }
        if ed.profile != before {
            ed.reset_nav();
        }
        let pi = ed.profile;

        let new = button(ui, Look::Soft, Some(symbols::PLUS), "New");
        popover(
            &new.clone().on_hover_text("Create a profile for an app"),
            "New profile",
            340.0,
            |ui| new_profile_form(ui, ctx, edit, ed, tx, accent),
        );

        let Some(prof) = edit.profiles.get_mut(ed.profile) else {
            return;
        };

        // rename
        let rename = icon_button_look(
            ui,
            Look::Soft,
            symbols::PENCIL_SIMPLE,
            "Rename profile",
            true,
        );
        popover(&rename, "Rename profile", 300.0, |ui| {
            ui.add(
                text_field(&mut prof.name)
                    .desired_width(ui.available_width())
                    .hint_text("Profile name"),
            );
        });

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
        let apps = button(ui, Look::Soft, Some(symbols::APP_WINDOW), &label);
        popover(
            &apps.on_hover_text("Apps that use this profile"),
            "Apps of this profile",
            360.0,
            |ui| {
                if general {
                    muted(
                        ui,
                        "The general profile: used for every app without its own profile.",
                    );
                    return;
                }
                muted(ui, "Used while one of these is in front:");
                ui.add_space(GAP / 2.0);
                let mut drop_app: Option<usize> = None;
                let last = prof.apps.len() == 1;
                for (i, app) in prof.apps.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(app).size(13.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // a profile that is not the general one keeps at least one app
                            let remove = icon_button(
                                ui,
                                symbols::X,
                                "A profile needs at least one app",
                                !last,
                            );
                            if remove.clicked() {
                                drop_app = Some(i);
                            }
                        });
                    });
                }
                if let Some(i) = drop_app {
                    prof.apps.remove(i);
                }
                ui.add_space(GAP / 2.0);
                ui.horizontal(|ui| {
                    ui.add(
                        text_field(&mut ed.new_app)
                            .desired_width(ui.available_width() - 170.0)
                            .hint_text(platform::OS.app_id_hint()),
                    );
                    if button(ui, Look::Soft, None, "Add").clicked() {
                        let id = ed.new_app.trim().to_string();
                        if !id.is_empty() && !prof.apps.contains(&id) {
                            prof.apps.push(id);
                        }
                        ed.new_app.clear();
                    }
                    if button(ui, Look::Soft, None, "Pick\u{2026}").clicked() {
                        spawn_file_picker(
                            tx.clone(),
                            ctx.clone(),
                            platform::FileKind::App,
                            "Choose an application",
                            move |file| {
                                platform::bundle_id_of(&file)
                                    .map(|bundle| Msg::AppPicked { profile, bundle })
                            },
                        );
                    }
                });
            },
        );

        // share: export this profile with its icons, or import one
        let share = button(ui, Look::Soft, Some(symbols::EXPORT), "Share");
        popover(&share, "Share", 280.0, |ui| {
            if menu_item(ui, symbols::UPLOAD_SIMPLE, "Export this profile\u{2026}").clicked() {
                match share::export_to_downloads(prof) {
                    Ok(path) => {
                        platform::reveal_in_file_manager(&path);
                        notice(tx, ctx, format!("Exported to {}", path.display()));
                    }
                    Err(e) => notice(tx, ctx, format!("Export failed: {e}")),
                }
                ui.close();
            }
            if menu_item(ui, symbols::DOWNLOAD_SIMPLE, "Import a profile\u{2026}").clicked() {
                spawn_file_picker(
                    tx.clone(),
                    ctx.clone(),
                    platform::FileKind::Json,
                    "Choose a CursorPie profile",
                    |file| Some(Msg::ProfileImportPicked(file)),
                );
                ui.close();
            }
        });

        // the general profile is the fallback for every app, so it must stay
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_delete = edit.profiles.len() > 1 && !edit.profiles[pi].is_general();
            let tip = "Delete profile (the general profile cannot be deleted)";
            if icon_button_look(ui, Look::Danger, symbols::TRASH, tip, can_delete).clicked() {
                edit.profiles.remove(pi);
                ed.profile = pi.saturating_sub(1);
                ed.reset_nav();
                ed.stop_recording();
            }
        });
    });
}

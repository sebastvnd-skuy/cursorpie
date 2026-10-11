//! Menu tab: toolbar profil di atas, lalu dua kolom tanpa scroll luar.
//!   kiri  = breadcrumb + lingkaran pie
//!   kanan = editor slice / panel "Add a slice" / pemilih ikon (scroll sendiri kalau perlu)

use crate::app::App;
use crate::config::*;
use crate::gfx::*;
use crate::msg::Msg;
use crate::settings::items_editor::{Env, add_panel, item_inspector};
use crate::settings::pie_designer::{Designer, pie_designer};
use crate::settings::widgets::icon_btn;
use crate::system;
use eframe::egui::{self, Align, Layout, Rect, RichText, UiBuilder, pos2, vec2};
use std::sync::mpsc::Sender;

impl App {
    pub(in crate::settings) fn menu_tab(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        content_w: f32,
    ) {
        let App { edit, ed, tx, .. } = self;
        ed.profile = ed.profile.min(edit.profiles.len().saturating_sub(1));
        let accent = rgb(edit.accent);
        let accent_rgb = edit.accent;
        let tx_ref: &Sender<Msg> = tx;

        // ------------------------------------------------------------ toolbar profil
        ui.horizontal(|ui| {
            let names: Vec<String> = edit.profiles.iter().map(|p| p.name.clone()).collect();
            let before = ed.profile;
            egui::ComboBox::from_id_salt("profile_select")
                .width(170.0)
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

            if icon_btn(ui, "➕", "New profile", edit.profiles.len() < MAX_PROFILES).clicked() {
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
            if icon_btn(ui, "🗑", "Delete profile", edit.profiles.len() > 1).clicked() {
                edit.profiles.remove(pi);
                ed.profile = pi.saturating_sub(1);
                ed.reset_nav();
                ed.stop_recording();
            }

            let Some(prof) = edit.profiles.get_mut(ed.profile) else {
                return;
            };

            // ganti nama
            ui.menu_button("✏", |ui| {
                ui.set_min_width(220.0);
                ui.add(
                    egui::TextEdit::singleline(&mut prof.name)
                        .desired_width(200.0)
                        .hint_text("Profile name"),
                );
            })
            .response
            .on_hover_text("Rename profile");

            // aplikasi yang memakai profil ini
            let label = if prof.apps.is_empty() {
                "🖥 All apps".to_string()
            } else {
                format!(
                    "🖥 {} app{}",
                    prof.apps.len(),
                    if prof.apps.len() == 1 { "" } else { "s" }
                )
            };
            let profile = ed.profile;
            ui.menu_button(label, |ui| {
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
                        egui::TextEdit::singleline(&mut ed.new_app)
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
                    if ui.button("📂 Pick").clicked() {
                        let (tx, ctx) = (tx_ref.clone(), ctx.clone());
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
            })
            .response
            .on_hover_text("Apps that use this profile");
        });
        ui.add_space(4.0);

        // ------------------------------------------------------------ dua kolom
        // lebar dihitung eksplisit (bukan dari available_width) supaya tidak ikut melebar
        let avail = ui.available_rect_before_wrap();
        let left_w = 316.0;
        let gap = 14.0;
        let left = Rect::from_min_size(avail.min, vec2(left_w, avail.height()));
        let right = Rect::from_min_size(
            pos2(avail.min.x + left_w + gap, avail.min.y),
            vec2((content_w - left_w - gap).max(200.0), avail.height()),
        );

        let Some(prof) = edit.profiles.get_mut(ed.profile) else {
            return;
        };
        if items_at(&prof.items, &ed.nav).is_none() {
            ed.reset_nav();
        } // path basi setelah hapus item
        let nav = ed.nav.clone();
        let sel_before = ed.sel;

        // kiri: breadcrumb + pie
        let res = ui
            .scope_builder(UiBuilder::new().max_rect(left), |ui| {
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
                let size = (ui.available_height() - 6.0).clamp(200.0, 290.0);
                let ring = items_at(&prof.items, &nav).unwrap_or(&[]);
                ui.vertical_centered(|ui| pie_designer(ui, ed, ctx, ring, accent_rgb, size))
                    .inner
            })
            .inner;
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
        if ed.sel != sel_before {
            ed.picking_icon = false;
        }

        // kanan: editor
        let nav = ed.nav.clone(); // bisa berubah barusan
        let env = Env {
            ctx,
            tx: tx_ref,
            accent,
            profile: ed.profile,
            width: right.width(),
        };
        ui.scope_builder(UiBuilder::new().max_rect(right), |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(right)); // apa pun yang meluber dipotong
            egui::ScrollArea::vertical()
                .id_salt("inspector_scroll")
                .max_height(right.height())
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_max_width(right.width());
                    if let Some(ring) = ring_mut(&mut prof.items, &nav) {
                        if ed.adding {
                            add_panel(ui, ed, ring, nav.len(), accent, right.width());
                        } else {
                            item_inspector(ui, ed, &env, ring, &nav);
                        }
                    }
                });
        });
    }
}

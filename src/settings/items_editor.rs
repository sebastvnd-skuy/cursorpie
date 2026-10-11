//! Editor satu slice (di panel kanan) + panel "Add a slice". Submenu = rekursi lewat navigasi di menu.rs.

use super::action_editor::action_editor;
use super::icon_picker::icon_picker;
use super::state::{Ed, Rec, Slot};
use super::widgets::{chip, icon_btn, tile};
use crate::config::*;
use crate::gfx::*;
use crate::keys::{Shortcut, key};
use crate::msg::Msg;
use crate::system;
use crate::tap::{RECORDING, SWALLOW_KEY};
use eframe::egui::{self, Align, Align2, Color32, FontId, Layout, RichText, Vec2};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Read-only things the recursive editor needs.
pub(super) struct Env<'a> {
    pub ctx: &'a egui::Context,
    pub tx: &'a Sender<Msg>,
    pub accent: Color32,
    pub profile: usize,
    /// lebar panel kanan (dihitung eksplisit oleh menu.rs)
    pub width: f32,
}

fn default_kind(k: usize) -> ItemKind {
    match k {
        0 => presets().remove(0).kind,
        1 => ItemKind::Run(Action::Key(Shortcut::new(key::F13, 0))),
        _ => ItemKind::Submenu(Vec::new()),
    }
}

fn is_rec(ed: &Ed, profile: usize, path: &[usize], slot: Slot) -> bool {
    ed.recording
        .as_ref()
        .is_some_and(|r| r.profile == profile && r.path == path && r.slot == slot)
}

fn start_rec(ed: &mut Ed, profile: usize, path: &[usize], slot: Slot) {
    if is_rec(ed, profile, path, slot) {
        ed.stop_recording();
    } else {
        SWALLOW_KEY.store(-1, Ordering::SeqCst);
        RECORDING.store(true, Ordering::SeqCst);
        ed.recording = Some(Rec {
            profile,
            path: path.to_vec(),
            slot,
        });
    }
}

fn pick_icon(env: &Env, path: &[usize]) {
    let (tx, ctx, profile, path) = (env.tx.clone(), env.ctx.clone(), env.profile, path.to_vec());
    std::thread::spawn(move || {
        if let Some(file) = system::choose_file("public.image", "Choose an icon image") {
            let _ = tx.send(Msg::IconPicked {
                profile,
                path,
                file,
            });
        }
        std::thread::sleep(Duration::from_millis(120)); // beri waktu macOS selesai pindah fokus
        let _ = tx.send(Msg::Refocus);
        ctx.request_repaint();
    });
}

fn pick_app_for_action(env: &Env, path: &[usize], slot: Slot) {
    let (tx, ctx, profile, path) = (env.tx.clone(), env.ctx.clone(), env.profile, path.to_vec());
    std::thread::spawn(move || {
        if let Some(app) =
            system::choose_file("com.apple.application-bundle", "Choose an application")
        {
            let _ = tx.send(Msg::ActionAppPicked {
                profile,
                path,
                slot,
                app,
            });
        }
        std::thread::sleep(Duration::from_millis(120));
        let _ = tx.send(Msg::Refocus);
        ctx.request_repaint();
    });
}

pub(super) fn item_inspector(
    ui: &mut egui::Ui,
    ed: &mut Ed,
    env: &Env,
    ring: &mut Vec<Item>,
    nav: &[usize],
) {
    let Some(i) = ed.sel.filter(|&i| i < ring.len()) else {
        ui.add_space(70.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("👈").size(38.0));
            muted(ui, "Pick a slice");
        });
        return;
    };
    let n = ring.len();
    let mut path = nav.to_vec();
    path.push(i);

    // ---- mode pemilih ikon: menggantikan inspector sementara
    if ed.picking_icon {
        let item = &mut ring[i];
        let r = icon_picker(ui, ed, env.ctx, &mut item.icon, env.accent, env.width);
        if let Some(g) = r.chosen {
            item.icon = g;
            ed.picking_icon = false;
        }
        if r.pick_file {
            pick_icon(env, &path);
            ed.picking_icon = false;
        }
        if r.back {
            ed.picking_icon = false;
        }
        return;
    }

    let uid = format!(
        "insp_{}_{}",
        env.profile,
        path.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join("-")
    );
    let (rb, rf, rr) = (
        is_rec(ed, env.profile, &path, Slot::Back),
        is_rec(ed, env.profile, &path, Slot::Forward),
        is_rec(ed, env.profile, &path, Slot::Run),
    );
    let (mut start, mut remove, mut enter, mut shift) = (None::<Slot>, false, false, 0i32);
    let mut pick_app: Option<Slot> = None;
    let mut change_icon = false;

    egui::Frame::new()
        .fill(CARD)
        .corner_radius(egui::CornerRadius::same(14))
        .inner_margin(egui::Margin::same(12))
        .stroke(egui::Stroke::new(1.0, BORDER))
        .show(ui, |ui| {
            ui.set_width(env.width - 26.0);
            let item = &mut ring[i];

            // ikon (klik = ganti) + nama + jenis slice
            ui.horizontal(|ui| {
                let tex = ed.icon_for(env.ctx, &item.icon);
                let (r, resp) = tile(ui, Vec2::splat(58.0), false, env.accent);
                draw_icon(ui.painter(), tex.as_ref(), &item.icon, r.center(), 34.0, 255);
                ui.painter().text(
                    r.right_bottom() - Vec2::new(9.0, 9.0),
                    Align2::CENTER_CENTER,
                    "✏",
                    FontId::proportional(11.0),
                    Color32::from_white_alpha(150),
                );
                if resp.on_hover_text("Change icon").clicked() {
                    change_icon = true;
                }
                ui.vertical(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut item.name)
                            .desired_width(ui.available_width())
                            .hint_text("Name"),
                    );
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let now = match &item.kind {
                            ItemKind::Run(_) => 0,
                            ItemKind::Mode { .. } => 1,
                            ItemKind::Submenu(_) => 2,
                        };
                        let mut sel = now;
                        if chip(ui, "⚡", "Action", now == 0, env.accent)
                            .on_hover_text("Runs when you release over this slice")
                            .clicked()
                        {
                            sel = 0;
                        }
                        if chip(ui, "↔", "Side buttons", now == 1, env.accent)
                            .on_hover_text("Sets what the mouse back / forward buttons do")
                            .clicked()
                        {
                            sel = 1;
                        }
                        if nav.len() + 1 < MAX_DEPTH
                            && chip(ui, "📂", "Submenu", now == 2, env.accent)
                                .on_hover_text("Opens another ring")
                                .clicked()
                        {
                            sel = 2;
                        }
                        if sel != now {
                            item.kind = match sel {
                                0 => default_kind(1),
                                1 => default_kind(0),
                                _ => default_kind(2),
                            };
                        }
                    });
                });
            });

            ui.add_space(4.0);
            match &mut item.kind {
                ItemKind::Run(a) => {
                    let r = action_editor(ui, &format!("{uid}_r"), a, rr, env.accent);
                    if r.shortcut {
                        start = Some(Slot::Run);
                    }
                    if r.pick_app {
                        pick_app = Some(Slot::Run);
                    }
                }
                ItemKind::Mode { back, forward } => {
                    muted(ui, "◀  Back button");
                    let r = action_editor(ui, &format!("{uid}_b"), back, rb, env.accent);
                    if r.shortcut {
                        start = Some(Slot::Back);
                    }
                    if r.pick_app {
                        pick_app = Some(Slot::Back);
                    }
                    muted(ui, "▶  Forward button");
                    let r = action_editor(ui, &format!("{uid}_f"), forward, rf, env.accent);
                    if r.shortcut {
                        start = Some(Slot::Forward);
                    }
                    if r.pick_app {
                        pick_app = Some(Slot::Forward);
                    }
                }
                ItemKind::Submenu(ch) => {
                    muted(ui, &format!("{} slices inside", ch.len()));
                    if chip(ui, "✏", "Edit submenu  ›", false, env.accent).clicked() {
                        enter = true;
                    }
                }
            }

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if icon_btn(ui, "◀", "Move left", i > 0).clicked() {
                    shift = -1;
                }
                if icon_btn(ui, "▶", "Move right", i + 1 < n).clicked() {
                    shift = 1;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_btn(ui, "🗑", "Delete slice", n > 1 || !nav.is_empty()).clicked() {
                        remove = true;
                    }
                });
            });
        });

    if let Some(slot) = start {
        start_rec(ed, env.profile, &path, slot);
    }
    if let Some(slot) = pick_app {
        pick_app_for_action(env, &path, slot);
    }
    if change_icon {
        ed.picking_icon = true;
        ed.icon_query.clear();
        ed.user_icons = crate::icons::user_icons();
    }
    if shift != 0 {
        let j = (i as i32 + shift) as usize;
        ring.swap(i, j);
        ed.sel = Some(j);
        ed.stop_recording();
    }
    if remove {
        ring.remove(i);
        ed.sel = None;
        ed.stop_recording();
    }
    if enter {
        ed.nav.push(i);
        ed.sel = None;
    }
}

pub(super) fn add_panel(
    ui: &mut egui::Ui,
    ed: &mut Ed,
    ring: &mut Vec<Item>,
    depth: usize,
    accent: Color32,
    width: f32,
) {
    let mut picked: Option<Item> = None;
    let mut cancel = false;
    egui::Frame::new()
        .fill(CARD)
        .corner_radius(egui::CornerRadius::same(14))
        .inner_margin(egui::Margin::same(12))
        .stroke(egui::Stroke::new(1.0, BORDER))
        .show(ui, |ui| {
            let inner_w = width - 26.0;
            ui.set_width(inner_w);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Add a slice").size(15.0).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_btn(ui, "✖", "Cancel", true).clicked() {
                        cancel = true;
                    }
                });
            });
            let groups: [(&str, Vec<Item>); 3] = [
                ("Quick actions", quick_actions()),
                ("Side buttons", presets()),
                ("Custom", blank_items(depth + 1 < MAX_DEPTH)),
            ];
            let (tw, th, gap) = (86.0_f32, 64.0_f32, 6.0_f32);
            let cols = (((inner_w + gap) / (tw + gap)).floor().max(1.0)) as usize;
            for (title, list) in groups {
                muted(ui, title);
                for row in list.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for it in row {
                            let (r, resp) = tile(ui, Vec2::new(tw, th), false, accent);
                            draw_icon(
                                ui.painter(),
                                None,
                                &it.icon,
                                r.center() - Vec2::new(0.0, 10.0),
                                26.0,
                                255,
                            );
                            ui.painter().text(
                                r.center() + Vec2::new(0.0, 20.0),
                                Align2::CENTER_CENTER,
                                &it.name,
                                FontId::proportional(11.0),
                                Color32::from_white_alpha(190),
                            );
                            if resp.clicked() {
                                picked = Some(it.clone());
                            }
                        }
                    });
                }
            }
        });
    if cancel {
        ed.adding = false;
    }
    if let Some(it) = picked {
        ring.push(it);
        ed.sel = Some(ring.len() - 1);
        ed.adding = false;
    }
}

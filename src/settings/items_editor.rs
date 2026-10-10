//! Recursive editor for one ring of menu items (submenus recurse).

use super::action_editor::action_editor;
use super::state::{Ed, Rec, Slot};
use crate::config::*;
use crate::gfx::*;
use crate::keys::{Shortcut, key};
use crate::msg::Msg;
use crate::system;
use crate::tap::{RECORDING, SWALLOW_KEY};
use eframe::egui::{self, Color32, CornerRadius, Vec2};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Read-only things the recursive editor needs.
pub(super) struct Env<'a> {
    pub ctx: &'a egui::Context,
    pub tx: &'a Sender<Msg>,
    pub accent: Color32,
    pub profile: usize,
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
        muted(ui, "Click a slice on the circle to choose what it does.");
        return;
    };
    let n = ring.len();
    let mut path = nav.to_vec();
    path.push(i);
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
    let (mut start, mut pick, mut remove, mut enter, mut shift) =
        (None::<Slot>, false, false, false, 0i32);
    let mut pick_app: Option<Slot> = None;

    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        let item = &mut ring[i];

        // nama + ikon
        ui.horizontal(|ui| {
            let tex = ed.icon_for(env.ctx, &item.icon);
            let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(r, CornerRadius::same(12), Color32::from_rgb(14, 15, 18));
            draw_icon(
                ui.painter(),
                tex.as_ref(),
                &item.icon,
                r.center(),
                28.0,
                255,
            );
            ui.vertical(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut item.name)
                        .desired_width(220.0)
                        .hint_text("Name"),
                );
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut item.icon)
                            .desired_width(110.0)
                            .hint_text("Icon (text/emoji)"),
                    );
                    if ui.button("Pick image…").clicked() {
                        pick = true;
                    }
                });
            });
        });

        // jenis (tombol segmented, bahasa awam)
        ui.add_space(6.0);
        muted(ui, "What should this slice do?");
        let now = match &item.kind {
            ItemKind::Run(_) => 0,
            ItemKind::Mode { .. } => 1,
            ItemKind::Submenu(_) => 2,
        };
        let mut sel = now;
        let labels: &[&str] = if nav.len() + 1 < MAX_DEPTH {
            &["Do something", "Side buttons", "Open submenu"]
        } else {
            &["Do something", "Side buttons"]
        };
        if segmented(ui, &mut sel, labels, env.accent) {
            item.kind = match sel {
                0 => default_kind(1),
                1 => default_kind(0),
                _ => default_kind(2),
            };
        }

        match &mut item.kind {
            ItemKind::Run(a) => {
                muted(ui, "Runs when you release the button over this slice.");
                let r = action_editor(ui, &format!("{uid}_r"), a, rr, env.accent);
                if r.shortcut {
                    start = Some(Slot::Run);
                }
                if r.pick_app {
                    pick_app = Some(Slot::Run);
                }
            }
            ItemKind::Mode { back, forward } => {
                muted(
                    ui,
                    "While this slice is active, your mouse side buttons send:",
                );
                muted(ui, "Back button");
                let r = action_editor(ui, &format!("{uid}_b"), back, rb, env.accent);
                if r.shortcut {
                    start = Some(Slot::Back);
                }
                if r.pick_app {
                    pick_app = Some(Slot::Back);
                }
                muted(ui, "Forward button");
                let r = action_editor(ui, &format!("{uid}_f"), forward, rf, env.accent);
                if r.shortcut {
                    start = Some(Slot::Forward);
                }
                if r.pick_app {
                    pick_app = Some(Slot::Forward);
                }
            }
            ItemKind::Submenu(ch) => {
                muted(
                    ui,
                    &format!(
                        "{} slices inside. Drag outward over this slice to open it.",
                        ch.len()
                    ),
                );
                if ui.button("Edit submenu  ›").clicked() {
                    enter = true;
                }
            }
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(i > 0, egui::Button::new("‹ Move")).clicked() {
                shift = -1;
            }
            if ui
                .add_enabled(i + 1 < n, egui::Button::new("Move ›"))
                .clicked()
            {
                shift = 1;
            }
            if ui
                .add_enabled(n > 1 || !nav.is_empty(), egui::Button::new("Delete"))
                .clicked()
            {
                remove = true;
            }
        });
    });

    if let Some(slot) = start {
        start_rec(ed, env.profile, &path, slot);
    }
    if let Some(slot) = pick_app {
        pick_app_for_action(env, &path, slot);
    }
    if pick {
        pick_icon(env, &path);
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

pub(super) fn add_panel(ui: &mut egui::Ui, ed: &mut Ed, ring: &mut Vec<Item>, depth: usize) {
    let mut picked: Option<Item> = None;
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        section_title(ui, "Add a slice");
        let groups: [(&str, Vec<Item>); 3] = [
            ("Quick actions", quick_actions()),
            ("Side-button modes", presets()),
            ("Custom", blank_items(depth + 1 < MAX_DEPTH)),
        ];
        for (title, list) in groups {
            muted(ui, title);
            ui.horizontal_wrapped(|ui| {
                for it in list {
                    if ui.button(format!("{}  {}", it.icon, it.name)).clicked() {
                        picked = Some(it);
                    }
                }
            });
        }
        if ui.button("Cancel").clicked() {
            ed.adding = false;
        }
    });
    if let Some(it) = picked {
        ring.push(it);
        ed.sel = Some(ring.len() - 1);
        ed.adding = false;
    }
}

//! Recursive editor for one ring of menu items (submenus recurse).

use super::action_editor::action_editor;
use super::state::{Ed, Rec, Slot};
use crate::config::*;
use crate::gfx::*;
use crate::keys::{Shortcut, key};
use crate::msg::Msg;
use crate::system;
use crate::tap::{RECORDING, SWALLOW_KEY};
use eframe::egui::{self, Color32, CornerRadius, RichText, Vec2};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;

/// Read-only things the recursive editor needs.
pub(super) struct Env<'a> {
    pub ctx: &'a egui::Context,
    pub tx: &'a Sender<Msg>,
    pub accent: Color32,
    pub profile: usize,
}

const KIND_NAMES: [&str; 3] = ["Side-button mode", "Action", "Submenu"];

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
            ctx.request_repaint();
        }
    });
}

/// Edits one ring of items; submenus recurse.
pub(super) fn items_editor(
    ui: &mut egui::Ui,
    ed: &mut Ed,
    env: &Env,
    items: &mut Vec<Item>,
    path: &mut Vec<usize>,
) {
    let depth = path.len();
    let n = items.len();
    let mut remove: Option<usize> = None;
    let mut swap: Option<(usize, usize)> = None;

    for i in 0..n {
        path.push(i);
        let uid = format!(
            "{}_{}",
            env.profile,
            path.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("-")
        );
        let rec_back = is_rec(ed, env.profile, path, Slot::Back);
        let rec_fwd = is_rec(ed, env.profile, path, Slot::Forward);
        let rec_run = is_rec(ed, env.profile, path, Slot::Run);
        let mut start: Option<Slot> = None;
        let mut pick = false;

        ui.push_id(uid.clone(), |ui| {
            card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                let item = &mut items[i];

                // row 1: number, icon preview, name, kind, preset
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{}", i + 1))
                            .color(env.accent)
                            .strong(),
                    );
                    let tex = ed.icon_for(env.ctx, &item.icon);
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::hover());
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(8),
                        Color32::from_rgb(14, 15, 18),
                    );
                    draw_icon(
                        ui.painter(),
                        tex.as_ref(),
                        &item.icon,
                        rect.center(),
                        22.0,
                        255,
                    );

                    ui.add(egui::TextEdit::singleline(&mut item.name).desired_width(120.0));

                    let now = match &item.kind {
                        ItemKind::Mode { .. } => 0,
                        ItemKind::Run(_) => 1,
                        ItemKind::Submenu(_) => 2,
                    };
                    let mut sel = now;
                    egui::ComboBox::from_id_salt(format!("{uid}_type"))
                        .width(115.0)
                        .selected_text(KIND_NAMES[now])
                        .show_ui(ui, |ui| {
                            for (k, name) in KIND_NAMES.iter().enumerate() {
                                if k == 2 && depth + 1 >= MAX_DEPTH {
                                    continue; // no deeper submenus
                                }
                                ui.selectable_value(&mut sel, k, *name);
                            }
                        });
                    if sel != now {
                        item.kind = default_kind(sel);
                    }

                    if now == 0 {
                        egui::ComboBox::from_id_salt(format!("{uid}_preset"))
                            .width(60.0)
                            .selected_text("Preset")
                            .show_ui(ui, |ui| {
                                for p in presets() {
                                    if ui.selectable_label(false, p.name.clone()).clicked() {
                                        *item = p;
                                    }
                                }
                            });
                    }
                });

                // row 2: icon text + browse + order/remove
                ui.horizontal_wrapped(|ui| {
                    muted(ui, "Icon");
                    ui.add(
                        egui::TextEdit::singleline(&mut item.icon)
                            .desired_width(150.0)
                            .hint_text("text, emoji or image path"),
                    );
                    if ui.button("Browse...").clicked() {
                        pick = true;
                    }
                    if ui.add_enabled(i > 0, egui::Button::new("Up")).clicked() {
                        swap = Some((i, i - 1));
                    }
                    if ui
                        .add_enabled(i + 1 < n, egui::Button::new("Down"))
                        .clicked()
                    {
                        swap = Some((i, i + 1));
                    }
                    if ui
                        .add_enabled(n > 1 || depth > 0, egui::Button::new("Remove"))
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });

                // row 3: what the item does
                match &mut item.kind {
                    ItemKind::Mode { back, forward } => {
                        muted(ui, "Back side button");
                        if action_editor(ui, &format!("{uid}_b"), back, rec_back, env.accent) {
                            start = Some(Slot::Back);
                        }
                        muted(ui, "Forward side button");
                        if action_editor(ui, &format!("{uid}_f"), forward, rec_fwd, env.accent) {
                            start = Some(Slot::Forward);
                        }
                    }
                    ItemKind::Run(a) => {
                        muted(ui, "Runs when you release the button over this slice");
                        if action_editor(ui, &format!("{uid}_r"), a, rec_run, env.accent) {
                            start = Some(Slot::Run);
                        }
                    }
                    ItemKind::Submenu(children) => {
                        muted(ui, "Drag outward across the rim of this slice to open it");
                        egui::CollapsingHeader::new(format!(
                            "Items in this submenu ({})",
                            children.len()
                        ))
                        .id_salt(format!("{uid}_sub"))
                        .default_open(true)
                        .show(ui, |ui| {
                            items_editor(ui, ed, env, children, path);
                        });
                    }
                }
            });
        });

        if let Some(slot) = start {
            start_rec(ed, env.profile, path, slot);
        }
        if pick {
            pick_icon(env, path);
        }
        path.pop();
        ui.add_space(6.0);
    }

    if let Some((a, b)) = swap {
        items.swap(a, b);
        ed.stop_recording();
    }
    if let Some(i) = remove {
        items.remove(i);
        ed.stop_recording();
    }

    if items.len() < MAX_ITEMS {
        let mut add: Option<Item> = None;
        egui::ComboBox::from_id_salt(format!("add_{}_{}", env.profile, path.len()))
            .selected_text("+ Add item")
            .show_ui(ui, |ui| {
                muted(ui, "Side-button modes");
                for p in presets() {
                    if ui.selectable_label(false, p.name.clone()).clicked() {
                        add = Some(p);
                    }
                }
                ui.separator();
                muted(ui, "Quick actions");
                for q in quick_actions() {
                    if ui.selectable_label(false, q.name.clone()).clicked() {
                        add = Some(q);
                    }
                }
                ui.separator();
                muted(ui, "Custom");
                for b in blank_items(depth + 1 < MAX_DEPTH) {
                    if ui.selectable_label(false, b.name.clone()).clicked() {
                        add = Some(b);
                    }
                }
            });
        if let Some(it) = add {
            items.push(it);
        }
    } else {
        muted(ui, &format!("Maximum {MAX_ITEMS} items per ring reached."));
    }
}

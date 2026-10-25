//! The "Add a slice" panel: ready-made slices to append to the current ring.

use super::state::{AddCatalog, Ed};
use crate::config::*;
use crate::ui::draw::*;
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use eframe::egui::{self, Align, Align2, Color32, FontId, Layout, Vec2};

/// Draws the panel; picking a tile appends that slice to `ring` and selects it.
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
    // built once when the panel opens, not on every frame (see `AddCatalog`)
    let can_nest = depth + 1 < MAX_DEPTH;
    if ed
        .add_catalog
        .as_ref()
        .is_none_or(|c| c.can_nest != can_nest)
    {
        ed.add_catalog = Some(AddCatalog::new(can_nest));
    }
    let Some(catalog) = ed.add_catalog.as_ref() else {
        return;
    };
    card().show(ui, |ui| {
        let inner_w = width - 26.0;
        ui.set_width(inner_w);
        ui.horizontal(|ui| {
            card_title(ui, "Add a slice");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_btn(ui, "✖", "Cancel", true).clicked() {
                    cancel = true;
                }
            });
        });
        let (tw, th, gap) = (86.0_f32, 64.0_f32, GAP);
        let cols = (((inner_w + gap) / (tw + gap)).floor().max(1.0)) as usize;
        for (title, list) in &catalog.groups {
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
        ed.add_catalog = None;
    }
    if let Some(it) = picked {
        ring.push(it);
        ed.sel = Some(ring.len() - 1);
        ed.adding = false;
        ed.add_catalog = None;
    }
}

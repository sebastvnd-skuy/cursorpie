//! The "Add a slice" panel: ready-made slices to append to the current ring.

use super::state::Ed;
use crate::config::*;
use crate::ui::draw::*;
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use eframe::egui::{self, Align, Align2, Color32, FontId, Layout, RichText, Vec2};

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

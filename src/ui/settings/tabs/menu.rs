//! Menu tab: profile toolbar on top, then two columns without an outer scroll area.
//!   left  = breadcrumb + the interactive pie
//!   right = slice editor / "Add a slice" panel / icon picker (scrolls on its own if needed)

use super::profile_toolbar::profile_toolbar;
use crate::config::*;
use crate::msg::Msg;
use crate::ui::app::App;
use crate::ui::draw::*;
use crate::ui::settings::add_panel::add_panel;
use crate::ui::settings::inspector::{Env, item_inspector};
use crate::ui::settings::pie_designer::{Designer, pie_designer};
use eframe::egui::{self, Rect, RichText, UiBuilder, pos2, vec2};
use std::sync::mpsc::Sender;

impl App {
    /// Draws the Menu tab into `ui`, `content_w` pixels wide.
    pub(in crate::ui::settings) fn menu_tab(
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

        profile_toolbar(ui, ctx, edit, ed, tx_ref);
        ui.add_space(4.0);

        // ------------------------------------------------------------ two columns
        // widths are computed explicitly (not from available_width) so nothing grows with its content
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
        } // stale path after an item was deleted
        let nav = ed.nav.clone();
        let sel_before = ed.sel;

        // left: breadcrumb + pie
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
            } // select the submenu we came from
            Designer::AddClicked => {
                ed.adding = true;
                ed.sel = None;
            }
            Designer::None => {}
        }
        if ed.sel != sel_before {
            ed.picking_icon = false;
        }

        // right: editor
        let nav = ed.nav.clone(); // may have just changed
        let env = Env {
            ctx,
            tx: tx_ref,
            accent,
            profile: ed.profile,
            width: right.width(),
        };
        ui.scope_builder(UiBuilder::new().max_rect(right), |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(right)); // clip anything that overflows
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

//! Drawing of the pie menu (one ring at a time; submenus re-center the same window).

mod view;

use crate::config::{MAX_ITEMS, cfg};
use crate::geometry::*;
use crate::ui::app::App;
use crate::ui::draw::*;
use eframe::egui::{self, Align2, Color32, FontId, Shape, Stroke, Vec2};
use std::f32::consts::{PI, TAU};
use view::PieView;

impl App {
    /// Draws the pie (one ring) and advances its animations. Hides the window once the close animation ended.
    pub fn pie_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let dt = if self.fresh {
            self.fresh = false;
            0.0
        } else {
            ctx.input(|i| i.stable_dt).min(0.05)
        };

        let view = {
            let c = cfg();
            PieView::new(&c, self.profile, &self.path)
        };
        let total = view.items.len().max(1);

        let mut moving = smooth_approach(
            &mut self.anim,
            if self.pie_open { 1.0 } else { 0.0 },
            dt,
            16.0,
        );
        for i in 0..MAX_ITEMS {
            moving |= smooth_approach(
                &mut self.hover_anim[i],
                (self.hover == Some(i)) as u8 as f32,
                dt,
                20.0,
            );
            moving |= smooth_approach(
                &mut self.active_anim[i],
                (view.active == Some(i)) as u8 as f32,
                dt,
                12.0,
            );
        }

        if !self.pie_open && self.anim <= 0.0 {
            if self.visible {
                self.visible = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            return;
        }
        let top_level = self.path.is_empty();
        let previewing = self.preview_until.is_some();
        if moving || (self.pie_open && self.hover.is_none() && top_level) {
            ctx.request_repaint();
        }

        // load image icons before painting (needs &mut self)
        let tex: Vec<_> = view
            .items
            .iter()
            .map(|(ic, _, _)| self.ed.icon_for(&ctx, ic))
            .collect();
        let center_tex = self.ed.icon_for(&ctx, &view.center_icon);
        // None while not available (not installed, unreadable): the mode icon is shown instead
        let app_tex = view
            .app_icon
            .as_ref()
            .and_then(|s| self.ed.icon_for(&ctx, s));

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ui, |ui| {
                let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                let painter = ui.painter_at(rect);
                let c = rect.center();

                let fade = 1.0 - (1.0 - self.anim).powi(3);
                let pop = ease_out_back(self.anim);
                let r = pie_outer(self.win) * (0.80 + 0.20 * pop);
                let inner = pie_inner(self.win) * (0.75 + 0.25 * pop);
                let accent = rgb(view.accent);
                // false = the side buttons follow the other pie: tint this one with the inactive color
                let live = !view.side.shown || view.side.owner;
                let a = |v: f32, f: f32| (v * f).clamp(0.0, 255.0) as u8;

                // soft shadow behind the ring
                painter.circle_filled(c, r + 4.0, Color32::from_black_alpha(a(60.0, fade)));

                let sl = TAU / total as f32;
                let gap = if total > 1 { 0.05 } else { 0.0 };

                for (i, (icon, name, is_sub)) in view.items.iter().enumerate().take(MAX_ITEMS) {
                    let h = self.hover_anim[i];
                    let m = self.active_anim[i];

                    // slices appear one after another (stagger) while rotating slightly
                    let lp = (self.anim * 1.6 - 0.6 * i as f32 / total as f32).clamp(0.0, 1.0);
                    let lpe = ease_out_back(lp).max(0.0);
                    let lf = 1.0 - (1.0 - lp) * (1.0 - lp);
                    let start = slice_start(i, total) - (1.0 - lp) * 0.35;
                    let rr = inner + (r + 5.0 * h - inner) * lpe;

                    let t = (h * 0.95 + m * 0.4).min(1.0);
                    let col = if live {
                        mix([24, 26, 32], view.accent, t)
                    } else {
                        // the active mode is tinted with the inactive color, hovering still uses the accent
                        mix(
                            mix([24, 26, 32], view.inactive, (m * 0.4).min(1.0)),
                            view.accent,
                            (h * 0.95).min(1.0),
                        )
                    };
                    let fill =
                        Color32::from_rgba_unmultiplied(col[0], col[1], col[2], a(228.0, lf));
                    painter.add(ring_slice(
                        c,
                        inner,
                        rr,
                        start + gap / 2.0,
                        start + sl - gap / 2.0,
                        fill,
                    ));

                    let am = start + sl / 2.0;
                    let dir = Vec2::new(am.cos(), am.sin());
                    let tp = c + dir * ((inner + rr) / 2.0);
                    draw_icon(
                        &painter,
                        tex[i].as_ref(),
                        icon,
                        tp - Vec2::new(0.0, r * 0.04),
                        r * 0.30 * (1.0 + 0.15 * h),
                        a(255.0, lf),
                    );
                    painter.text(
                        tp + Vec2::new(0.0, r * 0.17),
                        Align2::CENTER_CENTER,
                        name,
                        FontId::proportional((r * 0.09).max(9.0)),
                        Color32::from_white_alpha(a(170.0, lf)),
                    );
                    if *is_sub {
                        // dot on the rim: drag outward across it to open the submenu
                        painter.circle_filled(
                            c + dir * (rr - 6.0),
                            2.5,
                            Color32::from_white_alpha(a(210.0, lf)),
                        );
                    }
                }

                // ring: accent = the side buttons follow this pie, red = they follow the other one
                let (ring_col, ring_w) = if view.side.shown {
                    let col = if view.side.owner {
                        accent
                    } else {
                        rgb(view.inactive)
                    };
                    (col.gamma_multiply(fade), 3.0)
                } else if view.is_app {
                    (accent, 2.5)
                } else {
                    (Color32::from_white_alpha(a(28.0, fade)), 1.0)
                };
                painter.circle_stroke(c, r, Stroke::new(ring_w, ring_col));

                // center disc: active mode (or the parent submenu icon) + "hold for settings" progress
                let core = inner * 0.86;
                painter.circle_filled(
                    c,
                    core,
                    if live {
                        Color32::from_rgba_unmultiplied(14, 15, 18, a(235.0, fade))
                    } else {
                        Color32::from_rgba_unmultiplied(36, 14, 14, a(235.0, fade))
                    },
                );
                if let Some(t) = app_tex.as_ref() {
                    // the app's icon as a white glyph
                    draw_icon(
                        &painter,
                        Some(t),
                        "",
                        c,
                        core * 1.3,
                        a(if live { 215.0 } else { 110.0 }, fade),
                    );
                } else {
                    draw_icon(
                        &painter,
                        center_tex.as_ref(),
                        &view.center_icon,
                        c,
                        core * 1.0,
                        a(if live { 150.0 } else { 80.0 }, fade),
                    );
                }
                if self.pie_open
                    && !previewing
                    && self.hover.is_none()
                    && top_level
                    && view.settle_ms > 0.0
                {
                    let prog = (self.opened_at.elapsed().as_millis() as f32 / view.settle_ms)
                        .clamp(0.0, 1.0);
                    if prog > 0.02 {
                        let pts = arc_points(c, core + 3.0, -PI / 2.0, -PI / 2.0 + TAU * prog);
                        painter.add(Shape::line(pts, Stroke::new(3.0, accent)));
                    }
                }
            });
    }
}

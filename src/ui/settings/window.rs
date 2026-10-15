//! The settings window itself: open/close animation, card chrome, and its layout:
//!
//! ```text
//! +---------+------------------------------------------+
//! | title   |                                          |
//! | Menu    |   body of the selected tab               |
//! | Behavior|                                          |
//! | General |                                          |
//! | About   |                                          |
//! |         +------------------------------------------+
//! | Save    | status bar: warning, message or hint     |
//! | Cancel  |                                          |
//! | Quit app|                                          |
//! +---------+------------------------------------------+
//! ```
//!
//! The sidebar holds everything you navigate or confirm with, so those buttons never move.

use super::state::Tab;
use super::tabs::about::about_tab;
use crate::about::*;
use crate::config::backups;
use crate::platform;
use crate::tap::TAP_ACTIVE;
use crate::ui::app::App;
use crate::ui::draw::*;
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use eframe::egui::{self, Color32, RichText, Vec2};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Width of the sidebar (navigation and Save / Cancel / Quit).
const SIDEBAR_W: f32 = 140.0;

/// What the sidebar buttons asked for this frame.
enum UiAction {
    None,
    Undo,
    Redo,
    Save,
    Cancel,
    Quit,
}

impl App {
    /// Draws the Settings card (animation, sidebar, body, status bar) and applies Save / Cancel / Quit.
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();

        // open / close animation
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        let target = if self.closing.is_some() { 0.0 } else { 1.0 };
        if smooth_approach(&mut self.settings_anim, target, dt, 14.0) {
            ctx.request_repaint();
        }
        if let Some(save) = self.closing
            && self.settings_anim <= 0.02
        {
            self.close_settings(&ctx, save);
            return;
        }
        // keep permission status fresh while the window is open
        ctx.request_repaint_after(Duration::from_millis(1000));

        let t = self.settings_anim.clamp(0.0, 1.0);
        let slide = (1.0 - t) * (1.0 - t) * 26.0;

        // Esc (when not recording) = cancel
        if self.closing.is_none()
            && self.ed.recording.is_none()
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.closing = Some(false);
        }
        self.history_keys(&ctx);
        if self
            .toast
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(6))
        {
            self.toast = None;
        }

        let accent = rgb(self.edit.accent);
        style_settings(ui, accent);
        ui.set_opacity(t);

        // card + shadow, always centered in the window. Everything is square; the border of the
        // card is the "focused window" border (accent, fading to a greener shade).
        let full = ui.max_rect();
        let card_rect = egui::Rect::from_center_size(full.center(), self.card_size)
            .translate(Vec2::new(0.0, slide));
        ui.painter().add(
            egui::Shadow {
                offset: [0, 6],
                blur: 20,
                spread: 0,
                color: Color32::from_black_alpha(110),
            }
            .as_shape(card_rect, RADIUS),
        );
        ui.painter().rect_filled(card_rect, RADIUS, BG);
        focus_border(ui.painter(), card_rect, 2.0, accent);

        // sidebar on the left, body + status bar on the right
        let inner = card_rect.shrink(GAP * 2.0);
        let side = egui::Rect::from_min_size(inner.min, Vec2::new(SIDEBAR_W, inner.height()));
        let right =
            egui::Rect::from_min_max(egui::pos2(side.max.x + GAP * 2.0, inner.min.y), inner.max);
        let status =
            egui::Rect::from_min_max(egui::pos2(right.min.x, right.max.y - CONTROL_H), right.max);
        let body = egui::Rect::from_min_max(right.min, egui::pos2(right.max.x, status.min.y - GAP));
        // thin divider between the sidebar and the rest
        let divider_x = side.max.x + GAP;
        ui.painter().line_segment(
            [
                egui::pos2(divider_x, inner.min.y),
                egui::pos2(divider_x, inner.max.y),
            ],
            egui::Stroke::new(1.0, BORDER),
        );

        // dragging the title area moves the window
        let title_rect = egui::Rect::from_min_size(side.min, Vec2::new(SIDEBAR_W, 36.0));
        let drag = ui.interact(
            title_rect,
            egui::Id::new("settings_drag"),
            egui::Sense::click_and_drag(),
        );
        if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let mut action = UiAction::None;
        let (can_undo, can_redo) = (self.history.can_undo(), self.history.can_redo());

        // ---------------------------------------------------------------- sidebar
        ui.scope_builder(egui::UiBuilder::new().max_rect(side), |ui| {
            ui.horizontal(|ui| {
                let (logo, _) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
                draw_logo(ui.painter(), logo.center(), 12.5, accent);
                ui.label(
                    RichText::new(APP_NAME)
                        .monospace()
                        .size(17.0)
                        .strong()
                        .color(TEXT),
                );
                ui.label(RichText::new("ℹ").color(MUTED)).on_hover_text(
                    "Hold the middle button for your main pie.\nDouble-click it for the pie of the app under the pointer.\nWhile the pie is open, right-click to switch which pie the side buttons follow.",
                );
            });
            ui.add_space(GAP * 2.0);

            let tabs = [
                (Tab::Menu, "Menu"),
                (Tab::Behavior, "Behavior"),
                (Tab::General, "General"),
                (Tab::About, "About"),
            ];
            ui.spacing_mut().item_spacing.y = 4.0;
            for (tab, label) in tabs {
                if nav_item(ui, label, self.ed.tab == tab, accent, SIDEBAR_W).clicked() {
                    self.ed.tab = tab;
                    self.ed.picking_icon = false;
                    if tab == Tab::General {
                        self.ed.backups = backups(); // read once, not on every frame
                    }
                }
            }

            // Undo / Redo, then Save / Cancel / Quit sit at the bottom, always in the same place
            ui.spacing_mut().item_spacing.y = GAP;
            let buttons_h = 4.0 * CONTROL_H + 3.0 * GAP;
            ui.add_space((ui.available_height() - buttons_h).max(0.0));
            let half = Vec2::new((SIDEBAR_W - GAP) / 2.0, CONTROL_H);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                ui.add_enabled_ui(can_undo, |ui| {
                    let undo = ui.add_sized(half, egui::Button::new("Undo"));
                    if undo.on_hover_text(format!("Undo the last change ({}+Z)", crate::platform::OS.primary_name())).clicked() {
                        action = UiAction::Undo;
                    }
                });
                ui.add_enabled_ui(can_redo, |ui| {
                    let redo = ui.add_sized(half, egui::Button::new("Redo"));
                    if redo.on_hover_text(format!("Redo ({}+Shift+Z)", crate::platform::OS.primary_name())).clicked() {
                        action = UiAction::Redo;
                    }
                });
            });
            let size = Vec2::new(SIDEBAR_W, CONTROL_H);
            let save = egui::Button::new(RichText::new("Save").strong().color(on_accent(accent)))
                .fill(accent)
                .stroke(egui::Stroke::new(1.0, accent));
            if ui.add_sized(size, save).clicked() {
                action = UiAction::Save;
            }
            if ui.add_sized(size, egui::Button::new("Cancel")).clicked() {
                action = UiAction::Cancel;
            }
            let quit = egui::Button::new(RichText::new("Quit app").color(BAD));
            if ui.add_sized(size, quit).clicked() {
                action = UiAction::Quit;
            }
        });

        // ---------------------------------------------------------------- body
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(body));
            if self.ed.tab == Tab::Menu {
                // Menu: two tiles, no outer scroll
                self.menu_tab(ui, &ctx, body.width());
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.ed.tab {
                        Tab::Behavior => self.behavior_tab(ui),
                        Tab::General => self.general_tab(ui),
                        Tab::About => about_tab(ui, accent),
                        Tab::Menu => {}
                    });
            }
        });

        // ---------------------------------------------------------------- status bar
        ui.painter().rect_filled(status, RADIUS, INPUT);
        ui.painter().rect_stroke(
            status,
            RADIUS,
            egui::Stroke::new(1.0, BORDER),
            egui::StrokeKind::Inside,
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(status.shrink2(Vec2::new(GAP + 2.0, 0.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                if !platform::is_trusted_cached() || !TAP_ACTIVE.load(Ordering::SeqCst) {
                    if platform::OS.needs_permission() {
                        ui.label(RichText::new("⚠ Accessibility permission needed").color(BAD));
                        if ui.small_button("Open System Settings").clicked() {
                            platform::open_accessibility_settings();
                        }
                    } else {
                        ui.label(RichText::new("⚠ Mouse hook not running").color(BAD));
                    }
                } else if let Some((msg, _)) = &self.toast {
                    ui.label(RichText::new(msg).color(accent));
                } else {
                    muted(
                        ui,
                        &format!(
                            "Esc cancels  ·  {}+Z undoes  ·  Save applies every tab at once",
                            platform::OS.primary_name()
                        ),
                    );
                }
            },
        );

        match action {
            UiAction::Undo => self.undo_edit(&ctx),
            UiAction::Redo => self.redo_edit(&ctx),
            UiAction::Save => {
                self.ed.stop_recording();
                self.closing = Some(true);
            }
            UiAction::Cancel => {
                self.ed.stop_recording();
                self.closing = Some(false);
            }
            UiAction::Quit => self.quit(&ctx),
            UiAction::None => {}
        }
        self.history_observe(&ctx);
    }
}

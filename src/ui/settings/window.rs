//! The settings window itself: open/close animation, window chrome, and its layout:
//!
//! ```text
//! +----------------------------------------------------------------------+
//! | logo CURSORPIE ⓘ | MENU BEHAVIOR GENERAL ABOUT |  UNDO REDO CANCEL SAVE |  top bar
//! +----------------------------------------------------------------------+
//! |                                                                      |
//! |   body of the selected tab                                           |
//! |                                                                      |
//! +----------------------------------------------------------------------+
//! | status: warning, message or hint                          QUIT APP   |  footer
//! +----------------------------------------------------------------------+
//! ```
//!
//! The top bar holds everything you navigate or confirm with, so those buttons never move. The
//! three bands come from [`regions`], a pure function with its own tests.

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
use eframe::egui::{self, RichText, Vec2};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Height of the top bar (brand, tabs, Undo / Redo / Cancel / Save).
const TOP_H: f32 = 52.0;
/// Height of the footer (status line, Quit).
const FOOT_H: f32 = 34.0;
/// Height of a tab cell: shorter than the bar, so the tabs float in it like segments.
const TAB_H: f32 = 28.0;
/// Space between the bands and the body, and the body's padding.
const PAD: f32 = GAP * 2.0;
/// Width kept free at the right end of the footer for the Quit cell, so a message never runs
/// under it.
const QUIT_W: f32 = 110.0;

/// The three bands of the window, as rectangles in screen points.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Regions {
    top: egui::Rect,
    body: egui::Rect,
    foot: egui::Rect,
}

/// Cuts the `card` rectangle into the top bar, the body and the footer. The bar and the footer
/// span the full width (they touch the window's outline); the body is inset by [`PAD`].
fn regions(card: egui::Rect) -> Regions {
    let top = egui::Rect::from_min_size(card.min, Vec2::new(card.width(), TOP_H));
    let foot = egui::Rect::from_min_max(egui::pos2(card.min.x, card.max.y - FOOT_H), card.max);
    let body = egui::Rect::from_min_max(
        egui::pos2(card.min.x + PAD, top.max.y + PAD),
        egui::pos2(card.max.x - PAD, foot.min.y - PAD),
    );
    Regions { top, body, foot }
}

/// What the top bar and footer buttons asked for this frame.
enum UiAction {
    None,
    Undo,
    Redo,
    Save,
    Cancel,
    Quit,
}

impl App {
    /// Draws the Settings window (animation, top bar, body, footer) and applies Save / Cancel / Quit.
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

        // The window, always centered. Its rectangle is snapped to whole pixels: a hairline that
        // sits between two pixels is smeared over both, which is what made the old card look soft.
        let full = ui.max_rect();
        let card_rect = snap_rect(
            egui::Rect::from_center_size(full.center(), self.card_size)
                .translate(Vec2::new(0.0, slide)),
            ctx.pixels_per_point(),
        );
        let Regions { top, body, foot } = regions(card_rect);
        let painter = ui.painter().clone();
        painter.add(window_shadow().as_shape(card_rect, CARD_RADIUS));
        painter.rect_filled(card_rect, CARD_RADIUS, BG);
        painter.rect_filled(top, TOP_RADIUS, CARD);
        painter.rect_filled(foot, BOTTOM_RADIUS, INPUT);
        let hairline = |y: f32| {
            painter.line_segment(
                [egui::pos2(card_rect.min.x, y), egui::pos2(card_rect.max.x, y)],
                egui::Stroke::new(1.0, BORDER),
            );
        };
        hairline(top.max.y);
        hairline(foot.min.y);
        window_outline(&painter, card_rect, accent);

        // dragging the empty part of the top bar moves the window; the buttons are added after
        // it, so they sit on top and keep their clicks
        let drag = ui.interact(
            top,
            egui::Id::new("settings_drag"),
            egui::Sense::click_and_drag(),
        );
        if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let mut action = UiAction::None;
        let (can_undo, can_redo) = (self.history.can_undo(), self.history.can_redo());
        let primary = platform::OS.primary_name();

        // ---------------------------------------------------------------- top bar
        let bar_line = |ui: &egui::Ui, x: f32| {
            ui.painter().line_segment(
                [egui::pos2(x, top.min.y + 14.0), egui::pos2(x, top.max.y - 14.0)],
                egui::Stroke::new(1.0, BORDER),
            );
        };
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(top)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(PAD);
                let (logo, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                draw_logo(ui.painter(), logo.center(), 11.5, accent);
                ui.add_space(GAP + 2.0);
                ui.label(
                    RichText::new(APP_NAME.to_uppercase())
                        .monospace()
                        .size(14.0)
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(GAP);
                let info = ui.label(RichText::new("ℹ").color(MUTED)).on_hover_text(
                    "Hold the middle button for your main pie.\nDouble-click it for the pie of the app under the pointer.\nWhile the pie is open, right-click to switch which pie the side buttons follow.",
                );
                // gap, hairline, gap: the line sits in the middle of the space
                ui.add_space(PAD);
                bar_line(&*ui, info.rect.right() + PAD / 2.0);

                let tabs = [
                    (Tab::Menu, "MENU"),
                    (Tab::Behavior, "BEHAVIOR"),
                    (Tab::General, "GENERAL"),
                    (Tab::About, "ABOUT"),
                ];
                ui.spacing_mut().item_spacing.x = 4.0;
                for (tab, label) in tabs {
                    if tab_item(ui, label, self.ed.tab == tab, TAB_H).clicked() {
                        self.ed.tab = tab;
                        self.ed.picking_icon = false;
                        if tab == Tab::General {
                            self.ed.backups = backups(); // read once, not on every frame
                        }
                    }
                }
            },
        );
        // Right side, added from the right edge inwards. Save is the primary (accent) cell at the
        // corner. The cells float inside the bar instead of touching its edge, so none of them
        // pokes out of the window's rounded corner.
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(top.shrink2(Vec2::new(GAP + 2.0, 0.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let on_save = on_accent(accent);
                let save = bar_button(ui, "SAVE", true, Some(accent), on_save, CONTROL_H);
                if save.on_hover_text("Apply every tab at once").clicked() {
                    action = UiAction::Save;
                }
                let cancel = bar_button(ui, "CANCEL", true, None, TEXT, CONTROL_H);
                if cancel.clicked() {
                    action = UiAction::Cancel;
                }
                bar_line(&*ui, cancel.rect.left() - 2.0 - GAP);
                ui.add_space(GAP * 2.0);
                let redo = bar_button(ui, "REDO", can_redo, None, TEXT, CONTROL_H);
                if redo
                    .on_hover_text(format!("Redo ({primary}+Shift+Z)"))
                    .clicked()
                {
                    action = UiAction::Redo;
                }
                let undo = bar_button(ui, "UNDO", can_undo, None, TEXT, CONTROL_H);
                if undo
                    .on_hover_text(format!("Undo the last change ({primary}+Z)"))
                    .clicked()
                {
                    action = UiAction::Undo;
                }
            },
        );

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

        // ---------------------------------------------------------------- footer
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_max(
                    egui::pos2(foot.min.x + PAD, foot.min.y),
                    egui::pos2(foot.max.x - QUIT_W, foot.max.y),
                ))
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
                    ui.label(
                        RichText::new(format!(
                            "Esc cancels  ·  {primary}+Z undoes  ·  Save applies every tab at once"
                        ))
                        .monospace()
                        .size(11.0)
                        .color(MUTED),
                    );
                }
            },
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(foot.shrink2(Vec2::new(GAP, 0.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                if bar_button(ui, "QUIT APP", true, None, BAD, FOOT_H - GAP).clicked() {
                    action = UiAction::Quit;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::app::{CARD_SIZE, WIN_MARGIN};

    fn card(w: f32, h: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(WIN_MARGIN, WIN_MARGIN), Vec2::new(w, h))
    }

    #[test]
    fn bands_tile_the_window_without_overlap() {
        let c = card(CARD_SIZE[0], CARD_SIZE[1]);
        let r = regions(c);
        assert_eq!(r.top.min, c.min, "the bar starts at the window's corner");
        assert_eq!(r.top.width(), c.width(), "the bar spans the window");
        assert_eq!(r.foot.max, c.max, "the footer ends at the window's corner");
        assert_eq!(r.foot.width(), c.width());
        assert_eq!(r.top.height(), TOP_H);
        assert_eq!(r.foot.height(), FOOT_H);
        assert!(r.top.max.y + PAD <= r.body.min.y + 1e-3, "body clears the bar");
        assert!(r.body.max.y + PAD <= r.foot.min.y + 1e-3, "body clears the footer");
        assert!(c.contains_rect(r.body));
    }

    #[test]
    fn the_body_is_inset_by_the_padding() {
        let c = card(1000.0, 680.0);
        let r = regions(c);
        assert_eq!(r.body.min.x - c.min.x, PAD);
        assert_eq!(c.max.x - r.body.max.x, PAD);
    }

    #[test]
    fn the_body_stays_usable_on_a_short_screen() {
        // `open_settings` never makes the card shorter than 420 points
        let r = regions(card(CARD_SIZE[0], 420.0));
        assert!(r.body.height() >= 280.0, "body: {}", r.body.height());
        // the Menu tab needs room for the profile row plus a pie of at least 200 points
        assert!(r.body.width() >= 800.0);
    }

    #[test]
    fn the_bands_follow_the_card_when_it_moves() {
        let a = regions(card(900.0, 600.0));
        let moved = regions(card(900.0, 600.0).translate(Vec2::new(0.0, 13.0)));
        assert_eq!(moved.top.min.y - a.top.min.y, 13.0);
        assert_eq!(moved.body.height(), a.body.height());
    }
}

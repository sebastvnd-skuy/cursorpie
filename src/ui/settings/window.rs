//! The settings window itself: open/close animation, window chrome, and its layout:
//!
//! ```text
//! +----------------------------------------------------------------------+
//! | logo CURSORPIE ⓘ                             UNDO REDO CANCEL SAVE   |  top bar
//! +--------------+-------------------------------------------------------+
//! | Menu         |                                                       |
//! | Behavior     |   page of the selected section                        |
//! | General      |                                                       |
//! | About        |                                                       |
//! |        v0.3  |                                                       |
//! +--------------+-------------------------------------------------------+
//! | status: warning, message or hint                          QUIT APP   |  footer
//! +----------------------------------------------------------------------+
//! ```
//!
//! The sidebar names the sections (title on the left, page on the right, like the settings of a
//! large application). The top bar holds everything you confirm with, so those buttons never move.
//! The four areas come from [`regions`], a pure function with its own tests.

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
/// Width of the sidebar that lists the sections.
const SIDE_W: f32 = 200.0;
/// Widest a page of panels grows. Wider than this, sliders and text lines become too long to
/// follow, so the page stays at the left of the content area, as the settings of Windows do.
const PAGE_W: f32 = 680.0;
/// Space between the bands and the body, and the body's padding.
const PAD: f32 = GAP * 2.0;
/// Width kept free at the right end of the footer for the Quit cell, so a message never runs
/// under it.
const QUIT_W: f32 = 110.0;

/// The four areas of the window, as rectangles in screen points.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Regions {
    top: egui::Rect,
    side: egui::Rect,
    body: egui::Rect,
    foot: egui::Rect,
}

/// Cuts the `card` rectangle into the top bar, the sidebar, the body and the footer. The bar and
/// the footer span the full width (they touch the window's outline); the sidebar fills the space
/// between them on the left; the body is the rest, inset by [`PAD`].
fn regions(card: egui::Rect) -> Regions {
    let top = egui::Rect::from_min_size(card.min, Vec2::new(card.width(), TOP_H));
    let foot = egui::Rect::from_min_max(egui::pos2(card.min.x, card.max.y - FOOT_H), card.max);
    let side = egui::Rect::from_min_max(
        egui::pos2(card.min.x, top.max.y),
        egui::pos2(card.min.x + SIDE_W, foot.min.y),
    );
    let body = egui::Rect::from_min_max(
        egui::pos2(side.max.x + PAD, top.max.y + PAD),
        egui::pos2(card.max.x - PAD, foot.min.y - PAD),
    );
    Regions {
        top,
        side,
        body,
        foot,
    }
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
        let Regions {
            top,
            side,
            body,
            foot,
        } = regions(card_rect);
        let painter = ui.painter().clone();
        painter.add(window_shadow().as_shape(card_rect, CARD_RADIUS));
        painter.rect_filled(card_rect, CARD_RADIUS, BG);
        painter.rect_filled(top, TOP_RADIUS, CARD);
        painter.rect_filled(foot, BOTTOM_RADIUS, INPUT);
        painter.rect_filled(side, 0.0, CARD);
        let hairline = |y: f32| {
            painter.line_segment(
                [
                    egui::pos2(card_rect.min.x, y),
                    egui::pos2(card_rect.max.x, y),
                ],
                egui::Stroke::new(1.0, BORDER),
            );
        };
        hairline(top.max.y);
        hairline(foot.min.y);
        painter.line_segment(
            [side.right_top(), side.right_bottom()],
            egui::Stroke::new(1.0, BORDER),
        );
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
                [
                    egui::pos2(x, top.min.y + 14.0),
                    egui::pos2(x, top.max.y - 14.0),
                ],
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
                ui.label(RichText::new("ℹ").color(MUTED)).on_hover_text(
                    "Hold the middle button for your main pie.\nDouble-click it for the pie of the app under the pointer.\nWhile the pie is open, right-click to switch which pie the side buttons follow.",
                );
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

        // ---------------------------------------------------------------- sidebar
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(side.shrink2(Vec2::new(GAP + 4.0, PAD))),
            |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let sections = [
                    (Tab::Menu, "Menu", "Profiles and slices"),
                    (Tab::Behavior, "Behavior", "Trigger, timing, apps"),
                    (Tab::General, "General", "Input, config, backups"),
                    (Tab::About, "About", "Version and credits"),
                ];
                for (tab, title, hint) in sections {
                    if nav_item(ui, title, hint, self.ed.tab == tab, accent).clicked() {
                        self.ed.tab = tab;
                        self.ed.picking_icon = false;
                        if tab == Tab::General {
                            self.ed.backups = backups(); // read once, not on every frame
                        }
                    }
                }
            },
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(side.shrink2(Vec2::new(PAD, PAD)))
                .layout(egui::Layout::bottom_up(egui::Align::Min)),
            |ui| {
                ui.label(
                    RichText::new(format!("v{VERSION}"))
                        .monospace()
                        .size(11.0)
                        .color(MUTED),
                );
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
                    .show(ui, |ui| {
                        ui.set_max_width(PAGE_W.min(ui.available_width()));
                        match self.ed.tab {
                            Tab::Behavior => self.behavior_tab(ui),
                            Tab::General => self.general_tab(ui),
                            Tab::About => about_tab(ui, accent),
                            Tab::Menu => {}
                        }
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
    use crate::ui::app::{CARD_MIN_H, CARD_MIN_W, CARD_SIZE, WIN_MARGIN};

    fn card(w: f32, h: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(WIN_MARGIN, WIN_MARGIN), Vec2::new(w, h))
    }

    #[test]
    fn areas_tile_the_window_without_overlap() {
        let c = card(CARD_SIZE[0], CARD_SIZE[1]);
        let r = regions(c);
        assert_eq!(r.top.min, c.min, "the bar starts at the window's corner");
        assert_eq!(r.top.width(), c.width(), "the bar spans the window");
        assert_eq!(r.foot.max, c.max, "the footer ends at the window's corner");
        assert_eq!(r.foot.width(), c.width());
        assert_eq!(r.top.height(), TOP_H);
        assert_eq!(r.foot.height(), FOOT_H);
        assert_eq!(r.side.min.y, r.top.max.y, "the sidebar hangs from the bar");
        assert_eq!(r.side.max.y, r.foot.min.y, "the sidebar ends at the footer");
        assert_eq!(r.side.min.x, c.min.x);
        assert_eq!(r.side.width(), SIDE_W);
        assert!(
            r.side.max.x + PAD <= r.body.min.x + 1e-3,
            "body clears the sidebar"
        );
        assert!(
            r.top.max.y + PAD <= r.body.min.y + 1e-3,
            "body clears the bar"
        );
        assert!(
            r.body.max.y + PAD <= r.foot.min.y + 1e-3,
            "body clears the footer"
        );
        assert!(c.contains_rect(r.body));
        assert!(c.contains_rect(r.side));
    }

    #[test]
    fn the_body_is_inset_by_the_padding() {
        let c = card(1120.0, 680.0);
        let r = regions(c);
        assert_eq!(r.body.min.x - r.side.max.x, PAD);
        assert_eq!(c.max.x - r.body.max.x, PAD);
        assert_eq!(r.body.min.y - r.top.max.y, PAD);
    }

    #[test]
    fn the_body_stays_usable_on_a_small_display() {
        // `card_size_for` never goes below this card
        let r = regions(card(CARD_MIN_W, CARD_MIN_H));
        assert!(r.body.height() >= 280.0, "body: {}", r.body.height());
        // the Menu tab needs a 320 point pie tile and an editor of at least 360 points beside it
        assert!(
            r.body.width() >= 320.0 + GAP + 360.0,
            "body: {}",
            r.body.width()
        );
    }

    #[test]
    fn the_full_card_gives_the_editor_as_much_room_as_before_the_sidebar() {
        // the editor was 640 points wide in the 1000 point card without a sidebar
        let r = regions(card(CARD_SIZE[0], CARD_SIZE[1]));
        assert!(r.body.width() - 320.0 - GAP >= 540.0);
    }

    #[test]
    fn the_areas_follow_the_card_when_it_moves() {
        let a = regions(card(900.0, 600.0));
        let moved = regions(card(900.0, 600.0).translate(Vec2::new(0.0, 13.0)));
        assert_eq!(moved.top.min.y - a.top.min.y, 13.0);
        assert_eq!(moved.side.min.y - a.side.min.y, 13.0);
        assert_eq!(moved.body.height(), a.body.height());
    }

    /// Draws the whole window for `frames` frames on a headless context.
    fn draw(app: &mut App, ctx: &egui::Context, frames: usize) {
        for _ in 0..frames {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(
                        CARD_SIZE[0] + 2.0 * WIN_MARGIN,
                        CARD_SIZE[1] + 2.0 * WIN_MARGIN,
                    ),
                )),
                ..Default::default()
            };
            // nothing renders the output, so the texture uploads are dropped on purpose
            ctx.run_ui(input, |ui| app.settings_ui(ui))
                .drop_without_applying_deltas();
        }
    }

    #[test]
    fn every_section_draws_without_panicking() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = App::new(rx, tx, 200.0, None);
        app.settings_open = true;
        app.edit = crate::config::Config::default();
        let ctx = egui::Context::default();
        for tab in [Tab::Menu, Tab::Behavior, Tab::General, Tab::About] {
            app.ed.tab = tab;
            draw(&mut app, &ctx, 3);
        }
        assert!(
            app.ed.tab == Tab::About,
            "drawing never changes the section"
        );
    }
}

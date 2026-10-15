//! Window management: showing the pie, opening/closing Settings, and the tray menu actions.

use super::{App, CARD_SIZE, WIN_MARGIN};
use crate::config::{self, MAX_ITEMS, cfg};
use crate::platform;
use crate::side;
use crate::tap::PAUSED;
use crate::ui::screen::screen_rect_at;
use crate::ui::settings::Tab;
use eframe::egui::{self, Pos2, Vec2};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

impl App {
    /// Closes Settings and turns the window back into the hidden, click-through pie window.
    /// With `save`, the edited config is sanitized, stored, saved to disk and the launch-at-login setting is applied.
    pub fn close_settings(&mut self, ctx: &egui::Context, save: bool) {
        self.ed.stop_recording();
        if save {
            let new = self.edit.clone().sanitize();
            let login = new.launch_at_login;
            *cfg() = new.clone();
            side::forget_all(); // a changed default should apply at once
            config::save_config_with_backup(&new);
            if let Err(e) = platform::set_launch_at_login(login) {
                eprintln!("CursorPie: could not update launch-at-login: {e}");
            }
            if let Some(t) = &self.tray {
                t.set_login_checked(login);
            }
        }
        self.ed.icons.revalidate(); // re-read only the images that changed on disk
        self.release_editor(); // the next open copies the config again
        self.win = cfg().pie_size;
        self.settings_open = false;
        self.closing = None;
        self.settings_anim = 0.0;
        self.pie_open = false;
        self.anim = 0.0;
        self.visible = false;

        let w = self.win;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([w, w].into()));
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.sleep_step(ctx); // the idle clock starts now
    }

    /// Turns the window into Settings, centered on the display that contains `at`.
    pub(super) fn open_settings(&mut self, ctx: &egui::Context, at: Pos2, about: bool) {
        if self.settings_open {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            if about {
                self.ed.tab = Tab::About;
            }
            return;
        }
        self.pie_open = false;
        self.settings_open = true;
        self.visible = true;
        self.closing = None;
        self.settings_anim = 0.0;
        self.ed.stop_recording();
        self.ed.icons.revalidate();
        self.ed.profile = 0;
        self.ed.reset_nav();
        self.ed.tab = if about { Tab::About } else { Tab::Menu };
        self.edit = cfg().clone();
        self.edit.launch_at_login = platform::launch_at_login_enabled();
        self.history.reset(&self.edit); // undo starts from what Settings opened with

        // centered on the display that contains the cursor
        let area = screen_rect_at(at)
            .unwrap_or_else(|| egui::Rect::from_center_size(at, egui::vec2(1600.0, 1000.0)));
        let max_h = (area.height() - 2.0 * WIN_MARGIN - 60.0).max(420.0);
        self.card_size = egui::vec2(CARD_SIZE[0], CARD_SIZE[1].min(max_h));
        let win_sz = self.card_size + Vec2::splat(2.0 * WIN_MARGIN);
        let top_left = area.center() - win_sz / 2.0;

        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(win_sz));
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::Normal,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(top_left));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    /// Opens the pie window for `profile` at (x, y). Submenus call this again to re-center.
    pub(super) fn show_pie(
        &mut self,
        ctx: &egui::Context,
        x: f64,
        y: f64,
        profile: usize,
        path: Vec<usize>,
    ) {
        if self.settings_open {
            return;
        }
        self.win = cfg().pie_size;
        self.profile = profile;
        self.path = path;
        self.hover = None;
        self.hover_anim = [0.0; MAX_ITEMS];
        self.pie_open = true;
        self.fresh = true;
        self.anim = 0.0; // submenus "bloom" again, re-centered on the cursor
        self.opened_at = Instant::now();
        self.pie_pos = egui::pos2(x as f32, y as f32);

        let w = self.win;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([w, w].into()));
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
        // whole points: a window on a half-point offset is resampled by the compositor on
        // standard-density screens, which softens everything drawn in it
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
            (x as f32 - w / 2.0).round(),
            (y as f32 - w / 2.0).round(),
        )));
        if !self.visible {
            self.visible = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        }
    }

    /// Menu bar item: switch which pie the side buttons follow in the app in front.
    fn swap_side(&mut self, ctx: &egui::Context) {
        let bundle = platform::frontmost_bundle_id();
        let (switched, general, ms) = {
            let c = cfg();
            (
                side::toggle(&c, bundle.as_deref()),
                c.global_profile(),
                c.preview_ms,
            )
        };
        // an app without a pie of its own has nothing to switch: just show the one pie there is
        let profile = switched.unwrap_or(general);
        if ms == 0 {
            return;
        }
        let at = self.cursor_or_default();
        let center = screen_rect_at(at).map_or(at, |r| r.center());
        self.show_pie(ctx, center.x as f64, center.y as f64, profile, Vec::new());
        self.preview_until = Some(Instant::now() + Duration::from_millis(ms));
        // the ring colors are computed for the app the last gesture was about
        side::set_target(bundle, false);
    }

    /// Current cursor position, or a fixed fallback if the OS cannot tell.
    fn cursor_or_default(&self) -> Pos2 {
        platform::cursor_pos()
            .map(|(x, y)| egui::pos2(x as f32, y as f32))
            .unwrap_or(egui::pos2(400.0, 300.0))
    }

    /// Quits the app. The close request may never be processed while the window is hidden, so a
    /// short-lived thread exits the process as a fallback.
    pub fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(1500));
            std::process::exit(0);
        });
    }

    /// Handles a menu bar item click by its id (see `ui::tray`).
    pub(super) fn handle_tray(&mut self, ctx: &egui::Context, id: &str) {
        match id {
            "settings" => {
                let at = self.cursor_or_default();
                self.open_settings(ctx, at, false);
            }
            "about" => {
                let at = self.cursor_or_default();
                self.open_settings(ctx, at, true);
            }
            "swap_side" => self.swap_side(ctx),
            "pause" => {
                if let Some(t) = &self.tray {
                    PAUSED.store(t.pause_checked(), Ordering::SeqCst);
                }
            }
            "login" => {
                if let Some(t) = &self.tray
                    && let Err(e) = platform::set_launch_at_login(t.login_checked())
                {
                    eprintln!("CursorPie: could not update launch-at-login: {e}");
                }
            }
            "quit" => self.quit(ctx),
            _ => {}
        }
    }
}

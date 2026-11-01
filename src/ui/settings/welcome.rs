//! The welcome window: a three-page introduction shown the first time CursorPie starts (and again
//! from the tray item "Welcome guide...").
//!
//! It is drawn in the same native window and with the same card, shadow, colors and widgets as
//! Settings, so it looks like part of the app and needs no second window. While it is open
//! `App::settings_open` is true (so the pie stays hidden) and `App::welcome` holds its state.
//!
//! ```text
//! +--------------------------------------------------------+
//! | logo CursorPie                                    Skip |  top bar
//! +--------------------------------------------------------+
//! |                                                        |
//! |   page 1  How it works    (hold - flick - release)     |
//! |   page 2  Get set up      (macOS: Accessibility,       |
//! |                            Windows: the system tray)   |
//! |   page 3  All set         (launch at login, tips)      |
//! |                                                        |
//! +--------------------------------------------------------+
//! | o o o                                  Back    Next    |  footer
//! +--------------------------------------------------------+
//! ```
//!
//! The texts that differ between macOS and Windows are small pure functions of [`Os`], so a test
//! can check both without running on both.

use super::theme::*;
use super::widgets::*;
use crate::about::*;
use crate::config::cfg;
use crate::platform::{self, Os};
use crate::tap::TAP_ACTIVE;
use crate::ui::app::App;
use crate::ui::draw::*;
use crate::ui::fonts;
use crate::ui::symbols;
use eframe::egui::{self, Vec2};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Number of pages of the welcome window.
pub const PAGES: usize = 3;

/// Height of the top bar.
const TOP_H: f32 = 68.0;
/// Height of the footer (page dots and buttons).
const FOOT_H: f32 = 76.0;
/// Space between the bands and the page, and the page's padding.
const PAD: f32 = GAP * 3.0;

/// State of the open welcome window.
pub struct Welcome {
    /// Index of the page on screen, `0..PAGES`.
    pub page: usize,
    /// The "Launch at login" box of the last page.
    pub launch_at_login: bool,
    /// Set when the window was left with "Get started" or "Open Settings" (not with Skip or Esc):
    /// only then is the launch-at-login box applied.
    pub apply: bool,
    /// Open Settings as soon as the window has closed.
    pub open_settings_after: bool,
}

impl Welcome {
    /// A new welcome window on its first page; `launch_at_login` is the current system setting.
    pub fn new(launch_at_login: bool) -> Self {
        Self {
            page: 0,
            launch_at_login,
            apply: false,
            open_settings_after: false,
        }
    }
}

/// What a button of the window asked for this frame.
enum Step {
    None,
    Next,
    Back,
    Skip,
    Finish { open_settings: bool },
}

/// Heading of the second page.
pub fn setup_title(os: Os) -> &'static str {
    match os {
        Os::Mac => "Allow CursorPie to watch the mouse",
        Os::Windows => "CursorPie lives in the system tray",
        Os::Linux => "CursorPie works in your X11 session",
    }
}

/// Explanation under the heading of the second page.
pub fn setup_lead(os: Os) -> &'static str {
    match os {
        Os::Mac => {
            "macOS only lets an app watch the mouse after you allow it. Open System Settings, go to \
             Privacy & Security > Accessibility and switch CursorPie on (macOS may also ask for Input \
             Monitoring). If CursorPie is not in the list, press + and add it from Applications. \
             This page notices the change by itself; there is nothing to restart."
        }
        Os::Windows => {
            "CursorPie has no window of its own: look for its pie icon next to the clock (click the ^ \
             arrow if you do not see it). Right-click it for Settings, Pause and Quit. Windows asks \
             for no permission. Programs that run as administrator only react if CursorPie runs as \
             administrator too."
        }
        Os::Linux => {
            "X11 asks for no permission. CursorPie has no tray icon on Linux yet: hold the trigger in \
             the center of the pie to open Settings, and quit from the bottom of Settings. In a \
             Wayland session it only sees X11 and XWayland windows, so log in with an X11 session for \
             full support."
        }
    }
}

/// The three steps of the gesture, as `(label, text)`. `hold_ms` is the configured hold time.
pub fn gesture_steps(hold_ms: u64) -> [(&'static str, String); 3] {
    [
        (
            "Hold",
            format!(
                "Press and hold the middle mouse button for {}.",
                seconds(hold_ms)
            ),
        ),
        (
            "Flick",
            "The pie opens under the pointer. Move toward the slice you want.".into(),
        ),
        (
            "Release",
            "Let go over a slice and CursorPie runs it: a shortcut, an app, a link.".into(),
        ),
    ]
}

/// `ms` as seconds with one decimal: "0.5 s".
fn seconds(ms: u64) -> String {
    format!("{:.1} s", ms as f64 / 1000.0)
}

/// Short tips of the last page.
pub fn tips(os: Os, settings_hold_ms: u64) -> Vec<String> {
    let last = if os.has_tray() {
        format!(
            "Pause, Settings and Quit are in the {} icon. \"Welcome guide...\" there shows this window again.",
            os.tray_name()
        )
    } else {
        "Pause and Quit are at the bottom of Settings. There is no tray icon on this system yet."
            .to_string()
    };
    vec![
        format!(
            "Hold the trigger in the center of the pie for {} to open Settings.",
            seconds(settings_hold_ms)
        ),
        "Double-click the trigger over an app that has its own profile to open that app's pie."
            .into(),
        last,
    ]
}

impl App {
    /// Draws the welcome window: animation, top bar, current page, footer.
    pub fn welcome_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();

        // open / close animation (the same as Settings)
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        let target = if self.closing.is_some() { 0.0 } else { 1.0 };
        if smooth_approach(&mut self.settings_anim, target, dt, 14.0) {
            ctx.request_repaint();
        }
        if self.closing.is_some() && self.settings_anim <= 0.02 {
            self.finish_welcome(&ctx);
            return;
        }
        // keep the permission status fresh while the window is open
        ctx.request_repaint_after(Duration::from_millis(1000));

        if self.closing.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.closing = Some(false);
        }
        let Some(page) = self.welcome.as_ref().map(|w| w.page) else {
            return;
        };

        let t = self.settings_anim.clamp(0.0, 1.0);
        let slide = (1.0 - t) * (1.0 - t) * 26.0;
        let accent = rgb(cfg().accent);
        style_settings(ui, accent);
        ui.set_opacity(t);

        let full = ui.max_rect();
        let card_rect = snap_rect(
            egui::Rect::from_center_size(full.center(), self.card_size)
                .translate(Vec2::new(0.0, slide)),
            ctx.pixels_per_point(),
        );
        let top = egui::Rect::from_min_size(card_rect.min, Vec2::new(card_rect.width(), TOP_H));
        let foot = egui::Rect::from_min_max(
            egui::pos2(card_rect.min.x, card_rect.max.y - FOOT_H),
            card_rect.max,
        );
        let body = egui::Rect::from_min_max(
            egui::pos2(card_rect.min.x + PAD, top.max.y + GAP),
            egui::pos2(card_rect.max.x - PAD, foot.min.y - GAP / 2.0),
        );

        let painter = ui.painter().clone();
        painter.add(window_shadow().as_shape(card_rect, CARD_RADIUS));
        painter.rect_filled(card_rect, CARD_RADIUS, pal().bg);
        window_outline(&painter, card_rect, accent);

        // dragging the empty part of the top bar moves the window
        let drag = ui.interact(
            top,
            egui::Id::new("welcome_drag"),
            egui::Sense::click_and_drag(),
        );
        if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let mut step = Step::None;

        // ---------------------------------------------------------------- top bar
        draw_logo(
            &painter,
            egui::pos2(top.min.x + PAD + 14.0, top.center().y),
            14.0,
            accent,
        );
        painter.text(
            egui::pos2(top.min.x + PAD + 40.0, top.center().y),
            egui::Align2::LEFT_CENTER,
            APP_NAME,
            egui::FontId::new(19.0, fonts::semibold()),
            pal().text,
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(top.shrink2(Vec2::new(PAD, 0.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                if page + 1 < PAGES
                    && button(ui, Look::Plain, None, "Skip")
                        .on_hover_text("Close this guide. You can reopen it from the tray menu.")
                        .clicked()
                {
                    step = Step::Skip;
                }
            },
        );

        // ---------------------------------------------------------------- page
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(body));
            match page {
                0 => intro_page(ui, accent),
                1 => setup_page(ui),
                _ => {
                    if let Some(w) = self.welcome.as_mut()
                        && ready_page(ui, w, accent)
                    {
                        step = Step::Finish {
                            open_settings: true,
                        };
                    }
                }
            }
        });

        // ---------------------------------------------------------------- footer
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(foot.shrink2(Vec2::new(PAD, 0.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                // page dots
                ui.spacing_mut().item_spacing.x = 4.0;
                for i in 0..PAGES {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                    let (r, color) = if i == page {
                        (5.0, accent)
                    } else {
                        (3.5, pal().border)
                    };
                    ui.painter().circle_filled(rect.center(), r, color);
                }
            },
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(foot.shrink2(Vec2::new(PAD, 0.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if page + 1 < PAGES {
                    if button(
                        ui,
                        Look::Primary(accent),
                        Some(symbols::CARET_RIGHT),
                        "Next",
                    )
                    .clicked()
                    {
                        step = Step::Next;
                    }
                } else if button(
                    ui,
                    Look::Primary(accent),
                    Some(symbols::CHECK),
                    "Get started",
                )
                .clicked()
                {
                    step = Step::Finish {
                        open_settings: false,
                    };
                }
                if page > 0 && button(ui, Look::Soft, None, "Back").clicked() {
                    step = Step::Back;
                }
            },
        );

        // Enter goes forward, like the primary button
        if matches!(step, Step::None)
            && self.closing.is_none()
            && ctx.input(|i| i.key_pressed(egui::Key::Enter))
        {
            step = if page + 1 < PAGES {
                Step::Next
            } else {
                Step::Finish {
                    open_settings: false,
                }
            };
        }

        if let Some(w) = self.welcome.as_mut() {
            match step {
                Step::None => {}
                Step::Next => w.page = (w.page + 1).min(PAGES - 1),
                Step::Back => w.page = w.page.saturating_sub(1),
                Step::Skip => self.closing = Some(false),
                Step::Finish { open_settings } => {
                    w.apply = true;
                    w.open_settings_after = open_settings;
                    self.closing = Some(false);
                }
            }
        }
    }
}

/// A big heading with a line of secondary text under it.
fn heading(ui: &mut egui::Ui, title: &str) {
    ui.label(fonts::title(title, 28.0, pal().text));
    ui.add_space(GAP);
}

/// Page 1: what CursorPie is and the three-step gesture.
fn intro_page(ui: &mut egui::Ui, accent: egui::Color32) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(96.0), egui::Sense::hover());
        draw_logo(ui.painter(), rect.center(), 46.0, accent);
        ui.add_space(GAP);
        ui.vertical(|ui| {
            ui.add_space(14.0);
            ui.label(fonts::title(
                format!("Welcome to {APP_NAME}"),
                28.0,
                pal().text,
            ));
            muted(ui, TAGLINE);
            ui.add_space(GAP / 2.0);
            badge(ui, &format!("Version {VERSION}"), accent);
        });
    });
    ui.add_space(GAP * 2.0);
    muted(
        ui,
        "Three moves open anything. Try them once you finish this guide:",
    );
    ui.add_space(GAP);
    let steps = gesture_steps(cfg().pie_hold_ms);
    let icons = [
        symbols::CURSOR_CLICK,
        symbols::ARROWS_OUT,
        symbols::CHECK_CIRCLE,
    ];
    ui.columns(3, |cols| {
        for (col, (icon, (label, text))) in cols.iter_mut().zip(icons.into_iter().zip(steps)) {
            // columns lay out justified, which would stretch the words of the text
            let left = egui::Layout::top_down(egui::Align::Min);
            card().show(col, |ui| {
                ui.with_layout(left, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(190.0);
                    ui.add_space(GAP);
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(rect.center(), 22.0, wash(accent, 0.12));
                    fonts::paint_icon(ui.painter(), rect.center(), icon, 24.0, accent, true);
                    ui.add_space(GAP);
                    ui.label(fonts::title(label, 17.0, pal().text));
                    ui.add_space(4.0);
                    muted(ui, &text);
                    ui.add_space(GAP);
                })
            });
        }
    });
}

/// Page 2: the permission (macOS) or the tray (Windows), with the live status.
fn setup_page(ui: &mut egui::Ui) {
    let os = platform::OS;
    heading(ui, setup_title(os));
    ui.set_max_width(ui.available_width().min(640.0));
    muted(ui, setup_lead(os));
    ui.add_space(PAD);

    let running = TAP_ACTIVE.load(Ordering::SeqCst);
    let ready = if os.needs_permission() {
        platform::is_trusted_cached() && running
    } else {
        running
    };
    panel(ui, "Status", |ui| {
        if os.needs_permission() {
            let granted = platform::is_trusted_cached();
            info_row(ui, "Accessibility", |ui| {
                status(
                    ui,
                    granted,
                    if granted {
                        "Granted"
                    } else {
                        "Not granted yet"
                    },
                );
            });
        }
        let name = if os.needs_permission() {
            "Event tap"
        } else {
            "Mouse hook"
        };
        info_row(ui, name, |ui| {
            status(
                ui,
                running,
                if running {
                    "Running"
                } else {
                    "Not running (retrying every 2 s)"
                },
            );
        });
    });
    if os.needs_permission() && !ready {
        ui.horizontal(|ui| {
            if button(
                ui,
                Look::Soft,
                Some(symbols::ARROW_SQUARE_OUT),
                "Accessibility settings",
            )
            .clicked()
            {
                platform::open_accessibility_settings();
            }
            if button(
                ui,
                Look::Soft,
                Some(symbols::ARROW_SQUARE_OUT),
                "Input Monitoring settings",
            )
            .clicked()
            {
                platform::open_input_monitoring_settings();
            }
        });
    } else if ready {
        ui.horizontal(|ui| {
            status(ui, true, "Ready");
            muted(ui, "CursorPie can see your mouse. Continue when you like.");
        });
    }
}

/// Page 3: tips and the launch-at-login box. Returns true when "Open Settings" was clicked.
fn ready_page(ui: &mut egui::Ui, w: &mut Welcome, accent: egui::Color32) -> bool {
    let os = platform::OS;
    heading(ui, "You're all set");
    ui.set_max_width(ui.available_width().min(640.0));
    let hold = cfg().settings_hold_ms;
    panel(ui, "Good to know", |ui| {
        for tip in tips(os, hold) {
            note(ui, &tip);
        }
    });
    panel(ui, "Startup", |ui| {
        toggle_row(
            ui,
            &mut w.launch_at_login,
            "Launch CursorPie at login",
            os.startup_hint(),
            accent,
        );
    });
    button(
        ui,
        Look::Soft,
        Some(symbols::SLIDERS_HORIZONTAL),
        "Open Settings now",
    )
    .clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_systems_have_their_own_setup_page() {
        assert_ne!(setup_title(Os::Mac), setup_title(Os::Windows));
        assert!(setup_lead(Os::Mac).contains("Accessibility"));
        assert!(setup_title(Os::Windows).contains("system tray"));
        assert!(setup_lead(Os::Windows).contains("administrator"));
        assert!(!setup_lead(Os::Windows).contains("Accessibility"));
    }

    #[test]
    fn the_gesture_uses_the_configured_hold_time() {
        let steps = gesture_steps(500);
        assert!(steps[0].1.contains("0.5 s"), "{}", steps[0].1);
        assert!(gesture_steps(1200)[0].1.contains("1.2 s"));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn tips_name_the_tray_of_the_system() {
        assert!(tips(Os::Mac, 1200).iter().any(|t| t.contains("menu bar")));
        assert!(
            tips(Os::Windows, 1200)
                .iter()
                .any(|t| t.contains("system tray"))
        );
        assert!(tips(Os::Mac, 1200)[0].contains("1.2 s"));
    }

    #[test]
    fn linux_has_its_own_setup_page_and_no_tray_tip() {
        assert!(setup_title(Os::Linux).contains("X11"));
        let lead = setup_lead(Os::Linux);
        assert!(lead.contains("no tray icon"), "{lead}");
        assert!(!lead.contains("Accessibility"));
        let t = tips(Os::Linux, 1200);
        assert_eq!(t.len(), 3);
        assert!(t.iter().all(|x| !x.contains("icon.")), "{t:?}");
        assert!(t[2].contains("bottom of Settings"));
    }

    /// Draws every page for a few frames in a headless egui context: a smoke test for layout
    /// panics (zero widths, bad column counts) that no other test would reach.
    #[test]
    fn every_page_draws_without_panicking() {
        let ctx = egui::Context::default();
        crate::ui::fonts::install(&ctx);
        let accent = egui::Color32::from_rgb(64, 156, 255);
        let mut w = Welcome::new(false);
        for page in 0..PAGES {
            for _ in 0..3 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(760.0, 520.0),
                    )),
                    ..Default::default()
                };
                let mut output = ctx.run_ui(input, |ui| match page {
                    0 => intro_page(ui, accent),
                    1 => setup_page(ui),
                    _ => {
                        ready_page(ui, &mut w, accent);
                    }
                });
                output.textures_delta.clear(); // nothing uploads the textures here
            }
        }
    }

    #[test]
    fn a_new_welcome_starts_on_the_first_page() {
        let w = Welcome::new(true);
        assert_eq!((w.page, w.launch_at_login, w.apply), (0, true, false));
    }
}

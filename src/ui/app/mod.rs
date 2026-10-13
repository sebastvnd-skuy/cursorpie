//! The eframe application: owns the UI state and reacts to messages from the other threads.
//!
//! | Submodule | Contents |
//! |---|---|
//! | `messages` | `App::handle_msg`: applies each [`Msg`] |
//! | `windows` | showing the pie, opening/closing Settings, quitting, menu bar actions |
//! | `lifecycle` | idle sleep, freeing the Settings copy, warming up icons before the pie opens |
//!
//! The same native window shows the pie and Settings (see
//! [`guides::architecture`](crate::guides::architecture#one-window-for-everything)). `App::logic`
//! runs every frame even while the window is hidden and must not draw; `App::ui` draws either the
//! pie ([`pie_ui`](crate::ui::pie)) or Settings.

mod lifecycle;
mod messages;
mod windows;

use crate::config::{Config, MAX_ITEMS, cfg};
use crate::history::History;
use crate::msg::Msg;
use crate::ui::settings::Ed;
use crate::ui::sleep::Sleeper;
use crate::ui::tray::TrayHandles;
use eframe::egui::{self, Pos2, Vec2};
use std::sync::mpsc;
use std::time::Instant;

/// Settings card size. The window is larger by WIN_MARGIN on each side for the drop shadow.
pub const CARD_SIZE: [f32; 2] = [960.0, 640.0];
/// Space around the card for its drop shadow (window size = card size + 2 x margin).
pub const WIN_MARGIN: f32 = 28.0;

/// Most undo steps Settings keeps. A step is a copy of the edited config (a few KiB).
pub const HISTORY_LIMIT: usize = 50;

/// All UI-thread state. Fields are grouped: the message channel, the pie, Settings.
pub struct App {
    pub rx: mpsc::Receiver<Msg>,
    pub tx: mpsc::Sender<Msg>,

    // pie
    pub hover: Option<usize>,
    pub profile: usize,
    pub path: Vec<usize>,
    pub pie_open: bool,
    pub visible: bool,
    pub fresh: bool,
    pub anim: f32,
    pub hover_anim: [f32; MAX_ITEMS],
    pub active_anim: [f32; MAX_ITEMS],
    pub win: f32,
    pub pie_pos: Pos2,
    pub opened_at: Instant,
    pub last_tick: Instant,
    /// Some while a preview (menu bar "Switch side-button pie") is on screen: when it ends
    pub preview_until: Option<Instant>,

    // settings
    pub settings_open: bool,
    pub quitting: bool,
    pub edit: Config,
    /// Undo / redo for `edit`, see [`crate::history`]
    pub history: History<Config>,
    pub ed: Ed,
    pub settings_anim: f32,
    /// Some(true) = closing and saving, Some(false) = closing without saving
    pub closing: Option<bool>,
    pub card_size: Vec2,
    pub toast: Option<(String, Instant)>,

    /// Idle clock that decides when caches and the Settings copy are freed
    pub sleeper: Sleeper,

    pub tray: Option<TrayHandles>,
}

impl App {
    /// Creates the app. `win` is the pie window size, `tray` the menu bar handles (if the icon could be created).
    pub fn new(
        rx: mpsc::Receiver<Msg>,
        tx: mpsc::Sender<Msg>,
        win: f32,
        tray: Option<TrayHandles>,
    ) -> Self {
        let edit = cfg().clone();
        Self {
            rx,
            tx,
            hover: None,
            profile: 0,
            path: Vec::new(),
            pie_open: false,
            visible: false,
            fresh: false,
            anim: 0.0,
            hover_anim: [0.0; MAX_ITEMS],
            active_anim: [0.0; MAX_ITEMS],
            win,
            pie_pos: egui::pos2(200.0, 200.0),
            opened_at: Instant::now(),
            last_tick: Instant::now(),
            preview_until: None,
            settings_open: false,
            quitting: false,
            history: History::new(&edit, HISTORY_LIMIT),
            edit,
            ed: Ed::new(),
            settings_anim: 0.0,
            closing: None,
            card_size: egui::vec2(CARD_SIZE[0], CARD_SIZE[1]),
            toast: None,
            sleeper: Sleeper::new(Instant::now()),
            tray,
        }
    }

    /// Shows a short message (toast) at the bottom of Settings; it disappears after a few seconds.
    pub fn say(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), Instant::now()));
    }
}

impl eframe::App for App {
    // (documented by the trait)
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // the window is always transparent; the settings card paints itself
    }

    /// Runs even while the window is hidden (as long as something calls request_repaint).
    /// Must not draw any UI.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // evicts icons nobody used lately (never one the last frame used)
        self.ed.icons.new_frame();
        while let Ok(msg) = self.rx.try_recv() {
            self.handle_msg(ctx, msg);
        }

        // end of a preview (see Msg::Preview)
        if let Some(until) = self.preview_until {
            let now = Instant::now();
            if now >= until {
                self.preview_until = None;
                self.hover = None;
                self.pie_open = false;
                ctx.request_repaint();
            } else {
                ctx.request_repaint_after(until - now);
            }
        }

        // Cmd+W / the close button on the settings window = cancel (with animation), not quit
        if self.settings_open
            && !self.quitting
            && self.closing.is_none()
            && ctx.input(|i| i.viewport().close_requested())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.closing = Some(false);
        }

        self.sleep_step(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.settings_open {
            self.settings_ui(ui);
        } else {
            self.pie_ui(ui);
        }
        // the pie may just have hidden the window: start the idle clock
        self.sleep_step(&ui.ctx().clone());
    }
}

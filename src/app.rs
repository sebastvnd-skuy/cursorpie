//! The eframe application: owns UI state and reacts to messages from the other threads.

use crate::config::{
    self, Action, Config, ItemKind, MAX_ITEMS, cfg, has_commands, item_at_mut, parse_config,
};
use crate::gfx::screen_rect_at;
use crate::msg::Msg;
use crate::settings::{Ed, Slot, Tab};
use crate::system;
use crate::tap::PAUSED;
use crate::tray::TrayHandles;
use eframe::egui::{self, Pos2, Vec2};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Settings card size. The window is larger by WIN_MARGIN on each side for the drop shadow.
pub const CARD_SIZE: [f32; 2] = [880.0, 620.0];
pub const WIN_MARGIN: f32 = 28.0;

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

    // settings
    pub settings_open: bool,
    pub quitting: bool,
    pub edit: Config,
    pub ed: Ed,
    pub settings_anim: f32,
    /// Some(true) = closing and saving, Some(false) = closing without saving
    pub closing: Option<bool>,
    pub card_size: Vec2,
    pub toast: Option<(String, Instant)>,

    pub tray: Option<TrayHandles>,
}

impl App {
    pub fn new(
        rx: mpsc::Receiver<Msg>,
        tx: mpsc::Sender<Msg>,
        win: f32,
        tray: Option<TrayHandles>,
    ) -> Self {
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
            settings_open: false,
            quitting: false,
            edit: cfg().clone(),
            ed: Ed::new(),
            settings_anim: 0.0,
            closing: None,
            card_size: egui::vec2(CARD_SIZE[0], CARD_SIZE[1]),
            toast: None,
            tray,
        }
    }

    pub fn say(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), Instant::now()));
    }

    pub fn close_settings(&mut self, ctx: &egui::Context, save: bool) {
        self.ed.stop_recording();
        if save {
            let new = self.edit.clone().sanitize();
            let login = new.launch_at_login;
            *cfg() = new.clone();
            config::save_config(&new);
            if let Err(e) = system::set_launch_at_login(login) {
                eprintln!("CursorPie: could not update launch-at-login: {e}");
            }
            if let Some(t) = &self.tray {
                t.login.set_checked(login);
            }
        }
        self.ed.icons.clear(); // re-read images that changed on disk
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
    }

    fn open_settings(&mut self, ctx: &egui::Context, at: Pos2, about: bool) {
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
        self.ed.icons.clear();
        self.ed.profile = 0;
        self.ed.reset_nav();
        self.ed.tab = if about { Tab::About } else { Tab::Menu };
        self.edit = cfg().clone();
        self.edit.launch_at_login = system::launch_at_login_enabled();

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

    fn cursor_or_default(&self) -> Pos2 {
        system::cursor_pos()
            .map(|(x, y)| egui::pos2(x as f32, y as f32))
            .unwrap_or(egui::pos2(400.0, 300.0))
    }

    fn handle_tray(&mut self, ctx: &egui::Context, id: &str) {
        match id {
            "settings" => {
                let at = self.cursor_or_default();
                self.open_settings(ctx, at, false);
            }
            "about" => {
                let at = self.cursor_or_default();
                self.open_settings(ctx, at, true);
            }
            "pause" => {
                if let Some(t) = &self.tray {
                    PAUSED.store(t.pause.is_checked(), Ordering::SeqCst);
                }
            }
            "login" => {
                if let Some(t) = &self.tray {
                    if let Err(e) = system::set_launch_at_login(t.login.is_checked()) {
                        eprintln!("CursorPie: could not update launch-at-login: {e}");
                    }
                }
            }
            "quit" => {
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                // fallback in case the window is hidden and the close request is never processed
                std::thread::spawn(|| {
                    std::thread::sleep(Duration::from_millis(1500));
                    std::process::exit(0);
                });
            }
            _ => {}
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // the window is always transparent; the settings card paints itself
    }

    /// Runs even while the window is hidden (as long as something calls request_repaint).
    /// Must not draw any UI.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::ShowPie {
                    x,
                    y,
                    profile,
                    path,
                } => {
                    if self.settings_open {
                        continue;
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
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                        x as f32 - w / 2.0,
                        y as f32 - w / 2.0,
                    )));
                    if !self.visible {
                        self.visible = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    }
                }
                Msg::OpenSettings { x, y, about } => {
                    self.open_settings(ctx, egui::pos2(x as f32, y as f32), about);
                }
                Msg::Hover(h) => {
                    if h.is_some()
                        && h != self.hover
                        && self.last_tick.elapsed() > Duration::from_millis(60)
                        && cfg().sound
                    {
                        system::tick();
                        self.last_tick = Instant::now();
                    }
                    self.hover = h;
                }
                Msg::Refocus => ctx.send_viewport_cmd(egui::ViewportCommand::Focus),
                Msg::HidePie => {
                    self.hover = None;
                    self.pie_open = false;
                }
                Msg::KeyCaptured(sc) => {
                    if let Some(rec) = self.ed.recording.take() {
                        if let Some(p) = self.edit.profiles.get_mut(rec.profile) {
                            if let Some(item) = item_at_mut(&mut p.items, &rec.path) {
                                match (&mut item.kind, rec.slot) {
                                    (ItemKind::Mode { back, .. }, Slot::Back) => {
                                        *back = Action::Key(sc)
                                    }
                                    (ItemKind::Mode { forward, .. }, Slot::Forward) => {
                                        *forward = Action::Key(sc)
                                    }
                                    (ItemKind::Run(a), Slot::Run) => *a = Action::Key(sc),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Msg::RecordCancelled => self.ed.recording = None,
                Msg::IconPicked {
                    profile,
                    path,
                    file,
                } => {
                    if let Some(p) = self.edit.profiles.get_mut(profile) {
                        if let Some(item) = item_at_mut(&mut p.items, &path) {
                            item.icon = file;
                        }
                    }
                }
                Msg::AppPicked { profile, bundle } => {
                    if let Some(p) = self.edit.profiles.get_mut(profile) {
                        if !p.apps.contains(&bundle) {
                            p.apps.push(bundle);
                        }
                    }
                }
                Msg::ImportPicked(file) => match std::fs::read_to_string(&file)
                    .ok()
                    .and_then(|t| parse_config(&t))
                {
                    Some(cfg_in) => {
                        let warn = has_commands(&cfg_in);
                        self.edit = cfg_in;
                        self.ed.profile = 0;
                        self.ed.reset_nav();
                        self.ed.stop_recording();
                        self.ed.icons.clear();
                        self.say(if warn {
                            "Imported. This config runs shell commands - review them before you Save."
                        } else {
                            "Imported. Review it, then press Save to apply."
                        });
                    }
                    None => self.say("That file is not a valid CursorPie config."),
                },
                Msg::Tray(id) => self.handle_tray(ctx, &id),
                Msg::ActionAppPicked {
                    profile,
                    path,
                    slot,
                    app,
                } => {
                    if let Some(p) = self.edit.profiles.get_mut(profile) {
                        if let Some(item) = item_at_mut(&mut p.items, &path) {
                            match (&mut item.kind, slot) {
                                (ItemKind::Run(a), Slot::Run) => *a = Action::OpenApp(app),
                                (ItemKind::Mode { back, .. }, Slot::Back) => {
                                    *back = Action::OpenApp(app)
                                }
                                (ItemKind::Mode { forward, .. }, Slot::Forward) => {
                                    *forward = Action::OpenApp(app)
                                }
                                _ => {}
                            }
                        }
                    }
                }
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
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.settings_open {
            self.settings_ui(ui);
        } else {
            self.pie_ui(ui);
        }
    }
}

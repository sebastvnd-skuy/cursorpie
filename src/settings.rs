//! The settings card: Menu (profiles + items), Behavior, General and About tabs.

use crate::about::*;
use crate::app::App;
use crate::config::*;
use crate::gfx::*;
use crate::keys::{key, Shortcut};
use crate::msg::Msg;
use crate::system;
use crate::tap::{RECORDING, SWALLOW_KEY, TAP_ACTIVE};
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Menu,
    Behavior,
    General,
    About,
}

/// Which action field of an item is being recorded.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Back,
    Forward,
    Run,
}

pub struct Rec {
    pub profile: usize,
    pub path: Vec<usize>,
    pub slot: Slot,
}

/// Editor-only state (kept apart from the edited config so both can be borrowed at once).
pub struct Ed {
    pub tab: Tab,
    pub profile: usize,
    pub recording: Option<Rec>,
    pub icons: HashMap<String, Option<egui::TextureHandle>>,
    pub new_app: String,
}

impl Ed {
    pub fn new() -> Self {
        Self { tab: Tab::Menu, profile: 0, recording: None, icons: HashMap::new(), new_app: String::new() }
    }

    pub fn stop_recording(&mut self) {
        RECORDING.store(false, Ordering::SeqCst);
        self.recording = None;
    }

    /// Cached texture for an image icon. None for text icons or images that fail to load.
    pub fn icon_for(&mut self, ctx: &egui::Context, icon: &str) -> Option<egui::TextureHandle> {
        let path = icon_path(icon)?;
        let key = path.to_string_lossy().to_string();
        if let Some(t) = self.icons.get(&key) {
            return t.clone();
        }
        let tex = load_icon(ctx, &path, &key);
        self.icons.insert(key, tex.clone());
        tex
    }
}

enum UiAction {
    None,
    Save,
    Cancel,
    Quit,
}

/// Read-only things the recursive editor needs.
struct Env<'a> {
    ctx: &'a egui::Context,
    tx: &'a Sender<Msg>,
    accent: Color32,
    profile: usize,
}

// ------------------------------------------------------------ action editing

const ACTION_KINDS: [&str; 6] = ["None", "Shortcut", "Open app", "Open URL / file", "Shell command", "Sequence"];

fn action_index(a: &Action) -> usize {
    match a {
        Action::Nothing => 0,
        Action::Key(_) => 1,
        Action::OpenApp(_) => 2,
        Action::OpenUrl(_) => 3,
        Action::Command(_) => 4,
        Action::Sequence(_) => 5,
    }
}

fn make_action(i: usize) -> Action {
    match i {
        1 => Action::Key(Shortcut::new(key::F13, 0)),
        2 => Action::OpenApp(String::new()),
        3 => Action::OpenUrl(String::new()),
        4 => Action::Command(String::new()),
        5 => Action::Sequence(Vec::new()),
        _ => Action::Nothing,
    }
}

/// Type selector + value editor for one action. Returns true when the shortcut button was clicked.
fn action_editor(ui: &mut egui::Ui, id: &str, a: &mut Action, recording: bool, accent: Color32) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let now = action_index(a);
        let mut sel = now;
        egui::ComboBox::from_id_salt(format!("{id}_kind"))
            .width(120.0)
            .selected_text(ACTION_KINDS[now])
            .show_ui(ui, |ui| {
                for (i, name) in ACTION_KINDS.iter().enumerate() {
                    ui.selectable_value(&mut sel, i, *name);
                }
            });
        if sel != now {
            *a = make_action(sel);
        }
        match a {
            Action::Key(sc) => {
                if shortcut_button(ui, sc, recording, accent).clicked() {
                    clicked = true;
                }
            }
            Action::OpenApp(s) => {
                ui.add(egui::TextEdit::singleline(s).desired_width(200.0).hint_text("Safari  or  /Applications/X.app"));
            }
            Action::OpenUrl(s) => {
                ui.add(egui::TextEdit::singleline(s).desired_width(200.0).hint_text("https://...  or a file path"));
            }
            Action::Command(s) => {
                ui.add(egui::TextEdit::singleline(s).desired_width(200.0).hint_text("shell command"));
            }
            Action::Sequence(v) => {
                muted(ui, &format!("{} steps (edit config.json to change them)", v.len()));
            }
            Action::Nothing => muted(ui, "click passes through to the app"),
        }
    });
    clicked
}

// -------------------------------------------------------------- item editing

const KIND_NAMES: [&str; 3] = ["Side-button mode", "Action", "Submenu"];

fn default_kind(k: usize) -> ItemKind {
    match k {
        0 => presets().remove(0).kind,
        1 => ItemKind::Run(Action::Key(Shortcut::new(key::F13, 0))),
        _ => ItemKind::Submenu(Vec::new()),
    }
}

fn is_rec(ed: &Ed, profile: usize, path: &[usize], slot: Slot) -> bool {
    ed.recording
        .as_ref()
        .is_some_and(|r| r.profile == profile && r.path == path && r.slot == slot)
}

fn start_rec(ed: &mut Ed, profile: usize, path: &[usize], slot: Slot) {
    if is_rec(ed, profile, path, slot) {
        ed.stop_recording();
    } else {
        SWALLOW_KEY.store(-1, Ordering::SeqCst);
        RECORDING.store(true, Ordering::SeqCst);
        ed.recording = Some(Rec { profile, path: path.to_vec(), slot });
    }
}

fn pick_icon(env: &Env, path: &[usize]) {
    let (tx, ctx, profile, path) = (env.tx.clone(), env.ctx.clone(), env.profile, path.to_vec());
    std::thread::spawn(move || {
        if let Some(file) = system::choose_file("public.image", "Choose an icon image") {
            let _ = tx.send(Msg::IconPicked { profile, path, file });
            ctx.request_repaint();
        }
    });
}

/// Edits one ring of items; submenus recurse.
fn items_editor(ui: &mut egui::Ui, ed: &mut Ed, env: &Env, items: &mut Vec<Item>, path: &mut Vec<usize>) {
    let depth = path.len();
    let n = items.len();
    let mut remove: Option<usize> = None;
    let mut swap: Option<(usize, usize)> = None;

    for i in 0..n {
        path.push(i);
        let uid = format!(
            "{}_{}",
            env.profile,
            path.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("-")
        );
        let rec_back = is_rec(ed, env.profile, path, Slot::Back);
        let rec_fwd = is_rec(ed, env.profile, path, Slot::Forward);
        let rec_run = is_rec(ed, env.profile, path, Slot::Run);
        let mut start: Option<Slot> = None;
        let mut pick = false;

        ui.push_id(uid.clone(), |ui| {
            card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                let item = &mut items[i];

                // row 1: number, icon preview, name, kind, preset
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}", i + 1)).color(env.accent).strong());
                    let tex = ed.icon_for(env.ctx, &item.icon);
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(14, 15, 18));
                    draw_icon(ui.painter(), tex.as_ref(), &item.icon, rect.center(), 22.0, 255);

                    ui.add(egui::TextEdit::singleline(&mut item.name).desired_width(120.0));

                    let now = match &item.kind {
                        ItemKind::Mode { .. } => 0,
                        ItemKind::Run(_) => 1,
                        ItemKind::Submenu(_) => 2,
                    };
                    let mut sel = now;
                    egui::ComboBox::from_id_salt(format!("{uid}_type"))
                        .width(115.0)
                        .selected_text(KIND_NAMES[now])
                        .show_ui(ui, |ui| {
                            for (k, name) in KIND_NAMES.iter().enumerate() {
                                if k == 2 && depth + 1 >= MAX_DEPTH {
                                    continue; // no deeper submenus
                                }
                                ui.selectable_value(&mut sel, k, *name);
                            }
                        });
                    if sel != now {
                        item.kind = default_kind(sel);
                    }

                    if now == 0 {
                        egui::ComboBox::from_id_salt(format!("{uid}_preset"))
                            .width(60.0)
                            .selected_text("Preset")
                            .show_ui(ui, |ui| {
                                for p in presets() {
                                    if ui.selectable_label(false, p.name.clone()).clicked() {
                                        *item = p;
                                    }
                                }
                            });
                    }
                });

                // row 2: icon text + browse + order/remove
                ui.horizontal_wrapped(|ui| {
                    muted(ui, "Icon");
                    ui.add(
                        egui::TextEdit::singleline(&mut item.icon)
                            .desired_width(150.0)
                            .hint_text("text, emoji or image path"),
                    );
                    if ui.button("Browse...").clicked() {
                        pick = true;
                    }
                    if ui.add_enabled(i > 0, egui::Button::new("Up")).clicked() {
                        swap = Some((i, i - 1));
                    }
                    if ui.add_enabled(i + 1 < n, egui::Button::new("Down")).clicked() {
                        swap = Some((i, i + 1));
                    }
                    if ui.add_enabled(n > 1 || depth > 0, egui::Button::new("Remove")).clicked() {
                        remove = Some(i);
                    }
                });

                // row 3: what the item does
                match &mut item.kind {
                    ItemKind::Mode { back, forward } => {
                        muted(ui, "Back side button");
                        if action_editor(ui, &format!("{uid}_b"), back, rec_back, env.accent) {
                            start = Some(Slot::Back);
                        }
                        muted(ui, "Forward side button");
                        if action_editor(ui, &format!("{uid}_f"), forward, rec_fwd, env.accent) {
                            start = Some(Slot::Forward);
                        }
                    }
                    ItemKind::Run(a) => {
                        muted(ui, "Runs when you release the button over this slice");
                        if action_editor(ui, &format!("{uid}_r"), a, rec_run, env.accent) {
                            start = Some(Slot::Run);
                        }
                    }
                    ItemKind::Submenu(children) => {
                        muted(ui, "Drag outward across the rim of this slice to open it");
                        egui::CollapsingHeader::new(format!("Items in this submenu ({})", children.len()))
                            .id_salt(format!("{uid}_sub"))
                            .default_open(true)
                            .show(ui, |ui| {
                                items_editor(ui, ed, env, children, path);
                            });
                    }
                }
            });
        });

        if let Some(slot) = start {
            start_rec(ed, env.profile, path, slot);
        }
        if pick {
            pick_icon(env, path);
        }
        path.pop();
        ui.add_space(6.0);
    }

    if let Some((a, b)) = swap {
        items.swap(a, b);
        ed.stop_recording();
    }
    if let Some(i) = remove {
        items.remove(i);
        ed.stop_recording();
    }

    if items.len() < MAX_ITEMS {
        let mut add: Option<Item> = None;
        egui::ComboBox::from_id_salt(format!("add_{}_{}", env.profile, path.len()))
            .selected_text("+ Add item")
            .show_ui(ui, |ui| {
                muted(ui, "Side-button modes");
                for p in presets() {
                    if ui.selectable_label(false, p.name.clone()).clicked() {
                        add = Some(p);
                    }
                }
                ui.separator();
                muted(ui, "Quick actions");
                for q in quick_actions() {
                    if ui.selectable_label(false, q.name.clone()).clicked() {
                        add = Some(q);
                    }
                }
                ui.separator();
                muted(ui, "Custom");
                for b in blank_items(depth + 1 < MAX_DEPTH) {
                    if ui.selectable_label(false, b.name.clone()).clicked() {
                        add = Some(b);
                    }
                }
            });
        if let Some(it) = add {
            items.push(it);
        }
    } else {
        muted(ui, &format!("Maximum {MAX_ITEMS} items per ring reached."));
    }
}

// ---------------------------------------------------------------- the window

impl App {
    fn menu_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let App { edit, ed, tx, .. } = self;
        ed.profile = ed.profile.min(edit.profiles.len().saturating_sub(1));
        let pi = ed.profile;
        let accent = rgb(edit.accent);

        section_title(ui, "Profile");
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let names: Vec<String> = edit.profiles.iter().map(|p| p.name.clone()).collect();
                egui::ComboBox::from_id_salt("profile_select")
                    .width(170.0)
                    .selected_text(names[pi].clone())
                    .show_ui(ui, |ui| {
                        for (i, name) in names.iter().enumerate() {
                            ui.selectable_value(&mut ed.profile, i, name.as_str());
                        }
                    });
                if ui.add_enabled(edit.profiles.len() < MAX_PROFILES, egui::Button::new("New")).clicked() {
                    edit.profiles.push(Profile {
                        name: "New profile".into(),
                        apps: vec![],
                        items: vec![presets().remove(0)],
                        active: vec![0],
                    });
                    ed.profile = edit.profiles.len() - 1;
                    ed.stop_recording();
                }
                if ui.add_enabled(edit.profiles.len() > 1, egui::Button::new("Delete")).clicked() {
                    edit.profiles.remove(pi);
                    ed.profile = pi.saturating_sub(1);
                    ed.stop_recording();
                }
            });

            let Some(prof) = edit.profiles.get_mut(ed.profile) else { return };
            ui.horizontal(|ui| {
                muted(ui, "Name");
                ui.add(egui::TextEdit::singleline(&mut prof.name).desired_width(220.0));
            });

            muted(
                ui,
                if prof.apps.is_empty() {
                    "No apps listed: used for every app that has no profile of its own."
                } else {
                    "Used while one of these apps is in front:"
                },
            );
            let mut drop_app: Option<usize> = None;
            for (i, app) in prof.apps.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(app).monospace());
                    if ui.small_button("Remove").clicked() {
                        drop_app = Some(i);
                    }
                });
            }
            if let Some(i) = drop_app {
                prof.apps.remove(i);
            }
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut ed.new_app)
                        .desired_width(220.0)
                        .hint_text("bundle id, e.g. com.apple.Safari"),
                );
                if ui.button("Add").clicked() {
                    let id = ed.new_app.trim().to_string();
                    if !id.is_empty() && !prof.apps.contains(&id) {
                        prof.apps.push(id);
                    }
                    ed.new_app.clear();
                }
                if ui.button("Pick app...").clicked() {
                    let (tx, ctx, profile) = (tx.clone(), ctx.clone(), ed.profile);
                    std::thread::spawn(move || {
                        let Some(file) = system::choose_file("com.apple.application-bundle", "Choose an application")
                        else {
                            return;
                        };
                        if let Some(bundle) = system::bundle_id_of(&file) {
                            let _ = tx.send(Msg::AppPicked { profile, bundle });
                            ctx.request_repaint();
                        }
                    });
                }
            });
        });

        ui.add_space(14.0);
        section_title(ui, "Menu items");
        muted(ui, "Hold the trigger button, drag to a slice and release. Side buttons use the active mode.");
        ui.add_space(4.0);

        let env = Env { ctx, tx, accent, profile: ed.profile };
        if let Some(prof) = edit.profiles.get_mut(ed.profile) {
            let mut path = Vec::new();
            items_editor(ui, ed, &env, &mut prof.items, &mut path);
        }
    }

    fn behavior_tab(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Behavior & look");
        let cfg = &mut self.edit;
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::Grid::new("behavior_grid").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
                muted(ui, "Trigger button");
                let name = |b: i64| match b {
                    2 => "Middle button",
                    5 => "Extra button 6",
                    6 => "Extra button 7",
                    _ => "Extra button 8",
                };
                egui::ComboBox::from_id_salt("trigger_button")
                    .selected_text(name(cfg.trigger_button))
                    .show_ui(ui, |ui| {
                        for b in [2i64, 5, 6, 7] {
                            ui.selectable_value(&mut cfg.trigger_button, b, name(b));
                        }
                    });
                ui.end_row();

                muted(ui, "Hold to open pie");
                ui.add(egui::Slider::new(&mut cfg.pie_hold_ms, 150..=1500).suffix(" ms"));
                ui.end_row();

                muted(ui, "Hold to open settings");
                ui.add(egui::Slider::new(&mut cfg.settings_hold_ms, 500..=4000).suffix(" ms"));
                ui.end_row();

                muted(ui, "Pie size");
                ui.add(egui::Slider::new(&mut cfg.pie_size, 140.0..=360.0).suffix(" px"));
                ui.end_row();

                muted(ui, "Accent color");
                ui.color_edit_button_srgb(&mut cfg.accent);
                ui.end_row();
            });
            ui.add_space(4.0);
            ui.checkbox(&mut cfg.flick, "Open the pie instantly when I drag (disables middle-drag in apps)");
            ui.checkbox(&mut cfg.sound, "Play a soft tick when the selection changes");
            ui.checkbox(&mut cfg.launch_at_login, "Launch at login");
        });
    }

    fn general_tab(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Permissions");
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let trusted = system::is_trusted();
            let running = TAP_ACTIVE.load(Ordering::SeqCst);
            ui.horizontal(|ui| {
                muted(ui, "Accessibility");
                if trusted {
                    ui.label(RichText::new("Granted").color(GOOD));
                } else {
                    ui.label(RichText::new("Missing").color(BAD));
                }
            });
            ui.horizontal(|ui| {
                muted(ui, "Event tap");
                if running {
                    ui.label(RichText::new("Running").color(GOOD));
                } else {
                    ui.label(RichText::new("Not running (retrying every 2 s)").color(BAD));
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Open Accessibility settings").clicked() {
                    system::open_accessibility_settings();
                }
                if ui.button("Open Input Monitoring settings").clicked() {
                    system::open_input_monitoring_settings();
                }
            });
            muted(ui, "If it still says 'Not running' after granting access, quit and reopen CursorPie.");
        });

        ui.add_space(14.0);
        section_title(ui, "Configuration");
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let path = config_path();
            ui.label(RichText::new(path.to_string_lossy()).monospace().color(MUTED));
            ui.horizontal_wrapped(|ui| {
                if ui.button("Reveal in Finder").clicked() {
                    system::reveal_in_finder(&path);
                }
                if ui.button("Export...").clicked() {
                    match export_config(&self.edit) {
                        Ok(p) => self.say(format!("Exported to {}", p.display())),
                        Err(e) => self.say(format!("Export failed: {e}")),
                    }
                }
                if ui.button("Import...").clicked() {
                    let (tx, ctx) = (self.tx.clone(), ui.ctx().clone());
                    std::thread::spawn(move || {
                        if let Some(f) = system::choose_file("public.json", "Choose a CursorPie config") {
                            let _ = tx.send(Msg::ImportPicked(f));
                            ctx.request_repaint();
                        }
                    });
                }
                if ui.button("Reset to defaults").clicked() {
                    let login = self.edit.launch_at_login;
                    self.edit = Config::default();
                    self.edit.launch_at_login = login;
                    self.ed.profile = 0;
                    self.ed.stop_recording();
                    self.say("Defaults restored. Press Save to apply.");
                }
            });
            muted(ui, "Export saves your whole setup to ~/Downloads so you can share it. Only import configs you trust: actions can run shell commands.");
        });
    }

    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();

        // open / close animation
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        let target = if self.closing.is_some() { 0.0 } else { 1.0 };
        if smooth_approach(&mut self.settings_anim, target, dt, 14.0) {
            ctx.request_repaint();
        }
        if let Some(save) = self.closing {
            if self.settings_anim <= 0.02 {
                self.close_settings(&ctx, save);
                return;
            }
        }
        // keep permission status fresh while the window is open
        ctx.request_repaint_after(Duration::from_millis(1000));

        let t = self.settings_anim.clamp(0.0, 1.0);
        let slide = (1.0 - t) * (1.0 - t) * 26.0;

        // Esc (when not recording) = cancel
        if self.closing.is_none() && self.ed.recording.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.closing = Some(false);
        }
        if self.toast.as_ref().is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(6)) {
            self.toast = None;
        }

        let accent = rgb(self.edit.accent);
        style_settings(ui, accent);
        ui.set_opacity(t);

        // card + shadow, always centered in the window
        let full = ui.max_rect();
        let card_rect = egui::Rect::from_center_size(full.center(), self.card_size).translate(Vec2::new(0.0, slide));
        for k in 1..=6 {
            let g = k as f32 * 3.0;
            let alpha = (24.0 - k as f32 * 3.6).max(2.0) as u8;
            ui.painter().rect_filled(
                card_rect.expand(g).translate(Vec2::new(0.0, 6.0)),
                CornerRadius::same((16.0 + g) as u8),
                Color32::from_black_alpha(alpha),
            );
        }
        ui.painter().rect_filled(card_rect, CornerRadius::same(16), BG);
        ui.painter().rect_stroke(card_rect, CornerRadius::same(16), Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

        let inner = card_rect.shrink(18.0);

        // dragging the title area moves the window
        let title_rect = egui::Rect::from_min_size(inner.min, Vec2::new(inner.width(), 56.0));
        let drag = ui.interact(title_rect, egui::Id::new("settings_drag"), egui::Sense::click_and_drag());
        if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let mut action = UiAction::None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
            ui.label(RichText::new(APP_NAME).size(26.0).strong().color(TEXT));
            muted(ui, "Hold the trigger button for the pie menu. Keep holding at the center to open settings.");

            if !system::is_trusted() || !TAP_ACTIVE.load(Ordering::SeqCst) {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Accessibility permission needed.").color(BAD));
                    if ui.small_button("Open System Settings").clicked() {
                        system::open_accessibility_settings();
                    }
                });
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for (t, name) in [(Tab::Menu, "Menu"), (Tab::Behavior, "Behavior"), (Tab::General, "General"), (Tab::About, "About")] {
                    ui.selectable_value(&mut self.ed.tab, t, name);
                }
            });
            ui.separator();

            let h = (ui.available_height() - 66.0).max(120.0);
            egui::ScrollArea::vertical().max_height(h).auto_shrink([false, false]).show(ui, |ui| match self.ed.tab {
                Tab::Menu => self.menu_tab(ui, &ctx),
                Tab::Behavior => self.behavior_tab(ui),
                Tab::General => self.general_tab(ui),
                Tab::About => about_tab(ui),
            });

            if let Some((msg, _)) = &self.toast {
                ui.label(RichText::new(msg).color(accent));
            }
            ui.separator();
            ui.horizontal(|ui| {
                let save = egui::Button::new(RichText::new("Save").strong().color(Color32::WHITE))
                    .fill(accent)
                    .min_size(Vec2::new(100.0, 32.0));
                if ui.add(save).clicked() {
                    action = UiAction::Save;
                }
                if ui.add(egui::Button::new("Cancel").min_size(Vec2::new(80.0, 32.0))).clicked() {
                    action = UiAction::Cancel;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Quit app").clicked() {
                        action = UiAction::Quit;
                    }
                });
            });
        });

        match action {
            UiAction::Save => {
                self.ed.stop_recording();
                self.closing = Some(true);
            }
            UiAction::Cancel => {
                self.ed.stop_recording();
                self.closing = Some(false);
            }
            UiAction::Quit => {
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                std::thread::spawn(|| {
                    std::thread::sleep(Duration::from_millis(1500));
                    std::process::exit(0);
                });
            }
            UiAction::None => {}
        }
    }
}

// ----------------------------------------------------------------- About page

fn link_row(ui: &mut egui::Ui, label: &str, text: &str, url: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(MUTED));
        ui.hyperlink_to(text, url);
    });
}

fn about_tab(ui: &mut egui::Ui) {
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(format!("{APP_NAME}  v{VERSION}")).size(20.0).strong().color(TEXT));
        muted(ui, TAGLINE);
    });
    ui.add_space(10.0);

    section_title(ui, "Created by");
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(AUTHOR_NAME).size(16.0).strong());
        link_row(ui, "GitHub", &format!("@{AUTHOR_GITHUB_USER}"), AUTHOR_GITHUB_URL);
        link_row(ui, "Email", AUTHOR_EMAIL, &format!("mailto:{AUTHOR_EMAIL}"));
    });
    ui.add_space(10.0);

    section_title(ui, "Project");
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        link_row(ui, "Source code", "GitHub repository", REPO_URL);
        link_row(ui, "Bugs / ideas", "Open an issue", ISSUES_URL);
        link_row(ui, "Downloads", "Releases", RELEASES_URL);
        link_row(ui, "Inspired by", INSPIRED_BY.0, INSPIRED_BY.1);
        ui.add_space(2.0);
        muted(ui, LICENSE_NAME);
        muted(ui, COPYRIGHT);
    });
    ui.add_space(10.0);

    section_title(ui, "Privacy");
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(PRIVACY).color(MUTED));
    });
    ui.add_space(10.0);

    section_title(ui, "Built with");
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        for (name, url, license) in CREDITS {
            ui.horizontal(|ui| {
                ui.hyperlink_to(*name, *url);
                muted(ui, license);
            });
        }
    });
}

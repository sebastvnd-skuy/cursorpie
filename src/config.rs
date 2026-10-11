//! Layout of a config:
//!   Config -> profiles[] -> items[] -> (Mode | Run | Submenu(items[]))
//! A profile with an empty `apps` list is the fallback for every app that has no profile of its own.

use crate::keys::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const CONFIG_VERSION: u32 = 2;
/// Max slices per ring.
pub const MAX_ITEMS: usize = 8;
/// Max menu levels (top level counts as 1).
pub const MAX_DEPTH: usize = 5;
pub const MAX_PROFILES: usize = 16;

// ---------------------------------------------------------------------- model

/// Something CursorPie can do.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
pub enum Action {
    /// Do nothing (for side buttons: let the click pass through to the app).
    #[default]
    Nothing,
    Key(Shortcut),
    /// App name ("Safari") or path ("/Applications/Safari.app"), passed to `open -a`.
    OpenApp(String),
    /// URL or file path, passed to `open`.
    OpenUrl(String),
    /// Shell command, run with `/bin/sh -c`.
    Command(String),
    Sequence(Vec<Action>),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ItemKind {
    /// Selecting it makes the side buttons (back/forward) send these actions.
    Mode { back: Action, forward: Action },
    /// Runs the action when you release the button over the slice.
    Run(Action),
    /// Drag outward across the rim to open it.
    Submenu(Vec<Item>),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Item {
    pub name: String,
    /// Text / emoji, or an image path (.png .jpg .jpeg .webp), e.g. "~/Pictures/undo.png"
    pub icon: String,
    pub kind: ItemKind,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub enum AppGesture {
    /// Cara lama: tahan = pie milik app di depan (kalau ada).
    Merged,
    /// Baru: tahan = pie umum, klik dua kali = pie milik app.
    #[default]
    DoubleClick,
}

/// Which pie the side buttons follow in an app that has a pie of its own, until you choose another.
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub enum SideTarget {
    /// The app's own pie.
    #[default]
    App,
    /// The general pie, even inside an app that has its own.
    General,
}

/// How CursorPie decides which app a gesture is about.
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub enum TargetApp {
    /// The app in front (the one with keyboard focus).
    Focused,
    /// The app whose window is under the pointer, even if it is not in front.
    #[default]
    UnderCursor,
}

impl Item {
    pub fn mode(name: &str, icon: &str, back: Shortcut, forward: Shortcut) -> Self {
        Self {
            name: name.into(),
            icon: icon.into(),
            kind: ItemKind::Mode {
                back: Action::Key(back),
                forward: Action::Key(forward),
            },
        }
    }
    pub fn run(name: &str, icon: &str, action: Action) -> Self {
        Self {
            name: name.into(),
            icon: icon.into(),
            kind: ItemKind::Run(action),
        }
    }
    pub fn submenu(name: &str, icon: &str, items: Vec<Item>) -> Self {
        Self {
            name: name.into(),
            icon: icon.into(),
            kind: ItemKind::Submenu(items),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Profile {
    pub name: String,
    /// Bundle identifiers, e.g. "com.apple.Safari". Empty = fallback profile.
    pub apps: Vec<String>,
    pub items: Vec<Item>,
    /// Path (indices) of the item whose side-button actions are currently active.
    pub active: Vec<usize>,
}

impl Profile {
    pub fn active_item(&self) -> Option<&Item> {
        item_at(&self.items, &self.active)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub profiles: Vec<Profile>,
    pub pie_hold_ms: u64,
    pub settings_hold_ms: u64,
    pub pie_size: f32,
    pub accent: [u8; 3],
    /// Mouse button that opens the pie: 2 = middle, 5/6/7 = extra buttons.
    pub trigger_button: i64,
    /// Dragging right after pressing opens the pie instantly (disables middle-drag pass-through).
    pub flick: bool,
    pub sound: bool,
    pub launch_at_login: bool,
    pub app_gesture: AppGesture,
    pub double_click_ms: u64,
    /// Which app picks the pie (and gets the shortcuts): the one in front or the one under the pointer.
    pub target_app: TargetApp,
    /// With `UnderCursor`: bring that app to the front before sending it a shortcut.
    pub focus_target: bool,
    /// Which pie the side buttons follow in an app that has its own pie, until you choose another.
    pub side_default: SideTarget,
    /// Choosing a side-button mode in a pie makes the side buttons follow that pie.
    pub side_adopt: bool,
    /// Forget that choice when another app becomes the target.
    pub side_forget: bool,
    /// Right-click while the pie is open to switch which pie the side buttons follow.
    pub right_click_switch: bool,
    /// Ring color tells which pie the side buttons follow: accent = this one, `inactive_color` = the other.
    pub side_indicator: bool,
    pub inactive_color: [u8; 3],
    /// How long a pie preview stays after switching from the menu bar (0 = no preview).
    pub preview_ms: u64,
    /// The center of an app's own pie shows that app's icon (as a white silhouette) instead of the
    /// icon of the active side-button mode.
    pub center_app_icon: bool,
}

// -------------------------------------------------------------- tree helpers

pub fn items_at<'a>(items: &'a [Item], path: &[usize]) -> Option<&'a [Item]> {
    let mut cur = items;
    for &i in path {
        match &cur.get(i)?.kind {
            ItemKind::Submenu(children) => cur = children.as_slice(),
            _ => return None,
        }
    }
    Some(cur)
}

pub fn item_at<'a>(items: &'a [Item], path: &[usize]) -> Option<&'a Item> {
    let (last, parents) = path.split_last()?;
    items_at(items, parents)?.get(*last)
}

pub fn item_at_mut<'a>(items: &'a mut Vec<Item>, path: &[usize]) -> Option<&'a mut Item> {
    let (first, rest) = path.split_first()?;
    let item = items.get_mut(*first)?;
    if rest.is_empty() {
        return Some(item);
    }
    match &mut item.kind {
        ItemKind::Submenu(children) => item_at_mut(children, rest),
        _ => None,
    }
}

fn first_mode(items: &[Item]) -> Option<Vec<usize>> {
    for (i, it) in items.iter().enumerate() {
        match &it.kind {
            ItemKind::Mode { .. } => return Some(vec![i]),
            ItemKind::Submenu(c) => {
                if let Some(mut p) = first_mode(c) {
                    p.insert(0, i);
                    return Some(p);
                }
            }
            _ => {}
        }
    }
    None
}

fn clean_items(items: &mut Vec<Item>, depth: usize) {
    items.truncate(MAX_ITEMS);
    for it in items.iter_mut() {
        if let ItemKind::Submenu(c) = &mut it.kind {
            if depth + 1 >= MAX_DEPTH {
                c.clear();
            } else {
                clean_items(c, depth + 1);
            }
        }
    }
}

/// true if any action in the tree runs a shell command (used to warn after importing a config).
pub fn has_commands(cfg: &Config) -> bool {
    fn act(a: &Action) -> bool {
        match a {
            Action::Command(_) => true,
            Action::Sequence(v) => v.iter().any(act),
            _ => false,
        }
    }
    fn items(v: &[Item]) -> bool {
        v.iter().any(|i| match &i.kind {
            ItemKind::Mode { back, forward } => act(back) || act(forward),
            ItemKind::Run(a) => act(a),
            ItemKind::Submenu(c) => items(c),
        })
    }
    cfg.profiles.iter().any(|p| items(&p.items))
}

// ------------------------------------------------------------------- presets

/// Side-button mode presets (the original eight).
pub fn presets() -> Vec<Item> {
    vec![
        Item::mode(
            "History",
            "<>",
            Shortcut::ch('z', MOD_CMD),
            Shortcut::ch('z', MOD_CMD | MOD_SHIFT),
        ),
        Item::mode(
            "Brackets",
            "[ ]",
            Shortcut::ch('[', 0),
            Shortcut::ch(']', 0),
        ),
        Item::mode(
            "Clipboard",
            "C/V",
            Shortcut::ch('c', MOD_CMD),
            Shortcut::ch('v', MOD_CMD),
        ),
        Item::mode(
            "Browser",
            "< >",
            Shortcut::ch('[', MOD_CMD),
            Shortcut::ch(']', MOD_CMD),
        ),
        Item::mode(
            "Tabs",
            "Tab",
            Shortcut::ch('[', MOD_CMD | MOD_SHIFT),
            Shortcut::ch(']', MOD_CMD | MOD_SHIFT),
        ),
        Item::mode(
            "Zoom",
            "+/-",
            Shortcut::ch('-', MOD_CMD),
            Shortcut::ch('=', MOD_CMD),
        ),
        Item::mode(
            "Scroll",
            "Pg",
            Shortcut::new(key::PAGE_UP, 0),
            Shortcut::new(key::PAGE_DOWN, 0),
        ),
        Item::mode(
            "Edit",
            "Ret",
            Shortcut::new(key::RETURN, 0),
            Shortcut::new(key::DELETE, 0),
        ),
    ]
}

/// Ready-made "run on release" slices.
pub fn quick_actions() -> Vec<Item> {
    vec![
        Item::run(
            "Screenshot",
            "Shot",
            Action::Key(Shortcut::ch('4', MOD_CMD | MOD_SHIFT)),
        ),
        Item::run(
            "Mission Ctrl",
            "MC",
            Action::Key(Shortcut::new(key::UP, MOD_CTRL)),
        ),
        Item::run(
            "Spotlight",
            "Find",
            Action::Key(Shortcut::new(key::SPACE, MOD_CMD)),
        ),
        Item::run(
            "Emoji",
            ":)",
            Action::Key(Shortcut::new(key::SPACE, MOD_CMD | MOD_CTRL)),
        ),
        Item::run(
            "Lock",
            "Lock",
            Action::Key(Shortcut::ch('q', MOD_CMD | MOD_CTRL)),
        ),
    ]
}

/// Blank slices for the "+ Add item" menu.
pub fn blank_items(allow_submenu: bool) -> Vec<Item> {
    let mut v = vec![
        Item::run("Shortcut", "Key", Action::Key(Shortcut::new(key::F13, 0))),
        Item::run("Open app", "App", Action::OpenApp(String::new())),
        Item::run("Open URL", "URL", Action::OpenUrl(String::new())),
        Item::run("Command", ">_", Action::Command(String::new())),
    ];
    if allow_submenu {
        v.push(Item::submenu("Submenu", "...", Vec::new()));
    }
    v
}

impl Default for Config {
    fn default() -> Self {
        let p = presets();
        let launch = Item::submenu(
            "Launch",
            "Go",
            vec![
                Item::run("Finder", "F", Action::OpenApp("Finder".into())),
                Item::run("Terminal", ">_", Action::OpenApp("Terminal".into())),
                Item::run("Safari", "S", Action::OpenApp("Safari".into())),
                Item::run(
                    "Shot",
                    "Shot",
                    Action::Key(Shortcut::ch('4', MOD_CMD | MOD_SHIFT)),
                ),
            ],
        );
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        Self {
            version: CONFIG_VERSION,
            profiles: vec![
                Profile {
                    name: "Default".into(),
                    apps: vec![],
                    items: vec![p[0].clone(), p[2].clone(), p[1].clone(), launch],
                    active: vec![0],
                },
                Profile {
                    name: "Browsers".into(),
                    apps: s(&[
                        "com.apple.Safari",
                        "com.google.Chrome",
                        "org.mozilla.firefox",
                        "company.thebrowser.Browser",
                        "com.brave.Browser",
                        "com.microsoft.edgemac",
                    ]),
                    items: vec![p[3].clone(), p[4].clone(), p[5].clone(), p[6].clone()],
                    active: vec![0],
                },
                Profile {
                    name: "Code".into(),
                    apps: s(&["com.microsoft.VSCode", "com.apple.dt.Xcode"]),
                    items: vec![p[0].clone(), p[1].clone(), p[7].clone()],
                    active: vec![0],
                },
            ],
            pie_hold_ms: 500,
            settings_hold_ms: 1200,
            pie_size: 200.0,
            accent: [64, 156, 255],
            trigger_button: 2,
            flick: false,
            sound: false,
            launch_at_login: false,
            app_gesture: AppGesture::DoubleClick,
            double_click_ms: 260,
            target_app: TargetApp::UnderCursor,
            focus_target: true,
            side_default: SideTarget::App,
            side_adopt: true,
            side_forget: false,
            right_click_switch: true,
            side_indicator: true,
            inactive_color: [232, 86, 86],
            preview_ms: 900,
            center_app_icon: true,
        }
    }
}

impl Config {
    pub fn sanitize(mut self) -> Self {
        self.version = CONFIG_VERSION;
        if self.profiles.is_empty() {
            self.profiles = Config::default().profiles;
        }
        self.profiles.truncate(MAX_PROFILES);
        self.double_click_ms = self.double_click_ms.clamp(150, 500);
        for p in &mut self.profiles {
            if p.items.is_empty() {
                p.items = vec![presets().remove(0)];
            }
            clean_items(&mut p.items, 0);
            let ok = matches!(
                item_at(&p.items, &p.active).map(|i| &i.kind),
                Some(ItemKind::Mode { .. })
            );
            if !ok {
                p.active = first_mode(&p.items).unwrap_or_default();
            }
        }
        self.pie_hold_ms = self.pie_hold_ms.clamp(150, 2000);
        self.settings_hold_ms = self.settings_hold_ms.max(self.pie_hold_ms + 300);
        self.pie_size = self.pie_size.clamp(140.0, 360.0);
        self.preview_ms = self.preview_ms.min(3000);
        if !matches!(self.trigger_button, 2 | 5 | 6 | 7) {
            self.trigger_button = 2;
        }
        self
    }

    pub fn global_profile(&self) -> usize {
        self.profiles
            .iter()
            .position(|p| p.apps.is_empty())
            .unwrap_or(0)
    }

    pub fn app_profile_for(&self, bundle: Option<&str>) -> Option<usize> {
        let b = bundle?;
        self.profiles
            .iter()
            .position(|p| p.apps.iter().any(|a| a == b))
    }

    /// Which profile applies for the given frontmost app.
    pub fn profile_for(&self, bundle: Option<&str>) -> usize {
        if let Some(b) = bundle
            && let Some(i) = self
                .profiles
                .iter()
                .position(|p| p.apps.iter().any(|a| a == b))
        {
            return i;
        }
        self.profiles
            .iter()
            .position(|p| p.apps.is_empty())
            .unwrap_or(0)
    }
}

/// Parses a config file. None = not valid JSON or not a CursorPie config.
pub fn parse_config(text: &str) -> Option<Config> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    v.get("version").and_then(|x| x.as_u64())?; // no "version" = old v1 format, rejected
    serde_json::from_value::<Config>(v)
        .ok()
        .map(Config::sanitize)
}

// -------------------------------------------------------------------- storage

pub fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
}

pub fn config_path() -> PathBuf {
    let mut p = home();
    p.push("Library");
    p.push("Application Support");
    p.push("CursorPie");
    fs::create_dir_all(&p).ok();
    p.push("config.json");
    p
}

pub fn load_config() -> Config {
    let path = config_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return Config::default();
    };
    match parse_config(&text) {
        Some(cfg) => cfg,
        None => {
            fs::copy(&path, path.with_file_name("config.broken.json")).ok();
            Config::default()
        }
    }
}

pub fn save_config(config: &Config) {
    if let Ok(data) = serde_json::to_string_pretty(config) {
        fs::write(config_path(), data).ok();
    }
}

/// Writes ~/Downloads/cursorpie-config.json and returns its path.
pub fn export_config(config: &Config) -> std::io::Result<PathBuf> {
    let mut p = home();
    p.push("Downloads");
    fs::create_dir_all(&p)?;
    p.push("cursorpie-config.json");
    let data = serde_json::to_string_pretty(config).map_err(std::io::Error::other)?;
    fs::write(&p, data)?;
    Ok(p)
}

// -------------------------------------------------------------------- globals

static CONFIG: OnceLock<Mutex<Config>> = OnceLock::new();

/// Locks a mutex and recovers from poisoning (the event tap must never panic because of a lock).
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The live config (shared between the UI thread and the event tap thread).
pub fn cfg() -> MutexGuard<'static, Config> {
    lock(CONFIG.get_or_init(|| Mutex::new(load_config())))
}

/// Remembers which mode is active for a profile (saved on a worker thread, never inside the tap callback).
pub fn set_active(profile: usize, path: Vec<usize>) {
    let snapshot = {
        let mut c = cfg();
        match c.profiles.get_mut(profile) {
            Some(p) => {
                p.active = path;
                Some(c.clone())
            }
            None => None,
        }
    };
    if let Some(c) = snapshot {
        std::thread::spawn(move || save_config(&c));
    }
}

/// Cincin (daftar item) pada path tertentu; path kosong = level atas.
pub fn ring_mut<'a>(items: &'a mut Vec<Item>, path: &[usize]) -> Option<&'a mut Vec<Item>> {
    if path.is_empty() {
        return Some(items);
    }
    match &mut item_at_mut(items, path)?.kind {
        ItemKind::Submenu(c) => Some(c),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_without_the_new_fields_still_loads() {
        // what a v0.2.0 file looks like: none of the side-button / app-detection settings
        let c = parse_config(r#"{"version": 2, "pie_hold_ms": 400}"#).expect("old config parses");
        assert_eq!(c.pie_hold_ms, 400);
        assert_eq!(c.target_app, TargetApp::UnderCursor);
        assert_eq!(c.side_default, SideTarget::App);
        assert!(c.focus_target && c.side_adopt && c.right_click_switch && c.side_indicator);
        assert!(c.center_app_icon);
        assert!(!c.side_forget);
    }

    #[test]
    fn new_fields_round_trip_and_preview_is_clamped() {
        let c = parse_config(
            r#"{"version": 2, "target_app": "Focused", "side_default": "General", "preview_ms": 999999}"#,
        )
        .unwrap();
        assert_eq!(c.target_app, TargetApp::Focused);
        assert_eq!(c.side_default, SideTarget::General);
        assert_eq!(c.preview_ms, 3000);
    }
}

//! Built-in slices (presets for the "Add a slice" panel) and the default [`Config`].

use super::{Action, AppGesture, CONFIG_VERSION, Config, Item, Profile, SideTarget, TargetApp};
use crate::keys::{MOD_CMD, MOD_CTRL, MOD_SHIFT, Shortcut, key};

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
    /// The configuration of a fresh install: three profiles (general, browsers, code) and the default options.
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

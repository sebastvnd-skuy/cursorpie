//! Built-in slices (presets for the "Add a slice" panel) and the default [`Config`].
//!
//! Defaults depend on the OS: macOS uses Cmd shortcuts, bundle ids and Finder/Safari; Windows uses
//! Ctrl/Alt/Win shortcuts, executable names and Explorer/Edge. Every function takes the OS as a
//! parameter in its `_for` form (so tests cover both); the plain form uses the OS the build runs on.

use super::{Action, AppGesture, CONFIG_VERSION, Config, Item, Profile, SideTarget, TargetApp};
use crate::keys::{MOD_ALT, MOD_CMD, MOD_CTRL, MOD_SHIFT, Shortcut, key};
use crate::platform::Os;

/// Side-button mode presets (the original eight) for this OS.
pub fn presets() -> Vec<Item> {
    presets_for(Os::current())
}

/// Side-button mode presets for `os`. "Primary" is Cmd on macOS and Ctrl on Windows.
pub fn presets_for(os: Os) -> Vec<Item> {
    let p = os.primary_mod();
    // Windows has no Cmd+[ / Cmd+Shift+[ conventions: browsers go back and forward with Alt+arrows
    // and switch tabs with Ctrl(+Shift)+Tab; redo is Ctrl+Y.
    let (redo, browser, tabs) = match os {
        Os::Mac => (
            Shortcut::ch('z', p | MOD_SHIFT),
            (Shortcut::ch('[', p), Shortcut::ch(']', p)),
            (
                Shortcut::ch('[', p | MOD_SHIFT),
                Shortcut::ch(']', p | MOD_SHIFT),
            ),
        ),
        Os::Windows => (
            Shortcut::ch('y', p),
            (
                Shortcut::new(key::LEFT, MOD_ALT),
                Shortcut::new(key::RIGHT, MOD_ALT),
            ),
            (
                Shortcut::new(key::TAB, MOD_CTRL | MOD_SHIFT),
                Shortcut::new(key::TAB, MOD_CTRL),
            ),
        ),
    };
    vec![
        Item::mode("History", "<>", Shortcut::ch('z', p), redo),
        Item::mode(
            "Brackets",
            "[ ]",
            Shortcut::ch('[', 0),
            Shortcut::ch(']', 0),
        ),
        Item::mode(
            "Clipboard",
            "C/V",
            Shortcut::ch('c', p),
            Shortcut::ch('v', p),
        ),
        Item::mode("Browser", "< >", browser.0, browser.1),
        Item::mode("Tabs", "Tab", tabs.0, tabs.1),
        Item::mode("Zoom", "+/-", Shortcut::ch('-', p), Shortcut::ch('=', p)),
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

/// Ready-made "run on release" slices for this OS.
pub fn quick_actions() -> Vec<Item> {
    quick_actions_for(Os::current())
}

/// Ready-made "run on release" slices for `os`.
pub fn quick_actions_for(os: Os) -> Vec<Item> {
    let key_item = |name: &str, icon: &str, sc: Shortcut| Item::run(name, icon, Action::Key(sc));
    match os {
        Os::Mac => vec![
            key_item("Screenshot", "Shot", Shortcut::ch('4', MOD_CMD | MOD_SHIFT)),
            key_item("Mission Ctrl", "MC", Shortcut::new(key::UP, MOD_CTRL)),
            key_item("Spotlight", "Find", Shortcut::new(key::SPACE, MOD_CMD)),
            key_item("Emoji", ":)", Shortcut::new(key::SPACE, MOD_CMD | MOD_CTRL)),
            key_item("Lock", "Lock", Shortcut::ch('q', MOD_CMD | MOD_CTRL)),
        ],
        // MOD_CMD is the Windows key here
        Os::Windows => vec![
            key_item("Screenshot", "Shot", Shortcut::ch('s', MOD_CMD | MOD_SHIFT)),
            key_item("Task view", "TV", Shortcut::new(key::TAB, MOD_CMD)),
            key_item("Search", "Find", Shortcut::ch('s', MOD_CMD)),
            key_item("Emoji", ":)", Shortcut::ch('.', MOD_CMD)),
            key_item("Lock", "Lock", Shortcut::ch('l', MOD_CMD)),
        ],
    }
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
    /// The configuration of a fresh install on this OS.
    fn default() -> Self {
        Config::default_for(Os::current())
    }
}

impl Config {
    /// The configuration of a fresh install on `os`: three profiles (general, browsers, code) and the
    /// default options.
    pub fn default_for(os: Os) -> Self {
        let p = presets_for(os);
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let (launchers, browsers, code, shot) = match os {
            Os::Mac => (
                [
                    ("Finder", "F", "Finder"),
                    ("Terminal", ">_", "Terminal"),
                    ("Safari", "S", "Safari"),
                ],
                s(&[
                    "com.apple.Safari",
                    "com.google.Chrome",
                    "org.mozilla.firefox",
                    "company.thebrowser.Browser",
                    "com.brave.Browser",
                    "com.microsoft.edgemac",
                ]),
                s(&["com.microsoft.VSCode", "com.apple.dt.Xcode"]),
                Shortcut::ch('4', MOD_CMD | MOD_SHIFT),
            ),
            Os::Windows => (
                [
                    ("Explorer", "F", "explorer"),
                    ("Terminal", ">_", "wt"),
                    ("Edge", "S", "msedge"),
                ],
                s(&[
                    "msedge.exe",
                    "chrome.exe",
                    "firefox.exe",
                    "brave.exe",
                    "opera.exe",
                    "vivaldi.exe",
                ]),
                s(&["code.exe", "devenv.exe"]),
                Shortcut::ch('s', MOD_CMD | MOD_SHIFT),
            ),
        };
        let mut launch_items: Vec<Item> = launchers
            .iter()
            .map(|(name, icon, app)| Item::run(name, icon, Action::OpenApp((*app).into())))
            .collect();
        launch_items.push(Item::run("Shot", "Shot", Action::Key(shot)));
        let launch = Item::submenu("Launch", "Go", launch_items);
        Config {
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
                    apps: browsers,
                    items: vec![p[3].clone(), p[4].clone(), p[5].clone(), p[6].clone()],
                    active: vec![0],
                },
                Profile {
                    name: "Code".into(),
                    apps: code,
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
            sleep_after_s: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Characterization tests for the macOS defaults (see `keys::tests` for why the US/ANSI
    //! fallback applies). Key codes are asserted literally on purpose.
    use super::*;
    use crate::config::ItemKind;

    /// `(back, forward)` of a mode slice as `(key, mods)` pairs.
    fn keys_of(item: &Item) -> ((u16, u8), (u16, u8)) {
        let ItemKind::Mode { back, forward } = &item.kind else {
            panic!("{} is not a mode", item.name);
        };
        let k = |a: &Action| match a {
            Action::Key(s) => (s.key, s.mods),
            other => panic!("not a key action: {other:?}"),
        };
        (k(back), k(forward))
    }

    fn key_of(item: &Item) -> (u16, u8) {
        match &item.kind {
            ItemKind::Run(Action::Key(s)) => (s.key, s.mods),
            other => panic!("not a key slice: {other:?}"),
        }
    }

    #[test]
    fn the_eight_mode_presets() {
        // the macOS form, pinned: the Windows form has its own test below
        let p = presets_for(Os::Mac);
        let names: Vec<_> = p.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "History",
                "Brackets",
                "Clipboard",
                "Browser",
                "Tabs",
                "Zoom",
                "Scroll",
                "Edit"
            ]
        );
        let cmd = MOD_CMD;
        assert_eq!(keys_of(&p[0]), ((6, cmd), (6, cmd | MOD_SHIFT)));
        assert_eq!(keys_of(&p[1]), ((33, 0), (30, 0)));
        assert_eq!(keys_of(&p[2]), ((8, cmd), (9, cmd)));
        assert_eq!(keys_of(&p[3]), ((33, cmd), (30, cmd)));
        assert_eq!(
            keys_of(&p[4]),
            ((33, cmd | MOD_SHIFT), (30, cmd | MOD_SHIFT))
        );
        assert_eq!(keys_of(&p[5]), ((27, cmd), (24, cmd)));
        assert_eq!(keys_of(&p[6]), ((116, 0), (121, 0)));
        assert_eq!(keys_of(&p[7]), ((36, 0), (51, 0)));
        assert_eq!(p[0].icon, "<>");
    }

    #[test]
    fn the_quick_actions() {
        let q = quick_actions_for(Os::Mac);
        let names: Vec<_> = q.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            ["Screenshot", "Mission Ctrl", "Spotlight", "Emoji", "Lock"]
        );
        assert_eq!(key_of(&q[0]), (21, MOD_CMD | MOD_SHIFT));
        assert_eq!(key_of(&q[1]), (126, MOD_CTRL));
        assert_eq!(key_of(&q[2]), (49, MOD_CMD));
        assert_eq!(key_of(&q[3]), (49, MOD_CMD | MOD_CTRL));
        assert_eq!(key_of(&q[4]), (12, MOD_CMD | MOD_CTRL));
    }

    #[test]
    fn blank_items_offer_a_submenu_only_when_allowed() {
        let with = blank_items(true);
        let without = blank_items(false);
        assert_eq!(with.len(), 5);
        assert_eq!(without.len(), 4);
        assert!(matches!(with[4].kind, ItemKind::Submenu(_)));
        assert_eq!(key_of(&with[0]), (key::F13, 0));
    }

    #[test]
    fn the_default_config() {
        let c = Config::default_for(Os::Mac);
        assert_eq!(c.version, CONFIG_VERSION);
        let names: Vec<_> = c.profiles.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Default", "Browsers", "Code"]);
        assert_eq!(
            c.profiles.iter().map(|p| p.items.len()).collect::<Vec<_>>(),
            [4, 4, 3]
        );
        assert!(
            c.profiles[0].apps.is_empty(),
            "the first profile is the general one"
        );
        assert_eq!(c.profiles[1].apps.len(), 6);
        assert_eq!(c.profiles[1].apps[0], "com.apple.Safari");
        assert_eq!(
            c.profiles[2].apps,
            ["com.microsoft.VSCode", "com.apple.dt.Xcode"]
        );
        assert_eq!(c.pie_hold_ms, 500);
        assert_eq!(c.settings_hold_ms, 1200);
        assert_eq!(c.pie_size, 200.0);
        assert_eq!(c.accent, [64, 156, 255]);
        assert_eq!(c.trigger_button, 2);
        assert_eq!(c.double_click_ms, 260);
        assert_eq!(c.sleep_after_s, 60);
        assert!(!c.flick && !c.sound && !c.launch_at_login);
        assert_eq!(c.app_gesture, AppGesture::DoubleClick);
        assert_eq!(c.target_app, TargetApp::UnderCursor);
    }

    #[test]
    fn the_default_launch_submenu() {
        let c = Config::default_for(Os::Mac);
        let ItemKind::Submenu(launch) = &c.profiles[0].items[3].kind else {
            panic!("slice 3 of the general pie is the launcher");
        };
        let apps: Vec<_> = launch
            .iter()
            .filter_map(|i| match &i.kind {
                ItemKind::Run(Action::OpenApp(a)) => Some(a.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(apps, ["Finder", "Terminal", "Safari"]);
        assert_eq!(key_of(&launch[3]), (21, MOD_CMD | MOD_SHIFT));
    }

    #[test]
    fn the_default_config_is_a_fixed_point_of_sanitize_and_json() {
        let c = Config::default();
        let json = serde_json::to_string_pretty(&c).unwrap();
        assert_eq!(crate::config::parse_config(&json), Some(c.clone()));
        assert_eq!(c.clone().sanitize(), c);
    }

    #[test]
    fn action_json_shape_is_stable() {
        let j = |a: &Action| serde_json::to_string(a).unwrap();
        assert_eq!(j(&Action::Nothing), r#""Nothing""#);
        assert_eq!(
            j(&Action::Key(Shortcut::new(6, MOD_CMD))),
            r#"{"Key":{"key":6,"mods":8}}"#
        );
        assert_eq!(
            j(&Action::OpenApp("Safari".into())),
            r#"{"OpenApp":"Safari"}"#
        );
        assert_eq!(
            j(&Action::OpenUrl("https://x.y".into())),
            r#"{"OpenUrl":"https://x.y"}"#
        );
        assert_eq!(j(&Action::Command("ls".into())), r#"{"Command":"ls"}"#);
        assert_eq!(
            j(&Action::Sequence(vec![Action::Nothing])),
            r#"{"Sequence":["Nothing"]}"#
        );
    }

    #[test]
    fn the_os_specific_forms_match_the_plain_ones_on_this_os() {
        assert_eq!(presets(), presets_for(Os::current()));
        assert_eq!(quick_actions(), quick_actions_for(Os::current()));
        assert_eq!(Config::default(), Config::default_for(Os::current()));
    }

    #[test]
    fn the_macos_defaults_do_not_depend_on_the_build_target() {
        // the characterization tests above run on the build OS; this pins the macOS form on any OS
        let p = presets_for(Os::Mac);
        assert_eq!(keys_of(&p[0]), ((6, MOD_CMD), (6, MOD_CMD | MOD_SHIFT)));
        assert_eq!(keys_of(&p[3]), ((33, MOD_CMD), (30, MOD_CMD)));
        let c = Config::default_for(Os::Mac);
        assert_eq!(c.profiles[1].apps[0], "com.apple.Safari");
    }

    #[test]
    fn windows_presets_use_ctrl_alt_and_win() {
        let p = presets_for(Os::Windows);
        assert_eq!(
            keys_of(&p[0]),
            ((6, MOD_CTRL), (16, MOD_CTRL)),
            "Ctrl+Z / Ctrl+Y"
        );
        assert_eq!(
            keys_of(&p[2]),
            ((8, MOD_CTRL), (9, MOD_CTRL)),
            "Ctrl+C / Ctrl+V"
        );
        assert_eq!(
            keys_of(&p[3]),
            ((key::LEFT, MOD_ALT), (key::RIGHT, MOD_ALT)),
            "Alt+arrows"
        );
        assert_eq!(
            keys_of(&p[4]),
            ((key::TAB, MOD_CTRL | MOD_SHIFT), (key::TAB, MOD_CTRL))
        );
        assert_eq!(
            keys_of(&p[5]),
            ((27, MOD_CTRL), (24, MOD_CTRL)),
            "Ctrl+- / Ctrl+="
        );
        assert_eq!(p.len(), presets_for(Os::Mac).len());
        for item in &p {
            let (a, b) = keys_of(item);
            assert!(
                a.1 & MOD_CMD == 0 || item.name == "never",
                "no Win key in {}",
                item.name
            );
            assert!(b.1 & MOD_CMD == 0);
        }
    }

    #[test]
    fn windows_quick_actions_use_the_win_key() {
        let q = quick_actions_for(Os::Windows);
        let names: Vec<_> = q.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            ["Screenshot", "Task view", "Search", "Emoji", "Lock"]
        );
        assert_eq!(key_of(&q[0]), (1, MOD_CMD | MOD_SHIFT), "Win+Shift+S");
        assert_eq!(key_of(&q[1]), (key::TAB, MOD_CMD), "Win+Tab");
        assert_eq!(key_of(&q[2]), (1, MOD_CMD), "Win+S");
        assert_eq!(key_of(&q[3]), (47, MOD_CMD), "Win+.");
        assert_eq!(key_of(&q[4]), (37, MOD_CMD), "Win+L");
    }

    #[test]
    fn the_windows_default_config_uses_exe_names() {
        let c = Config::default_for(Os::Windows);
        let names: Vec<_> = c.profiles.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Default", "Browsers", "Code"]);
        assert_eq!(c.profiles[1].apps[0], "msedge.exe");
        assert!(c.profiles[1].apps.contains(&"chrome.exe".to_string()));
        assert_eq!(c.profiles[2].apps, ["code.exe", "devenv.exe"]);
        for p in &c.profiles {
            for a in &p.apps {
                assert_eq!(a, &a.to_ascii_lowercase(), "ids are lower case on Windows");
            }
        }
        let ItemKind::Submenu(launch) = &c.profiles[0].items[3].kind else {
            panic!("launcher");
        };
        let apps: Vec<_> = launch
            .iter()
            .filter_map(|i| match &i.kind {
                ItemKind::Run(Action::OpenApp(a)) => Some(a.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(apps, ["explorer", "wt", "msedge"]);
        assert_eq!(key_of(&launch[3]), (1, MOD_CMD | MOD_SHIFT));
    }

    #[test]
    fn both_default_configs_survive_sanitize_and_json() {
        for os in [Os::Mac, Os::Windows] {
            let c = Config::default_for(os);
            let json = serde_json::to_string(&c).unwrap();
            let back: Config = serde_json::from_str(&json).unwrap();
            assert_eq!(back, c, "{os:?} round trip");
            assert!(
                c.profiles
                    .iter()
                    .all(|p| p.items.len() <= crate::config::MAX_ITEMS)
            );
            assert_eq!(c.profiles.iter().filter(|p| p.is_general()).count(), 1);
        }
        // sanitize is only a fixed point of the defaults of the OS it runs on (it normalizes app ids)
        let c = Config::default();
        assert_eq!(c.clone().sanitize(), c);
    }

    #[cfg(not(windows))]
    #[test]
    fn app_ids_match_exactly_on_macos() {
        let c = Config::default();
        assert_eq!(c.app_profile_for(Some("com.apple.Safari")), Some(1));
        assert_eq!(c.app_profile_for(Some("com.apple.safari")), None);
        assert_eq!(c.profile_for(Some("com.microsoft.VSCode")), 2);
        assert_eq!(
            c.profile_for(Some("unknown.app")),
            0,
            "unknown apps use the general pie"
        );
        assert_eq!(c.profile_for(None), 0);
    }
}

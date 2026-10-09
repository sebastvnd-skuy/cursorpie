//! Everything shown on the About page. Edit the TODO values to put your own name on the project.

pub const APP_NAME: &str = "CursorPie";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const TAGLINE: &str = "A hold-to-open pie menu for macOS";

/// Used as the LaunchAgent label and the .app bundle identifier (keep it unique, reverse-DNS style).
pub const BUNDLE_ID: &str = "io.github.cursorpie";

// TODO: replace these with your own details.
pub const AUTHOR_NAME: &str = "Sebastvnd";
pub const AUTHOR_GITHUB_USER: &str = "sebastvnd";
pub const AUTHOR_GITHUB_URL: &str = "https://github.com/sebastvnd";
pub const AUTHOR_EMAIL: &str = "sebastvnd@gmail.com";

pub const REPO_URL: &str = "https://github.com/sebastvnd/cursorpie";
pub const ISSUES_URL: &str = "https://github.com/sebastvnd/cursorpie/issues";
pub const RELEASES_URL: &str = "https://github.com/sebastvnd/cursorpie/releases";

pub const LICENSE_NAME: &str = "MIT License";
pub const COPYRIGHT: &str = "Copyright (c) 2026 Sebastvnd";

pub const INSPIRED_BY: (&str, &str) = ("Kando", "https://github.com/kando-menu/kando");

/// (name, url, license) of the main open-source building blocks.
pub const CREDITS: &[(&str, &str, &str)] = &[
    (
        "egui / eframe",
        "https://github.com/emilk/egui",
        "MIT / Apache-2.0",
    ),
    (
        "core-graphics",
        "https://github.com/servo/core-foundation-rs",
        "MIT / Apache-2.0",
    ),
    (
        "tray-icon / muda",
        "https://github.com/tauri-apps/tray-icon",
        "MIT / Apache-2.0",
    ),
    (
        "objc2",
        "https://github.com/madsmtm/objc2",
        "MIT / Zlib / Apache-2.0",
    ),
    (
        "serde",
        "https://github.com/serde-rs/serde",
        "MIT / Apache-2.0",
    ),
    (
        "image",
        "https://github.com/image-rs/image",
        "MIT / Apache-2.0",
    ),
];

pub const PRIVACY: &str = "CursorPie works entirely on your Mac. It makes no network connections and collects no data. \
Keyboard events are only inspected while you are recording a shortcut in Settings, and are never stored or logged.";

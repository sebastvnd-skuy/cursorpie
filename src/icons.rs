//! Katalog ikon bawaan + ikon buatan pengguna.
//!
//! - `CATEGORIES`   : daftar glyph (emoji / simbol / teks pendek) yang bisa dipakai sebagai ikon slice.
//!                    Tinggal tambah baris `ic("glyph", "Nama", "kata kunci pencarian")`.
//! - `user_icons()` : gambar (.png .jpg .jpeg .webp) yang ada di folder
//!                    `~/Library/Application Support/CursorPie/icons/`.
//!
//! Catatan: glyph digambar oleh font bawaan egui (Noto Emoji monokrom + emoji-icon-font).
//! Kalau satu glyph tampil sebagai kotak kosong, hapus baris itu atau pakai gambar di folder ikon.

use crate::config::home;
use std::fs;
use std::path::{Path, PathBuf};

pub struct IconDef {
    pub glyph: &'static str,
    pub name: &'static str,
    /// kata kunci huruf kecil, dipisah spasi
    pub tags: &'static str,
}

pub struct Category {
    pub title: &'static str,
    pub glyph: &'static str,
    pub icons: &'static [IconDef],
}

const fn ic(glyph: &'static str, name: &'static str, tags: &'static str) -> IconDef {
    IconDef { glyph, name, tags }
}

pub const CATEGORIES: &[Category] = &[
    Category {
        title: "Edit",
        glyph: "✏",
        icons: &[
            ic("✂", "Cut", "cut scissors"),
            ic("📄", "Copy", "copy duplicate page"),
            ic("📋", "Paste", "paste clipboard"),
            ic("↩", "Undo", "undo back"),
            ic("↪", "Redo", "redo forward"),
            ic("🔍", "Find", "search find spotlight"),
            ic("✏", "Edit", "pencil write edit"),
            ic("💾", "Save", "save disk"),
            ic("🗑", "Delete", "trash delete remove"),
            ic("📌", "Pin", "pin"),
            ic("🔗", "Link", "link url chain"),
            ic("📎", "Attach", "clip attach"),
            ic("✔", "Done", "check ok yes tick"),
            ic("✖", "Close", "close cancel x"),
            ic("🔖", "Bookmark", "bookmark"),
            ic("🏷", "Tag", "tag label"),
        ],
    },
    Category {
        title: "Navigate",
        glyph: "➡",
        icons: &[
            ic("⬅", "Left", "left arrow back"),
            ic("➡", "Right", "right arrow forward"),
            ic("⬆", "Up", "up arrow"),
            ic("⬇", "Down", "down arrow"),
            ic("↔", "Horizontal", "horizontal sides"),
            ic("↕", "Vertical", "vertical scroll"),
            ic("🏠", "Home", "home"),
            ic("🔙", "Back", "back"),
            ic("🔄", "Refresh", "reload refresh sync"),
            ic("⏪", "Rewind", "rewind fast back"),
            ic("⏩", "Fast forward", "fast forward"),
            ic("➕", "Add / zoom in", "plus add zoom in"),
            ic("➖", "Zoom out", "minus zoom out"),
            ic("🔎", "Zoom", "magnifier zoom"),
            ic("🔝", "Top", "top up"),
        ],
    },
    Category {
        title: "Media",
        glyph: "▶",
        icons: &[
            ic("▶", "Play", "play"),
            ic("⏸", "Pause", "pause"),
            ic("⏹", "Stop", "stop"),
            ic("⏮", "Previous", "previous track"),
            ic("⏭", "Next", "next track"),
            ic("🔊", "Volume up", "volume loud sound"),
            ic("🔉", "Volume down", "volume quiet"),
            ic("🔇", "Mute", "mute silent"),
            ic("🎵", "Music", "music note"),
            ic("🎧", "Headphones", "headphones audio"),
            ic("🎤", "Microphone", "mic microphone record"),
            ic("📷", "Camera", "camera photo"),
            ic("📸", "Screenshot", "screenshot capture"),
            ic("🎬", "Video", "video film clapper"),
            ic("🖼", "Image", "image picture"),
        ],
    },
    Category {
        title: "System",
        glyph: "⚙",
        icons: &[
            ic("⚙", "Settings", "settings gear preferences"),
            ic("🔒", "Lock", "lock secure"),
            ic("🔓", "Unlock", "unlock"),
            ic("🔋", "Battery", "battery power"),
            ic("📶", "Wi-Fi", "wifi network signal"),
            ic("💡", "Brightness", "brightness light bulb"),
            ic("🌙", "Night", "moon night dark focus"),
            ic("☀", "Light", "sun light day"),
            ic("🔌", "Plug", "plug power"),
            ic("⌨", "Keyboard", "keyboard keys"),
            ic("🖥", "Display", "display screen monitor desktop"),
            ic("🖱", "Mouse", "mouse cursor"),
            ic("🖨", "Print", "print printer"),
            ic("🔔", "Notifications", "bell notification"),
            ic("🕐", "Clock", "clock time"),
            ic("📅", "Calendar", "calendar date"),
            ic("💻", "Laptop", "laptop computer"),
            ic("📱", "Phone", "phone mobile"),
        ],
    },
    Category {
        title: "Apps",
        glyph: "📁",
        icons: &[
            ic("📁", "Folder", "folder finder"),
            ic("📂", "Open folder", "open folder"),
            ic("🗂", "Files", "files archive"),
            ic("📝", "Notes", "notes memo text"),
            ic("✉", "Mail", "mail email envelope"),
            ic("💬", "Chat", "chat message talk"),
            ic("🌐", "Web", "web browser globe internet"),
            ic("🎮", "Games", "game controller"),
            ic("🎨", "Design", "design art palette paint"),
            ic("📦", "Package", "package box"),
            ic("🔧", "Tools", "tools wrench"),
            ic("🔨", "Build", "build hammer"),
            ic("☁", "Cloud", "cloud sync"),
            ic("🚀", "Launch", "launch rocket start"),
            ic("📖", "Book", "book read"),
            ic("📊", "Chart", "chart stats data"),
        ],
    },
    Category {
        title: "Fun",
        glyph: "⭐",
        icons: &[
            ic("🐛", "Bug", "bug debug"),
            ic("⚡", "Quick", "zap quick fast lightning"),
            ic("🔥", "Fire", "fire hot"),
            ic("✅", "Success", "success check done"),
            ic("❌", "Error", "error cross no"),
            ic("❓", "Help", "help question"),
            ic("❗", "Alert", "alert important"),
            ic("⭐", "Star", "star favorite"),
            ic("❤", "Heart", "heart love like"),
            ic("🏁", "Flag", "flag finish"),
            ic("🎯", "Target", "target goal"),
            ic("💎", "Gem", "gem diamond"),
            ic("🎁", "Gift", "gift present"),
            ic("👍", "Like", "like thumbs up"),
        ],
    },
    Category {
        title: "Text",
        glyph: "Aa",
        icons: &[
            ic("<>", "History", "history undo redo"),
            ic("< >", "Browser", "browser back forward"),
            ic("[ ]", "Brackets", "brackets"),
            ic("{ }", "Braces", "braces code"),
            ic("C/V", "Clipboard", "copy paste clipboard"),
            ic("+/-", "Zoom", "zoom plus minus"),
            ic(">_", "Terminal", "terminal command shell"),
            ic("Tab", "Tab", "tab tabs"),
            ic("Esc", "Escape", "escape"),
            ic("Ret", "Return", "return enter"),
            ic("Pg", "Page", "page scroll"),
            ic("...", "More", "more menu dots"),
        ],
    },
];

/// Hasil pencarian. `cat` = 0 untuk semua kategori, 1.. untuk kategori ke-(cat-1).
pub fn filtered(query: &str, cat: usize) -> Vec<&'static IconDef> {
    let q = query.trim().to_lowercase();
    let mut out = Vec::new();
    for (ci, c) in CATEGORIES.iter().enumerate() {
        if cat != 0 && cat != ci + 1 {
            continue;
        }
        for d in c.icons {
            if q.is_empty() || d.name.to_lowercase().contains(&q) || d.tags.contains(&q) {
                out.push(d);
            }
        }
    }
    out
}

// ------------------------------------------------------------ folder ikon pengguna

pub const MAX_USER_ICONS: usize = 120;
const MAX_ICON_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// ~/Library/Application Support/CursorPie/icons (dibuat kalau belum ada).
pub fn user_icons_dir() -> PathBuf {
    let mut p = home();
    p.push("Library");
    p.push("Application Support");
    p.push("CursorPie");
    p.push("icons");
    fs::create_dir_all(&p).ok();
    p
}

fn as_tilde(p: &Path) -> String {
    match p.strip_prefix(home()) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => p.display().to_string(),
    }
}

/// Gambar di folder ikon, sebagai string yang langsung bisa dipakai di `Item.icon`.
pub fn user_icons() -> Vec<String> {
    let Ok(rd) = fs::read_dir(user_icons_dir()) else {
        return Vec::new();
    };
    let mut v: Vec<String> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|x| x.to_str())
                .map(|x| matches!(x.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp"))
                .unwrap_or(false)
        })
        .filter(|p| {
            p.metadata()
                .map(|m| m.len() <= MAX_ICON_FILE_BYTES)
                .unwrap_or(false)
        })
        .map(|p| as_tilde(&p))
        .collect();
    v.sort();
    v.truncate(MAX_USER_ICONS);
    v
}

/// Membuka folder ikon di Finder (proses `open` dipanen supaya tidak jadi zombie).
pub fn open_icons_folder() {
    if let Ok(mut child) = std::process::Command::new("open")
        .arg(user_icons_dir())
        .spawn()
    {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

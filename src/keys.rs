//! Shortcut model: virtual key code + modifier bitmask, with display labels.

use core_graphics::event::CGEventFlags;
use serde::{Deserialize, Serialize};

/// Modifier bit: Shift.
pub const MOD_SHIFT: u8 = 1;
/// Modifier bit: Control.
pub const MOD_CTRL: u8 = 2;
/// Modifier bit: Option (Alt).
pub const MOD_ALT: u8 = 4;
/// Modifier bit: Command.
pub const MOD_CMD: u8 = 8;

/// Virtual key codes that mean the same thing on every layout (no letters / digits / punctuation:
/// those move around between layouts, use `Shortcut::ch` for them).
pub mod key {
    /// Space bar.
    pub const SPACE: u16 = 49;
    /// Return.
    pub const RETURN: u16 = 36;
    /// Backspace (the key macOS calls Delete).
    pub const DELETE: u16 = 51;
    /// Page Up.
    pub const PAGE_UP: u16 = 116;
    /// Page Down.
    pub const PAGE_DOWN: u16 = 121;
    /// Arrow up.
    pub const UP: u16 = 126;
    /// F13 (a safe default for a new shortcut slice).
    pub const F13: u16 = 105;
}

/// One shortcut = any key code + modifier combination (recorded from the keyboard).
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Shortcut {
    pub key: u16,
    pub mods: u8,
}

impl Shortcut {
    /// A shortcut from a virtual key code and a `MOD_*` bitmask.
    pub const fn new(key: u16, mods: u8) -> Self {
        Self { key, mods }
    }

    /// The key that *types* `c` on the current layout (AZERTY, QWERTZ, Dvorak, ...), plus any
    /// modifier that layout needs to type it. Use for built-in presets such as Cmd+Z.
    /// Falls back to the US/ANSI position if the layout is not available yet.
    pub fn ch(c: char, mods: u8) -> Self {
        let c = c.to_ascii_lowercase();
        if let Some((code, need)) = crate::platform::layout::find_key(&c.to_string()) {
            return Self::new(code, mods | need);
        }
        Self::new(ansi_code(c).unwrap_or(0), mods)
    }

    /// Display text. Shift + a symbol key shows the resulting symbol ("+" instead of "Shift + =").
    pub fn label(&self) -> String {
        let shift = self.mods & MOD_SHIFT != 0;
        let sym = if shift {
            shifted_symbol(self.key)
        } else {
            None
        };

        let mut parts: Vec<String> = Vec::new();
        if self.mods & MOD_CTRL != 0 {
            parts.push("Ctrl".into());
        }
        if self.mods & MOD_ALT != 0 {
            parts.push("Opt".into());
        }
        if shift && sym.is_none() {
            parts.push("Shift".into());
        }
        if self.mods & MOD_CMD != 0 {
            parts.push("Cmd".into());
        }
        parts.push(match sym {
            Some(s) => s,
            None => key_name(self.key),
        });
        parts.join(" + ")
    }
}

/// Symbol a key types while Shift is held on the *current* layout, when that is a symbol/digit
/// rather than just the capital letter ("+" instead of "Shift + =").
fn shifted_symbol(code: u16) -> Option<String> {
    if special_name(code).is_some() {
        return None;
    }
    let shifted = crate::platform::layout::char_for(code, true)?;
    if shifted.chars().any(|c| c.is_alphabetic()) {
        return None;
    }
    if crate::platform::layout::char_for(code, false).as_deref() == Some(shifted.as_str()) {
        return None;
    }
    Some(shifted)
}

/// Keys that are named the same on every layout (not produced by UCKeyTranslate).
pub fn special_name(code: u16) -> Option<&'static str> {
    Some(match code {
        36 => "Return",
        48 => "Tab",
        49 => "Space",
        51 => "Backspace",
        53 => "Esc",
        64 => "F17",
        65 => "Num .",
        67 => "Num *",
        69 => "Num +",
        71 => "Num Clear",
        72 => "Volume Up",
        73 => "Volume Down",
        74 => "Mute",
        75 => "Num /",
        76 => "Num Enter",
        78 => "Num -",
        79 => "F18",
        80 => "F19",
        81 => "Num =",
        82 => "Num 0",
        83 => "Num 1",
        84 => "Num 2",
        85 => "Num 3",
        86 => "Num 4",
        87 => "Num 5",
        88 => "Num 6",
        89 => "Num 7",
        90 => "F20",
        91 => "Num 8",
        92 => "Num 9",
        96 => "F5",
        97 => "F6",
        98 => "F7",
        99 => "F3",
        100 => "F8",
        101 => "F9",
        102 => "Eisu",
        103 => "F11",
        104 => "Kana",
        105 => "F13",
        106 => "F16",
        107 => "F14",
        109 => "F10",
        111 => "F12",
        113 => "F15",
        114 => "Help",
        115 => "Home",
        116 => "Page Up",
        117 => "Delete",
        118 => "F4",
        119 => "End",
        120 => "F2",
        121 => "Page Down",
        122 => "F1",
        123 => "Left",
        124 => "Right",
        125 => "Down",
        126 => "Up",
        _ => return None,
    })
}

/// US/ANSI letters, digits and punctuation. Only a fallback for when the layout snapshot is not
/// available (e.g. queried from another thread before `layout::init()` ran).
const ANSI: &[(u16, &str)] = &[
    (0, "A"),
    (1, "S"),
    (2, "D"),
    (3, "F"),
    (4, "H"),
    (5, "G"),
    (6, "Z"),
    (7, "X"),
    (8, "C"),
    (9, "V"),
    (11, "B"),
    (12, "Q"),
    (13, "W"),
    (14, "E"),
    (15, "R"),
    (16, "Y"),
    (17, "T"),
    (18, "1"),
    (19, "2"),
    (20, "3"),
    (21, "4"),
    (22, "6"),
    (23, "5"),
    (24, "="),
    (25, "9"),
    (26, "7"),
    (27, "-"),
    (28, "8"),
    (29, "0"),
    (30, "]"),
    (31, "O"),
    (32, "U"),
    (33, "["),
    (34, "I"),
    (35, "P"),
    (37, "L"),
    (38, "J"),
    (39, "'"),
    (40, "K"),
    (41, ";"),
    (42, "\\"),
    (43, ","),
    (44, "/"),
    (45, "N"),
    (46, "M"),
    (47, "."),
    (50, "`"),
];

/// Key code of a letter, digit or punctuation mark on a US/ANSI keyboard.
fn ansi_code(c: char) -> Option<u16> {
    let s = c.to_string();
    ANSI.iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(&s))
        .map(|(k, _)| *k)
}

/// Display name of a key on the current layout. Letters are shown uppercase ("Z", not "z").
pub fn key_name(code: u16) -> String {
    if let Some(s) = special_name(code) {
        return s.to_string();
    }
    if let Some(s) = crate::platform::layout::char_for(code, false) {
        // keep "ß"-style characters as typed instead of expanding them to "SS"
        let upper = s.to_uppercase();
        return if upper.chars().count() == s.chars().count() {
            upper
        } else {
            s
        };
    }
    if let Some((_, n)) = ANSI.iter().find(|(k, _)| *k == code) {
        return n.to_string();
    }
    format!("Key {code}")
}

/// Converts event flags from a `CGEvent` into a `MOD_*` bitmask.
pub fn mods_from_flags(f: CGEventFlags) -> u8 {
    let mut m = 0;
    if f.contains(CGEventFlags::CGEventFlagShift) {
        m |= MOD_SHIFT;
    }
    if f.contains(CGEventFlags::CGEventFlagControl) {
        m |= MOD_CTRL;
    }
    if f.contains(CGEventFlags::CGEventFlagAlternate) {
        m |= MOD_ALT;
    }
    if f.contains(CGEventFlags::CGEventFlagCommand) {
        m |= MOD_CMD;
    }
    m
}

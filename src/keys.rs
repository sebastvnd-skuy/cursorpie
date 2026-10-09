//! Shortcut model: virtual key code + modifier bitmask, with display labels.

use core_graphics::event::CGEventFlags;
use serde::{Deserialize, Serialize};

pub const MOD_SHIFT: u8 = 1;
pub const MOD_CTRL: u8 = 2;
pub const MOD_ALT: u8 = 4;
pub const MOD_CMD: u8 = 8;

/// macOS virtual key codes (ANSI layout) used by presets.
pub mod key {
    pub const MINUS: u16 = 27;
    pub const EQUAL: u16 = 24;
    pub const RBRACKET: u16 = 30;
    pub const LBRACKET: u16 = 33;
    pub const C: u16 = 8;
    pub const V: u16 = 9;
    pub const Z: u16 = 6;
    pub const Q: u16 = 12;
    pub const DIGIT4: u16 = 21;
    pub const SPACE: u16 = 49;
    pub const RETURN: u16 = 36;
    pub const DELETE: u16 = 51;
    pub const PAGE_UP: u16 = 116;
    pub const PAGE_DOWN: u16 = 121;
    pub const UP: u16 = 126;
    pub const F13: u16 = 105;
}

/// One shortcut = any key code + modifier combination (recorded from the keyboard).
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Shortcut {
    pub key: u16,
    pub mods: u8,
}

impl Shortcut {
    pub const fn new(key: u16, mods: u8) -> Self {
        Self { key, mods }
    }

    /// Display text. Shift + a symbol key shows the resulting symbol ("+" instead of "Shift + =").
    pub fn label(&self) -> String {
        let shift = self.mods & MOD_SHIFT != 0;
        let sym = if shift { shifted_symbol(self.key) } else { None };

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
            Some(s) => s.to_string(),
            None => key_name(self.key),
        });
        parts.join(" + ")
    }
}

/// Character produced by a key while Shift is held (US/ANSI layout).
fn shifted_symbol(code: u16) -> Option<&'static str> {
    Some(match code {
        18 => "!",
        19 => "@",
        20 => "#",
        21 => "$",
        23 => "%",
        22 => "^",
        26 => "&",
        28 => "*",
        25 => "(",
        29 => ")",
        27 => "_",
        24 => "+",
        33 => "{",
        30 => "}",
        42 => "|",
        41 => ":",
        39 => "\"",
        43 => "<",
        47 => ">",
        44 => "?",
        50 => "~",
        _ => return None,
    })
}

pub fn key_name(code: u16) -> String {
    let s = match code {
        0 => "A", 1 => "S", 2 => "D", 3 => "F", 4 => "H", 5 => "G", 6 => "Z", 7 => "X",
        8 => "C", 9 => "V", 11 => "B", 12 => "Q", 13 => "W", 14 => "E", 15 => "R", 16 => "Y",
        17 => "T", 18 => "1", 19 => "2", 20 => "3", 21 => "4", 22 => "6", 23 => "5", 24 => "=",
        25 => "9", 26 => "7", 27 => "-", 28 => "8", 29 => "0", 30 => "]", 31 => "O", 32 => "U",
        33 => "[", 34 => "I", 35 => "P", 36 => "Return", 37 => "L", 38 => "J", 39 => "'",
        40 => "K", 41 => ";", 42 => "\\", 43 => ",", 44 => "/", 45 => "N", 46 => "M", 47 => ".",
        48 => "Tab", 49 => "Space", 50 => "`", 51 => "Backspace", 53 => "Esc",
        64 => "F17", 65 => "Num .", 67 => "Num *", 69 => "Num +", 71 => "Num Clear",
        72 => "Volume Up", 73 => "Volume Down", 74 => "Mute", 75 => "Num /", 76 => "Num Enter",
        78 => "Num -", 79 => "F18", 80 => "F19", 81 => "Num =", 82 => "Num 0", 83 => "Num 1",
        84 => "Num 2", 85 => "Num 3", 86 => "Num 4", 87 => "Num 5", 88 => "Num 6", 89 => "Num 7",
        90 => "F20", 91 => "Num 8", 92 => "Num 9", 96 => "F5", 97 => "F6", 98 => "F7", 99 => "F3",
        100 => "F8", 101 => "F9", 103 => "F11", 105 => "F13", 106 => "F16", 107 => "F14",
        109 => "F10", 111 => "F12", 113 => "F15", 114 => "Help", 115 => "Home", 116 => "Page Up",
        117 => "Delete", 118 => "F4", 119 => "End", 120 => "F2", 121 => "Page Down", 122 => "F1",
        123 => "Left", 124 => "Right", 125 => "Down", 126 => "Up",
        _ => return format!("Key {code}"),
    };
    s.to_string()
}

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

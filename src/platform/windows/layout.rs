//! Keyboard-layout awareness (`MapVirtualKeyExW` + `ToUnicodeEx`).
//!
//! Key codes are physical positions (see `platform::keymap`). What a position types
//! depends on the active layout, so this module builds a snapshot of it:
//!
//! * [`char_for`] shows correct labels for recorded shortcuts;
//! * [`find_key`] resolves "the key that types `z`" for built-in presets.
//!
//! Unlike macOS, Windows layouts are per thread and can change at any time, so the snapshot is
//! keyed by the layout handle of the foreground window and rebuilt when that changes. Any thread
//! may call these functions. Until a snapshot exists callers fall back to a US/ANSI table.

use crate::keys::{MOD_ALT, MOD_CTRL, MOD_SHIFT};
use crate::platform::keymap;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, MAPVK_VSC_TO_VK_EX, MapVirtualKeyExW, ToUnicodeEx,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// Highest macOS key code that can be a layout key (the table has 128 slots, like on macOS).
const SLOTS: usize = 128;

/// Snapshot of one layout.
pub(super) struct Table {
    /// `chars[code][variant]`; variant bit 0 = Shift, bit 1 = AltGr.
    pub chars: Vec<[Option<String>; 4]>,
    /// text -> (code, `MOD_*` bits needed to type it). The lowest-modifier combination wins.
    pub reverse: HashMap<String, (u16, u8)>,
}

/// The `MOD_*` bits a variant needs: Shift, and AltGr (which Windows treats as Ctrl + Alt).
pub(super) fn variant_mods(variant: usize) -> u8 {
    let mut m = 0;
    if variant & 1 != 0 {
        m |= MOD_SHIFT;
    }
    if variant & 2 != 0 {
        m |= MOD_CTRL | MOD_ALT;
    }
    m
}

/// Builds the reverse map of `chars`. The outer loop is the variant, so "no modifier" beats "Shift"
/// beats "AltGr" when several keys type the same text.
pub(super) fn build_reverse(chars: &[[Option<String>; 4]]) -> HashMap<String, (u16, u8)> {
    let mut reverse: HashMap<String, (u16, u8)> = HashMap::new();
    for variant in 0..4usize {
        let mods = variant_mods(variant);
        for (code, row) in chars.iter().enumerate() {
            if let Some(s) = &row[variant] {
                reverse.entry(s.clone()).or_insert((code as u16, mods));
            }
        }
    }
    reverse
}

impl Table {
    fn from_chars(chars: Vec<[Option<String>; 4]>) -> Self {
        let reverse = build_reverse(&chars);
        Self { chars, reverse }
    }
}

/// The keyboard layout of the foreground window (or of the calling thread when there is none).
fn current_layout() -> isize {
    // SAFETY: plain queries.
    unsafe {
        let fg = GetForegroundWindow();
        let thread = if fg.is_null() {
            0
        } else {
            GetWindowThreadProcessId(fg, std::ptr::null_mut())
        };
        GetKeyboardLayout(thread) as isize
    }
}

/// The text typed by scan code `scan` with `variant` held on layout `hkl`; `None` for dead keys,
/// control characters and keys that type nothing.
fn translate(hkl: isize, scan: u16, variant: usize) -> Option<String> {
    let mut state = [0u8; 256];
    if variant & 1 != 0 {
        state[0x10] = 0x80; // VK_SHIFT
    }
    if variant & 2 != 0 {
        state[0x11] = 0x80; // VK_CONTROL
        state[0x12] = 0x80; // VK_MENU: together they are AltGr
    }
    let mut buf = [0u16; 8];
    // SAFETY: the state array has 256 entries and the buffer length is passed along. Flag 4 keeps
    // the call from changing the keyboard's dead-key state (Windows 10 1607 and later).
    let n = unsafe {
        let vk = MapVirtualKeyExW(scan as u32, MAPVK_VSC_TO_VK_EX, hkl as _);
        if vk == 0 {
            return None;
        }
        ToUnicodeEx(
            vk,
            scan as u32,
            state.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as i32,
            4,
            hkl as _,
        )
    };
    if n <= 0 {
        return None; // 0 = nothing, negative = dead key
    }
    let s = String::from_utf16(&buf[..(n as usize).min(buf.len())]).ok()?;
    (!s.chars().any(|c| c.is_control())).then_some(s)
}

/// Builds the snapshot of layout `hkl`.
fn build(hkl: isize) -> Table {
    let mut chars: Vec<[Option<String>; 4]> = vec![Default::default(); SLOTS];
    for code in 0..SLOTS as u16 {
        let Some(scan) = keymap::scancode_for(code) else {
            continue;
        };
        for variant in 0..4usize {
            chars[code as usize][variant] = translate(hkl, scan, variant);
        }
    }
    Table::from_chars(chars)
}

/// The cached snapshot and the layout it was built for.
static TABLE: RwLock<Option<(isize, Arc<Table>)>> = RwLock::new(None);

/// The snapshot of the current layout, rebuilt when the layout changed.
fn table() -> Option<Arc<Table>> {
    let hkl = current_layout();
    if let Ok(g) = TABLE.read()
        && let Some((cached, t)) = g.as_ref()
        && *cached == hkl
    {
        return Some(t.clone());
    }
    let t = Arc::new(build(hkl));
    if let Ok(mut g) = TABLE.write() {
        *g = Some((hkl, t.clone()));
    }
    Some(t)
}

/// Builds the first snapshot. Nothing needs the main thread on Windows.
pub fn init() {
    let _ = table();
}

/// Rebuilds the snapshot if the layout changed.
pub fn refresh() {
    let _ = table();
}

/// Text typed by physical key `code` on the current layout (`None` if it is not a layout key).
pub fn char_for(code: u16, shift: bool) -> Option<String> {
    table()?.chars.get(code as usize)?[shift as usize].clone()
}

/// Physical key that types `text` on the current layout, plus the `MOD_*` bits it needs
/// (for example `[` on a German layout needs AltGr, which is Ctrl + Alt).
pub fn find_key(text: &str) -> Option<(u16, u8)> {
    table()?.reverse.get(text).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(none: &str, shift: &str, altgr: Option<&str>) -> [Option<String>; 4] {
        [
            Some(none.into()),
            Some(shift.into()),
            altgr.map(Into::into),
            None,
        ]
    }

    #[test]
    fn variants_name_their_modifiers() {
        assert_eq!(variant_mods(0), 0);
        assert_eq!(variant_mods(1), MOD_SHIFT);
        assert_eq!(variant_mods(2), MOD_CTRL | MOD_ALT);
        assert_eq!(variant_mods(3), MOD_SHIFT | MOD_CTRL | MOD_ALT);
    }

    #[test]
    fn the_lowest_modifier_combination_wins() {
        let mut chars: Vec<[Option<String>; 4]> = vec![Default::default(); 8];
        chars[1] = row("a", "A", None);
        chars[2] = row("q", "Q", Some("a")); // "a" is also AltGr+q: the plain key must win
        let rev = build_reverse(&chars);
        assert_eq!(rev["a"], (1, 0));
        assert_eq!(rev["A"], (1, MOD_SHIFT));
    }

    #[test]
    fn a_symbol_that_needs_altgr_reports_ctrl_and_alt() {
        // German layout: "[" is AltGr+8
        let mut chars: Vec<[Option<String>; 4]> = vec![Default::default(); 40];
        chars[28] = row("8", "(", Some("["));
        let rev = build_reverse(&chars);
        assert_eq!(rev["["], (28, MOD_CTRL | MOD_ALT));
        assert_eq!(rev["8"], (28, 0));
        assert_eq!(rev["("], (28, MOD_SHIFT));
    }

    #[test]
    fn table_from_chars_keeps_both_directions() {
        let mut chars: Vec<[Option<String>; 4]> = vec![Default::default(); 4];
        chars[3] = row("z", "Z", None);
        let t = Table::from_chars(chars);
        assert_eq!(t.chars[3][0].as_deref(), Some("z"));
        assert_eq!(t.reverse.get("Z"), Some(&(3, MOD_SHIFT)));
    }
}

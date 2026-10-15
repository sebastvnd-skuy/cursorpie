//! Key tables that translate between CursorPie's key codes and Windows keys.
//!
//! `config.json` stores macOS virtual key codes (see `docs/WINDOWS_MIGRATION.md`). A code is a
//! *physical position*, so the Windows backend maps it to the key in the same position:
//!
//! * **layout keys** (letters, digits, punctuation) by PS/2 **scan code**, which is also a position
//!   and therefore follows the layout exactly like the macOS code does;
//! * **special keys** (Return, arrows, function keys, numpad...) by **virtual-key code**, because
//!   their meaning does not depend on the layout.
//!
//! The tables are plain data with no OS types, so they are unit tested on every OS.

/// `(macOS code, set-1 scan code)` of the keys that type text, in US/ANSI positions, plus the
/// extra ISO key (macOS 10, scan code 0x56).
const LAYOUT_KEYS: &[(u16, u16)] = &[
    (0, 0x1E),  // A
    (1, 0x1F),  // S
    (2, 0x20),  // D
    (3, 0x21),  // F
    (4, 0x23),  // H
    (5, 0x22),  // G
    (6, 0x2C),  // Z
    (7, 0x2D),  // X
    (8, 0x2E),  // C
    (9, 0x2F),  // V
    (10, 0x56), // ISO extra key
    (11, 0x30), // B
    (12, 0x10), // Q
    (13, 0x11), // W
    (14, 0x12), // E
    (15, 0x13), // R
    (16, 0x15), // Y
    (17, 0x14), // T
    (18, 0x02), // 1
    (19, 0x03), // 2
    (20, 0x04), // 3
    (21, 0x05), // 4
    (22, 0x07), // 6
    (23, 0x06), // 5
    (24, 0x0D), // =
    (25, 0x0A), // 9
    (26, 0x08), // 7
    (27, 0x0C), // -
    (28, 0x09), // 8
    (29, 0x0B), // 0
    (30, 0x1B), // ]
    (31, 0x18), // O
    (32, 0x16), // U
    (33, 0x1A), // [
    (34, 0x17), // I
    (35, 0x19), // P
    (37, 0x26), // L
    (38, 0x24), // J
    (39, 0x28), // '
    (40, 0x25), // K
    (41, 0x27), // ;
    (42, 0x2B), // \
    (43, 0x33), // ,
    (44, 0x35), // /
    (45, 0x31), // N
    (46, 0x32), // M
    (47, 0x34), // .
    (50, 0x29), // `
];

/// `(macOS code, virtual-key code, extended key)` of the keys whose meaning ignores the layout.
const SPECIAL_KEYS: &[(u16, u16, bool)] = &[
    (36, 0x0D, false),  // Return
    (76, 0x0D, true),   // numpad Enter (the same VK, flagged extended)
    (48, 0x09, false),  // Tab
    (49, 0x20, false),  // Space
    (51, 0x08, false),  // Backspace
    (53, 0x1B, false),  // Esc
    (122, 0x70, false), // F1
    (120, 0x71, false), // F2
    (99, 0x72, false),  // F3
    (118, 0x73, false), // F4
    (96, 0x74, false),  // F5
    (97, 0x75, false),  // F6
    (98, 0x76, false),  // F7
    (100, 0x77, false), // F8
    (101, 0x78, false), // F9
    (109, 0x79, false), // F10
    (103, 0x7A, false), // F11
    (111, 0x7B, false), // F12
    (105, 0x7C, false), // F13
    (107, 0x7D, false), // F14
    (113, 0x7E, false), // F15
    (106, 0x7F, false), // F16
    (64, 0x80, false),  // F17
    (79, 0x81, false),  // F18
    (80, 0x82, false),  // F19
    (90, 0x83, false),  // F20
    (115, 0x24, true),  // Home
    (119, 0x23, true),  // End
    (116, 0x21, true),  // Page Up
    (121, 0x22, true),  // Page Down
    (117, 0x2E, true),  // Delete (forward)
    (114, 0x2D, true),  // Help is Insert on a PC keyboard
    (123, 0x25, true),  // Left
    (126, 0x26, true),  // Up
    (124, 0x27, true),  // Right
    (125, 0x28, true),  // Down
    (82, 0x60, false),  // numpad 0
    (83, 0x61, false),  // numpad 1
    (84, 0x62, false),  // numpad 2
    (85, 0x63, false),  // numpad 3
    (86, 0x64, false),  // numpad 4
    (87, 0x65, false),  // numpad 5
    (88, 0x66, false),  // numpad 6
    (89, 0x67, false),  // numpad 7
    (91, 0x68, false),  // numpad 8
    (92, 0x69, false),  // numpad 9
    (67, 0x6A, false),  // numpad *
    (69, 0x6B, false),  // numpad +
    (78, 0x6D, false),  // numpad -
    (65, 0x6E, false),  // numpad .
    (75, 0x6F, true),   // numpad /
    (71, 0x0C, false),  // numpad Clear
    (72, 0xAF, false),  // Volume Up
    (73, 0xAE, false),  // Volume Down
    (74, 0xAD, false),  // Mute
];

/// Scan code of the layout key at macOS position `code`.
pub fn scancode_for(code: u16) -> Option<u16> {
    LAYOUT_KEYS
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, s)| *s)
}

/// macOS position of the layout key with this scan code (not extended).
pub fn code_for_scancode(scan: u16) -> Option<u16> {
    LAYOUT_KEYS
        .iter()
        .find(|(_, s)| *s == scan)
        .map(|(c, _)| *c)
}

/// `(virtual-key code, extended)` of a special key.
pub fn vk_for(code: u16) -> Option<(u16, bool)> {
    SPECIAL_KEYS
        .iter()
        .find(|(c, _, _)| *c == code)
        .map(|(_, vk, ext)| (*vk, *ext))
}

/// macOS code of a special key from its virtual-key code. `extended` is the "extended key" flag of
/// the event; it tells numpad Enter from Return.
pub fn code_for_vk(vk: u16, extended: bool) -> Option<u16> {
    SPECIAL_KEYS
        .iter()
        .find(|(_, v, e)| *v == vk && *e == extended)
        .map(|(c, _, _)| *c)
}

/// Whether `vk` is a modifier key (Shift, Ctrl, Alt, either Windows key). They never become the
/// key of a recorded shortcut.
pub fn is_modifier_vk(vk: u16) -> bool {
    matches!(vk, 0x10 | 0x11 | 0x12 | 0xA0..=0xA5 | 0x5B | 0x5C)
}

/// Translates a key event of the Windows hook into a macOS code: layout keys by scan code, the
/// rest by virtual-key code. `None` for modifiers and keys CursorPie has no code for.
pub fn code_for_event(vk: u16, scan: u16, extended: bool) -> Option<u16> {
    if is_modifier_vk(vk) {
        return None;
    }
    if !extended && let Some(code) = code_for_scancode(scan) {
        return Some(code);
    }
    code_for_vk(vk, extended)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::special_name;

    #[test]
    fn no_duplicate_entries() {
        for (i, (c, s)) in LAYOUT_KEYS.iter().enumerate() {
            for (c2, s2) in &LAYOUT_KEYS[i + 1..] {
                assert_ne!(c, c2, "mac code {c} twice");
                assert_ne!(s, s2, "scan code {s:#x} twice");
            }
        }
        for (i, (c, v, e)) in SPECIAL_KEYS.iter().enumerate() {
            for (c2, v2, e2) in &SPECIAL_KEYS[i + 1..] {
                assert_ne!(c, c2, "mac code {c} twice");
                assert!(!(v == v2 && e == e2), "vk {v:#x} twice");
            }
        }
    }

    #[test]
    fn layout_and_special_keys_do_not_overlap() {
        for (c, _) in LAYOUT_KEYS {
            assert!(vk_for(*c).is_none(), "{c} is in both tables");
            assert!(special_name(*c).is_none(), "{c} has a fixed name");
        }
    }

    #[test]
    fn every_layout_position_of_the_ansi_table_has_a_scan_code() {
        for code in 0..=50u16 {
            if special_name(code).is_none() && code != 10 {
                // codes 0..=50 that are not special are exactly the ANSI letters, digits and marks
                assert!(scancode_for(code).is_some(), "code {code}");
            }
        }
    }

    #[test]
    fn every_special_key_with_a_name_has_a_virtual_key() {
        // Eisu, Kana and numpad "=" exist on Apple keyboards only
        let apple_only = [81u16, 102, 104];
        for code in 0..128u16 {
            if special_name(code).is_some() && !apple_only.contains(&code) {
                assert!(
                    vk_for(code).is_some(),
                    "{} ({code})",
                    special_name(code).unwrap()
                );
            }
        }
    }

    #[test]
    fn round_trips() {
        for (c, s) in LAYOUT_KEYS {
            assert_eq!(code_for_scancode(*s), Some(*c));
            assert_eq!(code_for_event(0x41, *s, false), Some(*c));
        }
        for (c, v, e) in SPECIAL_KEYS {
            assert_eq!(code_for_vk(*v, *e), Some(*c));
            assert_eq!(vk_for(*c), Some((*v, *e)));
        }
    }

    #[test]
    fn well_known_positions() {
        assert_eq!(scancode_for(6), Some(0x2C), "Z");
        assert_eq!(scancode_for(0), Some(0x1E), "A");
        assert_eq!(scancode_for(12), Some(0x10), "Q");
        assert_eq!(scancode_for(18), Some(0x02), "1");
        assert_eq!(scancode_for(29), Some(0x0B), "0");
        assert_eq!(vk_for(49), Some((0x20, false)), "Space");
        assert_eq!(vk_for(126), Some((0x26, true)), "Up");
    }

    #[test]
    fn numpad_enter_is_not_return() {
        assert_eq!(code_for_event(0x0D, 0x1C, false), Some(36));
        assert_eq!(code_for_event(0x0D, 0x1C, true), Some(76));
    }

    #[test]
    fn modifiers_are_never_keys() {
        for vk in [
            0x10, 0x11, 0x12, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0x5B, 0x5C,
        ] {
            assert!(is_modifier_vk(vk));
            assert_eq!(code_for_event(vk, 0x2A, false), None);
        }
        assert!(!is_modifier_vk(0x41));
    }

    #[test]
    fn unknown_keys_have_no_code() {
        assert_eq!(code_for_event(0xFF, 0x7F, false), None);
    }

    #[test]
    fn text_keys_win_over_the_virtual_key() {
        // on a Cyrillic layout the VK of the "Z" position is not 'Z', but the scan code still is
        assert_eq!(code_for_event(0xDE, 0x2C, false), Some(6));
    }
}

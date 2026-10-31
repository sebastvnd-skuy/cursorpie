//! The mice the kernel lists in `/proc/bus/input/devices` (pure: the file is read by
//! `backend::mouse_devices`, so the parser is tested on every OS).
//!
//! One device is a block of lines separated from the next by an empty line:
//!
//! ```text
//! I: Bus=0003 Vendor=046d Product=c08b Version=0111
//! N: Name="Logitech G502 HERO Gaming Mouse"
//! H: Handlers=mouse1 event7
//! B: EV=17
//! B: KEY=ffff0000 0 0 0 0
//! B: REL=143
//! ```
//!
//! `B: KEY` lists the buttons and keys the device can send as a bitmap of hexadecimal words, the
//! highest word first. A mouse has `BTN_LEFT` and moves with `REL_X` and `REL_Y`; a touchpad or a
//! tablet moves with absolute axes and is left out. The extra buttons are `BTN_SIDE` (0x113) and the
//! codes after it. The X server numbers them 8, 9, 10... in that order (xf86-input-libinput), and
//! CursorPie's numbers are those minus 5 (3 = back, 4 = forward, 5 and up extra), see `mouse`.

use crate::mouse::{self, Device};

/// `BTN_LEFT`: every mouse has it.
const BTN_LEFT: usize = 0x110;
/// `BTN_SIDE`: the first extra button, X button 8, CursorPie's button 3 (back).
const BTN_SIDE: usize = 0x113;
/// The last code of the mouse group of `input-event-codes.h`.
const BTN_LAST: usize = 0x11f;
/// `REL_X` and `REL_Y`.
const REL_X: usize = 0;
const REL_Y: usize = 1;

/// Words of a device name that mark a program's pointer, not a mouse in a hand: they would be met as
/// a "new mouse" every time the program starts.
const VIRTUAL: [&str; 3] = ["virtual", "ydotool", "uinput"];

/// Whether bit `bit` is set in a kernel bitmap such as `ffff0000 0 0`: words of hexadecimal digits,
/// the highest word first. A word has as many bits as the kernel's `long` (64 on a 64-bit kernel).
fn bit_set(map: &str, bit: usize) -> bool {
    let words: Vec<&str> = map.split_whitespace().collect();
    let width = if words.iter().any(|w| w.len() > 8) {
        64
    } else {
        usize::BITS as usize
    };
    let Some(word) = words
        .len()
        .checked_sub(bit / width + 1)
        .and_then(|at| words.get(at))
    else {
        return false;
    };
    let Ok(value) = u64::from_str_radix(word, 16) else {
        return false;
    };
    ((value >> (bit % width)) & 1) == 1
}

/// The value of `key=` in a line such as `Bus=0003 Vendor=046d Product=c08b`.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let at = line.find(key)? + key.len();
    line[at..].split_whitespace().next()
}

/// The device of one block, if it is a mouse.
fn mouse_of(block: &str) -> Option<Device> {
    let (mut ids, mut name, mut handlers) = (None, None, "");
    let (mut key, mut rel) = ("", "");
    for line in block.lines() {
        if let Some(rest) = line.strip_prefix("I: ") {
            ids = Some(rest);
        } else if let Some(rest) = line.strip_prefix("N: Name=") {
            name = Some(rest.trim().trim_matches('"'));
        } else if let Some(rest) = line.strip_prefix("H: Handlers=") {
            handlers = rest;
        } else if let Some(rest) = line.strip_prefix("B: KEY=") {
            key = rest;
        } else if let Some(rest) = line.strip_prefix("B: REL=") {
            rel = rest;
        }
    }
    let name = name?;
    let is_mouse = handlers.split_whitespace().any(|h| h.starts_with("mouse"))
        && bit_set(key, BTN_LEFT)
        && bit_set(rel, REL_X)
        && bit_set(rel, REL_Y);
    let lower = name.to_ascii_lowercase();
    if !is_mouse || VIRTUAL.iter().any(|v| lower.contains(v)) {
        return None;
    }
    let hw_id = hardware_id(ids);
    let extra = (BTN_SIDE..=BTN_LAST)
        .filter(|code| bit_set(key, *code))
        .map(|code| mouse::BACK + (code - BTN_SIDE) as i64)
        .collect();
    Some(Device {
        name: name.to_string(),
        hw_id,
        extra,
    })
}

/// `046d:c08b` from the `I:` line of a device; empty when the line is missing or the ids are zero.
fn hardware_id(ids: Option<&str>) -> String {
    let Some(line) = ids else {
        return String::new();
    };
    let (Some(vendor), Some(product)) = (field(line, "Vendor="), field(line, "Product=")) else {
        return String::new();
    };
    if is_zero(vendor) && is_zero(product) {
        return String::new();
    }
    format!("{vendor}:{product}")
}

/// Whether a hexadecimal id is all zeros (devices that have no real ids, such as virtual ones).
fn is_zero(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c == '0')
}

/// The mice in the text of `/proc/bus/input/devices`.
pub fn parse_devices(text: &str) -> Vec<Device> {
    text.split("\n\n").filter_map(mouse_of).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A gaming mouse with seven extra buttons (BTN_SIDE to BTN_TASK and two more).
    const GAMING: &str = "\
I: Bus=0003 Vendor=046d Product=c08b Version=0111
N: Name=\"Logitech G502 HERO Gaming Mouse\"
P: Phys=usb-0000:00:14.0-2/input0
S: Sysfs=/devices/pci0000:00/0000:00:14.0/usb1/1-2/1-2:1.0/input/input12
U: Uniq=
H: Handlers=mouse1 event7
B: PROP=0
B: EV=17
B: KEY=ffff0000 0 0 0 0
B: REL=943
B: MSC=10
";

    /// A plain office mouse: left, right, middle, back and forward.
    const OFFICE: &str = "\
I: Bus=0003 Vendor=413c Product=301a Version=0111
N: Name=\"Dell MS116 USB Optical Mouse\"
H: Handlers=mouse0 event3
B: EV=17
B: KEY=1f0000 0 0 0 0
B: REL=903
";

    const KEYBOARD: &str = "\
I: Bus=0003 Vendor=046d Product=c31c Version=0110
N: Name=\"Logitech USB Keyboard\"
H: Handlers=sysrq kbd leds event4
B: EV=120013
B: KEY=1000000000007 ff9f207ac14057ff febeffdfffefffff fffffffffffffffe
B: MSC=10
B: LED=7
";

    const TOUCHPAD: &str = "\
I: Bus=0018 Vendor=04f3 Product=3140 Version=0100
N: Name=\"ELAN0412:00 04F3:3140 Touchpad\"
H: Handlers=mouse2 event9
B: EV=b
B: KEY=e520 10000 0 0 0 0
B: ABS=260800000000003
";

    fn blocks(parts: &[&str]) -> String {
        parts.join("\n")
    }

    #[test]
    fn bitmaps_read_the_highest_word_first() {
        // 0x1f0000: bits 16 to 20 of the last word
        assert!(bit_set("1f0000", 16));
        assert!(bit_set("1f0000", 20));
        assert!(!bit_set("1f0000", 21));
        // a bit of a higher word: bit 272 is bit 16 of the fifth word from the right (64-bit words)
        let map = "1f0000 0 0 0 0";
        if usize::BITS == 64 {
            assert!(bit_set(map, 0x110));
            assert!(bit_set(map, 0x114));
            assert!(!bit_set(map, 0x115));
        }
        // a map that is too short, or empty, has no such bit
        assert!(!bit_set("1f0000", 0x110));
        assert!(!bit_set("", 0));
        assert!(!bit_set("zz", 0));
    }

    #[test]
    fn a_mouse_is_found_with_its_ids_and_extra_buttons() {
        if usize::BITS != 64 {
            return; // the sample maps are those of a 64-bit kernel
        }
        let found = parse_devices(&blocks(&[GAMING, OFFICE]));
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "Logitech G502 HERO Gaming Mouse");
        assert_eq!(found[0].hw_id, "046d:c08b");
        // ffff0000 is bits 16 to 31 of the last word: BTN_LEFT 0x110 to 0x11f
        assert_eq!(
            found[0].extra,
            [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
        assert_eq!(found[1].hw_id, "413c:301a");
        assert_eq!(
            found[1].extra,
            [3, 4],
            "BTN_SIDE and BTN_EXTRA: back, forward"
        );
    }

    #[test]
    fn keyboards_and_touchpads_are_not_mice() {
        if usize::BITS != 64 {
            return;
        }
        assert!(parse_devices(&blocks(&[KEYBOARD, TOUCHPAD])).is_empty());
        // the mouse between them is still found
        let found = parse_devices(&blocks(&[KEYBOARD, OFFICE, TOUCHPAD]));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Dell MS116 USB Optical Mouse");
    }

    #[test]
    fn a_program_pretending_to_be_a_mouse_is_left_out() {
        if usize::BITS != 64 {
            return;
        }
        let fake = OFFICE.replace("Dell MS116 USB Optical Mouse", "ydotoold virtual device");
        assert!(parse_devices(&fake).is_empty());
    }

    #[test]
    fn a_device_without_real_ids_is_known_by_its_name() {
        if usize::BITS != 64 {
            return;
        }
        let none = OFFICE.replace("Vendor=413c Product=301a", "Vendor=0000 Product=0000");
        let found = parse_devices(&none);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].hw_id, "");
        assert_eq!(found[0].id(), "name:dell ms116 usb optical mouse");
    }

    #[test]
    fn garbage_gives_no_mice() {
        assert!(parse_devices("").is_empty());
        assert!(parse_devices("not a device list\n\nat all").is_empty());
        assert!(parse_devices("N: Name=\"x\"\nH: Handlers=mouse0\n").is_empty());
    }
}

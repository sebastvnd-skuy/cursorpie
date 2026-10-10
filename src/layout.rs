//! Keyboard-layout awareness (TIS + UCKeyTranslate).
//!
//! Virtual key codes are *physical positions*. What a position types depends on the active layout
//! (QWERTY, AZERTY, QWERTZ, Dvorak, Colemak, ISO/JIS...). This module builds a snapshot of the
//! current layout so the rest of the app can:
//!   * show correct labels for recorded shortcuts (`char_for`)
//!   * resolve "the key that types 'z'" for built-in presets (`find_key`)
//!
//! Threading: the TIS APIs must be called on the main thread (macOS 14+ asserts otherwise). Only
//! `init()` / `refresh()` touch TIS, and both are no-ops off the main thread. Everything else reads
//! the cached snapshot, so it is safe to call from any thread.

use crate::keys::{MOD_ALT, MOD_SHIFT};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, RwLock};

type CFTypeRef = *const c_void;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyUnicodeKeyLayoutData: CFTypeRef;
    static kTISPropertyInputSourceIsASCIICapable: CFTypeRef;
    static kTISNotifySelectedKeyboardInputSourceChanged: CFTypeRef;

    fn TISCopyCurrentKeyboardInputSource() -> CFTypeRef;
    fn TISCopyCurrentASCIICapableKeyboardLayoutInputSource() -> CFTypeRef;
    fn TISGetInputSourceProperty(source: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
    fn LMGetKbdType() -> u8;
    fn UCKeyTranslate(
        layout: *const u8,
        virtual_key_code: u16,
        key_action: u16,
        modifier_key_state: u32,
        keyboard_type: u32,
        key_translate_options: u32,
        dead_key_state: *mut u32,
        max_string_length: usize,
        actual_string_length: *mut usize,
        unicode_string: *mut u16,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: CFTypeRef);
    fn CFDataGetBytePtr(data: CFTypeRef) -> *const u8;
    fn CFBooleanGetValue(boolean: CFTypeRef) -> u8;
    fn CFNotificationCenterGetDistributedCenter() -> CFTypeRef;
    fn CFNotificationCenterAddObserver(
        center: CFTypeRef,
        observer: *const c_void,
        callback: extern "C" fn(CFTypeRef, *mut c_void, CFTypeRef, CFTypeRef, CFTypeRef),
        name: CFTypeRef,
        object: CFTypeRef,
        suspension_behavior: isize,
    );
}

unsafe extern "C" {
    fn pthread_main_np() -> i32;
}

/// Carbon modifier bits as UCKeyTranslate wants them: (EventModifiers >> 8) & 0xFF.
const UC_SHIFT: u32 = 0x02; // shiftKey  (0x0200) >> 8
const UC_OPTION: u32 = 0x08; // optionKey (0x0800) >> 8
const UC_ACTION_DOWN: u16 = 0; // kUCKeyActionDown
const UC_NO_DEAD_KEYS: u32 = 1; // kUCKeyTranslateNoDeadKeysMask (the *Bit* constant is 0, don't pass that)
const NO_ERR: i32 = 0; // noErr

/// Snapshot of one layout.
struct Table {
    /// chars[keycode][variant], variant bit0 = Shift, bit1 = Option.
    chars: Vec<[Option<String>; 4]>,
    /// text -> (keycode, MOD_* bits needed to type it). Lowest-modifier combination wins.
    reverse: HashMap<String, (u16, u8)>,
}

static TABLE: RwLock<Option<Arc<Table>>> = RwLock::new(None);

fn is_main_thread() -> bool {
    unsafe { pthread_main_np() != 0 }
}

/// Keys whose meaning does not depend on the layout (named statically in keys.rs).
fn is_layout_key(code: u16) -> bool {
    code < 128 && crate::keys::special_name(code).is_none()
}

unsafe fn translate(layout: *const u8, kbd: u32, code: u16, uc_mods: u32) -> Option<String> {
    let mut dead = 0u32;
    let mut buf = [0u16; 8];
    let mut len = 0usize;
    let status = unsafe {
        UCKeyTranslate(
            layout,
            code,
            UC_ACTION_DOWN,
            uc_mods,
            kbd,
            UC_NO_DEAD_KEYS,
            &mut dead,
            buf.len(),
            &mut len,
            buf.as_mut_ptr(),
        )
    };
    if status != NO_ERR || len == 0 {
        return None;
    }
    let s = String::from_utf16(&buf[..len.min(buf.len())]).ok()?;
    if s.chars().any(|c| c.is_control()) {
        return None; // Return/Tab/arrows etc. come back as control characters
    }
    Some(s)
}

fn build() -> Option<Table> {
    unsafe {
        let mut src = TISCopyCurrentKeyboardInputSource();
        if src.is_null() {
            return None;
        }

        // Shortcuts on a non-Latin layout (Cyrillic, Arabic, ...) use the Latin fallback layout,
        // and input methods (Japanese, Chinese...) have no layout data at all.
        let v = TISGetInputSourceProperty(src, kTISPropertyInputSourceIsASCIICapable);
        let ascii = !v.is_null() && CFBooleanGetValue(v) != 0;
        let mut data: CFTypeRef = std::ptr::null();
        if ascii {
            data = TISGetInputSourceProperty(src, kTISPropertyUnicodeKeyLayoutData);
        }
        if data.is_null() {
            CFRelease(src);
            src = TISCopyCurrentASCIICapableKeyboardLayoutInputSource();
            if src.is_null() {
                return None;
            }
            data = TISGetInputSourceProperty(src, kTISPropertyUnicodeKeyLayoutData);
        }
        if data.is_null() {
            CFRelease(src);
            return None;
        }

        let layout = CFDataGetBytePtr(data);
        let kbd = LMGetKbdType() as u32;

        let mut chars: Vec<[Option<String>; 4]> = vec![Default::default(); 128];
        for code in 0u16..128 {
            if !is_layout_key(code) {
                continue;
            }
            for variant in 0..4usize {
                let mut m = 0;
                if variant & 1 != 0 {
                    m |= UC_SHIFT;
                }
                if variant & 2 != 0 {
                    m |= UC_OPTION;
                }
                chars[code as usize][variant] = translate(layout, kbd, code, m);
            }
        }
        CFRelease(src); // `layout` points into `src`; not used past this point

        // Reverse map. Outer loop = variant so "no modifier" beats "Shift" beats "Option".
        let mut reverse: HashMap<String, (u16, u8)> = HashMap::new();
        for variant in 0..4usize {
            let mut mods = 0u8;
            if variant & 1 != 0 {
                mods |= MOD_SHIFT;
            }
            if variant & 2 != 0 {
                mods |= MOD_ALT;
            }
            for code in 0u16..128 {
                if let Some(s) = &chars[code as usize][variant] {
                    reverse.entry(s.clone()).or_insert((code, mods));
                }
            }
        }
        Some(Table { chars, reverse })
    }
}

extern "C" fn on_layout_changed(_c: CFTypeRef, _o: *mut c_void, _n: CFTypeRef, _obj: CFTypeRef, _u: CFTypeRef) {
    refresh();
}

/// Rebuild the snapshot. Main thread only (no-op elsewhere).
pub fn refresh() {
    if !is_main_thread() {
        return;
    }
    let t = build().map(Arc::new);
    if let Ok(mut g) = TABLE.write() {
        *g = t;
    }
}

/// Call once on the main thread (inside the eframe creation closure): builds the snapshot and
/// keeps it fresh when the user switches input source.
pub fn init() {
    refresh();
    static TOKEN: u8 = 0;
    unsafe {
        CFNotificationCenterAddObserver(
            CFNotificationCenterGetDistributedCenter(),
            &TOKEN as *const u8 as *const c_void,
            on_layout_changed,
            kTISNotifySelectedKeyboardInputSourceChanged,
            std::ptr::null(),
            4, // CFNotificationSuspensionBehaviorDeliverImmediately
        );
    }
}

fn table() -> Option<Arc<Table>> {
    if let Ok(g) = TABLE.read()
        && let Some(t) = g.as_ref()
    {
        return Some(t.clone());
    }
    if is_main_thread() {
        refresh(); // lazy build if init() has not run yet
        return TABLE.read().ok()?.clone();
    }
    None
}

/// Text typed by physical key `code` on the current layout (None if it is not a layout key).
pub fn char_for(code: u16, shift: bool) -> Option<String> {
    let t = table()?;
    t.chars.get(code as usize)?[shift as usize].clone()
}

/// Physical key that types `text` on the current layout, plus the MOD_* bits it needs
/// (e.g. `[` on a German layout needs Option).
pub fn find_key(text: &str) -> Option<(u16, u8)> {
    table()?.reverse.get(text).copied()
}

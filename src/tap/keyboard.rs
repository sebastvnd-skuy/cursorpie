//! Key events: only used to record a shortcut for the settings window.

use super::{RECORDING, SWALLOW_KEY};
use crate::keys::{Shortcut, mods_from_flags};
use crate::msg::{Msg, Notifier};
use core_graphics::event::{CGEvent, CGEventType, CallbackResult, EventField};
use std::sync::atomic::Ordering;

/// Captures the next key press while Settings is recording a shortcut (plain Esc cancels).
/// The key is swallowed until its key-up so it does not leak to the app in front. Outside
/// recording every key event passes through untouched.
pub(super) fn handle_key(
    notifier: &Notifier,
    etype: CGEventType,
    event: &CGEvent,
) -> CallbackResult {
    let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as i32;
    let swallow = SWALLOW_KEY.load(Ordering::SeqCst);

    if matches!(etype, CGEventType::KeyUp) {
        if swallow == code {
            SWALLOW_KEY.store(-1, Ordering::SeqCst);
            return CallbackResult::Drop;
        }
        return CallbackResult::Keep;
    }

    // KeyDown (including auto-repeat of the key that was just recorded)
    if swallow == code {
        return CallbackResult::Drop;
    }
    if !RECORDING.load(Ordering::SeqCst) {
        return CallbackResult::Keep;
    }

    RECORDING.store(false, Ordering::SeqCst);
    SWALLOW_KEY.store(code, Ordering::SeqCst);
    let mods = mods_from_flags(event.get_flags());
    if code == 53 && mods == 0 {
        notifier.send(Msg::RecordCancelled); // plain Esc cancels
    } else {
        notifier.send(Msg::KeyCaptured(Shortcut::new(code as u16, mods)));
    }
    CallbackResult::Drop
}

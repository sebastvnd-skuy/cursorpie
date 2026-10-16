//! One running copy at a time.
//!
//! Two copies would each install their own mouse hook, so every gesture would fire twice. This
//! happens easily once the app starts at login: the user double-clicks the shortcut while the tray
//! icon is already there.

use super::util::wide;
use crate::about::APP_NAME;
use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows_sys::Win32::System::Threading::CreateMutexW;

/// Claims the right to run. Returns `false` when another copy of this app already runs in this
/// user session; the caller should exit quietly.
pub fn single_instance() -> bool {
    // `Local\` keeps the name per session, so two users on one PC do not block each other.
    let name = wide(&format!(r"Local\{APP_NAME}-single-instance"));
    // SAFETY: `name` is zero terminated; null security attributes are allowed.
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return true; // cannot tell: running twice is better than not running
    }
    // SAFETY: plain call, made right after the one that sets the error.
    let taken = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    // The handle is deliberately never closed: the mutex must live as long as the process does.
    !taken
}

//! "Launch at login" through the per-user `Run` registry key.
//!
//! The entry always points at the installed copy (see [`install`](super::install)), never at
//! wherever the program happened to be started from: a download folder or an unzipped archive can
//! be moved or emptied, and then nothing would start at the next login.

use super::install::startup_exe;
use super::registry;
use super::util::{STARTUP_ENABLED, run_value, startup_entry_disabled};
use crate::about::APP_NAME;

/// The key every per-user startup entry lives in.
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Where Windows keeps the "switched off in Task Manager" flag of each startup entry.
const APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

/// Whether CursorPie starts with Windows: the entry exists and the user has not switched it off
/// in Task Manager (Windows then keeps the entry but never runs it).
pub fn launch_at_login_enabled() -> bool {
    if !registry::has_value(RUN_KEY, APP_NAME) {
        return false;
    }
    !registry::get_bytes(APPROVED_KEY, APP_NAME).is_some_and(|d| startup_entry_disabled(&d))
}

/// Adds or removes the startup entry. It takes effect at the next login (no second copy is started now).
///
/// Turning it on also clears a "disabled" flag left by Task Manager, otherwise the checkbox would
/// say yes while Windows still skipped the entry.
pub fn set_launch_at_login(on: bool) -> std::io::Result<()> {
    if !on {
        return registry::delete_value(RUN_KEY, APP_NAME);
    }
    let exe = startup_exe(&std::env::current_exe()?);
    registry::set_string(RUN_KEY, APP_NAME, &run_value(&exe.to_string_lossy()))?;
    // An explicit "enabled" flag, so that an old "disabled" flag left by Task Manager is overridden.
    registry::set_binary(APPROVED_KEY, APP_NAME, &STARTUP_ENABLED)
}

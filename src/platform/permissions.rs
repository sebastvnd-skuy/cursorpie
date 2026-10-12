//! Accessibility permission (needed for the event tap) and shortcuts to the System Settings pages.

use super::shell::spawn_reaped;
use std::process::Command;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

/// Whether this process has the Accessibility permission (needed for the event tap).
pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

/// Opens System Settings at Privacy > Accessibility.
pub fn open_accessibility_settings() {
    open_raw("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility");
}

/// Opens System Settings at Privacy > Input Monitoring.
pub fn open_input_monitoring_settings() {
    open_raw("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent");
}

/// Opens a URL (here: a System Settings pane) with `open`.
fn open_raw(target: &str) {
    spawn_reaped(Command::new("open").arg(target));
}

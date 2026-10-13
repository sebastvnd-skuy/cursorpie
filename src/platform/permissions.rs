//! Accessibility permission (needed for the event tap) and shortcuts to the System Settings pages.

use super::shell::spawn_reaped;
use crate::config::lock;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

/// Whether this process has the Accessibility permission (needed for the event tap).
pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

/// How long [`is_trusted_cached`] reuses an answer.
const TRUST_TTL: Duration = Duration::from_secs(1);

/// The last answer of [`is_trusted`] and when it was asked.
static TRUST: Mutex<Option<(Instant, bool)>> = Mutex::new(None);

/// [`is_trusted`], at most once per second. Settings asks on every frame to draw its status bar
/// and the General tab; the permission cannot change faster than a person can click, so the frame
/// loop should not call into the system that often.
pub fn is_trusted_cached() -> bool {
    let mut last = lock(&TRUST);
    if let Some((at, value)) = *last
        && at.elapsed() < TRUST_TTL
    {
        return value;
    }
    let value = is_trusted();
    *last = Some((Instant::now(), value));
    value
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

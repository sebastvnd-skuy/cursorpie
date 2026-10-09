//! Thin wrappers around macOS: permissions, frontmost app, launch-at-login, file pickers.

use crate::about::BUNDLE_ID;
use crate::config::home;
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

// ------------------------------------------------------------- accessibility

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

/// Whether this process has the Accessibility permission (needed for the event tap).
pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

pub fn open_accessibility_settings() {
    open_raw("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility");
}

pub fn open_input_monitoring_settings() {
    open_raw("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent");
}

fn open_raw(target: &str) {
    Command::new("open").arg(target).spawn().ok();
}

// ------------------------------------------------------------ frontmost app

/// Bundle identifier of the frontmost application (the key used by per-app profiles).
/// NSWorkspace can be read from any thread; the pool drains temporaries when called off the main thread.
pub fn frontmost_bundle_id() -> Option<String> {
    use objc2::rc::autoreleasepool;
    use objc2_app_kit::NSWorkspace;

    autoreleasepool(|_| {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        app.bundleIdentifier().map(|s| s.to_string())
    })
}

/// Hides the Dock icon / app switcher entry (same effect as LSUIElement, also when run via `cargo run`).
pub fn set_accessory_policy() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    if let Some(mtm) = MainThreadMarker::new() {
        let app = NSApplication::sharedApplication(mtm);
        let _ = app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    }
}

pub fn cursor_pos() -> Option<(f64, f64)> {
    let src = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()?;
    let p = CGEvent::new(src).ok()?.location();
    Some((p.x, p.y))
}

// ----------------------------------------------------------- launch at login

fn agent_path() -> PathBuf {
    home().join("Library").join("LaunchAgents").join(format!("{BUNDLE_ID}.plist"))
}

pub fn launch_at_login_enabled() -> bool {
    agent_path().exists()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Installs / removes a LaunchAgent. It takes effect at the next login (we don't start a second copy now).
pub fn set_launch_at_login(on: bool) -> std::io::Result<()> {
    let path = agent_path();
    if !on {
        return match fs::remove_file(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        };
    }
    let exe = std::env::current_exe()?;
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>{label}</string>
    <key>ProgramArguments</key>
    <array><string>{exe}</string></array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><false/>
    <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
"#,
        label = BUNDLE_ID,
        exe = xml_escape(&exe.to_string_lossy()),
    );
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, plist)
}

// ------------------------------------------------------------------- helpers

/// Native file chooser through osascript (no extra dependency). `uti` e.g. "public.image".
pub fn choose_file(uti: &str, prompt: &str) -> Option<String> {
    let script = format!(r#"POSIX path of (choose file of type {{"{uti}"}} with prompt "{prompt}")"#);
    let out = Command::new("osascript").arg("-e").arg(script).output().ok()?;
    if !out.status.success() {
        return None; // cancelled
    }
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!p.is_empty()).then_some(p)
}

/// CFBundleIdentifier of an .app bundle, via Spotlight metadata.
pub fn bundle_id_of(app_path: &str) -> Option<String> {
    let out = Command::new("mdls")
        .args(["-name", "kMDItemCFBundleIdentifier", "-raw", app_path])
        .output()
        .ok()?;
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!id.is_empty() && id != "(null)").then_some(id)
}

pub fn reveal_in_finder(path: &std::path::Path) {
    Command::new("open").arg("-R").arg(path).spawn().ok();
}

/// Soft tick used as hover feedback (afplay ships with macOS).
pub fn tick() {
    Command::new("afplay")
        .args(["-v", "0.25", "/System/Library/Sounds/Tink.aiff"])
        .spawn()
        .ok();
}

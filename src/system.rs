//! Thin wrappers around macOS: permissions, frontmost app, launch-at-login, file pickers.

use crate::about::BUNDLE_ID;
use crate::config::home;
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
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
    spawn_reaped(Command::new("open").arg(target));
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

/// Bundle identifier of the app that owns the topmost window at `p` (global coordinates, the same
/// space as the cursor), whether or not that app is in front. Only regular app windows count: not
/// the Dock, the menu bar or the desktop, and CursorPie's own windows are skipped. None when
/// nothing suitable is under the point or the app has no bundle identifier.
pub fn bundle_id_at(p: CGPoint) -> Option<String> {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::geometry::CGRect;
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowAlpha, kCGWindowBounds, kCGWindowLayer,
        kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly, kCGWindowOwnerPID,
    };

    // ordered front to back
    let windows = copy_window_info(
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
        kCGNullWindowID,
    )?;
    let (k_layer, k_alpha, k_pid, k_bounds) = unsafe {
        (
            CFString::wrap_under_get_rule(kCGWindowLayer),
            CFString::wrap_under_get_rule(kCGWindowAlpha),
            CFString::wrap_under_get_rule(kCGWindowOwnerPID),
            CFString::wrap_under_get_rule(kCGWindowBounds),
        )
    };
    let me = std::process::id() as i64;

    for w in windows.iter() {
        let d: CFDictionary<CFString, CFType> =
            unsafe { CFDictionary::wrap_under_get_rule(*w as CFDictionaryRef) };
        let number = |k: &CFString| d.find(k).and_then(|v| v.downcast::<CFNumber>());

        // layer 0 = normal windows; up to 19 also covers floating palettes and panels
        let Some(layer) = number(&k_layer).and_then(|n| n.to_i64()) else {
            continue;
        };
        if !(0..=19).contains(&layer) {
            continue;
        }
        // fully transparent overlays do not count as "the window under the pointer"
        if number(&k_alpha)
            .and_then(|n| n.to_f64())
            .is_some_and(|a| a < 0.01)
        {
            continue;
        }
        let Some(pid) = number(&k_pid).and_then(|n| n.to_i64()) else {
            continue;
        };
        if pid == me {
            continue;
        }
        let Some(bounds) = d
            .find(&k_bounds)
            .and_then(|v| v.downcast::<CFDictionary>())
            .and_then(|b| CGRect::from_dict_representation(&b))
        else {
            continue;
        };
        if bounds.contains(&p) {
            return bundle_id_of_pid(pid as i32);
        }
    }
    None
}

fn bundle_id_of_pid(pid: i32) -> Option<String> {
    use objc2::rc::autoreleasepool;
    use objc2_app_kit::NSRunningApplication;

    autoreleasepool(|_| {
        let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)?;
        app.bundleIdentifier().map(|s| s.to_string())
    })
}

/// The icon of an app as TIFF data holding every size macOS has for it (one page per size), found by
/// bundle identifier. This is the real icon, the one the Dock and Finder show. None if no such app is
/// installed. Meant for the main thread; the result is decoded by `appicon`.
pub fn app_icon_tiff(bundle: &str) -> Option<Vec<u8>> {
    use objc2::rc::autoreleasepool;
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::NSString;

    autoreleasepool(|_| {
        let ws = NSWorkspace::sharedWorkspace();
        let url = ws.URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle))?;
        let path = url.path()?;
        let icon = ws.iconForFile(&path);
        let data = icon.TIFFRepresentation()?;
        Some(data.to_vec())
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
    home()
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{BUNDLE_ID}.plist"))
}

pub fn launch_at_login_enabled() -> bool {
    agent_path().exists()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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

/// Menjalankan proses lalu "memanen" statusnya di thread kecil, supaya tidak jadi zombie.
pub fn spawn_reaped(cmd: &mut Command) {
    if let Ok(mut child) = cmd.spawn() {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

/// Native file chooser through osascript (no extra dependency). `uti` e.g. "public.image".
pub fn choose_file(uti: &str, prompt: &str) -> Option<String> {
    let script =
        format!(r#"POSIX path of (choose file of type {{"{uti}"}} with prompt "{prompt}")"#);
    let out = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
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
    spawn_reaped(Command::new("open").arg("-R").arg(path));
}

/// Soft tick used as hover feedback (afplay ships with macOS).
pub fn tick() {
    spawn_reaped(Command::new("afplay").args(["-v", "0.25", "/System/Library/Sounds/Tink.aiff"]));
}

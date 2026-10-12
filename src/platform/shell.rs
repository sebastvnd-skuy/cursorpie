//! Helpers that shell out to tools shipped with macOS (`open`, `osascript`, `mdls`, `afplay`).

use std::process::Command;

/// Starts a process and reaps its exit status on a tiny thread, so it never becomes a zombie.
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

/// Reveals a file in Finder.
pub fn reveal_in_finder(path: &std::path::Path) {
    spawn_reaped(Command::new("open").arg("-R").arg(path));
}

/// Soft tick used as hover feedback (afplay ships with macOS).
pub fn tick() {
    spawn_reaped(Command::new("afplay").args(["-v", "0.25", "/System/Library/Sounds/Tink.aiff"]));
}

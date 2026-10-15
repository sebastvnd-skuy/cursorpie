//! "Launch at login" through a per-user LaunchAgent.

use crate::about::BUNDLE_ID;
use crate::config::home;
use std::fs;
use std::path::PathBuf;

/// Where the LaunchAgent plist lives: `~/Library/LaunchAgents/<bundle id>.plist`.
fn agent_path() -> PathBuf {
    home()
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{BUNDLE_ID}.plist"))
}

/// Whether the LaunchAgent is installed.
pub fn launch_at_login_enabled() -> bool {
    agent_path().exists()
}

/// Escapes text for use inside the plist XML.
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

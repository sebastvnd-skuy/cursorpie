//! The one way the tools talk to Git: run the `git` program in the repository and read its output.

use std::path::Path;
use std::process::Command;

/// Runs `git` in `root` and returns its output without the trailing newline.
///
/// # Errors
/// When `git` cannot be started or exits with a failure; the message holds what `git` printed.
pub fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("git {}: {}", args.join(" "), err.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

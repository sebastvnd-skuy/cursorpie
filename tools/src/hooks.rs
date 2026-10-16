//! `cargo tools hooks install|uninstall`: turns the Git hooks of `.githooks/` on or off.
//!
//! The hooks are plain scripts kept in the repository, so every contributor gets the same ones
//! and they change through pull requests like any other file. Git only runs them after
//! `core.hooksPath` points at the folder, which is a per-clone setting and the one thing
//! `install` does. Each hook calls a tool, so the rules live in Rust (tested), not in shell:
//!
//! | Hook | Runs | Takes |
//! |---|---|---|
//! | `pre-commit` | `cargo tools ci --quick` | seconds |
//! | `commit-msg` | `cargo tools commit-msg FILE` | instant |
//! | `pre-push` | `cargo tools ci` | minutes, like CI |
//!
//! The hooks are a convenience: `--no-verify` skips them once, and CI runs the same checks and
//! has the last word.

use std::path::Path;
use std::process::ExitCode;

use crate::git::git;

/// The folder with the hook scripts, relative to the repository root.
const HOOKS_DIR: &str = ".githooks";
/// The Git setting that makes Git look for hooks in a folder of the repository.
const SETTING: &str = "core.hooksPath";

/// What to do with the hooks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Point `core.hooksPath` at `.githooks/`.
    Install,
    /// Remove that setting again.
    Uninstall,
}

/// Reads the arguments after `hooks`.
///
/// # Errors
/// When the action is missing, unknown, or followed by more words.
pub fn parse_args(args: &[String]) -> Result<Action, String> {
    match args {
        [word] if word == "install" => Ok(Action::Install),
        [word] if word == "uninstall" => Ok(Action::Uninstall),
        _ => Err("say hooks install or hooks uninstall".to_string()),
    }
}

/// Makes a hook script executable, which a zip file or a Windows checkout may have lost. Only
/// Unix has such a permission.
#[cfg(unix)]
fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    std::fs::set_permissions(path, permissions)
}

/// Makes a hook script executable: nothing to do outside Unix.
#[cfg(not(unix))]
fn make_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Turns the hooks on or off for the clone at `root` and says what it did.
///
/// # Errors
/// When `.githooks/` is missing or a `git` or file call fails. `uninstall` refuses to remove a
/// `core.hooksPath` that points somewhere else, because that setting is not ours.
pub fn run(root: &Path, action: Action) -> Result<ExitCode, String> {
    match action {
        Action::Install => {
            let dir = root.join(HOOKS_DIR);
            let entries = std::fs::read_dir(&dir)
                .map_err(|e| format!("{HOOKS_DIR}/ cannot be read in {}: {e}", root.display()))?;
            for entry in entries.flatten() {
                make_executable(&entry.path())
                    .map_err(|e| format!("{}: {e}", entry.path().display()))?;
            }
            git(root, &["config", SETTING, HOOKS_DIR])?;
            println!(
                "Git hooks are on for this clone: pre-commit (quick checks), commit-msg and \
                 pre-push (all checks).\n\
                 `cargo tools hooks uninstall` turns them off; `--no-verify` skips them once."
            );
        }
        Action::Uninstall => match git(root, &["config", "--get", SETTING]) {
            Ok(path) if path == HOOKS_DIR => {
                git(root, &["config", "--unset", SETTING])?;
                println!("Git hooks are off for this clone.");
            }
            Ok(other) => {
                return Err(format!(
                    "{SETTING} is '{other}', not '{HOOKS_DIR}': it was not set by this tool, so \
                     it is left alone"
                ));
            }
            Err(_) => println!("Git hooks were not on for this clone."),
        },
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_action_is_read() {
        assert_eq!(parse_args(&words(&["install"])), Ok(Action::Install));
        assert_eq!(parse_args(&words(&["uninstall"])), Ok(Action::Uninstall));
        assert!(parse_args(&words(&[])).is_err());
        assert!(parse_args(&words(&["enable"])).is_err());
        assert!(parse_args(&words(&["install", "now"])).is_err());
    }
}

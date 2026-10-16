//! First launch detection: decides whether the welcome window is shown.
//!
//! The welcome window appears once per user. "Once" is remembered by an empty marker file,
//! `welcome_shown`, in the app support directory (next to `config.json`). A copy that finds a
//! `config.json` but no marker belongs to someone who used CursorPie before the welcome window
//! existed, so it is not shown to them: the marker is written instead. The tray menu item
//! "Welcome guide..." opens the window again at any time.

use super::app_support_dir;
use std::fs;
use std::path::Path;

/// Name of the marker file inside the app support directory.
pub const WELCOME_MARKER: &str = "welcome_shown";

/// What a start of the app should do about the welcome window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirstRun {
    /// A brand-new user: show the welcome window.
    Show,
    /// Used CursorPie before the welcome window existed: stay quiet, but remember it.
    ExistingUser,
    /// Already welcomed.
    Done,
}

/// The decision, from the two facts it depends on. Pure, so it is unit tested.
pub fn decide(marker_exists: bool, config_exists: bool) -> FirstRun {
    match (marker_exists, config_exists) {
        (true, _) => FirstRun::Done,
        (false, true) => FirstRun::ExistingUser,
        (false, false) => FirstRun::Show,
    }
}

/// Reads the state of `dir` (an app support directory) and decides. Writes the marker for an
/// existing user, so the answer is `Done` from then on.
pub fn check_in(dir: &Path) -> FirstRun {
    let marker = dir.join(WELCOME_MARKER);
    let verdict = decide(marker.exists(), dir.join("config.json").exists());
    if verdict == FirstRun::ExistingUser {
        mark_welcomed_in(dir);
    }
    verdict
}

/// Writes the marker into `dir`. Errors are ignored: the worst case is one more welcome window.
pub fn mark_welcomed_in(dir: &Path) {
    let _ = fs::write(dir.join(WELCOME_MARKER), b"");
}

/// Whether the welcome window should open now (see `check_in`). Call it once, at start-up,
/// before anything saves `config.json`.
pub fn welcome_due() -> bool {
    check_in(&app_support_dir()) == FirstRun::Show
}

/// Remembers that the user has seen the welcome window.
pub fn mark_welcomed() {
    mark_welcomed_in(&app_support_dir());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty directory unique to this test.
    fn temp_dir(name: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("cursorpie-first-run-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn decision_table() {
        assert_eq!(decide(false, false), FirstRun::Show);
        assert_eq!(decide(false, true), FirstRun::ExistingUser);
        assert_eq!(decide(true, false), FirstRun::Done);
        assert_eq!(decide(true, true), FirstRun::Done);
    }

    #[test]
    fn a_new_user_is_welcomed_until_the_marker_is_written() {
        let d = temp_dir("new");
        assert_eq!(check_in(&d), FirstRun::Show);
        assert_eq!(check_in(&d), FirstRun::Show, "asking does not use it up");
        mark_welcomed_in(&d);
        assert_eq!(check_in(&d), FirstRun::Done);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn an_existing_user_is_skipped_and_remembered() {
        let d = temp_dir("old");
        fs::write(d.join("config.json"), "{}").unwrap();
        assert_eq!(check_in(&d), FirstRun::ExistingUser);
        assert!(d.join(WELCOME_MARKER).exists());
        assert_eq!(check_in(&d), FirstRun::Done);
        let _ = fs::remove_dir_all(&d);
    }
}

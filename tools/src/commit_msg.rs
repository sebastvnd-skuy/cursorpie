//! `cargo tools commit-msg FILE`: the commit message rules, called by the `commit-msg` hook of
//! `.githooks/`.
//!
//! The release tool turns commit subjects into changelog lines when nobody wrote notes (see
//! `bump`), so the subject is written for a reader of the changelog. The rules are few and
//! mechanical, so a machine can check them and a reviewer does not have to:
//!
//! - the subject is between `MIN_LEN` and `MAX_LEN` characters;
//! - it starts with a capital letter and does not end with a full stop;
//! - it starts with an order (`Fix`, `Add`), not with a log word (`Fixed`, `Adding`, `WIP`);
//! - a blank line separates the subject from the body.
//!
//! Merges, reverts, `Release ...` commits and `fixup!`, `squash!` and `amend!` commits are not
//! checked: Git or the release tool writes them. The check is local; `git commit --no-verify`
//! skips it once.

use std::path::Path;
use std::process::ExitCode;

/// The shortest subject that can say what changed.
const MIN_LEN: usize = 10;
/// The longest subject that fits on one line of `git log --oneline` and of a changelog.
const MAX_LEN: usize = 72;

/// Subjects that Git or the release tool write themselves.
const EXEMPT_PREFIXES: &[&str] = &[
    "Merge ", "Revert ", "Release ", "fixup! ", "squash! ", "amend! ",
];

/// First words (lower case) that read as a log entry instead of an order. Only exact words are
/// listed, so a word like `Embed` or `Bring` is never taken for a past tense by mistake.
const NOT_AN_ORDER: &[&str] = &[
    "fixed", "fixes", "fixing", "added", "adds", "adding", "updated", "updates", "updating",
    "removed", "removes", "removing", "changed", "changes", "changing", "wip",
];

/// What is wrong with a commit message, one line each. Empty means the message is fine.
///
/// Lines that start with `#` are comments for Git and are ignored, as Git itself ignores them.
fn check_message(message: &str) -> Vec<String> {
    let lines: Vec<&str> = message.lines().filter(|l| !l.starts_with('#')).collect();
    let Some(start) = lines.iter().position(|l| !l.trim().is_empty()) else {
        return vec!["the commit message is empty".to_string()];
    };
    let subject = lines[start].trim();
    if EXEMPT_PREFIXES.iter().any(|p| subject.starts_with(p)) {
        return Vec::new();
    }

    let mut problems = Vec::new();
    let len = subject.chars().count();
    if len < MIN_LEN {
        problems.push(format!(
            "the subject has {len} characters: write at least {MIN_LEN}, enough to say what changed"
        ));
    }
    if len > MAX_LEN {
        problems.push(format!(
            "the subject has {len} characters: keep it to {MAX_LEN} and put the details in the body"
        ));
    }
    if !subject.chars().next().is_some_and(char::is_uppercase) {
        problems.push("start the subject with a capital letter".to_string());
    }
    if subject.ends_with('.') {
        problems.push("do not end the subject with a full stop".to_string());
    }
    let first = subject
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(':')
        .to_lowercase();
    if NOT_AN_ORDER.contains(&first.as_str()) {
        problems.push(format!(
            "write the subject as an order ('Fix ...', 'Add ...'), not '{first}': \
             it can become a line of the changelog"
        ));
    }
    if lines.get(start + 1).is_some_and(|l| !l.trim().is_empty()) {
        problems.push("leave a blank line between the subject and the body".to_string());
    }
    problems
}

/// Reads the message in `file` and prints what is wrong with it. Success means the message is
/// fine, so Git goes on with the commit.
///
/// # Errors
/// When the file cannot be read.
pub fn run(file: &Path) -> Result<ExitCode, String> {
    let message = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let problems = check_message(&message);
    if problems.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    eprintln!("The commit message was rejected:");
    for problem in &problems {
        eprintln!("  - {problem}");
    }
    eprintln!(
        "\nThe rules are in CONTRIBUTING.md, \"Commit messages\". Fix the message and commit again \
         (`git commit --no-verify` skips this check once)."
    );
    Ok(ExitCode::FAILURE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problems(message: &str) -> usize {
        check_message(message).len()
    }

    #[test]
    fn a_clear_imperative_subject_passes() {
        assert_eq!(problems("Fix double-click replay on slow clicks"), 0);
        assert_eq!(
            problems("Add a sidebar to Settings\n\nWhy: the tabs ran out of room.\n"),
            0
        );
        assert_eq!(
            problems("Embed the logo as SVG"),
            0,
            "only exact log words are refused"
        );
    }

    #[test]
    fn length_case_and_full_stop_are_checked() {
        assert_eq!(problems("Fix bug"), 1, "too short");
        assert_eq!(problems(&format!("Fix {}", "x".repeat(80))), 1, "too long");
        assert_eq!(problems("fix the double-click replay"), 1, "capital letter");
        assert_eq!(problems("Fix the double-click replay."), 1, "full stop");
    }

    #[test]
    fn log_words_are_refused_as_the_first_word() {
        for subject in [
            "Fixed the double-click replay",
            "Adding a sidebar to Settings",
            "WIP: the sidebar",
            "Updates the icons",
        ] {
            assert_eq!(problems(subject), 1, "{subject}");
        }
    }

    #[test]
    fn the_body_starts_after_a_blank_line() {
        assert_eq!(
            problems("Fix the double-click replay\nthe body sticks to it"),
            1
        );
        assert_eq!(
            problems("Fix the double-click replay\n\nthe body is apart"),
            0
        );
    }

    #[test]
    fn git_comments_and_leading_blank_lines_are_ignored() {
        let message = "\n# Please enter the commit message\nFix the double-click replay\n# more\n";
        assert_eq!(problems(message), 0);
    }

    #[test]
    fn an_empty_message_is_refused() {
        assert_eq!(problems(""), 1);
        assert_eq!(problems("# only a comment\n"), 1);
    }

    #[test]
    fn generated_subjects_are_not_checked() {
        for subject in [
            "Merge branch 'main' into fix/x",
            "Revert \"Add a sidebar\"",
            "Release 0.4.0",
            "fixup! Fix the replay",
        ] {
            assert_eq!(problems(subject), 0, "{subject}");
        }
    }
}

//! Running a native file chooser that is a separate program (`zenity`, `kdialog`, `osascript`).

use std::process::{Command, Stdio};
use std::time::Duration;

/// Runs the chooser `cmd` and returns the path it printed. `None` if it was cancelled, failed or
/// printed nothing.
pub fn run(cmd: &mut Command) -> Option<String> {
    run_with(cmd, ANSWER_WAIT)
}

/// How long to wait for the answer after the chooser has ended (it is written before it ends).
const ANSWER_WAIT: Duration = Duration::from_millis(500);

/// Runs `cmd` as a chooser and returns the path on the first line of its output. `None` if it was cancelled,
/// failed or printed nothing.
///
/// It waits for the *program* to end, not for its output pipe to close: a chooser that was
/// cancelled may leave a helper behind (a portal or D-Bus service) that inherited the pipe, and
/// waiting for the pipe would keep CursorPie waiting for a dialog that is long gone.
fn run_with(cmd: &mut Command, answer_wait: Duration) -> Option<String> {
    use std::io::BufRead;
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    // ends by itself when the pipe closes; a helper that holds it open only keeps this thread
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = std::io::BufReader::new(stdout).read_line(&mut line);
        let _ = tx.send(line);
    });
    let status = child.wait().ok()?;
    if !status.success() {
        return None;
    }
    let line = rx.recv_timeout(answer_wait).ok()?;
    let path = line.trim();
    (!path.is_empty()).then(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn sh(script: &str) -> Command {
        let mut c = Command::new("sh");
        c.arg("-c").arg(script);
        c
    }

    #[test]
    fn a_chosen_path_is_the_first_line_of_the_output() {
        let answer = Duration::from_millis(500);
        assert_eq!(
            run_with(&mut sh("echo /home/u/a.json"), answer).as_deref(),
            Some("/home/u/a.json")
        );
        assert_eq!(run_with(&mut sh("true"), answer), None, "nothing printed");
        assert_eq!(
            run_with(&mut sh("echo /x; exit 1"), answer),
            None,
            "cancelled"
        );
    }

    #[test]
    fn a_helper_left_behind_does_not_keep_us_waiting() {
        // the chooser is gone (cancelled), its helper still holds the output pipe for 5 s
        let start = Instant::now();
        let r = run_with(&mut sh("sleep 5 & exit 1"), Duration::from_millis(500));
        assert_eq!(r, None);
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "{:?}",
            start.elapsed()
        );
        // and a path is still read when the helper stays
        let r = run_with(&mut sh("sleep 5 & echo /p"), Duration::from_millis(500));
        assert_eq!(r.as_deref(), Some("/p"));
    }
}

//! Rolling backups of `config.json`, so a Save that overwrote something by mistake can be undone.
//!
//! Every Save in Settings first stores the text of the file it is about to replace as
//! `config.backup.1.json`, shifting the older ones up to `config.backup.<KEEP>.json`. The
//! functions here only move text around, so they take the directory as a parameter and are unit
//! tested with a scratch directory. The `store` module next to this one connects them to the real
//! config.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How many previous versions are kept.
pub const KEEP: usize = 5;

/// One stored version.
#[derive(Clone, Debug)]
pub struct Backup {
    /// 1 = the newest backup.
    pub n: usize,
    /// When the backup file was written, if the file system says so.
    pub modified: Option<SystemTime>,
}

/// How long ago a backup was written, for a button label: "just now", "5 min ago", "3 h ago",
/// "2 d ago".
pub fn age_label(age: Duration) -> String {
    let s = age.as_secs();
    match s {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", s / 60),
        3600..=86_399 => format!("{} h ago", s / 3600),
        _ => format!("{} d ago", s / 86_400),
    }
}

/// Path of backup `n` (1 = newest) inside `dir`.
pub fn backup_path(dir: &Path, n: usize) -> PathBuf {
    dir.join(format!("config.backup.{n}.json"))
}

/// Writes `data` to `path` through a temporary file in the same directory, so a crash or a full
/// disk never leaves half a file behind.
pub fn write_atomic(path: &Path, data: &str) -> io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Stores `previous` as backup 1 and shifts the older ones (the oldest falls off after `keep`).
/// Does nothing if `previous` is blank or equal to backup 1.
pub fn push_backup(dir: &Path, previous: &str, keep: usize) -> io::Result<()> {
    if previous.trim().is_empty() || read_backup(dir, 1).as_deref() == Some(previous) {
        return Ok(());
    }
    for n in (1..keep.max(1)).rev() {
        let from = backup_path(dir, n);
        if from.exists() {
            fs::rename(&from, backup_path(dir, n + 1))?;
        }
    }
    write_atomic(&backup_path(dir, 1), previous)
}

/// The text of backup `n`, if it exists.
pub fn read_backup(dir: &Path, n: usize) -> Option<String> {
    fs::read_to_string(backup_path(dir, n)).ok()
}

/// The backups that exist, newest first.
pub fn list_backups(dir: &Path, keep: usize) -> Vec<Backup> {
    (1..=keep)
        .filter_map(|n| {
            let meta = fs::metadata(backup_path(dir, n)).ok()?;
            Some(Backup {
                n,
                modified: meta.modified().ok(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cursorpie-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn newest_backup_is_number_one_and_older_ones_shift_up() {
        let d = scratch("backup-shift");
        for v in ["v1", "v2", "v3"] {
            push_backup(&d, v, 5).unwrap();
        }
        assert_eq!(read_backup(&d, 1).as_deref(), Some("v3"));
        assert_eq!(read_backup(&d, 2).as_deref(), Some("v2"));
        assert_eq!(read_backup(&d, 3).as_deref(), Some("v1"));
        assert_eq!(list_backups(&d, 5).len(), 3);
    }

    #[test]
    fn only_the_last_few_are_kept() {
        let d = scratch("backup-keep");
        for i in 1..=7 {
            push_backup(&d, &format!("v{i}"), 3).unwrap();
        }
        assert_eq!(read_backup(&d, 1).as_deref(), Some("v7"));
        assert_eq!(read_backup(&d, 3).as_deref(), Some("v5"));
        assert!(read_backup(&d, 4).is_none());
    }

    #[test]
    fn identical_and_blank_versions_are_skipped() {
        let d = scratch("backup-dup");
        push_backup(&d, "same", 5).unwrap();
        push_backup(&d, "same", 5).unwrap();
        push_backup(&d, "  \n", 5).unwrap();
        assert_eq!(list_backups(&d, 5).len(), 1);
    }

    #[test]
    fn age_labels_use_the_largest_sensible_unit() {
        let l = |s| age_label(Duration::from_secs(s));
        assert_eq!(l(5), "just now");
        assert_eq!(l(60), "1 min ago");
        assert_eq!(l(59 * 60 + 59), "59 min ago");
        assert_eq!(l(3 * 3600), "3 h ago");
        assert_eq!(l(2 * 86_400 + 5), "2 d ago");
    }

    #[test]
    fn atomic_write_replaces_the_file_and_leaves_no_temp_file() {
        let d = scratch("backup-atomic");
        let f = d.join("config.json");
        write_atomic(&f, "one").unwrap();
        write_atomic(&f, "two").unwrap();
        assert_eq!(fs::read_to_string(&f).unwrap(), "two");
        assert!(!f.with_extension("json.tmp").exists());
    }
}

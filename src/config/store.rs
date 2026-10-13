//! Reading, writing and sharing the config: parsing, the file on disk, and the live global copy.

use super::{Config, backup};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Parses a config file. None = not valid JSON or not a CursorPie config.
pub fn parse_config(text: &str) -> Option<Config> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    v.get("version").and_then(|x| x.as_u64())?; // no "version" = old v1 format, rejected
    serde_json::from_value::<Config>(v)
        .ok()
        .map(Config::sanitize)
}

/// The user's home directory (`$HOME`, or `/tmp` if unset).
pub fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
}

/// `~/Library/Application Support/CursorPie` (created if missing).
pub fn app_support_dir() -> PathBuf {
    let p = home()
        .join("Library")
        .join("Application Support")
        .join("CursorPie");
    fs::create_dir_all(&p).ok();
    p
}

/// `~/Library/Application Support/CursorPie/icons`, where the user's own icons live (created if
/// missing). Imported profiles put their icons here too.
pub fn icons_dir() -> PathBuf {
    let p = app_support_dir().join("icons");
    fs::create_dir_all(&p).ok();
    p
}

/// Writes a path under the home directory as `~/...` (the form used in icon strings).
pub fn tilde(p: &std::path::Path) -> String {
    match p.strip_prefix(home()) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => p.display().to_string(),
    }
}

/// The config file: `<app support dir>/config.json`.
pub fn config_path() -> PathBuf {
    app_support_dir().join("config.json")
}

/// Reads `config.json`. A missing file gives the defaults; an invalid one is copied to
/// `config.broken.json` first and the defaults are used.
pub fn load_config() -> Config {
    let path = config_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return Config::default();
    };
    match parse_config(&text) {
        Some(cfg) => cfg,
        None => {
            fs::copy(&path, path.with_file_name("config.broken.json")).ok();
            Config::default()
        }
    }
}

/// Serializes every write to the config directory (the UI thread and the `set_active` workers
/// both save), so two writers can never interleave on the temporary file.
static IO: Mutex<()> = Mutex::new(());

/// Writes the config as pretty JSON, ignoring I/O errors (a failed save must never crash the app).
/// The file is replaced atomically. Do not call this from the event-tap thread; see [`set_active`].
pub fn save_config(config: &Config) {
    if let Ok(data) = serde_json::to_string_pretty(config) {
        let _io = lock(&IO);
        backup::write_atomic(&config_path(), &data).ok();
    }
}

/// Like [`save_config`], but first keeps the file it replaces as a backup (see [`backup`]), so a
/// Save that turns out to be a mistake can be undone with [`load_backup`]. Used by *Save* in
/// Settings, not by the frequent saves of [`set_active`].
pub fn save_config_with_backup(config: &Config) {
    let Ok(data) = serde_json::to_string_pretty(config) else {
        return;
    };
    let _io = lock(&IO);
    let path = config_path();
    if let Ok(old) = fs::read_to_string(&path)
        && old != data
    {
        backup::push_backup(&app_support_dir(), &old, backup::KEEP).ok();
    }
    backup::write_atomic(&path, &data).ok();
}

/// The backups on disk, newest first.
pub fn backups() -> Vec<backup::Backup> {
    backup::list_backups(&app_support_dir(), backup::KEEP)
}

/// Parses backup `n` (1 = newest). None if it does not exist or is not a valid config.
pub fn load_backup(n: usize) -> Option<Config> {
    parse_config(&backup::read_backup(&app_support_dir(), n)?)
}

/// Writes ~/Downloads/cursorpie-config.json and returns its path.
pub fn export_config(config: &Config) -> std::io::Result<PathBuf> {
    let mut p = home();
    p.push("Downloads");
    fs::create_dir_all(&p)?;
    p.push("cursorpie-config.json");
    let data = serde_json::to_string_pretty(config).map_err(std::io::Error::other)?;
    fs::write(&p, data)?;
    Ok(p)
}

// -------------------------------------------------------------------- globals

/// The live config, created on first use by [`cfg()`].
static CONFIG: OnceLock<Mutex<Config>> = OnceLock::new();

/// Locks a mutex and recovers from poisoning (the event tap must never panic because of a lock).
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The live config (shared between the UI thread and the event tap thread).
pub fn cfg() -> MutexGuard<'static, Config> {
    lock(CONFIG.get_or_init(|| Mutex::new(load_config())))
}

/// Remembers which mode is active for a profile (saved on a worker thread, never inside the tap callback).
pub fn set_active(profile: usize, path: Vec<usize>) {
    let snapshot = {
        let mut c = cfg();
        match c.profiles.get_mut(profile) {
            Some(p) => {
                p.active = path;
                Some(c.clone())
            }
            None => None,
        }
    };
    if let Some(c) = snapshot {
        std::thread::spawn(move || save_config(&c));
    }
}

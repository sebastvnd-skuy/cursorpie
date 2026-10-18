//! Desktop entries (`*.desktop`): the Linux counterpart of macOS bundle ids. The **app id** CursorPie
//! uses is the entry's file name, lower case, without `.desktop` (`firefox`, `org.mozilla.firefox`,
//! `google-chrome`). A window reports a raw class (`WM_CLASS`) that is mapped to that id through the
//! installed entries. Everything here is pure except [`scan`], which only reads files.

use std::path::{Path, PathBuf};

/// One installed application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Canonical app id.
    pub id: String,
    pub name: String,
    /// `StartupWMClass`: the window class the app says it uses.
    pub wm_class: Option<String>,
    /// Path of the `.desktop` file (what `gio launch` takes).
    pub path: String,
}

/// An app id in canonical form: trimmed, lower case, without a `.desktop` suffix.
pub fn id_key(s: &str) -> String {
    let t = s.trim().to_ascii_lowercase();
    match t.strip_suffix(".desktop") {
        Some(stem) => stem.to_string(),
        None => t,
    }
}

/// Parses the text of a desktop file. `file_name` is its name (`firefox.desktop`), `path` where it
/// is. Only applications that are not `Hidden` are returned.
pub fn parse(file_name: &str, path: &str, text: &str) -> Option<Entry> {
    let mut in_main = false;
    let mut kind: Option<String> = None;
    let mut name: Option<String> = None;
    let mut wm_class: Option<String> = None;
    let mut hidden = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_main = line == "[Desktop Entry]";
            continue;
        }
        if !in_main {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Type" => kind = Some(value.to_string()),
            "Name" => name = Some(value.to_string()),
            "StartupWMClass" if !value.is_empty() => wm_class = Some(value.to_string()),
            "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }
    if kind.as_deref() != Some("Application") || hidden {
        return None;
    }
    let id = id_key(file_name);
    if id.is_empty() {
        return None;
    }
    Some(Entry {
        name: name.unwrap_or_else(|| id.clone()),
        id,
        wm_class,
        path: path.to_string(),
    })
}

/// The installed applications.
#[derive(Clone, Debug, Default)]
pub struct Index {
    entries: Vec<Entry>,
}

impl Index {
    pub fn new(entries: Vec<Entry>) -> Index {
        Index { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The entry with this app id (`firefox`, `Firefox.desktop` and `FIREFOX` are the same).
    pub fn find(&self, name: &str) -> Option<&Entry> {
        let key = id_key(name);
        self.entries.iter().find(|e| e.id == key)
    }

    /// The canonical app id for the raw identifiers a window reports (class, instance...), best
    /// candidate first. Order of preference: a `StartupWMClass` match, an exact id match, then an id
    /// that ends in `.<candidate>` when only one does (`org.mozilla.firefox` for `firefox`).
    pub fn canonical(&self, candidates: &[&str]) -> Option<String> {
        let keys: Vec<String> = candidates
            .iter()
            .map(|c| id_key(c))
            .filter(|k| !k.is_empty())
            .collect();
        for k in &keys {
            if let Some(e) = self.entries.iter().find(|e| {
                e.wm_class
                    .as_ref()
                    .is_some_and(|w| w.eq_ignore_ascii_case(k))
            }) {
                return Some(e.id.clone());
            }
            if let Some(e) = self.entries.iter().find(|e| &e.id == k) {
                return Some(e.id.clone());
            }
        }
        for k in &keys {
            let suffix = format!(".{k}");
            let mut hits = self.entries.iter().filter(|e| e.id.ends_with(&suffix));
            if let (Some(e), None) = (hits.next(), hits.next()) {
                return Some(e.id.clone());
            }
        }
        None
    }
}

/// Reads the `*.desktop` files of the given `applications` directories (not recursively). A
/// directory listed earlier wins over a later one with the same id, so a user's own entry overrides
/// the system's.
pub fn scan(dirs: &[PathBuf]) -> Index {
    let mut entries: Vec<Entry> = Vec::new();
    for dir in dirs {
        let Ok(read) = std::fs::read_dir(dir) else {
            continue;
        };
        for file in read.flatten() {
            let path = file.path();
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !file_name.ends_with(".desktop") {
                continue;
            }
            let Some(text) = read_small(&path) else {
                continue;
            };
            if let Some(e) = parse(file_name, &path.to_string_lossy(), &text)
                && !entries.iter().any(|x| x.id == e.id)
            {
                entries.push(e);
            }
        }
    }
    Index::new(entries)
}

/// The text of a file, if it is valid UTF-8 and smaller than 64 KiB (desktop files are tiny).
fn read_small(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > 64 * 1024 {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, wm: Option<&str>) -> Entry {
        Entry {
            id: id.into(),
            name: id.into(),
            wm_class: wm.map(str::to_string),
            path: format!("/usr/share/applications/{id}.desktop"),
        }
    }

    const FIREFOX: &str = "# comment\n[Desktop Entry]\nName=Firefox\nName[de]=Feuerfuchs\nType=Application\nExec=firefox %u\nStartupWMClass=firefox\n\n[Desktop Action new-window]\nName=New Window\nType=Hidden\n";

    #[test]
    fn ids_are_lower_case_without_the_suffix() {
        assert_eq!(id_key("  Firefox.desktop "), "firefox");
        assert_eq!(id_key("org.Mozilla.Firefox"), "org.mozilla.firefox");
        assert_eq!(id_key(".desktop"), "");
    }

    #[test]
    fn a_desktop_file_is_parsed() {
        let e = parse("Firefox.desktop", "/p/Firefox.desktop", FIREFOX).unwrap();
        assert_eq!(e.id, "firefox");
        assert_eq!(e.name, "Firefox", "the localized Name[de] is ignored");
        assert_eq!(e.wm_class.as_deref(), Some("firefox"));
        assert_eq!(e.path, "/p/Firefox.desktop");
    }

    #[test]
    fn links_hidden_and_broken_entries_are_skipped() {
        assert!(parse("a.desktop", "/a", "[Desktop Entry]\nType=Link\nName=A").is_none());
        assert!(
            parse(
                "a.desktop",
                "/a",
                "[Desktop Entry]\nType=Application\nHidden=true"
            )
            .is_none()
        );
        assert!(parse("a.desktop", "/a", "garbage").is_none());
        assert!(parse(".desktop", "/a", "[Desktop Entry]\nType=Application").is_none());
        let named = parse("tool.desktop", "/t", "[Desktop Entry]\nType=Application").unwrap();
        assert_eq!(named.name, "tool", "falls back to the id");
    }

    #[test]
    fn a_window_class_maps_to_the_canonical_id() {
        let idx = Index::new(vec![
            entry("org.mozilla.firefox", Some("firefox")),
            entry("google-chrome", Some("Google-chrome")),
            entry("code", Some("Code")),
            entry("org.gnome.nautilus", None),
        ]);
        assert_eq!(
            idx.canonical(&["firefox"]).as_deref(),
            Some("org.mozilla.firefox")
        );
        assert_eq!(
            idx.canonical(&["Google-chrome", "google-chrome"])
                .as_deref(),
            Some("google-chrome")
        );
        assert_eq!(idx.canonical(&["Code"]).as_deref(), Some("code"));
        assert_eq!(
            idx.canonical(&["nautilus"]).as_deref(),
            Some("org.gnome.nautilus")
        );
        assert_eq!(idx.canonical(&["xterm"]), None);
        assert_eq!(idx.canonical(&[]), None);
        assert_eq!(idx.canonical(&["  "]), None);
    }

    #[test]
    fn a_suffix_match_needs_a_unique_entry() {
        let idx = Index::new(vec![
            entry("org.a.player", None),
            entry("org.b.player", None),
        ]);
        assert_eq!(idx.canonical(&["player"]), None, "two candidates: no guess");
        let one = Index::new(vec![entry("org.a.player", None)]);
        assert_eq!(one.canonical(&["player"]).as_deref(), Some("org.a.player"));
    }

    #[test]
    fn the_wm_class_beats_an_id_of_the_same_name() {
        let idx = Index::new(vec![
            entry("editor", None),
            entry("my-editor", Some("editor")),
        ]);
        assert_eq!(idx.canonical(&["editor"]).as_deref(), Some("my-editor"));
    }

    #[test]
    fn find_ignores_case_and_the_suffix() {
        let idx = Index::new(vec![entry("firefox", None)]);
        assert!(idx.find("Firefox.desktop").is_some());
        assert!(idx.find("FIREFOX").is_some());
        assert!(idx.find("chrome").is_none());
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn scan_reads_files_and_the_first_directory_wins() {
        let base = std::env::temp_dir().join(format!("cursorpie-de-{}", std::process::id()));
        let user = base.join("user");
        let system = base.join("system");
        std::fs::create_dir_all(&user).unwrap();
        std::fs::create_dir_all(&system).unwrap();
        std::fs::write(
            user.join("app.desktop"),
            "[Desktop Entry]\nType=Application\nName=Mine",
        )
        .unwrap();
        std::fs::write(
            system.join("app.desktop"),
            "[Desktop Entry]\nType=Application\nName=Theirs",
        )
        .unwrap();
        std::fs::write(system.join("notes.txt"), "x").unwrap();
        let idx = scan(&[user.clone(), system.clone(), base.join("missing")]);
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.find("app").unwrap().name, "Mine");
        let _ = std::fs::remove_dir_all(&base);
    }
}

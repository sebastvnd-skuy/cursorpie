//! The icon texture cache shared by the pie and the Settings window.
//!
//! An icon string is either text / emoji (nothing to load), the path of an image file, or
//! `app:<bundle id>` for the real icon of an app (see [`appicon`]). Loading an image means reading
//! and scaling a file; loading an app icon means asking macOS for a multi-size TIFF and decoding
//! it. Both are far too slow to do on the frame that opens the pie, so decoded textures are kept
//! here:
//!
//! * The cache is **bounded** ([`CAPACITY`]) and evicts the least recently used entries, see
//!   [`Lru`].
//! * It **survives** opening and closing Settings. Only image files that changed on disk are
//!   reloaded ([`IconCache::revalidate`]), instead of throwing everything away every time.
//! * It is **warmed up** while the trigger is held, before the pie opens, see
//!   [`icons_to_warm`] and `Msg::Warm`.
//! * It is **dropped** when the UI goes to sleep ([`IconCache::clear`]).

use crate::cache::Lru;
use crate::config::{Config, Item, ItemKind};
use crate::ui::appicon;
use crate::ui::draw::{icon_path, load_icon};
use eframe::egui;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// Most textures kept between frames. An icon is at most
/// [`ICON_MAX_PX`](crate::ui::draw::ICON_MAX_PX) pixels on its longest side (128 x 128 = 64 KiB),
/// so this is a few MiB at the very most.
pub const CAPACITY: usize = 96;

/// How long to wait before trying again to load an app icon that could not be loaded.
const APP_ICON_RETRY: Duration = Duration::from_secs(3);

/// What identifies a version of an image file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileStamp {
    modified: Option<SystemTime>,
    len: u64,
}

impl FileStamp {
    /// The stamp of the file at `path`, or None if it cannot be read.
    pub fn of(path: &std::path::Path) -> Option<Self> {
        let m = std::fs::metadata(path).ok()?;
        Some(Self {
            modified: m.modified().ok(),
            len: m.len(),
        })
    }
}

/// One cached icon, or the memory of a failed attempt.
struct Entry {
    /// None = the icon could not be loaded.
    tex: Option<egui::TextureHandle>,
    /// For image files: where it came from and what it looked like, to notice changes.
    file: Option<(PathBuf, Option<FileStamp>)>,
    /// When loading failed (an app icon is tried again after [`APP_ICON_RETRY`]).
    failed_at: Option<Instant>,
}

/// Bounded texture cache for icon strings. See the module docs.
#[derive(Default)]
pub struct IconCache {
    lru: Lru<String, Entry>,
}

impl IconCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of cached icons (including remembered failures).
    pub fn len(&self) -> usize {
        self.lru.len()
    }

    /// Whether nothing is cached.
    pub fn is_empty(&self) -> bool {
        self.lru.is_empty()
    }

    /// Call once per frame: trims the cache to [`CAPACITY`] (never touching icons used by the last frame).
    pub fn new_frame(&mut self) {
        self.lru.new_frame(CAPACITY);
    }

    /// Drops every texture and gives the table's memory back.
    pub fn clear(&mut self) {
        self.lru.clear();
    }

    /// Forgets what is stale: image files that changed, vanished or failed to load. Cheap (one
    /// `stat` per cached file), so it runs whenever Settings opens or closes.
    pub fn revalidate(&mut self) {
        self.lru.retain(|_, e| match &e.file {
            Some((path, stamp)) => e.tex.is_some() && FileStamp::of(path) == *stamp,
            None => true,
        });
    }

    /// The texture for an icon string. None for text icons and for images that cannot be loaded.
    pub fn get(&mut self, ctx: &egui::Context, icon: &str) -> Option<egui::TextureHandle> {
        // "app:<bundle id>" = the real icon of an app, asked from macOS (see `appicon`)
        if let Some(bundle) = icon.strip_prefix(appicon::PREFIX) {
            if let Some(e) = self.lru.get(icon) {
                // a failure is not final: macOS may simply not have had the icon ready. Try again
                // after a pause, but not on every frame (decoding is not free).
                let retry =
                    e.tex.is_none() && e.failed_at.is_none_or(|at| at.elapsed() >= APP_ICON_RETRY);
                if !retry {
                    return e.tex.clone();
                }
            }
            let tex = appicon::load(bundle)
                .map(|img| ctx.load_texture(icon, img, egui::TextureOptions::LINEAR));
            let failed_at = tex.is_none().then(Instant::now);
            self.lru.insert(
                icon.to_string(),
                Entry {
                    tex: tex.clone(),
                    file: None,
                    failed_at,
                },
            );
            return tex;
        }

        let path = icon_path(icon)?;
        let key = path.to_string_lossy().to_string();
        if let Some(e) = self.lru.get(&key) {
            return e.tex.clone();
        }
        let stamp = FileStamp::of(&path);
        let tex = load_icon(ctx, &path, &key);
        self.lru.insert(
            key,
            Entry {
                tex: tex.clone(),
                file: Some((path, stamp)),
                failed_at: None,
            },
        );
        tex
    }
}

/// The icon strings a pie of `profile` needs on its first frames: its top ring, one level of
/// submenus, the center icon and the app icon of the targeted app. Text icons are included on
/// purpose: [`IconCache::get`] returns at once for them. At most [`CAPACITY`] strings.
pub fn icons_to_warm(c: &Config, profile: usize, target: Option<&str>) -> Vec<String> {
    let Some(p) = c.profiles.get(profile) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    let mut add = |s: &str| {
        if !s.is_empty() && out.len() < CAPACITY && !out.iter().any(|o| o == s) {
            out.push(s.to_string());
        }
    };
    if let Some(i) = p.active_item() {
        add(&i.icon);
    }
    if c.center_app_icon
        && let Some(b) = appicon::bundle_for(&p.apps, target)
    {
        add(&format!("{}{}", appicon::PREFIX, b));
    }
    let ring = |items: &[Item], add: &mut dyn FnMut(&str)| {
        for it in items {
            add(&it.icon);
        }
    };
    ring(&p.items, &mut add);
    for it in &p.items {
        if let ItemKind::Submenu(children) = &it.kind {
            ring(children, &mut add);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A unique scratch directory under the system temp dir.
    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cursorpie-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write_png(path: &std::path::Path, rgba: [u8; 4], size: u32) {
        let mut img = image::RgbaImage::new(size, size);
        for p in img.pixels_mut() {
            *p = image::Rgba(rgba);
        }
        img.save(path).unwrap();
    }

    #[test]
    fn text_icons_need_no_texture() {
        let ctx = egui::Context::default();
        let mut c = IconCache::new();
        assert!(c.get(&ctx, "<>").is_none());
        assert!(c.is_empty(), "text icons are not cached");
    }

    #[test]
    fn a_file_icon_is_loaded_once_and_kept() {
        let dir = scratch("once");
        let file = dir.join("a.png");
        write_png(&file, [255, 0, 0, 255], 16);
        let ctx = egui::Context::default();
        let mut c = IconCache::new();
        let s = file.to_string_lossy().to_string();
        let a = c.get(&ctx, &s).expect("loads");
        let b = c.get(&ctx, &s).expect("cached");
        assert_eq!(a.id(), b.id(), "same texture, not reloaded");
        assert_eq!(c.len(), 1);
        c.revalidate();
        assert_eq!(
            c.len(),
            1,
            "an unchanged file stays cached across revalidate"
        );
    }

    #[test]
    fn revalidate_drops_changed_and_missing_files() {
        let dir = scratch("changed");
        let (a, b) = (dir.join("a.png"), dir.join("b.png"));
        write_png(&a, [255, 0, 0, 255], 16);
        write_png(&b, [0, 255, 0, 255], 16);
        let ctx = egui::Context::default();
        let mut c = IconCache::new();
        let (sa, sb) = (
            a.to_string_lossy().to_string(),
            b.to_string_lossy().to_string(),
        );
        let old = c.get(&ctx, &sa).unwrap().id();
        let _ = c.get(&ctx, &sb).unwrap();
        // a changes size (so the stamp differs even on a coarse clock), b disappears
        write_png(&a, [0, 0, 255, 255], 32);
        std::fs::remove_file(&b).unwrap();
        c.revalidate();
        assert_eq!(c.len(), 0);
        assert_ne!(c.get(&ctx, &sa).unwrap().id(), old, "reloaded from disk");
        assert!(c.get(&ctx, &sb).is_none());
        c.revalidate();
        assert_eq!(
            c.len(),
            1,
            "a failed load is forgotten so it can be retried"
        );
    }

    #[test]
    fn a_broken_file_is_remembered_as_failed() {
        let dir = scratch("broken");
        let file = dir.join("x.png");
        std::fs::File::create(&file)
            .unwrap()
            .write_all(b"not a png")
            .unwrap();
        let ctx = egui::Context::default();
        let mut c = IconCache::new();
        let s = file.to_string_lossy().to_string();
        assert!(c.get(&ctx, &s).is_none());
        assert_eq!(
            c.len(),
            1,
            "the failure is cached: no decode attempt per frame"
        );
    }

    #[test]
    fn the_cache_is_bounded_but_never_thrashes_within_a_frame() {
        let dir = scratch("bound");
        let ctx = egui::Context::default();
        let mut c = IconCache::new();
        let files: Vec<String> = (0..CAPACITY + 10)
            .map(|i| {
                let f = dir.join(format!("{i}.png"));
                write_png(&f, [i as u8, 0, 0, 255], 4);
                f.to_string_lossy().to_string()
            })
            .collect();
        c.new_frame();
        for f in &files {
            let _ = c.get(&ctx, f);
        }
        c.new_frame(); // everything was used in the last frame: nothing is evicted
        assert_eq!(c.len(), CAPACITY + 10);
        c.new_frame(); // nothing used in between: trimmed to the capacity
        assert_eq!(c.len(), CAPACITY);
        c.clear();
        assert!(c.is_empty());
    }

    #[test]
    fn warm_list_covers_ring_submenu_center_and_app_icon() {
        use crate::config::{Action, Config, Item, Profile};
        let mut c = Config::default();
        c.center_app_icon = true;
        c.profiles = vec![Profile {
            name: "App".into(),
            apps: vec!["com.example.App".into()],
            items: vec![
                Item::run("A", "~/a.png", Action::Nothing),
                Item::submenu("S", "S", vec![Item::run("B", "~/b.png", Action::Nothing)]),
                Item::run("Dup", "~/a.png", Action::Nothing),
            ],
            active: vec![],
        }];
        let w = icons_to_warm(&c, 0, Some("com.example.App"));
        assert!(w.contains(&"app:com.example.App".to_string()));
        assert!(w.contains(&"~/a.png".to_string()) && w.contains(&"~/b.png".to_string()));
        assert_eq!(
            w.iter().filter(|s| *s == "~/a.png").count(),
            1,
            "no duplicates"
        );
        assert!(icons_to_warm(&c, 7, None).is_empty(), "unknown profile");
    }
}

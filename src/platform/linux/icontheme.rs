//! The icon of an app on Linux: the `Icon=` of its desktop entry, looked up in the icon themes.
//!
//! This follows the freedesktop icon theme specification in the parts that matter for application
//! icons: the themes are searched in order (the user's theme, the themes it inherits from, then
//! `hicolor`), in every icon directory (`~/.local/share/icons`, `~/.icons`, `/usr/share/icons`,
//! the Flatpak exports...), in the `apps` folders only, and `/usr/share/pixmaps` comes last. It is
//! simpler than the specification where that costs nothing: it does not read the `Directories`
//! lists of `index.theme`, it recognizes the size folders by their names (`48x48`, `48x48@2`, `48`)
//! and it takes the closest size over all themes together instead of theme by theme.
//!
//! Everything here is pure except [`find`], which only looks at files, so the tests run on every OS.
//! Reading the file and turning it into pixels is the job of the backend (see `backend::app_icon`).

use std::fs;
use std::path::{Path, PathBuf};

/// The side, in pixels, an icon is wanted at: what `ui::appicon` shrinks every icon to.
pub const WANT: u32 = 128;

/// An icon file that was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found {
    Png(PathBuf),
    /// Needs a renderer, see `backend::render_svg`.
    Svg(PathBuf),
}

/// One file that could be the icon.
struct Candidate {
    path: PathBuf,
    /// Pixels on a side; 0 when the file does not say (a pixmap, a scalable icon).
    size: u32,
    svg: bool,
}

// ------------------------------------------------------------------------------ reading settings

/// The value of `key` in `[section]` of an ini-like text (`index.theme`, `settings.ini`,
/// `kdeglobals`). Empty values count as missing.
pub fn ini_value(text: &str, section: &str, key: &str) -> Option<String> {
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            inside = name == section;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some((k, v)) = line.split_once('=')
            && k.trim() == key
        {
            let v = v.trim();
            return (!v.is_empty()).then(|| v.to_string());
        }
    }
    None
}

/// The themes a theme inherits from, from the text of its `index.theme`.
pub fn inherits(index_theme: &str) -> Vec<String> {
    ini_value(index_theme, "Icon Theme", "Inherits")
        .map(|v| {
            v.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The icon theme named in a GTK `settings.ini`.
pub fn gtk_theme(settings_ini: &str) -> Option<String> {
    ini_value(settings_ini, "Settings", "gtk-icon-theme-name")
}

/// The icon theme named in KDE's `kdeglobals`.
pub fn kde_theme(kdeglobals: &str) -> Option<String> {
    ini_value(kdeglobals, "Icons", "Theme")
}

/// The theme in the output of `gsettings get org.gnome.desktop.interface icon-theme` (`'Adwaita'`).
pub fn gsettings_theme(output: &str) -> Option<String> {
    let t = output.trim().trim_matches('\'').trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// The themes to search, in order: `first`, the themes it inherits from (and theirs), then
/// `hicolor`, which every theme falls back to. `read_index` gives the text of a theme's
/// `index.theme`. A loop of `Inherits` or a very long chain is cut off.
pub fn theme_chain(
    first: Option<&str>,
    read_index: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut queue: Vec<String> = first
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .into_iter()
        .collect();
    while !queue.is_empty() && out.len() < 16 {
        let name = queue.remove(0);
        if out.contains(&name) {
            continue;
        }
        if let Some(text) = read_index(&name) {
            queue.extend(inherits(&text));
        }
        out.push(name);
    }
    if !out.iter().any(|t| t == "hicolor") {
        out.push("hicolor".to_string());
    }
    out
}

// ------------------------------------------------------------------------------------ finding

/// The pixels on a side of a size folder: `48x48` and `48` are 48, `48x48@2` is 96.
fn dir_pixels(name: &str) -> Option<u32> {
    let (base, scale) = match name.split_once('@') {
        Some((b, s)) => (b, s.parse::<u32>().ok()?),
        None => (name, 1),
    };
    let side: u32 = base.split('x').next()?.parse().ok()?;
    (side > 0 && scale > 0).then(|| side.saturating_mul(scale))
}

/// Adds `<dir>/<name>.png` (when `with_png`) and `<dir>/<name>.svg` if those files exist.
fn add_files(out: &mut Vec<Candidate>, dir: &Path, name: &str, size: u32, with_png: bool) {
    if with_png {
        let png = dir.join(format!("{name}.png"));
        if png.is_file() {
            out.push(Candidate {
                path: png,
                size,
                svg: false,
            });
        }
    }
    let svg = dir.join(format!("{name}.svg"));
    if svg.is_file() {
        out.push(Candidate {
            path: svg,
            size,
            svg: true,
        });
    }
}

/// Adds the files of the icon `name` in the `apps` folders of one theme directory. Both layouts
/// are read: `<size>/apps/<name>` (hicolor, Adwaita) and `apps/<size>/<name>` (some newer themes).
fn theme_candidates(theme_dir: &Path, name: &str, out: &mut Vec<Candidate>) {
    let Ok(read) = fs::read_dir(theme_dir) else {
        return;
    };
    for d1 in read.flatten() {
        let n1 = d1.file_name().to_string_lossy().into_owned();
        let p1 = d1.path();
        if n1 == "scalable" {
            add_files(out, &p1.join("apps"), name, 0, false);
        } else if let Some(px) = dir_pixels(&n1) {
            add_files(out, &p1.join("apps"), name, px, true);
        } else if n1 == "apps" {
            let Ok(inner) = fs::read_dir(&p1) else {
                continue;
            };
            for d2 in inner.flatten() {
                let n2 = d2.file_name().to_string_lossy().into_owned();
                if n2 == "scalable" {
                    add_files(out, &d2.path(), name, 0, false);
                } else if let Some(px) = dir_pixels(&n2) {
                    add_files(out, &d2.path(), name, px, true);
                }
            }
        }
    }
}

/// How many pixels a candidate counts as: a file that does not say is taken to fit.
fn pixels(c: &Candidate, want: u32) -> u32 {
    if c.size == 0 { want } else { c.size }
}

/// Best first: the smallest PNG that is at least `want` wide, then the SVGs (they scale), then the
/// PNGs that are smaller, the biggest first.
fn order(cands: Vec<Candidate>, want: u32) -> Vec<Found> {
    let (svgs, pngs): (Vec<Candidate>, Vec<Candidate>) = cands.into_iter().partition(|c| c.svg);
    let (mut big, mut small): (Vec<Candidate>, Vec<Candidate>) =
        pngs.into_iter().partition(|c| pixels(c, want) >= want);
    big.sort_by_key(|c| pixels(c, want));
    small.sort_by_key(|c| std::cmp::Reverse(pixels(c, want)));
    let png = |c: Candidate| Found::Png(c.path);
    let svg = |c: Candidate| Found::Svg(c.path);
    big.into_iter()
        .map(png)
        .chain(svgs.into_iter().map(svg))
        .chain(small.into_iter().map(png))
        .collect()
}

/// The files that may be the icon `icon` (the `Icon=` of a desktop entry), best first. Nothing when
/// there are none. `icon` is a name looked up in `themes` (searched in each of `icon_dirs`, see
/// [`theme_chain`]) and, if no theme has it, in `pixmap_dirs`; or an absolute path.
pub fn find(
    icon: &str,
    themes: &[String],
    icon_dirs: &[PathBuf],
    pixmap_dirs: &[PathBuf],
    want: u32,
) -> Vec<Found> {
    let icon = icon.trim();
    if icon.is_empty() {
        return Vec::new();
    }
    if icon.starts_with('/') {
        let path = PathBuf::from(icon);
        if !path.is_file() {
            return Vec::new();
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        return match ext.as_deref() {
            Some("png") => vec![Found::Png(path)],
            Some("svg") => vec![Found::Svg(path)],
            _ => Vec::new(),
        };
    }
    // a few entries name the file (`vscode.png`) instead of the icon
    let name = [".png", ".svg", ".xpm"]
        .iter()
        .find_map(|ext| icon.strip_suffix(*ext))
        .unwrap_or(icon);
    if name.is_empty() || name.contains('/') {
        return Vec::new();
    }
    let mut cands: Vec<Candidate> = Vec::new();
    for theme in themes {
        for dir in icon_dirs {
            theme_candidates(&dir.join(theme), name, &mut cands);
        }
    }
    if cands.is_empty() {
        for dir in pixmap_dirs {
            add_files(&mut cands, dir, name, 0, true);
        }
    }
    order(cands, want)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty directory of its own for one test.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cursorpie-icontheme-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }

    fn themes(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn size_folders_are_read_by_name() {
        assert_eq!(dir_pixels("48x48"), Some(48));
        assert_eq!(dir_pixels("128"), Some(128));
        assert_eq!(dir_pixels("48x48@2"), Some(96));
        assert_eq!(dir_pixels("scalable"), None);
        assert_eq!(dir_pixels("symbolic"), None);
        assert_eq!(dir_pixels("0x0"), None);
        assert_eq!(dir_pixels("48x48@x"), None);
    }

    #[test]
    fn settings_are_read_from_their_files() {
        let index = "[Icon Theme]\nName=Adwaita\nInherits = AdwaitaLegacy, hicolor ,\n[Other]\nInherits=no\n";
        assert_eq!(inherits(index), vec!["AdwaitaLegacy", "hicolor"]);
        assert!(inherits("[Icon Theme]\nName=x\n").is_empty());
        assert_eq!(
            gtk_theme("[Settings]\ngtk-theme-name=Foo\ngtk-icon-theme-name=Papirus\n").as_deref(),
            Some("Papirus")
        );
        assert_eq!(gtk_theme("[Settings]\ngtk-icon-theme-name=\n"), None);
        assert_eq!(
            kde_theme("[General]\nTheme=no\n[Icons]\nTheme=breeze-dark\n").as_deref(),
            Some("breeze-dark")
        );
        assert_eq!(gsettings_theme("'Adwaita'\n").as_deref(), Some("Adwaita"));
        assert_eq!(gsettings_theme("\n"), None);
    }

    #[test]
    fn the_theme_chain_follows_inherits_and_ends_with_hicolor() {
        let read = |name: &str| match name {
            "Papirus" => Some("[Icon Theme]\nInherits=Adwaita,hicolor\n".to_string()),
            "Adwaita" => Some("[Icon Theme]\nInherits=Papirus\n".to_string()), // a loop
            _ => None,
        };
        assert_eq!(
            theme_chain(Some("Papirus"), read),
            vec!["Papirus", "Adwaita", "hicolor"]
        );
        assert_eq!(theme_chain(None, |_| None), vec!["hicolor"]);
        assert_eq!(theme_chain(Some(" Foo "), |_| None), vec!["Foo", "hicolor"]);
    }

    #[test]
    fn the_smallest_png_that_is_big_enough_wins() {
        let root = scratch("sizes");
        for size in ["16x16", "48x48", "128x128", "256x256"] {
            touch(&root.join(format!("hicolor/{size}/apps/app.png")));
        }
        touch(&root.join("hicolor/scalable/apps/app.svg"));
        let got = find(
            "app",
            &themes(&["hicolor"]),
            std::slice::from_ref(&root),
            &[],
            128,
        );
        assert_eq!(
            got[0],
            Found::Png(root.join("hicolor/128x128/apps/app.png"))
        );
        assert_eq!(
            got[1],
            Found::Png(root.join("hicolor/256x256/apps/app.png"))
        );
        assert_eq!(
            got[2],
            Found::Svg(root.join("hicolor/scalable/apps/app.svg"))
        );
        // the small ones come last, the bigger first
        assert_eq!(got[3], Found::Png(root.join("hicolor/48x48/apps/app.png")));
        assert_eq!(got[4], Found::Png(root.join("hicolor/16x16/apps/app.png")));
        assert_eq!(got.len(), 5);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn only_small_pngs_still_give_the_biggest_one() {
        let root = scratch("small");
        touch(&root.join("hicolor/16x16/apps/app.png"));
        touch(&root.join("hicolor/32x32/apps/app.png"));
        let got = find(
            "app",
            &themes(&["hicolor"]),
            std::slice::from_ref(&root),
            &[],
            128,
        );
        assert_eq!(got[0], Found::Png(root.join("hicolor/32x32/apps/app.png")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]

    fn both_folder_layouts_and_every_theme_and_dir_are_searched() {
        let user = scratch("user");
        let system = scratch("system");

        // apps/<size>/ layout, in the user's theme
        touch(&user.join("Papirus/apps/64/app.png"));

        // <size>/apps/ layout with a scale, in the system's hicolor
        touch(&system.join("hicolor/64x64@2/apps/app.png"));

        let icon_dirs = [user.clone(), system.clone()];
        let theme_list = themes(&["Papirus", "hicolor"]);

        // A missing icon must not be found.
        assert!(find("other", &theme_list, &icon_dirs, &[], 128).is_empty());

        // Both layouts and both icon directories must be searched.
        let got = find("app", &theme_list, &icon_dirs, &[], 128);

        assert_eq!(
            got,
            vec![
                Found::Png(system.join("hicolor/64x64@2/apps/app.png")),
                Found::Png(user.join("Papirus/apps/64/app.png")),
            ]
        );

        // Other folders of a theme (places, symbolic) are not app icons.
        touch(&user.join("Papirus/places/64/other.png"));
        touch(&user.join("Papirus/symbolic/apps/other.svg"));

        assert!(find("other", &theme_list, &icon_dirs, &[], 128).is_empty());

        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&system);
    }

    #[test]
    fn pixmaps_are_the_last_resort() {
        let root = scratch("pixmaps-icons");
        let pix = scratch("pixmaps");
        touch(&pix.join("legacy.png"));
        touch(&root.join("hicolor/48x48/apps/modern.png"));
        touch(&pix.join("modern.png"));
        let dirs = [root.clone()];
        let pixmaps = [pix.clone()];
        let t = themes(&["hicolor"]);
        assert_eq!(
            find("legacy", &t, &dirs, &pixmaps, 128),
            vec![Found::Png(pix.join("legacy.png"))]
        );
        // a theme that has the icon makes the pixmaps unnecessary
        assert_eq!(
            find("modern", &t, &dirs, &pixmaps, 128),
            vec![Found::Png(root.join("hicolor/48x48/apps/modern.png"))]
        );
        // the file name instead of the icon name works too
        assert_eq!(
            find("legacy.png", &t, &dirs, &pixmaps, 128),
            vec![Found::Png(pix.join("legacy.png"))]
        );
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&pix);
    }

    #[test]
    fn an_absolute_path_is_used_as_it_is() {
        let root = scratch("absolute");
        touch(&root.join("icon.png"));
        touch(&root.join("icon.svg"));
        touch(&root.join("icon.xpm"));
        let path = |n: &str| root.join(n).to_string_lossy().into_owned();
        let none: [PathBuf; 0] = [];
        assert_eq!(
            find(&path("icon.png"), &[], &none, &none, 128),
            vec![Found::Png(root.join("icon.png"))]
        );
        assert_eq!(
            find(&path("icon.svg"), &[], &none, &none, 128),
            vec![Found::Svg(root.join("icon.svg"))]
        );
        assert!(find(&path("icon.xpm"), &[], &none, &none, 128).is_empty());
        assert!(find(&path("missing.png"), &[], &none, &none, 128).is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn odd_names_find_nothing() {
        let root = scratch("odd");
        touch(&root.join("hicolor/48x48/apps/app.png"));
        let t = themes(&["hicolor"]);
        let dirs = [root.clone()];
        for name in ["", "  ", "../app", "a/b", ".png"] {
            assert!(find(name, &t, &dirs, &dirs, 128).is_empty(), "{name:?}");
        }
        assert!(find("not-there", &t, &dirs, &dirs, 128).is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}

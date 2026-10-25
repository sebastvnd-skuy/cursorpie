//! Makes an image the user chose fit to be a slice icon, and tells them what happened.
//!
//! An icon is drawn at 30 to 120 points, and the texture made from it is at most
//! [`ICON_MAX_PX`](crate::ui::draw::ICON_MAX_PX) pixels wide. A 12 megapixel photo or a 5 MB PNG is
//! therefore wasted disk space, a slow decode every time the cache reloads it, and a memory spike
//! while it is decoded. Instead of refusing such files, [`fit`] shrinks them **once**, when they are
//! chosen:
//!
//! * a small file (at most [`KEEP_BYTES`] and [`KEEP_SIDE`] px) is used as it is;
//! * a bigger one is scaled down to [`STORE_PX`] px (twice the texture size, so it stays sharp on a
//!   Retina display) and saved as a PNG in the icons folder. The original is never touched;
//! * a file that cannot be read, or is too big to open safely, is refused with a sentence for the
//!   status bar.
//!
//! The work runs on the file-chooser's worker thread, never on the UI thread. It uses the `image`
//! crate that is already a dependency, so nothing new is compiled in. The core ([`fit_into`]) takes
//! the directory as a parameter and is unit tested with a scratch directory.

use crate::config::{icons_dir, tilde};
use crate::ui::draw::{open_reader, prepare_icon};
use image::{ImageFormat, RgbaImage};
use std::fs;
use std::path::{Path, PathBuf};

/// Files up to this size (and [`KEEP_SIDE`]) are used unchanged.
pub const KEEP_BYTES: u64 = 512 * 1024;
/// Longest side, in pixels, of a file that is used unchanged.
pub const KEEP_SIDE: u32 = 512;
/// Longest side of the smaller copy that replaces a bigger image.
pub const STORE_PX: u32 = 256;
/// Images with more pixels than this are refused (decoded, they would take 4 bytes per pixel).
pub const MAX_IMPORT_PIXELS: u64 = 40_000_000;
/// Files bigger than this are refused without being opened.
pub const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;

/// The icon to use and, when something was changed or worth saying, a message for the user.
#[derive(Debug)]
pub struct Fitted {
    /// The icon string for `Item::icon` (`~/...` when the file is under the home directory).
    pub icon: String,
    pub note: Option<String>,
}

/// [`fit_into`] with the real icons folder. `src` is the path the file chooser returned.
pub fn fit(src: &str) -> Result<Fitted, String> {
    fit_into(Path::new(src), &icons_dir(), &tilde)
}

/// "340 KB" / "1.4 MB".
fn human(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes.div_ceil(1024).max(1))
    }
}

/// Straight (not premultiplied) colors from premultiplied ones, to save a normal PNG.
fn unpremultiply(img: &mut RgbaImage) {
    for px in img.as_chunks_mut::<4>().0 {
        let a = px[3] as u32;
        if a != 0 && a != 255 {
            for v in &mut px[..3] {
                *v = ((*v as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

/// A free file name `<stem>-icon.png`, `<stem>-icon-2.png`, ... inside `dir`.
fn free_name(dir: &Path, src: &Path) -> Option<PathBuf> {
    let stem: String = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(40)
        .collect();
    let stem = if stem.is_empty() {
        "icon".to_string()
    } else {
        stem
    };
    (1..1000)
        .map(|n| {
            if n == 1 {
                dir.join(format!("{stem}-icon.png"))
            } else {
                dir.join(format!("{stem}-icon-{n}.png"))
            }
        })
        .find(|p| !p.exists())
}

/// Checks `src`, and shrinks it into `dir` if it is bigger than an icon needs. `as_icon` turns the
/// final path into the icon string. The error is a sentence for the status bar.
pub fn fit_into(
    src: &Path,
    dir: &Path,
    as_icon: &dyn Fn(&Path) -> String,
) -> Result<Fitted, String> {
    let bytes = fs::metadata(src)
        .map_err(|_| "Could not read that file.".to_string())?
        .len();
    // the size comes from the header: nothing is decoded to learn it
    let (w, h) = open_reader(src)
        .and_then(|r| r.into_dimensions().ok())
        .ok_or("That is not an image CursorPie can read (use PNG, JPG or WebP).")?;
    if bytes > MAX_IMPORT_BYTES || u64::from(w) * u64::from(h) > MAX_IMPORT_PIXELS {
        return Err(format!(
            "That image is too large to use as an icon ({w} x {h} px, {}). Choose a smaller one.",
            human(bytes)
        ));
    }
    if bytes <= KEEP_BYTES && w.max(h) <= KEEP_SIDE {
        return Ok(Fitted {
            icon: as_icon(src),
            note: None,
        });
    }

    let img = open_reader(src)
        .and_then(|r| r.decode().ok())
        .ok_or("Could not decode that image.")?
        .into_rgba8();
    let mut small = prepare_icon(img, STORE_PX);
    unpremultiply(&mut small);
    let dest = free_name(dir, src).ok_or("Too many copies in the icons folder.")?;
    small
        .save_with_format(&dest, ImageFormat::Png)
        .map_err(|_| "Could not save the smaller copy of the image.".to_string())?;
    let after = fs::metadata(&dest).map_or(0, |m| m.len());
    Ok(Fitted {
        icon: as_icon(&dest),
        note: Some(format!(
            "Image shrunk for use as an icon: {w} x {h} px ({}) to {} x {} px ({}). \
             The smaller copy is in the icons folder; the original is untouched.",
            human(bytes),
            small.width(),
            small.height(),
            human(after)
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cursorpie-fit-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn as_icon(p: &Path) -> String {
        p.to_string_lossy().to_string()
    }

    #[test]
    fn a_small_image_is_used_as_it_is() {
        let dir = scratch("small");
        let src = dir.join("a.png");
        RgbaImage::from_pixel(64, 64, Rgba([9, 9, 9, 255]))
            .save(&src)
            .unwrap();
        let out = dir.join("out");
        fs::create_dir_all(&out).unwrap();
        let f = fit_into(&src, &out, &as_icon).unwrap();
        assert_eq!(f.icon, as_icon(&src));
        assert!(f.note.is_none());
        assert_eq!(
            fs::read_dir(&out).unwrap().count(),
            0,
            "nothing was written"
        );
    }

    #[test]
    fn a_big_image_is_shrunk_into_the_icons_folder_and_the_user_is_told() {
        let dir = scratch("big");
        let src = dir.join("My photo.png");
        RgbaImage::from_pixel(1200, 800, Rgba([200, 30, 30, 255]))
            .save(&src)
            .unwrap();
        let out = dir.join("out");
        fs::create_dir_all(&out).unwrap();
        let f = fit_into(&src, &out, &as_icon).unwrap();
        assert!(f.icon.ends_with("My_photo-icon.png"), "{}", f.icon);
        assert!(f.note.as_deref().unwrap().contains("1200 x 800"));
        let small = image::open(&f.icon).unwrap().to_rgba8();
        assert_eq!((small.width(), small.height()), (256, 171));
        assert_eq!(small.get_pixel(100, 80).0[3], 255);
        assert!(src.exists(), "the original is never touched");
        // choosing it again must not overwrite the first copy
        let again = fit_into(&src, &out, &as_icon).unwrap();
        assert_ne!(again.icon, f.icon);
    }

    #[test]
    fn unreadable_and_missing_files_give_a_sentence() {
        let dir = scratch("bad");
        let junk = dir.join("x.png");
        fs::write(&junk, b"not an image").unwrap();
        assert!(
            fit_into(&junk, &dir, &as_icon)
                .unwrap_err()
                .contains("not an image")
        );
        let missing = dir.join("nope.png");
        assert!(
            fit_into(&missing, &dir, &as_icon)
                .unwrap_err()
                .contains("read")
        );
    }

    #[test]
    fn unpremultiply_restores_straight_colors() {
        let mut img = RgbaImage::from_pixel(1, 1, Rgba([100, 50, 20, 128]));
        unpremultiply(&mut img);
        assert_eq!(img.get_pixel(0, 0).0, [199, 100, 40, 128]);
    }

    #[test]
    fn sizes_read_well() {
        assert_eq!(human(100), "1 KB");
        assert_eq!(human(340 * 1024), "340 KB");
        assert_eq!(human(1536 * 1024), "1.5 MB");
    }
}

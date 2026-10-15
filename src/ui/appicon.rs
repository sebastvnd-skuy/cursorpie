//! App icons for the center of a pie.
//!
//! The real icon comes from the OS (`platform::app_icon`: on macOS the image the Dock and Finder show, as a
//! TIFF with one page per size; on Windows an RGBA bitmap). `load` picks the page that fits, decodes it and reduces it to a
//! white glyph: everything that differs from the icon's background color is kept (in white, with
//! its own transparency) and the background itself is dropped.
//!
//! Icons are asked for with the icon string `app:<bundle id>`, see `PREFIX` and `Ed::icon_for`
//! (which caches the texture, so each app is read from macOS only once).

use crate::platform;
use eframe::egui;
use image::{Rgba, RgbaImage, imageops};
use std::collections::BTreeMap;
use std::io::Cursor;
use tiff::ColorType;
use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;

/// Icon strings starting with this are the real icon of the app with that bundle id.
pub const PREFIX: &str = "app:";

/// Size (px) of the icon texture. The pie shows it at about a third of that.
const SIZE: u32 = 128;

/// Which app's icon a pie shows: the app being targeted if it belongs to the profile, otherwise
/// the first app of the profile. None for a profile without apps (the general pie).
pub fn bundle_for<'a>(apps: &'a [String], target: Option<&str>) -> Option<&'a str> {
    if let Some(t) = target
        && let Some(a) = apps.iter().find(|a| a.as_str() == t)
    {
        return Some(a.as_str());
    }
    apps.first().map(|a| a.as_str())
}

/// The icon of `bundle` as a white glyph, ready to become a texture. None if the app is not
/// installed or its icon cannot be read. Failures are reported on stderr (once per attempt), so a
/// missing icon can be diagnosed from `cargo run` instead of failing silently.
pub fn load(bundle: &str) -> Option<egui::ColorImage> {
    let Some(icon) = platform::app_icon(bundle) else {
        eprintln!("CursorPie: app icon for {bundle}: the system returned no icon (app not found?)");
        return None;
    };
    let img = match icon {
        platform::NativeIcon::Tiff(data) => match decode_best(&data) {
            Ok(img) => img,
            Err(why) => {
                eprintln!(
                    "CursorPie: app icon for {bundle}: cannot decode the TIFF ({} bytes): {why}",
                    data.len()
                );
                return None;
            }
        },
        platform::NativeIcon::Rgba {
            width,
            height,
            data,
        } => RgbaImage::from_raw(width, height, data)?,
    };
    let img = resize_straight(&img);
    let glyph = to_glyph(&img);
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [glyph.width() as usize, glyph.height() as usize],
        glyph.as_raw(),
    ))
}

/// Resizes to `SIZE` x `SIZE` in premultiplied space. Resizing straight RGBA mixes the (black)
/// color of fully transparent pixels into the edge of the icon; those dark pixels then count as
/// "differs from the background" and show up as a white outline around the glyph.
fn resize_straight(img: &RgbaImage) -> RgbaImage {
    let mut pm = img.clone();
    for c in pm.chunks_exact_mut(4) {
        let a = c[3] as u32;
        for v in &mut c[..3] {
            *v = ((*v as u32 * a + 127) / 255) as u8;
        }
    }
    let mut out = imageops::resize(&pm, SIZE, SIZE, imageops::FilterType::Lanczos3);
    unpremultiply(&mut out);
    out
}

/// Picks the best page of a multi-size TIFF (the smallest one that is at least `SIZE` wide,
/// otherwise the largest) and decodes it to straight (not premultiplied) RGBA. If that page cannot
/// be decoded the next best ones are tried (at most 4), so one odd page does not lose the icon.
fn decode_best(data: &[u8]) -> Result<RgbaImage, String> {
    let mut d = Decoder::new(Cursor::new(data)).map_err(|e| format!("not a readable TIFF: {e}"))?;

    // pass 1: the width of every page (nothing is decoded yet)
    let mut pages: Vec<(usize, u32)> = Vec::new();
    let mut index = 0;
    loop {
        if let Ok((w, _)) = d.dimensions() {
            pages.push((index, w));
        }
        if !d.more_images() || d.next_image().is_err() {
            break;
        }
        index += 1;
    }
    if pages.is_empty() {
        return Err("the TIFF has no readable page".into());
    }

    // order of preference: big enough (smallest first), then too small (largest first)
    let mut big: Vec<(usize, u32)> = pages.iter().copied().filter(|(_, w)| *w >= SIZE).collect();
    big.sort_by_key(|&(_, w)| w);
    let mut small: Vec<(usize, u32)> = pages.iter().copied().filter(|(_, w)| *w < SIZE).collect();
    small.sort_by_key(|&(_, w)| std::cmp::Reverse(w));

    // pass 2: decode the best page that works
    let mut last = String::from("no usable page");
    for (page, w) in big.into_iter().chain(small).take(4) {
        match decode_page(&mut d, page) {
            Ok(img) => return Ok(img),
            Err(e) => last = format!("page {page} ({w} px wide): {e}"),
        }
    }
    Err(last)
}

/// Decodes one TIFF page to straight RGBA (handles 8/16-bit, float, gray and premultiplied alpha).
fn decode_page(d: &mut Decoder<Cursor<&[u8]>>, page: usize) -> Result<RgbaImage, String> {
    d.seek_to_image(page).map_err(|e| format!("seek: {e}"))?;
    let (w, h) = d.dimensions().map_err(|e| format!("size: {e}"))?;
    let color = d.colortype().map_err(|e| format!("color type: {e}"))?;
    // ExtraSamples = 1: the alpha is "associated", i.e. the colors are premultiplied with it
    let premultiplied = d
        .find_tag(Tag::ExtraSamples)
        .ok()
        .flatten()
        .and_then(|v| v.into_u16().ok())
        == Some(1);
    let result = d
        .read_image()
        .map_err(|e| format!("decode ({color:?}): {e}"))?;
    // float samples are 0..1 (values outside that range are clamped)
    let unit = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let buf: Vec<u8> = match result {
        DecodingResult::U8(b) => b,
        // 16 bits per channel: keep the high byte
        DecodingResult::U16(b) => b.iter().map(|v| (v >> 8) as u8).collect(),
        // wide-gamut icons from macOS come as 16-bit (half) or 32-bit floats
        DecodingResult::F16(b) => b.iter().map(|v| unit(v.to_f32())).collect(),
        DecodingResult::F32(b) => b.iter().map(|&v| unit(v)).collect(),
        DecodingResult::F64(b) => b.iter().map(|&v| unit(v as f32)).collect(),
        DecodingResult::I16(b) => b.iter().map(|&v| (v.max(0) >> 7) as u8).collect(),
        DecodingResult::U32(b) => b.iter().map(|v| (v >> 24) as u8).collect(),
        _ => return Err(format!("unsupported sample type ({color:?})")),
    };
    let color = match color {
        ColorType::RGBA(_) => ColorType::RGBA(8),
        ColorType::RGB(_) => ColorType::RGB(8),
        ColorType::GrayA(_) => ColorType::GrayA(8),
        ColorType::Gray(_) => ColorType::Gray(8),
        other => other,
    };
    to_rgba(&buf, w, h, color, premultiplied)
        .ok_or_else(|| format!("unsupported pixel layout ({color:?}, {w}x{h})"))
}

/// Expands a decoded buffer of the given layout to RGBA8; `None` for layouts we do not support.
fn to_rgba(buf: &[u8], w: u32, h: u32, color: ColorType, premultiplied: bool) -> Option<RgbaImage> {
    let n = (w * h) as usize;
    let mut px = Vec::with_capacity(n * 4);
    match color {
        ColorType::RGBA(8) if buf.len() >= n * 4 => {
            px.extend_from_slice(&buf[..n * 4]);
            if premultiplied {
                unpremultiply(&mut px);
            }
        }
        ColorType::RGB(8) if buf.len() >= n * 3 => {
            for c in buf[..n * 3].chunks_exact(3) {
                px.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        ColorType::GrayA(8) if buf.len() >= n * 2 => {
            for c in buf[..n * 2].chunks_exact(2) {
                px.extend_from_slice(&[c[0], c[0], c[0], c[1]]);
            }
        }
        ColorType::Gray(8) if buf.len() >= n => {
            for &g in &buf[..n] {
                px.extend_from_slice(&[g, g, g, 255]);
            }
        }
        _ => return None,
    }
    RgbaImage::from_raw(w, h, px)
}

/// RGBA with premultiplied colors -> straight colors.
fn unpremultiply(px: &mut [u8]) {
    for c in px.chunks_exact_mut(4) {
        let a = c[3] as u32;
        if a != 0 && a != 255 {
            for v in &mut c[..3] {
                *v = ((*v as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

/// White glyph version of an icon: white pixels whose opacity says how much that spot differs from
/// the icon's background (and how opaque it was). Falls back to a plain white silhouette when the
/// icon has no contrast at all.
pub fn to_glyph(img: &RgbaImage) -> RgbaImage {
    // background = the most common color among the (mostly) opaque pixels, bucketed to 3 bits per
    // channel so a gradient falls into a few buckets
    let mut buckets: BTreeMap<u16, (u32, [u32; 3])> = BTreeMap::new();
    for p in img.pixels().filter(|p| p[3] >= 200) {
        let key = ((p[0] >> 5) as u16) << 6 | ((p[1] >> 5) as u16) << 3 | (p[2] >> 5) as u16;
        let e = buckets.entry(key).or_insert((0, [0; 3]));
        e.0 += 1;
        for i in 0..3 {
            e.1[i] += p[i] as u32;
        }
    }
    let Some((n, sum)) = buckets.values().max_by_key(|v| v.0).copied() else {
        return silhouette(img);
    };
    let bg = [
        sum[0] as f32 / n as f32,
        sum[1] as f32 / n as f32,
        sum[2] as f32 / n as f32,
    ];

    let mut out = RgbaImage::new(img.width(), img.height());
    let mut mass = 0.0f32;
    for (x, y, p) in img.enumerate_pixels() {
        let d = ((p[0] as f32 - bg[0]).powi(2)
            + (p[1] as f32 - bg[1]).powi(2)
            + (p[2] as f32 - bg[2]).powi(2))
        .sqrt()
            / 441.67; // 0..1 (255 * sqrt 3)
        let alpha = (p[3] as f32 / 255.0) * (d * 3.5).clamp(0.0, 1.0);
        mass += alpha;
        out.put_pixel(x, y, Rgba([255, 255, 255, (alpha * 255.0).round() as u8]));
    }
    // (almost) nothing stood out from the background
    if mass < 0.02 * (img.width() * img.height()) as f32 {
        return silhouette(img);
    }
    out
}

/// A plain white copy of the icon, keeping only its alpha.
fn silhouette(img: &RgbaImage) -> RgbaImage {
    let mut out = img.clone();
    for p in out.pixels_mut() {
        *p = Rgba([255, 255, 255, p[3]]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiff::encoder::{TiffEncoder, colortype};

    /// size x size icon: round (transparent corners), blue background, red square in the middle.
    fn sample(size: u32) -> RgbaImage {
        let mid = (size as f32 - 1.0) / 2.0;
        let mut img = RgbaImage::new(size, size);
        for (x, y, p) in img.enumerate_pixels_mut() {
            let (dx, dy) = (x as f32 - mid, y as f32 - mid);
            let r = size as f32 * 0.47;
            let inner = size as f32 * 0.17;
            *p = if dx.hypot(dy) > r {
                Rgba([0, 0, 0, 0])
            } else if dx.abs() < inner && dy.abs() < inner {
                Rgba([220, 40, 40, 255])
            } else {
                Rgba([30, 90, 200, 255])
            };
        }
        img
    }

    fn tiff_with_pages(sizes: &[u32]) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        let mut enc = TiffEncoder::new(&mut out).unwrap();
        for &s in sizes {
            enc.write_image::<colortype::RGBA8>(s, s, sample(s).as_raw())
                .unwrap();
        }
        drop(enc);
        out.into_inner()
    }

    #[test]
    fn glyph_keeps_what_differs_from_the_background() {
        let g = to_glyph(&sample(64));
        assert!(g.get_pixel(32, 32)[3] > 200, "the red square stays");
        assert!(g.get_pixel(8, 32)[3] < 10, "the blue background goes");
        assert_eq!(g.get_pixel(0, 0)[3], 0, "transparent stays transparent");
        assert!(g.pixels().all(|p| p.0[..3] == [255, 255, 255]), "all white");
    }

    #[test]
    fn flat_icon_falls_back_to_a_silhouette() {
        let mut img = sample(64);
        for p in img.pixels_mut().filter(|p| p[3] != 0) {
            *p = Rgba([30, 90, 200, 255]);
        }
        let g = to_glyph(&img);
        assert_eq!(g.get_pixel(32, 32), &Rgba([255, 255, 255, 255]));
        assert_eq!(g.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn empty_image_does_not_panic() {
        let g = to_glyph(&RgbaImage::new(8, 8));
        assert!(g.pixels().all(|p| p[3] == 0));
    }

    #[test]
    fn picks_the_smallest_page_that_is_big_enough() {
        // pages in an arbitrary order, as macOS may write them
        let img = decode_best(&tiff_with_pages(&[16, 512, 256, 32])).unwrap();
        assert_eq!(img.width(), 256);
        assert_eq!(img.get_pixel(128, 128), &Rgba([220, 40, 40, 255]));
    }

    #[test]
    fn falls_back_to_the_largest_page_when_all_are_small() {
        let img = decode_best(&tiff_with_pages(&[16, 64, 32])).unwrap();
        assert_eq!(img.width(), 64);
    }

    #[test]
    fn sixteen_bit_pages_are_narrowed_to_eight() {
        let mut out = Cursor::new(Vec::new());
        let mut enc = TiffEncoder::new(&mut out).unwrap();
        let px: Vec<u16> = std::iter::repeat([0x1100u16, 0x2200, 0x3300, 0xffff])
            .take(16 * 16)
            .flatten()
            .collect();
        enc.write_image::<colortype::RGBA16>(16, 16, &px).unwrap();
        drop(enc);
        let img = decode_best(&out.into_inner()).unwrap();
        assert_eq!(img.get_pixel(3, 3), &Rgba([0x11, 0x22, 0x33, 0xff]));
    }

    #[test]
    fn resizing_does_not_darken_the_edge() {
        // flat blue disc on a transparent canvas: after resizing, every visible pixel must still
        // be (about) that blue; the black of the transparent pixels must not leak into the edge
        let mut img = sample(256);
        for p in img.pixels_mut().filter(|p| p[3] != 0) {
            *p = Rgba([30, 90, 200, 255]);
        }
        let out = resize_straight(&img);
        assert!(
            out.pixels()
                .filter(|p| p[3] > 128)
                .all(|p| p[2] >= 190 && p[1] >= 80)
        );
    }

    #[test]
    fn garbage_is_not_an_icon() {
        assert!(decode_best(b"not a tiff").is_err());
    }

    #[test]
    fn premultiplied_colors_are_restored() {
        let mut px = [100, 50, 20, 128, 7, 7, 7, 0, 9, 9, 9, 255];
        unpremultiply(&mut px);
        assert_eq!(px, [199, 100, 40, 128, 7, 7, 7, 0, 9, 9, 9, 255]);
    }

    #[test]
    fn icon_belongs_to_the_targeted_app_first() {
        let apps: Vec<String> = ["com.apple.Safari", "com.google.Chrome"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            bundle_for(&apps, Some("com.google.Chrome")),
            Some("com.google.Chrome")
        );
        assert_eq!(
            bundle_for(&apps, Some("com.other")),
            Some("com.apple.Safari")
        );
        assert_eq!(bundle_for(&apps, None), Some("com.apple.Safari"));
        assert_eq!(bundle_for(&[], Some("com.apple.Safari")), None);
    }
}

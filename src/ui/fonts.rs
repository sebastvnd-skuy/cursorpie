//! The fonts of the interface: Inter for text, Phosphor for icons.
//!
//! Three families are registered on startup ([`install`]):
//!
//! | Family | Font | Use |
//! |---|---|---|
//! | `Proportional` | Inter Regular, then egui's own fonts | all text; the fallbacks keep emoji and other scripts that Inter does not hold |
//! | [`semibold`] | Inter SemiBold | titles and the names of things |
//! | [`icons`] / [`icons_fill`] | Phosphor Regular / Fill | every icon, see [`symbols`](super::symbols) |
//!
//! The font files in `assets/fonts/` are cut down to what the interface needs (see
//! `scripts/subset-fonts.py`), 340 KB for all four, and compiled into the binary: the app makes no
//! network connection and reads no font from the system, so it looks the same everywhere.

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, RichText};
use std::sync::Arc;

static INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
static INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.ttf");
static ICONS_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Icons-Regular.ttf");
static ICONS_FILL: &[u8] = include_bytes!("../../assets/fonts/Icons-Fill.ttf");

/// Name of the family for titles.
const SEMIBOLD: &str = "semibold";
/// Name of the family for outline icons.
const ICONS: &str = "icons";
/// Name of the family for filled icons (the selected entry of the sidebar).
const ICONS_FILL_NAME: &str = "icons-fill";

/// Inter SemiBold: titles and the names of things.
pub fn semibold() -> FontFamily {
    FontFamily::Name(SEMIBOLD.into())
}

/// Phosphor Regular: outline icons.
pub fn icons() -> FontFamily {
    FontFamily::Name(ICONS.into())
}

/// Phosphor Fill: the same icons, filled.
pub fn icons_fill() -> FontFamily {
    FontFamily::Name(ICONS_FILL_NAME.into())
}

/// Text in the title weight.
pub fn title(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text)
        .font(FontId::new(size, semibold()))
        .color(color)
}

/// An icon as text, for a label or a button that has no room for the painter.
pub fn icon(glyph: char, size: f32, color: Color32) -> RichText {
    RichText::new(glyph.to_string())
        .font(FontId::new(size, icons()))
        .color(color)
}

/// Draws an icon centered at `center`, in the outline or the filled weight.
///
/// The icon fonts are laid out so that the em square is the line box, so centering the text
/// centers the picture: there is no per-icon offset to maintain.
pub fn paint_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    glyph: char,
    size: f32,
    color: Color32,
    filled: bool,
) {
    let family = if filled { icons_fill() } else { icons() };
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        glyph,
        FontId::new(size, family),
        color,
    );
}

/// The font definitions: egui's defaults with ours first.
///
/// Kept apart from [`install`] so a test can build them without a window.
pub fn definitions() -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    for (name, bytes) in [
        ("Inter", INTER_REGULAR),
        ("Inter-SemiBold", INTER_SEMIBOLD),
        ("Icons-Regular", ICONS_REGULAR),
        ("Icons-Fill", ICONS_FILL),
    ] {
        defs.font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }

    // Inter first; egui's fonts stay behind it as the fallback for what Inter does not have
    // (emoji used as slice icons, scripts it was not cut for).
    let fallback = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut text = vec!["Inter".to_owned()];
    text.extend(fallback.iter().cloned());
    let mut bold = vec!["Inter-SemiBold".to_owned()];
    bold.extend(text.iter().cloned());
    defs.families.insert(FontFamily::Proportional, text);
    defs.families.insert(semibold(), bold);
    defs.families
        .insert(icons(), vec!["Icons-Regular".to_owned()]);
    defs.families
        .insert(icons_fill(), vec!["Icons-Fill".to_owned()]);
    defs
}

/// Registers the fonts. Call once, when the egui context is created.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(definitions());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::symbols;

    #[test]
    fn every_family_has_a_font() {
        let defs = definitions();
        for family in [
            FontFamily::Proportional,
            FontFamily::Monospace,
            semibold(),
            icons(),
            icons_fill(),
        ] {
            let list = defs.families.get(&family).expect("family is defined");
            assert!(!list.is_empty());
            for name in list {
                assert!(defs.font_data.contains_key(name), "{name} has no data");
            }
        }
        assert_eq!(
            defs.families[&FontFamily::Proportional][0],
            "Inter",
            "Inter leads the text family"
        );
    }

    /// Lays out a frame with the fonts installed, so a font that ab_glyph cannot parse fails here
    /// instead of on a user's screen, and every icon of the set must have a glyph in both weights.
    #[test]
    fn every_icon_has_a_glyph_in_both_weights() {
        let ctx = egui::Context::default();
        install(&ctx);
        let missing = std::cell::RefCell::new(Vec::new());
        let input = egui::RawInput::default();
        // nothing renders the output, so the texture uploads are dropped on purpose
        ctx.run_ui(input, |ui| {
            ui.ctx().fonts_mut(|fonts| {
                for family in [icons(), icons_fill()] {
                    let id = FontId::new(16.0, family);
                    for (name, glyph) in symbols::ALL {
                        if !fonts.has_glyph(&id, *glyph) {
                            missing.borrow_mut().push(*name);
                        }
                    }
                }
                let text = FontId::new(14.0, FontFamily::Proportional);
                assert!(fonts.has_glyph(&text, 'A'), "Inter has Latin");
                assert!(fonts.has_glyph(&text, '\u{2026}'), "Inter has the ellipsis");
                assert!(fonts.has_glyph(&text, '\u{203a}'), "Inter has the chevron");
            });
        })
        .drop_without_applying_deltas();
        assert!(
            missing.borrow().is_empty(),
            "no glyph for {:?}",
            missing.borrow()
        );
    }
}

//! The display scale on X11. X11 has one global scale, not one per monitor. It is what the toolkit
//! (winit) uses, so the pointer and the pie window agree: the `WINIT_X11_SCALE_FACTOR` override, else
//! the `Xft.dpi` resource divided by 96, else 1.

/// `Xft.dpi` from the text of the `RESOURCE_MANAGER` property (lines like `Xft.dpi:\t144`).
pub fn parse_xft_dpi(resources: &str) -> Option<f64> {
    resources.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.trim() != "Xft.dpi" {
            return None;
        }
        value.trim().parse::<f64>().ok().filter(|d| *d > 0.0)
    })
}

/// The scale factor: `override_value` (the environment override) if it is a sensible number, else
/// `xft_dpi / 96`, else 1. Always between 0.5 and 8.
pub fn scale_from(override_value: Option<&str>, xft_dpi: Option<f64>) -> f64 {
    let from_env = override_value
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s > 0.0);
    let s = from_env
        .or_else(|| xft_dpi.map(|d| d / 96.0))
        .unwrap_or(1.0);
    s.clamp(0.5, 8.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xft_dpi_is_read_from_the_resource_text() {
        let text = "Xft.antialias:\t1\nXft.dpi:\t144\nXft.hinting:\t1\n";
        assert_eq!(parse_xft_dpi(text), Some(144.0));
        assert_eq!(parse_xft_dpi("Xft.dpi: 96"), Some(96.0));
        assert_eq!(parse_xft_dpi("Xft.dpix: 96"), None);
        assert_eq!(parse_xft_dpi("Xft.dpi: nope"), None);
        assert_eq!(parse_xft_dpi("Xft.dpi: 0"), None);
        assert_eq!(parse_xft_dpi(""), None);
    }

    #[test]
    fn the_scale_prefers_the_override_then_the_dpi() {
        assert_eq!(scale_from(Some("2"), Some(96.0)), 2.0);
        assert_eq!(scale_from(None, Some(192.0)), 2.0);
        assert_eq!(scale_from(None, Some(120.0)), 1.25);
        assert_eq!(scale_from(None, None), 1.0);
        assert_eq!(scale_from(Some("abc"), Some(144.0)), 1.5);
        assert_eq!(scale_from(Some("-3"), None), 1.0);
        assert_eq!(scale_from(Some("100"), None), 8.0, "clamped");
        assert_eq!(scale_from(None, Some(10.0)), 0.5, "clamped");
    }
}

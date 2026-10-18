//! Windows as the window manager reports them, and the decision "which app is under this point".
//! Pure: the X11 backend gathers the list, this module decides.

/// One top-level window.
#[derive(Clone, Debug, PartialEq)]
pub struct Toplevel {
    /// Canonical app id of the window (see `desktop_entry`).
    pub app: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Minimized or hidden.
    pub hidden: bool,
    /// A dock, desktop, popup, tooltip, notification or CursorPie's own window.
    pub skip: bool,
    /// The workspace it is on; `None` = on all of them.
    pub desktop: Option<u32>,
}

/// Index of the window under `(px, py)` in `windows`, given **top-most first**: the first visible,
/// non-skipped window on the current workspace that contains the point.
pub fn hit_test(
    windows: &[Toplevel],
    px: f64,
    py: f64,
    current_desktop: Option<u32>,
) -> Option<usize> {
    windows.iter().position(|t| {
        !t.hidden
            && !t.skip
            && match (t.desktop, current_desktop) {
                (Some(d), Some(c)) => d == c,
                _ => true,
            }
            && px >= t.x
            && px < t.x + t.w
            && py >= t.y
            && py < t.y + t.h
    })
}

/// `(instance, class)` from the bytes of a `WM_CLASS` property (two NUL-terminated strings). A
/// single string is used for both. `None` if there is nothing readable.
pub fn parse_wm_class(bytes: &[u8]) -> Option<(String, String)> {
    let mut parts = bytes
        .split(|b| *b == 0)
        .map(|p| String::from_utf8_lossy(p).trim().to_string())
        .filter(|p| !p.is_empty());
    let instance = parts.next()?;
    let class = parts.next().unwrap_or_else(|| instance.clone());
    Some((instance, class))
}

/// Whether a window with these `_NET_WM_WINDOW_TYPE` atom names is not a real application window.
pub fn is_skipped_type(type_names: &[&str]) -> bool {
    type_names.iter().any(|t| {
        matches!(
            *t,
            "_NET_WM_WINDOW_TYPE_DOCK"
                | "_NET_WM_WINDOW_TYPE_DESKTOP"
                | "_NET_WM_WINDOW_TYPE_NOTIFICATION"
                | "_NET_WM_WINDOW_TYPE_TOOLTIP"
                | "_NET_WM_WINDOW_TYPE_MENU"
                | "_NET_WM_WINDOW_TYPE_DROPDOWN_MENU"
                | "_NET_WM_WINDOW_TYPE_POPUP_MENU"
                | "_NET_WM_WINDOW_TYPE_COMBO"
                | "_NET_WM_WINDOW_TYPE_DND"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(app: &str, x: f64, y: f64, w: f64, h: f64) -> Toplevel {
        Toplevel {
            app: app.into(),
            x,
            y,
            w,
            h,
            hidden: false,
            skip: false,
            desktop: None,
        }
    }

    #[test]
    fn the_topmost_window_under_the_point_wins() {
        let list = [
            win("front", 100.0, 100.0, 200.0, 200.0),
            win("back", 0.0, 0.0, 800.0, 600.0),
        ];
        assert_eq!(hit_test(&list, 150.0, 150.0, None), Some(0));
        assert_eq!(hit_test(&list, 50.0, 50.0, None), Some(1));
        assert_eq!(hit_test(&list, 900.0, 50.0, None), None);
    }

    #[test]
    fn edges_are_half_open() {
        let list = [win("a", 10.0, 10.0, 100.0, 50.0)];
        assert_eq!(hit_test(&list, 10.0, 10.0, None), Some(0));
        assert_eq!(hit_test(&list, 110.0, 30.0, None), None);
        assert_eq!(hit_test(&list, 50.0, 60.0, None), None);
    }

    #[test]
    fn hidden_skipped_and_other_workspace_windows_are_ignored() {
        let mut hidden = win("hidden", 0.0, 0.0, 500.0, 500.0);
        hidden.hidden = true;
        let mut dock = win("dock", 0.0, 0.0, 500.0, 500.0);
        dock.skip = true;
        let mut elsewhere = win("elsewhere", 0.0, 0.0, 500.0, 500.0);
        elsewhere.desktop = Some(2);
        let mut sticky = win("sticky", 0.0, 0.0, 500.0, 500.0);
        sticky.desktop = None;
        let list = [hidden, dock, elsewhere, sticky];
        assert_eq!(hit_test(&list, 10.0, 10.0, Some(0)), Some(3));
        let on_two = [win("two", 0.0, 0.0, 9.0, 9.0)];
        assert_eq!(hit_test(&on_two, 1.0, 1.0, Some(0)), Some(0));
    }

    #[test]
    fn a_window_on_the_current_workspace_matches() {
        let mut w = win("here", 0.0, 0.0, 100.0, 100.0);
        w.desktop = Some(1);
        assert_eq!(hit_test(&[w.clone()], 5.0, 5.0, Some(1)), Some(0));
        assert_eq!(hit_test(&[w.clone()], 5.0, 5.0, Some(0)), None);
        assert_eq!(
            hit_test(&[w], 5.0, 5.0, None),
            Some(0),
            "unknown workspace: assume visible"
        );
    }

    #[test]
    fn wm_class_has_an_instance_and_a_class() {
        assert_eq!(
            parse_wm_class(b"navigator\0Firefox\0"),
            Some(("navigator".into(), "Firefox".into()))
        );
        assert_eq!(
            parse_wm_class(b"xterm\0"),
            Some(("xterm".into(), "xterm".into()))
        );
        assert_eq!(parse_wm_class(b"\0\0"), None);
        assert_eq!(parse_wm_class(b""), None);
    }

    #[test]
    fn docks_and_popups_are_not_apps() {
        assert!(is_skipped_type(&["_NET_WM_WINDOW_TYPE_DOCK"]));
        assert!(is_skipped_type(&[
            "_NET_WM_WINDOW_TYPE_NORMAL",
            "_NET_WM_WINDOW_TYPE_TOOLTIP"
        ]));
        assert!(!is_skipped_type(&["_NET_WM_WINDOW_TYPE_NORMAL"]));
        assert!(!is_skipped_type(&["_NET_WM_WINDOW_TYPE_DIALOG"]));
        assert!(!is_skipped_type(&[]));
    }
}

//! Pure helpers of the Windows backend (no Win32 calls), unit tested on every OS.

use crate::platform::FileKind;

/// UTF-16 with a terminating zero, for Win32 `PCWSTR` parameters.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Text up to the first zero of a UTF-16 buffer.
pub fn from_wide(buf: &[u16]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

/// The app id of an executable: its file name in lower case (`C:\Apps\Chrome.EXE` -> `chrome.exe`).
/// `None` for an empty name.
pub fn exe_id_from_path(path: &str) -> Option<String> {
    let name = path.rsplit(['\\', '/']).next()?.trim();
    (!name.is_empty()).then(|| name.to_ascii_lowercase())
}

/// Whether a path names an executable.
pub fn is_exe(path: &str) -> bool {
    path.trim().to_ascii_lowercase().ends_with(".exe")
}

/// Scale factor of a monitor from its DPI (96 dpi = 100 %).
pub fn scale_from_dpi(dpi: u32) -> f64 {
    if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 }
}

/// Physical pixels to logical points at `scale`.
pub fn px_to_logical(px: f64, scale: f64) -> f64 {
    if scale <= 0.0 { px } else { px / scale }
}

/// Logical points to physical pixels at `scale`.
pub fn logical_to_px(pt: f64, scale: f64) -> f64 {
    if scale <= 0.0 { pt } else { pt * scale }
}

/// Window styles of a frameless window: the caption (title bar + border + dialog frame) and the
/// sizing frame are removed, the window becomes a `WS_POPUP`, and the 3D edge styles that make the
/// DWM draw a frame are cleared. `None` when `style` / `style_ex` are already like that.
pub fn frameless_styles(style: u32, style_ex: u32) -> Option<(u32, u32)> {
    let new = (frameless_style(style), frameless_ex_style(style_ex));
    (new != (style, style_ex)).then_some(new)
}

/// The `GWL_STYLE` half of [`frameless_styles`].
pub fn frameless_style(style: u32) -> u32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{WS_CAPTION, WS_POPUP, WS_THICKFRAME};
    (style & !(WS_CAPTION | WS_THICKFRAME)) | WS_POPUP
}

/// The `GWL_EXSTYLE` half of [`frameless_styles`].
pub fn frameless_ex_style(style_ex: u32) -> u32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME, WS_EX_STATICEDGE, WS_EX_WINDOWEDGE,
    };
    style_ex & !(WS_EX_WINDOWEDGE | WS_EX_CLIENTEDGE | WS_EX_DLGMODALFRAME | WS_EX_STATICEDGE)
}

/// The value written to the `Run` registry key: the executable path in quotes.
pub fn run_value(exe: &str) -> String {
    format!("\"{exe}\"")
}

/// Parameters for `cmd.exe` that run `cmd` exactly as typed. `/S` makes cmd strip only the outer
/// pair of quotes, so a command that itself starts with a quote still works.
pub fn shell_params(cmd: &str) -> String {
    format!("/S /C \"{cmd}\"")
}

/// Parameters for `explorer.exe` that select a file in a folder window.
pub fn select_params(path: &str) -> String {
    format!("/select,\"{path}\"")
}

/// The file-type filter of the open dialog: pairs of `description\0pattern\0`, ended by a double
/// zero, as `OPENFILENAMEW::lpstrFilter` wants it.
pub fn filter_for(kind: FileKind) -> Vec<u16> {
    let (desc, pattern) = match kind {
        FileKind::Image => ("Images", "*.png;*.jpg;*.jpeg;*.webp"),
        FileKind::Json => ("JSON files", "*.json"),
        FileKind::App => ("Programs", "*.exe"),
    };
    let mut v: Vec<u16> = Vec::new();
    for part in [desc, pattern] {
        v.extend(part.encode_utf16());
        v.push(0);
    }
    v.push(0);
    v
}

/// Converts BGRA pixels (what `GetDIBits` returns) to straight RGBA in place. When no pixel has any
/// alpha (an old icon without an alpha channel) every pixel becomes opaque, except those the
/// 1-bit `mask` (one byte per pixel, nonzero = transparent) says are background.
pub fn bgra_to_rgba(px: &mut [u8], mask: Option<&[u8]>) {
    let has_alpha = px.chunks_exact(4).any(|p| p[3] != 0);
    for (i, p) in px.chunks_exact_mut(4).enumerate() {
        p.swap(0, 2);
        if !has_alpha {
            let transparent = mask.and_then(|m| m.get(i)).is_some_and(|m| *m != 0);
            p[3] = if transparent { 0 } else { 255 };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_framed_window_loses_its_frame_styles() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            WS_BORDER, WS_CAPTION, WS_CLIPSIBLINGS, WS_EX_ACCEPTFILES, WS_EX_TOPMOST,
            WS_EX_WINDOWEDGE, WS_POPUP, WS_SYSMENU, WS_THICKFRAME, WS_VISIBLE,
        };
        // what winit creates for `with_decorations(false)`: caption and border are still there
        let style = WS_CAPTION | WS_BORDER | WS_CLIPSIBLINGS | WS_SYSMENU | WS_VISIBLE;
        let ex = WS_EX_WINDOWEDGE | WS_EX_ACCEPTFILES | WS_EX_TOPMOST;
        let (s, e) = frameless_styles(style, ex).expect("has a frame");
        assert_eq!(s & (WS_CAPTION | WS_BORDER | WS_THICKFRAME), 0);
        assert_ne!(s & WS_POPUP, 0);
        assert_eq!(e & WS_EX_WINDOWEDGE, 0);
        // everything else is kept: visibility, clipping, system menu, always on top, file drop
        assert_eq!(
            s & (WS_CLIPSIBLINGS | WS_SYSMENU | WS_VISIBLE),
            WS_CLIPSIBLINGS | WS_SYSMENU | WS_VISIBLE
        );
        assert_eq!(
            e & (WS_EX_ACCEPTFILES | WS_EX_TOPMOST),
            WS_EX_ACCEPTFILES | WS_EX_TOPMOST
        );
    }

    #[test]
    fn a_frameless_window_is_left_alone() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{WS_POPUP, WS_VISIBLE};
        // idempotent: the second call (every frame) must report "nothing to do"
        let (s, e) =
            frameless_styles(WS_POPUP | WS_VISIBLE, 0).map_or((WS_POPUP | WS_VISIBLE, 0), |x| x);
        assert_eq!(frameless_styles(s, e), None);
    }

    #[test]
    fn wide_strings_round_trip() {
        let w = wide("Chrome \u{e9}");
        assert_eq!(*w.last().unwrap(), 0);
        assert_eq!(from_wide(&w), "Chrome \u{e9}");
        assert_eq!(
            from_wide(&[104, 105, 0, 120, 121]),
            "hi",
            "stops at the first zero"
        );
        assert_eq!(from_wide(&[104, 105]), "hi", "no terminator needed");
        assert_eq!(wide(""), vec![0]);
    }

    #[test]
    fn exe_ids_are_lower_case_file_names() {
        assert_eq!(
            exe_id_from_path(r"C:\Program Files\Google\Chrome\Application\Chrome.EXE"),
            Some("chrome.exe".into())
        );
        assert_eq!(exe_id_from_path("C:/x/Code.exe"), Some("code.exe".into()));
        assert_eq!(exe_id_from_path("notepad.exe"), Some("notepad.exe".into()));
        assert_eq!(exe_id_from_path(r"C:\dir\"), None);
        assert_eq!(exe_id_from_path(""), None);
    }

    #[test]
    fn only_exe_files_count_as_apps() {
        assert!(is_exe(r"C:\a\B.EXE"));
        assert!(is_exe("  x.exe "));
        assert!(!is_exe("x.lnk"));
        assert!(!is_exe("exe"));
    }

    #[test]
    fn dpi_math() {
        assert_eq!(scale_from_dpi(96), 1.0);
        assert_eq!(scale_from_dpi(144), 1.5);
        assert_eq!(scale_from_dpi(192), 2.0);
        assert_eq!(scale_from_dpi(0), 1.0, "a failed lookup means 100 %");
        assert_eq!(px_to_logical(300.0, 1.5), 200.0);
        assert_eq!(logical_to_px(200.0, 1.5), 300.0);
        assert_eq!(px_to_logical(10.0, 0.0), 10.0);
        let v = 1234.0;
        assert!((logical_to_px(px_to_logical(v, 1.25), 1.25) - v).abs() < 1e-9);
    }

    #[test]
    fn registry_and_command_lines() {
        assert_eq!(
            run_value(r"C:\Apps\cursorpie.exe"),
            r#""C:\Apps\cursorpie.exe""#
        );
        assert_eq!(shell_params("echo hi"), r#"/S /C "echo hi""#);
        assert_eq!(
            shell_params(r#""C:\a b\x.exe" /q"#),
            r#"/S /C ""C:\a b\x.exe" /q""#
        );
        assert_eq!(select_params(r"C:\a\b.json"), r#"/select,"C:\a\b.json""#);
    }

    #[test]
    fn the_file_filter_is_double_zero_terminated() {
        for kind in [FileKind::Image, FileKind::Json, FileKind::App] {
            let f = filter_for(kind);
            assert_eq!(&f[f.len() - 2..], &[0, 0]);
            assert_eq!(
                f.iter().filter(|&&c| c == 0).count(),
                3,
                "description, pattern, end"
            );
        }
        let json = String::from_utf16_lossy(&filter_for(FileKind::Json));
        assert!(json.contains("*.json"));
        assert!(String::from_utf16_lossy(&filter_for(FileKind::Image)).contains("*.webp"));
    }

    #[test]
    fn bgra_becomes_rgba() {
        let mut px = [10, 20, 30, 128, 1, 2, 3, 255];
        bgra_to_rgba(&mut px, None);
        assert_eq!(px, [30, 20, 10, 128, 3, 2, 1, 255]);
    }

    #[test]
    fn an_icon_without_alpha_becomes_opaque_except_where_masked() {
        let mut px = [1, 2, 3, 0, 4, 5, 6, 0];
        bgra_to_rgba(&mut px, Some(&[0, 1]));
        assert_eq!(px, [3, 2, 1, 255, 6, 5, 4, 0]);
        let mut all = [1, 2, 3, 0];
        bgra_to_rgba(&mut all, None);
        assert_eq!(all[3], 255);
    }
}

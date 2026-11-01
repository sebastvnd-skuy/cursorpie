//! The icon set: names and code points of the icons in `assets/fonts/Icons-*.ttf`.
//!
//! Every icon of the interface, and every built-in slice icon, is a glyph of one small icon font
//! (Phosphor, MIT license, see `assets/fonts/LICENSE-Phosphor.txt`). That keeps them all the same
//! weight and style, makes them crisp at any scale, and lets one color tint them, which a bitmap or
//! an emoji cannot do. The font is registered by [`fonts`](super::fonts); this module is only the
//! list of what it holds.
//!
//! * The interface uses the typed constants, for example `symbols::PLUS`, drawn with
//!   [`fonts::icon`](super::fonts::icon).
//! * A slice stores an icon as the string `ico:<name>` (`ico:copy`), see [`slice_icon`]. The old
//!   letter icons of earlier versions (`C/V`, `Tab`, `>_`...) are mapped to icons by [`legacy`], so
//!   a config written before this set existed shows pictures too, without being rewritten.
//!
//! To add an icon: add a line to the list below (the name is the one Phosphor uses, the code point
//! is in its stylesheet) and run `scripts/subset-fonts.py`, which rebuilds the font with it.

/// Prefix of a slice icon that names an icon of this set.
pub const PREFIX: &str = "ico:";

macro_rules! symbols {
    ($($(#[$meta:meta])* $ident:ident $name:literal $code:literal,)*) => {
        $($(#[$meta])* pub const $ident: char = $code;)*

        /// Every icon as `(name, glyph)`, in the order of the list.
        pub const ALL: &[(&str, char)] = &[$(($name, $ident)),*];
    };
}

symbols! {
    // Window and navigation
    CHART_PIE_SLICE "chart-pie-slice" '\u{e15a}',
    SLIDERS_HORIZONTAL "sliders-horizontal" '\u{e434}',
    GEAR_SIX "gear-six" '\u{e272}',
    INFO "info" '\u{e2ce}',
    CURSOR_CLICK "cursor-click" '\u{e7c8}',
    USER_CIRCLE "user-circle" '\u{e4c4}',
    SIGN_OUT "sign-out" '\u{e42a}',
    POWER "power" '\u{e3da}',
    // Actions of the interface
    ARROW_COUNTER_CLOCKWISE "arrow-counter-clockwise" '\u{e038}',
    ARROW_CLOCKWISE "arrow-clockwise" '\u{e036}',
    X "x" '\u{e4f6}',
    CHECK "check" '\u{e182}',
    PLUS "plus" '\u{e3d4}',
    MINUS "minus" '\u{e32a}',
    PLUS_CIRCLE "plus-circle" '\u{e3d6}',
    PENCIL_SIMPLE "pencil-simple" '\u{e3b4}',
    TRASH "trash" '\u{e4a6}',
    EXPORT "export" '\u{eaf0}',
    DOWNLOAD_SIMPLE "download-simple" '\u{e20c}',
    UPLOAD_SIMPLE "upload-simple" '\u{e4c0}',
    ARROW_SQUARE_OUT "arrow-square-out" '\u{e5de}',
    DOTS_THREE "dots-three" '\u{e1fe}',
    MAGNIFYING_GLASS "magnifying-glass" '\u{e30c}',
    FOLDER_OPEN "folder-open" '\u{e256}',
    STACK "stack" '\u{e466}',
    CLOCK_COUNTER_CLOCKWISE "clock-counter-clockwise" '\u{e1a0}',
    SPARKLE "sparkle" '\u{e6a2}',
    // Carets and status
    CARET_DOWN "caret-down" '\u{e136}',
    CARET_UP "caret-up" '\u{e13c}',
    CARET_LEFT "caret-left" '\u{e138}',
    CARET_RIGHT "caret-right" '\u{e13a}',
    CHECK_CIRCLE "check-circle" '\u{e184}',
    X_CIRCLE "x-circle" '\u{e4f8}',
    WARNING "warning" '\u{e4e0}',
    WARNING_CIRCLE "warning-circle" '\u{e4e2}',
    SEAL_CHECK "seal-check" '\u{e606}',
    CIRCLE_DASHED "circle-dashed" '\u{e602}',
    SHIELD_CHECK "shield-check" '\u{e40c}',
    // Kinds of action
    PROHIBIT "prohibit" '\u{e3de}',
    KEYBOARD "keyboard" '\u{e2d8}',
    APP_WINDOW "app-window" '\u{e5da}',
    LINK "link" '\u{e2e2}',
    TERMINAL_WINDOW "terminal-window" '\u{eae8}',
    MOUSE_SIMPLE "mouse-simple" '\u{e644}',
    IMAGE "image" '\u{e2ca}',
    SMILEY "smiley" '\u{e436}',
    GITHUB_LOGO "github-logo" '\u{e576}',
    ENVELOPE_SIMPLE "envelope-simple" '\u{e218}',
    // Slice icons: editing
    SCISSORS "scissors" '\u{eae0}',
    COPY "copy" '\u{e1ca}',
    CLIPBOARD_TEXT "clipboard-text" '\u{e198}',
    ARROW_U_UP_LEFT "arrow-u-up-left" '\u{e08a}',
    ARROW_U_UP_RIGHT "arrow-u-up-right" '\u{e08c}',
    PUSH_PIN "push-pin" '\u{e3e2}',
    PAPERCLIP "paperclip" '\u{e39a}',
    BOOKMARK_SIMPLE "bookmark-simple" '\u{e0ea}',
    SELECTION_ALL "selection-all" '\u{e746}',
    ERASER "eraser" '\u{e21e}',
    // Slice icons: navigation
    ARROW_LEFT "arrow-left" '\u{e058}',
    ARROW_RIGHT "arrow-right" '\u{e06c}',
    ARROW_UP "arrow-up" '\u{e08e}',
    ARROW_DOWN "arrow-down" '\u{e03e}',
    ARROWS_LEFT_RIGHT "arrows-left-right" '\u{e0a0}',
    ARROWS_DOWN_UP "arrows-down-up" '\u{e098}',
    HOUSE "house" '\u{e2c2}',
    ARROWS_CLOCKWISE "arrows-clockwise" '\u{e094}',
    REWIND "rewind" '\u{e6a8}',
    FAST_FORWARD "fast-forward" '\u{e6a6}',
    MAGNIFYING_GLASS_PLUS "magnifying-glass-plus" '\u{e310}',
    MAGNIFYING_GLASS_MINUS "magnifying-glass-minus" '\u{e30e}',
    ARROWS_OUT "arrows-out" '\u{e0a2}',
    ARROWS_IN "arrows-in" '\u{e09a}',
    ARROW_LINE_RIGHT "arrow-line-right" '\u{e064}',
    SWAP "swap" '\u{e83c}',
    // Slice icons: media
    PLAY "play" '\u{e3d0}',
    PAUSE "pause" '\u{e39e}',
    STOP "stop" '\u{e46c}',
    SKIP_BACK "skip-back" '\u{e5a4}',
    SKIP_FORWARD "skip-forward" '\u{e5a6}',
    SPEAKER_HIGH "speaker-high" '\u{e44a}',
    SPEAKER_SLASH "speaker-slash" '\u{e45a}',
    MICROPHONE "microphone" '\u{e326}',
    CAMERA "camera" '\u{e10e}',
    CROP "crop" '\u{e1d4}',
    VIDEO_CAMERA "video-camera" '\u{e4da}',
    MUSIC_NOTES "music-notes" '\u{e340}',
    // Slice icons: system
    LOCK_SIMPLE "lock-simple" '\u{e308}',
    LOCK_SIMPLE_OPEN "lock-simple-open" '\u{e30a}',
    WIFI_HIGH "wifi-high" '\u{e4ea}',
    BLUETOOTH "bluetooth" '\u{e0da}',
    SUN "sun" '\u{e472}',
    MOON "moon" '\u{e330}',
    BATTERY_FULL "battery-full" '\u{e0c0}',
    MONITOR "monitor" '\u{e32e}',
    PRINTER "printer" '\u{e3dc}',
    CALENDAR_BLANK "calendar-blank" '\u{e10a}',
    CLOCK "clock" '\u{e19a}',
    DEVICE_MOBILE "device-mobile" '\u{e1e0}',
    BELL "bell" '\u{e0ce}',
    GLOBE "globe" '\u{e288}',
    CPU "cpu" '\u{e610}',
    LAPTOP "laptop" '\u{e586}',
    HEADPHONES "headphones" '\u{e2a6}',
    // Slice icons: files and apps
    FOLDER_SIMPLE "folder-simple" '\u{e25a}',
    FILES "files" '\u{e710}',
    NOTE_PENCIL "note-pencil" '\u{e34c}',
    ARCHIVE "archive" '\u{e00c}',
    PACKAGE "package" '\u{e390}',
    CLOUD "cloud" '\u{e1aa}',
    ROCKET_LAUNCH "rocket-launch" '\u{e3fe}',
    BOOK_OPEN "book-open" '\u{e0e6}',
    CHART_BAR "chart-bar" '\u{e150}',
    FILE_TEXT "file-text" '\u{e23a}',
    BROWSER "browser" '\u{e0f4}',
    COLUMNS "columns" '\u{e546}',
    SQUARES_FOUR "squares-four" '\u{e464}',
    SIDEBAR_SIMPLE "sidebar-simple" '\u{ec24}',
    // Slice icons: symbols
    BUG "bug" '\u{e5f4}',
    LIGHTNING "lightning" '\u{e2de}',
    FIRE "fire" '\u{e242}',
    QUESTION "question" '\u{e3e8}',
    STAR "star" '\u{e46a}',
    FLAG "flag" '\u{e244}',
    TARGET "target" '\u{e47c}',
    DIAMOND "diamond" '\u{e1ec}',
    GIFT "gift" '\u{e276}',
    THUMBS_UP "thumbs-up" '\u{e48e}',
    HEART "heart" '\u{e2a8}',
    EYE "eye" '\u{e220}',
    EYE_SLASH "eye-slash" '\u{e224}',
    TAG "tag" '\u{e478}',
    LIGHTBULB "lightbulb" '\u{e2dc}',
    KEY "key" '\u{e2d6}',
    PAINT_BRUSH "paint-brush" '\u{e6f0}',
    EYEDROPPER "eyedropper" '\u{e568}',
    MAGIC_WAND "magic-wand" '\u{e6b6}',
    // Slice icons: code and keys
    BRACKETS_ANGLE "brackets-angle" '\u{e862}',
    BRACKETS_CURLY "brackets-curly" '\u{e860}',
    BRACKETS_SQUARE "brackets-square" '\u{e85e}',
    TERMINAL "terminal" '\u{e47e}',
    COMMAND "command" '\u{e1c4}',
    OPTION "option" '\u{e8a8}',
    ARROW_ELBOW_DOWN_LEFT "arrow-elbow-down-left" '\u{e044}',
    TEXT_AA "text-aa" '\u{e6ee}',
    HASH "hash" '\u{e2a2}',
    AT "at" '\u{e0ac}',
    CODE "code" '\u{e1bc}',
    TRANSLATE "translate" '\u{e4a2}',
    SCAN "scan" '\u{ebb6}',
}

/// The glyph of the icon called `name` (`"copy"`).
pub fn by_name(name: &str) -> Option<char> {
    ALL.iter().find(|(n, _)| *n == name).map(|&(_, c)| c)
}

/// The glyph of a slice icon string `ico:<name>`. None for any other string.
pub fn slice_icon(icon: &str) -> Option<char> {
    by_name(icon.strip_prefix(PREFIX)?)
}

/// The slice icon string for an icon of this set: `"copy"` gives `"ico:copy"`.
pub fn slice_string(name: &str) -> String {
    format!("{PREFIX}{name}")
}

/// The picture for the letter icons that earlier versions put in a config: the default slices
/// (`C/V`, `Tab`, `Shot`...) and the old catalog. Only these exact strings are mapped, so text a
/// person typed on purpose is left alone.
pub fn legacy(text: &str) -> Option<char> {
    let name = match text {
        "<>" => "arrow-u-up-left",
        "< >" => "browser",
        "[ ]" => "brackets-square",
        "{ }" => "brackets-curly",
        "C/V" => "clipboard-text",
        "+/-" => "magnifying-glass-plus",
        ">_" => "terminal-window",
        "Tab" => "columns",
        "Esc" => "x",
        "Ret" => "arrow-elbow-down-left",
        "Pg" => "arrows-down-up",
        "..." => "dots-three",
        "Shot" => "crop",
        "MC" => "squares-four",
        "Find" => "magnifying-glass",
        ":)" => "smiley",
        "Lock" => "lock-simple",
        "TV" => "squares-four",
        "F" => "folder-simple",
        "S" => "globe",
        "Go" => "rocket-launch",
        "Key" => "keyboard",
        "App" => "app-window",
        "URL" => "link",
        _ => return None,
    };
    by_name(name)
}

/// The glyph to draw for any icon string: an `ico:` name, else a legacy letter icon.
pub fn resolve(icon: &str) -> Option<char> {
    slice_icon(icon).or_else(|| legacy(icon))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_and_code_points_are_unique() {
        let names: HashSet<_> = ALL.iter().map(|(n, _)| *n).collect();
        let chars: HashSet<_> = ALL.iter().map(|(_, c)| *c).collect();
        assert_eq!(names.len(), ALL.len(), "a name is listed twice");
        assert_eq!(chars.len(), ALL.len(), "a code point is listed twice");
    }

    #[test]
    fn names_are_kebab_case() {
        for (n, _) in ALL {
            assert!(n.chars().all(|c| c.is_ascii_lowercase() || c == '-'), "{n}");
        }
    }

    #[test]
    fn slice_icons_round_trip() {
        assert_eq!(slice_icon(&slice_string("copy")), Some(COPY));
        assert_eq!(slice_icon("copy"), None, "the prefix is required");
        assert_eq!(slice_icon("ico:nonsense"), None);
        assert_eq!(resolve("ico:plus"), Some(PLUS));
    }

    #[test]
    fn every_legacy_icon_maps_to_an_icon_of_the_set() {
        for text in [
            "<>", "< >", "[ ]", "{ }", "C/V", "+/-", ">_", "Tab", "Esc", "Ret", "Pg", "...",
            "Shot", "MC", "Find", ":)", "Lock", "TV", "F", "S", "Go", "Key", "App", "URL",
        ] {
            assert!(legacy(text).is_some(), "{text}");
        }
        assert_eq!(legacy("hello"), None);
        assert_eq!(legacy("A"), None, "free text is not an old default");
    }
}

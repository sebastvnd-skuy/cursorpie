# Configuration

CursorPie keeps all settings in one JSON file. You can edit them in Settings, or by hand.

## Where the file lives

| System | `config.json` |
|---|---|
| macOS | `~/Library/Application Support/CursorPie/config.json` |
| Windows | `%APPDATA%\CursorPie\config.json` |
| Linux | `~/.config/cursorpie/config.json` (or `$XDG_CONFIG_HOME/cursorpie/`) |

**Settings > General** shows the exact path and has a button that reveals it. A change made by hand is picked
up the next time CursorPie starts. Next to the file you may find:

```text
config.json            the live config
config.backup.1.json   the version before your last Save (up to config.backup.5.json)
config.broken.json     a config that could not be read (see below)
icons/                 images copied in by a profile import
welcome_shown          empty file: the welcome window was already shown
```

Export writes `cursorpie-config.json` to your Downloads folder.

## Structure

`profiles[] > items[] > kind`, where `kind` is `Mode`, `Run` or `Submenu`. Actions are `Nothing`, `Key`,
`OpenApp`, `OpenUrl`, `Command` or `Sequence`. Enums use serde's default external tagging, for example
`{"Run": {"OpenApp": "Notes"}}`.

```json
{
  "version": 2,
  "pie_hold_ms": 500,
  "settings_hold_ms": 1200,
  "pie_size": 200.0,
  "accent": [64, 156, 255],
  "trigger_button": 2,
  "flick": false,
  "sound": false,
  "launch_at_login": false,
  "app_gesture": "DoubleClick",
  "double_click_ms": 260,
  "target_app": "UnderCursor",
  "focus_target": true,
  "desktop_mode": "SwitchToApp",
  "side_default": "App",
  "side_adopt": true,
  "side_forget": false,
  "right_click_switch": true,
  "side_indicator": true,
  "inactive_color": [232, 86, 86],
  "preview_ms": 900,
  "center_app_icon": true,
  "sleep_after_s": 60,
  "profiles": [
    {
      "name": "Default",
      "apps": [],
      "active": [0],
      "items": [
        {
          "name": "History",
          "icon": "<>",
          "kind": {
            "Mode": {
              "back": { "Key": { "key": 6, "mods": 8 } },
              "forward": { "Key": { "key": 6, "mods": 9 } }
            }
          }
        },
        {
          "name": "Notes",
          "icon": "~/Pictures/notes.png",
          "kind": { "Run": { "OpenApp": "Notes" } }
        },
        {
          "name": "Tools",
          "icon": "Go",
          "kind": {
            "Submenu": [
              { "name": "Docs", "icon": "URL", "kind": { "Run": { "OpenUrl": "https://example.com" } } },
              {
                "name": "Both",
                "icon": "2x",
                "kind": { "Run": { "Sequence": [ { "OpenApp": "Safari" }, { "OpenApp": "Notes" } ] } }
              }
            ]
          }
        }
      ]
    }
  ]
}
```

## Field reference

| Field | Meaning |
|---|---|
| `trigger_button` | `2` = middle, `5` / `6` / `7` = extra buttons (shown as "Extra button 6/7/8"). Only the middle button works on Windows and Linux |
| `pie_hold_ms` | Hold time before the pie opens (150 to 2000) |
| `settings_hold_ms` | Hold time before Settings opens (always at least `pie_hold_ms` + 300) |
| `pie_size` | Pie window size in px (140 to 360) |
| `app_gesture` | `"DoubleClick"` (hold = general pie, double-click = app pie) or `"Merged"` (classic) |
| `double_click_ms` | Window for the second click (150 to 500) |
| `target_app` | `"UnderCursor"` (app whose window is under the pointer) or `"Focused"` (app in front) |
| `focus_target` | With `UnderCursor`: bring that app to the front before sending it a shortcut |
| `desktop_mode` | `"SwitchToApp"` (default), `"BringToCurrent"` or `"DoNothing"`: what an Open app slice does when the app's window is on another desktop or Space. `BringToCurrent` is macOS only (experimental) and acts as `SwitchToApp` elsewhere. See [OTHER_DESKTOPS.md](OTHER_DESKTOPS.md) |
| `side_default` | `"App"` or `"General"`: the pie the side buttons follow in an app that has its own pie, until you choose |
| `side_adopt` | Picking a side-button mode in a pie makes the side buttons follow that pie |
| `side_forget` | Forget that choice when another app becomes the target |
| `right_click_switch` | Right-click while the pie is open to switch the side-button pie |
| `side_indicator` | Color the pie ring by who owns the side buttons; `inactive_color` is the "other pie" color |
| `preview_ms` | How long the pie stays on screen after switching from the menu bar (0 to 3000, 0 = off) |
| `center_app_icon` | Show the app's icon (white glyph) in the center of an app's own pie instead of the active mode's icon |
| `sleep_after_s` | Seconds without a pie or Settings before cached icons and the Settings copy are freed (10 to 3600, `0` = never) |
| `apps` | App ids: bundle identifiers on macOS, executable names on Windows, desktop-entry ids on Linux. An empty list marks the general profile; only the first such profile is kept, all others need at least one app |
| `active` | Path of the active side-button mode, as indices (`[3, 1]` = slice 1 inside slice 3) |
| `Key` | `key` is a macOS virtual key code (a physical key; Windows and Linux map it to the key in the same position). `mods` is a bitmask: Shift = 1, Ctrl = 2, Opt/Alt = 4, Cmd/Win/Super = 8 |

## What happens to bad files

- Out-of-range values are clamped when the file is loaded. Rings are trimmed to 8 slices and menus to 5 levels.
- A file without a `"version"` field (the old v1 format) or with invalid JSON is rejected: CursorPie copies it
  to `config.broken.json` next to the original and starts with the defaults.
- The next save (pressing Save in Settings, or picking a side-button mode in the pie) overwrites `config.json`,
  so fix or move the broken file first if you want to keep your edits.
- A config from another system still loads. App ids of the other system are kept but do not match anything.

## Security

`Command` actions run shell commands with your user's permissions. **Only import configs and profiles you
trust.** CursorPie warns you after importing a file that contains any.

## For developers

How the format is versioned and how to add a setting is in the Rust docs (`guides::config_format` and
`guides::extending`; run `cargo docs`).

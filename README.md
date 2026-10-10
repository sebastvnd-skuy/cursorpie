# CursorPie

A hold-to-open pie menu for macOS, inspired by [Kando](https://github.com/kando-menu/kando).

Hold your middle mouse button, flick toward a slice, release. Each slice can fire a keyboard shortcut,
launch an app, open a URL, run a shell command, open a submenu, or turn your mouse's back/forward buttons
into something more useful. Profiles let every app have its own pie.

- Written in Rust, UI in [egui](https://github.com/emilk/egui) / eframe, macOS only
- Lives in the menu bar: no Dock icon, no window until you need it
- Everything stays on your Mac: no network access, no telemetry

## Contents

- [How it works](#how-it-works)
- [Features](#features)
- [Install](#install)
- [First launch and permissions](#first-launch-and-permissions)
- [Using the pie](#using-the-pie)
- [Settings](#settings)
- [Configuration file](#configuration-file)
- [Project layout](#project-layout)
- [Privacy](#privacy)
- [Known limitations](#known-limitations)
- [Forking and rebranding](#forking-and-rebranding)
- [License](#license)

## How it works

CursorPie installs a global event tap that watches the mouse button you chose as the trigger
(middle button by default). What happens next depends on how you use it:

| You do this | CursorPie does this |
|---|---|
| **Hold** the trigger for 500 ms | Opens your general pie under the cursor |
| **Double-click** the trigger (default mode) | Opens the pie of the app in front, instantly, if that app has its own profile |
| **Click** quickly | Passes the click through to the app. In the default mode, if the app in front has its own profile, the click is delivered once the double-click window (260 ms) has passed |
| **Press and drag** before the pie opens | Hands the drag back to the app (so middle-drag panning still works) |
| **Release** over a slice | Runs that slice |
| **Drag outward** across the rim of a submenu slice | Opens the submenu, re-centered on the cursor |
| **Keep holding** in the center, with nothing hovered | After 1200 ms in total, opens Settings. A progress arc fills around the center disc |

In *classic* mode (see [Profiles](#features)) there is no double-click: holding opens the app's own pie if it has
one, otherwise the general pie.

The timings above are the defaults and can be changed in **Settings > Behavior**.

## Features

**Slices.** A slice is one of three kinds:

- **Do something (Run).** Runs an action when you release the button over it.
- **Side buttons (Mode).** Selecting it makes your mouse's back and forward buttons send the actions you
  assigned to this mode (for example Undo/Redo, or Cmd+[ / Cmd+]). The active mode is remembered per profile
  and shown in the center of the pie. If a mode's action is *None*, the native back/forward click passes through.
- **Submenu.** A nested ring. Rings hold up to 8 slices, and menus can go up to 5 levels deep
  (the top level counts as level 1).

**Actions.** Available for Run slices and for each side button of a Mode:

| Action | What it does |
|---|---|
| None | Nothing. For side buttons, the click passes through to the app |
| Shortcut | Sends a key combination. Record it by clicking the button and pressing the keys (Esc cancels) |
| Open app | `open -a <name or path>`, e.g. `Safari` or `/Applications/Safari.app` |
| Open URL / file | `open <target>`, e.g. `https://example.com` or a file path |
| Shell command | Runs `/bin/sh -c "<command>"` |
| Sequence | Several actions in order, 40 ms apart. Edit these in `config.json` (see below) |

Actions run on one worker thread so they always execute in the order you triggered them.

**Profiles.**

- Up to 16 profiles. Each lists app bundle identifiers (for example `com.apple.Safari`).
- A profile with **no apps** is the general (fallback) profile.
- Two ways to combine them, switchable in **Behavior**:
  - *Hold = general pie, double-click = pie of the app* (default)
  - *Classic: hold opens the app's own pie if it has one*, otherwise the general pie

**Looks and feel.**

- Slice icons can be text, emoji, or an image (`.png`, `.jpg`, `.jpeg`, `.webp`). A path may start with `~/`.
- Adjustable pie size (140 to 360 px), accent color, and timings.
- Optional soft tick sound when the hovered slice changes.
- Apps with their own profile get an accent-colored outer ring, so you can tell at a glance which pie you opened.
- Optional **instant flick**: drag right after pressing to open the pie with no hold delay
  (this disables middle-drag pass-through in other apps).
- The trigger can be the middle button or one of three extra mouse buttons.

**Menu bar.** Settings, About, Pause, Launch at login, Quit. While paused, every event passes through untouched.

**Import / export.** Save your whole setup as JSON and share it, or load someone else's.

### Default setup

A fresh install comes with three profiles:

| Profile | Apps | Slices |
|---|---|---|
| Default (general) | all apps without a profile | History (undo/redo), Clipboard (copy/paste), Brackets (`[` `]`), and a *Launch* submenu with Finder, Terminal, Safari and Shot (screenshot) |
| Browsers | Safari, Chrome, Firefox, Arc, Brave, Edge | Browser (back/forward), Tabs (previous/next tab), Zoom, Scroll (Page Up/Down) |
| Code | VS Code, Xcode | History, Brackets, Edit (Return/Delete) |

Ready-made slices you can add from the **+** button:

- *Side-button modes:* History, Brackets, Clipboard, Browser, Tabs, Zoom, Scroll, Edit
- *Quick actions:* Screenshot, Mission Ctrl, Spotlight, Emoji, Lock
- *Custom:* Shortcut, Open app, Open URL, Command, Submenu

## Install

CursorPie is built from source. You need **macOS 11 or later** and **Rust 1.88 or later** (edition 2024).

```sh
git clone https://github.com/sebastvnd/cursorpie.git
cd cursorpie

cargo run --release        # quick try; grant permissions to your terminal app
scripts/bundle.sh          # builds target/bundle/CursorPie.app
```

`scripts/bundle.sh` builds a release binary, wraps it in a menu-bar-only `.app` (`LSUIElement`), and signs it.
By default the signature is ad-hoc. To sign with a stable identity instead:

```sh
SIGN_ID="Developer ID Application: Your Name (TEAMID)" scripts/bundle.sh
```

Then move `CursorPie.app` to `/Applications` and open it.

## First launch and permissions

CursorPie needs the **Accessibility** permission to watch the mouse and to send shortcuts. macOS may also ask
for **Input Monitoring**.

1. Open **System Settings > Privacy & Security**.
2. Enable CursorPie (or your terminal app, if you used `cargo run`) under **Accessibility**, and under
   **Input Monitoring** if asked.

If access is missing, CursorPie retries every 2 seconds, so it starts working as soon as you grant it.
If it still reports *Event tap: Not running* afterwards, quit and reopen it. **Settings > General** shows the
current status and has buttons that jump straight to the right System Settings pages.

## Using the pie

1. Hold the middle mouse button (500 ms by default). The pie appears centered on the cursor.
2. Move toward a slice. It highlights as you hover.
3. Release to run it.

Tips:

- **Submenus** are marked with a dot on the rim. Move past the rim over such a slice and the next ring opens
  where your cursor is. Release over a slice in it to run that slice.
- **Side-button modes:** release over a mode slice to activate it. From then on, back/forward on your mouse
  send that mode's actions, in any app that uses the same profile. While the pie is open, pressing a side
  button over a mode slice sends that mode's actions without selecting it.
- **Cancel** by releasing in the center dead zone.
- Open **Settings** by holding in the center until the progress arc completes, or from the menu bar icon.

## Settings

Open it from the menu bar icon or by holding the trigger in the center of the pie.
**Save** applies and stores your changes. **Cancel**, **Esc** and **Cmd+W** discard them.

**Menu**

- Choose, create, rename and delete profiles, and list the apps each one applies to
  (type a bundle identifier or use **Pick app...**).
- An interactive pie shows the ring you are editing. Click a slice to edit it, click **+** to add one,
  click a selected submenu slice again to step into it, and click the center to go back.
- Per slice: name, icon (text, emoji or image), kind, action, and Move / Delete.

**Behavior**

- Trigger button, hold time for the pie, hold time for Settings, pie size, accent color
- Pie of the app in front: general + double-click, or classic merged mode, plus the double-click speed
- Instant flick, hover sound, launch at login

**General**

- Permission status and shortcuts to the System Settings pages
- Location of the config file, **Reveal in Finder**, **Export...**, **Import...**, **Reset to defaults**

**About**

- Version, author, project links, privacy note and credits

## Configuration file

Settings are stored in:

```
~/Library/Application Support/CursorPie/config.json
```

You can edit it in Settings or by hand. A change made by hand is picked up the next time CursorPie starts.

Structure: `profiles[] > items[] > kind`, where `kind` is `Mode`, `Run` or `Submenu`.
Actions are `Nothing`, `Key`, `OpenApp`, `OpenUrl`, `Command` or `Sequence`.

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

Reference:

| Field | Meaning |
|---|---|
| `trigger_button` | `2` = middle, `5` / `6` / `7` = extra buttons (shown as "Extra button 6/7/8") |
| `pie_hold_ms` | Hold time before the pie opens (150 to 2000) |
| `settings_hold_ms` | Hold time before Settings opens (always at least `pie_hold_ms` + 300) |
| `pie_size` | Pie window size in px (140 to 360) |
| `app_gesture` | `"DoubleClick"` (hold = general pie, double-click = app pie) or `"Merged"` (classic) |
| `double_click_ms` | Window for the second click (150 to 500) |
| `apps` | Bundle identifiers. An empty list marks the general (fallback) profile |
| `active` | Path of the active side-button mode, as indices (`[3, 1]` = slice 1 inside slice 3) |
| `Key` | `key` is a macOS virtual key code (physical key). `mods` is a bitmask: Shift = 1, Ctrl = 2, Opt = 4, Cmd = 8 |

Out-of-range values are clamped when the file is loaded. Rings are trimmed to 8 slices and menus to 5 levels.
A file without a `"version"` field (the old v1 format) or with invalid JSON is rejected: CursorPie copies it to
`config.broken.json` next to the original and starts with the defaults. The next save (pressing Save in
Settings, or picking a side-button mode in the pie) then overwrites `config.json`, so fix or move the broken
file first if you want to keep your edits.

**Security:** `Command` actions run shell commands with your user's permissions. Only import configs you trust.
CursorPie warns you after importing a file that contains any.

Export writes `~/Downloads/cursorpie-config.json`.

## Project layout

```
Cargo.toml              dependencies, release profile (opt-level 3, thin LTO, stripped)
scripts/bundle.sh       builds and signs target/bundle/CursorPie.app
icon/                   sample images for icons (brackets, clipboard); not loaded automatically
src/
  main.rs               entry point: transparent, borderless, always-on-top window; starts the threads
  about.rs              name, version, bundle id, author and project links (edit to rebrand)
  app.rs                eframe app: reacts to messages from the other threads, opens/closes Settings
  tap.rs                global event tap and the gesture state machine
  actions.rs            action worker thread; sends shortcuts, opens apps/URLs, runs commands
  config.rs             config model, defaults, presets, validation, load/save, import/export
  keys.rs               shortcut model (key code + modifiers) and key labels
  pie.rs                draws the pie and its animations
  gfx.rs                palette, pie geometry and hit-testing, icon loading, drawing helpers
  msg.rs                messages sent to the UI thread
  system.rs             macOS glue: permissions, frontmost app, launch at login, file pickers, hover tick
  tray.rs               menu bar icon and menu
  settings/             the Settings window
    window.rs           card, open/close animation, tab bar, Save / Cancel / Quit
    state.rs            editor-only state (tabs, selection, shortcut recording, icon cache)
    action_editor.rs    editor for a single action
    items_editor.rs     editor for one ring of slices, and the "add a slice" panel
    pie_designer.rs     interactive pie used to pick and arrange slices
    tabs/               Menu, Behavior, General and About tabs
```

How the pieces fit together:

- **Threads.** The UI runs on the main thread. The event tap runs on its own thread (`cursorpie-tap`) with a
  CoreFoundation run loop, actions run on `cursorpie-actions`, and short-lived threads handle hold timers and
  file pickers. They talk to the UI through a channel of `Msg` values that also wakes the UI up.
- **One window for everything.** The pie is a transparent, click-through, always-on-top window that moves to
  the cursor. Settings reuses the same window: it is resized, made interactive, and centered on the screen that
  contains the cursor.
- **Shared config.** One live `Config` behind a mutex is read by both the UI and the event tap. Settings edits a
  copy and writes it back when you press Save.
- **A careful tap.** The event callback never blocks and never panics out of it (panics are caught and the event
  passes through). If macOS disables the tap, it is re-enabled and any half-finished gesture is reset.
  Events that CursorPie posts itself carry a marker so the tap ignores them.
- **No extra tooling.** File choosers use `osascript`, bundle identifiers come from `mdls`, and the hover tick
  uses `afplay`, so there is nothing else to install.

## Privacy

CursorPie works entirely on your Mac. The app makes no network connections and collects no data.

The event tap receives key events so that Settings can capture a shortcut while you record one. Outside of
recording they are passed on untouched and are never stored or logged.

Actions you configure yourself (opening URLs, running commands) do whatever you tell them to.

## Known limitations

- macOS only. The build fails on other platforms.
- Shortcut labels and the shifted-symbol display assume a US/ANSI layout. Key codes themselves are physical keys.
- The back/forward side buttons are fixed to mouse buttons 4 and 5 (macOS button numbers 3 and 4).
  Only the trigger button is configurable.
- With a per-app profile and the default gesture, a plain middle-click in that app is delivered after the
  double-click window (260 ms by default). Use classic mode, or a shorter window, if that bothers you.
- Sequences cannot be edited in Settings yet, only in `config.json`.
- Launch at login installs a LaunchAgent that starts CursorPie at your next login. It points at the executable
  that was running when you enabled it, so toggle it off and on again if you move the app.
- Ad-hoc signed builds may need the Accessibility permission re-granted after each rebuild. Sign with a stable
  identity via `SIGN_ID` to avoid that.

## Forking and rebranding

Edit `src/about.rs` (name, author, links, bundle identifier), `Cargo.toml` and `LICENSE`.
Keep `BUNDLE_ID` in `src/about.rs` and in `scripts/bundle.sh` identical: the first is used for the launch-at-login
agent, the second for the `.app` bundle.

## License

MIT. See [LICENSE](LICENSE).

Built with egui / eframe, core-graphics, tray-icon / muda, objc2, serde and image.
Inspired by [Kando](https://github.com/kando-menu/kando).

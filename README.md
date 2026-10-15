# CursorPie

A hold-to-open pie menu for macOS and Windows, inspired by [Kando](https://github.com/kando-menu/kando).

**Version 0.3.0 adds Windows 10/11 support.** It behaves almost identically to macOS; the one gap is extra mouse
buttons (on Windows the trigger can only be the middle button). Details: [docs/WINDOWS_MIGRATION.md](docs/WINDOWS_MIGRATION.md).

Hold your middle mouse button, flick toward a slice, release. Each slice can fire a keyboard shortcut,
launch an app, open a URL, run a shell command, open a submenu, or turn your mouse's back/forward buttons
into something more useful. Profiles let every app have its own pie.

- Written in Rust, UI in [egui](https://github.com/emilk/egui) / eframe, for macOS 11+ and Windows 10/11
- Lives in the menu bar (macOS) or the system tray (Windows): no Dock icon or taskbar button, no window until you need it
- Light on memory: icons are cached while you use it and freed when it has been idle
- Settings has **Undo / Redo**, and every Save keeps a backup of the version it replaces
- Everything stays on your computer: no network access, no telemetry

## Contents

- [How it works](#how-it-works)
- [Features](#features)
- [Install](#install)
- [First launch and permissions](#first-launch-and-permissions)
- [Using the pie](#using-the-pie)
- [Settings](#settings)
- [Undo, redo and backups](#undo-redo-and-backups)
- [Sharing a profile](#sharing-a-profile)
- [Memory and performance](#memory-and-performance)
- [Look and rendering](#look-and-rendering)
- [Configuration file](#configuration-file)
- [Project layout](#project-layout)
- [Developer documentation](#developer-documentation)
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
| **Double-click** the trigger (default mode) | Opens the pie of the app under the pointer, instantly, if that app has its own profile |
| **Right-click** while the pie is open | Switches which pie the side buttons follow (general or the app's own); the ring changes color at once. See [Side buttons and app pies](#side-buttons-and-app-pies) |
| **Click** quickly | Passes the click through to the app. In the default mode, if the app in front has its own profile, the click is delivered once the double-click window (260 ms) has passed |
| **Press and drag** before the pie opens | Hands the drag back to the app (so middle-drag panning still works) |
| **Release** over a slice | Runs that slice |
| **Drag outward** across the rim of a submenu slice | Opens the submenu, re-centered on the cursor |
| **Keep holding** in the center, with nothing hovered | After 1200 ms in total, opens Settings. A progress arc fills around the center disc |

In *classic* mode (see [Profiles](#features)) there is no double-click: holding opens the app's own pie
if it has one, otherwise the general pie.

The timings above are the defaults and can be changed in **Settings > Behavior**.

## Features

**Slices.** A slice is one of three kinds:

- **Do something (Run).** Runs an action when you release the button over it.
- **Side buttons (Mode).** Selecting it makes your mouse's back and forward buttons send the actions you
  assigned to this mode (for example Undo/Redo, or Cmd+[ / Cmd+] on macOS, Alt+Left / Alt+Right on Windows). The active mode is remembered per profile
  and shown in the center of the pie. If a mode's action is *None*, the native back/forward click passes through.
- **Submenu.** A nested ring. Rings hold up to 8 slices, and menus can go up to 5 levels deep
  (the top level counts as level 1).

**Actions.** Available for Run slices and for each side button of a Mode:

| Action | What it does |
|---|---|
| None | Nothing. For side buttons, the click passes through to the app |
| Shortcut | Sends a key combination. Record it by clicking the button and pressing the keys (Esc cancels) |
| Open app | macOS: `open -a <name or path>`, e.g. `Safari` or `/Applications/Safari.app`. Windows: a program name or the path of an `.exe`, e.g. `chrome` or `C:\Tools\App.exe` |
| Open URL / file | `open <target>`, e.g. `https://example.com` or a file path |
| Shell command | macOS: runs `/bin/sh -c "<command>"`. Windows: runs `cmd /C "<command>"` without a console window |
| Sequence | Several actions in order, 40 ms apart. Edit these in `config.json` (see below) |

Actions run on one worker thread so they always execute in the order you triggered them.

**Profiles.**

- Up to 16 profiles. Each lists app ids: bundle identifiers on macOS (for example `com.apple.Safari`), executable names on Windows (for example `chrome.exe`, not case sensitive).
- There is **exactly one general profile**: the one with **no apps**, used for every app that has no
  profile of its own. It is created for you and cannot be deleted.
- Every other profile **belongs to at least one app**. **New profile** asks for a name and an app, and the
  last app of a profile cannot be removed. (A config file that has a second profile without apps loses it
  when loaded: such a profile could never be chosen.)
- Two ways to combine them, switchable in **Behavior**:
  - *Hold = general pie, double-click = pie of the app* (default)
  - *Classic: hold opens the app's own pie if it has one*, otherwise the general pie

**Looks and feel.**

- Slice icons can be text, emoji, or an image (`.png`, `.jpg`, `.jpeg`, `.webp`). A path may start with `~/`.
- Adjustable pie size (140 to 360 px), accent color, and timings.
- The Settings window has a modern, dense "pro tool" look: a graphite window with rounded corners, a top bar
  with the tabs and the Undo / Redo / Cancel / Save buttons (always in the same place), hairline borders,
  small monospace labels and a status footer. The accent color is the only saturated color. See
  [Look and rendering](#look-and-rendering).
- The pie and Settings are drawn with anti-aliased edges and a fixed-duration open animation, so they stay
  sharp and smooth on standard and Retina screens.
- Optional soft tick sound when the hovered slice changes.
- In an app that has its own pie, the outer ring shows which pie the side buttons follow: accent color = this pie,
  red (changeable) = the other one. See [Side buttons and app pies](#side-buttons-and-app-pies).
- The center of an app's own pie shows that app's real icon (the one the Dock shows) reduced to a white glyph;
  the general pie, and every pie when this is switched off, shows the icon of the active side-button mode.
- Optional **instant flick**: drag right after pressing to open the pie with no hold delay
  (this disables middle-drag pass-through in other apps).
- The trigger can be the middle button or one of three extra mouse buttons.

**Menu bar.** Settings, About, Pause, Launch at login, Quit. While paused, every event passes through untouched.

**Import / export.** Save your whole setup as JSON and share it, or load someone else's. To share just one
pie, use **Share** in the Menu tab: see [Sharing a profile](#sharing-a-profile).

### Default setup

A fresh install comes with three profiles:

| Profile | Apps | Slices |
|---|---|---|
| Default (general) | all apps without a profile | History (undo/redo), Clipboard (copy/paste), Brackets (`[` `]`), and a *Launch* submenu with Finder, Terminal, Safari and Shot (screenshot) on macOS, or Explorer, Terminal, Edge and Shot on Windows |
| Browsers | Safari, Chrome, Firefox, Arc, Brave, Edge | Browser (back/forward), Tabs (previous/next tab), Zoom, Scroll (Page Up/Down) |
| Code | VS Code, Xcode | History, Brackets, Edit (Return/Delete) |

Ready-made slices you can add from the **+** button:

- *Side-button modes:* History, Brackets, Clipboard, Browser, Tabs, Zoom, Scroll, Edit
- *Quick actions:* Screenshot, Mission Ctrl, Spotlight, Emoji, Lock
- *Custom:* Shortcut, Open app, Open URL, Command, Submenu

## Install

CursorPie is built from source. You need **macOS 11 or later** or **Windows 10 (1809) / 11**, and **Rust 1.88 or later** (edition 2024).

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

On **Windows** (PowerShell, with the MSVC build tools installed):

```powershell
cargo run --release              # quick try
scripts\package-windows.ps1      # builds target\bundle\CursorPie-windows.zip with cursorpie.exe
```

Run `cursorpie.exe`; a pie icon appears in the system tray. Windows needs no permission. To control windows of
programs that run as administrator, start CursorPie as administrator too (Windows blocks input to them
otherwise). Tick **Launch at login** in the tray menu to start it with Windows.

## First launch and permissions

*Windows needs no permission; skip this section.* On macOS, CursorPie needs the **Accessibility** permission to watch the mouse and to send shortcuts. macOS may also ask
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
- Open **Settings** by holding in the center until the progress arc completes, or from the menu bar / tray icon.

## Side buttons and app pies

An app with its own profile has two pies: its own and the general one. The side buttons (back / forward) follow
exactly one of them, and each pie shows which with its ring color: **accent = the side buttons follow this pie,
red = they follow the other one**.

- **Which one by default:** the app's pie (**Settings > Behavior > Side buttons**; you can pick the general pie instead).
- **Choosing:** pick a side-button mode in a pie and the side buttons follow that pie from then on, in that app.
  Open the general pie by holding the trigger, pick e.g. *History*, and Undo/Redo is on your side buttons even in an
  app with its own pie.
- **Switching:** hold the trigger so the pie is open, then **right-click**. The pie stays open and its ring turns
  blue (it now owns the side buttons) or red (the other pie does). Without opening a pie, use *Switch side-button pie*
  in the menu bar: the pie that now owns the side buttons appears for a moment (time adjustable, 0 turns it off).
- The choice is remembered per app while CursorPie runs. It can be forgotten when you switch to another app, and
  saving Settings clears it.

**Which app is "the app"?** By default the app whose window is **under the pointer**, so you don't have to click a
window to focus it first. Shortcuts always go to the app in front, so with this mode CursorPie brings that app to the
front (`open -b`) right before it sends a shortcut, unless the app is already in front. Both are options: switch back
to *the app in front* if you prefer the old behavior. If CursorPie cannot tell which window is under the pointer, it
uses the app in front.

## Settings

Open it from the menu bar icon or by holding the trigger in the center of the pie.
**Save** applies and stores your changes. **Cancel**, **Esc** and **Cmd+W** (Ctrl+W on Windows) discard them.
**Undo** and **Redo** (top bar, or **Cmd+Z** / **Cmd+Shift+Z**, Ctrl on Windows) step back and forward through your
edits while Settings is open; see [Undo, redo and backups](#undo-redo-and-backups).

**Menu**

- Choose, create (name + app), rename and delete profiles, list the apps each one applies to, and
  **Share** a profile (export / import, icons included)
  (type a bundle identifier or executable name, or use **Pick app...**).
- An interactive pie shows the ring you are editing. Click a slice to edit it, click **+** to add one,
  click a selected submenu slice again to step into it, and click the center to go back.
- Per slice: name, icon (text, emoji or image), kind, action, and Move / Delete.

**Behavior**

- Trigger button, hold time for the pie, hold time for Settings, pie size, accent color
- Pie of the app in front: general + double-click, or classic merged mode, plus the double-click speed
- App detection: app under the pointer or app in front, and whether to bring it to the front before sending shortcuts
- Side buttons: which pie they follow by default, pick-to-follow, forget on app switch, right-click switching, ring colors, preview time
- Instant flick, hover sound, app icon in the pie center, launch at login
- Memory: free cached icons and the Settings working copy after an idle time (on by default, 60 s)

**General**

- Permission status and shortcuts to the System Settings pages
- Location of the config file, **Reveal in Finder** (Explorer on Windows), **Export...**, **Import...**, **Reset to defaults**
- **Backups**: one button per saved earlier version, to load it back into the editor

**About**

- Version, author, project links, privacy note and credits

## Undo, redo and backups

Deleting a slice or a whole profile by accident is easy to fix.

**While Settings is open**, every change you make is recorded: adding, deleting or moving slices and
profiles, editing actions and apps, changing any option, importing a file and **Reset to defaults**.

- **Undo** / **Redo** are in the top bar, or press **Cmd+Z** and **Cmd+Shift+Z** (**Cmd+Y** also redoes; Ctrl instead of Cmd on Windows).
- Typing and dragging a slider count as **one** step, not one per keystroke: a step ends when you pause
  for a moment.
- Up to 50 steps are kept. Making a new change after an undo discards the steps you undid.
- While you type in a text field, **Cmd+Z** undoes the text first. While you record a shortcut, the keys
  go to the recorder instead.
- The history lives only as long as the Settings window: **Save**, **Cancel** and **Esc** close it and
  forget it. Which slice is selected is not part of it.

**After you have saved**, use the backups. Every **Save** first stores the version it replaces as
`config.backup.1.json` (the newest) next to `config.json`, and keeps the last five. Open
**Settings > General > Backups** and pick one: it is loaded into the editor, and nothing changes until you
press **Save**. Undo still brings back what you had before restoring.

```
~/Library/Application Support/CursorPie/      (%APPDATA%\CursorPie\ on Windows)
  config.json            the live config
  config.backup.1.json   the version before your last Save (up to config.backup.5.json)
  config.broken.json     a config that could not be read (see below)
```

A Save that would store exactly what is already in `config.json` makes no backup, and `config.json`
is written through a temporary file, so a crash or a full disk cannot leave half a file.

## Sharing a profile

**Menu > Share** moves one profile between Macs, with its icons, so it looks the same on arrival.

- **Export this profile...** writes `~/Downloads/<name>.cursorpie-profile.json` and reveals it in Finder.
  Every image icon the profile uses (including those in submenus) is embedded in the file, up to 2 MB
  each. Icons of apps (`app:`), text and emoji need nothing embedded. Missing or oversized images are left
  out and show up as `?` for the receiver.
- **Import a profile...** adds the file as a **new** profile (nothing is overwritten) and selects it.
  The embedded images are copied to `~/Library/Application Support/CursorPie/icons/` (an identical file is
  reused, a different one with the same name gets a number) and the slices are pointed at the copies.
  Press **Save** to apply it, or **Undo** to take it back.
- The profile must list at least one app; a name that is already taken gets ` (2)`, ` (3)`...
- Only PNG, JPEG and WebP data is written, whatever the file claims, and nothing outside the icons folder.
- **Only import profiles you trust:** actions can run shell commands. CursorPie warns you when an imported
  profile contains any, and nothing runs until you press Save and then use the slice.

## Memory and performance

CursorPie sits in the menu bar all day, so it is built to cost almost nothing while you are not using it:

- **No drawing while idle.** The window is hidden and nothing repaints until you press the trigger.
- **Icons are cached.** Decoding an app icon or an image is the slowest part of opening a pie, so the
  decoded icons are kept (at most 96) and shared by the pie and Settings. Closing Settings no longer
  throws them away: only image files that changed on disk are read again.
- **Icons are loaded before the pie opens.** Pressing the trigger tells the UI which pies may open, and it
  loads their icons during the hold time (500 ms by default), so the pie appears without a hitch.
- **Sleep.** After 60 seconds without a pie or Settings (**Settings > Behavior > Memory**), CursorPie drops
  the cached icons and the Settings working copy and asks macOS to take the freed pages back, so Activity
  Monitor shows the memory as released. The next press loads what the pie needs again. Set the option off
  to keep everything in memory.
- **Permission checks are throttled.** Settings asks macOS about the Accessibility permission at most
  once a second instead of on every frame.

What sleep cannot free: the text rendering and graphics buffers that the UI library keeps for the process
lifetime. That is a fixed base cost, not something that grows with use.

## Look and rendering

The interface is drawn by [egui](https://github.com/emilk/egui), which has no native widgets, so how
sharp and smooth it looks depends on how it is drawn. CursorPie takes care of the following:

- **Anti-aliased slices.** The slices of the pie are custom meshes, which egui draws as they are. Each slice
  gets a one-pixel fade on every edge (see `ui::draw::ring_slice_mesh`), the same trick egui uses for its
  own shapes, so edges are smooth instead of stair-stepped. The feather is one *physical* pixel, so it is
  right on Retina and on standard screens.
- **Crisp window.** The Settings window is snapped to whole pixels, has a rounded one-point outline and no
  gradient mesh border. The pie window is placed on a whole point.
- **Sharp icons.** Image icons are premultiplied and scaled with a Lanczos filter when they are loaded
  (at most 128 px), which avoids dark halos and shimmering edges.
- **Smooth animation.** The pie opens in 0.22 s and closes in 0.12 s on a linear clock with one easing
  curve on top, so the duration is the same at 60 and 144 fps and the end is crisp.

If an edge still looks soft on your screen, open an issue and mention your display scaling. The design
rules and how to test rendering changes are in [docs/UI_DESIGN.md](docs/UI_DESIGN.md).

## Configuration file

Settings are stored in:

```
~/Library/Application Support/CursorPie/config.json      (macOS)
%APPDATA%\CursorPie\config.json                           (Windows)
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
  "target_app": "UnderCursor",
  "focus_target": true,
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

Reference:

| Field | Meaning |
|---|---|
| `trigger_button` | `2` = middle, `5` / `6` / `7` = extra buttons (shown as "Extra button 6/7/8") |
| `pie_hold_ms` | Hold time before the pie opens (150 to 2000) |
| `settings_hold_ms` | Hold time before Settings opens (always at least `pie_hold_ms` + 300) |
| `pie_size` | Pie window size in px (140 to 360) |
| `app_gesture` | `"DoubleClick"` (hold = general pie, double-click = app pie) or `"Merged"` (classic) |
| `double_click_ms` | Window for the second click (150 to 500) |
| `target_app` | `"UnderCursor"` (app whose window is under the pointer) or `"Focused"` (app in front) |
| `focus_target` | With `UnderCursor`: bring that app to the front before sending it a shortcut |
| `side_default` | `"App"` or `"General"`: pie the side buttons follow in an app that has its own pie, until you choose |
| `side_adopt` | Picking a side-button mode in a pie makes the side buttons follow that pie |
| `side_forget` | Forget that choice when another app becomes the target |
| `right_click_switch` | Right-click while the pie is open to switch the side-button pie |
| `side_indicator` | Color the pie ring by who owns the side buttons; `inactive_color` is the "other pie" color |
| `preview_ms` | How long the pie stays on screen after switching from the menu bar (0 to 3000, 0 = off) |
| `center_app_icon` | Show the app's icon (white glyph) in the center of an app's own pie instead of the active mode's icon |
| `sleep_after_s` | Seconds without a pie or Settings before cached icons and the Settings copy are freed (10 to 3600, `0` = never) |
| `apps` | Bundle identifiers. An empty list marks the general (fallback) profile; only the first such profile is kept, all others need at least one app |
| `active` | Path of the active side-button mode, as indices (`[3, 1]` = slice 1 inside slice 3) |
| `Key` | `key` is a macOS virtual key code (physical key; Windows maps it to the key in the same position). `mods` is a bitmask: Shift = 1, Ctrl = 2, Opt/Alt = 4, Cmd/Win = 8 |

Out-of-range values are clamped when the file is loaded. Rings are trimmed to 8 slices and menus to 5 levels.
A file without a `"version"` field (the old v1 format) or with invalid JSON is rejected: CursorPie copies it to
`config.broken.json` next to the original and starts with the defaults. The next save (pressing Save in
Settings, or picking a side-button mode in the pie) then overwrites `config.json`, so fix or move the broken
file first if you want to keep your edits.

**Security:** `Command` actions run shell commands with your user's permissions. Only import configs you trust.
CursorPie warns you after importing a file that contains any.

Export writes `~/Downloads/cursorpie-config.json`.

## Project layout

```text
Cargo.toml              dependencies, release profile, docs.rs metadata
scripts/bundle.sh       builds and signs target/bundle/CursorPie.app (macOS)
scripts/package-windows.ps1  builds target\bundle\CursorPie-windows.zip (Windows)
icon/                   sample images for icons (brackets, clipboard); not loaded automatically
CONTRIBUTING.md         how to build, test and contribute
docs/                   WINDOWS_MIGRATION.md (Windows port), UI_DESIGN.md (look and rendering rules)
src/
  main.rs               thin binary: calls cursorpie::run()
  lib.rs                crate root: module list, run()
  about.rs              name, version, bundle id, author and project links (edit to rebrand)
  actions.rs            action worker thread; sends shortcuts, opens apps/URLs, runs commands
  cache.rs              bounded least-recently-used cache (backs the icon textures)
  history.rs            undo / redo snapshots with burst coalescing (used by Settings)
  side.rs               which pie the side buttons follow (per-app choice, indicator, switching)
  keys.rs               shortcut model (key code + modifiers) and key labels
  msg.rs                messages sent to the UI thread
  geometry.rs           pie sizes and slice hit-testing (shared by the tap and the UI)
  config/               config model, defaults, validation, load/save, import/export, backups,
                        profile sharing (`share.rs`)
  tap/                  global event tap and the gesture state machine
  platform/             OS glue: macos/, windows/ and a test stub behind one facade; keymap.rs (key tables)
  ui/                   everything on the UI thread
    app/                eframe app: state, message handling, window management, idle sleep
    pie/                drawing the pie
    settings/           the Settings window (tabs, editors, icon picker)
    icons.rs            icon texture cache (bounded, kept across Settings sessions, warmed on press)
    sleep.rs            idle policy: when caches and the Settings copy are freed
    tray.rs, appicon.rs, draw.rs, screen.rs
  guides/               long-form developer docs (rendered by rustdoc)
```

## Developer documentation

The technical documentation is written as Rust docs (rustdoc), so it is rendered like any other
crate on docs.rs and stays next to the code:

```sh
cargo docs      # alias for: cargo doc --no-deps --document-private-items --open
```

Start with the **architecture** guide (threads, message flow, the gesture state machine, locking
rules). The other guides cover the config file format and step-by-step recipes for common
changes. See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, checks and the pull-request checklist.

## Privacy

CursorPie works entirely on your computer. The app makes no network connections and collects no data.

The event tap receives key events so that Settings can capture a shortcut while you record one. Outside of
recording they are passed on untouched and are never stored or logged.

Actions you configure yourself (opening URLs, running commands) do whatever you tell them to.

## Known limitations

- macOS and Windows only. Other platforms build a do-nothing stub that exists for unit tests.
- Windows behaves almost identically to macOS. The one gap is extra mouse buttons: only the middle button can be
  the trigger (Windows reports only the middle, back and forward buttons, not buttons 6 to 8).
- Windows: **app detection cannot tell UWP apps apart.** UWP (Microsoft Store / "packaged") apps, such as
  Calculator, Photos or Settings, are shown by Windows inside one shared host process. CursorPie identifies
  an app by its executable name, so every one of them reports `applicationframehost.exe`. A profile with
  that app id applies to all UWP apps at once, and a profile cannot be made for just one of them. Normal
  desktop programs (Chrome, VS Code, Explorer, Office...) are detected correctly. If you need a different
  pie for one UWP app, use the desktop version of the program when there is one.
- Windows: programs running **as administrator** ignore the input CursorPie sends unless CursorPie also runs
  as administrator (a Windows rule, UIPI). With monitors at different scales the pie may be placed slightly
  off when it moves between them, and Windows may refuse to bring a window to the front.
- Shortcut labels and the shifted-symbol display assume a US/ANSI layout. Key codes themselves are physical keys.
- The back/forward side buttons are fixed to mouse buttons 4 and 5 (macOS button numbers 3 and 4; the same two
  buttons on Windows). Only the trigger button is configurable.
- With a per-app profile and the default gesture, a plain middle-click in that app is delivered after the
  double-click window (260 ms by default). Use classic mode, or a shorter window, if that bothers you.
- The right-click that switches the side-button pie is swallowed only while a pie is open; otherwise right-clicks pass
  through untouched.
- "App under the pointer" looks at normal app windows (not the Dock, menu bar or desktop). Bringing it to the front
  activates the whole app, not one particular window.
- Sequences cannot be edited in Settings yet, only in `config.json`.
- Undo covers one Settings session. After you save and close, use **Settings > General > Backups** (the last
  five saves). Changes that Settings does not make, such as the active side-button mode that CursorPie saves
  when you pick a mode in the pie, are not backed up on their own.
- Launch at login starts CursorPie at your next login: a LaunchAgent on macOS, a `Run` registry value on Windows.
  It points at the executable that was running when you enabled it, so toggle it off and on again if you move the app.
- If the app icon does not show up in the center of an app's own pie, start CursorPie from a terminal
  (`cargo run --release`): a line starting with `CursorPie: app icon for <bundle id>` says why it could not be
  read. The icon only appears in the pie of an app that has its own profile (double-click in the default mode).
- macOS: ad-hoc signed builds may need the Accessibility permission re-granted after each rebuild. Sign with a stable
  identity via `SIGN_ID` to avoid that.

## Forking and rebranding

Edit `src/about.rs` (name, author, links, bundle identifier), `Cargo.toml` and `LICENSE`.
Keep `BUNDLE_ID` in `src/about.rs` and in `scripts/bundle.sh` identical: the first is used for the launch-at-login
agent, the second for the `.app` bundle.

## License

MIT. See [LICENSE](LICENSE).

Built with egui / eframe, core-graphics, tray-icon / muda, objc2, serde and image.
Inspired by [Kando](https://github.com/kando-menu/kando).

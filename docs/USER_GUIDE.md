# User guide

Everything about using CursorPie once it is installed. New here? Read [The gesture](#the-gesture) and
[Slices and actions](#slices-and-actions), and skim the rest when you need it.

## Contents

- [The gesture](#the-gesture)
- [Slices and actions](#slices-and-actions)
- [Profiles](#profiles)
- [Side buttons and app pies](#side-buttons-and-app-pies)
- [Settings](#settings)
- [Undo, redo and backups](#undo-redo-and-backups)
- [Sharing a profile](#sharing-a-profile)
- [Default setup](#default-setup)
- [Memory and performance](#memory-and-performance)

## The gesture

1. **Hold** the trigger (the middle mouse button by default) for 500 ms. The pie opens under the cursor.
2. **Move** toward a slice. It highlights as you hover.
3. **Release** to run it. Release in the center dead zone to cancel.

What else the trigger does:

| You do this | CursorPie does this |
|---|---|
| **Hold** for 500 ms | Opens your general pie under the cursor |
| **Double-click** (default mode) | Opens the pie of the app under the pointer, instantly, if that app has its own profile |
| **Right-click** while the pie is open | Switches which pie the side buttons follow, and the ring changes color at once |
| **Click** quickly | Passes the click through to the app. If the app has its own profile, the click is delivered after the double-click window (260 ms) |
| **Press and drag** before the pie opens | Hands the drag back to the app, so middle-drag panning still works |
| **Release** over a slice | Runs that slice |
| **Drag outward** across the rim of a submenu slice | Opens the submenu, re-centered on the cursor |
| **Keep holding** in the center | After 1200 ms in total, opens Settings. A progress arc fills around the center disc |

In *classic* mode (see [Profiles](#profiles)) there is no double-click: holding opens the app's own pie if it has
one, otherwise the general pie. Every timing can be changed in **Settings > Behavior**.

**Instant flick** is an option: drag right after pressing to open the pie with no hold delay. It disables
middle-drag pass-through in other apps. The trigger can be the middle button, or on macOS one of three extra
mouse buttons.

## Slices and actions

A slice is one of three kinds:

- **Run** does something when you release the button over it.
- **Mode** makes your mouse's back and forward buttons send the actions you assigned to this mode, for example
  Undo and Redo, or Cmd+[ and Cmd+] on macOS and Alt+Left and Alt+Right on Windows. The active mode is
  remembered per profile and shown in the center of the pie. If an action is *None*, the native back or
  forward click passes through.
- **Submenu** opens a nested ring. A ring holds up to 8 slices and menus go up to 5 levels deep. A dot on the
  rim marks a submenu.

Actions are available for Run slices and for each side button of a Mode:

| Action | What it does |
|---|---|
| None | Nothing. For side buttons, the click passes through to the app |
| Shortcut | Sends a key combination. Click the button and press the keys to record it (Esc cancels) |
| Open app | macOS: `open -a <name or path>`, for example `Safari` or `/Applications/Safari.app`. Windows: a program name or the path of an `.exe`, for example `chrome` or `C:\Tools\App.exe`. Linux: a desktop-entry id, a program name or a path. If the app already runs on another virtual desktop or Space, CursorPie goes to it |
| Open URL / file | Opens the target with the system, for example `https://example.com` or a file path |
| Shell command | macOS and Linux: `/bin/sh -c "<command>"`. Windows: `cmd /C "<command>"` without a console window |
| Sequence | Several actions in order, 40 ms apart. Edit these in `config.json` (see [Configuration](CONFIGURATION.md)) |

Actions run on one worker thread, so they always execute in the order you triggered them.

**Icons** can be text, emoji, or an image (`.png`, `.jpg`, `.jpeg`, `.webp`); a path may start with `~/`. The
pie size (140 to 360 px), accent color and sounds are adjustable. An optional soft tick plays when the hovered
slice changes.

## Profiles

A profile is a pie plus the apps it applies to. Up to 16 profiles are allowed.

- Apps are identified by bundle id on macOS (for example `com.apple.Safari`), executable name on Windows (for
  example `chrome.exe`, not case sensitive) and desktop-entry id on Linux (for example `firefox`).
- There is **exactly one general profile**: the one with **no apps**. It serves every app without a profile of
  its own, is created for you and cannot be deleted.
- Every other profile **belongs to at least one app**. **New profile** asks for a name and an app, and the last
  app of a profile cannot be removed.
- Two ways to combine them, switchable in **Behavior**:
  - *Hold = general pie, double-click = pie of the app* (default).
  - *Classic*: hold opens the app's own pie if it has one, otherwise the general pie.

## Side buttons and app pies

An app with its own profile has two pies: its own and the general one. The back and forward buttons follow
exactly one of them. Each pie shows which one with its ring color: **accent = the side buttons follow this pie,
red = they follow the other one** (the colors can be changed).

- **Default:** the app's pie. Pick the general pie instead in **Settings > Behavior > Side buttons**.
- **Choosing:** pick a Mode slice in a pie and the side buttons follow that pie from then on, in that app. Hold
  the trigger, pick *History* in the general pie, and Undo and Redo are on your side buttons even in an app with
  its own pie.
- **Switching:** with a pie open, **right-click**. The pie stays open and its ring changes color. Without
  opening a pie, use *Switch side-button pie* in the menu bar; the pie that now owns the side buttons appears
  for a moment (time adjustable, 0 turns it off).
- The choice is remembered per app while CursorPie runs. It can be forgotten when you switch to another app, and
  saving Settings clears it.

**Which app is "the app"?** By default the app whose window is **under the pointer**, so you do not have to
click a window to focus it first. Shortcuts always go to the app in front, so CursorPie brings that app to the
front right before it sends one, unless it already is. Both are options in Behavior: switch to *the app in
front* for the old behavior. If CursorPie cannot tell which window is under the pointer, it uses the app in
front.

## Settings

Open it from the menu bar or tray icon, or by holding the trigger in the center of the pie. **Save** applies
and stores your changes. **Cancel**, **Esc** and **Cmd+W** (Ctrl+W elsewhere) discard them. **Undo** and
**Redo** step through your edits.

**Menu** is where you build pies.

- Choose, create, rename and delete profiles, list the apps each one applies to (type an id or use **Pick
  app...**), and **Share** a profile.
- An interactive pie shows the ring you are editing. Click a slice to edit it, **+** to add one, a selected
  submenu slice again to step into it, and the center to go back.
- Per slice: name, icon, kind, action, and Move or Delete.
- Ready-made slices on the **+** button: *side-button modes* (History, Brackets, Clipboard, Browser, Tabs, Zoom,
  Scroll, Edit), *quick actions* (Screenshot, Mission Ctrl, Spotlight, Emoji, Lock) and *custom* ones (Shortcut,
  Open app, Open URL, Command, Submenu).

**Behavior** holds the knobs.

- Trigger button, hold times for the pie and for Settings, pie size, accent color.
- Whether holding opens the general pie or the app's own, and the double-click speed.
- App detection: under the pointer or in front, and whether to bring it forward before sending shortcuts.
- *Other desktops / Other Spaces*: what an Open app slice does when the app's window is elsewhere. Switch to it
  (default), bring the window to the current Space (macOS, experimental) or just open the app here. See
  [OTHER_DESKTOPS.md](OTHER_DESKTOPS.md).
- Side-button options: default pie, pick-to-follow, forget on app switch, right-click switching, ring colors,
  preview time.
- Instant flick, hover sound, app icon in the pie center, launch at login, and **Memory** (free caches after an
  idle time, on by default, 60 s).

**General** shows the permission status (macOS) with shortcuts to System Settings, the config file location,
**Reveal in Finder** (Explorer, or the file manager on Linux), **Export...**, **Import...**, **Reset to
defaults** and **Backups**. On Linux it also has Pause and Quit.

**About** shows the version, author, project links, privacy note and credits.

## Undo, redo and backups

Deleting a slice or a profile by accident is easy to fix.

**While Settings is open**, every change is recorded: adding, deleting or moving slices and profiles, editing
actions and apps, changing options, importing a file and **Reset to defaults**.

- **Undo** and **Redo** are in the top bar. Use **Cmd+Z** and **Cmd+Shift+Z** (**Cmd+Y** also redoes; Ctrl
  instead of Cmd on Windows and Linux).
- Typing and dragging a slider count as **one** step: a step ends when you pause for a moment.
- Up to 50 steps are kept. A new change after an undo discards the steps you undid.
- In a text field, **Cmd+Z** undoes the text first. While you record a shortcut, the keys go to the recorder.
- The history lives only as long as the Settings window. **Save**, **Cancel** and **Esc** close it and forget it.

**After you have saved**, use backups. Every **Save** first stores the version it replaces as
`config.backup.1.json` (the newest) next to `config.json` and keeps the last five. Open **Settings > General >
Backups** and pick one: it is loaded into the editor, and nothing changes until you press **Save**. Undo still
brings back what you had before restoring.

A Save that would store exactly what is already in `config.json` makes no backup. The file is written through a
temporary one, so a crash or a full disk cannot leave half a file. Where the files live is in
[CONFIGURATION.md](CONFIGURATION.md#where-the-file-lives).

## Sharing a profile

**Menu > Share** moves one profile between computers, with its icons, so it looks the same on arrival.

- **Export this profile...** writes `<name>.cursorpie-profile.json` to your Downloads folder and reveals it.
  Every image icon the profile uses, including those in submenus, is embedded, up to 2 MB each. Icons of apps,
  text and emoji need nothing embedded. Missing or oversized images show up as `?` for the receiver.
- **Import a profile...** adds the file as a **new** profile (nothing is overwritten) and selects it. The
  embedded images are copied into the `icons` folder next to `config.json`: an identical file is reused, a
  different one with the same name gets a number. Press **Save** to apply it, or **Undo** to take it back.
- A profile must list at least one app. A name already taken gets ` (2)`, ` (3)`...
- Only PNG, JPEG and WebP data is written, whatever the file claims, and nothing outside the icons folder.

**Only import profiles you trust.** Actions can run shell commands. CursorPie warns you when an imported
profile contains any, and nothing runs until you press Save and then use the slice.

## Default setup

A fresh install comes with three profiles:

| Profile | Apps | Slices |
|---|---|---|
| Default (general) | all apps without a profile | History (undo/redo), Clipboard (copy/paste), Brackets, and a *Launch* submenu with a file manager, terminal, browser and screenshot tool |
| Browsers | Safari, Chrome, Firefox, Arc, Brave, Edge | Browser (back/forward), Tabs (previous/next), Zoom, Scroll (Page Up/Down) |
| Code | VS Code, Xcode | History, Brackets, Edit (Return/Delete) |

The Launch submenu and the app ids adapt to your system (for example Finder on macOS, Explorer on Windows).

## Memory and performance

CursorPie sits in the menu bar all day, so it is built to cost almost nothing while you are not using it.

- **No drawing while idle.** The window is hidden and nothing repaints until you press the trigger.
- **Icons are cached** (at most 96) and shared by the pie and Settings, and loaded during the hold time so the
  pie appears without a hitch.
- **Sleep.** After 60 seconds without a pie or Settings (**Settings > Behavior > Memory**), CursorPie drops the
  cached icons and the Settings working copy and asks the system to take the freed pages back. The next press
  loads what the pie needs again. Switch the option off to keep everything in memory.

What sleep cannot free is the text and graphics buffers the UI library keeps for the process lifetime: a fixed
base cost that does not grow with use.

If edges look soft on your screen, open an issue and mention your display scaling. How the interface is drawn is
explained in [UI_DESIGN.md](UI_DESIGN.md).

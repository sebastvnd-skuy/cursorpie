<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="icon/cursorpie_logo.png">
  <img src="icon/cursorpie_logo_black.png" alt="CursorPie" width="420">
</picture>

**Hold a mouse button. Flick. Release.**<br>
A pie menu for your cursor on macOS, Windows and Linux.

[![CI](https://github.com/sebastvnd/cursorpie/actions/workflows/ci.yml/badge.svg)](https://github.com/sebastvnd/cursorpie/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/sebastvnd/cursorpie?include_prereleases)](https://github.com/sebastvnd/cursorpie/releases)
[![License: MIT](https://img.shields.io/github/license/sebastvnd/cursorpie)](LICENSE)
![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-blue)

[Install](docs/INSTALL.md) · [User guide](docs/USER_GUIDE.md) · [Configuration](docs/CONFIGURATION.md) · [Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md)

</div>

---

## What is CursorPie?

CursorPie is a small pie menu that waits in your menu bar or system tray. Hold the middle mouse button anywhere,
flick toward a slice, release: the slice runs. A slice can send a keyboard shortcut, open an app, a link or a
file, run a command, open a submenu, or turn your mouse's back and forward buttons into something more useful.
Every app can have its own pie.

It is written in Rust with [egui](https://github.com/emilk/egui) and inspired by
[Kando](https://github.com/kando-menu/kando).

## Who is it for?

- **People with a multi-button mouse** who want their favorite actions under the cursor instead of in
  their memory.
- **Developers, designers and editors** who live in a few apps and want a different pie for each one:
  undo and redo in the editor, tabs and zoom in the browser.
- **Anyone who prefers a light, private tool.** No account, no network access, no telemetry.

You need a mouse with a middle button (a clickable wheel is enough).

## Highlights

- **Fast gesture.** Hold (500 ms by default) or flick, then release over a slice. Hold in the center to open Settings.
- **Per-app profiles.** A general pie plus one pie for each app you choose.
- **Side-button modes.** Put Undo/Redo, tab switching or zoom on the back and forward buttons, and switch between them from the pie.
- **Flexible actions.** Send a shortcut, open an app, a URL or a file, run a shell command, or chain several in a sequence.
- **Submenus.** Up to 8 slices per ring and 5 levels deep.
- **Safe to experiment.** Undo and redo inside Settings, plus a backup of every earlier save.
- **Easy to share.** Export one profile, icons included, as a single JSON file.
- **Light on resources.** Nothing is drawn while idle, and cached icons are freed after a pause.

## Quick start

Download a build from the [Releases page](https://github.com/sebastvnd/cursorpie/releases) when one is published
for your system, or build it yourself with [Rust](https://rustup.rs) 1.88 or later:

```sh
git clone https://github.com/sebastvnd/cursorpie.git
cd cursorpie
cargo run --release
```

Then:

1. Hold the **middle mouse button** for half a second. The pie opens under the cursor.
2. Move toward a slice.
3. Release to run it.

Step-by-step installs, app bundles and installers, and the permission macOS asks for are in
[docs/INSTALL.md](docs/INSTALL.md).

## Platform support

| Platform | Status | Good to know |
|---|---|---|
| macOS 11 or later | Supported | Needs the Accessibility permission. Any of four mouse buttons can be the trigger |
| Windows 10 (1809) / 11 | Supported, stable | No permission needed. Only the middle button can be the trigger |
| Linux, X11 session | Supported, experimental | No permission needed. Its behavior on different distributions and desktops is still being observed. Wayland only works over X11 and XWayland windows, and there is no tray icon yet |

The full list of limits is in [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md).

## Documentation

| I want to... | Read |
|---|---|
| Install or build the app | [docs/INSTALL.md](docs/INSTALL.md) |
| Learn every gesture, action and setting | [docs/USER_GUIDE.md](docs/USER_GUIDE.md) |
| Edit or share `config.json` | [docs/CONFIGURATION.md](docs/CONFIGURATION.md) |
| Know what does not work yet | [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md) |
| Fix a bug or add a feature | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Understand how the code works | Rust docs: run `cargo docs` |
| See what changed | [CHANGELOG.md](CHANGELOG.md) |

Design notes for each platform and for the interface are listed in [docs/README.md](docs/README.md).

## Contributing

Contributions are welcome. The short version:

```sh
cargo tools hooks install   # once per clone
cargo tools ci              # every check CI runs
```

[CONTRIBUTING.md](CONTRIBUTING.md) explains the workflow, the project rules and how to open a pull request.
Found a bug? [Open an issue](https://github.com/sebastvnd/cursorpie/issues).

## Privacy

CursorPie works entirely on your computer. It makes no network connections and collects no data. The event tap
sees key events only so that Settings can capture a shortcut while you record one; otherwise they are passed on
untouched and never stored or logged. Actions you configure yourself, such as opening a URL or running a
command, do whatever you tell them to.

## License

[MIT](LICENSE). Built with egui / eframe, tray-icon, serde and image. Inspired by [Kando](https://github.com/kando-menu/kando).

# Changelog

## Unreleased

## 0.4.2

- **Welcome window.** The first start on macOS and on Windows opens a three-page welcome window in the
  Settings style: how the gesture works, the step that is needed next (macOS: allow Accessibility, with a live
  status; Windows: where the tray icon is) and a "Launch at login" box. It is shown once (the empty file
  `welcome_shown` next to `config.json` remembers it), never to someone who already has a `config.json`, and
  **Welcome guide...** in the menu bar / tray menu opens it again.
- **Windows: Setup wizard.** `scripts\package-windows.ps1` also builds `CursorPie-<version>-setup.exe` when
  Inno Setup 6 is installed (`installer/cursorpie.iss`): a per-user installer (no administrator rights) with
  license page, Start Menu shortcut, "start at sign-in" option, update-in-place and an uninstaller that keeps
  your settings. A copy installed by the wizard no longer installs itself again or adds a second "Apps &
  features" entry; an older self-install entry is removed. `SIGN_CERT_THUMBPRINT` signs the files.
- **macOS: disk image.** `scripts/make-dmg.sh` builds `CursorPie-<version>-<arch>.dmg` with an Applications
  shortcut, optionally signed and notarized (`SIGN_ID`, `NOTARY_PROFILE`), plus a `.sha256` file.
  `UNIVERSAL=1` (also understood by `scripts/bundle.sh`) builds one app for Apple silicon and Intel. A
  Developer ID signature now uses the hardened runtime, which notarization requires.
- Tests that assumed macOS defaults are pinned to them, so the test suite passes on Windows.

## 0.4.1

- Fix dead code warnings on the Windows target

## 0.4.0

- **Developer tools.** `cargo tools` runs the repository's own tools (the `tools/` package, standard
  library only, not part of the app). `cargo tools bump patch|minor|major` raises the version in
  `Cargo.toml` and `Cargo.lock`, turns the notes under `## Unreleased` (or, without notes, the commit
  subjects since the last release) into the changelog entry, leaves a fresh `## Unreleased` section and
  can commit and tag the release from `main`. It lists the steps that stay manual and warns when
  something is probably stale. `cargo tools check` verifies that the files agree (and, with `--tag`,
  that a tag matches them), `cargo tools ci` runs every check CI runs, and `cargo tools hooks install`
  turns on Git hooks that check formatting and commit messages. This replaces the earlier
  `cargo run --example bump`. The workflow and its rules are in `CONTRIBUTING.md`, "Development
  workflow" and "Releasing".
- **Windows: CursorPie installs itself.** The first start copies the program to
  `%LOCALAPPDATA%\Programs\CursorPie`, turns on "Launch at login" for that copy, adds an "Apps & features"
  entry (`cursorpie.exe --uninstall` removes it, settings are kept) and continues from the installed copy.
  Before, the startup entry pointed at wherever the `.exe` was started from and had to be switched on by hand,
  so the app was gone after a restart as soon as the download folder was moved or cleaned. A newer download
  updates the installed copy. `cargo` builds and `CURSORPIE_NO_INSTALL=1` skip the install.
- **Windows: one running copy.** A second start exits at once instead of installing a second mouse hook.
- **Windows: "Launch at login" honors Task Manager.** An entry switched off in Task Manager reads as off in
  Settings, and ticking the box switches it on again.
- **Settings has a sidebar.** The sections (Menu, Behavior, General, About) are listed on the left and the
  selected page is on the right, like the settings of a large application. Behavior is one column of panels.
  The window is 1120 points wide (it shrinks on small displays) so the Menu editor keeps its room.

## 0.3.0

- **Windows 10/11 support.** The same code base now builds and runs on Windows and behaves almost
  identically to macOS: global mouse and keyboard hooks, app detection, per-app profiles, shortcuts,
  launchers, system tray, launch at login, Settings. Config files and shared profiles are portable
  between the two systems. See `docs/WINDOWS_MIGRATION.md`.
- **Truly transparent pie on Windows.** The Windows build uses the `glow` renderer, and the window
  frame that winit leaves on borderless windows is removed (`platform::clear_window_frame`): no white
  box or border around the pie, no old-style window flashing up when it opens, no thin white line
  along the top edge.
- Not at parity on Windows: extra mouse buttons. Only the middle button can be the trigger (Windows
  reports only the middle, back and forward buttons).
- Known limits on Windows: per-app profiles cannot tell UWP / Microsoft Store apps apart (they all
  report `applicationframehost.exe`), and programs running as administrator ignore CursorPie's input unless
  CursorPie runs as administrator too.
- macOS behavior is unchanged.

## 0.2.0

- macOS only. Config format version 2.

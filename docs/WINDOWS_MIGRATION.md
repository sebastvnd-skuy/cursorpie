# Windows support: migration plan

CursorPie 0.2.0 builds for macOS only (`compile_error!` in `src/lib.rs`). This document explains
what is macOS-specific today, the target design that supports macOS **and** Windows 10/11, and the
order in which the code is migrated so macOS behavior never changes along the way.

Status of each phase is tracked at the end ([Progress](#progress)).

## Goals and non-goals

Goals

- One code base, one `config.json` format, two supported platforms: macOS 11+ and Windows 10 (1809+) / 11.
- **No behavior change on macOS.** Existing `config.json` files, shared profile files and the
  `.app` bundle keep working. macOS code is moved, not rewritten.
- Everything that is a *decision* (gesture state machine, shortcut model, defaults, config paths,
  DPI math, key tables) is plain Rust that can be unit tested on any OS, including Linux CI.
- Windows needs no special permission and no installer: a single `cursorpie.exe` in the tray.

Non-goals (for the first Windows release)

- Linux support. A *stub* backend exists only so `cargo test` runs on Linux; it does nothing at runtime.
- Extra mouse buttons 5 to 7 as trigger (Windows only reports middle, back and forward).
- Perfect behavior on mixed-DPI multi-monitor setups (see [Risks](#risks-and-known-limitations)).

## What is macOS-specific today

| Area | Files | macOS API | Windows counterpart |
|---|---|---|---|
| Global mouse/keyboard hook | `tap/mod.rs`, `tap/*.rs` | `CGEventTap` + `CFRunLoop` | `SetWindowsHookExW(WH_MOUSE_LL / WH_KEYBOARD_LL)` + message loop |
| Event types in the state machine | `tap/trigger.rs`, `side_buttons.rs`, `right_click.rs`, `keyboard.rs`, `gesture.rs`, `resolve.rs`, `geometry.rs` | `CGEvent`, `CGEventType`, `CGPoint` | portable `Point`, `Kind`, `Verdict` |
| Sending keys / replaying clicks | `actions.rs` | `CGEvent::post` | `SendInput` |
| Frontmost app, app under pointer | `platform/apps.rs` | `NSWorkspace`, `CGWindowList` | `GetForegroundWindow`, `WindowFromPoint`, `QueryFullProcessImageNameW` |
| App icons | `platform/apps.rs`, `ui/appicon.rs` | `NSWorkspace` icon as TIFF | `SHGetFileInfoW` + `GetDIBits` (RGBA) |
| Permission | `platform/permissions.rs` | `AXIsProcessTrusted`, System Settings URLs | none needed |
| Keyboard layout | `platform/layout.rs` | TIS + `UCKeyTranslate` | `GetKeyboardLayout`, `MapVirtualKeyExW`, `ToUnicodeEx` |
| Launch at login | `platform/login_item.rs` | LaunchAgent plist | `HKCU\...\Run` registry value |
| Return memory to the OS | `platform/memory.rs` | `malloc_zone_pressure_relief` | `SetProcessWorkingSetSize(-1, -1)` |
| File chooser, reveal, open, tick | `platform/shell.rs`, `ui/settings/icon_catalog.rs` | `osascript`, `open`, `mdls`, `afplay` | `GetOpenFileNameW`, `explorer`, `ShellExecuteW`, `MessageBeep` |
| Run actions | `actions.rs` | `open -a`, `open`, `/bin/sh -c` | `ShellExecuteW`, `cmd /C` |
| Menu bar icon | `ui/tray.rs` | `tray-icon` (template image) | `tray-icon` (coloured image) |
| Dock icon | `lib.rs`, `platform/apps.rs` | `NSApplication` activation policy | none (use `with_taskbar(false)`) |
| Config location | `config/store.rs` | `~/Library/Application Support/CursorPie` | `%APPDATA%\CursorPie` |
| Display bounds | `ui/screen.rs` | `CGDisplay` | `EnumDisplayMonitors` |
| Defaults | `config/defaults.rs` | Cmd shortcuts, bundle ids, Finder/Safari | Ctrl/Win shortcuts, exe names, Explorer/Edge |
| Build gate | `lib.rs`, `Cargo.toml` | `compile_error!`, unconditional macOS crates | `cfg` gating, target-specific dependencies |
| UI text | `ui/settings/**`, `about.rs` | "Cmd", "Finder", "Accessibility", "Mac" | per-platform strings |
| Packaging | `scripts/bundle.sh` | `.app` + `codesign` | `scripts/package-windows.ps1` |

## Design decisions

1. **Key codes stay macOS virtual key codes in `config.json`.** `Shortcut { key, mods }` is a
   physical key position (the same idea as a PS/2 scan code), so the file format does not change and
   a profile exported on a Mac imports on Windows. The Windows backend translates at the edge:
   layout keys (letters, digits, punctuation) by scan code, special keys (Return, arrows, F-keys,
   numpad...) by virtual-key code. The tables live in plain Rust (`platform/keymap.rs`) and are tested.
2. **A "primary modifier" per OS.** `MOD_CMD` keeps meaning the Command key on macOS and the Windows
   key on Windows. Built-in presets use `MOD_PRIMARY` (Cmd on macOS, Ctrl on Windows), so "Undo" is
   Cmd+Z on a Mac and Ctrl+Z on Windows. Labels use the platform's names (Cmd/Opt vs Win/Alt).
3. **"Bundle id" becomes "app id".** On macOS it is still the bundle identifier. On Windows it is the
   lower-case executable name (`chrome.exe`). Profiles store it in the same `apps` field. Matching is
   case-insensitive on Windows only.
4. **Coordinates are logical points.** `geometry::Point` is in the same unit egui uses. macOS already
   reports points; the Windows backend divides physical pixels by the monitor scale (`dpi / 96`).
5. **The event layer is portable.** Native glue (`tap/macos.rs`, `tap/windows.rs`) converts a native
   event into `Ev` and the portable `dispatch` returns a `Verdict` (`Keep` or `Drop`). The state
   machine and all its locking rules (see `guides::architecture`) are untouched. On Windows, mouse
   *moves* are never swallowed: the hook always passes them on, because dropping them would freeze
   the cursor.
6. **Backends are modules with identical function names.** `platform/mod.rs` picks `macos`, `windows`
   or `stub` with `cfg` and re-exports it, so call sites do not contain `cfg` attributes (except
   the window builder in `lib.rs`).
7. **Windows code is type-checkable on any OS.** The Windows backend avoids `std::os::windows`,
   compiles under `--features check-windows` on Linux/macOS, and its pure helpers have unit tests that
   run everywhere.
8. **No new dependencies besides `windows-sys`** (Windows only). The tray, config and image crates
   are unchanged; `tray-icon` is restricted to macOS and Windows.

## Phases

Each phase is one commit and leaves macOS working. Tests come first.

| # | Phase | Exit criteria |
|---|---|---|
| 0 | This document | reviewed |
| 1 | **Characterization tests** (before touching any code) pinning today's macOS behavior: shortcut labels, key names, presets, default config, JSON shape of actions, config parsing, geometry, path helpers | tests added, pass on macOS |
| 2 | **Portable core:** `geometry::Point`; the tap split into portable `dispatch` + `tap/macos.rs` glue; `platform/` split into `macos/` and `stub/`; Cargo target-specific dependencies; `compile_error!` removed | all old and new tests pass; `cargo test` also passes on Linux through the stub |
| 3 | **Windows backend:** hook, `SendInput`, app lookup, DPI, layout, registry, shell, file chooser, icons, tray | `cargo check --features check-windows` is clean; pure helpers tested |
| 4 | **Per-OS defaults and text**, config paths, case-insensitive app ids, README / CONTRIBUTING / rustdoc guides, CI for macOS + Windows + Linux, Windows packaging script | tests cover both `Os::Mac` and `Os::Windows` defaults |
| 5 | **Manual verification on real machines** using the checklist below | checklist signed off |

## Test strategy

- **Characterization first (phase 1).** Constants are asserted literally (for example Cmd+Z is
  `Shortcut { key: 6, mods: 8 }`) so a refactor that changes macOS output fails loudly. Tests that
  need the keyboard layout use the built-in US/ANSI fallback, which is what test threads see on
  every OS (the layout snapshot is only built on the main thread).
- **Parametrize on `Os`.** Defaults, labels and paths take the target OS as an argument
  (`presets_for(Os::Windows)`), so one test run covers both platforms.
- **Plain data tables are tested for consistency:** no duplicate scan codes, every mapped key
  round-trips (`mac code -> native -> mac code`), special names exist for every special key.
- **What cannot be unit tested** (hooks, `SendInput`, tray, window behavior) is covered by the manual
  checklist and by CI compiling the real target.

### Manual verification checklist (phase 5)

Run on macOS 11+ and on Windows 10 and 11:

1. Hold the middle button 500 ms: pie opens under the cursor; release over a slice runs it.
2. Quick middle click passes through; middle-drag still pans in a browser or design tool.
3. Double-click opens the app's own pie in an app that has a profile.
4. Back/forward buttons send the active mode's shortcuts; right-click while the pie is open switches pies.
5. Settings: record a shortcut (including Esc to cancel), undo/redo, save, backup restore, import/export.
6. A shortcut slice reaches the app under the pointer (focus is moved first).
7. Tray menu: Settings, About, Pause, Launch at login (reboot or re-login), Quit.
8. Non-US keyboard layout (AZERTY or QWERTZ): labels and preset shortcuts match the layout.
9. Two monitors, same scale, and 125 % / 150 % scaling on Windows: pie size and hit-testing match.
10. Idle for the sleep time, then open the pie: icons reload, no crash.

## Risks and known limitations

- **Low-level hooks have a timeout.** Windows removes a hook whose callback is too slow (about 300 ms
  by default) without telling the app. The callback only does atomic loads and short locks, like the
  macOS tap, and never blocks. A watchdog that reinstalls the hook is a possible follow-up.
- **Elevated windows.** A normal process cannot send input to an administrator window (UIPI);
  shortcuts to such windows are ignored. Run CursorPie as administrator if you need that.
- **UWP apps share one host process** (`ApplicationFrameHost.exe`), so per-app profiles cannot tell
  them apart.
- **Foreground rules.** `SetForegroundWindow` can be refused by Windows; the "focus target" option is
  best effort there.
- **Mixed DPI.** With monitors at different scales the pie window may be placed slightly off when it
  moves between monitors; single-scale setups are exact.
- **Icons in a copied `config.json`.** Image icons are stored as `~/...` paths, which differ per OS. Profile
  *share files* embed the images and are portable; copying a whole `config.json` between OSes is not.
- **Trigger button.** Only the middle button is offered on Windows.
- **Unverified here.** The Windows backend was written and type-checked without a Windows machine.
  Treat phase 5 as required before a release.

## Progress

| Phase | State |
|---|---|
| 0 | done |
| 1 | done: 40+ characterization tests, all green on the refactored code |
| 2 | done: `cargo test` runs on Linux (121 tests); macOS glue moved, **not compiled here** (no macOS toolchain) |
| 3 | done: `cargo check --features check-windows` clean; 144 tests incl. Windows pure helpers pass on Linux; **never run on Windows** |
| 4 | done: per-OS defaults, paths, app ids, docs, CI, packaging script; 155 tests green |
| 5 | needs real hardware |

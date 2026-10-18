# Known issues and limitations

What does not work yet, or works differently, grouped by system. If you hit something that is not listed, please
[open an issue](https://github.com/sebastvnd/cursorpie/issues) with your system version, how you started
CursorPie, the trigger button and the output of `cargo run --release` from a terminal.

## All systems

- Shortcut labels and the shifted-symbol display assume a US/ANSI layout. Key codes themselves are physical keys.
- The back and forward side buttons are fixed to mouse buttons 4 and 5. Only the trigger button is configurable.
- With a per-app profile and the default gesture, a plain middle-click in that app is delivered after the
  double-click window (260 ms by default). Use classic mode, or a shorter window, if that bothers you.
- The right-click that switches the side-button pie is swallowed only while a pie is open. Otherwise right-clicks
  pass through untouched.
- "App under the pointer" looks at normal app windows (not the Dock, menu bar or desktop). Bringing it to the
  front activates the whole app, not one particular window.
- Sequences cannot be edited in Settings yet, only in `config.json`.
- Undo covers one Settings session. After you save and close, use **Settings > General > Backups** (the last
  five saves). Changes Settings does not make, such as the active side-button mode saved when you pick a mode in
  the pie, are not backed up on their own.
- If the app icon does not show in the center of an app's own pie, start CursorPie from a terminal: a line
  starting with `CursorPie: app icon for` says why it could not be read. The icon only appears in the pie of an
  app that has its own profile.

## macOS

- Ad-hoc signed builds may need the Accessibility permission again after each rebuild. Sign with a stable
  identity (`SIGN_ID`) to avoid that.
- **Launch at login** points at the executable that was running when you enabled it. Toggle it off and on again
  if you move the app.
- **Other Spaces** tries three ways, each only when the previous one did not bring a window of the app onto the
  screen: `open -a` (works when *When switching to an application, switch to a Space with open windows for the
  application* is on), then Control+Right or Control+Left pressed directly, then the same through System Events
  (which asks for the Automation permission once). The arrow keys need the Mission Control shortcuts *Move left /
  right a space* to be on. If nothing works you stay where you were and `CursorPie: could not reach the Space of
  <app>` is printed. Start with `CURSORPIE_DEBUG=1 cargo run --release` to see what each way found. How it works:
  [OTHER_DESKTOPS.md](OTHER_DESKTOPS.md).
- **Bring the window to this Space** is experimental. macOS has no public call that moves another app's window,
  so CursorPie uses private SkyLight calls (the ones Hammerspoon and yabai use), checks that the window really
  arrived and switches to the app's Space when it did not. Apple may change or block those calls in any release;
  Hammerspoon reports they silently do nothing on macOS 27, so expect the fallback on some systems.

## Windows

- Only the middle button can be the trigger. Windows reports only the middle, back and forward buttons.
- Programs running **as administrator** ignore the input CursorPie sends unless CursorPie also runs as
  administrator (a Windows rule, UIPI).
- **UWP apps cannot be told apart.** Store apps such as Calculator, Photos or Settings run inside one shared host
  process, so every one of them reports `applicationframehost.exe`. A profile with that id applies to all of them
  at once. Normal desktop programs (Chrome, VS Code, Explorer, Office...) are detected correctly.
- **Other desktops** presses Win+Ctrl+Right or Win+Ctrl+Left, one desktop at a time, until a window of the app
  is on the desktop that is shown (at most ten each way), then brings it to the front. Windows has no public call
  that switches to a given desktop, so the desktops flick past on the way. If the app cannot be found, you stay
  where you are and the app is opened as before. There is no "bring the window here" choice on Windows.
- With monitors at different scales the pie may be placed slightly off when it moves between them, and Windows
  may refuse to bring a window to the front.
- **Launch at login** always points at the installed copy, so moving or deleting the download does not matter.
  If you switched the entry off in Task Manager, ticking **Launch at login** switches it back on.

## Linux

Linux support is new and experimental. See [LINUX_MIGRATION.md](LINUX_MIGRATION.md) for what is built.

- **X11 sessions only.** In a Wayland session CursorPie sees only X11 and XWayland windows, so the gesture does
  nothing over Wayland-native windows. Log in with an X11 session for full support. Wayland support is designed
  but not built.
- No tray icon yet. Pause and Quit are in **Settings > General**, and Settings opens by holding the trigger in
  the middle of the pie.
- No app icons in the pie, and no keyboard-layout labels on non-US layouts.
- Only the middle button can be the trigger.
- Handing a **drag** back to the app while the middle button is held (for example middle-drag scrolling) may lose
  the start of the drag. A quick click is passed on reliably.
- Whether your window manager follows the request to switch workspace for **Open app** has not been checked on
  real desktops.
- Install `zenity` or `kdialog` to use the file chooser buttons.
- Other Unixes build a do-nothing stub that exists only so `cargo test` runs.

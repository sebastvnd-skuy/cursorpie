# CursorPie

A hold-to-open pie menu for macOS, inspired by [Kando](https://github.com/kando-menu/kando).
Hold the middle mouse button, drag to a slice, release. Per-app profiles, shortcuts, launchers, submenus.

## Features
- **Pie menu on middle-click hold** (or an extra mouse button). A quick click still reaches the app.
- **Per-app profiles**: each profile lists app bundle IDs; the one for the frontmost app opens. A profile with no apps is the fallback.
- **Three slice types**
  - *Side-button mode*: makes your mouse back/forward buttons send custom shortcuts.
  - *Action*: runs on release. Shortcut, open app, open URL/file, shell command, or a sequence.
  - *Submenu*: drag outward across the rim; the next ring opens re-centered on the cursor (up to 3 levels, 8 slices each).
- **Instant flick** (optional): drag right after pressing to open the pie with no hold delay.
- Menu bar icon: Settings, About, Pause, Launch at login, Quit. No Dock icon.
- Image icons (png/jpg/webp), accent color, pie size, timings, optional hover sound.
- Export / import your setup as JSON. Configs from the first version migrate automatically.

## Build
Requires Rust 1.88+ and macOS 11+.

    cargo run --release            # quick try (grant permissions to your terminal app)
    scripts/bundle.sh              # builds target/bundle/CursorPie.app

First launch: allow **Accessibility** (and **Input Monitoring** if asked) in
System Settings -> Privacy & Security. CursorPie retries every 2 seconds, so it starts working
as soon as access is granted (restart it if it does not).

## Configuration
`~/Library/Application Support/CursorPie/config.json` (edit in Settings, or by hand).
Shape: `profiles[] -> items[] -> kind: Mode | Run | Submenu`. Actions: `Nothing`, `Key`, `OpenApp`,
`OpenUrl`, `Command`, `Sequence`.

**Security:** `Command` actions run shell commands. Only import configs you trust.

## Privacy
Everything stays on your Mac. No network access, no telemetry. Keyboard events are inspected only
while you record a shortcut and are never stored.

## Make it yours
Edit `src/about.rs` (name, GitHub, links, bundle id) and `Cargo.toml` / `LICENSE`.

## Known limitations
- Key names and the shifted-symbol display assume a US/ANSI layout (key codes are physical).
- Launch at login is a LaunchAgent that starts at your next login.
- Ad-hoc signed builds may need the Accessibility permission re-granted after each rebuild;
  sign with a stable identity via `SIGN_ID` to avoid that.

## License
MIT. See `LICENSE`.

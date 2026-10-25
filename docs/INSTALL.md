# Install CursorPie

CursorPie runs on macOS, Windows and Linux (X11). Pick your system below. If a ready-made package is
published on the [Releases page](https://github.com/sebastvnd/cursorpie/releases), use it; otherwise build
from source, which takes a few minutes.

## Requirements

| | macOS | Windows | Linux |
|---|---|---|---|
| System | macOS 11 or later | Windows 10 (1809) or 11 | A distribution with an **X11 session** |
| To build | Rust 1.88+, Xcode Command Line Tools | Rust 1.88+, MSVC build tools | Rust 1.88+, a C linker, libx11, libxi, libxtst and libxrandr development files |
| Permission | Accessibility | none | none |

Install Rust with [rustup](https://rustup.rs). Platform build tools are listed in
[CONTRIBUTING.md](../CONTRIBUTING.md#prerequisites).

## Try it without installing

```sh
git clone https://github.com/sebastvnd/cursorpie.git
cd cursorpie
cargo run --release
```

On macOS, grant the permission to your terminal app (see [Permissions](#permissions-on-macos)).

## macOS

```sh
scripts/bundle.sh            # builds target/bundle/CursorPie.app
```

Move `CursorPie.app` to `/Applications` and open it. The app lives in the menu bar and has no Dock icon.

By default the app gets an ad-hoc signature. To sign with your own identity, which also keeps the
Accessibility permission across rebuilds:

```sh
SIGN_ID="Developer ID Application: Your Name (TEAMID)" scripts/bundle.sh
```

**Disk image.** To hand the app to someone else, build a `.dmg` with an Applications shortcut:

```sh
scripts/make-dmg.sh                  # target/bundle/CursorPie-<version>-<arch>.dmg
UNIVERSAL=1 scripts/make-dmg.sh      # one image for Apple silicon and Intel
SIGN_ID="Developer ID Application: Your Name (TEAMID)" NOTARY_PROFILE=my-profile scripts/make-dmg.sh
```

With `SIGN_ID` and `NOTARY_PROFILE` (saved with `xcrun notarytool store-credentials`) the image is signed,
notarized and stapled, so Gatekeeper opens it without a warning. Without them, the first start on another
Mac needs a right-click, then **Open**. A `.sha256` file is written next to the image. If
[`create-dmg`](https://github.com/create-dmg/create-dmg) is installed, the window gets a fixed size and
icon positions.

### Permissions on macOS

CursorPie needs **Accessibility** to watch the mouse and send shortcuts, and macOS may also ask for
**Input Monitoring**.

1. Open **System Settings > Privacy & Security**.
2. Enable CursorPie (or your terminal app, if you used `cargo run`) under **Accessibility**, and under
   **Input Monitoring** if asked.

CursorPie checks again every 2 seconds, so it starts working as soon as you grant access. If **Settings >
General** still shows *Event tap: Not running*, quit and reopen the app. That tab also has buttons that jump
to the right System Settings pages.

An ad-hoc signed build may need the permission again after each rebuild. Sign with `SIGN_ID` to avoid it.

## Windows

In PowerShell, with the MSVC build tools installed:

```powershell
scripts\package-windows.ps1     # builds target\bundle\CursorPie-windows.zip
```

**Setup wizard.** Install [Inno Setup 6](https://jrsoftware.org/isinfo.php) (`winget install JRSoftware.InnoSetup`)
and the same script also builds `CursorPie-<version>-setup.exe` (`-NoInstaller` skips it). The wizard installs for
your user only, needs no administrator rights, adds a Start Menu shortcut and an *Apps & features* entry, asks
whether to start CursorPie at sign-in, and updates an older version in place. Your settings are never touched.
Set `SIGN_CERT_THUMBPRINT` to sign the `.exe` files; unsigned installers make SmartScreen show a warning.

**Without the wizard**, unzip and run `cursorpie.exe`. A pie icon appears in the system tray. The first start
installs CursorPie for your user: it copies itself to `%LOCALAPPDATA%\Programs\CursorPie`, turns on
**Launch at login**, adds an entry to **Settings > Apps** and continues from the installed copy. You can delete
the unzipped folder afterwards. Running a newer `cursorpie.exe` updates the installed copy and keeps your
settings. Builds made with `cargo` are never installed, and `CURSORPIE_NO_INSTALL=1` turns the install step off
for any copy.

**Uninstall** from **Settings > Apps > CursorPie > Uninstall**, or run `cursorpie.exe --uninstall`. Your
`config.json` is kept. To only stop it starting at login, untick **Launch at login** in Settings or in the
tray menu.

Windows needs no permission. To control windows of programs that run as administrator, start CursorPie as
administrator too, because Windows blocks input to them otherwise.

## Linux (X11)

Linux (X11) is supported but **experimental**: it works, but its behavior on different distributions and
desktops is still being observed, so expect rough edges and please report them. Install the development files
first:

```sh
# Debian, Ubuntu
sudo apt install build-essential libx11-dev libxi-dev libxtst-dev libxrandr-dev
# Fedora
sudo dnf install gcc libX11-devel libXi-devel libXtst-devel libXrandr-devel
```

Then build a tarball and install it:

```sh
scripts/package-linux.sh                      # builds target/bundle/cursorpie-<version>-linux-<arch>.tar.gz
tar -xzf target/bundle/cursorpie-*-linux-*.tar.gz
cd cursorpie-*-linux-*
./install.sh --user                           # under ~/.local; plain ./install.sh uses /usr/local
```

Start CursorPie from your applications menu or run `cursorpie`. It needs no permission and can start at login
from **Settings > Behavior**. The icon in the package needs ImageMagick at build time; without it the package is
built without an icon. Build on an older distribution if the binary must also start on older ones.

Good to know:

- Log in with an **X11 session**. In a Wayland session CursorPie only sees X11 and XWayland windows. See
  [KNOWN_ISSUES.md](KNOWN_ISSUES.md).
- There is no tray icon yet. Pause and Quit are in **Settings > General**, and Settings opens by holding the
  trigger in the middle of the pie.
- A minimal system may lack libraries the window toolkit loads at run time (OpenGL, libxkbcommon-x11, libxcursor).
- Install `zenity` or `kdialog` to use the file chooser buttons.

## First launch

On macOS and Windows a short welcome window opens the first time. It shows the gesture, then the one thing to
do next (on macOS, allow Accessibility with a live status; on Windows, where to find the tray icon) and ends with
a **Launch at login** box. Skip it with **Skip** or `Esc`. **Welcome guide...** in the menu bar or tray menu shows
it again. The "already shown" flag is the empty file `welcome_shown` next to `config.json`; delete it to see the
window at the next start.

## Next steps

- [User guide](USER_GUIDE.md): gestures, slices, profiles and Settings.
- [Configuration](CONFIGURATION.md): where `config.json` lives and what it contains.

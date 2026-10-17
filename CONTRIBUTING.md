# Contributing to CursorPie

Thank you for helping! This guide covers how to set up, build, test and submit changes.
The technical documentation lives in the Rust docs (see [Documentation](#documentation)).

## Prerequisites

- **macOS 11 or later, or Windows 10 (1809) / 11.** Linux builds a do-nothing stub, enough to run `cargo test`.
- **Rust 1.88 or later** (edition 2024), installed with [rustup](https://rustup.rs).
- macOS: Xcode Command Line Tools (`xcode-select --install`) for the linker and `codesign`.
- Windows: the MSVC build tools (Visual Studio Build Tools, "Desktop development with C++").

## Build and run

```sh
cargo run --release     # quick try; on macOS grant Accessibility to your terminal app
scripts/bundle.sh       # macOS: builds target/bundle/CursorPie.app (see README.md)
scripts/make-dmg.sh     # macOS: builds the .dmg to hand out (see README.md)
scripts/package-windows.ps1   # Windows: builds target/bundle/CursorPie-windows.zip
```

CursorPie needs the **Accessibility** permission (and sometimes *Input Monitoring*) to watch the
mouse. When you start it with `cargo run`, macOS attributes the permission to your terminal app.
While you develop, a rebuilt ad-hoc-signed binary may need the permission granted again.

## Build for every platform

CursorPie is one code base with three kinds of build. Only a native build on macOS or Windows gives
you a working app; the other builds exist to check that your change still compiles there.

| You are on | You want | Run | Result |
|---|---|---|---|
| macOS | the macOS app | `scripts/bundle.sh` | `target/bundle/CursorPie.app` |
| macOS | a disk image to distribute | `scripts/make-dmg.sh` | `target/bundle/CursorPie-<version>-<arch>.dmg` |
| Windows | the Windows app | `scripts\package-windows.ps1` | `target/bundle/CursorPie-windows.zip`, and `CursorPie-<version>-setup.exe` if Inno Setup 6 is installed |
| Linux | to run the tests | `cargo test` | no app: Linux builds the do-nothing stub |
| any OS | to type-check the Windows backend | `cargo check --features check-windows` | no binary |
| macOS or Linux | a Windows `.exe` (best effort) | see [Cross-compiling to Windows](#cross-compiling-to-windows) | `cursorpie.exe` |

### macOS

```sh
xcode-select --install          # once: linker and codesign
scripts/bundle.sh               # ad-hoc signature, for your own machine
SIGN_ID="Developer ID Application: Name (TEAMID)" scripts/bundle.sh   # signed with your identity
```

`scripts/make-dmg.sh` wraps the result in a `.dmg` (see `README.md` for signing and notarizing). It
accepts the same `SIGN_ID`, and `UNIVERSAL=1` replaces the manual steps below.

`scripts/bundle.sh` builds for the architecture of the machine it runs on. To ship one app for both
Intel and Apple silicon, build both targets, join them with `lipo`, put the result into the bundle
and sign it again:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
lipo -create -output target/CursorPie-universal \
    target/aarch64-apple-darwin/release/cursorpie target/x86_64-apple-darwin/release/cursorpie
scripts/bundle.sh
cp target/CursorPie-universal target/bundle/CursorPie.app/Contents/MacOS/CursorPie
codesign --force --deep --sign - target/bundle/CursorPie.app
lipo -info target/bundle/CursorPie.app/Contents/MacOS/CursorPie   # must list x86_64 and arm64
```

### Windows

Install the MSVC build tools, then in PowerShell:

```powershell
scripts\package-windows.ps1
```

The result is a zip with `cursorpie.exe`, `LICENSE` and `README.md`. The executable is a tray app
without a console window. It is not code signed, so SmartScreen warns about it on first start.

The first start of a packaged `cursorpie.exe` installs it for the current user (see
`platform/windows/install.rs`). Builds that sit in a `target\debug` or `target\release` folder are never
installed, so `cargo run --release` cannot replace the copy that starts at login; set
`CURSORPIE_NO_INSTALL=1` to get the same for any other copy. To test the install, run the packaged exe
from the unzipped folder and follow checks 7a to 7c of `docs/WINDOWS_MIGRATION.md`.

If [Inno Setup 6](https://jrsoftware.org/isinfo.php) is installed (`winget install JRSoftware.InnoSetup`),
`scripts\package-windows.ps1` also builds the Setup wizard `target\bundle\CursorPie-<version>-setup.exe` from
`installer/cursorpie.iss` (`-NoInstaller` skips it). It installs per user, needs no administrator rights and
keeps `config.json`. A program installed by the wizard does not install itself again. Test it with check 7d of
`docs/WINDOWS_MIGRATION.md`. Set `SIGN_CERT_THUMBPRINT` (and have `signtool` on `PATH`) to sign the
executable and the wizard.

### Linux

Linux has no real backend yet. `cargo build` and `cargo test` work (the stub does nothing), which is
enough to run the whole unit test suite and to write platform-independent code. How a real Linux backend
would be built, and in which order, is described in `docs/LINUX_MIGRATION.md` (a proposal). If linking fails,
install your distribution's X11, Wayland and xkbcommon development packages.

### Cross-compiling to Windows

You can produce a `cursorpie.exe` from macOS or Linux with
[cargo-xwin](https://github.com/rust-cross/cargo-xwin). It needs `clang` and `lld` (LLVM) and
downloads the Windows SDK on first use, which means accepting Microsoft's license:

```sh
rustup target add x86_64-pc-windows-msvc
cargo install --locked cargo-xwin
cargo xwin build --release --target x86_64-pc-windows-msvc
# result: target/x86_64-pc-windows-msvc/release/cursorpie.exe
```

Treat this as a smoke test. The mouse hook, the tray icon, DPI handling and the transparent window only
prove themselves on a real Windows machine, so run the manual checklist in `docs/WINDOWS_MIGRATION.md`
before you call a change finished. CI-style checks that need no Windows machine are
`cargo check --features check-windows` and `cargo clippy --all-targets --features check-windows`.

### What each OS can check

Rust only compiles the code of the OS you build on, so a lint can only see that code:

| Code | Checked by |
|---|---|
| `platform/macos/`, `tap/macos.rs` | `cargo clippy` and `cargo test` on macOS only |
| `platform/windows/`, `tap/windows.rs` | Windows, or any OS with `--features check-windows` |
| everything else | any OS |

To release, follow [Releasing](#releasing). Never change the version by hand.

## Development workflow

Everything you run for the project is one command: `cargo tools <command>`. The tools live in the
`tools/` package (standard library only, never part of the app) and behave the same on macOS,
Windows and Linux. `cargo tools help` lists them.

| Command | What it does |
|---|---|
| `cargo tools hooks install` | once per clone: turns on the Git hooks of `.githooks/` |
| `cargo tools ci` | every check CI runs, in order, stopping at the first failure |
| `cargo tools ci --quick` | only the checks that take seconds (formatting, version files) |
| `cargo tools check` | `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md` agree on the version |
| `cargo tools bump LEVEL` | raises the version and prepares the changelog (see [Releasing](#releasing)) |
| `cargo tools commit-msg FILE` | checks a commit message (the `commit-msg` hook runs it) |

### The loop

```text
1. cargo tools hooks install          once, right after cloning
2. git switch -c fix/short-name       start from an up-to-date main; one branch per change
3. edit, test, commit in small steps  the commit-msg hook checks every subject
4. cargo tools ci                     the pre-push hook runs it before every push
5. open a pull request                CI must be green on macOS, Windows and Linux
6. squash-merge into main             the pull request title becomes the commit subject
```

The hooks run `cargo tools ci --quick` before a commit, check the subject of the message, and run
`cargo tools ci` before a push. They are a convenience: `--no-verify` skips them once, and CI runs
the same command, so CI has the last word. A branch without a pull request does not start CI: open
a draft pull request when you want it.

### Before you open a pull request

Run `cargo tools ci` on your OS. It must finish without warnings. It runs, in this order:

| Step | Command | Where |
|---|---|---|
| format | `cargo fmt --all -- --check` | everywhere |
| version files agree | `cargo tools check` | everywhere |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | everywhere |
| tests | `cargo test --workspace --all-targets` | everywhere |
| doc tests | `cargo test --doc` | everywhere |
| docs | `cargo doc --workspace --no-deps --document-private-items`, with `RUSTDOCFLAGS="-D warnings"` | macOS only: docs.rs builds for macOS, and only there the links to the macOS backend resolve |
| Windows backend | `cargo clippy --all-targets --features check-windows -- -D warnings` and `cargo test --features check-windows` | macOS and Linux (on Windows the backend is the build itself) |

The list lives in `tools/src/ci.rs` and CI runs the same command, so there is one definition of
"the checks pass". Plain `cargo test` runs only the app: use `cargo test --workspace` to include
the tools. `cargo docs` (an alias from `.cargo/config.toml`) builds the docs and opens them.

### Project rules

These rules keep the project maintainable by more than one person over a long time. Each one
exists because the alternative costs more later.

- **`main` is always releasable.** Nothing is pushed to it directly: every change arrives in a pull
  request with green CI. Maintainers: switch on branch protection in the GitHub settings (require a
  pull request and the three `test` jobs).
- **One concern per pull request.** Formatting-only changes and drive-by cleanups go in their own
  pull request, so a review shows what changed and `git blame` stays useful.
- **Zero warnings.** rustfmt, clippy and rustdoc run with warnings as errors. Fix the cause; add an
  `#[allow(...)]` only with a comment that says why.
- **Tests come with logic.** A bug fix starts with a test that fails before the fix, whenever the
  bug can be reproduced without a screen. What cannot be unit tested (gestures, rendering, caches
  after sleep) is listed under "Tested by hand" in the pull request.
- **Notes for users.** A user-visible change adds a line under `## Unreleased` in `CHANGELOG.md` in
  the same pull request, written for users of the app. Refactors, tests, docs and tooling need none.
  A breaking change starts its line with `Breaking:` and says how to migrate.
- **The version and the tags belong to the release tool.** Only maintainers release, from `main`,
  with `cargo tools bump` (see [Releasing](#releasing)). A pushed tag is never moved, deleted or
  reused: a bad release is fixed by the next patch release.
- **Dependencies stay few and reviewed.** `Cargo.lock` is committed, so every build uses the
  versions that were tested. A new dependency needs a reason in the pull request (what it does, why
  the standard library is not enough, its license, whether it is maintained). Dependency updates are
  their own pull request.
- **Both OSes stay in step.** A change in `platform/`, `tap/` or behind a `cfg` is checked for the
  other OS too (`cargo tools ci` type-checks the Windows backend on macOS and Linux), and the manual
  checklists in `docs/` are run for what the change touches.
- **Old config files keep loading.** See *Config compatibility* under Coding guidelines.

### Commit messages

The release tool turns commit subjects into changelog lines when nobody wrote notes, so a subject
is written for a reader of the changelog. The `commit-msg` hook rejects a message when:

- the subject is shorter than 10 or longer than 72 characters;
- it does not start with a capital letter, or ends with a full stop;
- it starts with a log word instead of an order (`Fixed`, `Adding`, `WIP`): write `Fix`, `Add`,
  `Update`;
- there is no blank line between the subject and the body.

Merges, reverts, `Release ...` commits and `fixup!` or `squash!` commits are not checked. The body
is for the *why*: what you tried, what you ruled out, what the reader must know.

## Project layout

```text
src/main.rs        thin binary: calls cursorpie::run()
src/lib.rs         crate root: module list, run()
src/config/        data model, defaults, validation, config.json, backups, profile sharing, the live global config
src/tap/           global event tap thread and the gesture state machine
src/actions.rs     worker thread that executes actions in order
src/cache.rs       bounded LRU cache (icon textures)
src/history.rs     undo / redo snapshots (Settings editor)
src/side.rs        which pie the back/forward buttons follow
src/platform/      OS glue behind one facade: macos/, windows/, stub.rs, plus keymap.rs (key tables)
docs/              WINDOWS_MIGRATION.md: design, phases and the manual test checklist;
                   UI_DESIGN.md: look, rendering rules and the manual visual checklist;
                   OTHER_DESKTOPS.md: Open app across Spaces / desktops, logic and manual check
src/ui/            everything on the UI thread: app, pie renderer, Settings, menu bar icon,
                   icon cache (`icons.rs`) and idle policy (`sleep.rs`)
src/guides/        long-form developer docs (rendered by rustdoc)
tools/             the repository tools, run as `cargo tools ...`: ci, check, bump (see Releasing), hooks, commit-msg
.githooks/         Git hooks that call the tools (turned on with `cargo tools hooks install`)
.github/           the CI workflow and the pull request template
scripts/bundle.sh  builds and signs the .app
scripts/package-windows.ps1   builds the Windows zip
```

Start with the `guides::architecture` page of the Rust docs: it explains the threads, the data
flow and the rules that keep the event tap safe.

## Coding guidelines

- **Format** with `rustfmt` defaults. Do not mix formatting-only changes into functional ones.
- **Comments explain *why*, not *what*.** Add a short comment where the reason is not obvious
  (a macOS quirk, an ordering constraint, a lock rule). Do not narrate the code.
- **Document public items** with `///`, and every module with `//!`. Start with a one-line
  summary. Use intra-doc links (``[`Config`](crate::config::Config)``) rather than plain code
  names where an item exists. Add `# Panics`, `# Errors` or `# Safety` sections when they apply.
- **Write comments, docs, commit messages and identifiers in English.**
- **Keep the event tap safe** (details in the architecture guide):
  never block in the callback, never panic out of it, pass events through when unsure, release
  the gesture lock before sending messages, and keep the lock order *gesture, then config, then
  side state*.
- **Keep the Settings look consistent.** Colors, corner radii and spacing come from
  `src/ui/settings/theme.rs` (controls are 32 px high, the gap is 8 px), controls from
  `src/ui/settings/widgets.rs` (see `guides::extending` and `docs/UI_DESIGN.md`). No emoji on buttons.
- **Keep drawing sharp.** Never push a hand-made `egui::Mesh` for something with an edge: use
  `ui::draw::ring_slice` (anti-aliased) or an egui shape. Drive animations with `ui::draw::step_linear`
  plus one easing curve, not by feeding an exponential through an easing function. Test the geometry
  and math you add (see the tests in `src/ui/draw.rs`).
- **Bound every cache and give it a way out.** Anything the UI keeps between frames (textures,
  decoded images, lists read from disk) needs a size limit, and must be dropped in
  `App::enter_sleep` or when Settings closes (see `guides::architecture`, *Idle sleep and
  caches*). Never key a cache by unbounded input without eviction. Do not decode or read files on
  the frame that opens the pie: warm them in `App::warm`.
- **Keep Settings undoable.** Settings edits `App::edit`, and `History` watches it as a whole, so
  state that belongs in the config needs no extra code. State kept elsewhere (`Ed`) is not part of
  the history: make it tolerate a config that changed under it (see `Ed::fit_to`).
- **One code base, two OSes.** Put OS calls in `src/platform/<os>/` (or the native hook glue `src/tap/macos.rs`, `src/tap/windows.rs`) and give every backend the same function names. Everything above them takes an `Os` or portable types (`geometry::Point`, `tap::Ev`). Do not add `cfg` attributes to call sites. Keep config keys macOS virtual key codes; translate at the edge. Do not break macOS when working on Windows, and vice versa: run the tests on both.
- **Keep logic testable.** Put decisions that need no OS API in plain functions and unit
  test them (see `side::resolve`, `config` tests, `appicon` tests). `cache`, `history`,
  `ui::sleep` and `config::backup` are written with no macOS or egui types on purpose, and take
  the clock or the directory as a parameter instead of reading them.
- **Never edit the version by hand.** It lives in `Cargo.toml` and is raised only by the release tool
  (`cargo tools bump`, see [Releasing](#releasing)), which also keeps `Cargo.lock` and
  `CHANGELOG.md` in step. Add user-visible notes under `## Unreleased` in `CHANGELOG.md` as you work.
  The tools in `tools/` follow the same rules as the app: standard library only, decisions in pure
  functions, every function documented and tested.
- **No new dependencies** without a good reason. The app is deliberately small and makes no
  network connections.
- **Config compatibility.** New settings need a `Default`; older `config.json` files must keep
  loading (see `guides::config_format`).
- **Security.** `Command` actions run shell commands with the user's permissions. Never run or
  import one silently.

For step-by-step instructions on common changes (a new action kind, a setting, a Settings tab,
icons...) see `guides::extending`.

## Commits and pull requests

- Keep a pull request focused on one change. Describe *what* and *why*; link the issue.
- Use clear, imperative commit subjects (`Fix double-click replay on slow clicks`; the rules are
  under [Commit messages](#commit-messages)). Add a body when the reason is not obvious. Write the subject for a *user* of the app when the change is
  visible: if nobody wrote notes under `## Unreleased`, the release tool turns the subjects since
  the last release into the changelog entry.
- Pull requests are squash-merged, so the title of the pull request is the subject that stays in
  the history: write it by the same rules.
- Include tests for logic changes, and update `README.md` when behavior or settings change.
- For anything that touches caching, sleep or undo, say what you checked by hand: pie opening after a
  long idle time, Settings after sleep, Cmd+Z after deleting a slice, restoring a backup.
- Mention what you tested by hand (macOS or Windows version, trigger button, which apps) for gesture or UI
  changes, since those cannot be covered by unit tests.

## Releasing

The version is raised by a tool, not by hand, so `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md` can
never disagree. It uses only the standard library, so it runs on every OS:

```sh
cargo tools bump minor --dry-run   # 1. look at the plan: nothing is written
cargo tools bump minor             # 2. do it
cargo tools bump minor --commit    #    or do it, commit "Release X.Y.Z" and tag vX.Y.Z
cargo tools check                  # verify the files agree (also part of cargo tools ci)
```

### Which level

CursorPie uses `MAJOR.MINOR.PATCH`. Pick the level by what the *user* sees, not by how much code
changed:

| Level | Use it for | Example |
|---|---|---|
| `patch` | fixes only: nothing new, nothing changed for the user | `0.3.0` -> `0.3.1` |
| `minor` | a new feature or setting, or changed behavior. **Below 1.0.0 a breaking change is also a minor release** | `0.3.1` -> `0.4.0` |
| `major` | from 1.0.0 on, a breaking change (a config that no longer loads, a removed feature); and the step to 1.0.0 itself | `0.4.0` -> `1.0.0` |

The tool only moves one step forward. It never goes back, never skips levels and never makes a
pre-release (`-beta`); a version that was released is never reused. The lower numbers reset to zero.
The version of the config file (`CONFIG_VERSION`) is a separate number with its own rule, see
`guides::config_format`.

### What the tool does

- Raises `version` in `Cargo.toml`, and the `cursorpie` entry of `Cargo.lock` when the file exists.
  Everything else that shows the version reads `Cargo.toml`: `about::VERSION` (the About tab and the
  Windows "Apps & features" entry) and `scripts/bundle.sh` (the `.app` bundle).
- Writes the changelog entry as a `## X.Y.Z` section on top of `CHANGELOG.md`:
  1. the notes you wrote under `## Unreleased` become the section (this is the best source: it is
     written for readers);
  2. without them, the subjects of the commits since the last `v*` tag are listed, minus merges and
     `Release ...` commits. Use `--since REV` when there is no tag. Generated entries are always
     flagged for review.
  3. with neither, it stops and changes nothing.
- Leaves a fresh, empty `## Unreleased` section on top of the changelog, where the notes of the next
  release go.
- With `--commit` it commits those files as `Release X.Y.Z` and tags `vX.Y.Z`. That needs a clean
  working tree, the `main` branch (`--any-branch` allows another one, for a rehearsal) and a tag
  that does not exist yet. It never pushes.
- Refuses to start when `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md` already disagree, and writes
  nothing unless every file could be prepared. If a write fails halfway, the files already written
  are put back, so a failed run leaves the repository as it was.

### What you still do by hand

The tool cannot judge these, so it prints them at the end of every run, with a `WARNING:` where
something is probably wrong right now:

1. **Read the changelog entry.** Always, and especially when it was generated.
2. **Rewrite sentences that name the old version**, such as the "Version 0.3.0 adds ..." line at the
   top of `README.md`. The tool finds them and tells you the line, but cannot rephrase them. Do not
   touch mentions of older versions in the history of `CHANGELOG.md` or `docs/`: they are facts.
3. **Decide on `CONFIG_VERSION`.** If this release changes `config.json` in a way an old reader would
   misread, raise it in `src/config/mod.rs` and write a migration. A new setting with a `Default`
   needs nothing.
4. **Update `README.md`** for a `minor` or `major` release when behavior or settings changed.
5. **Build, test and publish.** Build on each OS, run the manual checklists, push with
   `git push --follow-tags` (CI then checks that the tag matches the files), and attach `CursorPie-<version>-<arch>.dmg` (from `scripts/make-dmg.sh`, with its `.sha256`), `CursorPie-<version>-setup.exe` and
   `CursorPie-windows.zip` to a GitHub release. Keep `BUNDLE_ID` equal in `src/about.rs` and `scripts/bundle.sh`.

If `check` reports a mismatch, someone changed the version by hand: undo that change and run the
tool instead.

## Reporting bugs

Open an issue with: macOS or Windows version, how CursorPie was started (`.app` or `cargo run`), the
trigger button, what you did, what you expected, and the output of `cargo run --release` from a
terminal (CursorPie prints diagnostics to stderr). Attach your `config.json` if it is relevant,
after removing any commands you do not want to share.

## Documentation

- Rust docs: `cargo docs` (or docs.rs once published). Source of truth for developers.
- `README.md`: user-facing documentation.
- Docs for the guides live in `src/guides/*.rs` as ordinary `//!` comments.

## License

By contributing you agree that your contributions are licensed under the project's
MIT License (see the `LICENSE` file).

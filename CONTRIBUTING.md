# Contributing to CursorPie

Thank you for helping! This guide covers how to set up, build, test and submit changes.
The technical documentation lives in the Rust docs (see [Documentation](#documentation)).

## Prerequisites

- **macOS 11 or later.** The code uses macOS-only APIs and refuses to build on other platforms.
- **Rust 1.88 or later** (edition 2024), installed with [rustup](https://rustup.rs).
- Xcode Command Line Tools (`xcode-select --install`) for the linker and `codesign`.

## Build and run

```sh
cargo run --release     # quick try; grant Accessibility to your terminal app
scripts/bundle.sh       # builds target/bundle/CursorPie.app (see README.md)
```

CursorPie needs the **Accessibility** permission (and sometimes *Input Monitoring*) to watch the
mouse. When you start it with `cargo run`, macOS attributes the permission to your terminal app.
While you develop, a rebuilt ad-hoc-signed binary may need the permission granted again.

## Before you open a pull request

Run all of these:

```sh
cargo fmt --all -- --check      # formatting (rustfmt defaults)
cargo clippy --all-targets      # lints (do not add new warnings)
cargo test                      # unit tests
cargo docs                      # builds the docs with private items and opens them
```

`cargo docs` is an alias defined in `.cargo/config.toml`. To fail on documentation warnings
(such as broken links) run
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items`.

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
src/platform/      macOS glue (permissions, apps, login item, keyboard layout, shell tools)
src/ui/            everything on the UI thread: app, pie renderer, Settings, menu bar icon,
                   icon cache (`icons.rs`) and idle policy (`sleep.rs`)
src/guides/        long-form developer docs (rendered by rustdoc)
scripts/bundle.sh  builds and signs the .app
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
- **Keep the Settings look consistent.** Square corners, colors and spacing come from
  `src/ui/settings/theme.rs` (controls are 32 px high, the gap is 8 px), controls from
  `src/ui/settings/widgets.rs` (see `guides::extending`). No emoji on buttons.
- **Bound every cache and give it a way out.** Anything the UI keeps between frames (textures,
  decoded images, lists read from disk) needs a size limit, and must be dropped in
  `App::enter_sleep` or when Settings closes (see `guides::architecture`, *Idle sleep and
  caches*). Never key a cache by unbounded input without eviction. Do not decode or read files on
  the frame that opens the pie: warm them in `App::warm`.
- **Keep Settings undoable.** Settings edits `App::edit`, and `History` watches it as a whole, so
  state that belongs in the config needs no extra code. State kept elsewhere (`Ed`) is not part of
  the history: make it tolerate a config that changed under it (see `Ed::fit_to`).
- **Keep logic testable.** Put decisions that need no macOS API in plain functions and unit
  test them (see `side::resolve`, `config` tests, `appicon` tests). `cache`, `history`,
  `ui::sleep` and `config::backup` are written with no macOS or egui types on purpose, and take
  the clock or the directory as a parameter instead of reading them.
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
- Use clear, imperative commit subjects (`Fix double-click replay on slow clicks`). Add a body
  when the reason is not obvious.
- Include tests for logic changes, and update `README.md` when behavior or settings change.
- For anything that touches caching, sleep or undo, say what you checked by hand: pie opening after a
  long idle time, Settings after sleep, Cmd+Z after deleting a slice, restoring a backup.
- Mention what you tested by hand (macOS version, trigger button, which apps) for gesture or UI
  changes, since those cannot be covered by unit tests.

## Reporting bugs

Open an issue with: macOS version, how CursorPie was started (`.app` or `cargo run`), the
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

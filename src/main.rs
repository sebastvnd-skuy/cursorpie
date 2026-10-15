//! Binary entry point. All logic lives in the `cursorpie` library crate (see `src/lib.rs`).

// Release builds on Windows are tray apps: no console window. (Debug builds keep it for diagnostics.)
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(any(target_os = "macos", windows))]
fn main() -> eframe::Result<()> {
    cursorpie::run()
}

#[cfg(not(any(target_os = "macos", windows)))]
fn main() {
    eprintln!("CursorPie runs on macOS and Windows only.");
    std::process::exit(1);
}

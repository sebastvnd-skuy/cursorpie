//! Linux support. See `docs/LINUX_MIGRATION.md` for the design.
//!
//! The pure helpers (session detection, desktop entries, window hit test, XDG paths, autostart file
//! text, launch resolution, display scale) compile on every OS, so their tests run everywhere. The X11
//! backend (`xlib`, `x11`, `backend`) is only compiled on Linux.

pub mod autostart;
pub mod desktop_entry;
pub mod launch;
pub mod scale;
pub mod session;
pub mod toplevels;
pub mod xdg;

#[cfg(target_os = "linux")]
pub(crate) mod backend;
#[cfg(target_os = "linux")]
pub(crate) mod x11;
#[cfg(target_os = "linux")]
pub(crate) mod xlib;

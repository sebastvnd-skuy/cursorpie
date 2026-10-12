//! Everything that lives on the UI thread.
//!
//! | Module | Contents |
//! |---|---|
//! | [`app`] | the eframe app: state, message handling, window management |
//! | [`pie`] | drawing the pie |
//! | [`settings`] | the Settings window (tabs, editors, icon picker) |
//! | [`tray`] | the menu bar icon and menu |
//! | [`appicon`] | real app icons, reduced to white glyphs |
//! | [`draw`] | drawing helpers shared by the pie and Settings |
//! | [`screen`] | display lookup for window placement |
//!
//! The UI never runs actions or inspects raw input: the event tap does that and sends
//! [`Msg`](crate::msg::Msg) values here.

pub mod app;
pub mod appicon;
pub mod draw;
pub mod pie;
pub mod screen;
pub mod settings;
pub mod tray;

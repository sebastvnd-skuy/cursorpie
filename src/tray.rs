//! Menu bar icon: Settings, About, Pause, Launch at login, Quit.
//!
//! tray-icon on macOS must be created on the main thread once the event loop is running;
//! eframe calls the app creator from `resumed`, which satisfies that.

use crate::about::APP_NAME;
use crate::msg::{Msg, Notifier};
use crate::system;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct TrayHandles {
    /// keep alive: dropping it removes the icon
    pub _icon: TrayIcon,
    pub pause: CheckMenuItem,
    pub login: CheckMenuItem,
}

/// Four pie slices as a monochrome template image (macOS recolors it for light/dark menu bars).
fn icon_rgba(n: u32) -> Vec<u8> {
    let mut px = vec![0u8; (n * n * 4) as usize];
    let c = n as f32 / 2.0;
    for y in 0..n {
        for x in 0..n {
            let dx = x as f32 + 0.5 - c;
            let dy = y as f32 + 0.5 - c;
            let d = dx.hypot(dy) / c;
            let ang = dy.atan2(dx);
            // gaps along the four diagonals
            let from_diag = (ang.rem_euclid(FRAC_PI_2) - FRAC_PI_4).abs();
            let in_gap = from_diag < 0.11;

            let ring = ((0.94 - d) * c).clamp(0.0, 1.0) * ((d - 0.50) * c).clamp(0.0, 1.0);
            let ring = if in_gap { 0.0 } else { ring };
            let dot = ((0.20 - d) * c).clamp(0.0, 1.0);
            let alpha = ring.max(dot);

            let i = ((y * n + x) * 4) as usize;
            px[i + 3] = (alpha * 255.0) as u8; // black + alpha
        }
    }
    px
}

pub fn build(n: &Notifier) -> Option<TrayHandles> {
    let menu = Menu::new();
    let settings = MenuItem::with_id("settings", "Settings...", true, None);
    let about = MenuItem::with_id("about", format!("About {APP_NAME}"), true, None);
    let pause = CheckMenuItem::with_id("pause", format!("Pause {APP_NAME}"), true, false, None);
    let login = CheckMenuItem::with_id("login", "Launch at login", true, system::launch_at_login_enabled(), None);
    let quit = MenuItem::with_id("quit", format!("Quit {APP_NAME}"), true, None);

    let _ = menu.append(&settings);
    let _ = menu.append(&about);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&pause);
    let _ = menu.append(&login);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&quit);

    // forward menu clicks to the UI thread (this also wakes it up)
    let notifier = n.clone();
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        notifier.send(Msg::Tray(e.id.0.clone()));
    }));

    let icon = match Icon::from_rgba(icon_rgba(44), 44, 44) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("CursorPie: could not create tray icon image: {e}");
            return None;
        }
    };
    match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(APP_NAME)
        .with_icon(icon)
        .with_icon_as_template(true)
        .build()
    {
        Ok(tray) => Some(TrayHandles { _icon: tray, pause, login }),
        Err(e) => {
            eprintln!("CursorPie: could not create menu bar icon: {e}");
            None
        }
    }
}

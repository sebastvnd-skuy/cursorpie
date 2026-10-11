#[cfg(not(target_os = "macos"))]
compile_error!("CursorPie only supports macOS.");

mod about;
mod actions;
mod app;
mod config;
mod gfx;
mod icons;
mod keys;
mod layout;
mod msg;
mod pie;
mod settings;
mod system;
mod tap;
mod tray;

use app::App;
use eframe::egui;
use msg::Notifier;
use std::sync::mpsc;

fn main() -> eframe::Result<()> {
    actions::init();
    let (tx, rx) = mpsc::channel();
    let win = config::cfg().pie_size;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([win, win])
            .with_decorations(false)
            .with_transparent(true)
            .with_has_shadow(false)
            .with_resizable(false)
            .with_always_on_top()
            .with_mouse_passthrough(true)
            .with_active(false)
            .with_visible(false),
        ..Default::default()
    };

    eframe::run_native(
        about::APP_NAME,
        options,
        Box::new(move |cc| {
            // runs on the main thread inside the running event loop
            system::set_accessory_policy(); // no Dock icon
            layout::init(); // keyboard-layout snapshot (TIS must run on the main thread)
            let notifier = Notifier {
                tx: tx.clone(),
                ctx: cc.egui_ctx.clone(),
            };
            tap::spawn(notifier.clone());
            let tray = tray::build(&notifier);
            Ok(Box::new(App::new(rx, tx, win, tray)))
        }),
    )
}

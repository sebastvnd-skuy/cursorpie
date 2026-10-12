//! Behavior tab: trigger and timings, look, and how the app / side-button pies are chosen.
//!
//! Two columns of titled panels (`widgets::panel`). Left: what you press and how the pie looks.
//! Right: which pie opens for which app.

use crate::config::{AppGesture, SideTarget, TargetApp};
use crate::ui::app::App;
use crate::ui::draw::rgb;
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use eframe::egui;

/// Space kept free at the right of a slider for its value box ("1200 ms").
const SLIDER_VALUE_W: f32 = 96.0;

/// A label with a full-width slider under it, so every slider has the same size and place.
fn slider_row<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Num,
    range: std::ops::RangeInclusive<Num>,
    suffix: &str,
) {
    muted(ui, label);
    ui.spacing_mut().slider_width = (ui.available_width() - SLIDER_VALUE_W).max(60.0);
    ui.add(egui::Slider::new(value, range).suffix(suffix));
}

/// A label on the left and a color swatch on the right.
fn color_row(ui: &mut egui::Ui, label: &str, color: &mut [u8; 3]) {
    ui.horizontal(|ui| {
        muted(ui, label);
        ui.color_edit_button_srgb(color);
    });
}

impl App {
    /// Draws the Behavior tab. Widgets edit `self.edit`, so nothing changes until Save.
    pub(in crate::ui::settings) fn behavior_tab(&mut self, ui: &mut egui::Ui) {
        let accent = rgb(self.edit.accent);
        let cfg = &mut self.edit;
        ui.columns(2, |cols| {
            // ------------------------------------------------------------ left column
            let ui = &mut cols[0];
            panel(ui, "Trigger", |ui| {
                muted(ui, "Trigger button");
                let name = |b: i64| match b {
                    2 => "Middle button",
                    5 => "Extra button 6",
                    6 => "Extra button 7",
                    _ => "Extra button 8",
                };
                egui::ComboBox::from_id_salt("trigger_button")
                    .selected_text(name(cfg.trigger_button))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for b in [2i64, 5, 6, 7] {
                            ui.selectable_value(&mut cfg.trigger_button, b, name(b));
                        }
                    });
                slider_row(ui, "Hold to open pie", &mut cfg.pie_hold_ms, 150..=1500, " ms");
                slider_row(
                    ui,
                    "Hold to open settings",
                    &mut cfg.settings_hold_ms,
                    500..=4000,
                    " ms",
                );
                toggle(
                    ui,
                    &mut cfg.flick,
                    "Open the pie instantly when I drag (disables middle-drag in apps)",
                    accent,
                );
            });
            panel(ui, "Look & feel", |ui| {
                slider_row(ui, "Pie size", &mut cfg.pie_size, 140.0..=360.0, " px");
                color_row(ui, "Accent color", &mut cfg.accent);
                toggle(ui, &mut cfg.sound, "Play a soft tick when the selection changes", accent);
                toggle(
                    ui,
                    &mut cfg.center_app_icon,
                    "Show the app's icon (white) in the center of an app's own pie",
                    accent,
                );
            });
            panel(ui, "Startup", |ui| {
                toggle(ui, &mut cfg.launch_at_login, "Launch at login", accent);
            });

            // ------------------------------------------------------------ right column
            let ui = &mut cols[1];
            panel(ui, "Pie of the app in front", |ui| {
                choice(
                    ui,
                    &mut cfg.app_gesture,
                    AppGesture::DoubleClick,
                    "Hold = general pie · Double-click = pie of the app (e.g. Photoshop)",
                    accent,
                );
                choice(
                    ui,
                    &mut cfg.app_gesture,
                    AppGesture::Merged,
                    "Classic: hold opens the app's own pie if it has one",
                    accent,
                );
                if cfg.app_gesture == AppGesture::DoubleClick {
                    slider_row(ui, "Double-click speed", &mut cfg.double_click_ms, 150..=500, " ms");
                }
            });
            panel(ui, "App detection", |ui| {
                muted(ui, "Which app picks the pie");
                choice(
                    ui,
                    &mut cfg.target_app,
                    TargetApp::UnderCursor,
                    "The app under the pointer: no need to click its window first",
                    accent,
                );
                choice(
                    ui,
                    &mut cfg.target_app,
                    TargetApp::Focused,
                    "The app in front (the one with keyboard focus)",
                    accent,
                );
                ui.add_enabled_ui(cfg.target_app == TargetApp::UnderCursor, |ui| {
                    toggle(
                        ui,
                        &mut cfg.focus_target,
                        "Bring that app to the front before a slice or side button sends it a shortcut",
                        accent,
                    );
                });
            });
            panel(ui, "Side buttons", |ui| {
                muted(ui, "In an app that has its own pie, the side buttons follow");
                choice(ui, &mut cfg.side_default, SideTarget::App, "The app's pie", accent);
                choice(
                    ui,
                    &mut cfg.side_default,
                    SideTarget::General,
                    "The general pie",
                    accent,
                );
                toggle(
                    ui,
                    &mut cfg.side_adopt,
                    "Choosing a side-button mode in a pie makes the side buttons follow that pie",
                    accent,
                );
                toggle(
                    ui,
                    &mut cfg.side_forget,
                    "Forget that choice when I switch to another app",
                    accent,
                );
                toggle(
                    ui,
                    &mut cfg.right_click_switch,
                    "While the pie is open, right-click switches which pie the side buttons follow",
                    accent,
                );
                toggle(
                    ui,
                    &mut cfg.side_indicator,
                    "Color the pie ring: accent = side buttons follow this pie, other color = the other pie",
                    accent,
                );
                color_row(ui, "Other-pie color", &mut cfg.inactive_color);
                slider_row(
                    ui,
                    "Show the pie after switching (menu bar)",
                    &mut cfg.preview_ms,
                    0..=2000,
                    " ms",
                );
            });
        });
    }
}

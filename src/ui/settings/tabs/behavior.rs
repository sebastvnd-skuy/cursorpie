//! Behavior tab: trigger and timings, look, and how the app / side-button pies are chosen.
//!
//! One column of titled panels (`widgets::panel`), the way the pages of a large application's
//! settings are laid out: what you press, how the pie looks, which pie opens for which app, then
//! startup and memory.

use crate::config::{AppGesture, DesktopMode, SideTarget, TargetApp};
use crate::ui::app::App;
use crate::ui::draw::rgb;
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

/// What the OS calls a desktop: a Space on macOS, a virtual desktop on Windows.
fn desktop_noun() -> &'static str {
    match crate::platform::OS {
        crate::platform::Os::Mac => "Space",
        crate::platform::Os::Windows => "virtual desktop",
    }
}

/// Title of the panel that holds the [`DesktopMode`] choices.
fn desktops_title() -> &'static str {
    match crate::platform::OS {
        crate::platform::Os::Mac => "Other Spaces",
        crate::platform::Os::Windows => "Other desktops",
    }
}

/// The text of one [`DesktopMode`] choice.
fn desktop_mode_label(mode: DesktopMode) -> String {
    let noun = desktop_noun();
    match mode {
        DesktopMode::SwitchToApp => format!("Switch to the {noun} where the app is"),
        DesktopMode::BringToCurrent => {
            format!("Bring the app's window to this {noun} (experimental)")
        }
        DesktopMode::DoNothing => format!("Do nothing: open the app on this {noun}, as before"),
    }
}

impl App {
    /// Draws the Behavior tab. Widgets edit `self.edit`, so nothing changes until Save.
    pub(in crate::ui::settings) fn behavior_tab(&mut self, ui: &mut egui::Ui) {
        let accent = rgb(self.edit.accent);
        let cfg = &mut self.edit;
        page_header(
            ui,
            "Behavior",
            "How the pie opens, how it looks and which pie each app gets.",
        );
        {
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
                        for &b in crate::platform::OS.trigger_buttons() {
                            ui.selectable_value(&mut cfg.trigger_button, b, name(b));
                        }
                    });
                slider_row(
                    ui,
                    "Hold to open pie",
                    &mut cfg.pie_hold_ms,
                    150..=1500,
                    " ms",
                );
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
                toggle(
                    ui,
                    &mut cfg.sound,
                    "Play a soft tick when the selection changes",
                    accent,
                );
                toggle(
                    ui,
                    &mut cfg.center_app_icon,
                    "Show the app's icon (white) in the center of an app's own pie",
                    accent,
                );
            });
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
                    slider_row(
                        ui,
                        "Double-click speed",
                        &mut cfg.double_click_ms,
                        150..=500,
                        " ms",
                    );
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
            panel(ui, desktops_title(), |ui| {
                muted(
                    ui,
                    &format!(
                        "An 'Open app' slice may open an app whose window is on another {}",
                        desktop_noun()
                    ),
                );
                for &mode in crate::platform::OS.desktop_modes() {
                    choice(
                        ui,
                        &mut cfg.desktop_mode,
                        mode,
                        &desktop_mode_label(mode),
                        accent,
                    );
                }
            });
            panel(ui, "Side buttons", |ui| {
                muted(
                    ui,
                    "In an app that has its own pie, the side buttons follow",
                );
                choice(
                    ui,
                    &mut cfg.side_default,
                    SideTarget::App,
                    "The app's pie",
                    accent,
                );
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
                    &format!(
                        "Show the pie after switching ({})",
                        crate::platform::OS.tray_name()
                    ),
                    &mut cfg.preview_ms,
                    0..=2000,
                    " ms",
                );
            });
            panel(ui, "Startup", |ui| {
                toggle(ui, &mut cfg.launch_at_login, "Launch at login", accent);
                muted(ui, crate::platform::OS.startup_hint());
            });
            panel(ui, "Memory", |ui| {
                let mut on = cfg.sleep_after_s != 0;
                if toggle(ui, &mut on, "Free memory while CursorPie is idle", accent).changed() {
                    cfg.sleep_after_s = if on { 60 } else { 0 };
                }
                if cfg.sleep_after_s != 0 {
                    slider_row(
                        ui,
                        "Idle time before freeing",
                        &mut cfg.sleep_after_s,
                        10..=600,
                        " s",
                    );
                }
                muted(
                    ui,
                    "After this long without a pie or Settings, cached icons and the editor's working copy are dropped. They are loaded again when you press the trigger.",
                );
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_desktop_choice_has_its_own_label() {
        let labels: Vec<String> = crate::platform::OS
            .desktop_modes()
            .iter()
            .map(|&m| desktop_mode_label(m))
            .collect();
        for (i, a) in labels.iter().enumerate() {
            assert!(!a.is_empty());
            assert!(!labels[..i].contains(a), "{a} is listed twice");
        }
        assert!(desktop_mode_label(DesktopMode::BringToCurrent).contains("experimental"));
    }
}

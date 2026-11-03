//! Behavior tab: trigger and timings, look, and how the app / side-button pies are chosen.
//!
//! One column of titled panels (`widgets::panel`), the way the pages of a large application's
//! settings are laid out: what you press, how the pie looks, which pie opens for which app, then
//! startup and memory.

use crate::config::{
    AppGesture, AppIconStyle, Appearance, Config, DesktopMode, IconShadow, PieStyle, SideTarget,
    TargetApp,
};
use crate::ui::app::App;
use crate::ui::draw::rgb;
use crate::ui::pie::glass::Glass;
use crate::ui::settings::demo;
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use crate::ui::symbols;
use eframe::egui::{self, Color32, Vec2};

/// What the OS calls a desktop: a Space on macOS, a virtual desktop on Windows, a workspace on Linux.
fn desktop_noun() -> &'static str {
    match crate::platform::OS {
        crate::platform::Os::Mac => "Space",
        crate::platform::Os::Windows => "virtual desktop",
        crate::platform::Os::Linux => "workspace",
    }
}

/// Title of the panel that holds the [`DesktopMode`] choices.
fn desktops_title() -> &'static str {
    match crate::platform::OS {
        crate::platform::Os::Mac => "Other Spaces",
        crate::platform::Os::Windows => "Other desktops",
        crate::platform::Os::Linux => "Other workspaces",
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
        panel(ui, "Trigger", |ui| {
            demo::hold_demo(ui, accent, cfg.pie_hold_ms, cfg.settings_hold_ms, cfg.flick);
            let name = |b: i64| match b {
                2 => "Middle button",
                5 => "Extra button 6",
                6 => "Extra button 7",
                _ => "Extra button 8",
            };
            row(
                ui,
                "Trigger button",
                "The mouse button you hold to open the pie.",
                |ui| {
                    let buttons = crate::platform::OS.trigger_buttons();
                    let labels: Vec<&str> = buttons.iter().map(|&b| name(b)).collect();
                    let at = buttons
                        .iter()
                        .position(|&b| b == cfg.trigger_button)
                        .unwrap_or(0);
                    if let Some(i) = select(ui, 190.0, name(cfg.trigger_button), &labels, at) {
                        cfg.trigger_button = buttons[i];
                    }
                },
            );
            slider_row(
                ui,
                "Hold to open the pie",
                &mut cfg.pie_hold_ms,
                150..=1500,
                " ms",
                accent,
            );
            slider_row(
                ui,
                "Hold to open settings",
                &mut cfg.settings_hold_ms,
                500..=4000,
                " ms",
                accent,
            );
            toggle_row(
                ui,
                &mut cfg.flick,
                "Open the pie instantly when I drag",
                "Disables middle-drag in apps.",
                accent,
            );
        });
        panel(ui, "Look & feel", |ui| {
            demo::look_demo(
                ui,
                accent,
                cfg.pie_size,
                cfg.show_labels,
                cfg.center_app_icon,
                cfg.app_icon_style,
                Glass::of(cfg),
            );
            row_wide(ui, "Appearance", "Settings and the pie.", 300.0, |ui| {
                let modes = [
                    (Appearance::System, symbols::MONITOR, "System"),
                    (Appearance::Light, symbols::SUN, "Light"),
                    (Appearance::Dark, symbols::MOON, "Dark"),
                ];
                let items: Vec<(Option<char>, &str)> = modes
                    .iter()
                    .map(|&(_, icon, label)| (Some(icon), label))
                    .collect();
                let at = modes
                    .iter()
                    .position(|m| m.0 == cfg.appearance)
                    .unwrap_or(0);
                if let Some(i) = segmented(ui, &items, at) {
                    cfg.appearance = modes[i].0;
                }
            });
            slider_row(
                ui,
                "Pie size",
                &mut cfg.pie_size,
                140.0..=360.0,
                " px",
                accent,
            );
            row_wide(ui, "Accent color", "", 330.0, |ui| {
                accent_picker(ui, &mut cfg.accent)
            });
            toggle_row(
                ui,
                &mut cfg.sound,
                "Play a soft tick",
                "When the selection changes.",
                accent,
            );
            toggle_row(
                ui,
                &mut cfg.show_labels,
                "Show names under the icons",
                "Off: the pie shows icons only.",
                accent,
            );
            toggle_row(
                ui,
                &mut cfg.center_app_icon,
                "Show the app's icon in the center",
                "In an app's own pie.",
                accent,
            );
        });
        panel(ui, "Pie material", |ui| {
            row_wide(
                ui,
                "Style",
                "Glass is see-through and frosted, always smoked dark.",
                230.0,
                |ui| {
                    let styles = [
                        (PieStyle::Solid, symbols::CHART_PIE_SLICE, "Solid"),
                        (PieStyle::Glass, symbols::SPARKLE, "Glass"),
                    ];
                    let items: Vec<(Option<char>, &str)> = styles
                        .iter()
                        .map(|&(_, icon, label)| (Some(icon), label))
                        .collect();
                    let at = styles
                        .iter()
                        .position(|m| m.0 == cfg.pie_style)
                        .unwrap_or(0);
                    if let Some(i) = segmented(ui, &items, at) {
                        cfg.pie_style = styles[i].0;
                    }
                },
            );
            if cfg.pie_style == PieStyle::Glass {
                glass_preview(ui, cfg);
                slider_row(ui, "Clearness", &mut cfg.glass_clear, 0..=100, " %", accent);
                slider_row(ui, "Frost", &mut cfg.glass_frost, 0..=100, " %", accent);
                slider_row(ui, "Shine", &mut cfg.glass_shine, 0..=100, " %", accent);
            }
        });
        panel(ui, "Pie of the app in front", |ui| {
            demo::gesture_demo(ui, accent, cfg.app_gesture, cfg.double_click_ms);
            choice_row(
                ui,
                &mut cfg.app_gesture,
                AppGesture::DoubleClick,
                "Double-click opens the app's pie",
                "Hold opens the general pie; double-click opens the pie of the app (e.g. Photoshop).",
                accent,
            );
            choice_row(
                ui,
                &mut cfg.app_gesture,
                AppGesture::Merged,
                "Classic",
                "Hold opens the app's own pie if it has one.",
                accent,
            );
            if cfg.app_gesture == AppGesture::DoubleClick {
                slider_row(
                    ui,
                    "Double-click speed",
                    &mut cfg.double_click_ms,
                    150..=500,
                    " ms",
                    accent,
                );
            }
        });
        panel(ui, "App icon in the pie", |ui| {
            demo::app_icon_demo(ui, accent, cfg.app_icon_style, cfg.center_app_icon);
            ui.add_enabled_ui(cfg.center_app_icon, |ui| {
                choice_row(
                    ui,
                    &mut cfg.app_icon_style,
                    AppIconStyle::Glyph,
                    "One color",
                    "A silhouette of the icon in the ink of the pie, like the other icons.",
                    accent,
                );
                choice_row(
                    ui,
                    &mut cfg.app_icon_style,
                    AppIconStyle::Color,
                    "Full color",
                    "The app's own icon, in its own colors.",
                    accent,
                );
            });
            ui.add_enabled_ui(cfg.center_app_icon, |ui| {
                ui.add_space(GAP / 2.0);
                section_title(ui, "Shadow under the icon");
                choice_row(
                    ui,
                    &mut cfg.app_icon_shadow,
                    IconShadow::Auto,
                    "Automatic",
                    "A soft shadow on a plain light center, none where the icon already stands out.",
                    accent,
                );
                choice_row(
                    ui,
                    &mut cfg.app_icon_shadow,
                    IconShadow::Soft,
                    "Soft",
                    "Always a light shadow.",
                    accent,
                );
                choice_row(
                    ui,
                    &mut cfg.app_icon_shadow,
                    IconShadow::Strong,
                    "Strong",
                    "A deeper shadow for pale icons on a white center.",
                    accent,
                );
                choice_row(
                    ui,
                    &mut cfg.app_icon_shadow,
                    IconShadow::Off,
                    "None",
                    "No shadow.",
                    accent,
                );
            });
            if !cfg.center_app_icon {
                note(
                    ui,
                    "Off: turn on \"Show the app's icon in the center\" under Look & feel to choose.",
                );
            }
        });
        panel(ui, "App detection", |ui| {
            demo::target_demo(ui, accent, cfg.target_app, cfg.focus_target);
            choice_row(
                ui,
                &mut cfg.target_app,
                TargetApp::UnderCursor,
                "The app under the pointer",
                "No need to click its window first.",
                accent,
            );
            choice_row(
                ui,
                &mut cfg.target_app,
                TargetApp::Focused,
                "The app in front",
                "The one with keyboard focus.",
                accent,
            );
            ui.add_enabled_ui(cfg.target_app == TargetApp::UnderCursor, |ui| {
                toggle_row(
                    ui,
                    &mut cfg.focus_target,
                    "Bring that app to the front first",
                    "Before a slice or side button sends it a shortcut.",
                    accent,
                );
            });
        });
        panel(ui, desktops_title(), |ui| {
            let mode = cfg.desktop_mode.effective(crate::platform::OS);
            demo::desktop_demo(ui, accent, mode, desktop_noun());
            note(
                ui,
                &format!(
                    "An 'Open app' slice may open an app whose window is on another {}.",
                    desktop_noun()
                ),
            );
            for &mode in crate::platform::OS.desktop_modes() {
                choice_row(
                    ui,
                    &mut cfg.desktop_mode,
                    mode,
                    &desktop_mode_label(mode),
                    "",
                    accent,
                );
            }
        });
        panel(ui, "Side buttons", |ui| {
            demo::side_demo(
                ui,
                accent,
                rgb(cfg.inactive_color),
                cfg.side_default,
                cfg.right_click_switch,
                cfg.side_indicator,
            );
            note(
                ui,
                "In an app that has its own pie, the side buttons follow:",
            );
            choice_row(
                ui,
                &mut cfg.side_default,
                SideTarget::App,
                "The app's pie",
                "",
                accent,
            );
            choice_row(
                ui,
                &mut cfg.side_default,
                SideTarget::General,
                "The general pie",
                "",
                accent,
            );
            toggle_row(
                ui,
                &mut cfg.side_adopt,
                "Follow the pie I choose a mode in",
                "Choosing a side-button mode in a pie makes the side buttons follow that pie.",
                accent,
            );
            toggle_row(
                ui,
                &mut cfg.side_forget,
                "Forget that choice on another app",
                "When I switch to another app.",
                accent,
            );
            toggle_row(
                ui,
                &mut cfg.right_click_switch,
                "Right-click switches the pie",
                "While the pie is open, right-click switches which pie the side buttons follow.",
                accent,
            );
            toggle_row(
                ui,
                &mut cfg.side_indicator,
                "Color the pie ring",
                "Accent = the side buttons follow this pie; the other color = the other pie.",
                accent,
            );
            row_wide(ui, "Other-pie color", "", 40.0, |ui| {
                ui.color_edit_button_srgb(&mut cfg.inactive_color);
            });
            slider_row(
                ui,
                &format!(
                    "Show the pie after switching ({})",
                    crate::platform::OS.tray_name()
                ),
                &mut cfg.preview_ms,
                0..=2000,
                " ms",
                accent,
            );
        });
        panel(ui, "Startup", |ui| {
            toggle_row(
                ui,
                &mut cfg.launch_at_login,
                "Launch at login",
                crate::platform::OS.startup_hint(),
                accent,
            );
        });
        panel(ui, "Memory", |ui| {
            demo::memory_demo(ui, accent, cfg.sleep_after_s != 0, cfg.sleep_after_s);
            let mut on = cfg.sleep_after_s != 0;
            if toggle_row(
                ui,
                &mut on,
                "Free memory while idle",
                "After this long without a pie or Settings, cached icons and the editor's working copy are dropped. They are loaded again when you press the trigger.",
                accent,
            )
            .changed()
            {
                cfg.sleep_after_s = if on { 60 } else { 0 };
            }
            if cfg.sleep_after_s != 0 {
                slider_row(
                    ui,
                    "Idle time before freeing",
                    &mut cfg.sleep_after_s,
                    10..=600,
                    " s",
                    accent,
                );
            }
        });
    }
}

/// A strip of colors with four slices of the configured glass over it, so a change of the
/// sliders shows at once: how much of the colors comes through and how thick the frost is.
fn glass_preview(ui: &mut egui::Ui, cfg: &Config) {
    use crate::ui::draw::{glass_colors, rgb};
    use crate::ui::pie::glass;
    let Some(g) = glass::Glass::of(cfg) else {
        return;
    };
    ui.add_space(GAP);
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 120.0), egui::Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, CONTROL_R, rgb([64, 120, 214]));
    // what the glass sits on: a few blobs of color and a sharp diagonal, so blur and clarity show
    for (dx, dy, r, c) in [
        (0.12, 0.2, 52.0, [255, 149, 0]),
        (0.35, 0.9, 44.0, [255, 45, 85]),
        (0.62, 0.15, 56.0, [52, 199, 89]),
        (0.88, 0.8, 50.0, [175, 82, 222]),
    ] {
        painter.circle_filled(
            egui::pos2(
                rect.min.x + rect.width() * dx,
                rect.min.y + rect.height() * dy,
            ),
            r,
            rgb(c),
        );
    }
    painter.line_segment(
        [rect.left_bottom(), rect.right_top()],
        egui::Stroke::new(3.0, Color32::WHITE),
    );
    let center = rect.center();
    let (inner, outer) = (22.0, 54.0);
    let step = std::f32::consts::TAU / 4.0;
    for i in 0..4 {
        let a0 = i as f32 * step + 0.04 - std::f32::consts::FRAC_PI_4 * 2.0;
        glass::slice(
            &painter,
            center,
            (inner, outer),
            (a0, a0 + step - 0.08),
            glass_colors().slice,
            if i == 1 { 1.0 } else { 0.0 },
            1.0,
            g,
            false,
        );
    }
    glass::core(&painter, center, inner * 0.86, 1.0, None, g);
    ui.add_space(GAP);
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

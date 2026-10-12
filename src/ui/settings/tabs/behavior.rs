//! Behavior tab: trigger button, timings, size, accent color, toggles.

use crate::config::{AppGesture, SideTarget, TargetApp};
use crate::ui::app::App;
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use eframe::egui;

impl App {
    /// Draws the Behavior tab. Widgets edit `self.edit`, so nothing changes until Save.
    pub(in crate::ui::settings) fn behavior_tab(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Behavior & look");
        let cfg = &mut self.edit;
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::Grid::new("behavior_grid")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    muted(ui, "Trigger button");
                    let name = |b: i64| match b {
                        2 => "Middle button",
                        5 => "Extra button 6",
                        6 => "Extra button 7",
                        _ => "Extra button 8",
                    };
                    egui::ComboBox::from_id_salt("trigger_button")
                        .selected_text(name(cfg.trigger_button))
                        .show_ui(ui, |ui| {
                            for b in [2i64, 5, 6, 7] {
                                ui.selectable_value(&mut cfg.trigger_button, b, name(b));
                            }
                        });
                    ui.end_row();

                    muted(ui, "Hold to open pie");
                    ui.add(egui::Slider::new(&mut cfg.pie_hold_ms, 150..=1500).suffix(" ms"));
                    ui.end_row();

                    muted(ui, "Hold to open settings");
                    ui.add(egui::Slider::new(&mut cfg.settings_hold_ms, 500..=4000).suffix(" ms"));
                    ui.end_row();

                    muted(ui, "Pie size");
                    ui.add(egui::Slider::new(&mut cfg.pie_size, 140.0..=360.0).suffix(" px"));
                    ui.end_row();

                    muted(ui, "Accent color");
                    ui.color_edit_button_srgb(&mut cfg.accent);
                    ui.end_row();
                });

            ui.add_space(6.0);
            muted(ui, "Pie of the app in front");
            ui.radio_value(
                &mut cfg.app_gesture,
                AppGesture::DoubleClick,
                "Hold = general pie · Double-click = pie of the app (e.g. Photoshop)",
            );
            ui.radio_value(
                &mut cfg.app_gesture,
                AppGesture::Merged,
                "Classic: hold opens the app's own pie if it has one",
            );
            if cfg.app_gesture == AppGesture::DoubleClick {
                ui.horizontal(|ui| {
                    muted(ui, "Double-click speed");
                    ui.add(egui::Slider::new(&mut cfg.double_click_ms, 150..=500).suffix(" ms"));
                });
            }

            ui.add_space(4.0);
            ui.checkbox(
                &mut cfg.flick,
                "Open the pie instantly when I drag (disables middle-drag in apps)",
            );
            ui.checkbox(
                &mut cfg.sound,
                "Play a soft tick when the selection changes",
            );
            ui.checkbox(
                &mut cfg.center_app_icon,
                "Show the app's icon (white) in the center of an app's own pie",
            );
            ui.checkbox(&mut cfg.launch_at_login, "Launch at login");
        });

        ui.add_space(14.0);
        section_title(ui, "App detection");
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            muted(ui, "Which app picks the pie");
            ui.radio_value(
                &mut cfg.target_app,
                TargetApp::UnderCursor,
                "The app under the pointer: no need to click its window first",
            );
            ui.radio_value(
                &mut cfg.target_app,
                TargetApp::Focused,
                "The app in front (the one with keyboard focus)",
            );
            ui.add_enabled(
                cfg.target_app == TargetApp::UnderCursor,
                egui::Checkbox::new(
                    &mut cfg.focus_target,
                    "Bring that app to the front before a slice or side button sends it a shortcut",
                ),
            );
        });

        ui.add_space(14.0);
        section_title(ui, "Side buttons");
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            muted(ui, "In an app that has its own pie, the side buttons follow");
            ui.radio_value(&mut cfg.side_default, SideTarget::App, "The app's pie");
            ui.radio_value(&mut cfg.side_default, SideTarget::General, "The general pie");

            ui.add_space(4.0);
            ui.checkbox(
                &mut cfg.side_adopt,
                "Choosing a side-button mode in a pie makes the side buttons follow that pie",
            );
            ui.checkbox(
                &mut cfg.side_forget,
                "Forget that choice when I switch to another app",
            );
            ui.checkbox(
                &mut cfg.right_click_switch,
                "While the pie is open, right-click switches which pie the side buttons follow",
            );
            ui.checkbox(
                &mut cfg.side_indicator,
                "Color the pie ring: accent = side buttons follow this pie, other color = the other pie",
            );

            ui.add_space(4.0);
            egui::Grid::new("side_grid")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    muted(ui, "Other-pie color");
                    ui.color_edit_button_srgb(&mut cfg.inactive_color);
                    ui.end_row();

                    muted(ui, "Show the pie after switching (menu bar)");
                    ui.add(egui::Slider::new(&mut cfg.preview_ms, 0..=2000).suffix(" ms"));
                    ui.end_row();
                });
        });
    }
}

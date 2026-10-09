//! Behavior tab: trigger button, timings, size, accent color, toggles.

use crate::app::App;
use crate::gfx::*;
use eframe::egui;

impl App {
    pub(in crate::settings) fn behavior_tab(&mut self, ui: &mut egui::Ui) {
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
            ui.add_space(4.0);
            ui.checkbox(
                &mut cfg.flick,
                "Open the pie instantly when I drag (disables middle-drag in apps)",
            );
            ui.checkbox(
                &mut cfg.sound,
                "Play a soft tick when the selection changes",
            );
            ui.checkbox(&mut cfg.launch_at_login, "Launch at login");
        });
    }
}

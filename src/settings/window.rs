//! The settings window itself: open/close animation, card chrome, tab bar,
//! footer buttons (Save / Cancel / Quit).

use super::state::Tab;
use super::tabs::about::about_tab;
use crate::about::*;
use crate::app::App;
use crate::gfx::*;
use crate::system;
use crate::tap::TAP_ACTIVE;
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};
use std::sync::atomic::Ordering;
use std::time::Duration;

enum UiAction {
    None,
    Save,
    Cancel,
    Quit,
}

impl App {
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();

        // open / close animation
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        let target = if self.closing.is_some() { 0.0 } else { 1.0 };
        if smooth_approach(&mut self.settings_anim, target, dt, 14.0) {
            ctx.request_repaint();
        }
        if let Some(save) = self.closing
            && self.settings_anim <= 0.02
        {
            self.close_settings(&ctx, save);
            return;
        }
        // keep permission status fresh while the window is open
        ctx.request_repaint_after(Duration::from_millis(1000));

        let t = self.settings_anim.clamp(0.0, 1.0);
        let slide = (1.0 - t) * (1.0 - t) * 26.0;

        // Esc (when not recording) = cancel
        if self.closing.is_none()
            && self.ed.recording.is_none()
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.closing = Some(false);
        }
        if self
            .toast
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(6))
        {
            self.toast = None;
        }

        let accent = rgb(self.edit.accent);
        style_settings(ui, accent);
        ui.set_opacity(t);

        // card + shadow, always centered in the window
        let full = ui.max_rect();
        let card_rect = egui::Rect::from_center_size(full.center(), self.card_size)
            .translate(Vec2::new(0.0, slide));
        for k in 1..=6 {
            let g = k as f32 * 3.0;
            let alpha = (24.0 - k as f32 * 3.6).max(2.0) as u8;
            ui.painter().rect_filled(
                card_rect.expand(g).translate(Vec2::new(0.0, 6.0)),
                CornerRadius::same((16.0 + g) as u8),
                Color32::from_black_alpha(alpha),
            );
        }
        ui.painter()
            .rect_filled(card_rect, CornerRadius::same(16), BG);
        ui.painter().rect_stroke(
            card_rect,
            CornerRadius::same(16),
            Stroke::new(1.0, BORDER),
            egui::StrokeKind::Inside,
        );

        let inner = card_rect.shrink(18.0);

        // dragging the title area moves the window
        let title_rect = egui::Rect::from_min_size(inner.min, Vec2::new(inner.width(), 56.0));
        let drag = ui.interact(
            title_rect,
            egui::Id::new("settings_drag"),
            egui::Sense::click_and_drag(),
        );
        if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let mut action = UiAction::None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
            ui.label(RichText::new(APP_NAME).size(26.0).strong().color(TEXT));
            muted(ui, "Hold the middle button for your main pie · double-click it for the pie of the app in front.");

            if !system::is_trusted() || !TAP_ACTIVE.load(Ordering::SeqCst) {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Accessibility permission needed.").color(BAD));
                    if ui.small_button("Open System Settings").clicked() {
                        system::open_accessibility_settings();
                    }
                });
            }

            ui.add_space(6.0);
            ui.separator();

            let tabs = [(Tab::Menu, "Menu"), (Tab::Behavior, "Behavior"), (Tab::General, "General"), (Tab::About, "About")];
            let mut idx = tabs.iter().position(|(t, _)| *t == self.ed.tab).unwrap_or(0);
            if segmented(ui, &mut idx, &tabs.map(|t| t.1), accent) { self.ed.tab = tabs[idx].0; }

            let h = (ui.available_height() - 66.0).max(120.0);
            egui::ScrollArea::vertical().max_height(h).auto_shrink([false, false]).show(ui, |ui| match self.ed.tab {
                Tab::Menu => self.menu_tab(ui, &ctx),
                Tab::Behavior => self.behavior_tab(ui),
                Tab::General => self.general_tab(ui),
                Tab::About => about_tab(ui),
            });

            if let Some((msg, _)) = &self.toast {
                ui.label(RichText::new(msg).color(accent));
            }
            ui.separator();
            ui.horizontal(|ui| {
                let save = egui::Button::new(RichText::new("Save").strong().color(Color32::WHITE))
                    .fill(accent)
                    .min_size(Vec2::new(100.0, 32.0));
                if ui.add(save).clicked() {
                    action = UiAction::Save;
                }
                if ui.add(egui::Button::new("Cancel").min_size(Vec2::new(80.0, 32.0))).clicked() {
                    action = UiAction::Cancel;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Quit app").clicked() {
                        action = UiAction::Quit;
                    }
                });
            });
        });

        match action {
            UiAction::Save => {
                self.ed.stop_recording();
                self.closing = Some(true);
            }
            UiAction::Cancel => {
                self.ed.stop_recording();
                self.closing = Some(false);
            }
            UiAction::Quit => {
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                std::thread::spawn(|| {
                    std::thread::sleep(Duration::from_millis(1500));
                    std::process::exit(0);
                });
            }
            UiAction::None => {}
        }
    }
}

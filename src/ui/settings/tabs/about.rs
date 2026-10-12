//! About tab: logo and version, project links, author, privacy note, credits.
//!
//! A hero panel on top (logo, name, version, link buttons), then two columns of panels.

use crate::about::*;
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use eframe::egui::{self, Color32, RichText, Vec2};

/// One "label  link" row.
fn link_row(ui: &mut egui::Ui, label: &str, text: &str, url: &str) {
    info_row(ui, |ui| {
        muted(ui, label);
        ui.hyperlink_to(text, url);
    });
}

/// A small boxed label such as the version.
fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(INPUT)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(RADIUS)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).monospace().size(12.0).color(color));
        });
}

/// A button that opens `url` in the browser.
fn link_button(ui: &mut egui::Ui, text: &str, url: &str) {
    let b = egui::Button::new(text).min_size(Vec2::new(0.0, CONTROL_H));
    if ui.add(b).on_hover_text(url).clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
    }
}

/// Draws the About tab from the constants in [`crate::about`]. `accent` colors the logo.
pub(in crate::ui::settings) fn about_tab(ui: &mut egui::Ui, accent: Color32) {
    // ------------------------------------------------------------ hero
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(88.0), egui::Sense::hover());
            draw_logo(ui.painter(), rect.center(), 42.0, accent);
            ui.add_space(GAP);
            ui.vertical(|ui| {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(APP_NAME)
                        .monospace()
                        .size(28.0)
                        .strong()
                        .color(TEXT),
                );
                ui.horizontal(|ui| {
                    badge(ui, &format!("v{VERSION}"), accent);
                    badge(ui, LICENSE_NAME, MUTED);
                });
                muted(ui, TAGLINE);
            });
        });
        ui.add_space(GAP);
        ui.horizontal_wrapped(|ui| {
            link_button(ui, "GitHub repository", REPO_URL);
            link_button(ui, "Report an issue", ISSUES_URL);
            link_button(ui, "Downloads", RELEASES_URL);
        });
    });
    ui.add_space(GAP);

    // ------------------------------------------------------------ two columns
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        panel(ui, "Created by", |ui| {
            ui.label(RichText::new(AUTHOR_NAME).strong().color(TEXT));
            link_row(
                ui,
                "GitHub",
                &format!("@{AUTHOR_GITHUB_USER}"),
                AUTHOR_GITHUB_URL,
            );
            link_row(ui, "Email", AUTHOR_EMAIL, &format!("mailto:{AUTHOR_EMAIL}"));
        });
        panel(ui, "Project", |ui| {
            link_row(ui, "Inspired by", INSPIRED_BY.0, INSPIRED_BY.1);
            muted(ui, LICENSE_NAME);
            muted(ui, COPYRIGHT);
        });

        let ui = &mut cols[1];
        panel(ui, "Privacy", |ui| {
            ui.label(RichText::new(PRIVACY).color(MUTED));
        });
        panel(ui, "Built with", |ui| {
            for (name, url, license) in CREDITS {
                info_row(ui, |ui| {
                    ui.hyperlink_to(*name, *url);
                    muted(ui, license);
                });
            }
        });
    });
}

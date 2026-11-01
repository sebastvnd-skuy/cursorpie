//! About tab: logo and version, project links, author, privacy note, credits.
//!
//! A hero panel on top (logo, name, version, link buttons), then one column of panels.

use crate::about::*;
use crate::ui::fonts;
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use crate::ui::symbols;
use eframe::egui::{self, Color32, Vec2};

/// One "label  link" row.
fn link_row(ui: &mut egui::Ui, label: &str, text: &str, url: &str) {
    info_row(ui, label, |ui| {
        ui.hyperlink_to(text, url);
    });
}

/// A button that opens `url` in the browser.
fn link_button(ui: &mut egui::Ui, icon: char, text: &str, url: &str) {
    if button(ui, Look::Soft, Some(icon), text)
        .on_hover_text(url)
        .clicked()
    {
        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
    }
}

/// Draws the About tab from the constants in [`crate::about`]. `accent` colors the logo.
pub(in crate::ui::settings) fn about_tab(ui: &mut egui::Ui, accent: Color32) {
    // ------------------------------------------------------------ hero
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.add_space(GAP);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(88.0), egui::Sense::hover());
            draw_logo(ui.painter(), rect.center(), 42.0, accent);
            ui.add_space(GAP);
            ui.vertical(|ui| {
                ui.add_space(6.0);
                ui.label(fonts::title(APP_NAME, 30.0, pal().text));
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    badge(ui, &format!("Version {VERSION}"), accent);
                    badge(ui, LICENSE_NAME, pal().muted);
                });
                ui.add_space(2.0);
                muted(ui, TAGLINE);
            });
        });
        ui.add_space(GAP);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(8.0);
            link_button(ui, symbols::GITHUB_LOGO, "GitHub repository", REPO_URL);
            link_button(ui, symbols::BUG, "Report an issue", ISSUES_URL);
            link_button(ui, symbols::DOWNLOAD_SIMPLE, "Downloads", RELEASES_URL);
        });
        ui.add_space(GAP);
    });
    ui.add_space(SECTION_GAP);

    panel(ui, "Created by", |ui| {
        info_row(ui, AUTHOR_NAME, |_| {});
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
        info_row(ui, "License", |ui| muted(ui, LICENSE_NAME));
        info_row(ui, "Copyright", |ui| muted(ui, COPYRIGHT));
    });
    panel(ui, "Privacy", |ui| {
        note(ui, PRIVACY);
    });
    panel(ui, "Built with", |ui| {
        for (name, url, license) in CREDITS {
            row(ui, name, license, |ui| {
                if icon_button(ui, symbols::ARROW_SQUARE_OUT, url, true).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(*url));
                }
            });
        }
    });
}

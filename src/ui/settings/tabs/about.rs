//! About tab: author, project links, privacy note, credits.

use crate::about::*;
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use eframe::egui::{self, RichText};

/// One "label  link" row.
fn link_row(ui: &mut egui::Ui, label: &str, text: &str, url: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(MUTED));
        ui.hyperlink_to(text, url);
    });
}

/// Draws the About tab from the constants in [`crate::about`].
pub(in crate::ui::settings) fn about_tab(ui: &mut egui::Ui) {
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new(format!("{APP_NAME}  v{VERSION}"))
                .size(20.0)
                .strong()
                .color(TEXT),
        );
        muted(ui, TAGLINE);
    });
    ui.add_space(10.0);

    section_title(ui, "Created by");
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(AUTHOR_NAME).size(16.0).strong());
        link_row(
            ui,
            "GitHub",
            &format!("@{AUTHOR_GITHUB_USER}"),
            AUTHOR_GITHUB_URL,
        );
        link_row(ui, "Email", AUTHOR_EMAIL, &format!("mailto:{AUTHOR_EMAIL}"));
    });
    ui.add_space(10.0);

    section_title(ui, "Project");
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        link_row(ui, "Source code", "GitHub repository", REPO_URL);
        link_row(ui, "Bugs / ideas", "Open an issue", ISSUES_URL);
        link_row(ui, "Downloads", "Releases", RELEASES_URL);
        link_row(ui, "Inspired by", INSPIRED_BY.0, INSPIRED_BY.1);
        ui.add_space(2.0);
        muted(ui, LICENSE_NAME);
        muted(ui, COPYRIGHT);
    });
    ui.add_space(10.0);

    section_title(ui, "Privacy");
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(PRIVACY).color(MUTED));
    });
    ui.add_space(10.0);

    section_title(ui, "Built with");
    card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        for (name, url, license) in CREDITS {
            ui.horizontal(|ui| {
                ui.hyperlink_to(*name, *url);
                muted(ui, license);
            });
        }
    });
}

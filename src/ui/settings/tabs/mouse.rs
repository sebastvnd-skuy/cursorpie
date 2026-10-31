//! The "Mouse" panel of the General tab: which mouse is in use, how many extra buttons it has, a scan
//! to find them by pressing them, and the mice CursorPie knows.
//!
//! The mice are device state (`mice.json`), not part of the config: what happens here is saved at
//! once, and Save, Undo and backups do not touch it.

use crate::mouse::{self, MouseProfile, Recognition};
use crate::ui::app::App;
use crate::ui::settings::theme::*;
use crate::ui::settings::widgets::*;
use eframe::egui::{self, RichText};
use std::time::Duration;

/// The names of the buttons, "Back button, Extra button 6", or a dash when there are none.
fn names_of(buttons: &[i64]) -> String {
    if buttons.is_empty() {
        return "none".into();
    }
    buttons
        .iter()
        .map(|b| mouse::button_label(*b))
        .collect::<Vec<_>>()
        .join(", ")
}

/// How long ago, for the list of mice.
fn ago(seconds: u64) -> String {
    match seconds {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", seconds / 60),
        3600..=86_399 => format!("{} h ago", seconds / 3600),
        _ => format!("{} days ago", seconds / 86_400),
    }
}

/// What the person asked for in this frame; applied after the panel is drawn.
#[derive(Default)]
struct Wanted {
    detect: bool,
    start_scan: bool,
    done: bool,
    cancel: bool,
    forget: Option<String>,
}

impl App {
    /// Draws the Mouse panel.
    pub(in crate::ui::settings) fn mouse_panel(&mut self, ui: &mut egui::Ui) {
        // a scan that nobody finishes ends by itself, so no button stays swallowed
        if self.ed.scan.as_ref().is_some_and(mouse::Scan::idle) {
            self.ed.scan = None;
            self.say("Scan stopped: no button was pressed for a while.");
        }
        let book = mouse::snapshot();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let mut want = Wanted::default();
        let scanning = self.ed.scan.is_some();
        let pressed = self.ed.scan.as_ref().map(mouse::Scan::pressed);

        panel(ui, "Mouse", |ui| {
            match book.active_profile() {
                Some(p) => {
                    info_row(ui, |ui| {
                        muted(ui, "In use");
                        ui.label(RichText::new(&p.name).color(TEXT));
                    });
                    info_row(ui, |ui| {
                        muted(ui, "Extra buttons");
                        ui.label(RichText::new(p.buttons().len().to_string()).color(TEXT));
                        if p.is_scanned() {
                            ui.label(RichText::new("scanned").color(GOOD));
                        } else {
                            ui.label(RichText::new("reported by the system").color(MUTED));
                        }
                    });
                    muted(ui, &names_of(&p.buttons()));
                }
                None => muted(
                    ui,
                    "No mouse found yet. Scan it to tell CursorPie which buttons it has.",
                ),
            }

            if let Some(list) = &pressed {
                ui.add_space(4.0);
                ui.label(RichText::new("Press every extra button of your mouse once.").color(TEXT));
                muted(
                    ui,
                    &format!(
                        "Heard so far: {}. The buttons do nothing else while the scan runs.",
                        names_of(list)
                    ),
                );
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!list.is_empty(), egui::Button::new("Done"))
                        .clicked()
                    {
                        want.done = true;
                    }
                    if ui.button("Cancel").clicked() {
                        want.cancel = true;
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    if ui.button("Scan this mouse").clicked() {
                        want.start_scan = true;
                    }
                    if ui.button("Detect again").clicked() {
                        want.detect = true;
                    }
                });
                muted(
                    ui,
                    "Detect again asks the system which mice are connected. Scan finds the buttons by pressing them: use it when the list is wrong or empty.",
                );
            }

            if book.mice.len() > 1 || (book.mice.len() == 1 && book.active_profile().is_none()) {
                ui.add_space(4.0);
                muted(ui, "Mice CursorPie knows");
                for p in &book.mice {
                    mouse_row(ui, p, p.id == book.active, now, &mut want);
                }
            }
            muted(
                ui,
                "Saved at once, not with Save. Slices set to 'Side buttons' can then use every button listed here.",
            );
        });

        if scanning {
            // the presses arrive as messages; look again soon so the list and the timeout stay current
            ui.ctx().request_repaint_after(Duration::from_millis(400));
        }
        if want.start_scan {
            self.ed.scan = Some(mouse::Scan::start());
        }
        if want.cancel {
            self.ed.scan = None;
        }
        if want.done
            && let Some(list) = pressed
        {
            self.ed.scan = None;
            let n = mouse::finish_scan(&list);
            self.say(format!("Saved: this mouse has {n} extra buttons."));
        }
        if let Some(id) = want.forget {
            mouse::forget(&id);
            self.say("Mouse forgotten. It is recognized as new when it is connected again.");
        }
        if want.detect {
            let out = mouse::refresh();
            self.say(match out.recognition {
                Recognition::Absent => {
                    "No mouse was listed by the system. Use Scan to find the buttons.".to_string()
                }
                _ => format!("{}: {} extra buttons.", out.name, out.extra),
            });
        }
    }
}

/// One line of the list of known mice.
fn mouse_row(ui: &mut egui::Ui, p: &MouseProfile, in_use: bool, now: u64, want: &mut Wanted) {
    info_row(ui, |ui| {
        let color = if in_use { TEXT } else { MUTED };
        ui.label(RichText::new(&p.name).color(color));
        ui.label(
            RichText::new(format!(
                "{} extra · {}",
                p.buttons().len(),
                ago(now.saturating_sub(p.last_seen))
            ))
            .color(MUTED),
        );
        if ui.small_button("Forget").clicked() {
            want.forget = Some(p.id.clone());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_are_listed_by_name() {
        assert_eq!(names_of(&[]), "none");
        assert_eq!(
            names_of(&[3, 4, 5]),
            "Back button, Forward button, Extra button 6"
        );
    }

    #[test]
    fn ages_are_short() {
        assert_eq!(ago(5), "just now");
        assert_eq!(ago(120), "2 min ago");
        assert_eq!(ago(7200), "2 h ago");
        assert_eq!(ago(3 * 86_400), "3 days ago");
    }
}

//! Panel pemilih ikon: katalog bawaan (icons.rs) + gambar dari folder ikon + teks bebas.
//!
//! Semua lebar dihitung dari `w` (lebar panel yang diberikan pemanggil), bukan dari
//! `available_width()`, dan grid disusun manual per baris. Dengan begitu tidak ada yang bisa
//! meluber keluar kartu walaupun egui menghitung sisa lebar dengan salah.

use super::state::Ed;
use super::widgets::{chip, tile};
use crate::gfx::*;
use crate::icons::{self, CATEGORIES, IconDef};
use eframe::egui::{self, Align2, Color32, FontId, Vec2};

pub(super) struct Picked {
    pub chosen: Option<String>,
    pub pick_file: bool,
    pub back: bool,
}

enum Cell {
    Glyph(&'static IconDef),
    Mine(String),
}

pub(super) fn icon_picker(
    ui: &mut egui::Ui,
    ed: &mut Ed,
    ctx: &egui::Context,
    icon: &mut String,
    accent: Color32,
    w: f32,
) -> Picked {
    let mut out = Picked {
        chosen: None,
        pick_file: false,
        back: false,
    };
    let mine = CATEGORIES.len() + 1;

    // baris 1: kembali + cari
    ui.horizontal(|ui| {
        if ui.button("‹ Back").clicked() {
            out.back = true;
        }
        ui.add(
            egui::TextEdit::singleline(&mut ed.icon_query)
                .desired_width((w - 84.0).max(80.0))
                .hint_text("🔍 Search"),
        );
    });

    // baris 2: kategori, satu baris. Yang aktif menampilkan nama, sisanya hanya ikon.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if chip(ui, "", "All", ed.icon_cat == 0, accent).clicked() {
            ed.icon_cat = 0;
        }
        for (k, c) in CATEGORIES.iter().enumerate() {
            let on = ed.icon_cat == k + 1;
            if chip(ui, c.glyph, if on { c.title } else { "" }, on, accent)
                .on_hover_text(c.title)
                .clicked()
            {
                ed.icon_cat = k + 1;
            }
        }
        let on = ed.icon_cat == mine;
        if chip(ui, "🖼", if on { "Mine" } else { "" }, on, accent)
            .on_hover_text("My icons (icons folder)")
            .clicked()
        {
            ed.icon_cat = mine;
            ed.user_icons = icons::user_icons();
        }
    });

    // baris 3: gambar / folder / muat ulang / teks bebas
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if ui
            .button("🖼 Image…")
            .on_hover_text("Choose an image file")
            .clicked()
        {
            out.pick_file = true;
        }
        if ui
            .button("📂")
            .on_hover_text("Open the icons folder. Drop PNG / JPG / WebP files in it.")
            .clicked()
        {
            icons::open_icons_folder();
        }
        if ui
            .button("🔄")
            .on_hover_text("Reload the icons folder")
            .clicked()
        {
            ed.user_icons = icons::user_icons();
        }
        ui.add(
            egui::TextEdit::singleline(icon)
                .desired_width((w - 214.0).max(60.0))
                .hint_text("or type text / emoji"),
        );
    });

    // isi grid
    let q = ed.icon_query.trim().to_lowercase();
    let mut cells: Vec<Cell> = Vec::new();
    if ed.icon_cat != mine {
        for d in icons::filtered(&q, ed.icon_cat) {
            cells.push(Cell::Glyph(d));
        }
    }
    if ed.icon_cat == 0 || ed.icon_cat == mine {
        for p in &ed.user_icons {
            let file = p.rsplit('/').next().unwrap_or(p);
            if q.is_empty() || file.to_lowercase().contains(&q) {
                cells.push(Cell::Mine(p.clone()));
            }
        }
    }

    let (tile_w, gap) = (42.0_f32, 6.0_f32);
    let cols = (((w - 10.0) + gap) / (tile_w + gap)).floor().max(1.0) as usize;
    let h = ui.available_height().max(80.0);
    egui::ScrollArea::vertical()
        .id_salt("icon_grid")
        .max_height(h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_max_width(w);
            ui.spacing_mut().item_spacing = Vec2::splat(gap);
            if cells.is_empty() {
                muted(
                    ui,
                    if ed.icon_cat == mine {
                        "Drop images into the icons folder (📂), then press 🔄."
                    } else {
                        "No icons found."
                    },
                );
            }
            for row in cells.chunks(cols) {
                ui.horizontal(|ui| {
                    for cell in row {
                        match cell {
                            Cell::Glyph(def) => {
                                let (rect, resp) =
                                    tile(ui, Vec2::splat(tile_w), icon.as_str() == def.glyph, accent);
                                let fs = if def.glyph.chars().count() > 2 { 12.0 } else { 22.0 };
                                ui.painter().text(
                                    rect.center(),
                                    Align2::CENTER_CENTER,
                                    def.glyph,
                                    FontId::proportional(fs),
                                    Color32::from_white_alpha(235),
                                );
                                if resp.on_hover_text(def.name).clicked() {
                                    out.chosen = Some(def.glyph.to_string());
                                }
                            }
                            Cell::Mine(path) => {
                                let file = path.rsplit('/').next().unwrap_or(path);
                                let tex = ed.icon_for(ctx, path);
                                let (rect, resp) =
                                    tile(ui, Vec2::splat(tile_w), *icon == *path, accent);
                                draw_icon(ui.painter(), tex.as_ref(), path, rect.center(), 30.0, 255);
                                if resp.on_hover_text(file).clicked() {
                                    out.chosen = Some(path.clone());
                                }
                            }
                        }
                    }
                });
            }
        });

    out
}

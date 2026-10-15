//! The interactive pie in Settings: drawing, hit-testing and clicks.
use super::state::Ed;
use crate::config::*;
use crate::geometry::Point;
use crate::geometry::*;
use crate::ui::draw::*;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Sense, Vec2};
use std::f32::consts::TAU;

/// What the user did with the designer this frame.
pub(super) enum Designer {
    None,
    Enter(usize),
    Up,
    AddClicked,
}
/// What the pointer is over.
enum Hit {
    Out,
    Center,
    Slice(usize),
}

/// Draws the pie of `items` at `size` px; click a slice to select it, the "+" slice to add one.
pub(super) fn pie_designer(
    ui: &mut egui::Ui,
    ed: &mut Ed,
    ctx: &egui::Context,
    items: &[Item],
    accent: [u8; 3],
    size: f32,
) -> Designer {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let painter = ui.painter_at(rect);
    let c = rect.center();
    let (outer, inner) = (pie_outer(size), pie_inner(size));

    let n = items.len();
    let total = n + (n < MAX_ITEMS) as usize; // the "+" slice at the end

    let hit = |p: Pos2| -> Hit {
        let d = (p - c).length();
        if d > outer + 6.0 {
            Hit::Out
        } else if d < inner {
            Hit::Center
        } else {
            let i = slice_at(
                Point::new(c.x as f64, c.y as f64),
                Point::new(p.x as f64, p.y as f64),
                total,
                inner,
            );
            i.map(Hit::Slice).unwrap_or(Hit::Center)
        }
    };
    let hov = resp.hover_pos().map(hit).unwrap_or(Hit::Out);
    if !matches!(hov, Hit::Out) {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    // ---- draw ----
    let sl = TAU / total.max(1) as f32;
    for i in 0..total {
        let item = items.get(i); // None for the trailing "+" slice
        let ghost = i == n;
        let selected = ed.sel == Some(i);
        let hovered = matches!(hov, Hit::Slice(h) if h == i);
        let base = if ghost { [30, 32, 40] } else { [27, 29, 35] };
        let t = if selected {
            0.9
        } else if hovered {
            0.45
        } else {
            0.0
        };
        let a0 = slice_start(i, total);
        painter.add(ring_slice(
            &painter,
            c,
            inner,
            outer,
            a0 + 0.03,
            a0 + sl - 0.03,
            rgb(mix(base, accent, t)),
        ));

        let am = a0 + sl / 2.0;
        let tp = c + Vec2::new(am.cos(), am.sin()) * ((inner + outer) / 2.0);
        if ghost {
            painter.text(
                tp,
                Align2::CENTER_CENTER,
                "+",
                FontId::proportional(30.0),
                Color32::from_white_alpha(130),
            );
        } else if let Some(it) = item {
            let tex = ed.icon_for(ctx, &it.icon);
            draw_icon(
                &painter,
                tex.as_ref(),
                &it.icon,
                tp - Vec2::new(0.0, 8.0),
                32.0,
                255,
            );
            painter.text(
                tp + Vec2::new(0.0, 18.0),
                Align2::CENTER_CENTER,
                &it.name,
                FontId::proportional(11.0),
                Color32::from_white_alpha(190),
            );
            if matches!(it.kind, ItemKind::Submenu(_)) {
                painter.text(
                    tp + Vec2::new(0.0, 31.0),
                    Align2::CENTER_CENTER,
                    "›",
                    FontId::proportional(14.0),
                    rgb(accent),
                );
            }
        }
    }
    // center: "Back" while inside a submenu
    let core = inner * 0.86;
    let center_hot = matches!(hov, Hit::Center) && !ed.nav.is_empty();
    painter.circle_filled(
        c,
        core,
        if center_hot {
            Color32::from_rgb(40, 44, 54)
        } else {
            Color32::from_rgb(14, 15, 18)
        },
    );
    let label = if ed.nav.is_empty() { "Pie" } else { "‹ Back" };
    painter.text(
        c,
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.0),
        Color32::from_white_alpha(170),
    );

    // ---- clicks ----
    if resp.clicked()
        && let Some(p) = resp.interact_pointer_pos()
    {
        match hit(p) {
            Hit::Center if !ed.nav.is_empty() => return Designer::Up,
            Hit::Slice(i) if i == n && n < MAX_ITEMS => return Designer::AddClicked,
            Hit::Slice(i) if i < n => {
                let is_sub = matches!(items[i].kind, ItemKind::Submenu(_));
                if ed.sel == Some(i) && is_sub {
                    return Designer::Enter(i); // click again = step in
                }
                ed.sel = Some(i);
                ed.adding = false;
            }
            _ => {}
        }
    }
    Designer::None
}

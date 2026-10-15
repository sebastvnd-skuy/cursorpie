# UI design and rendering

This document explains how CursorPie looks, why it is drawn the way it is, and how to check a
change to either. The code is in `src/ui/`; the rules for contributors are summarized in
`CONTRIBUTING.md` and `guides::extending`.

## Contents

- [Design goals](#design-goals)
- [The Settings window](#the-settings-window)
- [The pie](#the-pie)
- [Why it used to look jagged and choppy](#why-it-used-to-look-jagged-and-choppy)
- [Rendering rules](#rendering-rules)
- [Testing](#testing)
- [Manual visual checklist](#manual-visual-checklist)

## Design goals

CursorPie is a tool you use many times a day for a second at a time, and a Settings window you open
rarely. The design follows from that:

- **Quiet and dense.** Neutral graphite surfaces, hairline borders instead of shadows, small type for
  chrome. Nothing competes with the pie or with the thing you are editing.
- **One accent.** The user picks the accent color. It marks focus, the primary button and the selected
  slice, and nothing else is saturated, so it always means "this".
- **Predictable.** The buttons you confirm with never move: Undo, Redo, Cancel and Save are in the top
  bar on every tab, Quit is in the footer.
- **Sharp and smooth.** Edges are anti-aliased and aligned to pixels, and animations take a fixed time.

## The Settings window

```text
+----------------------------------------------------------------------+
| logo CURSORPIE ⓘ | MENU BEHAVIOR GENERAL ABOUT |  UNDO REDO CANCEL SAVE |   top bar, 52 pt
+----------------------------------------------------------------------+
|                                                                      |
|   body of the selected tab (padding 16 pt)                           |
|                                                                      |
+----------------------------------------------------------------------+
| status or hint                                            QUIT APP   |   footer, 34 pt
+----------------------------------------------------------------------+
```

- The window is 1000 x 680 pt (`CARD_SIZE`) with a 12 pt corner radius. It is never taller than the
  screen allows and never shorter than 420 pt, where the body is still 302 pt high.
- `regions()` in `window.rs` cuts the window into the three bands. It is a pure function with tests.
- The selected tab is a solid light block with dark text, the other tabs are plain muted text. Of
  everything in the bar, "where am I" is the loudest.
- Panel titles, tabs and badges are small monospace capitals (`card_title`, `tab_item`, `badge`).
  Everything you read or edit is the normal font in sentence case.
- Controls are almost square (2 pt radius), 32 pt high, and 8 pt apart. Only the window itself is
  rounded. All colors, radii and spacing are constants in `theme.rs`.

### Palette

| Name | Use | `TEXT` on it | `MUTED` on it |
|---|---|---|---|
| `BG` | window | 15.7:1 | 6.7:1 |
| `CARD` | top bar, panels | 14.9:1 | 6.4:1 |
| `IDLE` | controls at rest | 13.7:1 | 5.9:1 |
| `HOT` | controls under the pointer | 11.8:1 | 5.1:1 |
| `INPUT` | text fields, footer | 16.2:1 | 7.0:1 |
| `BORDER` | hairlines (1.4:1 against `BG`, so they read as lines, not text) | - | - |

`GOOD` and `BAD` reach at least 7:1 on `CARD` and 8:1 on `INPUT`.

The tests in `theme.rs` enforce 7:1 for `TEXT` and 4.5:1 for `MUTED`, `GOOD` and `BAD`, and check that
`on_accent` stays readable on any accent the user can choose.

## The pie

The pie is a ring of dark "glass" slices with a one-pixel hairline each. The hovered slice fills with
the accent, gets a brighter hairline and a bright bar that grows along its rim. A soft shadow sits
behind the ring, built from a few stacked translucent discs. The center disc shows the active mode or
the app icon, and the "hold for Settings" progress arc draws around it.

Timing: 0.22 s to open (slices appear one after another, with a small overshoot), 0.12 s to close.

## Why it used to look jagged and choppy

| Symptom | Cause | Fix |
|---|---|---|
| Stair-stepped edges on the slices and on the logo | The slices were a hand-made `egui::Mesh`. egui anti-aliases the shapes it tessellates itself, but draws a raw mesh as is | `ring_slice_mesh` adds a transparent fringe around every edge, so the GPU fades the edge over one physical pixel |
| Jagged diagonal window border | The focus border was four mesh strips with a gradient, again unfiltered, and it could not follow rounded corners | `window_outline`: an ordinary rounded `rect_stroke` |
| Soft hairlines and text in Settings | The card was centered at fractional coordinates, so one-pixel lines fell between two pixels | `snap_rect` puts the window on whole pixels; the pie window is positioned on a whole point |
| Dark fringe around icons | Icons were scaled down with their transparent pixels (often black) mixed in | `prepare_icon` premultiplies before a Lanczos resize and clamps afterwards |
| Pie opening felt slow at the end, or stuttered with frame rate | `smooth_approach` (exponential) was fed through `ease_out_back`, so the curve was eased twice: long invisible tail, flattened overshoot | `step_linear` gives a fixed duration; one easing curve is applied on top |
| Ring cut off at the window edge while overshooting | The overshoot could grow the ring past the window | the radius is clamped to `pie_outer + OVERSHOOT_ROOM` |
| Visible facets on large rings | A fixed 0.06 to 0.07 rad step regardless of radius | `arc_segments` picks the step from a 0.02 pt error budget |

## Rendering rules

1. **Do not push a raw `egui::Mesh` for anything with an edge.** Use `ui::draw::ring_slice` or an egui
   shape (`circle_filled`, `rect_filled`, `Shape::line`, `Shape::closed_line`).
2. **Feather by one physical pixel**, that is `1 / pixels_per_point`, never a fixed number of points.
3. **Put static hairlines on the pixel grid** (`snap_rect`, `snap_to_pixel`). Do not snap things that
   animate: a snapped size that changes every frame jitters.
4. **Animate with `step_linear` plus one easing curve.** Use `smooth_approach` only for values that chase
   a moving target (hover highlights), where "never quite arrives" is wanted.
5. **Pick curve resolution from the error you accept** (`arc_segments`), not from a constant.
6. **Keep icons premultiplied** from load to draw; never upload a texture with a color channel above its
   alpha.
7. **Keep colors, radii and sizes in `theme.rs`.**

## Testing

Everything that does not need a window is unit tested, and runs on every OS through the stub backend:

```sh
cargo test ui::draw             # mesh geometry, easing, snapping, icon scaling
cargo test ui::settings::theme  # palette contrast, accent text, radii
cargo test ui::settings::window # window layout bands
cargo test                      # everything
```

What the tests pin down:

- `ring_slice_mesh`: every index is valid, only the core is opaque, the fringe lies half a feather
  outside each edge, the covered area is unchanged, a full ring has no seam, degenerate slices make no
  mesh.
- `prepare_icon`: size, color, premultiplied invariant, no bleed from transparent pixels.
- `step_linear`: fixed duration, stops exactly on the target, frame-rate independent.
- `arc_segments`: the chord error stays under the tolerance at every radius.
- `regions`: the bands tile the window without overlap, also at the minimum height.

Pixels on a screen cannot be unit tested, so see the checklist below before you open a pull request
that changes drawing.

## Manual visual checklist

Run `cargo run --release` and check on a **standard** display and a **Retina / high-DPI** display, or
at the very least change the display scaling once:

- [ ] Hold the trigger: the slices have smooth edges and a thin outline, with no stair-steps, at pie
      sizes 140, 200 and 360.
- [ ] Hover each slice: the accent fill and the rim bar fade in without flicker; icons stay sharp.
- [ ] Open a submenu: the ring blooms again around the cursor and is never cut off by the window.
- [ ] Open and close the pie ten times quickly: no leftover frame, no flash at the old position.
- [ ] Open Settings: the window outline is a clean one-pixel line, corners are round, no gradient
      mesh edge.
- [ ] Switch through all four tabs: nothing moves in the top bar; the selected tab is the light block.
- [ ] Pick a very light accent (yellow) and a very dark one: Save and the selected slice stay readable.
- [ ] Undo and Redo are dimmed when unavailable and show a tooltip.
- [ ] Shrink the screen height (or use a short laptop display): the Menu tab still fits.
- [ ] Use an image icon with a transparent background: no dark halo around it.

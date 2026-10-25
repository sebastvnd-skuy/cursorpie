# UI design and rendering

This document explains how CursorPie looks, why it is drawn the way it is, and how to check a
change to either. The code is in `src/ui/`; the rules for contributors are summarized in
`CONTRIBUTING.md` and `guides::extending`.

## Contents

- [Design goals](#design-goals)
- [The Settings window](#the-settings-window)
- [The welcome window](#the-welcome-window)
- [The pie](#the-pie)
- [Why it used to look jagged and choppy](#why-it-used-to-look-jagged-and-choppy)
- [Rendering rules](#rendering-rules)
- [Transparent window on Windows](#transparent-window-on-windows)
- [Transparent window on Linux](#transparent-window-on-linux)
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
| logo CURSORPIE ⓘ                            |  UNDO REDO CANCEL SAVE |   top bar, 52 pt
+--------------+-------------------------------------------------------+
| Menu         |                                                       |
| Behavior     |   page of the selected section (padding 16 pt)        |
| General      |                                                       |
| About        |                                                       |
|        v0.3  |                                                       |
+--------------+-------------------------------------------------------+
| status or hint                                            QUIT APP   |   footer, 34 pt
+----------------------------------------------------------------------+
```

- The window is 1120 x 680 pt (`CARD_SIZE`) with a 12 pt corner radius. `card_size_for` shrinks it to
  fit a small display, but never below 940 x 420 pt, where the Menu tab still has room for its pie tile
  and an editor, and the body is still 302 pt high.
- `regions()` in `window.rs` cuts the window into the top bar, the sidebar (200 pt wide), the body and
  the footer. It is a pure function with tests.
- **The sidebar names the section, the page is on its right.** Each entry has a title and a one-line hint.
  The selected entry is lifted (`HOT`) and has an accent marker on its left edge; the others are muted
  and light up under the pointer. The sidebar is `CARD`-colored and the page is `BG`, so panels read as
  cards on the page.
- Behavior, General and About are one scrolling column of panels, at most 680 pt wide and aligned to the
  left of the page, so a slider is never longer than the eye can follow. Menu fills the whole page.
- Panel titles and badges are small monospace capitals (`card_title`, `badge`). Everything you read or
  edit, including the sidebar entries and the page titles, is the normal font in sentence case.
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

## The welcome window

The first start (and "Welcome guide..." in the tray menu) shows a 760 x 520 pt card (`WELCOME_SIZE`,
shrunk by `welcome_size_for` on small displays, never below 600 x 440 pt) in the same window as Settings,
with the same shadow, outline, colors, open/close animation and widgets. It has a 52 pt top bar (logo,
name, Skip), a page in the middle and a 60 pt footer (page dots, Back, Next or Get started).

- Three pages: how the gesture works, what to do next (macOS: Accessibility, with a status that
  refreshes every second; Windows: the system tray) and tips with the launch-at-login box.
- It is a mode of the Settings window, not a second window: `App::welcome` is `Some` and
  `settings_open` is true, so the pie stays hidden while it is open.
- The only text that depends on the OS lives in pure functions of `Os` (`setup_title`, `setup_lead`,
  `tips`), so a test covers both systems.
- Esc and Skip close it without applying anything; Get started and Open Settings also apply the
  launch-at-login box. Either way it is remembered as shown.

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
8. **Draw only what is on screen.** A list or grid that can pass a screenful uses `ScrollArea::show_rows` (see `icon_picker.rs`), builds its tiles from indexes instead of cloned strings, and loads textures only for the visible rows, a few per frame (`LOADS_PER_FRAME`).
9. **Limit a file where it enters.** Read an image's size from its header and refuse it before decoding (`draw::MAX_SOURCE_SIDE`); shrink what the user chooses once (`ui::iconfit`) and say so in the status bar.

## Transparent window on Windows

The pie and the Settings card are drawn on a borderless window whose background is fully transparent
(`with_transparent(true)`, `clear_color` is `[0, 0, 0, 0]`, the central panel has no frame).
Whether that really is transparent depends on the renderer and the OS:

| Platform | Renderer | Result |
|---|---|---|
| macOS | `wgpu` (eframe default, Metal) | transparent |
| Windows | `wgpu` (eframe default since 0.34; DX12, Vulkan or GL surface) | opaque: the pie sits in a **black box** |
| Windows | `glow` (OpenGL through glutin) | transparent, **once the OS frame is stripped** (rule 4): the pixel format has an alpha channel and winit turns on per-pixel alpha. Verified on Windows in 0.3.0 |

So `Cargo.toml` selects the renderer per target: Windows builds eframe with `default-features = false`
and `glow`, every other target keeps the defaults. Rules:

1. **Never enable eframe's `wgpu` feature in the Windows dependency table.** If both renderers are
   compiled in, eframe prefers `wgpu` and the black box comes back.
2. **Keep every layer above the window transparent:** `clear_color` alpha 0, `CentralPanel` with
   `Frame::new()` in pie mode. Settings paints its own opaque card inside the margin (`WIN_MARGIN`).
3. **Check with a clean build.** After changing eframe features run `cargo tree -i wgpu --target
   x86_64-pc-windows-msvc`: it must report that `wgpu` is not part of the Windows build.

4. **Strip the OS frame (Windows).** `glow` alone is not enough: winit creates every window from
   `WS_CAPTION | WS_BORDER | WS_EX_WINDOWEDGE` and only *hides* the title bar for
   `with_decorations(false)` (it answers `WM_NCCALCSIZE` with "client area = whole window"). The DWM
   still sees a framed window, so around the transparent pie it draws a 1 px border (Windows 11),
   a shadow, and fills the frame with the inactive-caption color, which is **white**: that is the
   white box with a border. `platform::clear_window_frame` (`platform/windows/window.rs`) turns the
   window into a plain `WS_POPUP`, clears the edge styles, and sets `DWMWA_BORDER_COLOR = NONE`,
   `DWMWCP_DONOTROUND` and the empty blur region. winit rewrites the styles on every
   show / hide / click-through / window-level change, so `App::logic` calls it **on every frame**
   (it is a no-op when the styles are already right); `lib.rs` also calls it once at creation.
   Do not remove either call. A window subclass in the same file also answers `WM_STYLECHANGING`
   (fixes the styles *before* winit's write lands: no white flash), `WM_NCCALCSIZE` (egui-winit
   requests winit's "undecorated shadow" for every borderless window, which shifts the client area
   down 1 px and leaves a white 1 px line at the top) and `WM_NCPAINT`.

If a machine still shows a black box with `glow`, note the Windows version, the GPU and driver (laptops
with two GPUs behave differently), and whether Settings is affected too, then open an issue.

## Transparent window on Linux

The same single window is created as on the other platforms: borderless, transparent
(`with_transparent(true)`), always on top, click-through in pie mode (`with_mouse_passthrough`), not
focused (`with_active(false)`). What differs is what stands between that request and the screen.
**None of the rows below was checked on a real desktop yet**; the Linux backend has only been exercised
against a bare X server (see `docs/LINUX_MIGRATION.md`, "Implementation status").

| Setup | Expected result |
|---|---|
| X11, a compositing manager is running (Mutter, KWin, Xfwm with compositing on, picom...) | transparent: the window has an alpha channel and the compositor blends it |
| X11, **no** compositing manager (a bare i3, bspwm or openbox) | opaque: the pie sits in a **black box**. Per-pixel alpha needs a compositor. Not handled yet; the planned fallback is an XShape mask, see `docs/LINUX_MIGRATION.md` |
| Wayland session | the window runs through XWayland, because only eframe's `x11` feature is enabled (on purpose, so that the window can be placed under the pointer). Whether it is stacked above Wayland-native windows depends on the compositor |

Rules:

1. **Use `glow`, never `wgpu`, in the Linux dependency table.** `Cargo.toml` builds eframe with
   `default-features = false` and `glow` + `x11` on Linux, like Windows, so that a Vulkan-less machine
   still works and the surface has an alpha channel. If both renderers are compiled in, eframe prefers
   `wgpu`. After changing eframe features run `cargo tree -i wgpu --target x86_64-unknown-linux-gnu`:
   it must report that `wgpu` is not part of the Linux build. Do not add the `wayland` feature either
   without reading the "Pie window" part of `docs/LINUX_MIGRATION.md`: a plain Wayland window cannot
   choose its position or stay on top.
2. **Keep every layer above the window transparent**, exactly as in the Windows rules: `clear_color`
   alpha 0, `CentralPanel` with `Frame::new()` in pie mode.
3. **There is no frame to strip.** `platform::clear_window_frame` does nothing on Linux. winit asks the
   window manager for no decorations (`with_decorations(false)`); a window manager that ignores that
   would draw a frame, which has not been seen yet.
4. **The window class is `cursorpie`** (`with_app_id` in `lib.rs`, which sets `WM_CLASS`). It lets a
   window-manager rule or a launcher recognize the window. The backend also skips its own windows when
   it looks for the app under the pointer, by process id and by this id, so the pie never counts as
   "the app below".
5. **The scale is global.** X11 has no per-monitor scale. `platform/linux/x11.rs` reads
   `WINIT_X11_SCALE_FACTOR`, else `Xft.dpi` divided by 96, the numbers winit is expected to use, so the
   pointer and the window agree. If the pie sits off-center on a high-DPI display, compare
   `xrdb -query | grep Xft.dpi` with the scale the window uses.
6. **What is not done.** No `_NET_WM_WINDOW_TYPE` hint, no skip-taskbar or skip-pager hint, no
   override-redirect, no XShape fallback. A window manager may therefore show a taskbar button for the
   pie, or tile it (tiling window managers such as i3 and bspwm are the likely trouble). The planned
   test of the common window managers is spike S3 in `docs/LINUX_MIGRATION.md`.

If a machine shows a black box, note the desktop or window manager, whether a compositor runs, the GPU
and driver, and whether the session is X11 or Wayland (XWayland), and whether Settings is affected too,
then open an issue.

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
- [ ] Switch through all four sections: nothing moves in the top bar or the sidebar; the selected entry is
      lifted and has the accent marker.
- [ ] Shrink the display width (or use a 1366 x 768 display): the card fits, and the Menu editor still shows
      its fields without a horizontal scroll.
- [ ] Pick a very light accent (yellow) and a very dark one: Save and the selected slice stay readable.
- [ ] Undo and Redo are dimmed when unavailable and show a tooltip.
- [ ] Shrink the screen height (or use a short laptop display): the Menu tab still fits.
- [ ] Use an image icon with a transparent background: no dark halo around it.
- [ ] Icon picker, **Mine** page, with 100 or more images in the `icons` folder: scrolling stays smooth, tiles show a dot-dot-dot for a moment and then their image, and memory does not jump when the page opens.
- [ ] Choose an image over 1000 px with **Image...**: the status bar says it was shrunk, the icon stays sharp, the original is untouched. Choose a text file renamed `.png`: the status bar says it cannot be read.
- [ ] Welcome window (delete `welcome_shown`, or use the tray item): the three pages fit without scrolling at
      the smallest card size, the page dots follow Back and Next, and Skip, Esc and Enter behave as the buttons say.
- [ ] Windows: hold the trigger over a bright window (a white page): the pie floats over it with no
      black or white box around the ring, and the same for the margin around the Settings card.
- [ ] Linux (X11): hold the trigger over a bright window: the pie floats over it with no black box. Repeat
      with the compositor switched off to see the fallback behavior, and note it.
- [ ] Linux: the pie is centered under the pointer at 100 percent and at a larger scale, and a tiling window
      manager does not tile it.
- [ ] Linux (Wayland session, XWayland): the pie appears above a native Wayland window, or note that it does not.

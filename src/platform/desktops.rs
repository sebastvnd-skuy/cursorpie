//! Finding the virtual desktop (Windows) or Space (macOS) that holds a window: the decisions,
//! with no OS calls.
//!
//! Neither OS has a public call that switches to a given desktop, nor one that lists the desktops
//! in order. The only thing a backend can do is what the user does: press Win+Ctrl+Right /
//! Win+Ctrl+Left, or Ctrl+Right / Ctrl+Left on macOS, and look at where it ended up.
//! [`walk_to_target`] is that search. It takes the OS through the small [`Desktops`] trait, so the
//! rules that matter (never strand the user on another desktop, never loop forever, do not trust a
//! missing answer) are unit tested on every OS with a fake row of desktops. The implementations
//! are `platform/windows/desktops.rs` and `platform/macos/spaces.rs`.
//!
//! The same module holds the other decisions that need no OS call: where an app's windows are
//! ([`presence_of`]) and how one desktop is told from another ([`signature`]).

/// Whether two paths of an app bundle name the same one: `/Applications/Helium.app/`, as it may be
/// typed in a slice, and `/Applications/Helium.app`, as the system reports it. Trailing slashes
/// and letter case (the default file system ignores it) do not matter.
pub fn same_app_path(a: &str, b: &str) -> bool {
    a.trim()
        .trim_end_matches('/')
        .eq_ignore_ascii_case(b.trim().trim_end_matches('/'))
}

/// The name of an app bundle path without the folders and the `.app` suffix: `Helium` for
/// `/Applications/Helium.app/`. Empty for a path with no name.
pub fn app_name_of_path(path: &str) -> &str {
    let name = path
        .trim()
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("");
    match name.len().checked_sub(4) {
        Some(at) if name.is_char_boundary(at) && name[at..].eq_ignore_ascii_case(".app") => {
            &name[..at]
        }
        _ => name,
    }
}

/// How many desktops are crossed in one direction before the search gives up.
pub const MAX_STEPS: usize = 10;

/// Smallest width and height, in points, of a window that counts as "the app's window": palettes,
/// helper windows and one-pixel overlays are smaller.
const MIN_WINDOW_SIZE: f64 = 50.0;

/// One window of the system's window list, reduced to what the decisions need.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowInfo {
    /// The system's number for the window.
    pub number: u32,
    /// Process id of the app that owns it.
    pub pid: i32,
    /// Stacking layer: 0 is a normal window, menus, the Dock and overlays sit above.
    pub layer: i64,
    /// 0.0 (invisible) to 1.0.
    pub alpha: f64,
    pub width: f64,
    pub height: f64,
    /// Whether it is on a desktop that is shown right now.
    pub on_screen: bool,
}

impl WindowInfo {
    /// Whether this is a window the user would call a window of the app: a normal layer, visible
    /// and not tiny.
    pub fn is_ordinary(&self) -> bool {
        self.layer == 0
            && self.alpha >= 0.01
            && self.width >= MIN_WINDOW_SIZE
            && self.height >= MIN_WINDOW_SIZE
    }
}

/// Where the windows of one app are, relative to the desktop that is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presence {
    /// At least one of its windows is on the shown desktop.
    Here,
    /// None is, but one is somewhere else (or minimized): the number of the top-most such window.
    Elsewhere(u32),
    /// The app has no window of the kind the user would see.
    Nowhere,
}

/// Where the windows of the app `pid` are. `windows` is the whole window list, front to back, so
/// the first match is the top-most one.
pub fn presence_of(windows: &[WindowInfo], pid: i32) -> Presence {
    let mut elsewhere = None;
    for w in windows.iter().filter(|w| w.pid == pid && w.is_ordinary()) {
        if w.on_screen {
            return Presence::Here;
        }
        elsewhere.get_or_insert(w.number);
    }
    elsewhere.map_or(Presence::Nowhere, Presence::Elsewhere)
}

/// A number that tells one desktop from another, made of the ids of the windows on it: the same
/// set gives the same number whatever the order, a different set gives another. `None` when there
/// is nothing to go by. A set rather than one id, because a window shown on every desktop would
/// make two desktops look the same.
pub fn signature<T: std::hash::Hash + Ord>(mut ids: Vec<T>) -> Option<u64> {
    use std::hash::{Hash, Hasher};
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return None;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    ids.hash(&mut hasher);
    Some(hasher.finish())
}

/// The most steps a known route may take: a row of desktops is never this long, so it only guards
/// against a wrong answer from the system.
const MAX_ROUTE: usize = 32;

/// How many steps right (positive) or left (negative) it is from the desktop `from` to the nearest
/// of `targets` in `row`, the desktops in order from left to right. `None` when `from` is not in the
/// row or none of the targets is. A tie goes to the right.
pub fn steps_between<T: PartialEq>(row: &[T], from: &T, targets: &[T]) -> Option<isize> {
    let at = row.iter().position(|d| d == from)? as isize;
    targets
        .iter()
        .filter_map(|t| row.iter().position(|d| d == t))
        .map(|i| i as isize - at)
        .min_by_key(|n| (n.abs(), -n))
}

/// One display and its own row of desktops. With separate Spaces per display (the macOS default
/// with more than one screen) every display has Spaces of its own and shows one of them: the
/// Spaces of screen 1 are not on the way to a window on screen 2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayRow {
    /// What names the display to the system (on macOS the UUID of the display).
    pub display: String,
    /// The desktop this display shows right now.
    pub current: u64,
    /// The desktops of this display, from left to right.
    pub spaces: Vec<u64>,
}

/// The display to walk, and the desktops of the window on it. `window_spaces` holds, for every
/// window of the app (front to back), the desktops it is on; `pointer` is the display under the
/// pointer. A display that has a window of the app and is under the pointer wins: the switch then
/// happens where the user clicked. Otherwise it is the display of the top-most window, so the
/// switch happens on the screen the window is on, not on the one that happens to be clicked.
///
/// Returns the index in `rows` and the desktops of the app's windows on that display, `None` when
/// no window is on a desktop of any display.
pub fn pick_display(
    rows: &[DisplayRow],
    window_spaces: &[Vec<u64>],
    pointer: Option<&str>,
) -> Option<(usize, Vec<u64>)> {
    // (display, desktop) of every window, front to back
    let hits: Vec<(usize, u64)> = window_spaces
        .iter()
        .flatten()
        .filter_map(|space| {
            rows.iter()
                .position(|r| r.spaces.contains(space))
                .map(|at| (at, *space))
        })
        .collect();
    let chosen = hits
        .iter()
        .map(|(at, _)| *at)
        .find(|at| Some(rows[*at].display.as_str()) == pointer)
        .or_else(|| hits.first().map(|(at, _)| *at))?;
    let mut spaces: Vec<u64> = Vec::new();
    for (_, space) in hits.iter().filter(|(at, _)| *at == chosen) {
        if !spaces.contains(space) {
            spaces.push(*space);
        }
    }
    Some((chosen, spaces))
}

/// The row of virtual desktops, as the search sees it.
pub trait Desktops {
    /// What tells one desktop from another.
    type Id: Copy + PartialEq;

    /// Whether the window being looked for is on the desktop that is shown right now.
    fn target_here(&mut self) -> bool;

    /// The desktop shown right now. `None` when it cannot be told, for example because no window
    /// on it can be asked.
    fn current(&mut self) -> Option<Self::Id>;

    /// Moves one desktop to the right (`forward`) or to the left, and returns once the shell has
    /// finished the switch. At either end of the row it does nothing.
    fn step(&mut self, forward: bool);

    /// When the order of the desktops is known: how many steps right (positive) or left (negative)
    /// the desktop of the window is. `None` when it is not known, and the search has to feel its way.
    fn route(&mut self) -> Option<isize> {
        None
    }
}

/// Switches desktops until the window being looked for is on the one shown. When the order of the
/// desktops is known (`route`) it goes straight there, the shorter way, however many desktops are
/// in between. Otherwise it tries the right side first and then the left, at most `max_steps`
/// desktops each way.
///
/// Returns `true` when it got there. On `false` the user is back on the desktop they started on:
/// every way that was tried is walked back, and the walk back stops as soon as the starting
/// desktop shows again, so a step that did nothing at the end of the row is never undone by a step
/// that would move.
///
/// Nothing is felt for when the starting desktop cannot be told (`current` is `None` at the start):
/// without it there is no way to know that the walk back has arrived.
pub fn walk_to_target<D: Desktops>(d: &mut D, max_steps: usize) -> bool {
    if d.target_here() {
        return true;
    }
    let home = d.current();
    if let Some(n) = d.route().filter(|n| *n != 0) {
        let forward = n > 0;
        let mut taken = 0;
        // every step is taken: while a switch is animating the windows of two desktops are on the
        // screen, so "the window is here" on the way would stop the user at a desktop in between
        for _ in 0..n.unsigned_abs().min(MAX_ROUTE) {
            d.step(forward);
            taken += 1;
        }
        if d.target_here() {
            return true;
        }
        // the route was wrong: go home
        for _ in 0..taken {
            if home.is_some() && d.current() == home {
                break;
            }
            d.step(!forward);
        }
    }
    let Some(home) = home else {
        return false;
    };
    for forward in [true, false] {
        let mut last = Some(home);
        let mut taken = 0;
        for _ in 0..max_steps {
            d.step(forward);
            if d.target_here() {
                return true;
            }
            let now = d.current();
            // Same desktop as before: this was the end of the row. An unknown desktop (None) is
            // taken for a move, because an empty desktop is a real place to cross.
            if now.is_some() && now == last {
                break;
            }
            last = now;
            taken += 1;
        }
        for _ in 0..taken {
            if d.current() == Some(home) {
                break;
            }
            d.step(!forward);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row of `count` desktops. Desktops in `empty` have no window that could tell their id.
    struct Row {
        count: usize,
        at: usize,
        target: usize,
        empty: Vec<usize>,
        steps: Vec<bool>,
        /// What `route` answers
        route: Option<isize>,
        /// A desktop where `target_here` wrongly says yes, like one in the middle of an animation
        lie: Option<usize>,
    }

    impl Row {
        fn new(count: usize, at: usize, target: usize) -> Self {
            Row {
                count,
                at,
                target,
                empty: Vec::new(),
                steps: Vec::new(),
                route: None,
                lie: None,
            }
        }
    }

    impl Desktops for Row {
        type Id = usize;

        fn target_here(&mut self) -> bool {
            self.at == self.target || self.lie == Some(self.at)
        }

        fn current(&mut self) -> Option<usize> {
            (!self.empty.contains(&self.at)).then_some(self.at)
        }

        fn step(&mut self, forward: bool) {
            self.steps.push(forward);
            if forward && self.at + 1 < self.count {
                self.at += 1;
            } else if !forward && self.at > 0 {
                self.at -= 1;
            }
        }

        fn route(&mut self) -> Option<isize> {
            self.route
        }
    }

    #[test]
    fn an_app_path_matches_with_or_without_the_trailing_slash() {
        assert!(same_app_path(
            "/Applications/Helium.app/",
            "/Applications/Helium.app"
        ));
        assert!(same_app_path(
            " /Applications/Helium.app ",
            "/applications/helium.app/"
        ));
        assert!(!same_app_path(
            "/Applications/Helium.app",
            "/Applications/Other.app"
        ));
        assert!(!same_app_path(
            "/Applications/Helium.app",
            "/Users/me/Applications/Helium.app"
        ));
    }

    #[test]
    fn the_name_of_an_app_is_taken_from_its_path() {
        assert_eq!(app_name_of_path("/Applications/Helium.app/"), "Helium");
        assert_eq!(
            app_name_of_path("/Applications/Google Chrome.app"),
            "Google Chrome"
        );
        assert_eq!(app_name_of_path("/Applications/Tool.APP"), "Tool");
        assert_eq!(app_name_of_path("Safari"), "Safari");
        assert_eq!(app_name_of_path("/"), "");
        assert_eq!(app_name_of_path(".app"), "");
    }

    /// A window of `pid`, 800 x 600, normal layer.
    fn win(number: u32, pid: i32, on_screen: bool) -> WindowInfo {
        WindowInfo {
            number,
            pid,
            layer: 0,
            alpha: 1.0,
            width: 800.0,
            height: 600.0,
            on_screen,
        }
    }

    #[test]
    fn an_app_with_a_window_on_the_shown_desktop_is_here() {
        let list = [win(1, 7, false), win(2, 7, true), win(3, 8, true)];
        assert_eq!(presence_of(&list, 7), Presence::Here);
    }

    #[test]
    fn an_app_with_windows_only_elsewhere_gives_its_top_most_one() {
        let list = [win(5, 8, true), win(4, 7, false), win(3, 7, false)];
        assert_eq!(presence_of(&list, 7), Presence::Elsewhere(4));
    }

    #[test]
    fn helper_windows_do_not_count_as_the_apps_window() {
        let mut palette = win(1, 7, true);
        palette.width = 20.0;
        let mut menu = win(2, 7, true);
        menu.layer = 25;
        let mut ghost = win(3, 7, true);
        ghost.alpha = 0.0;
        let list = [palette, menu, ghost, win(4, 7, false)];
        assert_eq!(presence_of(&list, 7), Presence::Elsewhere(4));
        assert_eq!(presence_of(&list[..3], 7), Presence::Nowhere);
        assert_eq!(presence_of(&[], 7), Presence::Nowhere);
    }

    #[test]
    fn a_desktop_is_told_by_the_set_of_its_windows() {
        let a = signature(vec![1u32, 2, 3]);
        assert!(a.is_some());
        assert_eq!(
            a,
            signature(vec![3u32, 1, 2, 2]),
            "order and repeats do not matter"
        );
        assert_ne!(
            a,
            signature(vec![1u32, 2]),
            "another set is another desktop"
        );
        // a window shown on every desktop does not make two desktops look alike
        assert_ne!(signature(vec![9u32, 1]), signature(vec![9u32, 2]));
        assert_eq!(signature(Vec::<u32>::new()), None);
    }

    #[test]
    fn a_window_on_this_desktop_needs_no_step() {
        let mut row = Row::new(3, 1, 1);
        assert!(walk_to_target(&mut row, MAX_STEPS));
        assert!(row.steps.is_empty());
    }

    #[test]
    fn a_window_to_the_right_is_reached_by_stepping_right() {
        let mut row = Row::new(4, 0, 2);
        assert!(walk_to_target(&mut row, MAX_STEPS));
        assert_eq!((row.at, row.steps), (2, vec![true, true]));
    }

    #[test]
    fn a_window_to_the_left_is_found_after_the_right_side_ends() {
        let mut row = Row::new(4, 3, 1);
        assert!(walk_to_target(&mut row, MAX_STEPS));
        // one step into the wall on the right, then two to the left
        assert_eq!((row.at, row.steps), (1, vec![true, false, false]));
    }

    #[test]
    fn a_missing_window_leaves_the_user_where_they_started() {
        for home in 0..4 {
            let mut row = Row::new(4, home, 99);
            assert!(!walk_to_target(&mut row, MAX_STEPS));
            assert_eq!(row.at, home, "started on desktop {home}");
        }
    }

    #[test]
    fn an_empty_desktop_on_the_way_is_crossed() {
        let mut row = Row::new(4, 0, 2);
        row.empty = vec![1];
        assert!(walk_to_target(&mut row, MAX_STEPS));
        assert_eq!(row.at, 2);
    }

    #[test]
    fn nothing_is_tried_when_the_start_cannot_be_told() {
        let mut row = Row::new(3, 0, 2);
        row.empty = vec![0];
        assert!(!walk_to_target(&mut row, MAX_STEPS));
        assert!(row.steps.is_empty());
    }

    #[test]
    fn the_search_is_bounded_even_with_no_end_to_the_row() {
        let mut row = Row::new(100, 50, 99_999);
        assert!(!walk_to_target(&mut row, 3));
        assert_eq!(row.at, 50);
        // 3 out and 3 back on each side
        assert_eq!(row.steps.len(), 12);
    }

    #[test]
    fn an_unrecognised_end_of_the_row_does_not_carry_the_user_past_home() {
        // desktop 2 is the last one and has no window to ask: every step into the wall looks like
        // a move, so the walk back would be too long if it only counted
        let mut row = Row::new(3, 1, 99);
        row.empty = vec![2];
        assert!(!walk_to_target(&mut row, 5));
        assert_eq!(row.at, 1);
    }

    #[test]
    fn the_nearest_of_the_targets_is_chosen() {
        let row = ["a", "b", "c", "d", "e"];
        assert_eq!(steps_between(&row, &"c", &["a"]), Some(-2));
        assert_eq!(steps_between(&row, &"a", &["d"]), Some(3));
        assert_eq!(steps_between(&row, &"c", &["a", "d"]), Some(1));
        assert_eq!(
            steps_between(&row, &"c", &["b", "d"]),
            Some(1),
            "a tie goes right"
        );
        assert_eq!(steps_between(&row, &"c", &["c"]), Some(0));
    }

    #[test]
    fn no_route_when_a_desktop_is_not_in_the_row() {
        let row = ["a", "b"];
        assert_eq!(steps_between(&row, &"x", &["a"]), None);
        assert_eq!(steps_between(&row, &"a", &["x"]), None);
        assert_eq!(steps_between(&row, &"a", &[]), None);
    }

    /// Screen "a" has the desktops 1 to 3, screen "b" has 7 and 8.
    fn two_screens() -> Vec<DisplayRow> {
        vec![
            DisplayRow {
                display: "a".into(),
                current: 1,
                spaces: vec![1, 2, 3],
            },
            DisplayRow {
                display: "b".into(),
                current: 7,
                spaces: vec![7, 8],
            },
        ]
    }

    #[test]
    fn a_window_on_another_screen_is_walked_on_that_screen() {
        // the pointer is on screen "a", the window is on desktop 8 of screen "b"
        let picked = pick_display(&two_screens(), &[vec![8]], Some("a"));
        assert_eq!(picked, Some((1, vec![8])));
    }

    #[test]
    fn the_screen_under_the_pointer_wins_when_it_has_a_window_too() {
        let windows = [vec![8], vec![3]];
        assert_eq!(
            pick_display(&two_screens(), &windows, Some("a")),
            Some((0, vec![3]))
        );
        assert_eq!(
            pick_display(&two_screens(), &windows, Some("b")),
            Some((1, vec![8]))
        );
        // pointer on no known screen: the top-most window decides
        assert_eq!(
            pick_display(&two_screens(), &windows, None),
            Some((1, vec![8]))
        );
    }

    #[test]
    fn only_the_desktops_of_the_chosen_screen_are_targets() {
        let windows = [vec![2], vec![8], vec![3], vec![2]];
        assert_eq!(
            pick_display(&two_screens(), &windows, Some("a")),
            Some((0, vec![2, 3]))
        );
    }

    #[test]
    fn a_window_on_no_known_desktop_gives_no_screen() {
        assert_eq!(pick_display(&two_screens(), &[vec![99]], Some("a")), None);
        assert_eq!(pick_display(&two_screens(), &[Vec::new()], Some("a")), None);
        assert_eq!(pick_display(&two_screens(), &[], None), None);
        assert_eq!(pick_display(&[], &[vec![1]], None), None);
    }

    #[test]
    fn the_row_of_the_chosen_screen_gives_the_route() {
        // from desktop 1 of screen "a" to desktop 8 of screen "b" is not 7 steps on one long row:
        // the walk is on "b", from what "b" shows (7) to 8
        let rows = two_screens();
        let (at, targets) = pick_display(&rows, &[vec![8]], Some("a")).unwrap();
        assert_eq!(
            steps_between(&rows[at].spaces, &rows[at].current, &targets),
            Some(1)
        );
    }

    #[test]
    fn a_known_route_goes_straight_to_the_left() {
        let mut row = Row::new(6, 4, 1);
        row.route = Some(-3);
        assert!(walk_to_target(&mut row, MAX_STEPS));
        assert_eq!((row.at, row.steps), (1, vec![false; 3]));
    }

    #[test]
    fn a_known_route_crosses_many_desktops_to_the_right() {
        let mut row = Row::new(20, 0, 15);
        row.route = Some(15);
        assert!(walk_to_target(&mut row, 3));
        assert_eq!((row.at, row.steps.len()), (15, 15));
    }

    #[test]
    fn a_known_route_works_from_an_empty_desktop() {
        let mut row = Row::new(4, 0, 2);
        row.empty = vec![0];
        row.route = Some(2);
        assert!(walk_to_target(&mut row, MAX_STEPS));
        assert_eq!(row.at, 2);
    }

    #[test]
    fn a_wrong_route_falls_back_to_feeling_for_the_window() {
        // the route says left, the window is on the right
        let mut row = Row::new(5, 2, 4);
        row.route = Some(-1);
        assert!(walk_to_target(&mut row, MAX_STEPS));
        assert_eq!(row.at, 4);
    }

    #[test]
    fn a_known_route_does_not_stop_at_a_desktop_in_between() {
        let mut row = Row::new(6, 4, 1);
        row.route = Some(-3);
        row.lie = Some(3);
        assert!(walk_to_target(&mut row, MAX_STEPS));
        assert_eq!((row.at, row.steps), (1, vec![false; 3]));
    }
}

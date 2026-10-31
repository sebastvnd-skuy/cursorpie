//! The mouse in use: how many extra buttons it has, and a profile for every mouse CursorPie has met.
//!
//! A mode slice (`ItemKind::Mode`) can send an action for any extra button of the mouse, not only for
//! back and forward. To offer the right buttons, CursorPie has to know the mouse:
//!
//! 1. **Detect.** The OS says which mice are connected and how many buttons each one has
//!    ([`platform::mouse_devices`](crate::platform::mouse_devices)). On Linux that is the kernel's
//!    list of input devices; on Windows only the number of buttons; macOS has no list CursorPie can
//!    read, so a Mac mouse is found by scanning it.
//! 2. **Remember.** Every mouse gets a [`MouseProfile`] in `mice.json` next to `config.json`, keyed by
//!    its hardware id (the same model is the same profile). [`MouseBook::observe`] compares what is
//!    connected now with the book and says whether the mouse is [new](Recognition::New), one that was
//!    [seen before](Recognition::Returned), or [the same as before](Recognition::Same).
//! 3. **Scan.** The OS can over- or under-report. In Settings (General, Mouse) the person presses
//!    every extra button once ([`Scan`]); what was pressed is stored in the profile and wins over what
//!    the OS reported.
//!
//! Button numbers are the macOS ones used everywhere else (0 left, 1 right, 2 middle, 3 back,
//! 4 forward, 5 and up extra). [`FIRST_EXTRA`] to [`MAX_BUTTON`] are the ones a mode can use.
//!
//! The book is device state, not part of the config: it is saved at once, and Settings' Save, Undo and
//! backups never touch it. Lock order: never hold the config lock and this module's lock together.

use crate::config::{app_support_dir, backup, lock, read_text_capped};
use crate::msg::{Msg, Notifier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Button number of the back side button.
pub const BACK: i64 = 3;
/// Button number of the forward side button.
pub const FORWARD: i64 = 4;
/// The first button that is not left, right or middle.
pub const FIRST_EXTRA: i64 = 3;
/// The last button number CursorPie handles. On Linux this is X button 20.
pub const MAX_BUTTON: i64 = 15;
/// How many buttons [`FIRST_EXTRA`] to [`MAX_BUTTON`] are (the size of per-button tables).
pub const SLOTS: usize = (MAX_BUTTON - FIRST_EXTRA + 1) as usize;
/// Mice kept in the book; the one that was seen longest ago is forgotten first.
pub const MAX_MICE: usize = 24;
/// How often the watcher looks at the connected mice.
pub const WATCH_EVERY: Duration = Duration::from_secs(5);
/// A scan ends by itself after this long without a press, so no button stays swallowed for good.
pub const SCAN_IDLE: Duration = Duration::from_secs(45);

/// Version of the `mice.json` format.
const FORMAT: u32 = 1;
/// Largest `mice.json` that is read.
const MAX_FILE_BYTES: u64 = 256 * 1024;
/// Profile that receives a scan when no mouse could be detected (macOS, or no mouse found).
const GENERIC_ID: &str = "name:mouse";
/// Longest name kept for a mouse.
const MAX_NAME: usize = 80;

// ------------------------------------------------------------------ button numbers

/// Whether `button` is one a mode can send an action for (back, forward or an extra button).
pub fn is_extra(button: i64) -> bool {
    (FIRST_EXTRA..=MAX_BUTTON).contains(&button)
}

/// Position of `button` in a table of [`SLOTS`] entries; `None` if it is not an extra button.
pub fn slot(button: i64) -> Option<usize> {
    if is_extra(button) {
        Some((button - FIRST_EXTRA) as usize)
    } else {
        None
    }
}

/// What Settings calls a button: `Back button`, `Forward button`, `Extra button 6`. The numbers
/// count the way the trigger choices do (left is 1, so button number 5 is "Extra button 6").
pub fn button_label(button: i64) -> String {
    match button {
        BACK => "Back button".into(),
        FORWARD => "Forward button".into(),
        n => format!("Extra button {}", n + 1),
    }
}

/// The buttons the editor of a mode offers: the extra buttons of the mouse (without back, forward and
/// the trigger, which has its own job), plus the ones the mode already has an action for, so an action
/// is never hidden because another mouse is connected.
pub fn editor_buttons(mouse: &[i64], used: &[i64], trigger: i64) -> Vec<i64> {
    let own = |b: &i64| is_extra(*b) && *b != BACK && *b != FORWARD;
    let mut set: BTreeSet<i64> = mouse
        .iter()
        .copied()
        .filter(own)
        .filter(|b| *b != trigger)
        .collect();
    set.extend(used.iter().copied().filter(own));
    set.into_iter().collect()
}

/// Sorted, without duplicates, only buttons a mode can use.
fn clean(buttons: &[i64]) -> Vec<i64> {
    let set: BTreeSet<i64> = buttons.iter().copied().filter(|b| is_extra(*b)).collect();
    set.into_iter().collect()
}

// ------------------------------------------------------------------ devices and profiles

/// One mouse as the OS describes it right now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    /// The name the OS gives it ("Logitech G502 HERO Gaming Mouse").
    pub name: String,
    /// USB or Bluetooth ids such as `046d:c08b`; empty when the OS does not say.
    pub hw_id: String,
    /// The extra buttons the OS says it has (button numbers, 3 and up).
    pub extra: Vec<i64>,
}

impl Device {
    /// The key of its profile: the hardware id when there is one, otherwise the name. Two mice of the
    /// same model share a profile, which is what matters: they have the same buttons.
    pub fn id(&self) -> String {
        let hw = self.hw_id.trim().to_ascii_lowercase();
        if !hw.is_empty() {
            return format!("hw:{hw}");
        }
        let name = self
            .name
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        if name.is_empty() {
            "name:unknown".into()
        } else {
            format!("name:{name}")
        }
    }
}

/// What CursorPie keeps about one mouse.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
#[serde(default)]
pub struct MouseProfile {
    /// See [`Device::id`].
    pub id: String,
    pub name: String,
    /// The extra buttons the OS reported the last time this mouse was seen.
    pub reported: Vec<i64>,
    /// The extra buttons the person pressed in a scan. When there are any, they are the truth.
    pub scanned: Vec<i64>,
    /// Unix seconds.
    pub first_seen: u64,
    /// Unix seconds; moves when the mouse is connected again, not on every look.
    pub last_seen: u64,
    /// How many times it was connected (as a new mouse or as a returning one).
    pub times: u32,
}

impl MouseProfile {
    /// The extra buttons of this mouse: what was scanned, or if nothing was, what the OS reported.
    pub fn buttons(&self) -> Vec<i64> {
        if self.scanned.is_empty() {
            clean(&self.reported)
        } else {
            clean(&self.scanned)
        }
    }

    /// Whether a scan has confirmed the buttons.
    pub fn is_scanned(&self) -> bool {
        !self.scanned.is_empty()
    }
}

/// How the mice that are connected now compare with the book.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recognition {
    /// A mouse CursorPie had never met.
    New,
    /// A mouse that is in the book, and is not the one that was in use before.
    Returned,
    /// The mouse that was in use before.
    Same,
    /// No mouse is connected (or the OS cannot tell); the last one stays the mouse in use.
    Absent,
}

/// What [`MouseBook::observe`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub recognition: Recognition,
    /// Whether the book changed, so it has to be saved.
    pub changed: bool,
    /// Name of the mouse in use ("" if there is none).
    pub name: String,
    /// How many extra buttons the mouse in use has.
    pub extra: usize,
}

/// Every mouse CursorPie knows, and which one is in use.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
#[serde(default)]
pub struct MouseBook {
    pub version: u32,
    /// The `id` of the mouse in use ("" = none yet).
    pub active: String,
    pub mice: Vec<MouseProfile>,
}

impl MouseBook {
    /// The profile of the mouse in use.
    pub fn active_profile(&self) -> Option<&MouseProfile> {
        self.mice.iter().find(|p| p.id == self.active)
    }

    /// Forces a loaded book into the limits the rest of the code relies on.
    fn sanitize(mut self) -> Self {
        self.version = FORMAT;
        let mut seen = BTreeSet::new();
        self.mice
            .retain(|p| !p.id.is_empty() && seen.insert(p.id.clone()));
        for p in &mut self.mice {
            p.name = shown(&p.name);
            p.reported = clean(&p.reported);
            p.scanned = clean(&p.scanned);
        }
        self.prune();
        if !self.mice.iter().any(|p| p.id == self.active) {
            self.active.clear();
        }
        self
    }

    /// Drops the mice that were seen longest ago while there are more than [`MAX_MICE`]. The one in
    /// use stays.
    fn prune(&mut self) {
        while self.mice.len() > MAX_MICE {
            let oldest = self
                .mice
                .iter()
                .enumerate()
                .filter(|(_, p)| p.id != self.active)
                .min_by_key(|(_, p)| p.last_seen)
                .map(|(i, _)| i);
            match oldest {
                Some(i) => {
                    self.mice.remove(i);
                }
                None => break,
            }
        }
    }

    /// Compares the mice that are connected now (`devices`) with the book, adds the ones it has not
    /// met, and decides which one is in use:
    ///
    /// - a mouse that is new wins (the one just plugged in is the one in the hand),
    /// - otherwise the mouse that was in use stays, if it is still connected,
    /// - otherwise the first connected mouse that is in the book takes over (it is *returning*),
    /// - with nothing connected the last mouse stays and the result is [`Recognition::Absent`].
    pub fn observe(&mut self, devices: &[Device], now: u64) -> Outcome {
        let before = self.clone();
        // one entry per model: a receiver may list the same mouse twice
        let mut found: Vec<(String, Device)> = Vec::new();
        for d in devices {
            let id = d.id();
            match found.iter().position(|(i, _)| *i == id) {
                Some(at) => {
                    let first = &mut found[at].1;
                    first.extra.extend(d.extra.iter().copied());
                    first.extra = clean(&first.extra);
                }
                None => {
                    let mut d = d.clone();
                    d.extra = clean(&d.extra);
                    found.push((id, d));
                }
            }
        }

        let mut fresh: Option<String> = None;
        for (id, d) in &found {
            match self.mice.iter().position(|p| p.id == *id) {
                Some(at) => self.mice[at].reported = d.extra.clone(),
                None => {
                    if fresh.is_none() {
                        fresh = Some(id.clone());
                    }
                    self.mice.push(MouseProfile {
                        id: id.clone(),
                        name: shown(&d.name),
                        reported: d.extra.clone(),
                        scanned: Vec::new(),
                        first_seen: now,
                        last_seen: now,
                        times: 0,
                    });
                }
            }
        }

        let (active, recognition) = if found.is_empty() {
            (self.active.clone(), Recognition::Absent)
        } else if let Some(id) = fresh {
            (id, Recognition::New)
        } else if found.iter().any(|(i, _)| *i == self.active) {
            (self.active.clone(), Recognition::Same)
        } else {
            (found[0].0.clone(), Recognition::Returned)
        };
        self.active = active;
        if matches!(recognition, Recognition::New | Recognition::Returned)
            && let Some(p) = self.mice.iter_mut().find(|p| p.id == self.active)
        {
            p.times = p.times.saturating_add(1);
            p.last_seen = now;
        }
        self.prune();

        let (name, extra) = self
            .active_profile()
            .map_or((String::new(), 0), |p| (p.name.clone(), p.buttons().len()));
        Outcome {
            recognition,
            changed: *self != before,
            name,
            extra,
        }
    }

    /// Stores what a scan found as the buttons of the mouse in use. With no mouse in use (nothing
    /// could be detected) the scan goes to a generic profile named "Mouse". Returns how many extra
    /// buttons the mouse has now.
    pub fn record_scan(&mut self, pressed: &[i64], now: u64) -> usize {
        if !self.mice.iter().any(|p| p.id == self.active) {
            if !self.mice.iter().any(|p| p.id == GENERIC_ID) {
                self.mice.push(MouseProfile {
                    id: GENERIC_ID.into(),
                    name: "Mouse".into(),
                    first_seen: now,
                    ..MouseProfile::default()
                });
            }
            self.active = GENERIC_ID.into();
        }
        let mut count = 0;
        if let Some(p) = self.mice.iter_mut().find(|p| p.id == self.active) {
            p.scanned = clean(pressed);
            p.last_seen = now;
            count = p.buttons().len();
        }
        self.prune();
        count
    }

    /// Forgets one mouse. If it was in use, nothing is until the next look at the connected mice.
    pub fn forget(&mut self, id: &str) {
        self.mice.retain(|p| p.id != id);
        if self.active == id {
            self.active.clear();
        }
    }
}

/// A name as it is kept: trimmed, on one line, at most [`MAX_NAME`] characters. "Mouse" if empty.
fn shown(name: &str) -> String {
    let one_line = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = one_line.chars().take(MAX_NAME).collect();
    if cut.is_empty() { "Mouse".into() } else { cut }
}

// ------------------------------------------------------------------ the live book

/// The book, loaded on first use.
static BOOK: Mutex<Option<MouseBook>> = Mutex::new(None);
/// Serializes writes of `mice.json`.
static IO: Mutex<()> = Mutex::new(());

/// `<app support dir>/mice.json`.
pub fn book_path() -> PathBuf {
    app_support_dir().join("mice.json")
}

/// Reads `mice.json`; a missing, unreadable or invalid file gives an empty book.
fn load() -> MouseBook {
    read_text_capped(book_path(), MAX_FILE_BYTES)
        .and_then(|text| serde_json::from_str::<MouseBook>(&text).ok())
        .map_or_else(MouseBook::default, MouseBook::sanitize)
}

/// Writes the book. A failed save is ignored on purpose (it must never crash the app); the next
/// change tries again.
fn save(book: &MouseBook) {
    if let Ok(data) = serde_json::to_string_pretty(book) {
        let _io = lock(&IO);
        let _ = backup::write_atomic(&book_path(), &data);
    }
}

/// Runs `f` on the live book (loading it first if needed) and saves the book afterwards when `f`
/// says it changed it.
fn with_book<R>(f: impl FnOnce(&mut MouseBook) -> (R, bool)) -> R {
    let (result, snapshot) = {
        let mut guard = lock(&BOOK);
        let book = guard.get_or_insert_with(load);
        let (result, changed) = f(book);
        (result, changed.then(|| book.clone()))
    };
    if let Some(book) = snapshot {
        save(&book); // outside the lock: the disk can be slow
    }
    result
}

/// Seconds since 1970.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// A copy of the book, for Settings to draw.
pub fn snapshot() -> MouseBook {
    with_book(|b| (b.clone(), false))
}

/// The extra buttons of the mouse in use (empty if none is known).
pub fn active_buttons() -> Vec<i64> {
    with_book(|b| (b.active_profile().map(MouseProfile::buttons), false)).unwrap_or_default()
}

/// Looks at the connected mice now and updates the book. Called by the watcher and by the
/// "Detect again" button.
pub fn refresh() -> Outcome {
    let devices = crate::platform::mouse_devices();
    let out = with_book(|b| {
        let out = b.observe(&devices, now());
        let changed = out.changed;
        (out, changed)
    });
    diag::log!(
        "tap",
        "mouse_look",
        connected = devices.len(),
        recognition = format!("{:?}", out.recognition),
        extra = out.extra,
        changed = out.changed
    );
    out
}

/// Stores the result of a scan for the mouse in use. Returns the number of extra buttons it has now.
pub fn finish_scan(pressed: &[i64]) -> usize {
    let count = with_book(|b| {
        let count = b.record_scan(pressed, now());
        (count, true)
    });
    diag::log!("tap", "mouse_scan", pressed = pressed.len(), extra = count);
    count
}

/// Forgets one mouse of the book.
pub fn forget(id: &str) {
    with_book(|b| {
        b.forget(id);
        ((), true)
    });
}

/// Starts the thread that looks at the connected mice every [`WATCH_EVERY`] and tells the UI when
/// one arrives that is new or known from before.
pub fn spawn_watcher(notifier: Notifier) {
    let started = std::thread::Builder::new()
        .name("cursorpie-mouse".into())
        .spawn(move || {
            loop {
                let out = refresh();
                if matches!(out.recognition, Recognition::New | Recognition::Returned) {
                    notifier.send(Msg::MouseChanged {
                        new: out.recognition == Recognition::New,
                        name: out.name,
                        extra: out.extra,
                    });
                }
                std::thread::sleep(WATCH_EVERY);
            }
        });
    if let Err(e) = started {
        eprintln!("CursorPie: could not start the mouse watcher: {e}");
    }
}

// ------------------------------------------------------------------ scanning

/// Whether a scan is running. The input hook reads it for every extra-button press: while it is on,
/// the press is reported to Settings (`Msg::ButtonScanned`) and swallowed, so a scan never sends
/// "back" to the app in front.
static SCANNING: AtomicBool = AtomicBool::new(false);

/// Whether a scan is running.
pub fn scanning() -> bool {
    SCANNING.load(Ordering::SeqCst)
}

/// A scan in progress, kept in Settings' editor state. Dropping it ends the scan, so closing
/// Settings or freeing its state can never leave the buttons swallowed.
#[derive(Debug)]
pub struct Scan {
    /// The extra buttons pressed so far.
    pub seen: BTreeSet<i64>,
    last_press: Instant,
}

impl Scan {
    /// Starts a scan: from now on extra buttons are swallowed and reported.
    pub fn start() -> Self {
        SCANNING.store(true, Ordering::SeqCst);
        Self {
            seen: BTreeSet::new(),
            last_press: Instant::now(),
        }
    }

    /// Notes a press (the button number of a `Msg::ButtonScanned`).
    pub fn press(&mut self, button: i64) {
        if is_extra(button) {
            self.seen.insert(button);
            self.last_press = Instant::now();
        }
    }

    /// Whether nothing was pressed for [`SCAN_IDLE`].
    pub fn idle(&self) -> bool {
        self.last_press.elapsed() >= SCAN_IDLE
    }

    /// The pressed buttons, in order.
    pub fn pressed(&self) -> Vec<i64> {
        self.seen.iter().copied().collect()
    }
}

impl Drop for Scan {
    fn drop(&mut self) {
        SCANNING.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(name: &str, hw: &str, extra: &[i64]) -> Device {
        Device {
            name: name.into(),
            hw_id: hw.into(),
            extra: extra.to_vec(),
        }
    }

    #[test]
    fn extra_buttons_are_3_to_15() {
        assert!(!is_extra(2));
        assert!(is_extra(3) && is_extra(15));
        assert!(!is_extra(16));
        assert_eq!(slot(3), Some(0));
        assert_eq!(slot(15), Some(SLOTS - 1));
        assert_eq!(slot(2), None);
        assert_eq!(SLOTS, 13);
    }

    #[test]
    fn buttons_are_named_the_way_the_trigger_choices_are() {
        assert_eq!(button_label(3), "Back button");
        assert_eq!(button_label(4), "Forward button");
        assert_eq!(button_label(5), "Extra button 6");
        assert_eq!(button_label(9), "Extra button 10");
    }

    #[test]
    fn the_editor_offers_the_extras_of_the_mouse_and_the_ones_in_use() {
        // back, forward and the trigger are not offered
        assert_eq!(editor_buttons(&[3, 4, 5, 6, 7], &[], 6), [5, 7]);
        // an action that already exists stays visible, even for a button this mouse lacks
        assert_eq!(editor_buttons(&[5], &[9, 5], 2), [5, 9]);
        // even for the trigger button: it can be cleared
        assert_eq!(editor_buttons(&[5, 6], &[6], 6), [5, 6]);
        assert!(editor_buttons(&[], &[], 2).is_empty());
        // buttons outside the table never get in
        assert!(editor_buttons(&[1, 2, 99], &[0, 16], 2).is_empty());
    }

    #[test]
    fn a_device_is_known_by_its_hardware_id_or_else_its_name() {
        assert_eq!(dev("A", " 046D:C08B ", &[]).id(), "hw:046d:c08b");
        assert_eq!(dev("  My   Mouse ", "", &[]).id(), "name:my mouse");
        assert_eq!(dev("", "", &[]).id(), "name:unknown");
        // two names, one hardware id: one mouse
        assert_eq!(dev("A", "1:2", &[]).id(), dev("B", "1:2", &[]).id());
    }

    #[test]
    fn a_mouse_that_was_never_seen_is_new() {
        let mut book = MouseBook::default();
        let out = book.observe(&[dev("G502", "046d:c08b", &[3, 4, 5, 6, 7])], 100);
        assert_eq!(out.recognition, Recognition::New);
        assert!(out.changed);
        assert_eq!((out.name.as_str(), out.extra), ("G502", 5));
        assert_eq!(book.active, "hw:046d:c08b");
        let p = book.active_profile().unwrap();
        assert_eq!((p.first_seen, p.last_seen, p.times), (100, 100, 1));
    }

    #[test]
    fn the_same_mouse_again_changes_nothing() {
        let mut book = MouseBook::default();
        let d = dev("G502", "046d:c08b", &[3, 4, 5]);
        book.observe(std::slice::from_ref(&d), 100);
        let before = book.clone();
        let out = book.observe(&[d], 200);
        assert_eq!(out.recognition, Recognition::Same);
        assert!(!out.changed, "a look must not rewrite the file");
        assert_eq!(book, before);
    }

    #[test]
    fn swapping_mice_is_noticed_in_both_directions() {
        let mut book = MouseBook::default();
        let a = dev("Office mouse", "1111:0001", &[3, 4]);
        let b = dev("Gaming mouse", "2222:0002", &[3, 4, 5, 6, 7, 8]);
        assert_eq!(
            book.observe(std::slice::from_ref(&a), 10).recognition,
            Recognition::New
        );
        // a different mouse that was never seen
        let out = book.observe(std::slice::from_ref(&b), 20);
        assert_eq!(out.recognition, Recognition::New);
        assert_eq!(out.extra, 6);
        // back to the first one: known, but not the one in use
        let out = book.observe(std::slice::from_ref(&a), 30);
        assert_eq!(out.recognition, Recognition::Returned);
        assert_eq!(book.active, "hw:1111:0001");
        assert_eq!(book.mice.len(), 2);
        let p = book.active_profile().unwrap();
        assert_eq!((p.first_seen, p.last_seen, p.times), (10, 30, 2));
    }

    #[test]
    fn a_new_mouse_next_to_the_old_one_takes_over() {
        let mut book = MouseBook::default();
        let a = dev("A", "1:1", &[3, 4]);
        let b = dev("B", "2:2", &[3, 4, 5]);
        book.observe(std::slice::from_ref(&a), 1);
        let out = book.observe(&[a.clone(), b], 2);
        assert_eq!(out.recognition, Recognition::New);
        assert_eq!(book.active, "hw:2:2");
        // both stay connected: the one in use stays
        let out = book.observe(&[a, dev("B", "2:2", &[3, 4, 5])], 3);
        assert_eq!(out.recognition, Recognition::Same);
    }

    #[test]
    fn no_mouse_connected_keeps_the_last_one() {
        let mut book = MouseBook::default();
        book.observe(&[dev("A", "1:1", &[3, 4])], 1);
        let out = book.observe(&[], 2);
        assert_eq!(out.recognition, Recognition::Absent);
        assert!(!out.changed);
        assert_eq!(book.active, "hw:1:1");
        assert_eq!(out.extra, 2);
        // and with an empty book there is nothing to keep
        let out = MouseBook::default().observe(&[], 2);
        assert_eq!(
            (out.recognition, out.extra, out.name.as_str()),
            (Recognition::Absent, 0, "")
        );
    }

    #[test]
    fn one_model_listed_twice_is_one_mouse() {
        let mut book = MouseBook::default();
        let out = book.observe(
            &[
                dev("M720", "046d:405e", &[3, 4]),
                dev("M720", "046d:405e", &[5]),
            ],
            1,
        );
        assert_eq!(book.mice.len(), 1);
        assert_eq!(out.extra, 3);
    }

    #[test]
    fn a_scan_wins_over_what_the_os_reported() {
        let mut book = MouseBook::default();
        book.observe(&[dev("G502", "046d:c08b", &[3, 4, 5, 6, 7, 8])], 1);
        assert_eq!(book.active_profile().unwrap().buttons().len(), 6);
        // the person found only four buttons that really do something
        let n = book.record_scan(&[3, 4, 5, 6, 3, 99], 5);
        assert_eq!(n, 4);
        let p = book.active_profile().unwrap();
        assert_eq!(p.buttons(), [3, 4, 5, 6]);
        assert!(p.is_scanned());
        // the OS report is kept, and a later look does not undo the scan
        assert_eq!(p.reported.len(), 6);
        book.observe(&[dev("G502", "046d:c08b", &[3, 4, 5, 6, 7, 8])], 9);
        assert_eq!(book.active_profile().unwrap().buttons(), [3, 4, 5, 6]);
    }

    #[test]
    fn a_scan_without_a_detected_mouse_goes_to_a_generic_profile() {
        let mut book = MouseBook::default();
        assert_eq!(book.record_scan(&[3, 4, 5], 7), 3);
        assert_eq!(book.active, GENERIC_ID);
        assert_eq!(book.active_profile().unwrap().name, "Mouse");
        // a second scan reuses it
        book.record_scan(&[3], 8);
        assert_eq!(book.mice.len(), 1);
        assert_eq!(book.active_profile().unwrap().buttons(), [3]);
    }

    #[test]
    fn forgetting_the_mouse_in_use_leaves_none_in_use() {
        let mut book = MouseBook::default();
        book.observe(&[dev("A", "1:1", &[3])], 1);
        book.observe(&[dev("B", "2:2", &[3])], 2);
        book.forget("hw:2:2");
        assert_eq!(book.mice.len(), 1);
        assert!(book.active.is_empty() && book.active_profile().is_none());
        // and it is met as a new mouse the next time
        let out = book.observe(&[dev("B", "2:2", &[3])], 3);
        assert_eq!(out.recognition, Recognition::New);
    }

    #[test]
    fn the_book_keeps_a_limited_number_of_mice() {
        let mut book = MouseBook::default();
        for i in 0..(MAX_MICE as u64 + 6) {
            book.observe(&[dev("M", &format!("1:{i}"), &[3])], i + 1);
        }
        assert_eq!(book.mice.len(), MAX_MICE);
        assert!(book.active_profile().is_some(), "the mouse in use stays");
        assert!(
            book.mice.iter().all(|p| p.id != "hw:1:0"),
            "the one seen longest ago went first"
        );
    }

    #[test]
    fn a_loaded_book_is_forced_into_the_limits() {
        let json = r#"{"active":"gone","mice":[
            {"id":"a","name":"  Two\nlines  ","reported":[9,3,3,99,-1],"scanned":[]},
            {"id":"a","name":"duplicate"},
            {"id":"","name":"no id"}
        ]}"#;
        let book = serde_json::from_str::<MouseBook>(json).unwrap().sanitize();
        assert_eq!(book.version, FORMAT);
        assert_eq!(book.mice.len(), 1);
        assert_eq!(book.mice[0].name, "Two lines");
        assert_eq!(book.mice[0].reported, [3, 9]);
        assert!(book.active.is_empty(), "the mouse in use must exist");
    }

    #[test]
    fn a_file_with_fewer_fields_still_loads() {
        let book = serde_json::from_str::<MouseBook>(r#"{"mice":[{"id":"x"}]}"#).unwrap();
        assert_eq!(book.mice[0].name, "");
        assert!(book.mice[0].buttons().is_empty());
        assert_eq!(shown(""), "Mouse");
    }

    #[test]
    fn scan_keeps_the_extra_buttons_it_hears_about() {
        // built by hand: `Scan::start` switches the global flag on, which the tap tests read
        let mut scan = Scan {
            seen: BTreeSet::new(),
            last_press: Instant::now(),
        };
        for b in [4, 7, 4, 2, 0, 20] {
            scan.press(b);
        }
        assert_eq!(
            scan.pressed(),
            [4, 7],
            "no left, middle or out-of-range button"
        );
        assert!(!scan.idle());
    }
}

//! Undo / redo for the Settings editor.
//!
//! [`History`] stores whole snapshots of the edited value (here a [`Config`](crate::config::Config)
//! copy). Snapshots are simple and cannot get out of sync with the editor, which changes the
//! value from dozens of places. The value is small, and the number of steps is capped, so the
//! memory cost is bounded.
//!
//! The editor calls [`History::observe`] once per frame, after drawing. A burst of changes (typing
//! a name, dragging a slider) is **coalesced** into one step: the step is committed once the value
//! has not changed for [`SETTLE`]. Undoing while a burst is still open simply returns to the value
//! before the burst.
//!
//! The type is generic and free of egui and macOS types, so it is covered by unit tests.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// How long the value must stay unchanged before a burst of edits becomes one undo step.
pub const SETTLE: Duration = Duration::from_millis(400);

/// Snapshot-based undo / redo stacks with burst coalescing. See the module docs.
pub struct History<T> {
    /// The last committed value: what *Undo* returns to when a burst is open.
    base: T,
    /// The value seen at the last [`History::observe`].
    seen: T,
    /// `seen != base`: there is an uncommitted burst.
    dirty: bool,
    /// When `seen` last changed. Some while a burst is open.
    changed_at: Option<Instant>,
    undo: VecDeque<T>,
    redo: Vec<T>,
    limit: usize,
}

impl<T: Clone + PartialEq> History<T> {
    /// Starts a history at `initial`, keeping at most `limit` undo steps.
    pub fn new(initial: &T, limit: usize) -> Self {
        Self {
            base: initial.clone(),
            seen: initial.clone(),
            dirty: false,
            changed_at: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Forgets every step and makes `initial` the new starting point.
    pub fn reset(&mut self, initial: &T) {
        self.base = initial.clone();
        self.seen = initial.clone();
        self.dirty = false;
        self.changed_at = None;
        self.clear();
    }

    /// Drops the stored steps (the memory), keeping the current starting point.
    pub fn clear(&mut self) {
        self.undo = VecDeque::new();
        self.redo = Vec::new();
    }

    /// Reports the current value. Call it once per frame, after the widgets had their say.
    pub fn observe(&mut self, current: &T, now: Instant) {
        // an earlier burst that has settled in the meantime ends before this change is looked at
        self.commit_if_settled(now);
        if *current != self.seen {
            if self.changed_at.is_none() {
                self.redo.clear(); // a new edit makes the old "future" unreachable
            }
            self.seen = current.clone();
            self.dirty = self.seen != self.base;
            self.changed_at = Some(now);
        }
        self.commit_if_settled(now);
    }

    /// Commits the open burst if the value has not changed for [`SETTLE`].
    fn commit_if_settled(&mut self, now: Instant) {
        if let Some(at) = self.changed_at
            && now.saturating_duration_since(at) >= SETTLE
        {
            self.commit();
        }
    }

    /// Ends the open burst: the value from before it becomes an undo step.
    fn commit(&mut self) {
        self.changed_at = None;
        if self.dirty {
            let before = std::mem::replace(&mut self.base, self.seen.clone());
            self.push_undo(before);
            self.dirty = false;
        }
    }

    /// Pushes onto the undo stack, dropping the oldest step beyond the limit.
    fn push_undo(&mut self, v: T) {
        self.undo.push_back(v);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    /// How long until the open burst is committed, so the UI can schedule one more frame.
    /// None when no burst is open.
    pub fn next_commit_in(&self, now: Instant) -> Option<Duration> {
        self.changed_at
            .map(|at| SETTLE.saturating_sub(now.saturating_duration_since(at)))
    }

    /// Whether [`History::undo`] would change anything.
    pub fn can_undo(&self) -> bool {
        self.dirty || !self.undo.is_empty()
    }

    /// Whether [`History::redo`] would change anything.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Number of steps *Undo* can go back (an open burst counts as one).
    pub fn undo_steps(&self) -> usize {
        self.undo.len() + self.dirty as usize
    }

    /// Number of steps *Redo* can go forward.
    pub fn redo_steps(&self) -> usize {
        self.redo.len()
    }

    /// Steps back, writing the older value into `current`. Returns false if there is nothing to undo.
    pub fn undo(&mut self, current: &mut T) -> bool {
        if self.dirty {
            // an open burst: go back to the value before it
            self.redo.push(self.seen.clone());
            self.restore(current, self.base.clone());
            return true;
        }
        let Some(prev) = self.undo.pop_back() else {
            return false;
        };
        self.redo
            .push(std::mem::replace(&mut self.base, prev.clone()));
        self.restore(current, prev);
        true
    }

    /// Steps forward again, writing the newer value into `current`. Returns false if there is
    /// nothing to redo.
    pub fn redo(&mut self, current: &mut T) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        let before = std::mem::replace(&mut self.base, next.clone());
        self.push_undo(before);
        self.restore(current, next);
        true
    }

    /// Makes `value` both the current and the seen value, with no burst open.
    fn restore(&mut self, current: &mut T, value: T) {
        self.seen = value.clone();
        *current = value;
        self.dirty = false;
        self.changed_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake clock so the tests do not sleep.
    struct Clock(Instant);
    impl Clock {
        fn new() -> Self {
            Self(Instant::now())
        }
        fn at(&self, ms: u64) -> Instant {
            self.0 + Duration::from_millis(ms)
        }
    }

    #[test]
    fn a_settled_edit_becomes_one_undo_step() {
        let t = Clock::new();
        let mut v = 1;
        let mut h = History::new(&v, 10);
        v = 2;
        h.observe(&v, t.at(0));
        assert!(h.can_undo(), "an open burst can already be undone");
        h.observe(&v, t.at(500));
        assert_eq!(h.undo_steps(), 1);
        assert!(h.undo(&mut v));
        assert_eq!(v, 1);
        assert!(!h.can_undo());
        assert!(!h.undo(&mut v));
    }

    #[test]
    fn a_burst_of_edits_is_one_step() {
        let t = Clock::new();
        let mut v = 0;
        let mut h = History::new(&v, 10);
        for (i, x) in [1, 2, 3, 4].into_iter().enumerate() {
            v = x;
            h.observe(&v, t.at(i as u64 * 50)); // keystrokes 50 ms apart
        }
        h.observe(&v, t.at(1000));
        assert_eq!(h.undo_steps(), 1);
        assert!(h.undo(&mut v));
        assert_eq!(v, 0, "the whole burst is undone at once");
    }

    #[test]
    fn undo_in_the_middle_of_a_burst_goes_back_to_before_it() {
        let t = Clock::new();
        let mut v = 10;
        let mut h = History::new(&v, 10);
        v = 11;
        h.observe(&v, t.at(0));
        h.observe(&v, t.at(1000)); // committed: 10 -> 11
        v = 12;
        h.observe(&v, t.at(1100)); // a new burst is open
        assert!(h.undo(&mut v));
        assert_eq!(v, 11, "only the open burst is undone");
        assert!(h.redo(&mut v));
        assert_eq!(v, 12);
    }

    #[test]
    fn redo_walks_forward_and_a_new_edit_discards_it() {
        let t = Clock::new();
        let mut v = 0;
        let mut h = History::new(&v, 10);
        for (i, x) in [1, 2, 3].into_iter().enumerate() {
            v = x;
            h.observe(&v, t.at(i as u64 * 1000));
        }
        h.observe(&v, t.at(5000));
        assert!(h.undo(&mut v) && h.undo(&mut v));
        assert_eq!(v, 1);
        assert_eq!(h.redo_steps(), 2);
        assert!(h.redo(&mut v));
        assert_eq!(v, 2);
        v = 9; // a new edit
        h.observe(&v, t.at(6000));
        assert!(!h.can_redo());
        h.observe(&v, t.at(7000));
        assert!(h.undo(&mut v));
        assert_eq!(v, 2);
    }

    #[test]
    fn editing_back_to_the_start_adds_no_step() {
        let t = Clock::new();
        let mut v = 5;
        let mut h = History::new(&v, 10);
        v = 6;
        h.observe(&v, t.at(0));
        v = 5;
        h.observe(&v, t.at(100));
        h.observe(&v, t.at(1000));
        assert!(!h.can_undo());
    }

    #[test]
    fn the_oldest_steps_are_dropped_beyond_the_limit() {
        let t = Clock::new();
        let mut v = 0;
        let mut h = History::new(&v, 3);
        for i in 1..=6u64 {
            v = i as i32;
            h.observe(&v, t.at(i * 1000));
        }
        h.observe(&v, t.at(10_000));
        assert_eq!(h.undo_steps(), 3);
        while h.undo(&mut v) {}
        assert_eq!(v, 3, "steps older than the limit are gone");
    }

    #[test]
    fn reset_forgets_everything() {
        let t = Clock::new();
        let mut v = 0;
        let mut h = History::new(&v, 10);
        v = 1;
        h.observe(&v, t.at(0));
        h.observe(&v, t.at(1000));
        h.reset(&v);
        assert!(!h.can_undo() && !h.can_redo());
        assert!(h.next_commit_in(t.at(2000)).is_none());
    }

    #[test]
    fn next_commit_in_counts_down_during_a_burst() {
        let t = Clock::new();
        let mut v = 0;
        let mut h = History::new(&v, 10);
        assert!(h.next_commit_in(t.at(0)).is_none());
        v = 1;
        h.observe(&v, t.at(100));
        assert_eq!(
            h.next_commit_in(t.at(300)),
            Some(SETTLE - Duration::from_millis(200))
        );
        assert_eq!(h.next_commit_in(t.at(900)), Some(Duration::ZERO));
    }
}

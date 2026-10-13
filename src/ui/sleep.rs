//! When the UI may go to sleep: the idle policy that frees caches and Settings state.
//!
//! The window is hidden almost all the time, but memory held by the Settings editor and by decoded
//! icons stays allocated until something frees it. [`Sleeper`] decides *when*: after the UI has
//! been idle (no pie, no Settings, no preview) for `Config::sleep_after_s` seconds it reports
//! [`Tick::Sleep`] once. What is freed, and how the UI wakes up again, is described in
//! [`guides::architecture`](crate::guides::architecture#idle-sleep-and-caches).
//!
//! The policy is a plain state machine over [`Instant`]s with no egui or macOS types, so it is
//! covered by unit tests.

use std::time::{Duration, Instant};

/// What the caller should do after [`Sleeper::tick`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tick {
    /// Nothing to do.
    Stay,
    /// The idle time is over: free everything that can be rebuilt. Reported once per idle period.
    Sleep,
}

/// Tracks how long the UI has been idle. See the module docs.
#[derive(Debug)]
pub struct Sleeper {
    /// When the current idle period started. None while the UI is busy.
    idle_since: Option<Instant>,
    asleep: bool,
}

/// `Config::sleep_after_s` as a duration; 0 means "never sleep".
pub fn sleep_after(secs: u64) -> Option<Duration> {
    (secs > 0).then(|| Duration::from_secs(secs))
}

impl Sleeper {
    /// A sleeper that is awake and idle since `now` (the app starts hidden).
    pub fn new(now: Instant) -> Self {
        Self {
            idle_since: Some(now),
            asleep: false,
        }
    }

    /// Whether [`Tick::Sleep`] has been reported and the UI has not been busy since.
    pub fn is_asleep(&self) -> bool {
        self.asleep
    }

    /// Counts as activity without a window being shown (the trigger was pressed, so a pie may open
    /// any moment): restarts the idle clock and wakes up if asleep.
    pub fn poke(&mut self, now: Instant) {
        self.idle_since = Some(now);
        self.asleep = false;
    }

    /// Reports the current state. `busy` = a pie, Settings or a preview is showing. `after` is the
    /// idle time that triggers sleeping (None = never). Call it every frame and after hiding.
    pub fn tick(&mut self, now: Instant, busy: bool, after: Option<Duration>) -> Tick {
        if busy {
            self.idle_since = None;
            self.asleep = false;
            return Tick::Stay;
        }
        let since = *self.idle_since.get_or_insert(now);
        match after {
            Some(limit) if !self.asleep && now.saturating_duration_since(since) >= limit => {
                self.asleep = true;
                Tick::Sleep
            }
            _ => Tick::Stay,
        }
    }

    /// How long until [`Sleeper::tick`] would report [`Tick::Sleep`], so the UI can ask egui for
    /// one frame at that time. None when no timer is needed (busy, already asleep, or disabled).
    pub fn next_check(&self, now: Instant, after: Option<Duration>) -> Option<Duration> {
        let (since, limit) = (self.idle_since?, after?);
        if self.asleep {
            return None;
        }
        Some(limit.saturating_sub(now.saturating_duration_since(since)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AFTER: Option<Duration> = Some(Duration::from_secs(60));

    fn secs(t: Instant, s: u64) -> Instant {
        t + Duration::from_secs(s)
    }

    #[test]
    fn sleeps_once_after_the_idle_time() {
        let t0 = Instant::now();
        let mut s = Sleeper::new(t0);
        assert_eq!(s.tick(secs(t0, 59), false, AFTER), Tick::Stay);
        assert_eq!(s.tick(secs(t0, 60), false, AFTER), Tick::Sleep);
        assert!(s.is_asleep());
        assert_eq!(s.tick(secs(t0, 120), false, AFTER), Tick::Stay, "only once");
    }

    #[test]
    fn being_busy_restarts_the_clock_and_wakes_up() {
        let t0 = Instant::now();
        let mut s = Sleeper::new(t0);
        assert_eq!(s.tick(secs(t0, 70), false, AFTER), Tick::Sleep);
        assert_eq!(s.tick(secs(t0, 71), true, AFTER), Tick::Stay);
        assert!(!s.is_asleep());
        // idle again from t0 + 80
        assert_eq!(s.tick(secs(t0, 80), false, AFTER), Tick::Stay);
        assert_eq!(s.tick(secs(t0, 139), false, AFTER), Tick::Stay);
        assert_eq!(s.tick(secs(t0, 140), false, AFTER), Tick::Sleep);
    }

    #[test]
    fn a_poke_restarts_the_idle_clock_and_wakes_up() {
        let t0 = Instant::now();
        let mut s = Sleeper::new(t0);
        assert_eq!(s.tick(secs(t0, 70), false, AFTER), Tick::Sleep);
        s.poke(secs(t0, 100));
        assert!(!s.is_asleep());
        assert_eq!(s.tick(secs(t0, 159), false, AFTER), Tick::Stay);
        assert_eq!(s.tick(secs(t0, 160), false, AFTER), Tick::Sleep);
    }

    #[test]
    fn zero_means_never() {
        let t0 = Instant::now();
        let mut s = Sleeper::new(t0);
        assert_eq!(sleep_after(0), None);
        assert_eq!(s.tick(secs(t0, 100_000), false, sleep_after(0)), Tick::Stay);
        assert_eq!(s.next_check(secs(t0, 5), None), None);
    }

    #[test]
    fn next_check_counts_down_and_stops() {
        let t0 = Instant::now();
        let mut s = Sleeper::new(t0);
        assert_eq!(
            s.next_check(secs(t0, 20), AFTER),
            Some(Duration::from_secs(40))
        );
        assert_eq!(s.next_check(secs(t0, 90), AFTER), Some(Duration::ZERO));
        s.tick(secs(t0, 90), false, AFTER);
        assert_eq!(s.next_check(secs(t0, 91), AFTER), None, "asleep: no timer");
        s.tick(secs(t0, 92), true, AFTER);
        assert_eq!(s.next_check(secs(t0, 93), AFTER), None, "busy: no timer");
    }

    #[test]
    fn the_clock_starts_when_the_ui_becomes_idle() {
        let t0 = Instant::now();
        let mut s = Sleeper::new(t0);
        s.tick(secs(t0, 1), true, AFTER); // settings open for a long time
        assert_eq!(
            s.tick(secs(t0, 500), false, AFTER),
            Tick::Stay,
            "idle starts at 500"
        );
        assert_eq!(s.tick(secs(t0, 560), false, AFTER), Tick::Sleep);
    }
}

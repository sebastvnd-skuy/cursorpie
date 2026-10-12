//! Which pie do the side buttons follow?
//!
//! An app with a profile of its own has two pies: its own and the general one. The side buttons
//! (back / forward) follow exactly one of them, the *owner*:
//!
//! - by default the app's own pie (`Config::side_default`),
//! - or the pie you chose: by picking a side-button mode in it (`Config::side_adopt`), by
//!   right-clicking while the pie is open, or from the menu bar.
//!
//! The choice is remembered per app, in memory only. The pie ring shows who the owner is
//! (accent color = this pie, red = the other one).
//!
//! Lock order: always the config first, then this module's state. Functions here that need the
//! config take it as a parameter, so the caller already holds it.

use crate::config::{Config, SideTarget, lock};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Live, in-memory state. Lock order: config first, then this.
struct State {
    /// Bundle id of the app the current gesture is about (set when a gesture starts).
    target: Option<String>,
    /// What the user chose, per app.
    choice: BTreeMap<String, SideTarget>,
}

/// The one instance; see [`State`].
static STATE: Mutex<State> = Mutex::new(State {
    target: None,
    choice: BTreeMap::new(),
});

/// Where the two pies stand for one app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub general: usize,
    /// The app's own profile. None: the app has none, so there is only the general pie.
    pub app: Option<usize>,
    /// The profile the side buttons follow.
    pub owner: usize,
}

/// What the pie ring needs to know.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// Show the indicator at all (the app has two pies and the indicator is switched on).
    pub shown: bool,
    /// This pie is the one the side buttons follow.
    pub owner: bool,
}

// ------------------------------------------------------------------ pure part

/// `choice` is what the user picked for this app (None: nothing yet, use the default).
pub fn resolve(c: &Config, bundle: Option<&str>, choice: Option<SideTarget>) -> Resolved {
    let general = c.global_profile();
    let app = c.app_profile_for(bundle);
    let owner = match app {
        None => general,
        Some(a) => match choice.unwrap_or(c.side_default) {
            SideTarget::App => a,
            SideTarget::General => general,
        },
    };
    Resolved {
        general,
        app,
        owner,
    }
}

/// The other pie: the new choice and the profile that owns the side buttons after it.
/// None if the app has no pie of its own (nothing to switch between).
fn toggled(c: &Config, bundle: &str, choice: Option<SideTarget>) -> Option<(SideTarget, usize)> {
    let r = resolve(c, Some(bundle), choice);
    let app = r.app?;
    if r.owner == app {
        Some((SideTarget::General, r.general))
    } else {
        Some((SideTarget::App, app))
    }
}

// ----------------------------------------------------------------- live state

/// What the user chose for `bundle`, if anything.
fn choice_for(bundle: Option<&str>) -> Option<SideTarget> {
    let b = bundle?;
    lock(&STATE).choice.get(b).copied()
}

/// Remembers the app the current gesture is about. With `forget_on_switch`, moving on to another
/// app also drops what was chosen before.
pub fn set_target(bundle: Option<String>, forget_on_switch: bool) {
    let mut s = lock(&STATE);
    if forget_on_switch && s.target != bundle {
        s.choice.clear();
    }
    s.target = bundle;
}

/// Bundle id of the app the current gesture is about.
pub fn target() -> Option<String> {
    lock(&STATE).target.clone()
}

/// Drops every choice (used after Settings are saved, so a changed default applies at once).
pub fn forget_all() {
    lock(&STATE).choice.clear();
}

/// The profile the side buttons follow in `bundle`.
pub fn owner(c: &Config, bundle: Option<&str>) -> usize {
    resolve(c, bundle, choice_for(bundle)).owner
}

/// How the ring of `profile` should look right now.
pub fn view_of(c: &Config, profile: usize) -> View {
    if !c.side_indicator {
        return View::default();
    }
    let (target, choice) = {
        let s = lock(&STATE);
        let choice = s.target.as_ref().and_then(|b| s.choice.get(b).copied());
        (s.target.clone(), choice)
    };
    let r = resolve(c, target.as_deref(), choice);
    View {
        shown: r.app.is_some() && (profile == r.general || Some(profile) == r.app),
        owner: profile == r.owner,
    }
}

/// Switches which pie the side buttons follow in `bundle`. Returns the profile that owns them now,
/// or None if the app has no pie of its own.
pub fn toggle(c: &Config, bundle: Option<&str>) -> Option<usize> {
    let b = bundle?;
    let mut s = lock(&STATE);
    let current = s.choice.get(b).copied();
    let (next, owner) = toggled(c, b, current)?;
    s.choice.insert(b.to_string(), next);
    Some(owner)
}

/// A side-button mode was picked in `profile`: from now on the side buttons follow that pie.
pub fn adopt(c: &Config, bundle: Option<&str>, profile: usize) {
    let Some(b) = bundle else { return };
    let r = resolve(c, Some(b), None);
    let Some(app) = r.app else { return };
    let next = if profile == app {
        SideTarget::App
    } else if profile == r.general {
        SideTarget::General
    } else {
        return;
    };
    lock(&STATE).choice.insert(b.to_string(), next);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Item, Profile};

    const PS: &str = "com.adobe.Photoshop";

    fn profile(name: &str, apps: &[&str]) -> Profile {
        Profile {
            name: name.into(),
            apps: apps.iter().map(|s| s.to_string()).collect(),
            items: vec![Item::submenu("x", "x", Vec::new())],
            active: vec![],
        }
    }

    /// 0 = general, 1 = Photoshop
    fn two_pies() -> Config {
        Config {
            profiles: vec![profile("General", &[]), profile("Photoshop", &[PS])],
            ..Config::default()
        }
    }

    #[test]
    fn app_without_own_pie_always_uses_general() {
        let c = two_pies();
        for choice in [None, Some(SideTarget::App), Some(SideTarget::General)] {
            let r = resolve(&c, Some("com.apple.finder"), choice);
            assert_eq!((r.general, r.app, r.owner), (0, None, 0));
        }
        assert_eq!(resolve(&c, None, None).owner, 0);
    }

    #[test]
    fn app_with_own_pie_defaults_to_it() {
        let c = two_pies();
        let r = resolve(&c, Some(PS), None);
        assert_eq!((r.general, r.app, r.owner), (0, Some(1), 1));
    }

    #[test]
    fn choice_beats_default_and_default_applies_without_choice() {
        let mut c = two_pies();
        assert_eq!(resolve(&c, Some(PS), Some(SideTarget::General)).owner, 0);

        c.side_default = SideTarget::General;
        assert_eq!(resolve(&c, Some(PS), None).owner, 0);
        assert_eq!(resolve(&c, Some(PS), Some(SideTarget::App)).owner, 1);
    }

    #[test]
    fn toggled_flips_between_the_two_pies() {
        let c = two_pies();
        assert_eq!(toggled(&c, PS, None), Some((SideTarget::General, 0)));
        assert_eq!(
            toggled(&c, PS, Some(SideTarget::General)),
            Some((SideTarget::App, 1))
        );
        assert_eq!(
            toggled(&c, PS, Some(SideTarget::App)),
            Some((SideTarget::General, 0))
        );
        // follows the default when nothing was chosen yet
        let mut c = two_pies();
        c.side_default = SideTarget::General;
        assert_eq!(toggled(&c, PS, None), Some((SideTarget::App, 1)));
    }

    #[test]
    fn nothing_to_toggle_without_an_own_pie() {
        let c = two_pies();
        assert_eq!(toggled(&c, "com.apple.finder", None), None);
    }

    /// One test for everything that touches the global state, so parallel tests cannot interfere.
    #[test]
    fn live_state_follows_choices_per_app() {
        let c = two_pies();
        let (a, b) = (PS, "com.apple.finder");

        // adopt: picking a mode in the general pie hands the side buttons to it, only for that app
        assert_eq!(owner(&c, Some(a)), 1);
        adopt(&c, Some(a), 0);
        assert_eq!(owner(&c, Some(a)), 0);
        assert_eq!(owner(&c, Some(b)), 0);
        adopt(&c, Some(a), 1);
        assert_eq!(owner(&c, Some(a)), 1);

        // toggle: switches and reports the new owner; no own pie = nothing to switch
        assert_eq!(toggle(&c, Some(a)), Some(0));
        assert_eq!(owner(&c, Some(a)), 0);
        assert_eq!(toggle(&c, Some(a)), Some(1));
        assert_eq!(toggle(&c, Some(b)), None);
        assert_eq!(toggle(&c, None), None);

        // adopting a profile that is neither pie of this app changes nothing
        adopt(&c, Some(a), 7);
        assert_eq!(owner(&c, Some(a)), 1);

        // indicator
        set_target(Some(a.to_string()), false);
        assert_eq!(
            view_of(&c, 1),
            View {
                shown: true,
                owner: true
            }
        );
        assert_eq!(
            view_of(&c, 0),
            View {
                shown: true,
                owner: false
            }
        );
        adopt(&c, Some(a), 0);
        assert_eq!(
            view_of(&c, 0),
            View {
                shown: true,
                owner: true
            }
        );
        assert_eq!(
            view_of(&c, 1),
            View {
                shown: true,
                owner: false
            }
        );
        let off = Config {
            side_indicator: false,
            ..two_pies()
        };
        assert_eq!(view_of(&off, 0), View::default());
        set_target(Some(b.to_string()), false);
        assert!(
            !view_of(&c, 0).shown,
            "an app with a single pie needs no indicator"
        );

        // forget_all and forget-on-switch
        adopt(&c, Some(a), 0);
        forget_all();
        assert_eq!(owner(&c, Some(a)), 1);
        set_target(Some(a.to_string()), false);
        adopt(&c, Some(a), 0);
        set_target(Some(a.to_string()), true);
        assert_eq!(owner(&c, Some(a)), 0, "same app again keeps the choice");
        set_target(Some(b.to_string()), true);
        assert_eq!(owner(&c, Some(a)), 1, "switching apps forgets it");
        forget_all();
    }
}

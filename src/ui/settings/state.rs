//! Editor-only state: which tab is open, which profile is selected, which
//! shortcut slot is being recorded, and the icon texture cache.
//!
//! The texture cache lives here because both the pie and Settings reach it through
//! `App::ed`; it is an [`IconCache`] (bounded, kept across Settings sessions).

use crate::config::backup::Backup;
use crate::config::{Config, Item, blank_items, items_at, presets, quick_actions};
use crate::tap::RECORDING;
use crate::ui::icons::IconCache;
use eframe::egui;
use std::sync::atomic::Ordering;

/// The tabs of the Settings window.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Menu,
    Behavior,
    General,
    About,
}

/// Which action field of an item is being recorded.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Back,
    Forward,
    Run,
}

/// The action field currently recording a shortcut.
pub struct Rec {
    pub profile: usize,
    pub path: Vec<usize>,
    pub slot: Slot,
}

/// The ready-made slices of the "Add a slice" panel, grouped under a title.
///
/// Built when the panel opens and dropped when it closes (see `Ed::add_catalog`): the panel is
/// redrawn every frame, and rebuilding these lists with all their strings that often is wasted work.
pub struct AddCatalog {
    /// Whether the groups were built for a ring that may hold a submenu.
    pub can_nest: bool,
    pub groups: Vec<(&'static str, Vec<Item>)>,
}

impl AddCatalog {
    /// The groups of the panel; `can_nest` = the ring is not yet at the maximum depth.
    pub fn new(can_nest: bool) -> Self {
        Self {
            can_nest,
            groups: vec![
                ("Quick actions", quick_actions()),
                ("Side buttons", presets()),
                ("Custom", blank_items(can_nest)),
            ],
        }
    }
}

/// Editor-only state (kept apart from the edited config so both can be borrowed at once).
pub struct Ed {
    pub tab: Tab,
    pub profile: usize,
    pub recording: Option<Rec>,
    pub icons: IconCache,
    pub new_app: String,
    /// the "New profile" form: name and the app the profile belongs to
    pub new_profile_name: String,
    pub new_profile_app: String,
    pub nav: Vec<usize>,
    pub sel: Option<usize>,
    pub adding: bool,
    /// the tiles of the "Add a slice" panel; `Some` only while the panel is showing
    pub add_catalog: Option<AddCatalog>,
    /// the icon picker is open (it temporarily replaces the inspector)
    pub picking_icon: bool,
    pub icon_query: String,
    /// 0 = all, 1..=N = category, N+1 = the user's own icons
    pub icon_cat: usize,
    pub user_icons: Vec<String>,
    /// images in the icons folder that `user_icons` leaves out (too big, or over the limit)
    pub icons_not_listed: usize,
    /// backups of `config.json` found on disk (read when the General tab is opened)
    pub backups: Vec<Backup>,
}

impl Default for Ed {
    fn default() -> Self {
        Self::new()
    }
}

impl Ed {
    /// Fresh editor state: Menu tab, first profile, nothing selected.
    pub fn new() -> Self {
        Self {
            tab: Tab::Menu,
            profile: 0,
            recording: None,
            icons: IconCache::new(),
            new_app: String::new(),
            new_profile_name: String::new(),
            new_profile_app: String::new(),
            nav: Vec::new(),
            sel: None,
            adding: false,
            add_catalog: None,
            picking_icon: false,
            icon_query: String::new(),
            icon_cat: 0,
            user_icons: Vec::new(),
            icons_not_listed: 0,
            backups: Vec::new(),
        }
    }

    /// Reads the icons folder again (the "Mine" page): the images to show and how many were left out.
    pub fn reload_user_icons(&mut self) {
        let found = super::icon_catalog::scan_user_icons();
        self.user_icons = found.icons;
        self.icons_not_listed = found.not_listed;
    }

    /// Stops recording a shortcut (also disarms the event tap).
    pub fn stop_recording(&mut self) {
        RECORDING.store(false, Ordering::SeqCst);
        self.recording = None;
    }

    /// Cached texture for an image icon. None for text icons or images that fail to load.
    pub fn icon_for(&mut self, ctx: &egui::Context, icon: &str) -> Option<egui::TextureHandle> {
        self.icons.get(ctx, icon)
    }

    /// Makes the selection valid for `edit` again after the config was replaced from outside the
    /// editor (undo, redo, restoring a backup): clamps the profile, leaves a ring that no longer
    /// exists and drops a stale slice selection.
    pub fn fit_to(&mut self, edit: &Config) {
        self.stop_recording();
        self.profile = self.profile.min(edit.profiles.len().saturating_sub(1));
        let ring_len = edit
            .profiles
            .get(self.profile)
            .and_then(|p| items_at(&p.items, &self.nav))
            .map(<[_]>::len);
        match ring_len {
            None => self.reset_nav(),
            Some(n) => {
                if self.sel.is_some_and(|i| i >= n) {
                    self.sel = None;
                }
            }
        }
    }

    /// Back to the top ring with nothing selected.
    pub fn reset_nav(&mut self) {
        self.nav.clear();
        self.sel = None;
        self.adding = false;
        self.add_catalog = None;
        self.picking_icon = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiles(c: &AddCatalog) -> usize {
        c.groups.iter().map(|(_, list)| list.len()).sum()
    }

    #[test]
    fn the_add_catalog_offers_a_submenu_only_when_nesting_is_allowed() {
        assert_eq!(
            tiles(&AddCatalog::new(true)),
            tiles(&AddCatalog::new(false)) + 1
        );
    }

    #[test]
    fn leaving_the_ring_drops_the_add_catalog() {
        let mut ed = Ed::new();
        ed.adding = true;
        ed.add_catalog = Some(AddCatalog::new(true));
        ed.reset_nav();
        assert!(!ed.adding && ed.add_catalog.is_none());
    }
}

//! Editor-only state: which tab is open, which profile is selected, which
//! shortcut slot is being recorded, and the icon texture cache.

use crate::tap::RECORDING;
use crate::ui::appicon;
use crate::ui::draw::*;
use eframe::egui;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// The tabs of the Settings window.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Menu,
    Behavior,
    General,
    About,
}

/// How long to wait before trying again to load an app icon that could not be loaded.
const APP_ICON_RETRY: Duration = Duration::from_secs(3);

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

/// Editor-only state (kept apart from the edited config so both can be borrowed at once).
pub struct Ed {
    pub tab: Tab,
    pub profile: usize,
    pub recording: Option<Rec>,
    pub icons: HashMap<String, Option<egui::TextureHandle>>,
    /// when loading an `app:` icon last failed (it is tried again after `APP_ICON_RETRY`)
    pub icon_failed: HashMap<String, Instant>,
    pub new_app: String,
    pub nav: Vec<usize>,
    pub sel: Option<usize>,
    pub adding: bool,
    /// the icon picker is open (it temporarily replaces the inspector)
    pub picking_icon: bool,
    pub icon_query: String,
    /// 0 = all, 1..=N = category, N+1 = the user's own icons
    pub icon_cat: usize,
    pub user_icons: Vec<String>,
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
            icons: HashMap::new(),
            icon_failed: HashMap::new(),
            new_app: String::new(),
            nav: Vec::new(),
            sel: None,
            adding: false,
            picking_icon: false,
            icon_query: String::new(),
            icon_cat: 0,
            user_icons: Vec::new(),
        }
    }

    /// Stops recording a shortcut (also disarms the event tap).
    pub fn stop_recording(&mut self) {
        RECORDING.store(false, Ordering::SeqCst);
        self.recording = None;
    }

    /// Cached texture for an image icon. None for text icons or images that fail to load.
    pub fn icon_for(&mut self, ctx: &egui::Context, icon: &str) -> Option<egui::TextureHandle> {
        // "app:<bundle id>" = the real icon of an app, asked from macOS (see `appicon`)
        if let Some(bundle) = icon.strip_prefix(appicon::PREFIX) {
            if let Some(cached) = self.icons.get(icon) {
                // a failure is not final: macOS may simply not have had the icon ready. Try again
                // after a pause, but not on every frame (decoding is not free).
                let retry = cached.is_none()
                    && self
                        .icon_failed
                        .get(icon)
                        .is_none_or(|at| at.elapsed() >= APP_ICON_RETRY);
                if !retry {
                    return cached.clone();
                }
            }
            let tex = appicon::load(bundle)
                .map(|img| ctx.load_texture(icon, img, egui::TextureOptions::LINEAR));
            if tex.is_some() {
                self.icon_failed.remove(icon);
            } else {
                self.icon_failed.insert(icon.to_string(), Instant::now());
            }
            self.icons.insert(icon.to_string(), tex.clone());
            return tex;
        }
        let path = icon_path(icon)?;
        let key = path.to_string_lossy().to_string();
        if let Some(t) = self.icons.get(&key) {
            return t.clone();
        }
        let tex = load_icon(ctx, &path, &key);
        self.icons.insert(key, tex.clone());
        tex
    }

    /// Back to the top ring with nothing selected.
    pub fn reset_nav(&mut self) {
        self.nav.clear();
        self.sel = None;
        self.adding = false;
        self.picking_icon = false;
    }
}

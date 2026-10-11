//! Editor-only state: which tab is open, which profile is selected, which
//! shortcut slot is being recorded, and the icon texture cache.

use crate::gfx::*;
use crate::tap::RECORDING;
use eframe::egui;
use std::collections::HashMap;
use std::sync::atomic::Ordering;

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
    pub new_app: String,
    pub nav: Vec<usize>,
    pub sel: Option<usize>,
    pub adding: bool,
    /// panel pemilih ikon sedang terbuka (menggantikan inspector)
    pub picking_icon: bool,
    pub icon_query: String,
    /// 0 = semua, 1..=N = kategori, N+1 = ikon milik pengguna
    pub icon_cat: usize,
    pub user_icons: Vec<String>,
}

impl Ed {
    pub fn new() -> Self {
        Self {
            tab: Tab::Menu,
            profile: 0,
            recording: None,
            icons: HashMap::new(),
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

    pub fn stop_recording(&mut self) {
        RECORDING.store(false, Ordering::SeqCst);
        self.recording = None;
    }

    /// Cached texture for an image icon. None for text icons or images that fail to load.
    pub fn icon_for(&mut self, ctx: &egui::Context, icon: &str) -> Option<egui::TextureHandle> {
        let path = icon_path(icon)?;
        let key = path.to_string_lossy().to_string();
        if let Some(t) = self.icons.get(&key) {
            return t.clone();
        }
        let tex = load_icon(ctx, &path, &key);
        self.icons.insert(key, tex.clone());
        tex
    }

    pub fn reset_nav(&mut self) {
        self.nav.clear();
        self.sel = None;
        self.adding = false;
        self.picking_icon = false;
    }
}

//! Validation: forces a freshly loaded or edited [`Config`] back into the limits the app relies on.

use super::{
    CONFIG_VERSION, Config, Item, ItemKind, MAX_DEPTH, MAX_ITEMS, MAX_PROFILES, item_at, presets,
};

/// Path of the first `Mode` slice in depth-first order, if there is one.
fn first_mode(items: &[Item]) -> Option<Vec<usize>> {
    for (i, it) in items.iter().enumerate() {
        match &it.kind {
            ItemKind::Mode { .. } => return Some(vec![i]),
            ItemKind::Submenu(c) => {
                if let Some(mut p) = first_mode(c) {
                    p.insert(0, i);
                    return Some(p);
                }
            }
            _ => {}
        }
    }
    None
}

/// Trims a ring to `MAX_ITEMS` slices and empties submenus nested deeper than `MAX_DEPTH`.
fn clean_items(items: &mut Vec<Item>, depth: usize) {
    items.truncate(MAX_ITEMS);
    for it in items.iter_mut() {
        if let ItemKind::Submenu(c) = &mut it.kind {
            if depth + 1 >= MAX_DEPTH {
                c.clear();
            } else {
                clean_items(c, depth + 1);
            }
        }
    }
}

impl Config {
    /// Forces the config into the limits the rest of the program relies on (see
    /// [`guides::config_format`](crate::guides::config_format#validation) for the list).
    /// Called for every config that is loaded, imported or saved.
    pub fn sanitize(mut self) -> Self {
        self.version = CONFIG_VERSION;
        if self.profiles.is_empty() {
            self.profiles = Config::default().profiles;
        }
        self.profiles.truncate(MAX_PROFILES);
        self.double_click_ms = self.double_click_ms.clamp(150, 500);
        for p in &mut self.profiles {
            if p.items.is_empty() {
                p.items = vec![presets().remove(0)];
            }
            clean_items(&mut p.items, 0);
            let ok = matches!(
                item_at(&p.items, &p.active).map(|i| &i.kind),
                Some(ItemKind::Mode { .. })
            );
            if !ok {
                p.active = first_mode(&p.items).unwrap_or_default();
            }
        }
        self.pie_hold_ms = self.pie_hold_ms.clamp(150, 2000);
        self.settings_hold_ms = self.settings_hold_ms.max(self.pie_hold_ms + 300);
        self.pie_size = self.pie_size.clamp(140.0, 360.0);
        self.preview_ms = self.preview_ms.min(3000);
        if !matches!(self.trigger_button, 2 | 5 | 6 | 7) {
            self.trigger_button = 2;
        }
        self
    }
}

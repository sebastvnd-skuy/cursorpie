//! Validation: forces a freshly loaded or edited [`Config`] back into the limits the app relies on.

use super::{
    CONFIG_VERSION, Config, Item, ItemKind, MAX_DEPTH, MAX_ITEMS, MAX_PROFILES, item_at, presets,
    share::clean_apps,
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
        match &mut it.kind {
            ItemKind::Submenu(c) => {
                if depth + 1 >= MAX_DEPTH {
                    c.clear();
                } else {
                    clean_items(c, depth + 1);
                }
            }
            // back and forward have fields of their own; only real extra buttons are kept
            ItemKind::Mode { extra, .. } => extra.retain(|b, _| {
                crate::mouse::is_extra(*b)
                    && *b != crate::mouse::BACK
                    && *b != crate::mouse::FORWARD
            }),
            ItemKind::Run(_) => {}
        }
    }
}

impl Config {
    /// Forces the config into the limits the rest of the program relies on (see
    /// [`guides::config_format`](crate::guides::config_format#validation) for the list).
    /// Called for every config that is loaded, imported or saved.
    pub fn sanitize(mut self) -> Self {
        self.version = CONFIG_VERSION;
        // Only the first profile without apps is kept (the general one). A second one could never
        // be chosen: profiles are picked by app, and the fallback is the first without apps.
        let mut have_general = false;
        for p in &mut self.profiles {
            p.apps = clean_apps(&p.apps);
        }
        self.profiles.retain(|p| {
            let extra_general = p.is_general() && have_general;
            have_general |= p.is_general();
            !extra_general
        });
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
        if self.sleep_after_s != 0 {
            self.sleep_after_s = self.sleep_after_s.clamp(10, 3600);
        }
        if !matches!(self.trigger_button, 2 | 5 | 6 | 7) {
            self.trigger_button = 2;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Config {
        crate::config::parse_config(json).expect("valid config")
    }

    #[test]
    fn timings_and_sizes_are_clamped() {
        let c = parse(
            r#"{"version":2,"pie_hold_ms":1,"settings_hold_ms":1,"double_click_ms":9999,"pie_size":10}"#,
        );
        assert_eq!(c.pie_hold_ms, 150);
        assert_eq!(c.settings_hold_ms, 450, "at least 300 ms after the pie");
        assert_eq!(c.double_click_ms, 500);
        assert_eq!(c.pie_size, 140.0);
        let big = parse(r#"{"version":2,"pie_hold_ms":99999,"pie_size":9999}"#);
        assert_eq!(big.pie_hold_ms, 2000);
        assert_eq!(big.pie_size, 360.0);
    }

    #[test]
    fn extra_buttons_of_a_mode_are_kept_and_cleaned() {
        let c = parse(
            r#"{"version":2,"profiles":[{"name":"G","apps":[],"active":[0],"items":[
                {"name":"M","icon":"M","kind":{"Mode":{"back":"Nothing","forward":"Nothing",
                 "extra":{"5":{"OpenUrl":"a"},"3":{"OpenUrl":"b"},"2":{"OpenUrl":"c"},"99":"Nothing","7":"Nothing"}}}}
            ]}]}"#,
        );
        let ItemKind::Mode { extra, .. } = &c.profiles[0].items[0].kind else {
            panic!("not a mode");
        };
        let keys: Vec<i64> = extra.keys().copied().collect();
        assert_eq!(
            keys,
            [5, 7],
            "back, middle and out-of-range buttons are dropped"
        );
        // only buttons with an action are taken from the apps
        let used: Vec<i64> = crate::config::extra_buttons_in_use(&c)
            .into_iter()
            .collect();
        assert_eq!(used, [5]);
        // and a file written by an older version has none
        let old = parse(
            r#"{"version":2,"profiles":[{"name":"G","apps":[],"active":[0],"items":[
                {"name":"M","icon":"M","kind":{"Mode":{"back":"Nothing","forward":"Nothing"}}}
            ]}]}"#,
        );
        assert!(crate::config::extra_buttons_in_use(&old).is_empty());
        let json = serde_json::to_string(&old).unwrap();
        assert!(
            !json.contains("extra"),
            "nothing is written for a mode without extras"
        );
    }

    #[test]
    fn only_known_trigger_buttons_survive() {
        for ok in [2, 5, 6, 7] {
            let c = parse(&format!(r#"{{"version":2,"trigger_button":{ok}}}"#));
            assert_eq!(c.trigger_button, ok);
        }
        for bad in [0, 1, 3, 4, 8, -1] {
            let c = parse(&format!(r#"{{"version":2,"trigger_button":{bad}}}"#));
            assert_eq!(c.trigger_button, 2, "{bad} falls back to the middle button");
        }
    }

    #[test]
    fn rings_and_profiles_are_limited() {
        let item = |n: usize| {
            (0..n)
                .map(|i| format!(r#"{{"name":"{i}","icon":"x","kind":{{"Run":"Nothing"}}}}"#))
                .collect::<Vec<_>>()
                .join(",")
        };
        let c = parse(&format!(
            r#"{{"version":2,"profiles":[{{"name":"G","apps":[],"items":[{}],"active":[]}}]}}"#,
            item(12)
        ));
        assert_eq!(c.profiles[0].items.len(), MAX_ITEMS);
        let many: Vec<String> = (0..30)
            .map(|i| {
                format!(
                    r#"{{"name":"p{i}","apps":["a{i}"],"items":[{}],"active":[]}}"#,
                    item(1)
                )
            })
            .collect();
        let c = parse(&format!(
            r#"{{"version":2,"profiles":[{}]}}"#,
            many.join(",")
        ));
        assert_eq!(c.profiles.len(), MAX_PROFILES);
    }

    #[test]
    fn an_empty_profile_gets_a_starter_slice_and_an_active_mode() {
        let c =
            parse(r#"{"version":2,"profiles":[{"name":"G","apps":[],"items":[],"active":[7]}]}"#);
        assert!(!c.profiles[0].items.is_empty());
        assert_eq!(c.profiles[0].active, vec![0], "the starter slice is a mode");
        let none = parse(r#"{"version":2,"profiles":[]}"#);
        assert_eq!(none.profiles.len(), Config::default().profiles.len());
    }

    #[test]
    fn a_bad_active_path_is_repaired() {
        let json = r#"{"version":2,"profiles":[{"name":"G","apps":[],
            "items":[{"name":"r","icon":"x","kind":{"Run":"Nothing"}},
                     {"name":"m","icon":"x","kind":{"Mode":{"back":"Nothing","forward":"Nothing"}}}],
            "active":[0]}]}"#;
        let c = parse(json);
        assert_eq!(c.profiles[0].active, vec![1], "points at the first mode");
    }

    #[test]
    fn deep_submenus_are_emptied() {
        fn nest(depth: usize) -> String {
            if depth == 0 {
                return r#"{"name":"leaf","icon":"x","kind":{"Run":"Nothing"}}"#.into();
            }
            format!(
                r#"{{"name":"s","icon":"x","kind":{{"Submenu":[{}]}}}}"#,
                nest(depth - 1)
            )
        }
        let json = format!(
            r#"{{"version":2,"profiles":[{{"name":"G","apps":[],"items":[{}],"active":[]}}]}}"#,
            nest(8)
        );
        let c = parse(&json);
        let mut depth = 1;
        let mut items = &c.profiles[0].items;
        while let Some(ItemKind::Submenu(ch)) = items.first().map(|i| &i.kind) {
            if ch.is_empty() {
                break;
            }
            depth += 1;
            items = ch;
        }
        assert!(depth <= MAX_DEPTH, "depth {depth}");
    }
}

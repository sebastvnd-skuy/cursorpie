//! The configuration: data model, defaults, validation and persistence.
//!
//! ```text
//! Config -> profiles[] -> items[] -> (Mode | Run | Submenu(items[]))
//! ```
//!
//! A profile with an empty `apps` list is the fallback for every app that has no profile of its own.
//!
//! | Submodule | Contents |
//! |---|---|
//! | `model` | the serialized types: [`Config`], [`Profile`], [`Item`], [`ItemKind`], [`Action`] and the option enums |
//! | `tree` | walking a menu by *path* (a list of indices), see [`items_at`] |
//! | `defaults` | built-in slices and `Config::default()` |
//! | `sanitize` | [`Config::sanitize`]: clamps everything into the supported limits |
//! | `store` | `config.json` on disk, import/export and the live global copy ([`cfg()`]) |
//! | [`share`] | exporting and importing one profile as a file, icons included |
//! | [`backup`] | rolling backups of `config.json`, written by every Save in Settings |
//!
//! All public items are re-exported here, so use `crate::config::X`. The file format is described in
//! [`guides::config_format`](crate::guides::config_format).

pub mod backup;
mod defaults;
mod model;
mod sanitize;
pub mod share;
mod store;
mod tree;

pub use defaults::{blank_items, presets, quick_actions};
pub use model::*;
pub use share::image_path;
pub use store::*;
pub use tree::*;

/// Format version written to every config file; files without it are rejected.
pub const CONFIG_VERSION: u32 = 2;
/// Max slices per ring.
pub const MAX_ITEMS: usize = 8;
/// Max menu levels (top level counts as 1).
pub const MAX_DEPTH: usize = 5;
/// Max number of profiles.
pub const MAX_PROFILES: usize = 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_without_the_new_fields_still_loads() {
        // what a v0.2.0 file looks like: none of the side-button / app-detection settings
        let c = parse_config(r#"{"version": 2, "pie_hold_ms": 400}"#).expect("old config parses");
        assert_eq!(c.pie_hold_ms, 400);
        assert_eq!(c.target_app, TargetApp::UnderCursor);
        assert_eq!(c.side_default, SideTarget::App);
        assert!(c.focus_target && c.side_adopt && c.right_click_switch && c.side_indicator);
        assert!(c.center_app_icon);
        assert!(!c.side_forget);
    }

    #[test]
    fn new_fields_round_trip_and_preview_is_clamped() {
        let c = parse_config(
            r#"{"version": 2, "target_app": "Focused", "side_default": "General", "preview_ms": 999999}"#,
        )
        .unwrap();
        assert_eq!(c.target_app, TargetApp::Focused);
        assert_eq!(c.side_default, SideTarget::General);
        assert_eq!(c.preview_ms, 3000);
    }

    #[test]
    fn only_one_general_profile_survives_and_apps_are_cleaned() {
        let item = r#"[{"name":"A","icon":"A","kind":{"Run":"Nothing"}}]"#;
        let json = format!(
            r#"{{"version": 2, "profiles": [
                {{"name":"General","apps":[],"items":{item},"active":[]}},
                {{"name":"Dead","apps":["  "],"items":{item},"active":[]}},
                {{"name":"Safari","apps":[" com.apple.Safari ","com.apple.Safari"],"items":{item},"active":[]}}
            ]}}"#
        );
        let c = parse_config(&json).unwrap();
        let names: Vec<_> = c.profiles.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            ["General", "Safari"],
            "the app-less duplicate is dropped"
        );
        assert_eq!(c.profiles[1].apps, ["com.apple.Safari"]);
        assert_eq!(c.profiles.iter().filter(|p| p.is_general()).count(), 1);
    }

    #[test]
    fn a_new_profile_needs_a_name_and_an_app() {
        assert!(Profile::for_app("Photo", "").is_none());
        assert!(Profile::for_app("  ", "com.x.App").is_none());
        let p = Profile::for_app(" Photo ", " com.x.App ").unwrap();
        assert_eq!(
            (p.name.as_str(), p.apps.len(), p.is_general()),
            ("Photo", 1, false)
        );
        assert!(!p.items.is_empty());
        assert!(!profile_has_commands(&p));
    }

    #[test]
    fn sleep_setting_defaults_and_is_clamped() {
        let old = parse_config(r#"{"version": 2}"#).unwrap();
        assert_eq!(old.sleep_after_s, Config::default().sleep_after_s);
        assert!(old.sleep_after_s > 0, "sleeping is on by default");
        let never = parse_config(r#"{"version": 2, "sleep_after_s": 0}"#).unwrap();
        assert_eq!(never.sleep_after_s, 0, "0 = never sleep, kept as it is");
        let low = parse_config(r#"{"version": 2, "sleep_after_s": 1}"#).unwrap();
        assert_eq!(low.sleep_after_s, 10);
        let high = parse_config(r#"{"version": 2, "sleep_after_s": 999999}"#).unwrap();
        assert_eq!(high.sleep_after_s, 3600);
    }
}

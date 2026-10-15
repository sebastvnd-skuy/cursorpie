//! Helpers for walking the menu tree: `items[]` of a profile, with `Submenu` nesting.
//!
//! A *path* is a list of indices: `[3, 1]` is slice 1 inside the submenu at slice 3 of the top ring.

use super::{Action, Config, Item, ItemKind, Profile};

/// The ring at `path`: an empty path is the top ring, `[3]` is the submenu at slice 3, and so on.
/// `None` if the path leaves the tree.
pub fn items_at<'a>(items: &'a [Item], path: &[usize]) -> Option<&'a [Item]> {
    let mut cur = items;
    for &i in path {
        match &cur.get(i)?.kind {
            ItemKind::Submenu(children) => cur = children.as_slice(),
            _ => return None,
        }
    }
    Some(cur)
}

/// The slice at `path` (the last index selects the slice inside the ring the rest leads to).
pub fn item_at<'a>(items: &'a [Item], path: &[usize]) -> Option<&'a Item> {
    let (last, parents) = path.split_last()?;
    items_at(items, parents)?.get(*last)
}

/// Mutable version of [`item_at`].
pub fn item_at_mut<'a>(items: &'a mut [Item], path: &[usize]) -> Option<&'a mut Item> {
    let (first, rest) = path.split_first()?;
    let item = items.get_mut(*first)?;
    if rest.is_empty() {
        return Some(item);
    }
    match &mut item.kind {
        ItemKind::Submenu(children) => item_at_mut(children, rest),
        _ => None,
    }
}

/// Whether one action runs a shell command.
fn action_runs_command(a: &Action) -> bool {
    match a {
        Action::Command(_) => true,
        Action::Sequence(v) => v.iter().any(action_runs_command),
        _ => false,
    }
}

/// Whether any slice in `v` (submenus included) runs a shell command.
fn items_run_commands(v: &[Item]) -> bool {
    v.iter().any(|i| match &i.kind {
        ItemKind::Mode { back, forward } => {
            action_runs_command(back) || action_runs_command(forward)
        }
        ItemKind::Run(a) => action_runs_command(a),
        ItemKind::Submenu(c) => items_run_commands(c),
    })
}

/// true if any action in the tree runs a shell command (used to warn after importing a config).
pub fn has_commands(cfg: &Config) -> bool {
    cfg.profiles.iter().any(profile_has_commands)
}

/// true if any action of the profile runs a shell command (used to warn after importing a profile).
pub fn profile_has_commands(p: &Profile) -> bool {
    items_run_commands(&p.items)
}

/// Mutable version of [`items_at`]: the ring at `path`, where an empty path is the top ring.
/// `None` if the path leaves the tree.
pub fn ring_mut<'a>(items: &'a mut Vec<Item>, path: &[usize]) -> Option<&'a mut Vec<Item>> {
    if path.is_empty() {
        return Some(items);
    }
    match &mut item_at_mut(items, path)?.kind {
        ItemKind::Submenu(c) => Some(c),
        _ => None,
    }
}

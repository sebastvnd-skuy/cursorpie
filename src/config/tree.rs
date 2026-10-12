//! Helpers for walking the menu tree: `items[]` of a profile, with `Submenu` nesting.
//!
//! A *path* is a list of indices: `[3, 1]` is slice 1 inside the submenu at slice 3 of the top ring.

use super::{Action, Config, Item, ItemKind};

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
pub fn item_at_mut<'a>(items: &'a mut Vec<Item>, path: &[usize]) -> Option<&'a mut Item> {
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

/// true if any action in the tree runs a shell command (used to warn after importing a config).
pub fn has_commands(cfg: &Config) -> bool {
    /// Whether one action runs a shell command.
    fn act(a: &Action) -> bool {
        match a {
            Action::Command(_) => true,
            Action::Sequence(v) => v.iter().any(act),
            _ => false,
        }
    }
    /// Whether any slice below `v` runs a shell command.
    fn items(v: &[Item]) -> bool {
        v.iter().any(|i| match &i.kind {
            ItemKind::Mode { back, forward } => act(back) || act(forward),
            ItemKind::Run(a) => act(a),
            ItemKind::Submenu(c) => items(c),
        })
    }
    cfg.profiles.iter().any(|p| items(&p.items))
}

/// Cincin (daftar item) pada path tertentu; path kosong = level atas.
pub fn ring_mut<'a>(items: &'a mut Vec<Item>, path: &[usize]) -> Option<&'a mut Vec<Item>> {
    if path.is_empty() {
        return Some(items);
    }
    match &mut item_at_mut(items, path)?.kind {
        ItemKind::Submenu(c) => Some(c),
        _ => None,
    }
}

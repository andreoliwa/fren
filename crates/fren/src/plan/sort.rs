//! Bottom-up sort invariant.
//!
//! The executor processes plans in deepest-first order so that a plan's
//! `original_path` is still valid at execution time (no parent has been
//! renamed yet). At the same depth, files are processed before
//! directories so that within the same parent the dir rename happens
//! last, after its children have already been renamed.

use crate::RenamePlan;
use std::cmp::Reverse;

use crate::plan_types::ItemKind;

/// Sort a plan vector in-place: deepest first; at equal depth, files
/// before directories.
pub fn sort_bottom_up(plans: &mut [RenamePlan]) {
    plans.sort_by_key(|p| {
        let kind_order = match p.kind {
            ItemKind::File | ItemKind::Symlink => 0,
            ItemKind::Dir => 1,
        };
        (Reverse(p.depth), kind_order)
    });
}

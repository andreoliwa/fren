//! Planning: walk the filesystem, compute new names, detect conflicts,
//! produce a [`crate::RenamePlan`] vector sorted bottom-up for safe
//! execution.

mod conflict;
mod sort;
mod walker;

pub use sort::sort_bottom_up;

use crate::{
    plan_types::ItemKind, slugify::slugify_camel_iso_with_year, FrenError, PlanOpts, RenamePlan,
    SlugOpts,
};
use chrono::{Datelike, Local};
use std::ffi::OsString;
use std::path::Path;
use uuid::Uuid;

/// Build a [`RenamePlan`] vector for the given roots.
///
/// Walks each root recursively (per `opts.recursive`), computes new names
/// via [`slugify_camel_iso_with_year`], resolves within-batch and pre-existing
/// target conflicts according to `plan_opts.on_conflict`, and returns plans
/// sorted deepest-first / files-before-dirs at the same depth.
///
/// Under [`crate::ConflictPolicy::Abort`] (the default), any conflict is
/// returned as an error. Under [`crate::ConflictPolicy::Number`], colliding
/// plans are renamed to `{stem}-copy-{n}` variants instead of erroring.
///
/// Errors:
/// - `FrenError::Io` if a root or any descendant cannot be read.
/// - `FrenError::TargetExists` if a planned target already exists outside
///   the batch and policy is `Abort`.
/// - `FrenError::WithinBatchCollision` if two plans target the same path
///   and policy is `Abort`.
pub fn plan(
    roots: &[&Path],
    slug_opts: &SlugOpts,
    plan_opts: &PlanOpts,
) -> Result<Vec<RenamePlan>, FrenError> {
    let current_year = Local::now().year();
    plan_with_year(roots, slug_opts, plan_opts, current_year)
}

/// Variant of [`plan`] that accepts the current year explicitly for
/// deterministic testing. Production code should use [`plan`] instead.
pub fn plan_with_year(
    roots: &[&Path],
    slug_opts: &SlugOpts,
    plan_opts: &PlanOpts,
    current_year: i32,
) -> Result<Vec<RenamePlan>, FrenError> {
    let batch_id = Uuid::now_v7();
    let mut plans = Vec::new();

    for root in roots {
        let items = walker::walk(root, plan_opts)?;
        for item in items {
            // Skip the root itself (we don't rename what the user explicitly
            // pointed at - only its descendants).
            if item.path == *root {
                continue;
            }
            let new_name = compute_new_name(&item, slug_opts, current_year);
            let old_name = item
                .path
                .file_name()
                .map(OsString::from)
                .unwrap_or_default();
            if Some(new_name.as_os_str()) == Some(old_name.as_os_str()) {
                // No-op; nothing to rename.
                continue;
            }
            let parent = item
                .path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default();
            plans.push(RenamePlan {
                original_path: item.path.clone(),
                parent,
                old_name,
                new_name,
                depth: item.depth,
                kind: item.kind,
                detected_date: None,
                batch_id,
            });
        }
    }

    sort_bottom_up(&mut plans);
    conflict::resolve_within_batch(&mut plans, plan_opts.on_conflict)?;
    conflict::resolve_preexisting(&mut plans, plan_opts.on_conflict)?;
    Ok(plans)
}

fn compute_new_name(
    item: &walker::DiscoveredItem,
    slug_opts: &SlugOpts,
    current_year: i32,
) -> OsString {
    let raw_name = item
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match item.kind {
        ItemKind::Dir => slugify_camel_iso_with_year(&raw_name, slug_opts, current_year).into(),
        ItemKind::File | ItemKind::Symlink => {
            let path = std::path::Path::new(&raw_name);
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().into_owned())
                .unwrap_or_default();
            let new_stem = slugify_camel_iso_with_year(&stem, slug_opts, current_year);
            if ext.is_empty() {
                new_stem.into()
            } else {
                format!("{}.{}", new_stem, ext.to_lowercase()).into()
            }
        }
    }
}

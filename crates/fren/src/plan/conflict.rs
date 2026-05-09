//! Conflict detection for the planner.
//!
//! Two distinct conflict classes:
//!
//! 1. **Within-batch:** two plans want to rename to the same target. Always
//!    aborts (different files cannot share a target).
//! 2. **Pre-existing target:** the target path already exists on disk and
//!    is not part of the batch. Aborts under `ConflictPolicy::Abort` (the
//!    only currently-supported policy).

use crate::{FrenError, RenamePlan};
use std::collections::HashMap;
use std::path::PathBuf;

/// Verify no two plans target the same final path.
pub fn check_within_batch(plans: &[RenamePlan]) -> Result<(), FrenError> {
    let mut targets: HashMap<PathBuf, &RenamePlan> = HashMap::new();
    for plan in plans {
        let target = plan.parent.join(&plan.new_name);
        if let Some(prev) = targets.insert(target.clone(), plan) {
            return Err(FrenError::WithinBatchCollision {
                a: prev.original_path.clone(),
                b: plan.original_path.clone(),
                target,
            });
        }
    }
    Ok(())
}

/// Verify no plan's target already exists on disk outside the batch.
///
/// Plans whose target equals another plan's source are exempt - that is the
/// chain case (`a → b`, `b → c`) which the bottom-up executor handles
/// safely as long as ordering is correct.
pub fn check_preexisting(plans: &[RenamePlan]) -> Result<(), FrenError> {
    let plan_sources: std::collections::HashSet<&PathBuf> =
        plans.iter().map(|p| &p.original_path).collect();
    for plan in plans {
        let target = plan.parent.join(&plan.new_name);
        if plan_sources.contains(&target) {
            // Some other plan will move this path away first.
            continue;
        }
        if target.exists() {
            return Err(FrenError::TargetExists(target));
        }
    }
    Ok(())
}

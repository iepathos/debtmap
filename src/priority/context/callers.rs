//! Caller context extraction (Spec 263).
//!
//! For function-level debt items, the scope is a single `FunctionId`. For
//! god-object debt items the scope is the union of every method (GodClass) or
//! every function (GodFile/GodModule) attributed to the item, so that callers
//! are aggregated across the architectural unit rather than looked up at a
//! single (struct-name, struct-declaration-line) position that does not
//! correspond to a node in the call graph.

use super::types::{ContextRelationship, FileRange, RelatedContext};
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashSet;
use std::path::Path;

/// Extract caller context entries for a debt item's call-graph scope.
///
/// `scope` is the set of `FunctionId`s the debt item represents. For
/// function-level items this is a one-element slice; for god-object items it
/// is every member function attributed to the item.
///
/// Cross-edges that stay inside the scope (e.g. one struct method calling
/// another method of the same struct) are intentionally filtered out: they
/// represent internal cohesion, not external coupling.
///
/// `primary_file` is used to prioritize callers in the same file/module as
/// the debt item.
pub fn extract_caller_contexts(
    scope: &[FunctionId],
    primary_file: &Path,
    call_graph: &CallGraph,
    max_callers: u32,
) -> Vec<RelatedContext> {
    let scope_set: HashSet<&FunctionId> = scope.iter().collect();

    let mut external_callers: Vec<FunctionId> = scope
        .iter()
        .flat_map(|fid| call_graph.get_callers(fid))
        .filter(|caller| !scope_set.contains(caller))
        .collect();

    external_callers.sort();
    external_callers.dedup();

    external_callers.sort_by_key(|caller_id| {
        if caller_id.file == primary_file {
            0
        } else if is_same_module(&caller_id.file, primary_file) {
            1
        } else {
            2
        }
    });

    external_callers
        .into_iter()
        .take(max_callers as usize)
        .map(|caller| create_caller_context(&caller))
        .collect()
}

fn is_same_module(file1: &Path, file2: &Path) -> bool {
    file1.parent() == file2.parent()
}

fn create_caller_context(caller_id: &FunctionId) -> RelatedContext {
    let reason = format!("Called by {}", caller_id.name);

    let start_line = caller_id.line.saturating_sub(2);
    let end_line = caller_id.line + 20;

    RelatedContext {
        range: FileRange {
            file: caller_id.file.clone(),
            start_line: start_line as u32,
            end_line: end_line as u32,
            symbol: Some(caller_id.name.clone()),
        },
        relationship: ContextRelationship::Caller,
        reason,
    }
}

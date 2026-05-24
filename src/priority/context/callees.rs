//! Callee context extraction (Spec 263).
//!
//! See `callers.rs` for the rationale behind taking a scope (`&[FunctionId]`)
//! instead of a single `FunctionId`. Edges that stay inside the scope are
//! filtered as they represent internal cohesion, not external dependency.

use super::types::{ContextRelationship, FileRange, RelatedContext};
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashSet;

/// Extract callee context entries for a debt item's call-graph scope.
///
/// `scope` is the set of `FunctionId`s the debt item represents. Trivial
/// callees (e.g. simple accessors, `clone`/`unwrap`) are filtered, and
/// internal cross-edges are excluded so that god-object items only surface
/// genuinely external dependencies.
pub fn extract_callee_contexts(
    scope: &[FunctionId],
    call_graph: &CallGraph,
    max_callees: u32,
) -> Vec<RelatedContext> {
    let scope_set: HashSet<&FunctionId> = scope.iter().collect();

    let mut external_callees: Vec<FunctionId> = scope
        .iter()
        .flat_map(|fid| call_graph.get_callees(fid))
        .filter(|callee| !scope_set.contains(callee))
        .filter(|callee| !is_trivial_callee(callee))
        .collect();

    external_callees.sort();
    external_callees.dedup();

    external_callees
        .into_iter()
        .take(max_callees as usize)
        .map(|callee| create_callee_context(&callee))
        .collect()
}

fn is_trivial_callee(callee_id: &FunctionId) -> bool {
    let name = &callee_id.name;

    name.starts_with("get_")
        || name.starts_with("set_")
        || name.starts_with("is_")
        || name == "clone"
        || name == "to_string"
        || name == "unwrap"
        || name == "expect"
}

fn create_callee_context(callee_id: &FunctionId) -> RelatedContext {
    let reason = format!("Calls {}", callee_id.name);

    let start_line = callee_id.line.saturating_sub(2);
    let end_line = callee_id.line + 20;

    RelatedContext {
        range: FileRange {
            file: callee_id.file.clone(),
            start_line: start_line as u32,
            end_line: end_line as u32,
            symbol: Some(callee_id.name.clone()),
        },
        relationship: ContextRelationship::Callee,
        reason,
    }
}

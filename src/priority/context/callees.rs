//! Callee context extraction (Spec 263).

use super::types::{ContextRelationship, FileRange, RelatedContext};
use crate::priority::call_graph::{CallGraph, FunctionId};

pub fn extract_callee_contexts(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    max_callees: u32,
) -> Vec<RelatedContext> {
    let callees = call_graph.get_callees(func_id);

    let non_trivial: Vec<_> = callees
        .iter()
        .filter(|callee_id| !is_trivial_callee(callee_id))
        .collect();

    non_trivial
        .into_iter()
        .take(max_callees as usize)
        .map(create_callee_context)
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

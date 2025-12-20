//! Caller context extraction (Spec 263).

use super::types::{ContextRelationship, FileRange, RelatedContext};
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::path::Path;

pub fn extract_caller_contexts(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    max_callers: u32,
) -> Vec<RelatedContext> {
    let callers = call_graph.get_callers(func_id);

    let mut prioritized: Vec<_> = callers.iter().collect();
    prioritized.sort_by_key(|caller_id| {
        if caller_id.file == func_id.file {
            0
        } else if is_same_module(&caller_id.file, &func_id.file) {
            1
        } else {
            2
        }
    });

    prioritized
        .into_iter()
        .take(max_callers as usize)
        .map(create_caller_context)
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

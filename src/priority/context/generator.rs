//! Context suggestion generation (Spec 263).

use super::callees::extract_callee_contexts;
use super::callers::extract_caller_contexts;
use super::limits::apply_limits;
use super::tests_ctx::extract_test_contexts;
use super::types::{ContextRelationship, ContextSuggestion, FileRange, RelatedContext};
use super::types_ctx::extract_type_contexts;
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::UnifiedDebtItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub max_total_lines: u32,
    pub max_callers: u32,
    pub max_callees: u32,
    pub include_tests: bool,
    pub include_types: bool,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_total_lines: 500,
            max_callers: 3,
            max_callees: 3,
            include_tests: true,
            include_types: true,
        }
    }
}

pub fn generate_context_suggestion(
    item: &UnifiedDebtItem,
    call_graph: &CallGraph,
    config: &ContextConfig,
) -> Option<ContextSuggestion> {
    let primary = extract_primary_scope(item);

    let func_id = FunctionId::new(
        item.location.file.clone(),
        item.location.function.clone(),
        item.location.line,
    );

    let mut related = Vec::new();

    related.push(create_module_header_context(&item.location.file));

    let callers = extract_caller_contexts(&func_id, call_graph, config.max_callers);
    related.extend(callers.clone());

    let callees = extract_callee_contexts(&func_id, call_graph, config.max_callees);
    related.extend(callees.clone());

    if config.include_types {
        let types = extract_type_contexts(&item.location.file, &item.location.function);
        related.extend(types);
    }

    if config.include_tests {
        let tests = extract_test_contexts(&item.location.file, &item.location.function);
        related.extend(tests);
    }

    let confidence = calculate_completeness_confidence(
        !callers.is_empty(),
        !callees.is_empty(),
        config.include_types,
        config.include_tests,
        0,
    );

    let suggestion = ContextSuggestion {
        primary,
        related,
        total_lines: 0,
        completeness_confidence: confidence,
    };

    let limited = apply_limits(suggestion, config.max_total_lines);

    Some(limited)
}

fn extract_primary_scope(item: &UnifiedDebtItem) -> FileRange {
    let start_line = item.location.line.saturating_sub(2) as u32;

    let end_line = if item.function_length > 0 {
        start_line + item.function_length as u32 + 2
    } else {
        start_line + (item.cyclomatic_complexity * 3).max(20)
    };

    FileRange {
        file: item.location.file.clone(),
        start_line,
        end_line,
        symbol: Some(item.location.function.clone()),
    }
}

fn create_module_header_context(file: &std::path::Path) -> RelatedContext {
    RelatedContext {
        range: FileRange {
            file: file.to_path_buf(),
            start_line: 1,
            end_line: 20,
            symbol: None,
        },
        relationship: ContextRelationship::ModuleHeader,
        reason: "Module imports and constants".to_string(),
    }
}

fn calculate_completeness_confidence(
    has_callers: bool,
    has_callees: bool,
    has_types: bool,
    has_tests: bool,
    unresolved_dependencies: u32,
) -> f32 {
    let base = 0.5;
    let mut confidence = base;

    if has_callers {
        confidence += 0.1;
    }
    if has_callees {
        confidence += 0.1;
    }
    if has_types {
        confidence += 0.1;
    }
    if has_tests {
        confidence += 0.1;
    }

    confidence -= (unresolved_dependencies as f32) * 0.05;

    confidence.clamp(0.0, 1.0)
}

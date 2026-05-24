//! Context suggestion generation (Spec 263).

use super::callees::extract_callee_contexts;
use super::callers::extract_caller_contexts;
use super::limits::apply_limits;
use super::tests_ctx::extract_test_contexts;
use super::types::{ContextRelationship, ContextSuggestion, FileRange, RelatedContext};
use super::types_ctx::extract_type_contexts;
use crate::organization::DetectionType;
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::UnifiedDebtItem;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Global default config to avoid repeated construction in hot paths.
static DEFAULT_CONFIG: OnceLock<ContextConfig> = OnceLock::new();

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

impl ContextConfig {
    /// Get the global default config.
    ///
    /// This avoids repeated struct construction in hot paths.
    pub fn global_default() -> &'static Self {
        DEFAULT_CONFIG.get_or_init(Self::default)
    }
}

pub fn generate_context_suggestion(
    item: &UnifiedDebtItem,
    call_graph: &CallGraph,
    config: &ContextConfig,
) -> Option<ContextSuggestion> {
    let primary = extract_primary_scope(item);
    let scope = collect_scope_function_ids(item, call_graph);

    let mut related = Vec::new();

    related.push(create_module_header_context(&item.location.file));

    let callers =
        extract_caller_contexts(&scope, &item.location.file, call_graph, config.max_callers);
    related.extend(callers.clone());

    let callees = extract_callee_contexts(&scope, call_graph, config.max_callees);
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

/// Build the call-graph scope for a debt item.
///
/// Function-level items resolve to a single `FunctionId` at
/// `(file, function, line)` (matched fuzzily by `CallGraph::get_callers` /
/// `get_callees`). God-object items have no node at the struct's declaration
/// line, so the scope is built by enumerating member functions instead:
///
/// - `GodClass`: every function in the item's file whose name starts with
///   `"{struct_name}::"` (the qualified-method naming convention used by
///   the function visitor).
/// - `GodFile` / `GodModule`: every function in the item's file.
fn collect_scope_function_ids(item: &UnifiedDebtItem, call_graph: &CallGraph) -> Vec<FunctionId> {
    use crate::organization::DetectionType;

    if let Some(god) = item.god_object_indicators.as_ref() {
        match god.detection_type {
            DetectionType::GodClass => {
                if let Some(struct_name) =
                    god.struct_name.as_deref().filter(|name| !name.is_empty())
                {
                    let prefix = format!("{}::", struct_name);
                    let scope: Vec<FunctionId> = call_graph
                        .get_all_functions()
                        .filter(|f| f.file == item.location.file && f.name.starts_with(&prefix))
                        .cloned()
                        .collect();
                    if !scope.is_empty() {
                        return scope;
                    }
                }
                fallback_single_function_id(item)
            }
            DetectionType::GodFile | DetectionType::GodModule => {
                let scope: Vec<FunctionId> = call_graph
                    .get_all_functions()
                    .filter(|f| f.file == item.location.file)
                    .cloned()
                    .collect();
                if scope.is_empty() {
                    fallback_single_function_id(item)
                } else {
                    scope
                }
            }
        }
    } else {
        fallback_single_function_id(item)
    }
}

fn fallback_single_function_id(item: &UnifiedDebtItem) -> Vec<FunctionId> {
    vec![FunctionId::new(
        item.location.file.clone(),
        item.location.function.clone(),
        item.location.line,
    )]
}

fn extract_primary_scope(item: &UnifiedDebtItem) -> FileRange {
    if let Some(scope) = extract_god_class_primary_scope(item) {
        return scope;
    }

    let start_line = item.location.line.saturating_sub(2).max(1) as u32;

    let end_line = if item.function_length > 0 {
        start_line + item.function_length as u32 + 2
    } else {
        start_line + (item.cyclomatic_complexity * 3).max(20)
    };
    let end_line = clamp_to_file_line_count(end_line, item.file_line_count);

    FileRange {
        file: item.location.file.clone(),
        start_line,
        end_line,
        symbol: Some(item.location.function.clone()),
    }
}

fn extract_god_class_primary_scope(item: &UnifiedDebtItem) -> Option<FileRange> {
    let god_object = item.god_object_indicators.as_ref()?;
    if god_object.detection_type != DetectionType::GodClass {
        return None;
    }

    let location = god_object.struct_location.as_ref()?;
    let start_line = location.line.saturating_sub(2).max(1) as u32;
    let end_line = location.end_line.unwrap_or(location.line) as u32 + 2;
    let end_line = clamp_to_file_line_count(end_line, item.file_line_count);

    Some(FileRange {
        file: item.location.file.clone(),
        start_line,
        end_line,
        symbol: Some(item.location.function.clone()),
    })
}

fn clamp_to_file_line_count(end_line: u32, file_line_count: Option<usize>) -> u32 {
    file_line_count
        .and_then(|count| u32::try_from(count).ok())
        .filter(|count| *count > 0)
        .map_or(end_line, |count| end_line.min(count))
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

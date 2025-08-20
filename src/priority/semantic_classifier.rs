use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionRole {
    PureLogic,    // Business logic, high test priority
    Orchestrator, // Coordinates other functions
    IOWrapper,    // Thin I/O layer
    EntryPoint,   // Main entry points
    PatternMatch, // Pattern matching function (low complexity)
    Unknown,      // Cannot classify
}

pub fn classify_function_role(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
) -> FunctionRole {
    // Use a functional approach with classification rules
    classify_by_rules(func, func_id, call_graph).unwrap_or(FunctionRole::PureLogic)
}

// Pure function that applies classification rules in order
fn classify_by_rules(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
) -> Option<FunctionRole> {
    // Entry point has highest precedence
    if is_entry_point(func_id, call_graph) {
        return Some(FunctionRole::EntryPoint);
    }

    // Check for pattern matching functions (like detect_file_type)
    if is_pattern_matching_function(func, func_id) {
        return Some(FunctionRole::PatternMatch);
    }

    // Check I/O wrapper BEFORE orchestration
    if is_io_wrapper(func) {
        return Some(FunctionRole::IOWrapper);
    }

    // Only then check orchestration patterns
    if is_orchestrator(func, func_id, call_graph) {
        return Some(FunctionRole::Orchestrator);
    }

    None // Will default to PureLogic
}

// Pure function to check if a function is an entry point
fn is_entry_point(func_id: &FunctionId, call_graph: &CallGraph) -> bool {
    call_graph.is_entry_point(func_id) || is_entry_point_by_name(&func_id.name)
}

// Pure function to check if a function is a pattern matching function
fn is_pattern_matching_function(func: &FunctionMetrics, func_id: &FunctionId) -> bool {
    // Check for typical pattern matching function names
    let name_lower = func_id.name.to_lowercase();
    let pattern_match_names = [
        "detect",
        "classify",
        "identify",
        "determine",
        "resolve",
        "match",
        "parse_type",
        "get_type",
        "find_type",
    ];

    // Name suggests pattern matching AND has low cyclomatic but high cognitive complexity
    // (typical of if/else chains or match statements with many branches)
    let name_matches = pattern_match_names
        .iter()
        .any(|pattern| name_lower.contains(pattern));

    // Pattern matching functions typically have:
    // - Low cyclomatic complexity (1-2, just sequential checks)
    // - Higher cognitive complexity due to many conditions
    // - Cognitive/cyclomatic ratio > 5 suggests pattern matching
    if name_matches && func.cyclomatic <= 2 {
        let ratio = if func.cyclomatic > 0 {
            func.cognitive as f32 / func.cyclomatic as f32
        } else {
            func.cognitive as f32
        };
        return ratio > 5.0;
    }

    false
}

// Pure function to check if a function is an orchestrator
fn is_orchestrator(func: &FunctionMetrics, func_id: &FunctionId, call_graph: &CallGraph) -> bool {
    let config = crate::config::get_orchestration_config();

    // First check if there are meaningful callees to orchestrate
    let callees = call_graph.get_callees(func_id);
    let meaningful_callees: Vec<_> = callees
        .iter()
        .filter(|f| !is_std_or_utility_function(&f.name))
        .collect();

    // Check if this is a functional chain (all calls are functional methods)
    if config.allow_functional_chains
        && !meaningful_callees.is_empty()
        && callees.len() > meaningful_callees.len()
    {
        // If all non-utility calls are removed, this might be a functional chain
        let functional_chain = callees.iter().all(|f| {
            is_std_or_utility_function(&f.name)
                || f.name.contains("Pipeline")
                || f.name.contains("Stream")
                || f.name.contains("Iterator")
        });
        if functional_chain {
            return false;
        }
    }

    // Check for single delegation (adapter pattern)
    if config.exclude_adapters && meaningful_callees.len() == 1 {
        // This is likely an adapter/wrapper, not orchestration
        return false;
    }

    // Can't be an orchestrator without functions to orchestrate
    // Use configurable minimum delegation count
    if meaningful_callees.len() < config.min_delegations {
        return false;
    }

    // Name-based orchestration with low complexity
    let name_suggests_orchestration =
        is_orchestrator_by_name(&func_id.name) && func.cyclomatic <= 3;

    // Low complexity delegation pattern
    let is_simple_delegation = func.cyclomatic <= 2
        && func.cognitive <= 3
        && delegates_to_tested_functions(func_id, call_graph, 0.8);

    name_suggests_orchestration || is_simple_delegation
}

// Pure function to check if a function is an I/O wrapper
fn is_io_wrapper(func: &FunctionMetrics) -> bool {
    if !contains_io_patterns(func) {
        return false;
    }

    // Short I/O functions are clearly wrappers
    if func.length < 20 {
        return true;
    }

    // Longer functions can still be I/O wrappers if they match I/O orchestration patterns
    func.length <= 50 && is_io_orchestration(func)
}

fn is_entry_point_by_name(name: &str) -> bool {
    let entry_patterns = [
        "main", "run", "start", "init", "handle", "process", "execute", "serve", "listen",
    ];

    let name_lower = name.to_lowercase();
    entry_patterns
        .iter()
        .any(|pattern| name_lower.starts_with(pattern) || name_lower.ends_with(pattern))
}

fn is_orchestrator_by_name(name: &str) -> bool {
    let config = crate::config::get_orchestration_config();
    let name_lower = name.to_lowercase();

    // Check custom exclude patterns from config
    for pattern in &config.exclude_patterns {
        if name_lower.contains(&pattern.to_lowercase()) {
            return false;
        }
    }

    // Exclude common non-orchestration patterns first
    let exclude_patterns = [
        "print",
        "format",
        "create",
        "build",
        "extract",
        "parse",
        "new",
        "from",
        "to",
        "into",
        "write",
        "read",
        "display",
        "render",
        "emit",
        // Exclude adapter/wrapper patterns
        "adapt",
        "wrap",
        "convert",
        "transform",
        "translate",
        // Exclude functional patterns
        "map",
        "filter",
        "reduce",
        "fold",
        "collect",
        "apply",
        // Exclude single-purpose functions
        "get",
        "set",
        "find",
        "search",
        "check",
        "validate",
    ];

    for pattern in &exclude_patterns {
        if name_lower.starts_with(pattern) || name_lower.ends_with(pattern) {
            return false;
        }
    }

    // Check custom include patterns from config (these override excludes)
    for pattern in &config.include_patterns {
        if name_lower.contains(&pattern.to_lowercase()) {
            return true;
        }
    }

    // Then check for true orchestration patterns
    let orchestrator_patterns = [
        "orchestrate",
        "coordinate",
        "manage",
        "dispatch",
        "route",
        "if_requested",
        "if_needed",
        "if_enabled",
        "maybe",
        "try_",
        "attempt_",
        "delegate",
        "forward",
    ];

    // Check for conditional patterns like generate_report_if_requested
    if name_lower.contains("_if_") || name_lower.contains("_when_") {
        return true;
    }

    orchestrator_patterns
        .iter()
        .any(|pattern| name_lower.contains(pattern))
}

fn delegates_to_tested_functions(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    _threshold: f64,
) -> bool {
    let callees = call_graph.get_callees(func_id);
    if callees.is_empty() {
        return false;
    }

    // Filter out standard library functions and common utilities
    let meaningful_callees: Vec<_> = callees
        .iter()
        .filter(|f| !is_std_or_utility_function(&f.name))
        .collect();

    // Orchestrators should coordinate MULTIPLE functions (at least 2)
    // This is now consistent with the check in is_orchestrator
    meaningful_callees.len() >= 2 && call_graph.detect_delegation_pattern(func_id)
}

fn contains_io_patterns(func: &FunctionMetrics) -> bool {
    // Check for I/O related patterns in function name or content
    let io_keywords = vec![
        "read",
        "write",
        "file",
        "socket",
        "http",
        "request",
        "response",
        "stream",
        "buffer",
        "stdin",
        "stdout",
        "stderr",
        "print",
        "input",
        "output",
        "display",
        // Note: "format" removed - string formatting is not I/O
        "json",
        "serialize",
        "deserialize",
        "emit",
        "render",
        "save",
        "load",
        "export",
        "import",
        "log",
        "trace",
        "debug",
        "info",
        "warn",
        "error",
        "summary",
        "report",
    ];

    let name_lower = func.name.to_lowercase();
    io_keywords
        .iter()
        .any(|keyword| name_lower.contains(keyword))
}

fn is_io_orchestration(func: &FunctionMetrics) -> bool {
    // Function is I/O orchestration if it has I/O in the name and:
    // - Moderate cyclomatic complexity (mostly from format/output branching)
    // - Not deeply nested (nesting <= 3)
    // - Name strongly suggests I/O operations
    let name_lower = func.name.to_lowercase();

    // Strong I/O indicators in function name
    let strong_io_patterns = [
        "output_",
        "write_",
        "print_",
        "format_",
        "serialize_",
        "save_",
        "export_",
        "display_",
        "render_",
        "emit_",
    ];

    let has_strong_io_name = strong_io_patterns
        .iter()
        .any(|pattern| name_lower.starts_with(pattern));

    // I/O orchestration typically has branching for different formats/destinations
    // but not deep business logic
    has_strong_io_name && func.nesting <= 3
}

// Helper to identify standard library and utility functions that shouldn't count as delegation targets
fn is_std_or_utility_function(name: &str) -> bool {
    // Check the base name (after :: if present)
    let base_name = name.rsplit("::").next().unwrap_or(name);

    // Check if it's a functional chain method (like map, filter, collect)
    let is_functional_chain = matches!(
        base_name,
        "map"
            | "filter"
            | "filter_map"
            | "flat_map"
            | "fold"
            | "collect"
            | "zip"
            | "enumerate"
            | "chain"
            | "flatten"
            | "skip"
            | "take"
            | "skip_while"
            | "take_while"
            | "any"
            | "all"
            | "find"
            | "position"
            | "for_each"
            | "reduce"
            | "scan"
            | "inspect"
            | "partition"
            | "unzip"
    );

    // If it's a functional chain method on any type (e.g., LazyPipeline::map),
    // it's a utility function, not orchestration
    if is_functional_chain {
        return true;
    }

    matches!(
        base_name,
        // Standard library functions from macro expansion
        "format" | "write" | "print" | "println" |
        // Common utility functions that are too generic  
        "clone" | "to_string" | "into" | "from" | "as_ref" | "as_mut" |
        "borrow" | "borrow_mut" | "deref" | "deref_mut" |
        // Iterator methods that are utilities, not business logic
        "iter" | "into_iter" | "iter_mut" |
        // String manipulation utilities
        "to_lowercase" | "to_uppercase" | "trim" | "split" | "join" |
        // Common pattern checking functions
        "starts_with" | "ends_with" | "contains" | "is_empty" | "len" |
        // Memory management
        "new" | "default" | "drop" | "unwrap" | "expect" |
        "ok" | "err" | "some" | "none"
    ) || name.starts_with("std::")
        || name.starts_with("core::")
        || name.starts_with("alloc::")
        || name.ends_with("::iter")  // Any type's iter method
        || name.ends_with("::any") // Any type's any method
}

pub fn get_role_multiplier(role: FunctionRole) -> f64 {
    match role {
        FunctionRole::PureLogic => 1.5,    // High priority for business logic
        FunctionRole::Orchestrator => 0.2, // Low priority if delegates to tested code
        FunctionRole::IOWrapper => 0.1,    // Very low priority for thin I/O
        FunctionRole::EntryPoint => 0.8,   // Medium priority (integration test focus)
        FunctionRole::PatternMatch => 0.1, // Very low priority for pattern matching
        FunctionRole::Unknown => 1.0,      // Default multiplier
    }
}

pub fn calculate_semantic_priority(
    _func: &FunctionMetrics,
    role: FunctionRole,
    func_id: &FunctionId,
    call_graph: &CallGraph,
) -> f64 {
    let mut priority = match role {
        FunctionRole::PureLogic => 8.0,
        FunctionRole::Orchestrator => 2.0,
        FunctionRole::IOWrapper => 1.0,
        FunctionRole::EntryPoint => 6.0,
        FunctionRole::PatternMatch => 1.0,
        FunctionRole::Unknown => 5.0,
    };

    // Adjust based on criticality
    let criticality = call_graph.calculate_criticality(func_id);
    priority *= criticality;

    // Cap at 10.0
    priority.min(10.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FunctionMetrics;
    use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
    use std::path::PathBuf;

    fn create_test_metrics(
        name: &str,
        cyclomatic: u32,
        cognitive: u32,
        lines: usize,
    ) -> FunctionMetrics {
        FunctionMetrics {
            file: PathBuf::from("test.rs"),
            name: name.to_string(),
            line: 1,
            length: lines,
            cyclomatic,
            cognitive,
            nesting: 0,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
        }
    }

    #[test]
    fn test_entry_point_classification() {
        let graph = CallGraph::new();
        let func = create_test_metrics("main", 5, 8, 50);
        let func_id = FunctionId {
            file: PathBuf::from("main.rs"),
            name: "main".to_string(),
            line: 1,
        };

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::EntryPoint);
    }

    #[test]
    fn test_orchestrator_classification() {
        let mut graph = CallGraph::new();
        let func = create_test_metrics("coordinate_tasks", 2, 3, 15);
        let func_id = FunctionId {
            file: PathBuf::from("coord.rs"),
            name: "coordinate_tasks".to_string(),
            line: 10,
        };

        // Add the orchestrator function
        graph.add_function(func_id.clone(), false, false, 2, 15);

        // Add some worker functions it calls
        for i in 0..3 {
            let worker_id = FunctionId {
                file: PathBuf::from("worker.rs"),
                name: format!("worker_{i}"),
                line: i * 10,
            };
            graph.add_function(worker_id.clone(), false, false, 8, 40);
            graph.add_call(crate::priority::call_graph::FunctionCall {
                caller: func_id.clone(),
                callee: worker_id,
                call_type: crate::priority::call_graph::CallType::Delegate,
            });
        }

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::Orchestrator);
    }

    #[test]
    fn test_io_wrapper_classification() {
        let graph = CallGraph::new();
        let func = create_test_metrics("read_file", 1, 2, 10);
        let func_id = FunctionId {
            file: PathBuf::from("io.rs"),
            name: "read_file".to_string(),
            line: 5,
        };

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::IOWrapper);
    }

    #[test]
    fn test_io_orchestration_classification() {
        let graph = CallGraph::new();

        // Test case similar to output_unified_priorities:
        // - Has "output_" prefix (strong I/O pattern)
        // - 38 lines (within the 50 line limit)
        // - Cyclomatic 12 (from format branching)
        // - Nesting 3 (not deeply nested)
        let mut func = create_test_metrics("output_unified_priorities", 12, 19, 38);
        func.nesting = 3;

        let func_id = FunctionId {
            file: PathBuf::from("main.rs"),
            name: "output_unified_priorities".to_string(),
            line: 861,
        };

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::IOWrapper);

        // Test that high nesting disqualifies I/O orchestration
        func.nesting = 4;
        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::PureLogic);
    }

    #[test]
    fn test_pure_logic_classification() {
        let graph = CallGraph::new();
        let func = create_test_metrics("calculate_risk", 8, 12, 60);
        let func_id = FunctionId {
            file: PathBuf::from("calc.rs"),
            name: "calculate_risk".to_string(),
            line: 20,
        };

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::PureLogic);
    }

    #[test]
    fn test_role_multipliers() {
        assert_eq!(get_role_multiplier(FunctionRole::PureLogic), 1.5);
        assert_eq!(get_role_multiplier(FunctionRole::Orchestrator), 0.2);
        assert_eq!(get_role_multiplier(FunctionRole::IOWrapper), 0.1);
        assert_eq!(get_role_multiplier(FunctionRole::EntryPoint), 0.8);
        assert_eq!(get_role_multiplier(FunctionRole::Unknown), 1.0);
    }

    #[test]
    fn test_is_std_or_utility_function() {
        // Test standard library functions from macro expansion
        assert!(is_std_or_utility_function("format"));
        assert!(is_std_or_utility_function("write"));
        assert!(is_std_or_utility_function("print"));
        assert!(is_std_or_utility_function("println"));

        // Test std library paths
        assert!(is_std_or_utility_function("std::fmt::format"));
        assert!(is_std_or_utility_function("core::mem::drop"));
        assert!(is_std_or_utility_function("alloc::vec::Vec"));

        // Test common utility functions
        assert!(is_std_or_utility_function("clone"));
        assert!(is_std_or_utility_function("to_string"));
        assert!(is_std_or_utility_function("into"));
        assert!(is_std_or_utility_function("from"));

        // Test iterator methods
        assert!(is_std_or_utility_function("iter"));
        assert!(is_std_or_utility_function("any"));
        assert!(is_std_or_utility_function("filter"));
        assert!(is_std_or_utility_function("collect"));

        // Test qualified method names
        assert!(is_std_or_utility_function("ContextMap::iter"));
        assert!(is_std_or_utility_function("ContextMatcher::any"));
        assert!(is_std_or_utility_function("Vec::iter"));

        // Test string utilities
        assert!(is_std_or_utility_function("to_lowercase"));
        assert!(is_std_or_utility_function("starts_with"));
        assert!(is_std_or_utility_function("ends_with"));

        // Test non-std functions
        assert!(!is_std_or_utility_function("calculate_dash_count"));
        assert!(!is_std_or_utility_function("format_complexity_info"));
        assert!(!is_std_or_utility_function("my_custom_function"));
        assert!(!is_std_or_utility_function("is_entry_point_by_name"));
    }

    #[test]
    fn test_formatting_function_not_orchestrator() {
        let mut graph = CallGraph::new();

        // Create a function like format_recommendation_box_header
        let func = create_test_metrics("format_recommendation_box_header", 1, 0, 9);
        let func_id = FunctionId {
            file: PathBuf::from("insights.rs"),
            name: "format_recommendation_box_header".to_string(),
            line: 142,
        };

        // Add the function to the graph
        graph.add_function(func_id.clone(), false, false, 1, 9);

        // Add callees: calculate_dash_count and format (from macro)
        let callee1 = FunctionId {
            file: PathBuf::from("insights.rs"),
            name: "calculate_dash_count".to_string(),
            line: 138,
        };
        let callee2 = FunctionId {
            file: PathBuf::from("std"),
            name: "format".to_string(),
            line: 1,
        };

        graph.add_function(callee1.clone(), false, false, 1, 3);
        graph.add_function(callee2.clone(), false, false, 1, 1);

        graph.add_call(FunctionCall {
            caller: func_id.clone(),
            callee: callee1,
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: func_id.clone(),
            callee: callee2,
            call_type: CallType::Direct,
        });

        // Test that it's not classified as orchestrator
        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::PureLogic,
            "Formatting function should be PureLogic, not Orchestrator"
        );

        // Verify it doesn't match delegation pattern
        assert!(
            !delegates_to_tested_functions(&func_id, &graph, 0.8),
            "Should not be considered delegation when calling std functions"
        );
    }

    #[test]
    fn test_actual_orchestrator_with_meaningful_callees() {
        let mut graph = CallGraph::new();

        // Create an actual orchestrator function
        let func = create_test_metrics("coordinate_workflow", 2, 3, 15);
        let func_id = FunctionId {
            file: PathBuf::from("workflow.rs"),
            name: "coordinate_workflow".to_string(),
            line: 10,
        };

        graph.add_function(func_id.clone(), false, false, 2, 15);

        // Add meaningful callees (not std library)
        let callee1 = FunctionId {
            file: PathBuf::from("workflow.rs"),
            name: "process_step_one".to_string(),
            line: 50,
        };
        let callee2 = FunctionId {
            file: PathBuf::from("workflow.rs"),
            name: "process_step_two".to_string(),
            line: 100,
        };

        graph.add_function(callee1.clone(), false, false, 5, 30);
        graph.add_function(callee2.clone(), false, false, 5, 30);

        graph.add_call(FunctionCall {
            caller: func_id.clone(),
            callee: callee1,
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: func_id.clone(),
            callee: callee2,
            call_type: CallType::Direct,
        });

        // This should be classified as orchestrator
        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::Orchestrator,
            "Function coordinating multiple steps should be Orchestrator"
        );
    }
}

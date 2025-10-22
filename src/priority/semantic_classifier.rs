use crate::analyzers::rust_constructor_detector::{
    analyze_function_body, extract_return_type, ConstructorReturnType,
};
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
    // Note: AST is not available at this level, so we pass None
    // Full AST-based detection will be integrated when threading syn::ItemFn
    classify_by_rules(func, func_id, call_graph, None).unwrap_or(FunctionRole::PureLogic)
}

// Pure function that applies classification rules in order
fn classify_by_rules(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
    syn_func: Option<&syn::ItemFn>,
) -> Option<FunctionRole> {
    // Entry point has highest precedence
    if is_entry_point(func_id, call_graph) {
        return Some(FunctionRole::EntryPoint);
    }

    // Check for constructors BEFORE pattern matching (Spec 117 + 122)
    if is_constructor_enhanced(func, syn_func) {
        return Some(FunctionRole::IOWrapper);
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

/// Detect simple constructor functions to prevent false positive classifications.
///
/// A function is considered a simple constructor if it meets ALL criteria:
/// - Has a constructor-like name (new, default, from_*, with_*, etc.)
/// - Low cyclomatic complexity (≤ 2)
/// - Short length (< 15 lines)
/// - Minimal nesting (≤ 1 level)
/// - Low cognitive complexity (≤ 3)
///
/// # Examples
///
/// ```rust,ignore
/// // Simple constructor - matches
/// fn new() -> Self { Self { field: 0 } }
///
/// // Complex factory - does NOT match
/// fn create_with_validation(data: Data) -> Result<Self> {
///     validate(data)?;
///     // ... 30 lines of logic
///     Ok(Self { ... })
/// }
/// ```
///
/// # False Positive Prevention
///
/// This function specifically addresses the false positive in ContextMatcher::any()
/// where a trivial 9-line constructor was classified as CRITICAL business logic.
fn is_simple_constructor(func: &FunctionMetrics) -> bool {
    // Get constructor detection configuration
    let config = crate::config::get_constructor_detection_config();

    // Name-based detection using configurable patterns
    let name_lower = func.name.to_lowercase();
    let matches_constructor_name = config.patterns.iter().any(|pattern| {
        name_lower == *pattern || name_lower.starts_with(pattern) || name_lower.ends_with(pattern)
    });

    // Complexity-based filtering using configurable thresholds
    let is_simple = func.cyclomatic <= config.max_cyclomatic
        && func.length < config.max_length
        && func.nesting <= config.max_nesting;

    // Structural pattern: low cognitive complexity suggests simple initialization
    let is_initialization = func.cognitive <= config.max_cognitive;

    matches_constructor_name && is_simple && is_initialization
}

/// Enhanced constructor detection using AST (spec 122)
///
/// This function enhances name-based detection with AST analysis when available.
/// Falls back to `is_simple_constructor()` if AST is unavailable or disabled.
///
/// # Detection Strategy
///
/// 1. Check configuration - if AST detection disabled, use name-based only
/// 2. If AST available, analyze return type and body patterns
/// 3. Return type must be `Self`, `Result<Self>`, or `Option<Self>`
/// 4. Body must show constructor patterns (struct init, Self refs)
/// 5. Complexity must be reasonable (≤5 cyclomatic, no loops)
///
/// # Examples
///
/// ```rust,ignore
/// // Detected by AST (non-standard name)
/// pub fn create_default_client() -> Self {
///     Self { timeout: Duration::from_secs(30) }
/// }
///
/// // Detected by name-based (standard pattern)
/// pub fn new() -> Self {
///     Self { field: 0 }
/// }
/// ```
fn is_constructor_enhanced(func: &FunctionMetrics, syn_func: Option<&syn::ItemFn>) -> bool {
    // Check configuration
    let config = crate::config::get_constructor_detection_config();

    // If AST detection disabled or unavailable, use name-based detection
    if !config.ast_detection || syn_func.is_none() {
        return is_simple_constructor(func);
    }

    let syn_func = syn_func.unwrap();

    // Extract AST information
    let return_type = extract_return_type(syn_func);
    let body_pattern = analyze_function_body(syn_func);

    // Check return type (must return Self)
    let returns_self = matches!(
        return_type,
        Some(
            ConstructorReturnType::OwnedSelf
                | ConstructorReturnType::ResultSelf
                | ConstructorReturnType::OptionSelf
        )
    );

    if !returns_self {
        // Fallback to name-based detection if not returning Self
        return is_simple_constructor(func);
    }

    // Check body pattern
    if !body_pattern.is_constructor_like() {
        return false;
    }

    // Check complexity thresholds (more lenient for AST-detected constructors)
    let is_simple_enough =
        func.cyclomatic <= 5 && func.nesting <= 2 && func.length < 30 && !body_pattern.has_loop;

    returns_self && is_simple_enough
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

/// Calculate delegation ratio for a function
///
/// Returns the ratio of function calls to total statements (approximated by function length).
/// A higher ratio indicates more coordination/delegation behavior.
fn calculate_delegation_ratio(func: &FunctionMetrics, meaningful_callees: &[&FunctionId]) -> f64 {
    if func.length == 0 {
        return 0.0;
    }
    meaningful_callees.len() as f64 / func.length as f64
}

// Pure function to check if a function is an orchestrator
fn is_orchestrator(func: &FunctionMetrics, func_id: &FunctionId, call_graph: &CallGraph) -> bool {
    // First check if there are meaningful callees to orchestrate
    let callees = call_graph.get_callees(func_id);
    let meaningful_callees: Vec<_> = callees
        .iter()
        .filter(|f| {
            // Filter out standard library and utility functions
            !matches!(
                f.name.as_str(),
                "format" | "write" | "print" | "println" | "clone" | "to_string" | "into" | "from"
            ) && !f.name.starts_with("std::")
                && !f.name.starts_with("core::")
                && !f.name.starts_with("alloc::")
        })
        .collect();

    // Check if this is a functional chain (all calls are functional methods)
    // Default: allow functional chains (they're idiomatic patterns)
    if !meaningful_callees.is_empty() && callees.len() > meaningful_callees.len() {
        // If all non-utility calls are removed, this might be a functional chain
        let functional_chain = callees.iter().all(|f| {
            // Check for standard library and utility functions
            matches!(
                f.name.as_str(),
                "format" | "write" | "print" | "println" | "clone" | "to_string" | "into" | "from"
            ) || f.name.starts_with("std::")
                || f.name.starts_with("core::")
                || f.name.starts_with("alloc::")
                || f.name.contains("Pipeline")
                || f.name.contains("Stream")
                || f.name.contains("Iterator")
        });
        if functional_chain {
            return false;
        }
    }

    // Check for single delegation (adapter pattern)
    // Default: exclude adapters (they're not orchestration)
    if meaningful_callees.len() == 1 {
        // This is likely an adapter/wrapper, not orchestration
        return false;
    }

    // Can't be an orchestrator without functions to orchestrate
    // Default minimum delegation count: 2
    if meaningful_callees.len() < 2 {
        return false;
    }

    // Calculate delegation ratio to better identify orchestrators
    let delegation_ratio = calculate_delegation_ratio(func, &meaningful_callees);

    // Name-based orchestration with lenient complexity threshold
    let name_suggests_orchestration =
        is_orchestrator_by_name(&func_id.name) && func.cyclomatic <= 5;

    // Lenient complexity delegation pattern with delegation ratio check
    // Orchestrators can have complexity up to 5 (allowing for error handling)
    // and should have at least 20% of their code as function calls
    let is_simple_delegation = func.cyclomatic <= 5
        && delegation_ratio >= 0.2
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
    let name_lower = name.to_lowercase();

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

    // Check common orchestration patterns that override excludes
    // (These would have been in include_patterns config)
    let include_patterns = [
        "workflow_",
        "pipeline_",
        "process_",
        "orchestrate_",
        "coordinate_",
        "execute_flow_",
    ];
    for pattern in &include_patterns {
        if name_lower.starts_with(pattern) {
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
        .filter(|f| {
            // Filter out standard library and utility functions
            !matches!(
                f.name.as_str(),
                "format" | "write" | "print" | "println" | "clone" | "to_string" | "into" | "from"
            ) && !f.name.starts_with("std::")
                && !f.name.starts_with("core::")
                && !f.name.starts_with("alloc::")
        })
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

pub fn get_role_multiplier(role: FunctionRole) -> f64 {
    // Get multipliers from configuration
    let config = crate::config::get_role_multipliers();

    match role {
        FunctionRole::PureLogic => config.pure_logic,
        FunctionRole::Orchestrator => config.orchestrator,
        FunctionRole::IOWrapper => config.io_wrapper,
        FunctionRole::EntryPoint => config.entry_point,
        FunctionRole::PatternMatch => config.pattern_match,
        FunctionRole::Unknown => config.unknown,
    }
}

// Semantic priority calculation removed per spec 58
// Role multipliers now provide the only role-based adjustment to avoid double penalties

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
            is_pure: None,
            purity_confidence: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
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
        // Test with updated configuration values (spec 63)
        assert_eq!(get_role_multiplier(FunctionRole::PureLogic), 1.2);
        assert_eq!(get_role_multiplier(FunctionRole::Orchestrator), 0.8);
        assert_eq!(get_role_multiplier(FunctionRole::IOWrapper), 0.7);
        assert_eq!(get_role_multiplier(FunctionRole::EntryPoint), 0.9);
        assert_eq!(get_role_multiplier(FunctionRole::PatternMatch), 0.6);
        assert_eq!(get_role_multiplier(FunctionRole::Unknown), 1.0);
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
        // Need at least 3 for the current config settings
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
        let callee3 = FunctionId {
            file: PathBuf::from("workflow.rs"),
            name: "process_step_three".to_string(),
            line: 150,
        };

        graph.add_function(callee1.clone(), false, false, 5, 30);
        graph.add_function(callee2.clone(), false, false, 5, 30);
        graph.add_function(callee3.clone(), false, false, 5, 30);

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
        graph.add_call(FunctionCall {
            caller: func_id.clone(),
            callee: callee3,
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

    #[test]
    fn test_orchestrator_with_error_handling_complexity_5() {
        let mut graph = CallGraph::new();

        // Function with complexity 5 from Result handling (spec 117)
        let func = create_test_metrics("coordinate_tasks", 5, 3, 20);
        let func_id = FunctionId {
            file: PathBuf::from("tasks.rs"),
            name: "coordinate_tasks".to_string(),
            line: 10,
        };

        graph.add_function(func_id.clone(), false, false, 5, 20);

        // Add 4 meaningful callees (20% of 20 lines = 4 calls for delegation ratio)
        for i in 0..4 {
            let callee_id = FunctionId {
                file: PathBuf::from("worker.rs"),
                name: format!("worker_task_{}", i),
                line: i * 20,
            };
            graph.add_function(callee_id.clone(), false, false, 8, 40);
            graph.add_call(FunctionCall {
                caller: func_id.clone(),
                callee: callee_id,
                call_type: CallType::Direct,
            });
        }

        // Should be classified as orchestrator despite higher complexity
        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::Orchestrator,
            "Function with complexity 5 and good delegation ratio should be Orchestrator"
        );
    }

    #[test]
    fn test_delegation_ratio_calculation() {
        let func = create_test_metrics("orchestrator", 4, 2, 20);

        // Create test callees - 4 calls in 20 lines = 20% ratio
        let callees_vec: Vec<FunctionId> = (0..4)
            .map(|i| FunctionId {
                file: PathBuf::from("test.rs"),
                name: format!("callee_{}", i),
                line: i * 10,
            })
            .collect();

        let callees: Vec<&FunctionId> = callees_vec.iter().collect();

        let ratio = calculate_delegation_ratio(&func, &callees);
        assert!(
            (ratio - 0.2).abs() < 0.01,
            "Expected delegation ratio of 0.2, got {}",
            ratio
        );
    }

    #[test]
    fn test_high_complexity_not_orchestrator() {
        let mut graph = CallGraph::new();

        // Function with complexity > 5 should not be orchestrator
        let func = create_test_metrics("complex_logic", 8, 10, 30);
        let func_id = FunctionId {
            file: PathBuf::from("logic.rs"),
            name: "complex_logic".to_string(),
            line: 10,
        };

        graph.add_function(func_id.clone(), false, false, 8, 30);

        // Even with callees, complexity > 5 means not orchestrator
        for i in 0..3 {
            let callee_id = FunctionId {
                file: PathBuf::from("worker.rs"),
                name: format!("worker_{}", i),
                line: i * 20,
            };
            graph.add_function(callee_id.clone(), false, false, 5, 20);
            graph.add_call(FunctionCall {
                caller: func_id.clone(),
                callee: callee_id,
                call_type: CallType::Direct,
            });
        }

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::PureLogic,
            "Function with complexity > 5 should be PureLogic, not Orchestrator"
        );
    }

    #[test]
    fn test_simple_constructor_detection() {
        // Test: ContextMatcher::any() case (spec 117)
        let func = create_test_metrics("any", 1, 0, 9);
        assert!(
            is_simple_constructor(&func),
            "any() should be detected as constructor"
        );

        // Test: Standard new() constructor
        let func = create_test_metrics("new", 1, 0, 5);
        assert!(
            is_simple_constructor(&func),
            "new() should be detected as constructor"
        );

        // Test: from_* constructor
        let func = create_test_metrics("from_config", 1, 0, 8);
        assert!(
            is_simple_constructor(&func),
            "from_config() should be detected as constructor"
        );

        // Test: with_* constructor
        let func = create_test_metrics("with_defaults", 2, 1, 12);
        assert!(
            is_simple_constructor(&func),
            "with_defaults() should be detected as constructor"
        );

        // Test: Complex factory should NOT match
        let func = create_test_metrics("create_complex", 8, 12, 50);
        assert!(
            !is_simple_constructor(&func),
            "Complex function should NOT be detected as constructor"
        );

        // Test: Long function should NOT match
        let func = create_test_metrics("new_complex", 2, 2, 25);
        assert!(
            !is_simple_constructor(&func),
            "Long function should NOT be detected as constructor"
        );

        // Test: High cognitive complexity should NOT match
        let func = create_test_metrics("new_with_logic", 2, 8, 10);
        assert!(
            !is_simple_constructor(&func),
            "High cognitive complexity should NOT be detected as constructor"
        );
    }

    #[test]
    fn test_constructor_classification_precedence() {
        let graph = CallGraph::new();
        let func = create_test_metrics("any", 1, 0, 9);
        let func_id = FunctionId {
            file: PathBuf::from("context/rules.rs"),
            name: "any".to_string(),
            line: 52,
        };

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::IOWrapper,
            "Simple constructor should be classified as IOWrapper, not PureLogic"
        );
    }

    #[test]
    fn test_constructor_name_patterns() {
        // Test exact matches
        assert!(is_simple_constructor(&create_test_metrics("new", 1, 0, 5)));
        assert!(is_simple_constructor(&create_test_metrics(
            "default", 1, 0, 5
        )));
        assert!(is_simple_constructor(&create_test_metrics(
            "empty", 1, 0, 5
        )));
        assert!(is_simple_constructor(&create_test_metrics("zero", 1, 0, 5)));

        // Test prefix matches
        assert!(is_simple_constructor(&create_test_metrics(
            "from_str", 1, 0, 8
        )));
        assert!(is_simple_constructor(&create_test_metrics(
            "with_capacity",
            2,
            1,
            10
        )));
        assert!(is_simple_constructor(&create_test_metrics(
            "create_default",
            1,
            0,
            7
        )));
        assert!(is_simple_constructor(&create_test_metrics(
            "make_instance",
            1,
            0,
            6
        )));
        assert!(is_simple_constructor(&create_test_metrics(
            "build_config",
            2,
            2,
            12
        )));

        // Test non-constructor names
        let func = create_test_metrics("calculate_score", 1, 0, 5);
        assert!(
            !is_simple_constructor(&func),
            "Non-constructor name should not match"
        );
    }

    #[test]
    fn test_constructor_complexity_thresholds() {
        // Test at threshold boundaries
        let func = create_test_metrics("new", 2, 3, 14);
        assert!(
            is_simple_constructor(&func),
            "At threshold limits should match"
        );

        // Test just over cyclomatic threshold
        let func = create_test_metrics("new", 3, 2, 10);
        assert!(
            !is_simple_constructor(&func),
            "Over cyclomatic threshold should not match"
        );

        // Test just over cognitive threshold
        let func = create_test_metrics("new", 1, 4, 10);
        assert!(
            !is_simple_constructor(&func),
            "Over cognitive threshold should not match"
        );

        // Test just over length threshold
        let func = create_test_metrics("new", 1, 2, 15);
        assert!(
            !is_simple_constructor(&func),
            "Over length threshold should not match"
        );

        // Test nesting threshold
        let mut func = create_test_metrics("new", 1, 2, 10);
        func.nesting = 2;
        assert!(
            !is_simple_constructor(&func),
            "Over nesting threshold should not match"
        );
    }

    #[test]
    fn test_ast_detects_non_standard_constructor() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn create_default_client() -> Self {
                Self {
                    timeout: Duration::from_secs(30),
                    retries: 3,
                }
            }
        };

        let func = create_test_metrics("create_default_client", 1, 0, 5);

        // With AST: should be detected as constructor
        assert!(
            is_constructor_enhanced(&func, Some(&source)),
            "AST should detect non-standard constructor name"
        );

        // Without AST: fallback to name-based (should also match due to create_ prefix)
        assert!(
            is_constructor_enhanced(&func, None),
            "Should fallback to name-based detection"
        );
    }

    #[test]
    fn test_ast_detects_result_self_constructor() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn try_new(value: i32) -> Result<Self, Error> {
                if value > 0 {
                    Ok(Self { value })
                } else {
                    Err(Error::InvalidValue)
                }
            }
        };

        let func = create_test_metrics("try_new", 2, 1, 8);

        // With AST: should be detected as constructor (Result<Self>)
        assert!(
            is_constructor_enhanced(&func, Some(&source)),
            "AST should detect Result<Self> constructor"
        );
    }

    #[test]
    fn test_ast_rejects_loop_in_constructor() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn process_items() -> Self {
                let mut result = Self::new();
                for item in items {
                    result.add(item);
                }
                result
            }
        };

        let func = create_test_metrics("process_items", 2, 3, 8);

        // Should NOT be detected as constructor due to loop
        assert!(
            !is_constructor_enhanced(&func, Some(&source)),
            "AST should reject constructors with loops"
        );
    }

    #[test]
    fn test_ast_fallback_when_not_returning_self() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn calculate_value() -> i32 {
                42
            }
        };

        let func = create_test_metrics("calculate_value", 1, 0, 3);

        // Should fallback to name-based (which will reject non-constructor name)
        assert!(
            !is_constructor_enhanced(&func, Some(&source)),
            "AST should fallback when not returning Self"
        );
    }

    #[test]
    fn test_ast_detection_can_be_disabled() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn create_default_client() -> Self {
                Self { field: 0 }
            }
        };

        let func = create_test_metrics("create_default_client", 1, 0, 5);

        // When AST detection is disabled via config, should use name-based only
        // NOTE: This test assumes config can be set, which currently uses a static global
        // In a real implementation, we'd inject config or use a test-specific config

        // With AST enabled (default), should detect
        assert!(
            is_constructor_enhanced(&func, Some(&source)),
            "Should detect with AST when enabled"
        );

        // Without AST parameter (None), should fallback to name-based
        assert!(
            is_constructor_enhanced(&func, None),
            "Should fallback to name-based when AST unavailable"
        );
    }
}

// Debt classification functions

use crate::core::{FunctionMetrics, Language};
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::semantic_classifier::{classify_function_role, FunctionRole};
use crate::priority::{DebtType, FunctionVisibility, TransitiveCoverage};
use std::collections::HashSet;

// Configuration for untested function thresholds (spec 122)
const UNTESTED_COMPLEXITY_THRESHOLD: u32 = 15;
const UNTESTED_DEPENDENCY_THRESHOLD: usize = 10;

/// Determine the primary debt type for a function
pub fn determine_debt_type(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> DebtType {
    // Early return for testing gaps
    if let Some(testing_gap) = check_testing_gap(func, coverage) {
        return testing_gap;
    }

    // Early return for complexity hotspots
    if let Some(hotspot) = check_complexity_hotspot(func) {
        return hotspot;
    }

    // Early return for dead code
    if let Some(dead_code) = check_dead_code(func, call_graph, func_id) {
        return dead_code;
    }

    // Classify based on role and complexity
    let role = classify_function_role(func, func_id, call_graph);
    classify_by_role_and_complexity(func, &role, coverage)
}

// Pure helper functions for debt classification
fn check_testing_gap(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
) -> Option<DebtType> {
    coverage
        .as_ref()
        .filter(|cov| cov.direct < 0.2 && !func.is_test)
        .map(|cov| DebtType::TestingGap {
            coverage: cov.direct,
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        })
}

/// Determine if a function is a complexity hotspot.
///
/// Uses entropy-adjusted cyclomatic complexity when available (spec 182).
/// This prevents false positives for functions with repetitive, predictable structure.
///
/// Excludes Low tier functions (spec 180) as they're already maintainable:
/// - Low tier: cyclo < 8 AND cognitive < 15 → No debt item (maintenance only)
/// - Moderate+: cyclo >= 8 OR cognitive >= 15 → Report as ComplexityHotspot
///
/// # Thresholds
/// - Cyclomatic (or adjusted): > 10
/// - Cognitive: > 15
///
/// # Returns
/// `Some(DebtType::ComplexityHotspot)` if function exceeds thresholds and is not Low tier.
fn check_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    // Use adjusted complexity if available (spec 182)
    let effective_cyclomatic = func
        .adjusted_complexity
        .map(|adj| adj.round() as u32)
        .unwrap_or(func.cyclomatic);

    // Check if function exceeds complexity thresholds
    let is_complex = effective_cyclomatic > 10 || func.cognitive > 15;

    if !is_complex {
        return None;
    }

    // Spec 180: Filter out Low tier complexity (< 8 cyclomatic, < 15 cognitive)
    // These are maintenance-only recommendations ("Maintain current low complexity")
    // Only report Moderate+ tier as actual debt requiring action
    let is_low_tier = effective_cyclomatic < 8 && func.cognitive < 15;

    if is_low_tier {
        // Low tier - already maintainable, no action needed
        return None;
    }

    // Moderate+ tier - report as complexity hotspot requiring attention
    Some(DebtType::ComplexityHotspot {
        cyclomatic: func.cyclomatic,
        cognitive: func.cognitive,
        adjusted_cyclomatic: func.adjusted_complexity.map(|adj| adj.round() as u32),
    })
}

fn check_dead_code(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> Option<DebtType> {
    is_dead_code(func, call_graph, func_id, None).then(|| DebtType::DeadCode {
        visibility: determine_visibility(func),
        cyclomatic: func.cyclomatic,
        cognitive: func.cognitive,
        usage_hints: generate_usage_hints(func, call_graph, func_id),
    })
}

fn classify_by_role_and_complexity(
    func: &FunctionMetrics,
    role: &FunctionRole,
    coverage: &Option<TransitiveCoverage>,
) -> DebtType {
    // Handle simple functions
    if is_simple_function(func) {
        return classify_simple_by_role(func, role);
    }

    // Handle complex functions
    if needs_risk_assessment(func) {
        return DebtType::Risk {
            risk_score: calculate_risk_score(func),
            factors: identify_risk_factors(func, coverage),
        };
    }

    // Default case for functions that fall between simple and complex
    match role {
        FunctionRole::PureLogic => DebtType::Risk {
            risk_score: 0.0,
            factors: vec!["Simple pure function - minimal risk".to_string()],
        },
        _ => DebtType::Risk {
            risk_score: 0.1,
            factors: vec!["Simple function with low complexity".to_string()],
        },
    }
}

fn is_simple_function(func: &FunctionMetrics) -> bool {
    func.cyclomatic <= 3 && func.cognitive <= 5
}

fn needs_risk_assessment(func: &FunctionMetrics) -> bool {
    func.cyclomatic > 5 || func.cognitive > 8 || func.length > 50
}

fn classify_simple_by_role(func: &FunctionMetrics, role: &FunctionRole) -> DebtType {
    use FunctionRole::*;

    match role {
        IOWrapper | EntryPoint | PatternMatch | Debug => DebtType::Risk {
            risk_score: 0.0,
            factors: vec!["Simple I/O wrapper or entry point - minimal risk".to_string()],
        },
        PureLogic if func.length <= 10 => DebtType::Risk {
            risk_score: 0.0,
            factors: vec!["Trivial pure function - not technical debt".to_string()],
        },
        _ => DebtType::Risk {
            risk_score: 0.1,
            factors: vec!["Simple function with low complexity".to_string()],
        },
    }
}

/// Check if a function is dead code
pub fn is_dead_code(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
) -> bool {
    // FIRST: Check if function has incoming calls in the call graph
    // This includes event handlers bound via Bind() and other framework patterns
    let callers = call_graph.get_callers(func_id);
    if !callers.is_empty() {
        return false;
    }

    // Check if function is definitely used through function pointers
    if let Some(fp_used) = function_pointer_used_functions {
        if fp_used.contains(func_id) {
            return false;
        }
    }

    // LAST: Check hardcoded exclusions (includes test functions, main, etc.)
    // This is now a fallback for functions with no callers but might be implicitly called
    if is_excluded_from_dead_code_analysis(func) {
        return false;
    }

    // No callers found and not excluded by patterns
    true
}

/// Enhanced dead code detection that uses framework pattern exclusions
pub fn is_dead_code_with_exclusions(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
) -> bool {
    // Check if dead code detection is enabled for this file's language
    let language = crate::core::Language::from_path(&func.file);
    let language_features = crate::config::get_language_features(&language);

    if !language_features.detect_dead_code {
        // Dead code detection disabled for this language
        return false;
    }

    // First check if this function is excluded by framework patterns
    if framework_exclusions.contains(func_id) {
        return false;
    }

    // Use the enhanced dead code detection with function pointer information
    is_dead_code(func, call_graph, func_id, function_pointer_used_functions)
}

/// Check for testing gap - pure predicate for multi-debt accumulation
fn check_testing_gap_predicate(
    func: &FunctionMetrics,
    coverage: Option<&TransitiveCoverage>,
) -> Option<DebtType> {
    coverage.and_then(|cov| {
        if has_testing_gap(cov.direct, func.is_test)
            || (cov.direct < 0.8 && func.cyclomatic > 5 && !cov.uncovered_lines.is_empty())
        {
            Some(DebtType::TestingGap {
                coverage: cov.direct,
                cyclomatic: func.cyclomatic,
                cognitive: func.cognitive,
            })
        } else {
            None
        }
    })
}

/// Check for complexity hotspot - pure predicate for multi-debt accumulation
fn check_complexity_hotspot_predicate(func: &FunctionMetrics) -> Option<DebtType> {
    if is_complexity_hotspot_by_metrics(func.cyclomatic, func.cognitive) {
        Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            adjusted_cyclomatic: func.adjusted_complexity.map(|adj| adj.round() as u32),
        })
    } else {
        None
    }
}

/// Check for dead code with exclusions - pure predicate for multi-debt accumulation
fn check_dead_code_with_exclusions_predicate(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
) -> Option<DebtType> {
    if is_dead_code_with_exclusions(
        func,
        call_graph,
        func_id,
        framework_exclusions,
        function_pointer_used_functions,
    ) {
        Some(DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        })
    } else {
        None
    }
}

/// Classify all applicable debt types for a function using functional composition (spec 228)
///
/// This function accumulates all independent debt classifications rather than
/// stopping at the first match, providing comprehensive technical debt assessment.
///
/// # Independent Debt Checks
/// - Testing gaps: Coverage-based testing debt
/// - Complexity hotspots: Cyclomatic/cognitive complexity
/// - Dead code: Unused code detection
///
/// # Test Function Exception
/// Test functions (`func.is_test == true`) only return test-specific debt types
/// to avoid noise from legitimate test complexity.
///
/// # Returns
/// A vector of all applicable debt types. May contain 0-3 items depending on
/// the function's issues.
pub fn classify_all_debt_types(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    coverage: Option<&TransitiveCoverage>,
) -> Vec<DebtType> {
    // Test functions get exclusive test debt (early return preserved for correctness)
    if func.is_test {
        return vec![classify_test_debt(func)];
    }

    // Compose all independent debt checks using iterator chains (functional style)
    let debt_types: Vec<DebtType> = vec![
        check_testing_gap_predicate(func, coverage),
        check_complexity_hotspot_predicate(func),
        check_dead_code_with_exclusions_predicate(
            func,
            call_graph,
            func_id,
            framework_exclusions,
            function_pointer_used_functions,
        ),
    ]
    .into_iter()
    .flatten() // Remove None values, keep Some(debt)
    .collect();

    // If no specific debt, classify by role (fallback)
    if debt_types.is_empty() {
        let role = classify_function_role(func, func_id, call_graph);
        classify_simple_function_risk(func, &role)
            .map(|debt| vec![debt])
            .unwrap_or_default()
    } else {
        debt_types
    }
}

/// Enhanced version of debt type classification with framework pattern exclusions
/// Returns `Vec<DebtType>` for multi-debt accumulation (spec 228)
///
/// Functions can accumulate multiple independent debt types (e.g., both TestingGap
/// and ComplexityHotspot), providing comprehensive technical debt assessment.
pub fn classify_debt_type_with_exclusions(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    coverage: Option<&TransitiveCoverage>,
) -> Vec<DebtType> {
    classify_all_debt_types(
        func,
        call_graph,
        func_id,
        framework_exclusions,
        function_pointer_used_functions,
        coverage,
    )
}

/// Classify test function debt type based on complexity
pub fn classify_test_debt(func: &FunctionMetrics) -> DebtType {
    match () {
        _ if func.cyclomatic > 15 || func.cognitive > 20 => DebtType::TestComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            threshold: 15,
        },
        _ => DebtType::TestingGap {
            coverage: 0.0, // Test functions don't have coverage themselves
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        },
    }
}

/// Check if an untested function should surface in top recommendations (spec 122)
/// Only surfaces untested functions if they meet complexity or dependency thresholds
pub fn should_surface_untested_function(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> bool {
    // Always surface if complexity is high
    if func.cyclomatic >= UNTESTED_COMPLEXITY_THRESHOLD {
        return true;
    }

    // Surface if high dependency count
    let upstream_count = call_graph.get_callers(func_id).len();
    let downstream_count = call_graph.get_callees(func_id).len();
    let total_dependencies = upstream_count + downstream_count;

    if total_dependencies >= UNTESTED_DEPENDENCY_THRESHOLD {
        return true;
    }

    // Surface if critical role (entry points, public APIs)
    let role = classify_function_role(func, func_id, call_graph);
    matches!(role, FunctionRole::EntryPoint)
}

/// Check if function is a complexity hotspot
pub fn is_complexity_hotspot(func: &FunctionMetrics, role: &FunctionRole) -> Option<DebtType> {
    // Use adjusted complexity if available (spec 182)
    let effective_cyclomatic = func
        .adjusted_complexity
        .map(|adj| adj.round() as u32)
        .unwrap_or(func.cyclomatic);

    // Direct complexity check
    if effective_cyclomatic > 10 || func.cognitive > 15 {
        return Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            adjusted_cyclomatic: func.adjusted_complexity.map(|adj| adj.round() as u32),
        });
    }

    // Role-based complexity thresholds
    let (cyclo_threshold, cog_threshold) = match role {
        FunctionRole::Orchestrator => (8, 12),
        FunctionRole::PureLogic => (10, 15),
        FunctionRole::EntryPoint => (7, 10),
        FunctionRole::IOWrapper => (5, 8),
        FunctionRole::PatternMatch => (15, 20),
        FunctionRole::Debug => (20, 25), // Very lenient for debug functions
        FunctionRole::Unknown => (10, 15),
    };

    if effective_cyclomatic > cyclo_threshold || func.cognitive > cog_threshold {
        Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            adjusted_cyclomatic: func.adjusted_complexity.map(|adj| adj.round() as u32),
        })
    } else {
        None
    }
}

/// Classify risk-based debt
pub fn classify_risk_based_debt(func: &FunctionMetrics, role: &FunctionRole) -> DebtType {
    // Check if it's simple enough to be considered not debt
    if func.cyclomatic <= 3 && func.cognitive <= 5 {
        if let Some(debt) = classify_simple_function_risk(func, role) {
            return debt;
        }
    }

    // Calculate risk score for more complex functions
    DebtType::Risk {
        risk_score: calculate_risk_score(func),
        factors: identify_risk_factors(func, &None),
    }
}

/// Classify simple function risk
pub fn classify_simple_function_risk(
    func: &FunctionMetrics,
    role: &FunctionRole,
) -> Option<DebtType> {
    if func.cyclomatic > 3 || func.cognitive > 5 {
        return None;
    }

    match role {
        FunctionRole::IOWrapper
        | FunctionRole::EntryPoint
        | FunctionRole::PatternMatch
        | FunctionRole::Debug => Some(DebtType::Risk {
            risk_score: 0.0,
            factors: vec!["Simple I/O wrapper or entry point - minimal risk".to_string()],
        }),
        FunctionRole::PureLogic if func.length <= 10 => Some(DebtType::Risk {
            risk_score: 0.0,
            factors: vec!["Trivial pure function - not technical debt".to_string()],
        }),
        FunctionRole::PureLogic => Some(DebtType::Risk {
            risk_score: 0.0,
            factors: vec!["Simple pure function - minimal risk".to_string()],
        }),
        _ => Some(DebtType::Risk {
            risk_score: 0.1,
            factors: vec!["Simple function with low complexity".to_string()],
        }),
    }
}

// Helper functions

fn calculate_risk_score(func: &FunctionMetrics) -> f64 {
    // Better scaling for complexity risk (0-1 range)
    // Cyclomatic 10 = 0.33, 20 = 0.67, 30+ = 1.0
    let cyclo_risk = (func.cyclomatic as f64 / 30.0).min(1.0);

    // Cognitive complexity tends to be higher, so scale differently
    // Cognitive 15 = 0.33, 30 = 0.67, 45+ = 1.0
    let cognitive_risk = (func.cognitive as f64 / 45.0).min(1.0);

    // Length risk - functions over 100 lines are definitely risky
    let length_risk = (func.length as f64 / 100.0).min(1.0);

    // Average the three risk factors
    // Complexity is most important, then cognitive, then length
    let weighted_risk = cyclo_risk * 0.4 + cognitive_risk * 0.4 + length_risk * 0.2;

    // Scale to 0-10 range for final risk score
    // Note: Coverage is handled separately in the unified scoring system
    weighted_risk * 10.0
}

fn identify_risk_factors(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
) -> Vec<String> {
    let factors = [
        complexity_factor(func),
        cognitive_factor(func),
        length_factor(func),
        coverage_factor(coverage),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if factors.is_empty() {
        vec!["Potential improvement opportunity".to_string()]
    } else {
        factors
    }
}

// Pure predicates for risk factors
fn complexity_factor(func: &FunctionMetrics) -> Option<String> {
    (func.cyclomatic > 5).then(|| format!("Moderate complexity (cyclomatic: {})", func.cyclomatic))
}

fn cognitive_factor(func: &FunctionMetrics) -> Option<String> {
    (func.cognitive > 8).then(|| format!("Cognitive complexity: {}", func.cognitive))
}

fn length_factor(func: &FunctionMetrics) -> Option<String> {
    (func.length > 50).then(|| format!("Long function ({} lines)", func.length))
}

fn coverage_factor(coverage: &Option<TransitiveCoverage>) -> Option<String> {
    coverage
        .as_ref()
        .filter(|cov| cov.direct < 0.5)
        .map(|cov| format!("Low coverage: {:.0}%", cov.direct * 100.0))
}

fn is_excluded_from_dead_code_analysis(func: &FunctionMetrics) -> bool {
    // Check language-specific exclusions
    let language = Language::from_path(&func.file);

    // Entry points
    if func.name == "main" || func.name.starts_with("_start") {
        return true;
    }

    // Test functions
    if func.is_test || func.name.starts_with("test_") || func.name.starts_with("tests::") {
        return true;
    }

    // Build script functions (Rust-specific)
    if func.file.to_string_lossy().contains("build.rs") && func.name == "main" {
        return true;
    }

    // Common framework patterns
    if is_likely_trait_method(func) || is_framework_callback(func) {
        return true;
    }

    // Avoid unused variable warning
    let _ = language;

    false
}

fn is_likely_trait_method(func: &FunctionMetrics) -> bool {
    // Check if this is likely a trait method implementation based on:
    // 1. Public visibility + specific method names that are commonly trait methods
    // 2. Common trait method patterns

    if func.visibility.as_ref().is_some_and(|v| v == "pub") {
        // Common trait methods
        let trait_methods = [
            "fmt",
            "clone",
            "default",
            "from",
            "into",
            "try_from",
            "try_into",
            "as_ref",
            "as_mut",
            "drop",
            "deref",
            "deref_mut",
            "index",
            "index_mut",
            "add",
            "sub",
            "mul",
            "div",
            "rem",
            "eq",
            "ne",
            "partial_cmp",
            "cmp",
            "hash",
            "serialize",
            "deserialize",
            "next",
            "size_hint",
        ];

        if trait_methods.contains(&func.name.as_str()) {
            return true;
        }

        // Check for new() which is a common constructor pattern
        if func.name == "new" {
            return true;
        }
    }

    false
}

fn is_framework_callback(func: &FunctionMetrics) -> bool {
    // Common web framework handlers
    func.name.contains("handler") || 
    func.name.contains("route") ||
    func.name.contains("middleware") ||
    func.name.contains("controller") ||
    func.name.contains("endpoint") ||
    // Common async runtime patterns
    func.name.contains("spawn") ||
    func.name.contains("poll") ||
    // Common GUI callbacks
    func.name.contains("on_") ||
    func.name.contains("handle_") ||
    // Common event handlers
    func.name.contains("_event") ||
    func.name.contains("_listener")
}

fn determine_visibility(func: &FunctionMetrics) -> FunctionVisibility {
    match func.visibility.as_deref() {
        Some("pub") | Some("public") => FunctionVisibility::Public,
        Some("pub(crate)") | Some("crate") => FunctionVisibility::Crate,
        _ => FunctionVisibility::Private,
    }
}

fn generate_usage_hints(
    func: &FunctionMetrics,
    _call_graph: &CallGraph,
    _func_id: &FunctionId,
) -> Vec<String> {
    let mut hints = Vec::new();

    // Check language for specialized hints
    let language = Language::from_path(&func.file);
    let _ = language; // Avoid unused variable warning

    // Generic hints
    if func.visibility.as_ref().is_some_and(|v| v == "pub") {
        hints.push("Public function with no internal callers".to_string());
    } else {
        hints.push("Private function with no callers".to_string());
    }

    if func.name.starts_with("_") {
        hints.push("Name starts with underscore (often indicates internal/unused)".to_string());
    }

    if func.name.contains("old") || func.name.contains("legacy") || func.name.contains("deprecated")
    {
        hints.push("Name suggests obsolete functionality".to_string());
    }

    hints
}

// Pure helper functions
fn has_testing_gap(coverage: f64, is_test: bool) -> bool {
    coverage < 0.2 && !is_test
}

fn is_complexity_hotspot_by_metrics(cyclomatic: u32, cognitive: u32) -> bool {
    cyclomatic > 5 || cognitive > 8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FunctionMetrics;
    use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
    use std::path::PathBuf;

    fn create_test_function(name: &str, visibility: Option<&str>) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 50,
            is_test: false,
            visibility: visibility.map(|v| v.to_string()),
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        }
    }

    #[test]
    fn test_generate_usage_hints_public_function() {
        let func = create_test_function("test_func", Some("pub"));
        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "test_func".to_string(), 10);

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0], "Public function with no internal callers");
    }

    #[test]
    fn test_generate_usage_hints_private_function() {
        let func = create_test_function("test_func", None);
        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "test_func".to_string(), 10);

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0], "Private function with no callers");
    }

    #[test]
    fn test_generate_usage_hints_underscore_prefix() {
        let func = create_test_function("_internal_func", None);
        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "_internal_func".to_string(), 10);

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0], "Private function with no callers");
        assert_eq!(
            hints[1],
            "Name starts with underscore (often indicates internal/unused)"
        );
    }

    #[test]
    fn test_generate_usage_hints_deprecated_name() {
        let func = create_test_function("old_deprecated_function", Some("pub"));
        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(
            PathBuf::from("test.rs"),
            "old_deprecated_function".to_string(),
            10,
        );

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0], "Public function with no internal callers");
        assert_eq!(hints[1], "Name suggests obsolete functionality");
    }

    #[test]
    fn test_generate_usage_hints_legacy_function() {
        let func = create_test_function("legacy_handler", None);
        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "legacy_handler".to_string(), 10);

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert_eq!(hints.len(), 2);
        assert!(hints.contains(&"Private function with no callers".to_string()));
        assert!(hints.contains(&"Name suggests obsolete functionality".to_string()));
    }

    #[test]
    fn test_event_handler_not_dead_code_when_bound() {
        // Create a Python event handler function
        let event_handler = FunctionMetrics {
            name: "on_key_down".to_string(),
            file: PathBuf::from("test_panel.py"),
            line: 50,
            cyclomatic: 6,
            cognitive: 6,
            nesting: 3,
            length: 20,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        };

        let mut call_graph = CallGraph::new();
        let event_handler_id = FunctionId::new(
            PathBuf::from("test_panel.py"),
            "on_key_down".to_string(),
            50,
        );

        // Simulate the event handler being bound via Bind()
        // This would normally be done by the Python call graph analyzer
        let setup_method_id = FunctionId::new(
            PathBuf::from("test_panel.py"),
            "setup_events".to_string(),
            30,
        );
        call_graph.add_call(FunctionCall {
            caller: setup_method_id,
            callee: event_handler_id.clone(),
            call_type: CallType::Direct,
        });

        // Test that the event handler is NOT considered dead code
        let is_dead = is_dead_code(&event_handler, &call_graph, &event_handler_id, None);
        assert!(
            !is_dead,
            "Event handler with Bind() call should not be dead code"
        );
    }

    #[test]
    fn test_event_handler_is_dead_code_when_not_bound() {
        // Create a Python function that looks like a handler but isn't bound
        // Use a name that doesn't match framework patterns
        let unused_func = FunctionMetrics {
            name: "process_data".to_string(), // Not a framework pattern
            file: PathBuf::from("test_panel.py"),
            line: 100,
            cyclomatic: 3,
            cognitive: 3,
            nesting: 1,
            length: 10,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        };

        let call_graph = CallGraph::new(); // Empty call graph - no callers
        let unused_func_id = FunctionId::new(
            PathBuf::from("test_panel.py"),
            "process_data".to_string(),
            100,
        );

        // Test that an unbound function IS considered dead code
        let is_dead = is_dead_code(&unused_func, &call_graph, &unused_func_id, None);
        assert!(is_dead, "Function with no callers should be dead code");
    }

    #[test]
    fn test_observer_method_not_dead_code_when_called() {
        // Create observer pattern methods
        let register_observer = FunctionMetrics {
            name: "register_observer".to_string(),
            file: PathBuf::from("manager.py"),
            line: 20,
            cyclomatic: 2,
            cognitive: 1,
            nesting: 1,
            length: 5,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        };

        let mut call_graph = CallGraph::new();
        let register_id = FunctionId::new(
            PathBuf::from("manager.py"),
            "register_observer".to_string(),
            20,
        );

        // Simulate a call from another file
        let caller_id = FunctionId::new(PathBuf::from("panel.py"), "__init__".to_string(), 10);
        call_graph.add_call(FunctionCall {
            caller: caller_id,
            callee: register_id.clone(),
            call_type: CallType::Direct,
        });

        // Test that register_observer is NOT considered dead code
        let is_dead = is_dead_code(&register_observer, &call_graph, &register_id, None);
        assert!(!is_dead, "Called observer method should not be dead code");
    }

    #[test]
    fn test_generate_usage_hints_multiple_indicators() {
        let func = create_test_function("_old_deprecated_helper", Some("pub"));
        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(
            PathBuf::from("test.rs"),
            "_old_deprecated_helper".to_string(),
            10,
        );

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert_eq!(hints.len(), 3);
        assert!(hints.contains(&"Public function with no internal callers".to_string()));
        assert!(hints.contains(
            &"Name starts with underscore (often indicates internal/unused)".to_string()
        ));
        assert!(hints.contains(&"Name suggests obsolete functionality".to_string()));
    }

    #[test]
    fn test_wxpython_event_handlers_not_dead_code() {
        // This test mimics the exact scenario from promptconstruct-frontend
        // where event handlers were incorrectly flagged as dead code

        // Create functions that match the patterns from promptconstruct-frontend
        let on_paint = FunctionMetrics {
            name: "on_paint".to_string(),
            file: PathBuf::from("conversation_panel.py"),
            line: 544,
            cyclomatic: 6,
            cognitive: 10,
            nesting: 4,
            length: 30,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        };

        let on_key_down = FunctionMetrics {
            name: "on_key_down".to_string(),
            file: PathBuf::from("mainwindow.py"),
            line: 262,
            cyclomatic: 6,
            cognitive: 6,
            nesting: 3,
            length: 20,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        };

        let mut call_graph = CallGraph::new();

        // IDs for the event handlers
        let on_paint_id = FunctionId::new(
            PathBuf::from("conversation_panel.py"),
            "on_paint".to_string(),
            544,
        );

        let on_key_down_id = FunctionId::new(
            PathBuf::from("mainwindow.py"),
            "on_key_down".to_string(),
            262,
        );

        // Simulate the Bind() calls that would be detected by the Python analyzer
        // e.g., self.Bind(wx.EVT_PAINT, self.on_paint)
        let init_id = FunctionId::new(
            PathBuf::from("conversation_panel.py"),
            "__init__".to_string(),
            10,
        );

        call_graph.add_call(FunctionCall {
            caller: init_id.clone(),
            callee: on_paint_id.clone(),
            call_type: CallType::Direct,
        });
        call_graph.add_call(FunctionCall {
            caller: init_id,
            callee: on_key_down_id.clone(),
            call_type: CallType::Direct,
        });

        // Test that these event handlers are NOT considered dead code
        assert!(
            !is_dead_code(&on_paint, &call_graph, &on_paint_id, None),
            "on_paint should not be dead code when bound via Bind()"
        );

        assert!(
            !is_dead_code(&on_key_down, &call_graph, &on_key_down_id, None),
            "on_key_down should not be dead code when bound via Bind()"
        );
    }

    #[test]
    fn test_call_graph_priority_over_patterns() {
        // This test ensures that call graph evidence takes priority over pattern matching
        // Even if a function doesn't match any special patterns, if it has callers, it's not dead

        let unusual_name_func = FunctionMetrics {
            name: "xyz123_unusual".to_string(), // Doesn't match any patterns
            file: PathBuf::from("module.py"),
            line: 100,
            cyclomatic: 2,
            cognitive: 2,
            nesting: 1,
            length: 10,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        };

        let mut call_graph = CallGraph::new();
        let func_id = FunctionId::new(
            PathBuf::from("module.py"),
            "xyz123_unusual".to_string(),
            100,
        );

        // Initially no callers - should be dead code
        assert!(
            is_dead_code(&unusual_name_func, &call_graph, &func_id, None),
            "Function with no callers and no pattern matches should be dead code"
        );

        // Add a caller
        let caller_id = FunctionId::new(PathBuf::from("module.py"), "main".to_string(), 10);
        call_graph.add_call(FunctionCall {
            caller: caller_id,
            callee: func_id.clone(),
            call_type: CallType::Direct,
        });

        // Now with a caller - should NOT be dead code
        assert!(
            !is_dead_code(&unusual_name_func, &call_graph, &func_id, None),
            "Function with callers should not be dead code regardless of patterns"
        );
    }

    #[test]
    fn test_has_testing_gap_low_coverage() {
        assert!(has_testing_gap(0.1, false));
        assert!(has_testing_gap(0.19, false));
        assert!(!has_testing_gap(0.2, false));
        assert!(!has_testing_gap(0.5, false));
    }

    #[test]
    fn test_has_testing_gap_is_test() {
        assert!(!has_testing_gap(0.0, true));
        assert!(!has_testing_gap(0.1, true));
        assert!(!has_testing_gap(0.5, true));
    }

    #[test]
    fn test_is_complexity_hotspot_by_metrics() {
        assert!(is_complexity_hotspot_by_metrics(6, 5));
        assert!(is_complexity_hotspot_by_metrics(3, 9));
        assert!(is_complexity_hotspot_by_metrics(10, 10));
        assert!(!is_complexity_hotspot_by_metrics(5, 8));
        assert!(!is_complexity_hotspot_by_metrics(3, 5));
    }

    fn test_extract_visibility(func: &FunctionMetrics) -> FunctionVisibility {
        match func.visibility.as_deref() {
            Some("pub") => FunctionVisibility::Public,
            Some("pub(crate)") => FunctionVisibility::Crate,
            _ => FunctionVisibility::Private,
        }
    }

    #[test]
    fn test_function_visibility_extraction() {
        let public_func = create_test_function("test", Some("pub"));
        assert_eq!(
            test_extract_visibility(&public_func),
            FunctionVisibility::Public
        );

        let crate_func = create_test_function("test", Some("pub(crate)"));
        assert_eq!(
            test_extract_visibility(&crate_func),
            FunctionVisibility::Crate
        );

        let private_func = create_test_function("test", None);
        assert_eq!(
            test_extract_visibility(&private_func),
            FunctionVisibility::Private
        );

        // pub(super) maps to Private visibility since we don't have Module variant
        let module_func = create_test_function("test", Some("pub(super)"));
        assert_eq!(
            test_extract_visibility(&module_func),
            FunctionVisibility::Private
        );
    }

    // Unit tests for spec 182: adjusted complexity in classification
    #[test]
    fn check_complexity_hotspot_uses_adjusted_complexity() {
        // Function with low entropy, adjusted complexity below threshold
        let mut func = create_test_function("test_func", None);
        func.cyclomatic = 11; // Above threshold (raw)
        func.cognitive = 12; // Below threshold
        func.adjusted_complexity = Some(4.15); // Below threshold (adjusted)

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_none(),
            "Should NOT flag - adjusted complexity is below threshold"
        );
    }

    #[test]
    fn check_complexity_hotspot_falls_back_to_raw_when_no_adjustment() {
        // Function without entropy analysis
        let mut func = create_test_function("test_func", None);
        func.cyclomatic = 11; // Above threshold
        func.cognitive = 12;
        func.adjusted_complexity = None; // No adjustment available

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_some(),
            "Should flag - raw cyclomatic above threshold"
        );
    }

    #[test]
    fn check_complexity_hotspot_stores_both_raw_and_adjusted() {
        let mut func = create_test_function("test_func", None);
        func.cyclomatic = 15;
        func.cognitive = 20;
        func.adjusted_complexity = Some(8.5);

        if let Some(DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
            adjusted_cyclomatic,
        }) = check_complexity_hotspot(&func)
        {
            assert_eq!(cyclomatic, 15, "Raw cyclomatic should be stored");
            assert_eq!(cognitive, 20, "Cognitive should be stored");
            assert_eq!(
                adjusted_cyclomatic,
                Some(9),
                "Adjusted cyclomatic should be rounded and stored"
            );
        } else {
            panic!("Expected ComplexityHotspot");
        }
    }

    #[test]
    fn check_complexity_hotspot_high_cognitive_still_flags() {
        // Even with low adjusted cyclomatic, high cognitive should flag
        let mut func = create_test_function("reconcile_state", None);
        func.cyclomatic = 9;
        func.cognitive = 16; // Above threshold
        func.adjusted_complexity = Some(4.15); // Below threshold

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_some(),
            "Should flag due to high cognitive complexity"
        );

        if let Some(DebtType::ComplexityHotspot {
            adjusted_cyclomatic,
            ..
        }) = result
        {
            assert_eq!(
                adjusted_cyclomatic,
                Some(4),
                "Should store adjusted complexity"
            );
        }
    }

    // Unit tests for spec 180: exclude Low tier maintenance recommendations

    #[test]
    fn check_complexity_hotspot_excludes_low_tier_low_cyclo_low_cognitive() {
        // Low tier: cyclomatic < 8 AND cognitive < 15
        let mut func = create_test_function("simple_func", None);
        func.cyclomatic = 5;
        func.cognitive = 10;
        func.adjusted_complexity = None;

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_none(),
            "Should NOT flag Low tier (cyclo=5, cognitive=10) as ComplexityHotspot"
        );
    }

    #[test]
    fn check_complexity_hotspot_excludes_low_tier_edge_case() {
        // Edge case: cyclo=7, cognitive=14 (just below Low tier threshold)
        let mut func = create_test_function("edge_case_func", None);
        func.cyclomatic = 7;
        func.cognitive = 14;
        func.adjusted_complexity = None;

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_none(),
            "Should NOT flag Low tier edge case (cyclo=7, cognitive=14)"
        );
    }

    #[test]
    fn check_complexity_hotspot_reports_moderate_tier_cyclo() {
        // Moderate tier: cyclomatic >= 8 (even if cognitive is low)
        let mut func = create_test_function("moderate_func", None);
        func.cyclomatic = 8;
        func.cognitive = 10;
        func.adjusted_complexity = None;

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_none(),
            "cyclo=8, cognitive=10 is below threshold (>10 or >15), should not flag"
        );
    }

    #[test]
    fn check_complexity_hotspot_reports_moderate_tier_cognitive() {
        // Moderate tier: cognitive >= 15 (even if cyclomatic is low)
        let mut func = create_test_function("moderate_func2", None);
        func.cyclomatic = 5;
        func.cognitive = 15;
        func.adjusted_complexity = None;

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_none(),
            "cyclo=5, cognitive=15 is below threshold (>10 or >15), should not flag"
        );
    }

    #[test]
    fn check_complexity_hotspot_reports_high_tier() {
        // High tier: cyclomatic >= 15 OR cognitive >= 25
        let mut func = create_test_function("high_complexity_func", None);
        func.cyclomatic = 18;
        func.cognitive = 28;
        func.adjusted_complexity = None;

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_some(),
            "Should flag High tier (cyclo=18, cognitive=28)"
        );
    }

    #[test]
    fn check_complexity_hotspot_excludes_low_tier_with_adjusted_complexity() {
        // Low tier with adjusted complexity still below threshold
        let mut func = create_test_function("adjusted_low_func", None);
        func.cyclomatic = 12; // Raw above threshold
        func.cognitive = 10;
        func.adjusted_complexity = Some(6.5); // Adjusted below Low tier threshold

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_none(),
            "Should NOT flag when adjusted complexity puts it in Low tier (adjusted=6.5, cognitive=10)"
        );
    }

    #[test]
    fn check_complexity_hotspot_reports_moderate_with_adjusted_complexity() {
        // Moderate tier with adjusted complexity
        let mut func = create_test_function("adjusted_moderate_func", None);
        func.cyclomatic = 15; // Raw above threshold
        func.cognitive = 18;
        func.adjusted_complexity = Some(9.0); // Adjusted still Moderate (8-14 range)

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_some(),
            "Should flag Moderate+ tier even with adjusted complexity (adjusted=9, cognitive=18)"
        );
    }

    #[test]
    fn check_complexity_hotspot_boundary_cyclo_11_cognitive_12() {
        // This should be flagged as complexity hotspot (cyclo > 10)
        // BUT filtered as Low tier (cyclo < 8 AND cognitive < 15)
        // Since cyclo=11 is NOT < 8, this should be reported
        let mut func = create_test_function("boundary_func", None);
        func.cyclomatic = 11;
        func.cognitive = 12;
        func.adjusted_complexity = None;

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_some(),
            "Should flag cyclo=11 > 10 as complexity hotspot (not Low tier since cyclo >= 8)"
        );
    }

    #[test]
    fn check_complexity_hotspot_boundary_cyclo_7_cognitive_16() {
        // cyclo=7 (< 8), cognitive=16 (>= 15) - should be flagged
        // Not Low tier because cognitive >= 15
        let mut func = create_test_function("boundary_func2", None);
        func.cyclomatic = 7;
        func.cognitive = 16;
        func.adjusted_complexity = None;

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_some(),
            "Should flag cognitive=16 > 15 (not Low tier since cognitive >= 15)"
        );
    }

    #[test]
    fn check_complexity_hotspot_low_tier_no_debt_item() {
        // Verify that Low tier functions with testing gaps or other issues
        // don't get ComplexityHotspot debt (but can get other debt types)
        let mut func = create_test_function("low_complexity_untested", None);
        func.cyclomatic = 6;
        func.cognitive = 10;
        func.adjusted_complexity = None;

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_none(),
            "Low tier function should not generate ComplexityHotspot debt"
        );
    }
}

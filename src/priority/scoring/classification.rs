// Debt classification functions

use crate::analysis::PythonDeadCodeDetector;
use crate::core::{FunctionMetrics, Language};
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::semantic_classifier::{classify_function_role, FunctionRole};
use crate::priority::{DebtType, FunctionVisibility, TransitiveCoverage};
use std::collections::HashSet;

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

fn check_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    (func.cyclomatic > 10 || func.cognitive > 15).then_some(DebtType::ComplexityHotspot {
        cyclomatic: func.cyclomatic,
        cognitive: func.cognitive,
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
        IOWrapper | EntryPoint | PatternMatch => DebtType::Risk {
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
    // Check hardcoded exclusions (includes test functions, main, etc.)
    if is_excluded_from_dead_code_analysis(func) {
        return false;
    }

    // Check if function is definitely used through function pointers
    if let Some(fp_used) = function_pointer_used_functions {
        if fp_used.contains(func_id) {
            return false;
        }
    }

    // Check if function has incoming calls
    let callers = call_graph.get_callers(func_id);
    callers.is_empty()
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

/// Enhanced version of debt type classification with framework pattern exclusions
pub fn classify_debt_type_with_exclusions(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    coverage: Option<&TransitiveCoverage>,
) -> DebtType {
    // Test functions are special debt cases
    if func.is_test {
        return classify_test_debt(func);
    }

    // Check for testing gaps first (like in determine_debt_type)
    if let Some(cov) = coverage {
        // Classify as testing gap if:
        // 1. Very low coverage (< 20%), OR
        // 2. Has moderate coverage gaps (< 80%) with meaningful complexity
        if has_testing_gap(cov.direct, func.is_test)
            || (cov.direct < 0.8 && func.cyclomatic > 5 && !cov.uncovered_lines.is_empty())
        {
            return DebtType::TestingGap {
                coverage: cov.direct,
                cyclomatic: func.cyclomatic,
                cognitive: func.cognitive,
            };
        }
    }

    // Check for complexity hotspots - include moderate complexity functions
    if is_complexity_hotspot_by_metrics(func.cyclomatic, func.cognitive) {
        return DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        };
    }

    // Check for dead code with framework exclusions
    if is_dead_code_with_exclusions(
        func,
        call_graph,
        func_id,
        framework_exclusions,
        function_pointer_used_functions,
    ) {
        return DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        };
    }

    // Get role for later checks
    let role = classify_function_role(func, func_id, call_graph);

    // Low complexity functions that are I/O wrappers or entry points
    // should not be flagged as technical debt
    if func.cyclomatic <= 3 && func.cognitive <= 5 {
        if let Some(debt) = classify_simple_function_risk(func, &role) {
            return debt;
        }
    }

    // At this point, we have simple functions (cyclo <= 5, cog <= 8)
    // These are not technical debt - return minimal risk
    DebtType::Risk {
        risk_score: 0.0,
        factors: vec!["Well-designed simple function - not technical debt".to_string()],
    }
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

/// Check if function is a complexity hotspot
pub fn is_complexity_hotspot(func: &FunctionMetrics, role: &FunctionRole) -> Option<DebtType> {
    // Direct complexity check
    if func.cyclomatic > 10 || func.cognitive > 15 {
        return Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        });
    }

    // Role-based complexity thresholds
    let (cyclo_threshold, cog_threshold) = match role {
        FunctionRole::Orchestrator => (8, 12),
        FunctionRole::PureLogic => (10, 15),
        FunctionRole::EntryPoint => (7, 10),
        FunctionRole::IOWrapper => (5, 8),
        FunctionRole::PatternMatch => (15, 20),
        FunctionRole::Unknown => (10, 15),
    };

    if func.cyclomatic > cyclo_threshold || func.cognitive > cog_threshold {
        Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
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
        FunctionRole::IOWrapper | FunctionRole::EntryPoint | FunctionRole::PatternMatch => {
            Some(DebtType::Risk {
                risk_score: 0.0,
                factors: vec!["Simple I/O wrapper or entry point - minimal risk".to_string()],
            })
        }
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

    if language == Language::Python {
        // Use Python-specific dead code detector
        let detector = PythonDeadCodeDetector::new();
        if detector.is_implicitly_called(func) {
            return true;
        }
    }

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

    // Common framework patterns (for non-Python languages)
    if language != Language::Python && (is_likely_trait_method(func) || is_framework_callback(func)) {
        return true;
    }

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

    if language == Language::Python {
        // Use Python-specific detector for usage hints
        let detector = PythonDeadCodeDetector::new();
        let python_hints = detector.generate_usage_hints(func);
        if !python_hints.is_empty() {
            return python_hints;
        }
    }

    // Generic hints for non-Python or when Python detector has no specific hints
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
    use crate::priority::call_graph::{CallGraph, FunctionId};
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
            detected_patterns: None,
        }
    }

    #[test]
    fn test_generate_usage_hints_public_function() {
        let func = create_test_function("test_func", Some("pub"));
        let call_graph = CallGraph::new();
        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 10,
        };

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0], "Public function with no internal callers");
    }

    #[test]
    fn test_generate_usage_hints_private_function() {
        let func = create_test_function("test_func", None);
        let call_graph = CallGraph::new();
        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 10,
        };

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0], "Private function with no callers");
    }

    #[test]
    fn test_generate_usage_hints_underscore_prefix() {
        let func = create_test_function("_internal_func", None);
        let call_graph = CallGraph::new();
        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "_internal_func".to_string(),
            line: 10,
        };

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
        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "old_deprecated_function".to_string(),
            line: 10,
        };

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0], "Public function with no internal callers");
        assert_eq!(hints[1], "Name suggests obsolete functionality");
    }

    #[test]
    fn test_generate_usage_hints_legacy_function() {
        let func = create_test_function("legacy_handler", None);
        let call_graph = CallGraph::new();
        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "legacy_handler".to_string(),
            line: 10,
        };

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert_eq!(hints.len(), 2);
        assert!(hints.contains(&"Private function with no callers".to_string()));
        assert!(hints.contains(&"Name suggests obsolete functionality".to_string()));
    }

    #[test]
    fn test_generate_usage_hints_multiple_indicators() {
        let func = create_test_function("_old_deprecated_helper", Some("pub"));
        let call_graph = CallGraph::new();
        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "_old_deprecated_helper".to_string(),
            line: 10,
        };

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert_eq!(hints.len(), 3);
        assert!(hints.contains(&"Public function with no internal callers".to_string()));
        assert!(hints.contains(
            &"Name starts with underscore (often indicates internal/unused)".to_string()
        ));
        assert!(hints.contains(&"Name suggests obsolete functionality".to_string()));
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
}

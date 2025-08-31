// Debt classification functions

use crate::core::FunctionMetrics;
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
    // Determine primary debt type based on metrics
    if let Some(cov) = coverage {
        // Any untested function (< 20% coverage) that isn't a test itself is a testing gap
        // Even simple functions need basic tests
        if cov.direct < 0.2 && !func.is_test {
            return DebtType::TestingGap {
                coverage: cov.direct,
                cyclomatic: func.cyclomatic,
                cognitive: func.cognitive,
            };
        }
    }

    if func.cyclomatic > 10 || func.cognitive > 15 {
        return DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        };
    }

    // Check for dead code before falling back to generic risk
    if is_dead_code(func, call_graph, func_id, None) {
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
        // Check if it's an I/O wrapper or entry point
        if role == FunctionRole::IOWrapper
            || role == FunctionRole::EntryPoint
            || role == FunctionRole::PatternMatch
        {
            // These are acceptable patterns, not debt
            return DebtType::Risk {
                risk_score: 0.0,
                factors: vec!["Simple I/O wrapper or entry point - minimal risk".to_string()],
            };
        }

        // Pure logic functions that are very simple are not debt
        if role == FunctionRole::PureLogic && func.length <= 10 {
            // Simple pure functions like formatters don't need to be flagged
            // Return minimal risk to indicate no real debt
            return DebtType::Risk {
                risk_score: 0.0,
                factors: vec!["Trivial pure function - not technical debt".to_string()],
            };
        }
    }

    // Only flag as risk-based debt if there's actual complexity or other indicators
    if func.cyclomatic > 5 || func.cognitive > 8 || func.length > 50 {
        DebtType::Risk {
            risk_score: calculate_risk_score(func),
            factors: identify_risk_factors(func, coverage),
        }
    } else {
        // Simple functions with cyclomatic <= 5 and cognitive <= 8 and length <= 50
        // Simple functions are not debt in themselves
        if role == FunctionRole::PureLogic {
            // Simple pure functions are not debt - return minimal risk
            DebtType::Risk {
                risk_score: 0.0,
                factors: vec!["Simple pure function - minimal risk".to_string()],
            }
        } else {
            // Other simple functions - minimal risk
            DebtType::Risk {
                risk_score: 0.1,
                factors: vec!["Simple function with low complexity".to_string()],
            }
        }
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
        if has_testing_gap(cov.direct, func.is_test) {
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
    let mut factors = Vec::new();

    if func.cyclomatic > 5 {
        factors.push(format!(
            "Moderate complexity (cyclomatic: {})",
            func.cyclomatic
        ));
    }

    if func.cognitive > 8 {
        factors.push(format!("Cognitive complexity: {}", func.cognitive));
    }

    if func.length > 50 {
        factors.push(format!("Long function ({} lines)", func.length));
    }

    if let Some(cov) = coverage {
        if cov.direct < 0.5 {
            factors.push(format!("Low coverage: {:.0}%", cov.direct * 100.0));
        }
    }

    if factors.is_empty() {
        factors.push("Potential improvement opportunity".to_string());
    }

    factors
}

fn is_excluded_from_dead_code_analysis(func: &FunctionMetrics) -> bool {
    // Entry points
    if func.name == "main" || func.name.starts_with("_start") {
        return true;
    }

    // Test functions
    if func.is_test || func.name.starts_with("test_") || func.name.starts_with("tests::") {
        return true;
    }

    // Build script functions
    if func.file.to_string_lossy().contains("build.rs") && func.name == "main" {
        return true;
    }

    // Common framework patterns
    if is_likely_trait_method(func) || is_framework_callback(func) {
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

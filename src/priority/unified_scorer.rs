use crate::core::FunctionMetrics;
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    coverage_propagation::{
        calculate_coverage_urgency, calculate_transitive_coverage, TransitiveCoverage,
    },
    external_api_detector::{generate_enhanced_dead_code_hints, is_likely_external_api},
    semantic_classifier::{
        calculate_semantic_priority, classify_function_role, get_role_multiplier, FunctionRole,
    },
    ActionableRecommendation, DebtType, FunctionVisibility, ImpactMetrics,
};
use crate::risk::lcov::LcovData;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedScore {
    pub complexity_factor: f64, // 0-10, weighted 25%
    pub coverage_factor: f64,   // 0-10, weighted 35%
    pub roi_factor: f64,        // 0-10, weighted 25%
    pub semantic_factor: f64,   // 0-10, weighted 15%
    pub role_multiplier: f64,   // 0.1-1.5x based on function role
    pub final_score: f64,       // Computed composite score
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedDebtItem {
    pub location: Location,
    pub debt_type: DebtType,
    pub unified_score: UnifiedScore,
    pub function_role: FunctionRole,
    pub recommendation: ActionableRecommendation,
    pub expected_impact: ImpactMetrics,
    pub transitive_coverage: Option<TransitiveCoverage>,
    pub upstream_dependencies: usize,
    pub downstream_dependencies: usize,
    pub nesting_depth: u32,
    pub function_length: usize,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: PathBuf,
    pub function: String,
    pub line: usize,
}

pub fn calculate_unified_priority(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    roi_score: f64,
) -> UnifiedScore {
    let func_id = FunctionId {
        file: func.file.clone(),
        name: func.name.clone(),
        line: func.line,
    };

    // Calculate complexity factor (normalized to 0-10)
    let complexity_factor = normalize_complexity(func.cyclomatic, func.cognitive);

    // Calculate coverage factor (0-10, higher means more urgent to cover)
    let coverage_factor = if func.is_test {
        // Test functions don't need coverage - they are the coverage mechanism
        0.0
    } else if let Some(cov) = coverage {
        calculate_coverage_urgency(&func_id, call_graph, cov, func.cyclomatic)
    } else {
        // No coverage data - assume worst case
        10.0
    };

    // Calculate ROI factor (normalized to 0-10)
    let roi_factor = normalize_roi(roi_score);

    // Classify function role and calculate semantic priority
    let role = classify_function_role(func, &func_id, call_graph);
    let semantic_factor = calculate_semantic_priority(func, role, &func_id, call_graph);
    let role_multiplier = get_role_multiplier(role);

    // Calculate weighted composite score
    let base_score = complexity_factor * 0.25
        + coverage_factor * 0.35
        + roi_factor * 0.25
        + semantic_factor * 0.15;

    // Apply role multiplier
    let final_score = (base_score * role_multiplier).min(10.0);

    UnifiedScore {
        complexity_factor,
        coverage_factor,
        roi_factor,
        semantic_factor,
        role_multiplier,
        final_score,
    }
}

fn normalize_complexity(cyclomatic: u32, cognitive: u32) -> f64 {
    // Normalize complexity to 0-10 scale
    let combined = (cyclomatic + cognitive) as f64 / 2.0;

    // Use logarithmic scale for better distribution
    // Complexity of 1-5 = low (0-3), 6-10 = medium (3-6), 11+ = high (6-10)
    if combined <= 5.0 {
        combined * 0.6
    } else if combined <= 10.0 {
        3.0 + (combined - 5.0) * 0.6
    } else {
        6.0 + ((combined - 10.0) * 0.2).min(4.0)
    }
}

fn normalize_roi(roi: f64) -> f64 {
    // ROI typically ranges from 0.1 to 10.0
    // Normalize to 0-10 scale with logarithmic transformation
    if roi <= 0.0 {
        0.0
    } else if roi <= 1.0 {
        roi * 3.0
    } else if roi <= 5.0 {
        3.0 + (roi - 1.0) * 1.0
    } else {
        7.0 + ((roi - 5.0) * 0.6).min(3.0)
    }
}

pub fn create_unified_debt_item(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    roi_score: f64,
) -> UnifiedDebtItem {
    let func_id = FunctionId {
        file: func.file.clone(),
        name: func.name.clone(),
        line: func.line,
    };

    let unified_score = calculate_unified_priority(func, call_graph, coverage, roi_score);
    let role = classify_function_role(func, &func_id, call_graph);

    let transitive_coverage =
        coverage.map(|cov| calculate_transitive_coverage(&func_id, call_graph, cov));

    let debt_type = determine_debt_type(func, &transitive_coverage, call_graph, &func_id);
    let recommendation = generate_recommendation(func, &debt_type, role, &unified_score);
    let expected_impact = calculate_expected_impact(func, &debt_type, &unified_score);

    // Get dependency counts from call graph
    let upstream_dependencies = call_graph.get_callers(&func_id).len();
    let downstream_dependencies = call_graph.get_callees(&func_id).len();

    UnifiedDebtItem {
        location: Location {
            file: func.file.clone(),
            function: func.name.clone(),
            line: func.line,
        },
        debt_type,
        unified_score,
        function_role: role,
        recommendation,
        expected_impact,
        transitive_coverage,
        upstream_dependencies,
        downstream_dependencies,
        nesting_depth: func.nesting,
        function_length: func.length,
        cyclomatic_complexity: func.cyclomatic,
        cognitive_complexity: func.cognitive,
    }
}

fn determine_debt_type(
    func: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> DebtType {
    // Determine primary debt type based on metrics
    if let Some(cov) = coverage {
        if cov.direct < 0.2 && func.cyclomatic > 3 {
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
    if is_dead_code(func, call_graph, func_id) {
        return DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        };
    }

    // Default to risk-based debt
    DebtType::Risk {
        risk_score: 5.0,
        factors: vec!["General technical debt".to_string()],
    }
}

fn is_dead_code(func: &FunctionMetrics, call_graph: &CallGraph, func_id: &FunctionId) -> bool {
    // Skip obvious false positives
    if is_excluded_from_dead_code_analysis(func) {
        return false;
    }

    // Check if function has incoming calls
    let callers = call_graph.get_callers(func_id);
    callers.is_empty()
}

fn is_excluded_from_dead_code_analysis(func: &FunctionMetrics) -> bool {
    // Entry points
    if func.name == "main" || func.name.starts_with("_start") {
        return true;
    }

    // Test functions
    if func.is_test
        || func.name.starts_with("test_")
        || func.file.to_string_lossy().contains("/tests/")
    {
        return true;
    }

    // Exported functions (likely FFI or API) - check for common patterns
    if func.name.contains("extern") || func.name.starts_with("__") {
        return true;
    }

    // Common framework patterns
    if is_framework_callback(func) {
        return true;
    }

    false
}

fn is_framework_callback(func: &FunctionMetrics) -> bool {
    // Common web framework handlers
    func.name.contains("handler") || 
    func.name.contains("route") ||
    func.name.contains("view") ||
    func.name.contains("controller") ||
    // Common async patterns
    func.name.starts_with("on_") ||
    func.name.starts_with("handle_") ||
    // Common trait implementations
    func.name == "new" ||
    func.name == "default" ||
    func.name == "fmt" ||
    func.name == "drop" ||
    func.name == "clone"
}

fn determine_visibility(func: &FunctionMetrics) -> FunctionVisibility {
    // Use the visibility field from FunctionMetrics if available
    match &func.visibility {
        Some(vis) if vis == "pub" => FunctionVisibility::Public,
        Some(vis) if vis == "pub(crate)" => FunctionVisibility::Crate,
        Some(vis) if vis.starts_with("pub(") => FunctionVisibility::Crate, // pub(super), pub(in ...), etc.
        _ => FunctionVisibility::Private,
    }
}

fn generate_usage_hints(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> Vec<String> {
    let visibility = determine_visibility(func);

    // Use enhanced dead code hints
    let mut hints = generate_enhanced_dead_code_hints(func, &visibility);

    // Add call graph information
    let callees = call_graph.get_callees(func_id);
    if callees.is_empty() {
        hints.push("Function has no dependencies - safe to remove".to_string());
    } else {
        hints.push(format!("Function calls {} other functions", callees.len()));
    }

    hints
}

/// Helper to format complexity metrics for display
fn format_complexity_display(cyclomatic: &u32, cognitive: &u32) -> String {
    format!("cyclo={cyclomatic}, cog={cognitive}")
}

/// Helper to format role description
fn format_role_description(role: FunctionRole) -> &'static str {
    match role {
        FunctionRole::PureLogic => "business logic",
        FunctionRole::Orchestrator => "orchestration",
        FunctionRole::IOWrapper => "I/O wrapper",
        FunctionRole::EntryPoint => "entry point",
        FunctionRole::Unknown => "function",
    }
}

/// Generate steps for dead code based on visibility
fn generate_dead_code_steps(visibility: &FunctionVisibility) -> Vec<String> {
    match visibility {
        FunctionVisibility::Private => vec![
            "Verify no dynamic calls or reflection usage".to_string(),
            "Remove function definition".to_string(),
            "Remove associated tests if any".to_string(),
            "Check if removal enables further cleanup".to_string(),
        ],
        FunctionVisibility::Crate => vec![
            "Check if function is intended as internal API".to_string(),
            "Add documentation if keeping for future use".to_string(),
            "Remove if truly unused".to_string(),
            "Consider making private if only locally needed".to_string(),
        ],
        FunctionVisibility::Public => vec![
            "Verify no external callers exist".to_string(),
            "Add comprehensive documentation if keeping".to_string(),
            "Mark as deprecated if phasing out".to_string(),
            "Consider adding usage examples or tests".to_string(),
        ],
    }
}

/// Generate action and rationale for dead code
fn generate_dead_code_action(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
    func_name: &str,
    cyclomatic: &u32,
    cognitive: &u32,
) -> (String, String) {
    let complexity_str = format_complexity_display(cyclomatic, cognitive);

    match visibility {
        FunctionVisibility::Private => (
            "Remove unused private function".to_string(),
            format!("Private function '{func_name}' has no callers and can be safely removed (complexity: {complexity_str})"),
        ),
        FunctionVisibility::Crate => (
            "Remove or document unused crate function".to_string(),
            format!("Crate-public function '{func_name}' has no internal callers (complexity: {complexity_str})"),
        ),
        FunctionVisibility::Public => {
            let (is_likely_api, _) = is_likely_external_api(func, visibility);
            if is_likely_api {
                (
                    "Verify external usage before removal or deprecation".to_string(),
                    format!("Public function '{func_name}' appears to be external API - verify usage before action (complexity: {complexity_str})"),
                )
            } else {
                (
                    "Remove unused public function (no API indicators)".to_string(),
                    format!("Public function '{func_name}' has no callers and no external API indicators (complexity: {complexity_str})"),
                )
            }
        }
    }
}

/// Generate steps for testing gap based on complexity
fn generate_testing_gap_steps(is_complex: bool) -> Vec<String> {
    if is_complex {
        vec![
            "Identify and extract pure functions (no side effects)".to_string(),
            "Add property-based tests for pure logic".to_string(),
            "Replace conditionals with pattern matching where possible".to_string(),
            "Convert loops to map/filter/fold operations".to_string(),
            "Push I/O to the boundaries".to_string(),
        ]
    } else {
        vec![
            "Test happy path scenarios".to_string(),
            "Add edge case tests".to_string(),
            "Cover error conditions".to_string(),
        ]
    }
}

/// Generate recommendation for testing gap debt type
fn generate_testing_gap_recommendation(
    coverage: f64,
    cyclomatic: u32,
    cognitive: u32,
    role: FunctionRole,
) -> (String, String, Vec<String>) {
    let is_complex = cyclomatic > 10 || cognitive > 15;
    let coverage_pct = (coverage * 100.0) as i32;
    let role_str = format_role_description(role);

    if is_complex {
        (
            format!("Extract pure functions, add property tests, then refactor (cyclo={cyclomatic} to <10, cog={cognitive} to <15)"),
            format!("Complex {role_str} (cyclo={cyclomatic}, cog={cognitive}) with {coverage_pct}% coverage - extract pure logic first"),
            generate_testing_gap_steps(true),
        )
    } else {
        let role_display = match role {
            FunctionRole::PureLogic => "Business logic",
            FunctionRole::Orchestrator => "Orchestration",
            FunctionRole::IOWrapper => "I/O wrapper",
            FunctionRole::EntryPoint => "Entry point",
            FunctionRole::Unknown => "Function",
        };
        (
            format!("Add {} unit tests for full coverage", cyclomatic.max(2)),
            format!("{role_display} with {coverage_pct}% coverage, manageable complexity (cyclo={cyclomatic}, cog={cognitive})"),
            generate_testing_gap_steps(false),
        )
    }
}

/// Generate recommendation for dead code debt type
fn generate_dead_code_recommendation(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
    usage_hints: &[String],
    cyclomatic: u32,
    cognitive: u32,
) -> (String, String, Vec<String>) {
    let (action, rationale) =
        generate_dead_code_action(func, visibility, &func.name, &cyclomatic, &cognitive);
    let mut steps = generate_dead_code_steps(visibility);

    // Add usage hints to the steps
    for hint in usage_hints {
        steps.push(format!("Note: {hint}"));
    }

    (action, rationale, steps)
}

/// Generate recommendation for test-specific debt types
fn generate_test_debt_recommendation(debt_type: &DebtType) -> (String, String, Vec<String>) {
    match debt_type {
        DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive,
            threshold
        } => (
            format!("Simplify test - complexity {} exceeds test threshold {}", cyclomatic.max(cognitive), threshold),
            format!("Test has high complexity (cyclo={cyclomatic}, cognitive={cognitive}) - consider splitting into smaller tests"),
            vec![
                "Break complex test into multiple smaller tests".to_string(),
                "Extract test setup into helper functions".to_string(),
                "Use parameterized tests for similar test cases".to_string(),
            ],
        ),
        DebtType::TestTodo { priority: _, reason } => (
            "Complete test TODO".to_string(),
            format!("Test contains TODO: {}", reason.as_ref().unwrap_or(&"No reason specified".to_string())),
            vec![
                "Address the TODO comment".to_string(),
                "Implement missing test logic".to_string(),
                "Remove TODO once completed".to_string(),
            ],
        ),
        DebtType::TestDuplication { instances, total_lines, similarity: _ } => (
            format!("Remove test duplication - {instances} similar test blocks"),
            format!("{instances} duplicated test blocks found across {total_lines} lines"),
            vec![
                "Extract common test logic into helper functions".to_string(),
                "Create parameterized tests for similar test cases".to_string(),
                "Use test fixtures for shared setup".to_string(),
            ],
        ),
        _ => unreachable!("Not a test debt type"),
    }
}

/// Generate recommendation for infrastructure debt types (orchestration, duplication, risk)
fn generate_infrastructure_recommendation(debt_type: &DebtType) -> (String, String, Vec<String>) {
    match debt_type {
        DebtType::Orchestration { delegates_to } => (
            "Consider integration test instead of unit tests".to_string(),
            format!(
                "Orchestration function delegating to {} tested functions",
                delegates_to.len()
            ),
            vec![
                "Write integration test covering the flow".to_string(),
                "Verify delegation behavior".to_string(),
            ],
        ),
        DebtType::Duplication {
            instances,
            total_lines,
        } => (
            "Extract common logic into shared module".to_string(),
            format!(
                "Duplicated across {instances} locations ({total_lines} lines total)"
            ),
            vec![
                "Create shared utility module".to_string(),
                "Replace duplicated code with calls to shared module".to_string(),
                "Add comprehensive tests to shared module".to_string(),
            ],
        ),
        DebtType::Risk {
            risk_score,
            factors,
        } => (
            "Address technical debt".to_string(),
            format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
            vec![
                "Review and refactor problematic areas".to_string(),
                "Add missing tests".to_string(),
                "Update documentation".to_string(),
            ],
        ),
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => (
            format!(
                "Extract {} sub-functions to reduce complexity",
                cyclomatic / 5 + 1
            ),
            format!(
                "Highest complexity function (CC:{cyclomatic}, Cog:{cognitive}), affects all dependent calculations"
            ),
            vec![
                "Identify logical groups in the function".to_string(),
                "Extract each group into a named function".to_string(),
                "Add unit tests for extracted functions".to_string(),
            ],
        ),
        _ => unreachable!("Not an infrastructure debt type"),
    }
}

fn generate_recommendation(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    _score: &UnifiedScore,
) -> ActionableRecommendation {
    let (primary_action, rationale, steps) = match debt_type {
        DebtType::DeadCode {
            visibility,
            usage_hints,
            cyclomatic,
            cognitive,
        } => generate_dead_code_recommendation(
            func,
            visibility,
            usage_hints,
            *cyclomatic,
            *cognitive,
        ),
        DebtType::TestingGap {
            coverage,
            cyclomatic,
            cognitive,
        } => generate_testing_gap_recommendation(*coverage, *cyclomatic, *cognitive, role),
        DebtType::ComplexityHotspot { .. }
        | DebtType::Orchestration { .. }
        | DebtType::Duplication { .. }
        | DebtType::Risk { .. } => generate_infrastructure_recommendation(debt_type),
        DebtType::TestComplexityHotspot { .. }
        | DebtType::TestTodo { .. }
        | DebtType::TestDuplication { .. } => generate_test_debt_recommendation(debt_type),
    };

    ActionableRecommendation {
        primary_action,
        rationale,
        implementation_steps: steps,
        related_items: vec![],
    }
}

fn calculate_expected_impact(
    _func: &FunctionMetrics,
    debt_type: &DebtType,
    score: &UnifiedScore,
) -> ImpactMetrics {
    match debt_type {
        DebtType::DeadCode {
            cyclomatic,
            cognitive,
            ..
        } => ImpactMetrics {
            coverage_improvement: 0.0, // Dead code doesn't affect coverage
            lines_reduction: *cyclomatic + *cognitive, // Estimate based on complexity
            complexity_reduction: (*cyclomatic + *cognitive) as f64 * 0.5, // Removing reduces overall complexity
            risk_reduction: score.final_score * 0.3, // Moderate risk reduction from cleanup
        },
        DebtType::TestingGap {
            coverage,
            cyclomatic,
            cognitive,
        } => {
            // For high complexity functions, the impact includes both testing and refactoring benefits
            // Consider both cyclomatic and cognitive complexity
            // A function is complex if either metric exceeds its threshold
            let is_complex = *cyclomatic > 10 || *cognitive > 15;

            ImpactMetrics {
                // Show the actual coverage gain for this function/module
                // High complexity functions get less coverage benefit (need refactoring first)
                coverage_improvement: if is_complex {
                    (1.0 - coverage) * 50.0 // 50% of potential due to complexity
                } else {
                    (1.0 - coverage) * 100.0 // Full coverage potential for simple functions
                },
                lines_reduction: 0,
                complexity_reduction: if is_complex {
                    *cyclomatic as f64 * 0.3
                } else {
                    0.0
                },
                risk_reduction: score.final_score * 0.42,
            }
        }
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive: _,
        } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: (*cyclomatic as f64 * 0.5),
            risk_reduction: score.final_score * 0.35,
        },
        DebtType::Duplication {
            instances,
            total_lines,
        } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: *total_lines - (*total_lines / instances),
            complexity_reduction: 0.0,
            risk_reduction: score.final_score * 0.25,
        },
        DebtType::Orchestration { .. } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: score.final_score * 0.1, // Low priority for orchestration
        },
        DebtType::Risk { .. } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: score.final_score * 0.2,
        },
        // Test-specific debt types have lower impact on overall metrics
        DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive: _,
            threshold: _,
        } => ImpactMetrics {
            coverage_improvement: 0.0, // Tests don't improve coverage
            lines_reduction: 0,
            complexity_reduction: (*cyclomatic as f64 * 0.3),
            risk_reduction: score.final_score * 0.15, // Lower risk impact for tests
        },
        DebtType::TestTodo { .. } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: score.final_score * 0.1,
        },
        DebtType::TestDuplication {
            instances,
            total_lines,
            similarity: _,
        } => ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: *total_lines - (*total_lines / instances),
            complexity_reduction: 0.0,
            risk_reduction: score.final_score * 0.1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metrics() -> FunctionMetrics {
        FunctionMetrics {
            file: PathBuf::from("test.rs"),
            name: "test_function".to_string(),
            line: 10,
            length: 50,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 0,
            is_test: false,
            visibility: None,
        }
    }

    #[test]
    fn test_normalize_complexity() {
        assert!(normalize_complexity(1, 1) < 2.0);
        assert!(normalize_complexity(5, 5) > 2.0);
        assert!(normalize_complexity(5, 5) < 6.0);
        assert!(normalize_complexity(10, 10) > 5.0);
        assert!(normalize_complexity(20, 20) <= 10.0);
    }

    #[test]
    fn test_normalize_roi() {
        assert_eq!(normalize_roi(0.0), 0.0);
        assert!(normalize_roi(0.5) < 3.0);
        assert!(normalize_roi(1.0) <= 3.0);
        assert!(normalize_roi(3.0) > 3.0);
        assert!(normalize_roi(3.0) < 7.0);
        assert!(normalize_roi(10.0) <= 10.0);
    }

    #[test]
    fn test_unified_scoring() {
        let func = create_test_metrics();
        let graph = CallGraph::new();
        let score = calculate_unified_priority(&func, &graph, None, 5.0);

        assert!(score.complexity_factor > 0.0);
        assert!(score.coverage_factor > 0.0);
        assert!(score.roi_factor > 0.0);
        assert!(score.semantic_factor > 0.0);
        assert!(score.final_score > 0.0);
        assert!(score.final_score <= 10.0);
    }

    #[test]
    fn test_debt_type_determination() {
        let func = create_test_metrics();
        let coverage = Some(TransitiveCoverage {
            direct: 0.1,
            transitive: 0.1,
            propagated_from: vec![],
        });

        let call_graph = CallGraph::new();
        let func_id = FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };
        let debt_type = determine_debt_type(&func, &coverage, &call_graph, &func_id);
        match debt_type {
            DebtType::TestingGap { .. } => (),
            _ => panic!("Expected TestingGap debt type"),
        }
    }

    #[test]
    fn test_recommendation_generation() {
        let func = create_test_metrics();
        let debt_type = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 20,
        };
        let score = UnifiedScore {
            complexity_factor: 8.0,
            coverage_factor: 7.0,
            roi_factor: 6.0,
            semantic_factor: 5.0,
            role_multiplier: 1.0,
            final_score: 6.5,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::PureLogic, &score);
        assert!(rec.primary_action.contains("Extract"));
        assert!(rec.rationale.contains("complexity"));
        assert!(!rec.implementation_steps.is_empty());
    }

    #[test]
    fn test_dead_code_detection() {
        let mut func = create_test_metrics();
        func.name = "unused_helper".to_string();

        let mut call_graph = CallGraph::new();
        let func_id = FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };

        // Function exists but has no callers - should be dead code
        call_graph.add_function(func_id.clone(), false, false, func.cyclomatic, func.length);

        let debt_type = determine_debt_type(&func, &None, &call_graph, &func_id);

        match debt_type {
            DebtType::DeadCode {
                visibility: FunctionVisibility::Private,
                ..
            } => (),
            _ => panic!("Expected DeadCode for unused private function, got {debt_type:?}"),
        }
    }

    #[test]
    fn test_main_function_not_dead_code() {
        let mut func = create_test_metrics();
        func.name = "main".to_string();

        let call_graph = CallGraph::new();
        let func_id = FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };

        let debt_type = determine_debt_type(&func, &None, &call_graph, &func_id);

        // Main should not be flagged as dead code
        if let DebtType::DeadCode { .. } = debt_type {
            panic!("Main function should not be flagged as dead code")
        }
    }

    #[test]
    fn test_dead_code_recommendation() {
        let mut func = create_test_metrics();
        func.visibility = Some("pub".to_string()); // Make it public for the test
        let debt_type = DebtType::DeadCode {
            visibility: FunctionVisibility::Public,
            cyclomatic: 5,
            cognitive: 8,
            usage_hints: vec!["No internal callers".to_string()],
        };
        let score = UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 0.0,
            roi_factor: 0.0,
            semantic_factor: 1.0,
            role_multiplier: 1.0,
            final_score: 2.0,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::Unknown, &score);
        // With the new API detection, a public function in test.rs with no special indicators
        // will be marked as "Remove unused public function (no API indicators)"
        assert!(
            rec.primary_action.contains("Remove unused public function")
                || rec.primary_action.contains("Verify external usage")
        );
        assert!(rec.rationale.contains("no callers"));
        assert!(rec
            .implementation_steps
            .iter()
            .any(|s| s.contains("external callers") || s.contains("Verify")));
    }

    #[test]
    fn test_format_role_description_pure_logic() {
        let role = FunctionRole::PureLogic;
        let description = format_role_description(role);
        assert_eq!(description, "business logic");
    }

    #[test]
    fn test_format_role_description_orchestrator() {
        let role = FunctionRole::Orchestrator;
        let description = format_role_description(role);
        assert_eq!(description, "orchestration");
    }

    #[test]
    fn test_format_role_description_io_wrapper() {
        let role = FunctionRole::IOWrapper;
        let description = format_role_description(role);
        assert_eq!(description, "I/O wrapper");
    }

    #[test]
    fn test_format_role_description_entry_point() {
        let role = FunctionRole::EntryPoint;
        let description = format_role_description(role);
        assert_eq!(description, "entry point");
    }

    #[test]
    fn test_format_role_description_unknown() {
        let role = FunctionRole::Unknown;
        let description = format_role_description(role);
        assert_eq!(description, "function");
    }

    #[test]
    fn test_generate_testing_gap_recommendation_complex_high_cyclomatic() {
        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.25,
            15, // high cyclomatic (> 10)
            10, // normal cognitive
            FunctionRole::PureLogic,
        );

        assert!(action.contains("Extract pure functions"));
        assert!(action.contains("cyclo=15"));
        assert!(action.contains("cog=10"));
        assert!(rationale.contains("Complex business logic"));
        assert!(rationale.contains("25% coverage"));
        assert_eq!(steps.len(), 5);
        assert!(steps[0].contains("extract pure functions"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_complex_high_cognitive() {
        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.5,
            8,  // normal cyclomatic
            20, // high cognitive (> 15)
            FunctionRole::Orchestrator,
        );

        assert!(action.contains("Extract pure functions"));
        assert!(action.contains("cyclo=8"));
        assert!(action.contains("cog=20"));
        assert!(rationale.contains("Complex orchestration"));
        assert!(rationale.contains("50% coverage"));
        assert_eq!(steps.len(), 5);
        assert!(steps[1].contains("property-based tests"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_pure_logic() {
        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.0,
            5, // low complexity
            8, // low cognitive
            FunctionRole::PureLogic,
        );

        assert!(action.contains("Add 5 unit tests"));
        assert!(rationale.contains("Business logic"));
        assert!(rationale.contains("0% coverage"));
        assert!(rationale.contains("manageable complexity"));
        assert_eq!(steps.len(), 3);
        assert!(steps[0].contains("happy path"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_orchestrator() {
        let (action, rationale, steps) =
            generate_testing_gap_recommendation(0.75, 3, 5, FunctionRole::Orchestrator);

        assert!(action.contains("Add 3 unit tests"));
        assert!(rationale.contains("Orchestration"));
        assert!(rationale.contains("75% coverage"));
        assert!(rationale.contains("manageable complexity"));
        assert_eq!(steps.len(), 3);
        assert!(steps[1].contains("edge case"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_io_wrapper() {
        let (action, rationale, steps) =
            generate_testing_gap_recommendation(0.33, 2, 3, FunctionRole::IOWrapper);

        assert!(action.contains("Add 2 unit tests"));
        assert!(rationale.contains("I/O wrapper"));
        assert!(rationale.contains("33% coverage"));
        assert!(rationale.contains("manageable complexity"));
        assert_eq!(steps.len(), 3);
        assert!(steps[2].contains("error conditions"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_entry_point() {
        let (action, rationale, steps) =
            generate_testing_gap_recommendation(1.0, 1, 1, FunctionRole::EntryPoint);

        assert!(action.contains("Add 2 unit tests")); // max(1, 2) = 2
        assert!(rationale.contains("Entry point"));
        assert!(rationale.contains("100% coverage"));
        assert!(rationale.contains("manageable complexity"));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_unknown_role() {
        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.0,
            0, // will use max(0, 2) = 2
            0,
            FunctionRole::Unknown,
        );

        assert!(action.contains("Add 2 unit tests"));
        assert!(rationale.contains("Function"));
        assert!(rationale.contains("0% coverage"));
        assert!(rationale.contains("manageable complexity"));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn test_generate_testing_gap_recommendation_both_high_complexity() {
        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.1,
            25, // very high cyclomatic
            30, // very high cognitive
            FunctionRole::PureLogic,
        );

        assert!(action.contains("Extract pure functions"));
        assert!(action.contains("cyclo=25 to <10"));
        assert!(action.contains("cog=30 to <15"));
        assert!(rationale.contains("Complex business logic"));
        assert!(rationale.contains("10% coverage"));
        assert_eq!(steps.len(), 5);
        assert!(steps.iter().any(|s| s.contains("pattern matching")));
        assert!(steps.iter().any(|s| s.contains("map/filter/fold")));
    }
}

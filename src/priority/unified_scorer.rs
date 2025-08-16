use crate::config;
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
    ActionableRecommendation, DebtType, FunctionAnalysis, FunctionVisibility, ImpactMetrics,
};
use crate::risk::evidence_calculator::EvidenceBasedRiskCalculator;
use crate::risk::lcov::LcovData;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedScore {
    pub complexity_factor: f64, // 0-10, configurable weight (default 15%)
    pub coverage_factor: f64,   // 0-10, configurable weight (default 40%)
    pub roi_factor: f64,        // 0-10, configurable weight (default 25%)
    pub semantic_factor: f64,   // 0-10, configurable weight (default 5%)
    pub dependency_factor: f64, // 0-10, configurable weight (default 15%)
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
    pub upstream_callers: Vec<String>, // List of function names that call this function
    pub downstream_callees: Vec<String>, // List of functions that this function calls
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

    // Check if this function is actually technical debt
    // Simple I/O wrappers, entry points, and trivial pure functions with low complexity
    // are not technical debt UNLESS they're untested and non-trivial
    let role = classify_function_role(func, &func_id, call_graph);
    let is_trivial = (func.cyclomatic <= 3 && func.cognitive <= 5)
        && (role == FunctionRole::IOWrapper
            || role == FunctionRole::EntryPoint
            || (role == FunctionRole::PureLogic && func.length <= 10));

    // Check actual test coverage if we have lcov data
    let has_coverage = if let Some(cov) = coverage {
        cov.get_function_coverage(&func.file, &func.name)
            .map(|coverage_pct| coverage_pct > 0.0)
            .unwrap_or(false)
    } else {
        false // No coverage data means assume untested
    };

    // If it's trivial AND tested, it's definitely not technical debt
    if is_trivial && has_coverage {
        return UnifiedScore {
            complexity_factor: 0.0,
            coverage_factor: 0.0,
            roi_factor: 0.0,
            semantic_factor: 0.0,
            dependency_factor: 0.0,
            role_multiplier: 1.0,
            final_score: 0.0,
        };
    }

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

    // Calculate semantic priority
    let semantic_factor = calculate_semantic_priority(func, role, &func_id, call_graph);
    let role_multiplier = get_role_multiplier(role);

    // Calculate dependency factor based on upstream dependencies (functions that call this one)
    let upstream_count = call_graph.get_callers(&func_id).len();
    let dependency_factor = calculate_dependency_factor(upstream_count);

    // Get configurable weights
    let weights = config::get_scoring_weights();

    // Calculate weighted components for transparency
    let weighted_complexity = complexity_factor * weights.complexity;
    let weighted_coverage = coverage_factor * weights.coverage;
    let weighted_roi = roi_factor * weights.roi;
    let weighted_semantic = semantic_factor * weights.semantic;
    let weighted_dependency = dependency_factor * weights.dependency;

    // Calculate weighted composite score
    let base_score = weighted_complexity
        + weighted_coverage
        + weighted_roi
        + weighted_semantic
        + weighted_dependency;

    // Apply role multiplier
    let final_score = (base_score * role_multiplier).min(10.0);

    UnifiedScore {
        complexity_factor,
        coverage_factor,
        roi_factor,
        semantic_factor,
        dependency_factor,
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

fn calculate_dependency_factor(upstream_count: usize) -> f64 {
    // Calculate criticality based on number of upstream dependencies (callers)
    // Functions with many callers are on critical paths and should be prioritized
    // 0 callers = 0 (dead code)
    // 1-2 callers = 2-4 (low criticality)
    // 3-5 callers = 4-7 (medium criticality)
    // 6-10 callers = 7-9 (high criticality)
    // 10+ callers = 9-10 (critical path)

    match upstream_count {
        0 => 0.0, // Dead code - low priority unless it's complex
        1 => 2.0,
        2 => 3.0,
        3 => 4.0,
        4 => 5.0,
        5 => 6.0,
        6..=7 => 7.0,
        8..=9 => 8.0,
        10..=14 => 9.0,
        _ => 10.0, // 15+ callers = critical path function
    }
}

/// Create evidence-based risk assessment for a function
pub fn create_evidence_based_risk_assessment(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
) -> crate::risk::evidence::RiskAssessment {
    let calculator = EvidenceBasedRiskCalculator::new();

    // Convert FunctionMetrics to FunctionAnalysis
    let function_analysis = FunctionAnalysis {
        file: func.file.clone(),
        function: func.name.clone(),
        line: func.line,
        function_length: func.length,
        cyclomatic_complexity: func.cyclomatic,
        cognitive_complexity: func.cognitive,
        nesting_depth: func.nesting,
        is_test: func.is_test,
        visibility: determine_visibility(func),
    };

    calculator.calculate_risk(&function_analysis, call_graph, coverage)
}

/// Create a unified debt item with enhanced call graph analysis
pub fn create_unified_debt_item_enhanced(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    _enhanced_call_graph: Option<()>, // Placeholder for future enhanced call graph
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

    // Use enhanced debt type classification
    let debt_type = classify_debt_type_enhanced(func, call_graph, &func_id);

    let recommendation = generate_recommendation(func, &debt_type, role, &unified_score);
    let expected_impact = calculate_expected_impact(func, &debt_type, &unified_score);

    // Get caller and callee names
    let upstream_callers = call_graph.get_callers(&func_id);
    let downstream_callees = call_graph.get_callees(&func_id);

    let upstream_caller_names: Vec<String> =
        upstream_callers.iter().map(|id| id.name.clone()).collect();
    let downstream_callee_names: Vec<String> = downstream_callees
        .iter()
        .map(|id| id.name.clone())
        .collect();

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
        upstream_dependencies: upstream_callers.len(),
        downstream_dependencies: downstream_callees.len(),
        upstream_callers: upstream_caller_names,
        downstream_callees: downstream_callee_names,
        nesting_depth: 0,   // Would need to be calculated from AST
        function_length: 0, // Would need to be calculated from AST or additional metadata
        cyclomatic_complexity: func.cyclomatic,
        cognitive_complexity: func.cognitive,
    }
}

pub fn create_unified_debt_item_with_exclusions(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    roi_score: f64,
    framework_exclusions: &HashSet<FunctionId>,
) -> UnifiedDebtItem {
    let func_id = FunctionId {
        file: func.file.clone(),
        name: func.name.clone(),
        line: func.line,
    };

    // Calculate transitive coverage if direct coverage is available
    let transitive_coverage = coverage.and_then(|lcov| {
        lcov.get_function_coverage_with_line(&func.file, &func.name, func.line)
            .map(|_direct| calculate_transitive_coverage(&func_id, call_graph, lcov))
    });

    // Use the enhanced debt type classification with framework exclusions
    let debt_type =
        classify_debt_type_with_exclusions(func, call_graph, &func_id, framework_exclusions);

    // Calculate unified score
    let unified_score = calculate_unified_priority(func, call_graph, coverage, roi_score);

    // Determine function role for more accurate analysis
    let function_role = classify_function_role(func, &func_id, call_graph);

    // Generate contextual recommendation based on debt type and metrics
    let recommendation = generate_recommendation(func, &debt_type, function_role, &unified_score);

    // Calculate expected impact
    let expected_impact = calculate_expected_impact(func, &debt_type, &unified_score);

    // Get dependency information
    let upstream = call_graph.get_callers(&func_id);
    let downstream = call_graph.get_callees(&func_id);

    UnifiedDebtItem {
        location: Location {
            file: func.file.clone(),
            function: func.name.clone(),
            line: func.line,
        },
        debt_type,
        unified_score,
        function_role,
        recommendation,
        expected_impact,
        transitive_coverage,
        upstream_dependencies: upstream.len(),
        downstream_dependencies: downstream.len(),
        upstream_callers: upstream.iter().map(|f| f.name.clone()).collect(),
        downstream_callees: downstream.iter().map(|f| f.name.clone()).collect(),
        nesting_depth: 0, // FunctionMetrics doesn't have nesting_depth field
        function_length: func.length,
        cyclomatic_complexity: func.cyclomatic,
        cognitive_complexity: func.cognitive,
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

    // Get dependency counts and names from call graph
    let upstream_callers = call_graph.get_callers(&func_id);
    let downstream_callees = call_graph.get_callees(&func_id);

    let upstream_caller_names: Vec<String> =
        upstream_callers.iter().map(|id| id.name.clone()).collect();
    let downstream_callee_names: Vec<String> = downstream_callees
        .iter()
        .map(|id| id.name.clone())
        .collect();

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
        upstream_dependencies: upstream_callers.len(),
        downstream_dependencies: downstream_callees.len(),
        upstream_callers: upstream_caller_names,
        downstream_callees: downstream_callee_names,
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
    if is_dead_code(func, call_graph, func_id) {
        return DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        };
    }

    // Check if this is an orchestrator that doesn't need tests
    let role = classify_function_role(func, func_id, call_graph);
    if role == FunctionRole::Orchestrator {
        let callees = call_graph.get_callees(func_id);
        // Filter out standard library functions
        let meaningful_callees: Vec<_> = callees
            .iter()
            .filter(|f| !is_std_or_utility_function(&f.name))
            .collect();
        // Only flag as orchestration if there are actual functions being orchestrated
        if meaningful_callees.len() >= 2 {
            return DebtType::Orchestration {
                delegates_to: meaningful_callees.iter().map(|f| f.name.clone()).collect(),
            };
        }
    }

    // Low complexity functions that are I/O wrappers or entry points
    // should not be flagged as technical debt
    if func.cyclomatic <= 3 && func.cognitive <= 5 {
        // Check if it's an I/O wrapper or entry point
        if role == FunctionRole::IOWrapper || role == FunctionRole::EntryPoint {
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
        // Check if they're calling other functions (true delegation)
        let callees = call_graph.get_callees(func_id);
        // Filter out standard library functions
        let meaningful_callees: Vec<_> = callees
            .iter()
            .filter(|f| !is_std_or_utility_function(&f.name))
            .collect();
        if meaningful_callees.len() >= 2
            && func.cyclomatic <= 2
            && role == FunctionRole::Orchestrator
        {
            // This is a simple delegation function that was identified as an orchestrator
            DebtType::Orchestration {
                delegates_to: meaningful_callees.iter().map(|f| f.name.clone()).collect(),
            }
        } else if role == FunctionRole::PureLogic {
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

fn is_dead_code(func: &FunctionMetrics, call_graph: &CallGraph, func_id: &FunctionId) -> bool {
    // Check hardcoded exclusions (includes test functions, main, etc.)
    if is_excluded_from_dead_code_analysis(func) {
        return false;
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
    framework_exclusions: &std::collections::HashSet<FunctionId>,
) -> bool {
    // First check if this function is excluded by framework patterns
    if framework_exclusions.contains(func_id) {
        return false;
    }

    // Then check hardcoded exclusions for backward compatibility
    if is_excluded_from_dead_code_analysis(func) {
        return false;
    }

    // Check if function has incoming calls
    let callers = call_graph.get_callers(func_id);
    callers.is_empty()
}

/// Enhanced dead code detection using the enhanced call graph
/// Enhanced version of debt type classification with framework pattern exclusions
pub fn classify_debt_type_with_exclusions(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    framework_exclusions: &HashSet<FunctionId>,
) -> DebtType {
    // Test functions are special debt cases
    if func.is_test {
        return match () {
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
        };
    }

    // Check for complexity hotspots first
    if func.cyclomatic > 10 || func.cognitive > 15 {
        return DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        };
    }

    // Check for dead code with framework exclusions
    if is_dead_code_with_exclusions(func, call_graph, func_id, framework_exclusions) {
        return DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        };
    }

    // Check if this is an orchestrator that doesn't need tests
    let role = classify_function_role(func, func_id, call_graph);
    if role == FunctionRole::Orchestrator {
        let callees = call_graph.get_callees(func_id);
        // Filter out standard library functions
        let meaningful_callees: Vec<_> = callees
            .iter()
            .filter(|f| !is_std_or_utility_function(&f.name))
            .collect();
        // Only flag as orchestration if there are actual functions being orchestrated
        if meaningful_callees.len() >= 2 {
            return DebtType::Orchestration {
                delegates_to: meaningful_callees.iter().map(|f| f.name.clone()).collect(),
            };
        }
    }

    // Low complexity functions that are I/O wrappers or entry points
    // should not be flagged as technical debt
    if func.cyclomatic <= 3 && func.cognitive <= 5 {
        // Check if it's an I/O wrapper or entry point
        if role == FunctionRole::IOWrapper || role == FunctionRole::EntryPoint {
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
            factors: identify_risk_factors(func, &None),
        }
    } else {
        // Simple functions with cyclomatic <= 5 and cognitive <= 8 and length <= 50
        // Check if they're calling other functions (true delegation)
        let callees = call_graph.get_callees(func_id);
        // Filter out standard library functions
        let meaningful_callees: Vec<_> = callees
            .iter()
            .filter(|f| !is_std_or_utility_function(&f.name))
            .collect();
        if meaningful_callees.len() >= 2
            && func.cyclomatic <= 2
            && role == FunctionRole::Orchestrator
        {
            DebtType::Orchestration {
                delegates_to: meaningful_callees.iter().map(|f| f.name.clone()).collect(),
            }
        } else {
            // Not debt - well-designed simple function
            DebtType::Risk {
                risk_score: 0.0,
                factors: vec!["Well-designed simple function - not technical debt".to_string()],
            }
        }
    }
}

/// Enhanced version of debt type classification (legacy - kept for compatibility)
pub fn classify_debt_type_enhanced(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> DebtType {
    // Test functions are special debt cases
    if func.is_test {
        return match () {
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
        };
    }

    // Check for complexity hotspots first
    if func.cyclomatic > 10 || func.cognitive > 15 {
        return DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        };
    }

    // Check for dead code
    if is_dead_code(func, call_graph, func_id) {
        return DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        };
    }

    // Check if this is an orchestrator that doesn't need tests
    let role = classify_function_role(func, func_id, call_graph);
    if role == FunctionRole::Orchestrator {
        let callees = call_graph.get_callees(func_id);
        // Filter out standard library functions
        let meaningful_callees: Vec<_> = callees
            .iter()
            .filter(|f| !is_std_or_utility_function(&f.name))
            .collect();
        // Only flag as orchestration if there are actual functions being orchestrated
        if meaningful_callees.len() >= 2 {
            return DebtType::Orchestration {
                delegates_to: meaningful_callees.iter().map(|f| f.name.clone()).collect(),
            };
        }
    }

    // Low complexity functions that are I/O wrappers or entry points
    // should not be flagged as technical debt
    if func.cyclomatic <= 3 && func.cognitive <= 5 {
        // Check if it's an I/O wrapper or entry point
        if role == FunctionRole::IOWrapper || role == FunctionRole::EntryPoint {
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
            factors: identify_risk_factors(func, &None),
        }
    } else {
        // Simple functions with cyclomatic <= 5 and cognitive <= 8 and length <= 50
        // Check if they're calling other functions (true delegation)
        let callees = call_graph.get_callees(func_id);
        // Filter out standard library functions
        let meaningful_callees: Vec<_> = callees
            .iter()
            .filter(|f| !is_std_or_utility_function(&f.name))
            .collect();
        if meaningful_callees.len() >= 2
            && func.cyclomatic <= 2
            && role == FunctionRole::Orchestrator
        {
            // This is a simple delegation function that was identified as an orchestrator
            DebtType::Orchestration {
                delegates_to: meaningful_callees.iter().map(|f| f.name.clone()).collect(),
            }
        } else if role == FunctionRole::PureLogic {
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

    // Closures are part of their parent function - not independent dead code
    if func.name.contains("<closure@") {
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

    // Trait method implementations - these are called through trait objects
    // Use the new is_trait_method field for accurate detection
    if func.is_trait_method {
        return true;
    }

    // Also check for common trait method patterns as fallback
    if is_likely_trait_method(func) {
        return true;
    }

    false
}

fn is_likely_trait_method(func: &FunctionMetrics) -> bool {
    // Check if this is likely a trait method implementation based on:
    // 1. Public visibility + specific method names that are commonly trait methods
    // 2. Methods that are part of common trait implementations

    if func.visibility == Some("pub".to_string()) {
        // Common trait methods that should not be flagged as dead code
        let method_name = if let Some(pos) = func.name.rfind("::") {
            &func.name[pos + 2..]
        } else {
            &func.name
        };

        matches!(
            method_name,
            // Common trait methods from std library traits
            "write_results" | "write_risk_insights" |  // OutputWriter trait
            "fmt" | "clone" | "default" | "from" | "into" |
            "as_ref" | "as_mut" | "deref" | "deref_mut" |
            "drop" | "eq" | "ne" | "cmp" | "partial_cmp" |
            "hash" | "serialize" | "deserialize" |
            "try_from" | "try_into" | "to_string" |
            // Iterator trait methods
            "next" | "size_hint" | "count" | "last" | "nth" |
            // Async trait methods
            "poll" | "poll_next" | "poll_ready" | "poll_flush" |
            // Common custom trait methods
            "execute" | "run" | "process" | "handle" |
            "render" | "draw" | "update" | "tick" |
            "validate" | "is_valid" | "check" |
            "encode" | "decode" | "parse" | "format"
        )
    } else {
        false
    }
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

// Helper to identify standard library and utility functions that shouldn't count as delegation targets
fn is_std_or_utility_function(name: &str) -> bool {
    matches!(
        name,
        // Standard library functions from macro expansion
        "format" | "write" | "print" | "println" |
        // Common utility functions that are too generic
        "clone" | "to_string" | "into" | "from"
    ) || name.starts_with("std::")
        || name.starts_with("core::")
        || name.starts_with("alloc::")
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

/// Calculate number of functions to extract based on complexity
fn calculate_functions_to_extract(cyclomatic: u32, cognitive: u32) -> u32 {
    let max_complexity = cyclomatic.max(cognitive);
    match max_complexity {
        0..=10 => 1,
        11..=15 => 2,
        16..=20 => 3,
        21..=30 => 4,
        _ => 5,
    }
}

/// Generate combined testing and refactoring steps for complex functions with low coverage
fn generate_combined_testing_refactoring_steps(
    cyclomatic: u32,
    cognitive: u32,
    coverage_pct: i32,
) -> Vec<String> {
    vec![
        format!(
            "Write {} tests for critical uncovered paths (current coverage: {}%)",
            cyclomatic.max(3),
            coverage_pct
        ),
        "Identify and test edge cases and error conditions".to_string(),
        format!(
            "Extract {} pure functions to reduce complexity from cyclo={} to <10",
            calculate_functions_to_extract(cyclomatic, cognitive),
            cyclomatic
        ),
        "Add property-based tests for extracted pure functions".to_string(),
        format!("Achieve 80%+ coverage through comprehensive testing"),
        format!(
            "Goal: Reduce cyclomatic from {} to <10, cognitive from {} to <15",
            cyclomatic, cognitive
        ),
    ]
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
            format!("Extract {} pure functions to reduce complexity, then add {} tests for comprehensive coverage", 
                calculate_functions_to_extract(cyclomatic, cognitive),
                (calculate_functions_to_extract(cyclomatic, cognitive) * 3).max(cyclomatic)),
            format!("Complex {role_str} (cyclo={cyclomatic}, cog={cognitive}) with only {coverage_pct}% coverage - high testing priority"),
            generate_combined_testing_refactoring_steps(cyclomatic, cognitive, coverage_pct),
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
            format!("Add {} unit tests to achieve 80%+ coverage", cyclomatic.max(3)),
            format!("{role_display} with {coverage_pct}% coverage needs testing (cyclo={cyclomatic}, cog={cognitive})"),
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
            "Refactor to pure functions or extract testable units".to_string(),
            format!(
                "Orchestration function delegating to {} functions",
                delegates_to.len()
            ),
            vec![
                "Extract logic into pure functions".to_string(),
                "Compose smaller, testable functions".to_string(),
                "Add unit tests for extracted functions".to_string(),
            ],
        ),
        DebtType::Duplication {
            instances,
            total_lines,
        } => (
            "Extract common logic into shared module".to_string(),
            format!("Duplicated across {instances} locations ({total_lines} lines total)"),
            vec![
                "Create shared utility module".to_string(),
                "Replace duplicated code with calls to shared module".to_string(),
                "Add comprehensive tests to shared module".to_string(),
            ],
        ),
        DebtType::Risk {
            risk_score,
            factors,
        } => {
            // Check if any factor mentions complexity to provide more specific recommendations
            let has_complexity_issue = factors.iter().any(|f| {
                f.contains("complexity") || f.contains("cyclomatic") || f.contains("cognitive")
            });

            if has_complexity_issue {
                // Extract complexity values from factors string if present
                let cyclo_match = factors
                    .iter()
                    .find(|f| f.contains("cyclomatic"))
                    .and_then(|f| {
                        f.split(':')
                            .nth(1)?
                            .trim()
                            .strip_suffix(')')?
                            .parse::<u32>()
                            .ok()
                    });

                let _cognitive_match = factors
                    .iter()
                    .find(|f| f.contains("Cognitive complexity"))
                    .and_then(|f| f.split(':').nth(1)?.trim().parse::<u32>().ok());

                let cyclo = cyclo_match.unwrap_or(0);
                let _cognitive = _cognitive_match.unwrap_or(0);
                let is_moderate = cyclo > 5 && cyclo <= 10;

                if is_moderate {
                    // Check if coverage is also mentioned
                    let has_coverage_issue = factors.iter().any(|f| {
                        f.contains("coverage") || f.contains("Coverage") || f.contains("uncovered")
                    });

                    // Calculate how many functions to extract based on complexity
                    let functions_to_extract = (cyclo / 3).max(2);
                    let target_complexity = 3;

                    // ALWAYS include tests for moderate complexity functions, as coverage is likely an issue
                    // even if not explicitly mentioned in factors (due to how classify_debt_type works)
                    let action = format!("Extract {} pure functions (complexity {} → {}), then add {} comprehensive tests", 
                            functions_to_extract, cyclo, target_complexity, functions_to_extract * 3);

                    (
                        action,
                        format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
                        vec![
                            format!(
                                "Identify {} logical sections in the function",
                                functions_to_extract
                            ),
                            "Extract each section as a pure function (no side effects)".to_string(),
                            "Replace nested if/else with pattern matching or early returns"
                                .to_string(),
                            "Convert imperative loops to .map(), .filter(), .fold()".to_string(),
                            "Move all I/O operations to a single orchestrator function".to_string(),
                            format!(
                                "Write {} unit tests for the extracted pure functions",
                                functions_to_extract * 3
                            ),
                            if has_coverage_issue {
                                "Achieve 80%+ test coverage for all functions".to_string()
                            } else {
                                format!(
                                    "Goal: Reduce cyclomatic complexity from {} to <={}",
                                    cyclo, target_complexity
                                )
                            },
                        ],
                    )
                } else if cyclo > 10 {
                    // High complexity - needs more aggressive refactoring
                    let has_coverage_issue = factors.iter().any(|f| {
                        f.contains("coverage") || f.contains("Coverage") || f.contains("uncovered")
                    });

                    let functions_to_extract = (cyclo / 4).max(3);
                    let target_complexity = 5;

                    // ALWAYS include tests for high complexity functions, as coverage is critical
                    let action = format!("Decompose into {} pure functions (complexity {} → {}), then add {} comprehensive tests", 
                            functions_to_extract, cyclo, target_complexity, functions_to_extract * 4);

                    (
                        action,
                        format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
                        vec![
                            "Map each conditional branch to its core responsibility".to_string(),
                            format!(
                                "Create {} pure functions, one per responsibility",
                                functions_to_extract
                            ),
                            "Replace complex conditionals with function dispatch table".to_string(),
                            "Extract validation logic into composable predicates".to_string(),
                            "Transform data mutations into immutable transformations".to_string(),
                            "Isolate side effects in a thin orchestration layer".to_string(),
                            format!(
                                "Write {} unit tests plus property-based tests for pure functions",
                                functions_to_extract * 4
                            ),
                            if has_coverage_issue {
                                format!(
                                    "Target: Each function ≤{} complexity with 80%+ coverage",
                                    target_complexity
                                )
                            } else {
                                format!(
                                    "Target: Each function ≤{} cyclomatic complexity",
                                    target_complexity
                                )
                            },
                        ],
                    )
                } else {
                    // Low complexity but still flagged - likely other issues including coverage
                    let has_coverage_issue = factors.iter().any(|f| {
                        f.contains("coverage") || f.contains("Coverage") || f.contains("uncovered")
                    });

                    let action = if has_coverage_issue || cyclo > 3 {
                        format!(
                            "Extract helper functions for clarity, then add {} unit tests",
                            cyclo.max(3)
                        )
                    } else {
                        "Simplify function structure and improve testability".to_string()
                    };

                    (
                        action,
                        format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
                        vec![
                            "Extract helper functions for clarity".to_string(),
                            "Remove unnecessary branching".to_string(),
                            "Consolidate similar code paths".to_string(),
                            format!(
                                "Add {} unit tests for edge cases and main paths",
                                cyclo.max(3)
                            ),
                        ],
                    )
                }
            } else {
                // Non-complexity related risk
                (
                    "Address identified risk factors".to_string(),
                    format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
                    vec![
                        "Review and refactor problematic areas".to_string(),
                        "Add missing tests if coverage is low".to_string(),
                        "Update documentation".to_string(),
                    ],
                )
            }
        }
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => {
            // Note: We should check coverage here too, but ComplexityHotspot doesn't store it
            // For now, emphasize both testing and refactoring
            let functions_to_extract = calculate_functions_to_extract(*cyclomatic, *cognitive);
            (
                format!(
                    "Extract {} pure functions to reduce complexity from {} to <10, then add comprehensive tests",
                    functions_to_extract,
                    cyclomatic
                ),
                format!(
                    "High complexity function (cyclo={cyclomatic}, cog={cognitive}) likely with low coverage - needs testing and refactoring"
                ),
                vec![
                    format!("Identify {} logical sections in function", functions_to_extract),
                    "Extract pure functions for each section (no side effects)".to_string(),
                    "Move I/O and side effects to orchestrator function".to_string(),
                    format!("Write {} unit tests for extracted pure functions", functions_to_extract * 3),
                    "Add property-based tests for complex logic".to_string(),
                    format!("Goal: 80%+ coverage with each function <10 cyclomatic complexity"),
                ],
            )
        }
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

/// Determines if a function is considered complex based on its metrics
fn is_function_complex(cyclomatic: u32, cognitive: u32) -> bool {
    cyclomatic > 10 || cognitive > 15
}

/// Calculates the risk reduction factor based on debt type
fn calculate_risk_factor(debt_type: &DebtType) -> f64 {
    match debt_type {
        DebtType::TestingGap { .. } => 0.42,
        DebtType::ComplexityHotspot { .. } => 0.35,
        DebtType::DeadCode { .. } => 0.3,
        DebtType::Duplication { .. } => 0.25,
        DebtType::Risk { .. } => 0.2,
        DebtType::TestComplexityHotspot { .. } => 0.15,
        DebtType::Orchestration { .. }
        | DebtType::TestTodo { .. }
        | DebtType::TestDuplication { .. } => 0.1,
    }
}

/// Calculates coverage improvement potential for testing gaps
fn calculate_coverage_improvement(coverage: f64, is_complex: bool) -> f64 {
    let potential = 1.0 - coverage;
    if is_complex {
        potential * 50.0 // 50% of potential due to complexity
    } else {
        potential * 100.0 // Full coverage potential for simple functions
    }
}

/// Calculates lines that could be reduced through refactoring
fn calculate_lines_reduction(debt_type: &DebtType) -> u32 {
    match debt_type {
        DebtType::DeadCode {
            cyclomatic,
            cognitive,
            ..
        } => *cyclomatic + *cognitive,
        DebtType::Duplication {
            instances,
            total_lines,
        }
        | DebtType::TestDuplication {
            instances,
            total_lines,
            ..
        } => *total_lines - (*total_lines / instances),
        _ => 0,
    }
}

/// Calculates complexity reduction potential based on debt type
fn calculate_complexity_reduction(debt_type: &DebtType, is_complex: bool) -> f64 {
    match debt_type {
        DebtType::DeadCode {
            cyclomatic,
            cognitive,
            ..
        } => (*cyclomatic + *cognitive) as f64 * 0.5,
        DebtType::TestingGap { cyclomatic, .. } if is_complex => *cyclomatic as f64 * 0.3,
        DebtType::ComplexityHotspot { cyclomatic, .. } => *cyclomatic as f64 * 0.5,
        DebtType::TestComplexityHotspot { cyclomatic, .. } => *cyclomatic as f64 * 0.3,
        _ => 0.0,
    }
}

fn calculate_expected_impact(
    _func: &FunctionMetrics,
    debt_type: &DebtType,
    score: &UnifiedScore,
) -> ImpactMetrics {
    let risk_factor = calculate_risk_factor(debt_type);
    let risk_reduction = score.final_score * risk_factor;

    let (coverage_improvement, lines_reduction, complexity_reduction) = match debt_type {
        DebtType::TestingGap {
            coverage,
            cyclomatic,
            cognitive,
        } => {
            let is_complex = is_function_complex(*cyclomatic, *cognitive);
            (
                calculate_coverage_improvement(*coverage, is_complex),
                0,
                calculate_complexity_reduction(debt_type, is_complex),
            )
        }
        _ => (
            0.0,
            calculate_lines_reduction(debt_type),
            calculate_complexity_reduction(debt_type, false),
        ),
    };

    ImpactMetrics {
        coverage_improvement,
        lines_reduction,
        complexity_reduction,
        risk_reduction,
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
            is_trait_method: false,
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
            dependency_factor: 4.0,
            role_multiplier: 1.0,
            final_score: 6.5,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::PureLogic, &score);
        // ComplexityHotspot now extracts first, then tests
        assert!(rec.primary_action.contains("Extract"));
        assert!(rec.primary_action.contains("pure functions"));
        assert!(rec.primary_action.contains("comprehensive tests"));
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
    fn test_simple_io_wrapper_with_coverage_zero_score() {
        // Create a simple I/O wrapper function with test coverage
        let mut func = create_test_metrics();
        func.name = "extract_module_from_import".to_string();
        func.cyclomatic = 1;
        func.cognitive = 1;
        func.length = 3;
        func.nesting = 1;

        let call_graph = CallGraph::new();

        // Create mock coverage data showing function is tested
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            func.file.clone(),
            vec![crate::risk::lcov::FunctionCoverage {
                name: func.name.clone(),
                start_line: func.line,
                execution_count: 18,
                coverage_percentage: 100.0,
            }],
        );

        // Calculate priority score with coverage
        let score = calculate_unified_priority(&func, &call_graph, Some(&lcov), 0.0);

        // Tested simple I/O wrapper should have zero score (not technical debt)
        assert_eq!(score.final_score, 0.0);
        assert_eq!(score.complexity_factor, 0.0);
        assert_eq!(score.coverage_factor, 0.0);
        assert_eq!(score.roi_factor, 0.0);
        assert_eq!(score.semantic_factor, 0.0);
    }

    #[test]
    fn test_simple_io_wrapper_without_coverage_has_score() {
        // Create a simple I/O wrapper function without test coverage
        let mut func = create_test_metrics();
        func.name = "print_risk_function".to_string();
        func.cyclomatic = 1;
        func.cognitive = 0;
        func.length = 4;
        func.nesting = 1;

        let call_graph = CallGraph::new();

        // Calculate priority score without coverage (assume untested)
        let score = calculate_unified_priority(&func, &call_graph, None, 0.0);

        // Untested simple I/O wrapper should have a non-zero score (testing gap)
        assert!(
            score.final_score > 0.0,
            "Untested I/O wrapper should have non-zero score"
        );
    }

    #[test]
    fn test_simple_entry_point_with_coverage_zero_score() {
        // Create a simple entry point function with coverage
        let mut func = create_test_metrics();
        func.name = "main".to_string();
        func.cyclomatic = 2;
        func.cognitive = 3;
        func.length = 8;

        let call_graph = CallGraph::new();

        // Create mock coverage data
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            func.file.clone(),
            vec![crate::risk::lcov::FunctionCoverage {
                name: func.name.clone(),
                start_line: func.line,
                execution_count: 1,
                coverage_percentage: 100.0,
            }],
        );

        // Calculate priority score with coverage
        let score = calculate_unified_priority(&func, &call_graph, Some(&lcov), 0.0);

        // Tested simple entry point should have zero score (not technical debt)
        assert_eq!(score.final_score, 0.0);
    }

    #[test]
    fn test_simple_pure_function_without_coverage_has_score() {
        // Create a simple pure logic function without coverage
        let mut func = create_test_metrics();
        func.name = "format_string".to_string();
        func.cyclomatic = 1;
        func.cognitive = 2;
        func.length = 5;

        let call_graph = CallGraph::new();

        // Calculate priority score without coverage
        let score = calculate_unified_priority(&func, &call_graph, None, 0.0);

        // Untested pure function should have non-zero score (testing gap)
        assert!(
            score.final_score > 0.0,
            "Untested pure function should have non-zero score"
        );
    }

    #[test]
    fn test_complex_function_has_score() {
        // Create a complex function that should have a non-zero score
        let mut func = create_test_metrics();
        func.name = "complex_logic".to_string();
        func.cyclomatic = 8;
        func.cognitive = 12;
        func.length = 50;

        let call_graph = CallGraph::new();

        // Calculate priority score
        let score = calculate_unified_priority(&func, &call_graph, None, 5.0);

        // Complex function should have non-zero score (is technical debt)
        assert!(score.final_score > 0.0);
        assert!(score.complexity_factor > 0.0);
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
            dependency_factor: 0.0,
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

        assert!(action.contains("Extract") && action.contains("pure functions"));
        // New format mentions adding tests and extracting functions
        assert!(action.contains("tests") && action.contains("pure functions"));
        assert!(action.contains("reduce complexity"));
        assert!(
            rationale.contains("Complex business logic")
                || rationale.contains("high testing priority")
        );
        assert!(rationale.contains("25% coverage"));
        assert_eq!(steps.len(), 6);
        assert!(steps[0].contains("tests") || steps[0].contains("Write"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_complex_high_cognitive() {
        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.5,
            8,  // normal cyclomatic
            20, // high cognitive (> 15)
            FunctionRole::Orchestrator,
        );

        assert!(action.contains("Extract") && action.contains("pure functions"));
        // Normal cyclomatic but high cognitive, still complex
        assert!(action.contains("tests") && action.contains("pure functions"));
        assert!(action.contains("reduce complexity"));
        assert!(
            rationale.contains("Complex orchestration")
                || rationale.contains("high testing priority")
        );
        assert!(rationale.contains("50% coverage"));
        assert_eq!(steps.len(), 6);
        assert!(steps[1].contains("edge cases") || steps[1].contains("Identify"));
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
        assert!(rationale.contains("needs testing"));
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
        assert!(rationale.contains("needs testing"));
        assert_eq!(steps.len(), 3);
        assert!(steps[1].contains("edge case"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_io_wrapper() {
        let (action, rationale, steps) =
            generate_testing_gap_recommendation(0.33, 2, 3, FunctionRole::IOWrapper);

        assert!(action.contains("Add") && action.contains("unit tests"));
        assert!(rationale.contains("I/O wrapper"));
        assert!(rationale.contains("33% coverage"));
        assert!(rationale.contains("needs testing"));
        assert_eq!(steps.len(), 3);
        assert!(steps[2].contains("error conditions"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_entry_point() {
        let (action, rationale, steps) =
            generate_testing_gap_recommendation(1.0, 1, 1, FunctionRole::EntryPoint);

        assert!(action.contains("Add") && action.contains("unit tests")); // max(1, 2) = 2
        assert!(rationale.contains("Entry point"));
        assert!(rationale.contains("100% coverage"));
        assert!(rationale.contains("needs testing"));
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

        assert!(action.contains("Add") && action.contains("unit tests"));
        assert!(rationale.contains("Function"));
        assert!(rationale.contains("0% coverage"));
        assert!(rationale.contains("needs testing"));
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

        assert!(action.contains("Extract") && action.contains("pure functions"));
        assert!(action.contains("25") || action.contains("reduce complexity"));
        // Cognitive complexity mention removed in new format
        assert!(
            rationale.contains("Complex business logic")
                || rationale.contains("high testing priority")
        );
        assert!(rationale.contains("10% coverage"));
        assert_eq!(steps.len(), 6);
        // Steps have changed format, checking for test-related content
        assert!(steps
            .iter()
            .any(|s| s.contains("test") || s.contains("Test")));
        assert!(steps
            .iter()
            .any(|s| s.contains("extract") || s.contains("Extract")));
    }

    #[test]
    fn test_generate_test_debt_recommendation_complexity_hotspot() {
        let debt_type = DebtType::TestComplexityHotspot {
            cyclomatic: 15,
            cognitive: 20,
            threshold: 10,
        };

        let (action, rationale, steps) = generate_test_debt_recommendation(&debt_type);

        assert_eq!(
            action,
            "Simplify test - complexity 20 exceeds test threshold 10"
        );
        assert_eq!(rationale, "Test has high complexity (cyclo=15, cognitive=20) - consider splitting into smaller tests");
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], "Break complex test into multiple smaller tests");
        assert_eq!(steps[1], "Extract test setup into helper functions");
        assert_eq!(steps[2], "Use parameterized tests for similar test cases");
    }

    #[test]
    fn test_generate_test_debt_recommendation_todo_with_reason() {
        let debt_type = DebtType::TestTodo {
            priority: crate::core::Priority::Medium,
            reason: Some("Need to test error handling".to_string()),
        };

        let (action, rationale, steps) = generate_test_debt_recommendation(&debt_type);

        assert_eq!(action, "Complete test TODO");
        assert_eq!(rationale, "Test contains TODO: Need to test error handling");
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], "Address the TODO comment");
        assert_eq!(steps[1], "Implement missing test logic");
        assert_eq!(steps[2], "Remove TODO once completed");
    }

    #[test]
    fn test_generate_test_debt_recommendation_todo_without_reason() {
        let debt_type = DebtType::TestTodo {
            priority: crate::core::Priority::Low,
            reason: None,
        };

        let (action, rationale, steps) = generate_test_debt_recommendation(&debt_type);

        assert_eq!(action, "Complete test TODO");
        assert_eq!(rationale, "Test contains TODO: No reason specified");
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], "Address the TODO comment");
        assert_eq!(steps[1], "Implement missing test logic");
        assert_eq!(steps[2], "Remove TODO once completed");
    }

    #[test]
    fn test_generate_test_debt_recommendation_duplication() {
        let debt_type = DebtType::TestDuplication {
            instances: 5,
            total_lines: 150,
            similarity: 0.85,
        };

        let (action, rationale, steps) = generate_test_debt_recommendation(&debt_type);

        assert_eq!(action, "Remove test duplication - 5 similar test blocks");
        assert_eq!(rationale, "5 duplicated test blocks found across 150 lines");
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], "Extract common test logic into helper functions");
        assert_eq!(
            steps[1],
            "Create parameterized tests for similar test cases"
        );
        assert_eq!(steps[2], "Use test fixtures for shared setup");
    }

    #[test]
    fn test_is_function_complex() {
        // Not complex - both metrics below thresholds
        assert!(!is_function_complex(5, 10));
        assert!(!is_function_complex(10, 15));

        // Complex - cyclomatic exceeds threshold
        assert!(is_function_complex(11, 10));
        assert!(is_function_complex(15, 5));

        // Complex - cognitive exceeds threshold
        assert!(is_function_complex(5, 16));
        assert!(is_function_complex(10, 20));

        // Complex - both exceed thresholds
        assert!(is_function_complex(11, 16));
        assert!(is_function_complex(20, 25));
    }

    #[test]
    fn test_calculate_risk_factor() {
        // Test each debt type returns expected risk factor
        let testing_gap = DebtType::TestingGap {
            coverage: 0.5,
            cyclomatic: 5,
            cognitive: 8,
        };
        assert_eq!(calculate_risk_factor(&testing_gap), 0.42);

        let complexity = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 20,
        };
        assert_eq!(calculate_risk_factor(&complexity), 0.35);

        let dead_code = DebtType::DeadCode {
            cyclomatic: 5,
            cognitive: 7,
            visibility: FunctionVisibility::Private,
            usage_hints: vec![],
        };
        assert_eq!(calculate_risk_factor(&dead_code), 0.3);

        let duplication = DebtType::Duplication {
            instances: 3,
            total_lines: 90,
        };
        assert_eq!(calculate_risk_factor(&duplication), 0.25);

        let risk = DebtType::Risk {
            risk_score: 2.0,
            factors: vec!["test".to_string()],
        };
        assert_eq!(calculate_risk_factor(&risk), 0.2);

        let test_complexity = DebtType::TestComplexityHotspot {
            cyclomatic: 12,
            cognitive: 18,
            threshold: 10,
        };
        assert_eq!(calculate_risk_factor(&test_complexity), 0.15);

        let orchestration = DebtType::Orchestration {
            delegates_to: vec!["func1".to_string(), "func2".to_string()],
        };
        assert_eq!(calculate_risk_factor(&orchestration), 0.1);
    }

    #[test]
    fn test_calculate_coverage_improvement() {
        // Simple function with 0% coverage
        assert_eq!(calculate_coverage_improvement(0.0, false), 100.0);

        // Simple function with 60% coverage
        assert_eq!(calculate_coverage_improvement(0.6, false), 40.0);

        // Complex function with 0% coverage (reduced potential)
        assert_eq!(calculate_coverage_improvement(0.0, true), 50.0);

        // Complex function with 60% coverage (reduced potential)
        assert_eq!(calculate_coverage_improvement(0.6, true), 20.0);

        // Full coverage - no improvement possible
        assert_eq!(calculate_coverage_improvement(1.0, false), 0.0);
        assert_eq!(calculate_coverage_improvement(1.0, true), 0.0);
    }

    #[test]
    fn test_calculate_lines_reduction() {
        // Dead code reduction
        let dead_code = DebtType::DeadCode {
            cyclomatic: 5,
            cognitive: 7,
            visibility: FunctionVisibility::Private,
            usage_hints: vec![],
        };
        assert_eq!(calculate_lines_reduction(&dead_code), 12);

        // Duplication reduction
        let duplication = DebtType::Duplication {
            instances: 3,
            total_lines: 90,
        };
        assert_eq!(calculate_lines_reduction(&duplication), 60);

        // Test duplication reduction
        let test_dup = DebtType::TestDuplication {
            instances: 2,
            total_lines: 50,
            similarity: 0.9,
        };
        assert_eq!(calculate_lines_reduction(&test_dup), 25);

        // No reduction for other types
        let testing_gap = DebtType::TestingGap {
            coverage: 0.5,
            cyclomatic: 5,
            cognitive: 8,
        };
        assert_eq!(calculate_lines_reduction(&testing_gap), 0);
    }

    #[test]
    fn test_calculate_complexity_reduction() {
        // Dead code complexity reduction
        let dead_code = DebtType::DeadCode {
            cyclomatic: 10,
            cognitive: 14,
            visibility: FunctionVisibility::Private,
            usage_hints: vec![],
        };
        assert_eq!(calculate_complexity_reduction(&dead_code, false), 12.0);

        // Testing gap - complex function
        let testing_gap = DebtType::TestingGap {
            coverage: 0.3,
            cyclomatic: 15,
            cognitive: 20,
        };
        assert_eq!(calculate_complexity_reduction(&testing_gap, true), 4.5);

        // Testing gap - simple function
        assert_eq!(calculate_complexity_reduction(&testing_gap, false), 0.0);

        // Complexity hotspot
        let complexity = DebtType::ComplexityHotspot {
            cyclomatic: 20,
            cognitive: 25,
        };
        assert_eq!(calculate_complexity_reduction(&complexity, false), 10.0);

        // Test complexity hotspot
        let test_complexity = DebtType::TestComplexityHotspot {
            cyclomatic: 12,
            cognitive: 16,
            threshold: 10,
        };
        assert!((calculate_complexity_reduction(&test_complexity, false) - 3.6).abs() < 0.001);
    }

    #[test]
    fn test_risk_debt_recommendation_with_moderate_complexity() {
        let func = create_test_metrics();
        let debt_type = DebtType::Risk {
            risk_score: 5.0,
            factors: vec!["Moderate complexity (cyclomatic: 9)".to_string()],
        };
        let score = UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 3.0,
            roi_factor: 2.0,
            semantic_factor: 1.0,
            dependency_factor: 2.0,
            role_multiplier: 1.0,
            final_score: 3.0,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::PureLogic, &score);
        assert!(rec.primary_action.contains("Extract 3 pure functions"));
        assert!(rec.primary_action.contains("complexity 9 → 3"));
        assert!(rec.rationale.contains("Risk score 5.0"));
        assert!(rec.rationale.contains("Moderate complexity"));
        assert_eq!(rec.implementation_steps.len(), 7);
        assert!(rec.implementation_steps[0].contains("3 logical sections"));
        assert!(rec.implementation_steps[2].contains("pattern matching"));
        assert!(rec.implementation_steps[3].contains(".map(), .filter(), .fold()"));
        // Step order changed, now expecting test coverage goal
        assert!(
            rec.implementation_steps[6].contains("80%+")
                || rec.implementation_steps[6].contains("Goal")
        );
    }

    #[test]
    fn test_risk_debt_recommendation_with_high_complexity() {
        let func = create_test_metrics();
        let debt_type = DebtType::Risk {
            risk_score: 7.0,
            factors: vec!["High complexity (cyclomatic: 15)".to_string()],
        };
        let score = UnifiedScore {
            complexity_factor: 7.0,
            coverage_factor: 5.0,
            roi_factor: 3.0,
            semantic_factor: 2.0,
            dependency_factor: 3.0,
            role_multiplier: 1.0,
            final_score: 5.0,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::Unknown, &score);
        assert!(rec.primary_action.contains("Decompose into"));
        assert!(rec.primary_action.contains("pure functions"));
        assert!(rec.primary_action.contains("complexity 15"));
        assert!(rec.rationale.contains("Risk score 7.0"));
        assert_eq!(rec.implementation_steps.len(), 8);
        assert!(rec.implementation_steps[0].contains("conditional branch"));
        assert!(rec.implementation_steps[2].contains("function dispatch table"));
    }

    #[test]
    fn test_risk_debt_recommendation_without_complexity() {
        let func = create_test_metrics();
        let debt_type = DebtType::Risk {
            risk_score: 2.0,
            factors: vec!["Low coverage: 30%".to_string()],
        };
        let score = UnifiedScore {
            complexity_factor: 1.0,
            coverage_factor: 6.0,
            roi_factor: 2.0,
            semantic_factor: 1.0,
            dependency_factor: 1.0,
            role_multiplier: 1.0,
            final_score: 3.0,
        };

        let rec = generate_recommendation(&func, &debt_type, FunctionRole::Unknown, &score);
        assert_eq!(rec.primary_action, "Address identified risk factors");
        assert!(rec.rationale.contains("Low coverage"));
        assert_eq!(rec.implementation_steps.len(), 3);
        assert!(rec.implementation_steps[1].contains("missing tests"));
    }

    #[test]
    fn test_calculate_expected_impact_integration() {
        // Test the main function with various debt types
        let func = create_test_metrics();

        // Testing gap with complex function
        let testing_gap = DebtType::TestingGap {
            coverage: 0.2,
            cyclomatic: 12,
            cognitive: 18,
        };
        let score = UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 6.0,
            roi_factor: 4.0,
            semantic_factor: 3.0,
            dependency_factor: 2.0,
            role_multiplier: 1.0,
            final_score: 7.5,
        };

        let impact = calculate_expected_impact(&func, &testing_gap, &score);
        assert_eq!(impact.coverage_improvement, 40.0); // (1-0.2) * 50 for complex
        assert_eq!(impact.lines_reduction, 0);
        assert!((impact.complexity_reduction - 3.6).abs() < 0.001); // 12 * 0.3
        assert_eq!(impact.risk_reduction, 3.15); // 7.5 * 0.42
    }
}

// Functions for creating UnifiedDebtItem instances

use crate::core::FunctionMetrics;
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    scoring::recommendation_extended::{
        generate_assertion_complexity_recommendation, generate_async_misuse_recommendation,
        generate_collection_inefficiency_recommendation,
        generate_complexity_recommendation_with_patterns_and_coverage,
        generate_data_structure_recommendation, generate_feature_envy_recommendation,
        generate_flaky_test_recommendation, generate_god_object_recommendation,
        generate_infrastructure_recommendation_with_coverage, generate_magic_values_recommendation,
        generate_nested_loops_recommendation, generate_primitive_obsession_recommendation,
        generate_resource_leak_recommendation, generate_resource_management_recommendation,
        generate_string_concat_recommendation,
    },
    scoring::rust_recommendations::generate_rust_refactoring_recommendation,
    semantic_classifier::classify_function_role,
    ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, TransitiveCoverage,
    UnifiedDebtItem, UnifiedScore,
};

// Re-export construction functions for backward compatibility
pub use super::construction::{
    create_unified_debt_item_enhanced, create_unified_debt_item_with_aggregator,
    create_unified_debt_item_with_aggregator_and_data_flow,
    create_unified_debt_item_with_exclusions,
    create_unified_debt_item_with_exclusions_and_data_flow,
};

// Re-export computation functions for backward compatibility
pub(super) use super::computation::{calculate_entropy_details, calculate_expected_impact};

// Import computation functions for tests
#[cfg(test)]
use super::computation::{
    calculate_coverage_improvement, calculate_functions_to_extract, calculate_lines_reduction,
    calculate_needed_test_cases, calculate_risk_factor, calculate_simple_test_cases,
    is_function_complex,
};

// Import formatting functions for tests
#[cfg(test)]
use super::formatting::{
    format_complexity_display, format_role_description, get_role_display_name,
};

// Import recommendation helper functions for tests
#[cfg(test)]
use super::recommendation_helpers::generate_full_coverage_recommendation;

// Import types for tests
#[cfg(test)]
use crate::priority::FunctionVisibility;

// Re-export formatting and helper functions for backward compatibility
pub use super::formatting::determine_visibility;
pub(super) use super::formatting::is_rust_file;

// Import functions from recommendation_helpers module
use super::recommendation_helpers::{
    build_actionable_recommendation, extract_coverage_percent, extract_cyclomatic_complexity,
    generate_dead_code_recommendation, generate_error_swallowing_recommendation,
    generate_test_debt_recommendation, generate_testing_gap_recommendation,
};

// Import and re-export classification functions for backward compatibility
pub use super::classification::{
    classify_debt_type_with_exclusions, classify_risk_based_debt, classify_simple_function_risk,
    classify_test_debt, is_complexity_hotspot, is_dead_code, is_dead_code_with_exclusions,
};

/// Enhanced version of debt type classification (legacy - kept for compatibility)
pub fn classify_debt_type_enhanced(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> DebtType {
    // Test functions are special debt cases
    if func.is_test {
        return classify_test_debt(func);
    }

    let role = classify_function_role(func, func_id, call_graph);

    // Check for complexity hotspots
    if let Some(debt) = is_complexity_hotspot(func, &role) {
        return debt;
    }

    // Check for dead code
    if is_dead_code(func, call_graph, func_id, None) {
        // Import generate_usage_hints from recommendation_extended
        use crate::priority::scoring::recommendation_extended::generate_usage_hints;
        return DebtType::DeadCode {
            visibility: determine_visibility(func),
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            usage_hints: generate_usage_hints(func, call_graph, func_id),
        };
    }

    // Check for simple functions that aren't debt
    if let Some(debt) = classify_simple_function_risk(func, &role) {
        return debt;
    }

    // Default to risk-based classification
    classify_risk_based_debt(func, &role)
}

pub(super) fn generate_recommendation(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    _score: &UnifiedScore,
) -> Option<ActionableRecommendation> {
    generate_recommendation_with_data_flow(func, debt_type, role, _score, None)
}

fn generate_recommendation_with_data_flow(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    _score: &UnifiedScore,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> Option<ActionableRecommendation> {
    generate_recommendation_with_coverage_and_data_flow(
        func, debt_type, role, _score, &None, data_flow,
    )
}

pub(super) fn generate_recommendation_with_coverage_and_data_flow(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    _score: &UnifiedScore,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> Option<ActionableRecommendation> {
    // Try to use new concise recommendation format (spec 138a, spec 201) for supported debt types
    use super::concise_recommendation::generate_concise_recommendation;

    match debt_type {
        DebtType::TestingGap { .. }
        | DebtType::ComplexityHotspot { .. }
        | DebtType::DeadCode { .. } => {
            // Use new concise recommendation format (spec 201: returns None for clean dispatchers)
            return generate_concise_recommendation(debt_type, func, role, coverage);
        }
        _ => {
            // Fall back to legacy format for other debt types
        }
    }

    // Legacy path: Determine if we have actual coverage data (not just a None value)
    let has_coverage_data = coverage.is_some();

    // Create recommendation context using pure functions
    let recommendation_context =
        create_recommendation_context(func, debt_type, role, _score, coverage, has_coverage_data);

    // Generate recommendation using functional composition
    let (primary_action, rationale, steps) =
        generate_context_aware_recommendation(recommendation_context, data_flow);

    Some(build_actionable_recommendation(
        primary_action,
        rationale,
        steps,
    ))
}

// Pure function to create recommendation context
fn create_recommendation_context(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    score: &UnifiedScore,
    coverage: &Option<TransitiveCoverage>,
    has_coverage_data: bool,
) -> RecommendationContext {
    RecommendationContext {
        function_info: FunctionInfo::from_metrics(func),
        debt_type: debt_type.clone(),
        role,
        score: score.clone(),
        coverage: coverage.clone(),
        is_rust_file: is_rust_file(&func.file),
        coverage_percent: extract_coverage_percent(coverage),
        has_coverage_data,
    }
}

// Data structure to hold recommendation context
struct RecommendationContext {
    function_info: FunctionInfo,
    debt_type: DebtType,
    role: FunctionRole,
    score: UnifiedScore,
    coverage: Option<TransitiveCoverage>,
    is_rust_file: bool,
    coverage_percent: f64,
    has_coverage_data: bool,
}

// Pure data structure for function information
struct FunctionInfo {
    file: std::path::PathBuf,
    name: String,
    line: usize,
    nesting: u32,
    length: usize,
    cognitive: u32,
    is_pure: bool,
    purity_confidence: f32,
}

impl FunctionInfo {
    fn from_metrics(func: &FunctionMetrics) -> Self {
        Self {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
            nesting: func.nesting,
            length: func.length,
            cognitive: func.cognitive,
            is_pure: func.is_pure.unwrap_or(false),
            purity_confidence: func.purity_confidence.unwrap_or(0.0),
        }
    }
}

// Pure function to generate context-aware recommendations
fn generate_context_aware_recommendation(
    context: RecommendationContext,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
    match should_use_rust_specific_recommendation(&context) {
        Some(complexity) => generate_rust_complexity_recommendation(&context, complexity),
        None => generate_standard_recommendation_from_context(context, data_flow),
    }
}

// Pure function to determine if Rust-specific recommendation should be used
fn should_use_rust_specific_recommendation(context: &RecommendationContext) -> Option<u32> {
    if context.is_rust_file {
        if let DebtType::ComplexityHotspot { cyclomatic, .. } = &context.debt_type {
            return Some(*cyclomatic);
        }
    }
    None
}

// Pure function to generate Rust complexity recommendation
fn generate_rust_complexity_recommendation(
    context: &RecommendationContext,
    cyclomatic: u32,
) -> (String, String, Vec<String>) {
    let temp_item = create_temporary_debt_item(context);
    generate_rust_refactoring_recommendation(
        &temp_item,
        cyclomatic,
        context.coverage_percent,
        context.has_coverage_data,
    )
}

// Pure function to create temporary debt item for Rust recommendations
fn create_temporary_debt_item(context: &RecommendationContext) -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: context.function_info.file.clone(),
            function: context.function_info.name.clone(),
            line: context.function_info.line,
        },
        debt_type: context.debt_type.clone(),
        unified_score: context.score.clone(),
        function_role: context.role,
        recommendation: ActionableRecommendation {
            primary_action: String::new(),
            rationale: String::new(),
            implementation_steps: vec![],
            related_items: vec![],
            steps: None,
            estimated_effort_hours: None,
        },
        expected_impact: ImpactMetrics {
            risk_reduction: 0.0,
            complexity_reduction: 0.0,
            coverage_improvement: 0.0,
            lines_reduction: 0,
        },
        transitive_coverage: context.coverage.clone(),
        file_context: None,
        upstream_dependencies: 0,
        downstream_dependencies: 0,
        upstream_callers: vec![],
        downstream_callees: vec![],
        nesting_depth: context.function_info.nesting,
        function_length: context.function_info.length,
        cyclomatic_complexity: extract_cyclomatic_complexity(&context.debt_type),
        cognitive_complexity: context.function_info.cognitive,
        entropy_details: None,
        entropy_adjusted_cognitive: None,
        entropy_dampening_factor: None,
        is_pure: Some(context.function_info.is_pure),
        purity_confidence: Some(context.function_info.purity_confidence),
        purity_level: None,
        god_object_indicators: None,
        tier: None,
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
        context_multiplier: None,
        context_type: None,
        language_specific: None, // No language-specific data available in this context (spec 190)
        detected_pattern: None,  // No pattern detection available in this context (spec 204)
        contextual_risk: None,
        file_line_count: None, // No file line count caching for temporary items (spec 204)
        responsibility_category: None, // No responsibility category for temporary items (spec 254)
        error_swallowing_count: None,
        error_swallowing_patterns: None,
    }
}

// Function to generate standard recommendation from context
fn generate_standard_recommendation_from_context(
    context: RecommendationContext,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
    // Convert context back to original parameters for compatibility
    let func = reconstruct_function_metrics(&context);
    generate_standard_recommendation(
        &func,
        &context.debt_type,
        context.role,
        &context.coverage,
        data_flow,
    )
}

// Helper function to reconstruct function metrics from context
fn reconstruct_function_metrics(context: &RecommendationContext) -> FunctionMetrics {
    FunctionMetrics {
        file: context.function_info.file.clone(),
        name: context.function_info.name.clone(),
        line: context.function_info.line,
        nesting: context.function_info.nesting,
        length: context.function_info.length,
        cognitive: context.function_info.cognitive,
        is_pure: Some(context.function_info.is_pure),
        purity_confidence: Some(context.function_info.purity_confidence),
        purity_reason: None,
        call_dependencies: None,
        // Set reasonable defaults for other fields
        cyclomatic: extract_cyclomatic_complexity(&context.debt_type),
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
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

fn generate_standard_recommendation(
    func: &FunctionMetrics,
    debt_type: &DebtType,
    role: FunctionRole,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
    match debt_type {
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
            coverage: coverage_val,
            cyclomatic,
            cognitive,
        } => generate_testing_gap_recommendation(
            *coverage_val,
            *cyclomatic,
            *cognitive,
            role,
            func,
            coverage,
        ),
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => {
            // Always try to use intelligent pattern-based recommendations
            // The DataFlowGraph is passed through but may still be None in some cases
            generate_complexity_recommendation_with_patterns_and_coverage(
                func,
                *cyclomatic,
                *cognitive,
                coverage,
                data_flow,
            )
        }
        DebtType::Duplication { .. } | DebtType::Risk { .. } => {
            generate_infrastructure_recommendation_with_coverage(debt_type, coverage)
        }
        DebtType::TestComplexityHotspot { .. }
        | DebtType::TestTodo { .. }
        | DebtType::TestDuplication { .. } => generate_test_debt_recommendation(debt_type),
        DebtType::ErrorSwallowing { pattern, context } => {
            generate_error_swallowing_recommendation(pattern, context)
        }
        // Security debt types
        // Resource Management debt types
        DebtType::AllocationInefficiency { pattern, impact } => {
            generate_resource_management_recommendation("allocation", pattern, impact)
        }
        DebtType::StringConcatenation {
            loop_type,
            iterations,
        } => generate_string_concat_recommendation(loop_type, iterations),
        DebtType::NestedLoops {
            depth,
            complexity_estimate,
        } => generate_nested_loops_recommendation(*depth, complexity_estimate),
        DebtType::BlockingIO { operation, context } => {
            generate_resource_management_recommendation("blocking_io", operation, context)
        }
        DebtType::SuboptimalDataStructure {
            current_type,
            recommended_type,
        } => generate_data_structure_recommendation(current_type, recommended_type),
        // Organization debt types
        DebtType::GodObject {
            responsibilities,
            god_object_score,
            ..
        } => generate_god_object_recommendation(*responsibilities, god_object_score.value()),
        DebtType::FeatureEnvy {
            external_class,
            usage_ratio,
        } => generate_feature_envy_recommendation(external_class, *usage_ratio),
        DebtType::PrimitiveObsession {
            primitive_type,
            domain_concept,
        } => generate_primitive_obsession_recommendation(primitive_type, domain_concept),
        DebtType::MagicValues { value, occurrences } => {
            generate_magic_values_recommendation(value, *occurrences)
        }
        // Testing quality debt types
        DebtType::AssertionComplexity {
            assertion_count,
            complexity_score,
        } => generate_assertion_complexity_recommendation(*assertion_count, *complexity_score),
        DebtType::FlakyTestPattern {
            pattern_type,
            reliability_impact,
        } => generate_flaky_test_recommendation(pattern_type, reliability_impact),
        // Resource management debt types
        DebtType::AsyncMisuse {
            pattern,
            performance_impact,
        } => generate_async_misuse_recommendation(pattern, performance_impact),
        DebtType::ResourceLeak {
            resource_type,
            cleanup_missing,
        } => generate_resource_leak_recommendation(resource_type, cleanup_missing),
        DebtType::CollectionInefficiency {
            collection_type,
            inefficiency_type,
        } => generate_collection_inefficiency_recommendation(collection_type, inefficiency_type),
        // Type organization debt types (Spec 187)
        DebtType::ScatteredType {
            type_name,
            total_methods,
            file_count,
            ..
        } => (
            format!("Consolidate {} methods into impl block", type_name),
            format!(
                "{} methods scattered across {} files",
                total_methods, file_count
            ),
            vec![
                format!("Move all methods to {}'s definition file", type_name),
                "Update call sites to use methods instead of free functions".to_string(),
                "Run tests to verify behavior is preserved".to_string(),
            ],
        ),
        DebtType::OrphanedFunctions {
            target_type,
            function_count,
            ..
        } => (
            format!(
                "Convert {} functions to {} methods",
                function_count, target_type
            ),
            format!(
                "{} standalone functions should be methods on {}",
                function_count, target_type
            ),
            vec![
                format!("Move functions into impl block for {}", target_type),
                "Change function signatures to use &self or &mut self".to_string(),
                "Update call sites to use method syntax".to_string(),
            ],
        ),
        DebtType::UtilitiesSprawl {
            function_count,
            distinct_types,
        } => (
            "Break up utilities module".to_string(),
            format!(
                "{} functions operating on {} distinct types",
                function_count, distinct_types
            ),
            vec![
                "Move type-specific functions to appropriate modules".to_string(),
                "Create focused utility modules for truly generic functions".to_string(),
                "Consider deleting the utilities module once empty".to_string(),
            ],
        ),
        // Default for legacy variants
        _ => (
            "Address technical debt".to_string(),
            "Review and improve code quality".to_string(),
            vec!["Refactor to follow best practices".to_string()],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::score_types::Score0To100;

    fn assert_float_eq(left: f64, right: f64, epsilon: f64) {
        if (left - right).abs() > epsilon {
            panic!("assertion failed: `(left == right)`\n  left: `{}`,\n right: `{}`\n  diff: `{}`\nepsilon: `{}`", left, right, (left - right).abs(), epsilon);
        }
    }

    #[test]
    fn test_get_role_display_name() {
        assert_eq!(
            get_role_display_name(FunctionRole::PureLogic),
            "Business logic"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::Orchestrator),
            "Orchestration"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::IOWrapper),
            "I/O wrapper"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::EntryPoint),
            "Entry point"
        );
        assert_eq!(
            get_role_display_name(FunctionRole::PatternMatch),
            "Pattern matching"
        );
        assert_eq!(get_role_display_name(FunctionRole::Unknown), "Function");
    }

    #[test]
    fn test_calculate_needed_test_cases_full_coverage() {
        // When coverage is 100%, no test cases needed
        assert_eq!(calculate_needed_test_cases(10, 1.0), 0);
        assert_eq!(calculate_needed_test_cases(25, 1.0), 0);
    }

    #[test]
    fn test_calculate_needed_test_cases_no_coverage() {
        // Cyclo=10 uses Simple tier (linear): 10 × 1.0 = 10
        assert_eq!(calculate_needed_test_cases(10, 0.0), 10);
        // Cyclo=25 uses Moderate tier (sqrt): sqrt(25) * 1.5 + 2 = 5 * 1.5 + 2 = 9.5 → ceil = 10
        assert_eq!(calculate_needed_test_cases(25, 0.0), 10);
    }

    #[test]
    fn test_classify_test_debt() {
        let test_func = FunctionMetrics {
            name: "test_something".to_string(),
            file: std::path::PathBuf::from("tests/test.rs"),
            line: 10,
            length: 20,
            cyclomatic: 4,
            cognitive: 6,
            nesting: 1,
            visibility: Some("pub".to_string()),
            is_test: true,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.3),
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

        let debt = classify_test_debt(&test_func);
        match debt {
            DebtType::TestingGap {
                coverage,
                cyclomatic,
                cognitive,
            } => {
                assert_float_eq(coverage, 0.0, 0.01);
                assert_eq!(cyclomatic, 4);
                assert_eq!(cognitive, 6);
            }
            _ => panic!("Expected TestingGap debt type for test function"),
        }
    }

    #[test]
    fn test_calculate_needed_test_cases_partial_coverage() {
        // Cyclo=10 uses Simple tier (linear formula)
        // When 50% covered: 10 × 0.5 = 5
        assert_eq!(calculate_needed_test_cases(10, 0.5), 5);
        // When 80% covered: 10 × 0.2 = 2
        assert_eq!(calculate_needed_test_cases(10, 0.8), 2);
        // When 25% covered: 10 × 0.75 = 7.5 → ceil = 8
        assert_eq!(calculate_needed_test_cases(10, 0.25), 8);
    }

    #[test]
    fn test_calculate_simple_test_cases_minimum() {
        // Always returns at least 2 test cases
        assert_eq!(calculate_simple_test_cases(1, 0.5), 2);
        assert_eq!(calculate_simple_test_cases(1, 0.9), 2);
    }

    #[test]
    fn test_calculate_simple_test_cases_no_coverage() {
        // With no coverage, uses cyclomatic complexity (min 2)
        assert_eq!(calculate_simple_test_cases(5, 0.0), 5);
        assert_eq!(calculate_simple_test_cases(1, 0.0), 2);
    }

    #[test]
    fn test_calculate_simple_test_cases_partial_coverage() {
        // With partial coverage, calculates proportionally
        assert_eq!(calculate_simple_test_cases(10, 0.5), 5);
        assert_eq!(calculate_simple_test_cases(10, 0.8), 2);
    }

    #[test]
    fn test_generate_full_coverage_recommendation() {
        let (action, rationale, steps) =
            generate_full_coverage_recommendation(FunctionRole::PureLogic);
        assert_eq!(action, "Maintain test coverage");
        assert!(rationale.contains("Business logic"));
        assert!(rationale.contains("100% covered"));
        assert_eq!(steps.len(), 3);
        assert!(steps[0].contains("up to date"));
        assert!(steps[1].contains("property-based testing"));
        assert!(steps[2].contains("CI/CD"));
    }

    #[test]
    fn test_generate_full_coverage_recommendation_different_roles() {
        for role in [
            FunctionRole::Orchestrator,
            FunctionRole::IOWrapper,
            FunctionRole::EntryPoint,
            FunctionRole::PatternMatch,
            FunctionRole::Unknown,
        ] {
            let (action, rationale, _) = generate_full_coverage_recommendation(role);
            assert_eq!(action, "Maintain test coverage");
            assert!(rationale.contains("100% covered"));
        }
    }

    #[test]
    fn test_generate_testing_gap_recommendation_full_coverage() {
        // Test case for fully covered function
        let func = FunctionMetrics {
            name: "test_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 20,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(1.0),
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

        let (action, rationale, steps) = generate_testing_gap_recommendation(
            1.0, // 100% coverage
            5,   // cyclomatic
            8,   // cognitive
            FunctionRole::PureLogic,
            &func,
            &None,
        );

        assert_eq!(action, "Maintain test coverage");
        assert!(rationale.contains("Business logic function is currently 100% covered"));
        assert_eq!(steps.len(), 3);
        assert!(steps[0].contains("Keep tests up to date"));
    }

    #[test]
    fn test_generate_testing_gap_recommendation_complex_function_no_coverage() {
        // Test case for complex function with no coverage
        let func = FunctionMetrics {
            name: "complex_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 25,
            cognitive: 41,
            nesting: 4,
            length: 117,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(1.0),
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

        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.0, // 0% coverage
            25,  // cyclomatic
            41,  // cognitive
            FunctionRole::PureLogic,
            &func,
            &None,
        );

        // Should recommend adding tests and refactoring
        assert!(action.contains("Add"));
        assert!(action.contains("tests"));
        assert!(action.contains("100% coverage gap"));
        assert!(action.contains("refactor complexity"));
        assert!(rationale.contains("Complex"));
        assert!(rationale.contains("100% gap"));
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_generate_testing_gap_recommendation_complex_function_partial_coverage() {
        // Test case for complex function with partial coverage
        let func = FunctionMetrics {
            name: "complex_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 15,
            cognitive: 20,
            nesting: 3,
            length: 80,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.8),
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

        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.6, // 60% coverage
            15,  // cyclomatic
            20,  // cognitive
            FunctionRole::Orchestrator,
            &func,
            &None,
        );

        // Should recommend adding tests for 40% gap and refactoring
        assert!(action.contains("40% coverage gap"));
        assert!(action.contains("refactor complexity"));
        assert!(rationale.contains("Complex"));
        assert!(rationale.contains("40% gap"));
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_function_no_coverage() {
        // Test case for simple function with no coverage
        let func = FunctionMetrics {
            name: "simple_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 20,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.9),
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

        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.0, // 0% coverage
            5,   // cyclomatic
            8,   // cognitive
            FunctionRole::IOWrapper,
            &func,
            &None,
        );

        // Should recommend adding tests for simple function
        assert!(action.contains("Add"));
        assert!(action.contains("test"));
        assert!(action.contains("100% coverage"));
        assert!(rationale.contains("I/O wrapper"));
        assert!(rationale.contains("100% coverage gap"));
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_generate_testing_gap_recommendation_simple_function_partial_coverage() {
        // Test case for simple function with partial coverage
        let func = FunctionMetrics {
            name: "simple_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 8,
            cognitive: 10,
            nesting: 2,
            length: 30,
            is_test: false,
            visibility: Some("pub".to_string()),
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.95),
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

        let (action, rationale, steps) = generate_testing_gap_recommendation(
            0.75, // 75% coverage
            8,    // cyclomatic
            10,   // cognitive
            FunctionRole::EntryPoint,
            &func,
            &None,
        );

        // Should recommend adding tests for 25% gap
        assert!(action.contains("Add"));
        assert!(action.contains("test"));
        assert!(action.contains("25% coverage"));
        assert!(rationale.contains("Entry point"));
        assert!(rationale.contains("25% coverage gap"));
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_generate_testing_gap_recommendation_with_uncovered_lines() {
        // Test case with transitive coverage data
        let func = FunctionMetrics {
            name: "func_with_gaps".to_string(),
            file: "test.rs".into(),
            line: 10,
            cyclomatic: 6,
            cognitive: 9,
            nesting: 2,
            length: 25,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(1.0),
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

        let transitive_cov = TransitiveCoverage {
            direct: 0.5,
            transitive: 0.5,
            propagated_from: vec![],
            uncovered_lines: vec![13, 14, 17, 18, 19],
        };

        let (action, _rationale, steps) = generate_testing_gap_recommendation(
            0.5, // 50% coverage
            6,   // cyclomatic
            9,   // cognitive
            FunctionRole::PureLogic,
            &func,
            &Some(transitive_cov),
        );

        // Should include uncovered lines analysis in steps
        assert!(action.contains("50% coverage"));
        assert!(!steps.is_empty());
        // The steps should include recommendations from analyze_uncovered_lines
    }

    #[test]
    fn test_generate_testing_gap_recommendation_edge_at_complexity_threshold() {
        // Test edge case right at complexity threshold
        let func = FunctionMetrics {
            name: "edge_func".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 10,
            cognitive: 15,
            nesting: 3,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.85),
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

        // Test at cyclomatic=10 (not complex)
        let (action1, _, _) = generate_testing_gap_recommendation(
            0.3, // 30% coverage
            10,  // cyclomatic - at threshold
            14,  // cognitive - below threshold
            FunctionRole::PatternMatch,
            &func,
            &None,
        );

        // Should be treated as simple
        assert!(action1.contains("70% coverage"));
        assert!(!action1.contains("refactor complexity"));

        // Test at cyclomatic=11 (complex)
        let (action2, _, _) = generate_testing_gap_recommendation(
            0.3, // 30% coverage
            11,  // cyclomatic - above threshold
            14,  // cognitive - below threshold
            FunctionRole::PatternMatch,
            &func,
            &None,
        );

        // Should be treated as complex
        assert!(action2.contains("70% coverage gap"));
        assert!(action2.contains("refactor complexity"));
    }

    // Tests for extracted pure functions (spec 93)

    #[test]
    fn test_create_function_id() {
        use crate::priority::scoring::construction::create_function_id;

        let func = FunctionMetrics {
            name: "test_func".to_string(),
            file: "/path/to/file.rs".into(),
            line: 42,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 20,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.9),
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

        let func_id = create_function_id(&func);
        assert_eq!(func_id.name, "test_func");
        assert_eq!(func_id.file.to_str().unwrap(), "/path/to/file.rs");
        assert_eq!(func_id.line, 42);
    }

    #[test]
    fn test_calculate_functions_to_extract() {
        // Test various complexity levels
        assert_eq!(calculate_functions_to_extract(5, 5), 2);
        assert_eq!(calculate_functions_to_extract(10, 10), 2);
        assert_eq!(calculate_functions_to_extract(15, 15), 3);
        assert_eq!(calculate_functions_to_extract(20, 20), 4);
        assert_eq!(calculate_functions_to_extract(25, 25), 5);
        assert_eq!(calculate_functions_to_extract(30, 30), 6);
        assert_eq!(calculate_functions_to_extract(50, 50), 10);

        // Test with different cyclomatic and cognitive values
        assert_eq!(calculate_functions_to_extract(10, 20), 4);
        assert_eq!(calculate_functions_to_extract(30, 15), 6);
    }

    #[test]
    fn test_format_complexity_display() {
        assert_eq!(format_complexity_display(&5, &8), "cyclo=5, cog=8");
        assert_eq!(format_complexity_display(&10, &15), "cyclo=10, cog=15");
        assert_eq!(format_complexity_display(&0, &0), "cyclo=0, cog=0");
    }

    #[test]
    fn test_format_role_description() {
        assert_eq!(
            format_role_description(FunctionRole::PureLogic),
            "business logic"
        );
        assert_eq!(
            format_role_description(FunctionRole::Orchestrator),
            "orchestration"
        );
        assert_eq!(
            format_role_description(FunctionRole::IOWrapper),
            "I/O wrapper"
        );
        assert_eq!(
            format_role_description(FunctionRole::EntryPoint),
            "entry point"
        );
        assert_eq!(
            format_role_description(FunctionRole::PatternMatch),
            "pattern matching"
        );
        assert_eq!(format_role_description(FunctionRole::Unknown), "function");
    }

    #[test]
    fn test_is_function_complex() {
        // Test not complex
        assert!(!is_function_complex(5, 10));
        assert!(!is_function_complex(10, 15));

        // Test complex based on cyclomatic
        assert!(is_function_complex(11, 10));
        assert!(is_function_complex(20, 5));

        // Test complex based on cognitive
        assert!(is_function_complex(5, 16));
        assert!(is_function_complex(10, 20));

        // Test complex based on both
        assert!(is_function_complex(15, 20));
    }

    #[test]
    fn test_calculate_risk_factor() {
        // Test various debt types
        assert_eq!(
            calculate_risk_factor(&DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 10,
                cognitive: 15
            }),
            0.42
        );

        assert_eq!(
            calculate_risk_factor(&DebtType::ComplexityHotspot {
                cyclomatic: 20,
                cognitive: 25,
            }),
            0.35
        );

        assert_eq!(
            calculate_risk_factor(&DebtType::ErrorSwallowing {
                pattern: "unwrap_or_default".to_string(),
                context: None
            }),
            0.35
        );

        assert_eq!(
            calculate_risk_factor(&DebtType::DeadCode {
                visibility: FunctionVisibility::Private,
                cyclomatic: 5,
                cognitive: 8,
                usage_hints: vec![]
            }),
            0.3
        );
    }

    #[test]
    fn test_calculate_coverage_improvement() {
        // Test simple function
        assert_float_eq(calculate_coverage_improvement(0.0, false), 100.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.5, false), 50.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.8, false), 20.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(1.0, false), 0.0, 1e-10);

        // Test complex function (50% reduction)
        assert_float_eq(calculate_coverage_improvement(0.0, true), 50.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.5, true), 25.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(0.8, true), 10.0, 1e-10);
        assert_float_eq(calculate_coverage_improvement(1.0, true), 0.0, 1e-10);
    }

    #[test]
    fn test_calculate_lines_reduction() {
        // Test dead code
        let dead_code = DebtType::DeadCode {
            visibility: FunctionVisibility::Private,
            cyclomatic: 10,
            cognitive: 15,
            usage_hints: vec![],
        };
        assert_eq!(calculate_lines_reduction(&dead_code), 25);

        // Test duplication
        let duplication = DebtType::Duplication {
            instances: 4,
            total_lines: 100,
        };
        assert_eq!(calculate_lines_reduction(&duplication), 75);

        // Test other types
        let complexity = DebtType::ComplexityHotspot {
            cyclomatic: 20,
            cognitive: 25,
        };
        assert_eq!(calculate_lines_reduction(&complexity), 0);
    }

    #[test]
    fn test_is_rust_file() {
        use std::path::Path;
        assert!(is_rust_file(Path::new("test.rs")));
        assert!(is_rust_file(Path::new("/path/to/file.rs")));
        assert!(!is_rust_file(Path::new("test.py")));
        assert!(!is_rust_file(Path::new("test.js")));
        assert!(!is_rust_file(Path::new("test")));
    }

    #[test]
    fn test_extract_coverage_percent() {
        // Test with coverage
        let coverage = TransitiveCoverage {
            direct: 0.75,
            transitive: 0.85,
            propagated_from: vec![],
            uncovered_lines: vec![],
        };
        assert_eq!(extract_coverage_percent(&Some(coverage)), 0.75);

        // Test without coverage
        assert_eq!(extract_coverage_percent(&None), 0.0);
    }

    #[test]
    fn test_extract_cyclomatic_complexity() {
        assert_eq!(
            extract_cyclomatic_complexity(&DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
            }),
            15
        );

        assert_eq!(
            extract_cyclomatic_complexity(&DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 10,
                cognitive: 12,
            }),
            0
        );

        assert_eq!(
            extract_cyclomatic_complexity(&DebtType::DeadCode {
                visibility: FunctionVisibility::Public,
                cyclomatic: 5,
                cognitive: 8,
                usage_hints: vec![],
            }),
            0
        );
    }

    #[test]
    fn test_build_actionable_recommendation() {
        let recommendation = build_actionable_recommendation(
            "Fix the issue".to_string(),
            "This is why it matters".to_string(),
            vec!["Step 1".to_string(), "Step 2".to_string()],
        );

        assert_eq!(recommendation.primary_action, "Fix the issue");
        assert_eq!(recommendation.rationale, "This is why it matters");
        assert_eq!(recommendation.implementation_steps.len(), 2);
        assert_eq!(recommendation.implementation_steps[0], "Step 1");
        assert_eq!(recommendation.implementation_steps[1], "Step 2");
        assert!(recommendation.related_items.is_empty());
    }

    #[test]
    fn test_create_recommendation_context() {
        let func = FunctionMetrics {
            name: "test_func".to_string(),
            file: "/test.rs".into(),
            line: 10,
            cyclomatic: 15,
            cognitive: 20,
            nesting: 3,
            length: 50,
            is_test: false,
            visibility: Some("pub".to_string()),
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.95),
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

        let debt_type = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 20,
        };

        let score = UnifiedScore {
            complexity_factor: 7.5,
            coverage_factor: 6.0,
            dependency_factor: 2.0,
            role_multiplier: 1.2,
            final_score: Score0To100::new(8.5),
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
        };

        let coverage = TransitiveCoverage {
            direct: 0.6,
            transitive: 0.7,
            propagated_from: vec![],
            uncovered_lines: vec![15, 16, 20],
        };

        let context = create_recommendation_context(
            &func,
            &debt_type,
            FunctionRole::PureLogic,
            &score,
            &Some(coverage.clone()),
            true, // has_coverage_data
        );

        assert_eq!(context.function_info.name, "test_func");
        assert_eq!(context.function_info.line, 10);
        assert!(context.is_rust_file);
        assert_eq!(context.coverage_percent, 0.6);
        assert!(context.has_coverage_data);
        assert_eq!(context.role, FunctionRole::PureLogic);
    }

    #[test]
    fn test_function_info_from_metrics() {
        let func = FunctionMetrics {
            name: "my_function".to_string(),
            file: "/src/lib.rs".into(),
            line: 100,
            cyclomatic: 8,
            cognitive: 12,
            nesting: 2,
            length: 35,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.85),
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

        let info = FunctionInfo::from_metrics(&func);

        assert_eq!(info.name, "my_function");
        assert_eq!(info.file.to_str().unwrap(), "/src/lib.rs");
        assert_eq!(info.line, 100);
        assert_eq!(info.nesting, 2);
        assert_eq!(info.length, 35);
        assert_eq!(info.cognitive, 12);
        assert!(info.is_pure);
        assert_eq!(info.purity_confidence, 0.85);
    }
}

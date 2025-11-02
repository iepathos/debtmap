use debtmap::core::FunctionMetrics;
use debtmap::extraction_patterns::{
    AccumulationOp, AnalysisContext, ExtractablePattern, ExtractionAnalyzer, ExtractionSuggestion,
    GuardCheck, MatchedPattern, ReturnType, TransformStage, UnifiedExtractionAnalyzer,
    VerbosityLevel,
};
use std::path::PathBuf;

#[test]
fn test_unified_extraction_analyzer_creation() {
    let analyzer = UnifiedExtractionAnalyzer::new();
    assert!(analyzer
        .generate_recommendation(&create_test_suggestion(), VerbosityLevel::Summary)
        .contains("confidence"));
}

#[test]
fn test_extraction_recommendation_generation() {
    let analyzer = UnifiedExtractionAnalyzer::new();
    let suggestion = create_test_suggestion();

    // Test summary level
    let summary = analyzer.generate_recommendation(&suggestion, VerbosityLevel::Summary);
    assert!(summary.contains("calculate_sum"));
    assert!(summary.contains("85%"));
    assert!(summary.contains("lines 10-20"));

    // Test normal level
    let normal = analyzer.generate_recommendation(&suggestion, VerbosityLevel::Normal);
    assert!(normal.contains("Parameters:"));
    assert!(normal.contains("Returns:"));
    assert!(normal.contains("Complexity reduction:"));

    // Test detailed level
    let detailed = analyzer.generate_recommendation(&suggestion, VerbosityLevel::Detailed);
    assert!(detailed.contains("Example transformation:"));
    assert!(detailed.contains("Extracted function complexity:"));
}

#[test]
fn test_confidence_scoring_high() {
    let pattern = ExtractablePattern::AccumulationLoop {
        iterator_binding: "item".to_string(),
        accumulator: "sum".to_string(),
        operation: AccumulationOp::Sum,
        filter: None,
        transform: None,
        start_line: 10,
        end_line: 20,
    };

    let context = AnalysisContext {
        function_name: "test_func".to_string(),
        file_path: "test.rs".to_string(),
        language: "rust".to_string(),
        complexity_before: 5,
        has_side_effects: false,
        data_dependencies: vec![],
    };

    let matched = MatchedPattern {
        pattern,
        confidence: 0.0,
        context: context.clone(),
    };

    use debtmap::extraction_patterns::confidence::ConfidenceScorer;
    let score = ConfidenceScorer::score_pattern(&matched, &context);
    assert!(
        score > 0.8,
        "Accumulation loop without side effects should have high confidence"
    );
}

#[test]
fn test_confidence_scoring_with_side_effects() {
    let pattern = ExtractablePattern::TransformationPipeline {
        stages: vec![TransformStage {
            operation: "map".to_string(),
            input: "item".to_string(),
            output: "transformed".to_string(),
        }],
        input_binding: "data".to_string(),
        output_type: "Vec<T>".to_string(),
        start_line: 10,
        end_line: 20,
    };

    let context_no_side_effects = AnalysisContext {
        function_name: "test_func".to_string(),
        file_path: "test.rs".to_string(),
        language: "rust".to_string(),
        complexity_before: 5,
        has_side_effects: false,
        data_dependencies: vec![],
    };

    let context_with_side_effects = AnalysisContext {
        has_side_effects: true,
        ..context_no_side_effects.clone()
    };

    let matched_no_side = MatchedPattern {
        pattern: pattern.clone(),
        confidence: 0.0,
        context: context_no_side_effects.clone(),
    };

    let matched_with_side = MatchedPattern {
        pattern,
        confidence: 0.0,
        context: context_with_side_effects.clone(),
    };

    use debtmap::extraction_patterns::confidence::ConfidenceScorer;
    let score_no_side = ConfidenceScorer::score_pattern(&matched_no_side, &context_no_side_effects);
    let score_with_side =
        ConfidenceScorer::score_pattern(&matched_with_side, &context_with_side_effects);

    assert!(
        score_no_side > score_with_side,
        "Functions with side effects should have lower confidence"
    );
}

#[test]
fn test_function_name_inference() {
    use debtmap::extraction_patterns::naming::FunctionNameInferrer;

    // Test accumulation pattern naming
    let acc_pattern = ExtractablePattern::AccumulationLoop {
        iterator_binding: "item".to_string(),
        accumulator: "total".to_string(),
        operation: AccumulationOp::Sum,
        filter: None,
        transform: None,
        start_line: 1,
        end_line: 5,
    };

    let rust_name = FunctionNameInferrer::infer_name(&acc_pattern, "rust");
    assert!(rust_name.contains("sum"));
    assert_eq!(rust_name, "sum_item");

    // Test guard chain naming
    let guard_pattern = ExtractablePattern::GuardChainSequence {
        checks: vec![GuardCheck {
            condition: "value > 0".to_string(),
            return_value: Some("Error".to_string()),
            line: 1,
        }],
        early_return: ReturnType {
            type_name: "Result<()>".to_string(),
            is_early_return: true,
        },
        start_line: 1,
        end_line: 5,
    };

    let guard_name = FunctionNameInferrer::infer_name(&guard_pattern, "rust");
    assert!(guard_name.contains("validate"));
    assert_eq!(guard_name, "validate_precondition");

    // Test JavaScript camelCase naming
    let js_name = FunctionNameInferrer::infer_name(&guard_pattern, "javascript");
    assert_eq!(js_name, "validatePrecondition");
}

#[test]
fn test_complexity_impact_calculation() {
    let suggestion = create_test_suggestion();

    assert_eq!(suggestion.complexity_reduction.current_cyclomatic, 15);
    assert_eq!(suggestion.complexity_reduction.predicted_cyclomatic, 12);
    assert_eq!(
        suggestion
            .complexity_reduction
            .extracted_function_complexity,
        3
    );
}

#[test]
fn test_pattern_matching_for_different_languages() {
    let analyzer = UnifiedExtractionAnalyzer::new();

    // Create test function metrics for different languages
    let rust_func = FunctionMetrics {
        name: "test_rust".to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic: 10,
        cognitive: 15,
        nesting: 2,
        length: 50,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
    };

    let python_func = FunctionMetrics {
        name: "test_python".to_string(),
        file: PathBuf::from("test.py"),
        line: 10,
        cyclomatic: 10,
        cognitive: 15,
        nesting: 2,
        length: 50,
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
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
    };

    let js_func = FunctionMetrics {
        name: "testJavaScript".to_string(),
        file: PathBuf::from("test.js"),
        line: 10,
        cyclomatic: 10,
        cognitive: 15,
        nesting: 2,
        length: 50,
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
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
    };

    // Create dummy file metrics for testing
    let file_metrics = debtmap::core::FileMetrics {
        path: std::path::PathBuf::from("test.rs"),
        language: debtmap::core::Language::Rust,
        complexity: debtmap::core::ComplexityMetrics::default(),
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
        module_scope: None,
        classes: None,
    };

    // Test that analyzer handles different languages
    let rust_suggestions = analyzer.analyze_function(&rust_func, &file_metrics, None);

    let python_suggestions = analyzer.analyze_function(&python_func, &file_metrics, None);

    let js_suggestions = analyzer.analyze_function(&js_func, &file_metrics, None);

    // Currently returns empty as we haven't integrated actual AST parsing
    // But the infrastructure is in place
    assert_eq!(rust_suggestions.len(), 0);
    assert_eq!(python_suggestions.len(), 0);
    assert_eq!(js_suggestions.len(), 0);
}

// Helper function to create a test suggestion
fn create_test_suggestion() -> ExtractionSuggestion {
    use debtmap::extraction_patterns::{ComplexityImpact, Parameter};

    ExtractionSuggestion {
        pattern_type: ExtractablePattern::AccumulationLoop {
            iterator_binding: "item".to_string(),
            accumulator: "sum".to_string(),
            operation: AccumulationOp::Sum,
            filter: None,
            transform: None,
            start_line: 10,
            end_line: 20,
        },
        start_line: 10,
        end_line: 20,
        suggested_name: "calculate_sum".to_string(),
        confidence: 0.85,
        parameters: vec![Parameter {
            name: "items".to_string(),
            type_hint: "Vec<i32>".to_string(),
            is_mutable: false,
        }],
        return_type: "i32".to_string(),
        complexity_reduction: ComplexityImpact {
            current_cyclomatic: 15,
            predicted_cyclomatic: 12,
            current_cognitive: 20,
            predicted_cognitive: 15,
            extracted_function_complexity: 3,
        },
        example_transformation: "// Example transformation".to_string(),
    }
}

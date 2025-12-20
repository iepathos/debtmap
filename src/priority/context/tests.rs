//! Integration tests for context suggestion generation (Spec 263).

#[cfg(test)]
mod integration_tests {
    use crate::priority::context::{generate_context_suggestion, ContextConfig, FileRange};
    use crate::priority::unified_scorer::{Location, UnifiedDebtItem, UnifiedScore};
    use crate::priority::{
        ActionableRecommendation, CallGraph, DebtType, FunctionRole, ImpactMetrics,
    };
    use std::path::PathBuf;

    /// Create a test UnifiedDebtItem with known location for testing.
    fn create_test_debt_item(
        file: &str,
        function: &str,
        line: usize,
        length: usize,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from(file),
                function: function.to_string(),
                line,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 3.0,
                dependency_factor: 2.0,
                role_multiplier: 1.0,
                final_score: 35.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation::default(),
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 2,
            function_length: length,
            cyclomatic_complexity: 15,
            cognitive_complexity: 20,
            entropy_details: None,
            entropy_analysis: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            is_pure: Some(false),
            purity_confidence: Some(0.8),
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            file_context: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            context_suggestion: None,
        }
    }

    #[test]
    fn test_context_suggestion_generates_valid_file_ranges() {
        let item = create_test_debt_item("src/lib.rs", "complex_function", 100, 50);
        let call_graph = CallGraph::new();
        let config = ContextConfig::default();

        let suggestion = generate_context_suggestion(&item, &call_graph, &config);

        assert!(
            suggestion.is_some(),
            "Context suggestion should be generated"
        );
        let ctx = suggestion.unwrap();

        // Verify primary range has valid line numbers
        assert!(
            ctx.primary.start_line <= ctx.primary.end_line,
            "Primary range start_line ({}) should be <= end_line ({})",
            ctx.primary.start_line,
            ctx.primary.end_line
        );

        // Verify all related ranges have valid line numbers
        for (i, related) in ctx.related.iter().enumerate() {
            assert!(
                related.range.start_line <= related.range.end_line,
                "Related range {} start_line ({}) should be <= end_line ({})",
                i,
                related.range.start_line,
                related.range.end_line
            );
        }
    }

    #[test]
    fn test_context_suggestion_total_lines_matches_sum() {
        let item = create_test_debt_item("src/main.rs", "process_data", 50, 30);
        let call_graph = CallGraph::new();
        let config = ContextConfig::default();

        let suggestion = generate_context_suggestion(&item, &call_graph, &config);

        assert!(
            suggestion.is_some(),
            "Context suggestion should be generated"
        );
        let ctx = suggestion.unwrap();

        // Calculate expected total from primary + related ranges
        let primary_lines = ctx.primary.line_count();
        let related_lines: u32 = ctx.related.iter().map(|r| r.range.line_count()).sum();
        let expected_total = primary_lines + related_lines;

        // Total lines should match sum of all ranges
        assert_eq!(
            ctx.total_lines, expected_total,
            "total_lines ({}) should equal sum of primary ({}) + related ({}) = {}",
            ctx.total_lines, primary_lines, related_lines, expected_total
        );
    }

    #[test]
    fn test_context_suggestion_primary_contains_function() {
        let item = create_test_debt_item("src/parser.rs", "parse_expression", 200, 40);
        let call_graph = CallGraph::new();
        let config = ContextConfig::default();

        let suggestion = generate_context_suggestion(&item, &call_graph, &config);

        assert!(suggestion.is_some());
        let ctx = suggestion.unwrap();

        // Primary range should include the function's location
        // The function starts at line 200, so start_line should be <= 200
        // and end_line should be >= 200 + function_length
        assert!(
            ctx.primary.start_line <= item.location.line as u32,
            "Primary start_line ({}) should include function start ({})",
            ctx.primary.start_line,
            item.location.line
        );

        // Check that symbol matches function name
        assert_eq!(
            ctx.primary.symbol,
            Some(item.location.function.clone()),
            "Primary symbol should match function name"
        );
    }

    #[test]
    fn test_context_suggestion_completeness_confidence_valid() {
        let item = create_test_debt_item("src/utils.rs", "helper_fn", 10, 15);
        let call_graph = CallGraph::new();
        let config = ContextConfig::default();

        let suggestion = generate_context_suggestion(&item, &call_graph, &config);

        assert!(suggestion.is_some());
        let ctx = suggestion.unwrap();

        // Confidence should be between 0.0 and 1.0
        assert!(
            (0.0..=1.0).contains(&ctx.completeness_confidence),
            "completeness_confidence ({}) should be in range [0.0, 1.0]",
            ctx.completeness_confidence
        );
    }

    #[test]
    fn test_context_suggestion_respects_max_total_lines_for_related() {
        // When primary fits within budget, related contexts should be limited
        let item = create_test_debt_item("src/file.rs", "small_function", 50, 20);
        let call_graph = CallGraph::new();
        let config = ContextConfig {
            max_total_lines: 50,
            max_callers: 2,
            max_callees: 2,
            include_tests: true,
            include_types: true,
        };

        let suggestion = generate_context_suggestion(&item, &call_graph, &config);

        assert!(suggestion.is_some());
        let ctx = suggestion.unwrap();

        // Total lines should respect budget by limiting related contexts
        assert!(
            ctx.total_lines <= config.max_total_lines,
            "total_lines ({}) should not exceed max_total_lines ({}) when primary fits",
            ctx.total_lines,
            config.max_total_lines
        );
    }

    #[test]
    fn test_context_suggestion_primary_exceeds_budget_clears_related() {
        // When primary itself exceeds budget, related should be cleared
        let item = create_test_debt_item("src/large_file.rs", "big_function", 500, 200);
        let call_graph = CallGraph::new();
        let config = ContextConfig {
            max_total_lines: 100, // Budget smaller than function length
            max_callers: 2,
            max_callees: 2,
            include_tests: true,
            include_types: true,
        };

        let suggestion = generate_context_suggestion(&item, &call_graph, &config);

        assert!(suggestion.is_some());
        let ctx = suggestion.unwrap();

        // When primary exceeds budget, related should be cleared
        // and total_lines equals primary only
        assert!(
            ctx.related.is_empty(),
            "Related contexts should be cleared when primary exceeds budget"
        );
        assert_eq!(
            ctx.total_lines,
            ctx.primary.line_count(),
            "total_lines should equal primary when budget exceeded"
        );

        // Completeness confidence should be reduced
        assert!(
            ctx.completeness_confidence < 0.8,
            "Completeness confidence ({}) should be reduced when budget exceeded",
            ctx.completeness_confidence
        );
    }

    #[test]
    fn test_file_range_line_count_calculation() {
        // Test FileRange::line_count() method
        let range = FileRange {
            file: PathBuf::from("test.rs"),
            start_line: 10,
            end_line: 20,
            symbol: None,
        };

        assert_eq!(
            range.line_count(),
            11,
            "Lines 10-20 should be 11 lines inclusive"
        );

        // Edge case: single line range
        let single_line = FileRange {
            file: PathBuf::from("test.rs"),
            start_line: 5,
            end_line: 5,
            symbol: None,
        };

        assert_eq!(
            single_line.line_count(),
            1,
            "Single line range should have 1 line"
        );

        // Edge case: inverted range (shouldn't happen but should handle gracefully)
        let inverted = FileRange {
            file: PathBuf::from("test.rs"),
            start_line: 20,
            end_line: 10,
            symbol: None,
        };

        assert_eq!(inverted.line_count(), 0, "Inverted range should return 0");
    }

    #[test]
    fn test_context_suggestion_includes_module_header() {
        let item = create_test_debt_item("src/mod.rs", "init", 50, 20);
        let call_graph = CallGraph::new();
        let config = ContextConfig::default();

        let suggestion = generate_context_suggestion(&item, &call_graph, &config);

        assert!(suggestion.is_some());
        let ctx = suggestion.unwrap();

        // Should include module header context
        let has_module_header = ctx
            .related
            .iter()
            .any(|r| r.relationship == crate::priority::context::ContextRelationship::ModuleHeader);

        assert!(
            has_module_header,
            "Context suggestion should include module header context"
        );
    }

    #[test]
    fn test_context_suggestion_file_path_preserved() {
        let test_file = "src/deeply/nested/module.rs";
        let item = create_test_debt_item(test_file, "nested_fn", 25, 10);
        let call_graph = CallGraph::new();
        let config = ContextConfig::default();

        let suggestion = generate_context_suggestion(&item, &call_graph, &config);

        assert!(suggestion.is_some());
        let ctx = suggestion.unwrap();

        // Primary file path should match input
        assert_eq!(
            ctx.primary.file,
            PathBuf::from(test_file),
            "Primary file path should be preserved"
        );
    }
}

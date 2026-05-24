//! Integration tests for context suggestion generation (Spec 263).

#[cfg(test)]
mod integration_tests {
    use crate::common::{LocationConfidence, SourceLocation};
    use crate::organization::{
        DetectionType, GodObjectAnalysis, GodObjectConfidence, SplitAnalysisMethod,
    };
    use crate::priority::call_graph::{CallType, FunctionId};
    use crate::priority::context::{
        generate_context_suggestion, ContextConfig, ContextRelationship, FileRange,
    };
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
                debt_type_multiplier: None,
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
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 2,
            function_length: length,
            cyclomatic_complexity: 15,
            cognitive_complexity: 20,
            entropy_analysis: None,
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
    fn test_context_suggestion_primary_clamps_to_file_line_count() {
        let mut item = create_test_debt_item(
            "src/builders/parallel_unified_analysis.rs",
            "ParallelUnifiedAnalysisBuilder",
            503,
            1750,
        );
        item.file_line_count = Some(1874);
        let call_graph = CallGraph::new();
        let config = ContextConfig::default();

        let suggestion = generate_context_suggestion(&item, &call_graph, &config)
            .expect("Context suggestion should be generated");

        assert_eq!(suggestion.primary.start_line, 501);
        assert_eq!(
            suggestion.primary.end_line, 1874,
            "Primary context should not point past EOF"
        );
    }

    #[test]
    fn test_god_class_context_uses_struct_source_span() {
        let mut item = create_test_debt_item(
            "src/builders/parallel_unified_analysis.rs",
            "ParallelUnifiedAnalysisBuilder",
            503,
            1750,
        );
        item.file_line_count = Some(1874);
        item.god_object_indicators = Some(GodObjectAnalysis {
            is_god_object: true,
            method_count: 50,
            weighted_method_count: None,
            field_count: 10,
            responsibility_count: 5,
            lines_of_code: 1750,
            complexity_sum: 157,
            god_object_score: 89.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec![],
            responsibility_method_counts: Default::default(),
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            struct_name: Some("ParallelUnifiedAnalysisBuilder".to_string()),
            struct_line: Some(503),
            struct_location: Some(SourceLocation {
                line: 503,
                column: None,
                end_line: Some(1546),
                end_column: None,
                confidence: LocationConfidence::Approximate,
            }),
            visibility_breakdown: None,
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,
            trait_method_summary: None,
        });

        let suggestion =
            generate_context_suggestion(&item, &CallGraph::new(), &ContextConfig::default())
                .expect("Context suggestion should be generated");

        assert_eq!(suggestion.primary.start_line, 501);
        assert_eq!(
            suggestion.primary.end_line, 1548,
            "GodClass context should use the struct/impl source span, not production LOC or EOF"
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

    // =========================================================================
    // Tests for god-object call-graph context lookup.
    //
    // For a GodClass debt item, `item.location.function` is the struct's name
    // and `item.location.line` is the struct declaration line. There is no
    // FunctionId in the call graph for that pair, so a single-FunctionId
    // lookup returns no callers / callees and the suggestion ends up with
    // very low completeness. The fix aggregates callers/callees from every
    // method of the struct (GodClass) or every function in the file
    // (GodFile/GodModule), excluding internal cross-method edges.
    // =========================================================================

    fn make_god_class_item(file: &str, struct_name: &str, struct_line: usize) -> UnifiedDebtItem {
        let mut item = create_test_debt_item(file, struct_name, struct_line, 0);
        item.god_object_indicators = Some(GodObjectAnalysis {
            is_god_object: true,
            method_count: 0,
            weighted_method_count: None,
            field_count: 0,
            responsibility_count: 0,
            lines_of_code: 0,
            complexity_sum: 0,
            god_object_score: 0.0,
            recommended_splits: Vec::new(),
            confidence: GodObjectConfidence::Probable,
            responsibilities: Vec::new(),
            responsibility_method_counts: std::collections::HashMap::new(),
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            struct_name: Some(struct_name.to_string()),
            struct_line: Some(struct_line),
            struct_location: Some(SourceLocation {
                line: struct_line,
                column: None,
                end_line: Some(struct_line + 100),
                end_column: None,
                confidence: LocationConfidence::Approximate,
            }),
            visibility_breakdown: None,
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,
            trait_method_summary: None,
        });
        item.debt_type = DebtType::GodObject {
            methods: 0,
            fields: Some(0),
            responsibilities: 0,
            god_object_score: 89.0,
            lines: 1750,
        };
        item.file_line_count = Some(struct_line + 200);
        item
    }

    fn make_god_file_item(file: &str) -> UnifiedDebtItem {
        let mut item = make_god_class_item(file, "[file-scope]", 1);
        if let Some(ref mut god) = item.god_object_indicators {
            god.detection_type = DetectionType::GodFile;
            god.struct_name = None;
            god.struct_line = None;
            god.struct_location = None;
        }
        item
    }

    fn collect_caller_symbols(suggestion: &super::super::ContextSuggestion) -> Vec<String> {
        suggestion
            .related
            .iter()
            .filter(|r| r.relationship == ContextRelationship::Caller)
            .filter_map(|r| r.range.symbol.clone())
            .collect()
    }

    fn collect_callee_symbols(suggestion: &super::super::ContextSuggestion) -> Vec<String> {
        suggestion
            .related
            .iter()
            .filter(|r| r.relationship == ContextRelationship::Callee)
            .filter_map(|r| r.range.symbol.clone())
            .collect()
    }

    #[test]
    fn god_class_context_aggregates_callers_from_struct_methods() {
        // Set up a call graph in which the struct itself has no FunctionId
        // (only its methods do), and an external caller invokes one of those
        // methods. The previous single-FunctionId lookup at the struct
        // declaration line found nothing.
        let foo_file = PathBuf::from("src/foo.rs");
        let caller_file = PathBuf::from("src/caller.rs");

        let builder_a = FunctionId::new(foo_file.clone(), "Builder::a".to_string(), 10);
        let builder_b = FunctionId::new(foo_file.clone(), "Builder::b".to_string(), 20);
        let ext_caller = FunctionId::new(caller_file.clone(), "ext_caller".to_string(), 100);
        let ext_callee = FunctionId::new(caller_file.clone(), "ext_callee".to_string(), 200);

        let mut graph = CallGraph::new();
        graph.add_function(builder_a.clone(), false, false, 5, 10);
        graph.add_function(builder_b.clone(), false, false, 5, 10);
        graph.add_function(ext_caller.clone(), false, false, 5, 10);
        graph.add_function(ext_callee.clone(), false, false, 5, 10);

        // External caller invokes Builder::a
        graph.add_call_parts(ext_caller.clone(), builder_a.clone(), CallType::Direct);
        // Internal cross-method call (must be filtered out as caller)
        graph.add_call_parts(builder_a.clone(), builder_b.clone(), CallType::Direct);
        // Builder::b calls ext_callee (external callee)
        graph.add_call_parts(builder_b.clone(), ext_callee.clone(), CallType::Direct);

        let item = make_god_class_item("src/foo.rs", "Builder", 5);
        let suggestion = generate_context_suggestion(&item, &graph, &ContextConfig::default())
            .expect("god-class context suggestion should be generated");

        let callers = collect_caller_symbols(&suggestion);
        assert!(
            callers.iter().any(|c| c == "ext_caller"),
            "GodClass callers should include external callers of struct methods, got: {callers:?}"
        );
        assert!(
            !callers.iter().any(|c| c.starts_with("Builder::")),
            "Internal cross-method calls must not appear as callers, got: {callers:?}"
        );

        let callees = collect_callee_symbols(&suggestion);
        assert!(
            callees.iter().any(|c| c == "ext_callee"),
            "GodClass callees should include external callees of struct methods, got: {callees:?}"
        );
        assert!(
            !callees.iter().any(|c| c.starts_with("Builder::")),
            "Internal cross-method calls must not appear as callees, got: {callees:?}"
        );
    }

    #[test]
    fn god_class_context_completeness_reflects_aggregated_lookups() {
        // Without the fix, has_callers/has_callees are false (because the
        // struct-line FunctionId has no edges) and confidence stays low.
        // With the fix, both should be true and confidence should rise.
        let foo_file = PathBuf::from("src/foo.rs");

        let builder_a = FunctionId::new(foo_file.clone(), "Builder::a".to_string(), 10);
        let ext_caller = FunctionId::new(PathBuf::from("src/other.rs"), "external".to_string(), 50);
        let ext_callee = FunctionId::new(PathBuf::from("src/other.rs"), "callee".to_string(), 60);

        let mut graph = CallGraph::new();
        graph.add_function(builder_a.clone(), false, false, 5, 10);
        graph.add_function(ext_caller.clone(), false, false, 5, 10);
        graph.add_function(ext_callee.clone(), false, false, 5, 10);
        graph.add_call_parts(ext_caller.clone(), builder_a.clone(), CallType::Direct);
        graph.add_call_parts(builder_a.clone(), ext_callee.clone(), CallType::Direct);

        let item = make_god_class_item("src/foo.rs", "Builder", 5);
        let suggestion = generate_context_suggestion(&item, &graph, &ContextConfig::default())
            .expect("suggestion expected");

        // 0.5 base + 0.1 callers + 0.1 callees + 0.1 types + 0.1 tests = 0.9
        assert!(
            suggestion.completeness_confidence >= 0.85,
            "GodClass completeness should reflect aggregated callers/callees, got {}",
            suggestion.completeness_confidence
        );
    }

    #[test]
    fn god_file_context_aggregates_callers_from_all_file_functions() {
        // GodFile detection has no struct_name; aggregation must fall back to
        // every function in the file.
        let foo_file = PathBuf::from("src/big.rs");

        let free_a = FunctionId::new(foo_file.clone(), "free_a".to_string(), 10);
        let free_b = FunctionId::new(foo_file.clone(), "free_b".to_string(), 20);
        let ext_caller = FunctionId::new(PathBuf::from("src/elsewhere.rs"), "ext".to_string(), 30);

        let mut graph = CallGraph::new();
        graph.add_function(free_a.clone(), false, false, 5, 10);
        graph.add_function(free_b.clone(), false, false, 5, 10);
        graph.add_function(ext_caller.clone(), false, false, 5, 10);
        graph.add_call_parts(ext_caller.clone(), free_a.clone(), CallType::Direct);
        // Internal call: free_a calls free_b. Must not surface as a caller of
        // the GodFile scope (free_a is itself in the scope).
        graph.add_call_parts(free_a.clone(), free_b.clone(), CallType::Direct);

        let item = make_god_file_item("src/big.rs");
        let suggestion = generate_context_suggestion(&item, &graph, &ContextConfig::default())
            .expect("god-file suggestion expected");

        let callers = collect_caller_symbols(&suggestion);
        assert!(
            callers.iter().any(|c| c == "ext"),
            "GodFile should aggregate external callers, got: {callers:?}"
        );
        assert!(
            !callers.iter().any(|c| c == "free_a"),
            "Internal callers within the same file must be excluded, got: {callers:?}"
        );
    }

    #[test]
    fn function_level_context_lookup_unchanged() {
        // Regression guard: function-level items must still resolve via the
        // single-FunctionId lookup (no aggregation, no internal-edge filter).
        let file = PathBuf::from("src/lib.rs");

        let target = FunctionId::new(file.clone(), "complex_function".to_string(), 100);
        let caller = FunctionId::new(PathBuf::from("src/main.rs"), "entry".to_string(), 5);

        let mut graph = CallGraph::new();
        graph.add_function(target.clone(), false, false, 5, 10);
        graph.add_function(caller.clone(), false, false, 5, 10);
        graph.add_call_parts(caller.clone(), target.clone(), CallType::Direct);

        let item = create_test_debt_item("src/lib.rs", "complex_function", 100, 50);
        let suggestion = generate_context_suggestion(&item, &graph, &ContextConfig::default())
            .expect("function suggestion expected");

        let callers = collect_caller_symbols(&suggestion);
        assert!(
            callers.iter().any(|c| c == "entry"),
            "Function-level caller resolution must still work, got: {callers:?}"
        );
    }
}

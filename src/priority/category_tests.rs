#[cfg(test)]
mod tests {
    use crate::priority::{
        ActionableRecommendation, CallGraph, DebtCategory, DebtType, FileDebtItem, FileDebtMetrics,
        FileImpact, FunctionRole, FunctionVisibility, GodObjectIndicators, ImpactMetrics, Location,
        UnifiedAnalysis, UnifiedAnalysisQueries, UnifiedAnalysisUtils, UnifiedDebtItem,
        UnifiedScore,
    };
    use std::path::PathBuf;

    fn create_test_function_item(name: &str, debt_type: DebtType, score: f64) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                line: 10,
                function: name.to_string(),
            },
            debt_type,
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 8.0, // Set high enough to avoid filtering
                dependency_factor: 5.0,
                role_multiplier: 1.0,
                final_score: score,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: FunctionRole::Unknown,
            recommendation: ActionableRecommendation {
                primary_action: "Fix issue".to_string(),
                rationale: "Test reason".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 10.0,
                risk_reduction: 1.0,
                coverage_improvement: 10.0,
                lines_reduction: 50,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 1,
            downstream_dependencies: 2,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 2,
            function_length: 100,
            cyclomatic_complexity: 10,
            cognitive_complexity: 15,
            entropy_details: None,
            is_pure: Some(false),
            purity_confidence: Some(0.8),
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
        }
    }

    fn create_test_file_item(path: &str, is_god_object: bool, score: f64) -> FileDebtItem {
        FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from(path),
                total_lines: 500,
                function_count: 20,
                class_count: 2,
                avg_complexity: if is_god_object { 20.0 } else { 5.0 },
                max_complexity: if is_god_object { 50 } else { 10 },
                total_complexity: if is_god_object { 400 } else { 100 },
                coverage_percent: 0.5,
                uncovered_lines: 250,
                god_object_indicators: GodObjectIndicators {
                    methods_count: if is_god_object { 40 } else { 10 },
                    fields_count: if is_god_object { 20 } else { 5 },
                    responsibilities: if is_god_object { 8 } else { 2 },
                    is_god_object,
                    god_object_score: if is_god_object { 3.5 } else { 0.0 },
                    ..Default::default()
                },
                function_scores: vec![],
                god_object_type: None,
                file_type: None,
            },
            score,
            priority_rank: 1,
            recommendation: "Refactor".to_string(),
            impact: FileImpact {
                complexity_reduction: 20.0,
                maintainability_improvement: 30.0,
                test_effort: 15.0,
            },
        }
    }

    #[test]
    fn test_debt_category_classification() {
        // Architecture Issues
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::GodObject {
                methods: 10,
                fields: 5,
                responsibilities: 10,
                god_object_score: 90.0,
            }),
            DebtCategory::Architecture
        );
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::FeatureEnvy {
                external_class: "OtherClass".to_string(),
                usage_ratio: 0.8,
            }),
            DebtCategory::Architecture
        );
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::PrimitiveObsession {
                primitive_type: "String".to_string(),
                domain_concept: "UserId".to_string(),
            }),
            DebtCategory::Architecture
        );

        // Testing Gaps
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 15,
            }),
            DebtCategory::Testing
        );
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::TestComplexityHotspot {
                cyclomatic: 20,
                cognitive: 30,
                threshold: 10,
            }),
            DebtCategory::Testing
        );
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::FlakyTestPattern {
                pattern_type: "timing".to_string(),
                reliability_impact: "high".to_string(),
            }),
            DebtCategory::Testing
        );

        // Performance Issues
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::AsyncMisuse {
                pattern: "blocking".to_string(),
                performance_impact: "high".to_string(),
            }),
            DebtCategory::Performance
        );
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::NestedLoops {
                depth: 3,
                complexity_estimate: "O(n^3)".to_string(),
            }),
            DebtCategory::Performance
        );
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::BlockingIO {
                operation: "file_read".to_string(),
                context: "async_context".to_string(),
            }),
            DebtCategory::Performance
        );

        // Code Quality
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::ComplexityHotspot {
                cyclomatic: 25,
                cognitive: 40,
                adjusted_cyclomatic: None,
            }),
            DebtCategory::CodeQuality
        );
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::DeadCode {
                visibility: FunctionVisibility::Private,
                cyclomatic: 5,
                cognitive: 8,
                usage_hints: vec![],
            }),
            DebtCategory::CodeQuality
        );
        assert_eq!(
            DebtCategory::from_debt_type(&DebtType::MagicValues {
                value: "42".to_string(),
                occurrences: 10,
            }),
            DebtCategory::CodeQuality
        );
    }

    #[test]
    fn test_category_names_and_icons() {
        assert_eq!(DebtCategory::Architecture.name(), "Architecture Issues");
        assert_eq!(DebtCategory::Architecture.icon(), "[ARCH]");

        assert_eq!(DebtCategory::Testing.name(), "Testing Gaps");
        assert_eq!(DebtCategory::Testing.icon(), "[TEST]");

        assert_eq!(DebtCategory::Performance.name(), "Performance Issues");
        assert_eq!(DebtCategory::Performance.icon(), "[PERF]");

        assert_eq!(DebtCategory::CodeQuality.name(), "Code Quality");
        assert_eq!(DebtCategory::CodeQuality.icon(), "");
    }

    #[test]
    fn test_category_strategic_guidance() {
        let arch_guidance = DebtCategory::Architecture.strategic_guidance(5, 40);
        assert!(arch_guidance.contains("breaking down 5 complex components"));
        assert!(arch_guidance.contains("40 hours"));

        let test_guidance = DebtCategory::Testing.strategic_guidance(10, 20);
        assert!(test_guidance.contains("10 missing tests"));
        assert!(test_guidance.contains("20 hours"));

        let perf_guidance = DebtCategory::Performance.strategic_guidance(3, 12);
        assert!(perf_guidance.contains("3 performance bottlenecks"));
        assert!(perf_guidance.contains("12 hours"));

        let quality_guidance = DebtCategory::CodeQuality.strategic_guidance(8, 16);
        assert!(quality_guidance.contains("8 code quality issues"));
        assert!(quality_guidance.contains("16 hours"));
    }

    #[test]
    fn test_unified_analysis_categorization() {
        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Add various debt items
        analysis.add_item(create_test_function_item(
            "god_func",
            DebtType::GodObject {
                methods: 10,
                fields: 5,
                responsibilities: 10,
                god_object_score: 95.0,
            },
            95.0,
        ));

        analysis.add_item(create_test_function_item(
            "untested_func",
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 15,
                cognitive: 20,
            },
            75.0,
        ));

        analysis.add_item(create_test_function_item(
            "slow_func",
            DebtType::AsyncMisuse {
                pattern: "blocking".to_string(),
                performance_impact: "high".to_string(),
            },
            80.0,
        ));

        analysis.add_item(create_test_function_item(
            "complex_func",
            DebtType::ComplexityHotspot {
                cyclomatic: 25,
                cognitive: 35,
                adjusted_cyclomatic: None,
            },
            70.0,
        ));

        // Add file items
        analysis.add_file_item(create_test_file_item("src/god_class.rs", true, 120.0));
        analysis.add_file_item(create_test_file_item("src/normal.rs", false, 45.0));

        analysis.sort_by_priority();

        // Get categorized debt
        let categorized = analysis.get_categorized_debt(10);

        // Verify categories exist
        assert!(categorized
            .categories
            .contains_key(&DebtCategory::Architecture));
        assert!(categorized.categories.contains_key(&DebtCategory::Testing));
        assert!(categorized
            .categories
            .contains_key(&DebtCategory::Performance));
        assert!(categorized
            .categories
            .contains_key(&DebtCategory::CodeQuality));

        // Verify item counts
        let arch_summary = &categorized.categories[&DebtCategory::Architecture];
        assert_eq!(arch_summary.item_count, 2); // god_func + god_class.rs

        let test_summary = &categorized.categories[&DebtCategory::Testing];
        assert_eq!(test_summary.item_count, 1); // untested_func

        let perf_summary = &categorized.categories[&DebtCategory::Performance];
        assert_eq!(perf_summary.item_count, 1); // slow_func

        let quality_summary = &categorized.categories[&DebtCategory::CodeQuality];
        assert_eq!(quality_summary.item_count, 2); // complex_func + normal.rs
    }

    #[test]
    fn test_categorized_debt_summary_calculations() {
        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Add multiple testing gap items with high enough scores to avoid filtering
        for i in 0..3 {
            let mut item = create_test_function_item(
                &format!("untested_{}", i),
                DebtType::TestingGap {
                    coverage: 0.0,
                    cyclomatic: 10,
                    cognitive: 15,
                },
                60.0 + i as f64 * 5.0, // 60, 65, 70
            );
            // Ensure items have high enough complexity to not be filtered
            item.cyclomatic_complexity = 10;
            item.cognitive_complexity = 15;
            // Ensure each item has a unique location to avoid duplicate filtering
            item.location.line = 10 + i * 10;
            analysis.add_item(item);
        }

        // Verify items were added
        assert!(
            !analysis.items.is_empty(),
            "Items should have been added to analysis"
        );

        let categorized = analysis.get_categorized_debt(10);

        // Check if Testing category exists
        assert!(
            categorized.categories.contains_key(&DebtCategory::Testing),
            "Testing category should exist"
        );

        let test_summary = &categorized.categories[&DebtCategory::Testing];

        // Verify calculations
        assert_eq!(test_summary.item_count, 3);
        assert_eq!(test_summary.total_score, 60.0 + 65.0 + 70.0);
        assert_eq!(test_summary.average_severity, (60.0 + 65.0 + 70.0) / 3.0);

        // Top items should be sorted by score
        assert!(test_summary.top_items.len() <= 3);
    }

    #[test]
    fn test_cross_category_dependencies_detection() {
        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Add a god object (architecture issue)
        analysis.add_item(create_test_function_item(
            "god_func",
            DebtType::GodObject {
                methods: 10,
                fields: 5,
                responsibilities: 10,
                god_object_score: 95.0,
            },
            95.0,
        ));

        // Add testing gaps
        analysis.add_item(create_test_function_item(
            "untested_func",
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 15,
                cognitive: 20,
            },
            75.0,
        ));

        let categorized = analysis.get_categorized_debt(10);

        // Should detect architecture -> testing dependency
        assert!(!categorized.cross_category_dependencies.is_empty());

        let dep = &categorized.cross_category_dependencies[0];
        assert_eq!(dep.source_category, DebtCategory::Architecture);
        assert_eq!(dep.target_category, DebtCategory::Testing);
        assert!(dep.description.contains("God objects"));
    }

    #[test]
    fn test_effort_estimation() {
        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Add high severity architecture issue
        analysis.add_item(create_test_function_item(
            "critical_god",
            DebtType::GodObject {
                methods: 15,
                fields: 7,
                responsibilities: 15,
                god_object_score: 95.0,
            },
            95.0,
        ));

        // Add moderate testing issue
        analysis.add_item(create_test_function_item(
            "untested",
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 8,
                cognitive: 10,
            },
            55.0,
        ));

        let categorized = analysis.get_categorized_debt(10);

        // Architecture with high severity should have more effort
        let arch_summary = &categorized.categories[&DebtCategory::Architecture];
        assert!(arch_summary.estimated_effort_hours >= 16); // Critical = 16 hours per item

        // Testing with moderate severity should have less effort
        let test_summary = &categorized.categories[&DebtCategory::Testing];
        assert!(test_summary.estimated_effort_hours <= 4); // Moderate = 2-4 hours per item
    }

    #[test]
    fn test_category_ordering() {
        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Add items with different total scores per category
        analysis.add_item(create_test_function_item(
            "test1",
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 15,
            },
            50.0,
        ));

        analysis.add_item(create_test_function_item(
            "perf1",
            DebtType::AsyncMisuse {
                pattern: "blocking".to_string(),
                performance_impact: "high".to_string(),
            },
            80.0,
        ));

        analysis.add_item(create_test_function_item(
            "arch1",
            DebtType::GodObject {
                methods: 10,
                fields: 5,
                responsibilities: 10,
                god_object_score: 100.0,
            },
            100.0,
        ));

        let categorized = analysis.get_categorized_debt(10);

        // Architecture should come first (highest score: 100)
        let arch_score = categorized.categories[&DebtCategory::Architecture].total_score;
        let perf_score = categorized.categories[&DebtCategory::Performance].total_score;
        let test_score = categorized.categories[&DebtCategory::Testing].total_score;

        assert!(arch_score > perf_score);
        assert!(perf_score > test_score);
    }

    #[test]
    fn test_empty_categories_omitted() {
        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Only add architecture issues
        analysis.add_item(create_test_function_item(
            "god",
            DebtType::GodObject {
                methods: 10,
                fields: 5,
                responsibilities: 10,
                god_object_score: 90.0,
            },
            90.0,
        ));

        let categorized = analysis.get_categorized_debt(10);

        // Only Architecture category should exist
        assert_eq!(categorized.categories.len(), 1);
        assert!(categorized
            .categories
            .contains_key(&DebtCategory::Architecture));
        assert!(!categorized.categories.contains_key(&DebtCategory::Testing));
        assert!(!categorized
            .categories
            .contains_key(&DebtCategory::Performance));
    }

    #[test]
    fn test_file_categorization() {
        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // God object file -> Architecture
        let god_file = create_test_file_item("src/god.rs", true, 100.0);
        analysis.add_file_item(god_file);

        // Low coverage file -> Testing
        let mut low_coverage_file = create_test_file_item("src/untested.rs", false, 60.0);
        low_coverage_file.metrics.coverage_percent = 0.3;
        analysis.add_file_item(low_coverage_file);

        // High complexity file -> Code Quality
        let mut complex_file = create_test_file_item("src/complex.rs", false, 70.0);
        complex_file.metrics.avg_complexity = 20.0;
        analysis.add_file_item(complex_file);

        let categorized = analysis.get_categorized_debt(10);

        assert!(categorized
            .categories
            .contains_key(&DebtCategory::Architecture));
        assert!(categorized.categories.contains_key(&DebtCategory::Testing));
        assert!(categorized
            .categories
            .contains_key(&DebtCategory::CodeQuality));
    }
}

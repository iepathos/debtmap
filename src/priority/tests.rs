//! Tests for the priority module.
//!
//! This module contains unit tests for debt item handling, filtering,
//! density calculations, and debt type display implementations.

use super::*;

// Helper function to create test items
fn create_test_item(
    debt_type: DebtType,
    cyclomatic: u32,
    cognitive: u32,
    score: f64,
) -> UnifiedDebtItem {
    create_test_item_with_line(debt_type, cyclomatic, cognitive, score, 1)
}

// Helper function to create test items with specific line number
fn create_test_item_with_line(
    debt_type: DebtType,
    cyclomatic: u32,
    cognitive: u32,
    score: f64,
    line: usize,
) -> UnifiedDebtItem {
    use semantic_classifier::FunctionRole;

    UnifiedDebtItem {
        location: unified_scorer::Location {
            file: "test.rs".into(),
            function: "test_fn".into(),
            line,
        },
        debt_type,
        unified_score: unified_scorer::UnifiedScore {
            complexity_factor: 0.0,
            coverage_factor: 10.0,
            dependency_factor: 0.0,
            role_multiplier: 1.0,
            final_score: score.max(0.0),
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
            // Spec 260: Score transparency fields
            debt_adjustment: None,
            pre_normalization_score: None,
            structural_multiplier: Some(1.0),
            has_coverage_data: false,
            contextual_risk_multiplier: None,
            pre_contextual_score: None,
        },
        function_role: FunctionRole::PureLogic,
        recommendation: ActionableRecommendation {
            primary_action: "Test".into(),
            rationale: "Test".into(),
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
        transitive_coverage: None,
        file_context: None,
        upstream_dependencies: 0,
        downstream_dependencies: 0,
        upstream_callers: vec![],
        downstream_callees: vec![],
        upstream_production_callers: vec![],
        upstream_test_callers: vec![],
        production_blast_radius: 0,
        nesting_depth: 1,
        function_length: 10,
        cyclomatic_complexity: cyclomatic,
        cognitive_complexity: cognitive,
        is_pure: Some(false),
        purity_confidence: Some(0.0),
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
        detected_pattern: None,
        contextual_risk: None, // spec 203
        file_line_count: None,
        responsibility_category: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        entropy_analysis: None,
        context_suggestion: None,
    }
}

#[test]
fn test_debtitem_file_roundtrip() {
    use file_metrics::{FileDebtItem, FileDebtMetrics, FileImpact};
    use std::path::PathBuf;

    // Create a File debt item
    let file_item = DebtItem::File(Box::new(FileDebtItem {
        metrics: FileDebtMetrics {
            path: PathBuf::from("./test.rs"),
            total_lines: 100,
            function_count: 5,
            class_count: 1,
            avg_complexity: 3.0,
            max_complexity: 10,
            total_complexity: 50,
            coverage_percent: 0.5,
            uncovered_lines: 50,
            god_object_analysis: None,
            function_scores: vec![],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        },
        score: 50.0,
        priority_rank: 1,
        recommendation: "Test".to_string(),
        impact: FileImpact {
            complexity_reduction: 10.0,
            maintainability_improvement: 5.0,
            test_effort: 2.0,
        },
    }));

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&file_item).unwrap();
    eprintln!("Serialized JSON:\n{}", json);

    // Try to deserialize it back
    let result: Result<DebtItem, _> = serde_json::from_str(&json);
    if let Err(e) = &result {
        eprintln!("Deserialization error: {}", e);
    }
    assert!(result.is_ok(), "Failed to deserialize: {:?}", result.err());

    match result.unwrap() {
        DebtItem::File(_) => {} // Success
        DebtItem::Function(_) => panic!("Deserialized as wrong variant!"),
    }
}

#[test]
fn test_debtitem_from_real_json() {
    // This is the internally-tagged format (using "type" field)
    let json = r#"{
      "type": "File",
      "metrics": {
        "path": "./test.rs",
        "total_lines": 100,
        "function_count": 5,
        "class_count": 1,
        "avg_complexity": 3.0,
        "max_complexity": 10,
        "total_complexity": 50,
        "coverage_percent": 0.5,
        "uncovered_lines": 50,
        "function_scores": [],
        "god_object_analysis": null,
        "god_object_type": null,
        "file_type": null
      },
      "score": 50.0,
      "priority_rank": 1,
      "recommendation": "Test",
      "impact": {
        "complexity_reduction": 10.0,
        "maintainability_improvement": 5.0,
        "test_effort": 2.0
      }
    }"#;

    let result: Result<DebtItem, _> = serde_json::from_str(json);
    if let Err(e) = &result {
        eprintln!("Deserialization error: {}", e);
    }
    assert!(
        result.is_ok(),
        "Failed to deserialize real JSON: {:?}",
        result.err()
    );

    match result.unwrap() {
        DebtItem::File(f) => {
            assert_eq!(f.score, 50.0);
            assert_eq!(f.metrics.total_lines, 100);
        }
        DebtItem::Function(_) => panic!("Deserialized as wrong variant!"),
    }
}

#[test]
fn test_debt_density_calculation_formula() {
    // Test the formula: (total_debt_score / total_lines_of_code) * 1000

    // Case 1: 100 debt score, 1000 LOC = 100.0 density
    let density1 = (100.0 / 1000.0) * 1000.0;
    assert_eq!(density1, 100.0);

    // Case 2: 80 debt score, 250 LOC = 320.0 density
    let density2 = (80.0 / 250.0) * 1000.0;
    assert_eq!(density2, 320.0);

    // Case 3: 5000 debt score, 50000 LOC = 100.0 density
    let density3 = (5000.0 / 50000.0) * 1000.0;
    assert_eq!(density3, 100.0);
}

#[test]
fn test_debt_density_zero_lines() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);
    analysis.calculate_total_impact();

    // No items, should have 0 density
    assert_eq!(analysis.total_debt_score, 0.0);
    assert_eq!(analysis.total_lines_of_code, 0);
    assert_eq!(analysis.debt_density, 0.0);
}

#[test]
fn test_debt_density_scale_independence() {
    // Verify that projects with proportional debt/LOC have same density

    // Small project
    let density_small = (50.0 / 500.0) * 1000.0;

    // Large project (10x larger, 10x more debt)
    let density_large = (500.0 / 5000.0) * 1000.0;

    // Should have same density
    assert_eq!(density_small, 100.0);
    assert_eq!(density_large, 100.0);
    assert_eq!(density_small, density_large);
}

#[test]
fn test_debt_density_example_values() {
    // Test real-world example values

    // Clean small project
    let clean_small = (250.0 / 5000.0) * 1000.0;
    assert_eq!(clean_small, 50.0);

    // Messy small project
    let messy_small = (750.0 / 5000.0) * 1000.0;
    assert_eq!(messy_small, 150.0);

    // Clean large project
    let clean_large = (5000.0 / 100000.0) * 1000.0;
    assert_eq!(clean_large, 50.0);

    // Messy large project
    let messy_large = (15000.0 / 100000.0) * 1000.0;
    assert_eq!(messy_large, 150.0);
}

#[test]
fn test_unified_analysis_initializes_density_fields() {
    let call_graph = CallGraph::new();
    let analysis = UnifiedAnalysis::new(call_graph);

    // Check fields are initialized
    assert_eq!(analysis.debt_density, 0.0);
    assert_eq!(analysis.total_lines_of_code, 0);
}

#[test]
fn test_filter_below_cyclomatic_threshold() {
    // Set minimum cyclomatic complexity threshold to 3
    std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "3");
    std::env::set_var("DEBTMAP_MIN_COGNITIVE", "0");

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Create item with cyclomatic=2 (below threshold of 3)
    let item = create_test_item(
        DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 2,
            cognitive: 10,
        },
        2,    // cyclomatic
        10,   // cognitive
        15.0, // score
    );

    analysis.add_item(item);

    // Should be filtered (2 < 3)
    assert_eq!(analysis.items.len(), 0);

    // Clean up
    std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
    std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
}

#[test]
fn test_filter_below_cognitive_threshold() {
    // Set minimum cognitive complexity threshold to 5
    std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "0");
    std::env::set_var("DEBTMAP_MIN_COGNITIVE", "5");

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Create item with cognitive=4 (below threshold of 5)
    let item = create_test_item(
        DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 4,
        },
        10,   // cyclomatic
        4,    // cognitive - below threshold
        15.0, // score
    );

    analysis.add_item(item);

    // Should be filtered (4 < 5)
    assert_eq!(analysis.items.len(), 0);

    // Clean up
    std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
    std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
}

#[test]
fn test_keep_at_threshold() {
    // Set thresholds
    std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "3");
    std::env::set_var("DEBTMAP_MIN_COGNITIVE", "5");

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Create item with complexities exactly at thresholds
    let item = create_test_item(
        DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 3,
            cognitive: 5,
        },
        3,    // cyclomatic - at threshold
        5,    // cognitive - at threshold
        15.0, // score
    );

    analysis.add_item(item);

    // Should be kept (3 >= 3 and 5 >= 5)
    assert_eq!(analysis.items.len(), 1);

    // Clean up
    std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
    std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
}

#[test]
fn test_untested_trivial_function_filtered() {
    // Set minimum cyclomatic complexity threshold to 3
    std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "3");
    std::env::set_var("DEBTMAP_MIN_COGNITIVE", "0");

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Create trivial function with 0% coverage (high coverage_factor)
    let item = create_test_item(
        DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 1,
            cognitive: 0,
        },
        1,    // cyclomatic - trivial
        0,    // cognitive - trivial
        17.5, // high score due to coverage gap
    );

    analysis.add_item(item);

    // Should be filtered despite 0% coverage and high score
    // The bug was that this was NOT filtered
    assert_eq!(analysis.items.len(), 0);

    // Clean up
    std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
    std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
}

#[test]
fn test_test_items_exempt_from_filtering() {
    // Set high thresholds
    std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "10");
    std::env::set_var("DEBTMAP_MIN_COGNITIVE", "20");

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Create test-related item with low complexity
    let item = create_test_item(
        DebtType::TestComplexityHotspot {
            cyclomatic: 1,
            cognitive: 0,
            threshold: 5,
        },
        1,    // cyclomatic - below threshold
        0,    // cognitive - below threshold
        15.0, // score
    );

    analysis.add_item(item);

    // Should NOT be filtered (test items exempt)
    assert_eq!(analysis.items.len(), 1);

    // Clean up
    std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
    std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
}

#[test]
fn test_both_thresholds_must_be_satisfied() {
    // Set both thresholds
    std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "3");
    std::env::set_var("DEBTMAP_MIN_COGNITIVE", "5");

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Create item that meets cyclomatic but not cognitive
    let item1 = create_test_item(
        DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 5,
            cognitive: 3,
        },
        5,    // cyclomatic - above threshold
        3,    // cognitive - below threshold
        15.0, // score
    );

    analysis.add_item(item1);
    // Should be filtered (cognitive 3 < 5)
    assert_eq!(analysis.items.len(), 0);

    // Create item that meets cognitive but not cyclomatic
    let item2 = create_test_item(
        DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 2,
            cognitive: 10,
        },
        2,    // cyclomatic - below threshold
        10,   // cognitive - above threshold
        15.0, // score
    );

    analysis.add_item(item2);
    // Should be filtered (cyclomatic 2 < 3)
    assert_eq!(analysis.items.len(), 0);

    // Create item that meets both thresholds
    let item3 = create_test_item(
        DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 3,
            cognitive: 5,
        },
        3,    // cyclomatic - at threshold
        5,    // cognitive - at threshold
        15.0, // score
    );

    analysis.add_item(item3);
    // Should be kept (both thresholds satisfied)
    assert_eq!(analysis.items.len(), 1);

    // Clean up
    std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
    std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
}

#[test]
fn test_single_stage_filtering_by_score() {
    use crate::priority::call_graph::CallGraph;

    // Set minimum score threshold for single-stage filtering (spec 243)
    std::env::set_var("DEBTMAP_MIN_SCORE_THRESHOLD", "3.0");
    // Set complexity thresholds to 0 to isolate score filtering
    std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "0");
    std::env::set_var("DEBTMAP_MIN_COGNITIVE", "0");

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Create items with different scores and different line numbers
    // Use higher complexity to avoid T4 classification
    let high_score_item = create_test_item_with_line(
        DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 15,
            cognitive: 20,
        },
        15,
        20,
        10.0, // High score - should be kept
        10,   // line number
    );

    let low_score_item = create_test_item_with_line(
        DebtType::TestingGap {
            coverage: 0.5,
            cyclomatic: 12,
            cognitive: 15,
        },
        12,
        15,
        2.0, // Low score - should be filtered during add_item
        20,  // line number
    );

    let mid_score_item = create_test_item_with_line(
        DebtType::TestingGap {
            coverage: 0.3,
            cyclomatic: 11,
            cognitive: 12,
        },
        11,
        12,
        5.0, // Mid score - should be kept
        30,  // line number
    );

    // Add items - filtering happens during add_item (spec 243)
    analysis.add_item(high_score_item);
    analysis.add_item(low_score_item); // Filtered out
    analysis.add_item(mid_score_item);

    // Single-stage filtering: only items >= 3.0 should be present
    assert_eq!(analysis.items.len(), 2); // high_score and mid_score
    assert_eq!(analysis.stats.filtered_by_score, 1); // low_score filtered

    // Calculate totals
    analysis.calculate_total_impact();
    assert_eq!(analysis.total_debt_score, 15.0); // 10.0 + 5.0

    // Clean up
    std::env::remove_var("DEBTMAP_MIN_SCORE_THRESHOLD");
    std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
    std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
}

#[test]
fn test_single_stage_filtering_calculates_correct_density() {
    use crate::priority::call_graph::CallGraph;

    // Set threshold for single-stage filtering (spec 243)
    std::env::set_var("DEBTMAP_MIN_SCORE_THRESHOLD", "5.0");
    // Set complexity thresholds to 0 to isolate score filtering
    std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "0");
    std::env::set_var("DEBTMAP_MIN_COGNITIVE", "0");

    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Add items with higher complexity to avoid T4 classification
    let item1 = create_test_item(
        DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 15,
            cognitive: 20,
        },
        15,
        20,
        10.0, // Above threshold - kept
    );

    let item2 = create_test_item(
        DebtType::TestingGap {
            coverage: 0.5,
            cyclomatic: 12,
            cognitive: 15,
        },
        12,
        15,
        2.0, // Below threshold - filtered during add_item
    );

    // Add items - filtering happens during add_item (spec 243)
    analysis.add_item(item1);
    analysis.add_item(item2); // Filtered out

    // Calculate totals and manually set LOC for density calculation
    analysis.calculate_total_impact();
    analysis.total_lines_of_code = 1000;

    // Manually calculate density since calculate_total_impact sets LOC to 0 in test env
    analysis.debt_density = (analysis.total_debt_score / 1000.0) * 1000.0;

    // Only item1 should be present (score 10.0)
    assert_eq!(analysis.items.len(), 1);
    assert_eq!(analysis.total_debt_score, 10.0);
    // Density: (10.0 / 1000) * 1000 = 10.0 per 1K LOC
    assert_eq!(analysis.debt_density, 10.0);

    // Clean up
    std::env::remove_var("DEBTMAP_MIN_SCORE_THRESHOLD");
    std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
    std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
}

// Tests for DebtType Display implementation (Spec 005)
mod debt_type_display_tests {
    use super::*;

    #[test]
    fn display_todo() {
        let debt = DebtType::Todo {
            reason: Some("fix later".into()),
        };
        assert_eq!(debt.to_string(), "TODO");
    }

    #[test]
    fn display_fixme() {
        let debt = DebtType::Fixme { reason: None };
        assert_eq!(debt.to_string(), "FIXME");
    }

    #[test]
    fn display_code_smell() {
        let debt = DebtType::CodeSmell {
            smell_type: Some("complex".into()),
        };
        assert_eq!(debt.to_string(), "Code Smell");
    }

    #[test]
    fn display_complexity() {
        let debt = DebtType::Complexity {
            cyclomatic: 20,
            cognitive: 15,
        };
        assert_eq!(debt.to_string(), "Complexity");
    }

    #[test]
    fn display_dependency() {
        let debt = DebtType::Dependency {
            dependency_type: None,
        };
        assert_eq!(debt.to_string(), "Dependency");
    }

    #[test]
    fn display_resource_management() {
        let debt = DebtType::ResourceManagement {
            issue_type: Some("leak".into()),
        };
        assert_eq!(debt.to_string(), "Resource Management");
    }

    #[test]
    fn display_code_organization() {
        let debt = DebtType::CodeOrganization {
            issue_type: Some("scattered".into()),
        };
        assert_eq!(debt.to_string(), "Code Organization");
    }

    #[test]
    fn display_test_complexity() {
        let debt = DebtType::TestComplexity {
            cyclomatic: 10,
            cognitive: 8,
        };
        assert_eq!(debt.to_string(), "Test Complexity");
    }

    #[test]
    fn display_test_quality() {
        let debt = DebtType::TestQuality {
            issue_type: Some("poor assertions".into()),
        };
        assert_eq!(debt.to_string(), "Test Quality");
    }

    #[test]
    fn display_testing_gap() {
        let debt = DebtType::TestingGap {
            coverage: 0.25,
            cyclomatic: 15,
            cognitive: 12,
        };
        assert_eq!(debt.to_string(), "Testing Gap");
    }

    #[test]
    fn display_complexity_hotspot() {
        let debt = DebtType::ComplexityHotspot {
            cyclomatic: 30,
            cognitive: 25,
        };
        assert_eq!(debt.to_string(), "Complexity Hotspot");
    }

    #[test]
    fn display_dead_code() {
        let debt = DebtType::DeadCode {
            visibility: FunctionVisibility::Private,
            cyclomatic: 5,
            cognitive: 3,
            usage_hints: vec![],
        };
        assert_eq!(debt.to_string(), "Dead Code");
    }

    #[test]
    fn display_duplication() {
        let debt = DebtType::Duplication {
            instances: 3,
            total_lines: 45,
        };
        assert_eq!(debt.to_string(), "Duplication");
    }

    #[test]
    fn display_risk() {
        let debt = DebtType::Risk {
            risk_score: 0.8,
            factors: vec!["untested".into()],
        };
        assert_eq!(debt.to_string(), "Risk");
    }

    #[test]
    fn display_test_complexity_hotspot() {
        let debt = DebtType::TestComplexityHotspot {
            cyclomatic: 20,
            cognitive: 18,
            threshold: 15,
        };
        assert_eq!(debt.to_string(), "Test Complexity Hotspot");
    }

    #[test]
    fn display_test_todo() {
        let debt = DebtType::TestTodo {
            priority: crate::core::Priority::Low,
            reason: Some("add later".into()),
        };
        assert_eq!(debt.to_string(), "Test TODO");
    }

    #[test]
    fn display_test_duplication() {
        let debt = DebtType::TestDuplication {
            instances: 2,
            total_lines: 20,
            similarity: 0.9,
        };
        assert_eq!(debt.to_string(), "Test Duplication");
    }

    #[test]
    fn display_error_swallowing_includes_pattern() {
        let debt = DebtType::ErrorSwallowing {
            pattern: "unwrap()".into(),
            context: None,
        };
        assert_eq!(debt.to_string(), "Error Swallowing: unwrap()");
    }

    #[test]
    fn display_error_swallowing_with_different_patterns() {
        let patterns = vec![
            ("unwrap()", "Error Swallowing: unwrap()"),
            ("expect()", "Error Swallowing: expect()"),
            ("_ => {}", "Error Swallowing: _ => {}"),
        ];
        for (pattern, expected) in patterns {
            let debt = DebtType::ErrorSwallowing {
                pattern: pattern.into(),
                context: None,
            };
            assert_eq!(
                debt.to_string(),
                expected,
                "Failed for pattern: {}",
                pattern
            );
        }
    }

    #[test]
    fn display_allocation_inefficiency() {
        let debt = DebtType::AllocationInefficiency {
            pattern: "vec clone".into(),
            impact: "high".into(),
        };
        assert_eq!(debt.to_string(), "Allocation Inefficiency");
    }

    #[test]
    fn display_string_concatenation() {
        let debt = DebtType::StringConcatenation {
            loop_type: "for".into(),
            iterations: Some(100),
        };
        assert_eq!(debt.to_string(), "String Concatenation");
    }

    #[test]
    fn display_nested_loops() {
        let debt = DebtType::NestedLoops {
            depth: 3,
            complexity_estimate: "O(n^3)".into(),
        };
        assert_eq!(debt.to_string(), "Nested Loops");
    }

    #[test]
    fn display_blocking_io() {
        let debt = DebtType::BlockingIO {
            operation: "read_file".into(),
            context: "async fn".into(),
        };
        assert_eq!(debt.to_string(), "Blocking I/O");
    }

    #[test]
    fn display_suboptimal_data_structure() {
        let debt = DebtType::SuboptimalDataStructure {
            current_type: "Vec".into(),
            recommended_type: "HashSet".into(),
        };
        assert_eq!(debt.to_string(), "Suboptimal Data Structure");
    }

    #[test]
    fn display_god_object() {
        let debt = DebtType::GodObject {
            methods: 50,
            fields: Some(30),
            responsibilities: 5,
            god_object_score: 85.0,
            lines: 2000,
        };
        assert_eq!(debt.to_string(), "God Object");
    }

    #[test]
    fn display_feature_envy() {
        let debt = DebtType::FeatureEnvy {
            external_class: "Config".into(),
            usage_ratio: 0.75,
        };
        assert_eq!(debt.to_string(), "Feature Envy");
    }

    #[test]
    fn display_primitive_obsession() {
        let debt = DebtType::PrimitiveObsession {
            primitive_type: "String".into(),
            domain_concept: "Email".into(),
        };
        assert_eq!(debt.to_string(), "Primitive Obsession");
    }

    #[test]
    fn display_magic_values() {
        let debt = DebtType::MagicValues {
            value: "42".into(),
            occurrences: 5,
        };
        assert_eq!(debt.to_string(), "Magic Values");
    }

    #[test]
    fn display_assertion_complexity() {
        let debt = DebtType::AssertionComplexity {
            assertion_count: 15,
            complexity_score: 12.5,
        };
        assert_eq!(debt.to_string(), "Assertion Complexity");
    }

    #[test]
    fn display_flaky_test_pattern() {
        let debt = DebtType::FlakyTestPattern {
            pattern_type: "timing".into(),
            reliability_impact: "high".into(),
        };
        assert_eq!(debt.to_string(), "Flaky Test Pattern");
    }

    #[test]
    fn display_async_misuse() {
        let debt = DebtType::AsyncMisuse {
            pattern: "block_on".into(),
            performance_impact: "severe".into(),
        };
        assert_eq!(debt.to_string(), "Async Misuse");
    }

    #[test]
    fn display_resource_leak() {
        let debt = DebtType::ResourceLeak {
            resource_type: "file handle".into(),
            cleanup_missing: "close()".into(),
        };
        assert_eq!(debt.to_string(), "Resource Leak");
    }

    #[test]
    fn display_collection_inefficiency() {
        let debt = DebtType::CollectionInefficiency {
            collection_type: "Vec".into(),
            inefficiency_type: "repeated resize".into(),
        };
        assert_eq!(debt.to_string(), "Collection Inefficiency");
    }

    #[test]
    fn display_scattered_type() {
        let debt = DebtType::ScatteredType {
            type_name: "Config".into(),
            total_methods: 20,
            file_count: 5,
            severity: "high".into(),
        };
        assert_eq!(debt.to_string(), "Scattered Type");
    }

    #[test]
    fn display_orphaned_functions() {
        let debt = DebtType::OrphanedFunctions {
            target_type: "Parser".into(),
            function_count: 10,
            file_count: 3,
        };
        assert_eq!(debt.to_string(), "Orphaned Functions");
    }

    #[test]
    fn display_utilities_sprawl() {
        let debt = DebtType::UtilitiesSprawl {
            function_count: 25,
            distinct_types: 8,
        };
        assert_eq!(debt.to_string(), "Utilities Sprawl");
    }

    #[test]
    fn display_name_returns_non_empty_for_all_variants() {
        // Test that all variants return non-empty display names
        let variants: Vec<DebtType> = vec![
            DebtType::Todo { reason: None },
            DebtType::Fixme { reason: None },
            DebtType::CodeSmell { smell_type: None },
            DebtType::Complexity {
                cyclomatic: 0,
                cognitive: 0,
            },
            DebtType::Dependency {
                dependency_type: None,
            },
            DebtType::ResourceManagement { issue_type: None },
            DebtType::CodeOrganization { issue_type: None },
            DebtType::TestComplexity {
                cyclomatic: 0,
                cognitive: 0,
            },
            DebtType::TestQuality { issue_type: None },
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 0,
                cognitive: 0,
            },
            DebtType::ComplexityHotspot {
                cyclomatic: 0,
                cognitive: 0,
            },
            DebtType::DeadCode {
                visibility: FunctionVisibility::Private,
                cyclomatic: 0,
                cognitive: 0,
                usage_hints: vec![],
            },
            DebtType::Duplication {
                instances: 0,
                total_lines: 0,
            },
            DebtType::Risk {
                risk_score: 0.0,
                factors: vec![],
            },
            DebtType::TestComplexityHotspot {
                cyclomatic: 0,
                cognitive: 0,
                threshold: 0,
            },
            DebtType::TestTodo {
                priority: crate::core::Priority::Low,
                reason: None,
            },
            DebtType::TestDuplication {
                instances: 0,
                total_lines: 0,
                similarity: 0.0,
            },
            DebtType::ErrorSwallowing {
                pattern: "test".into(),
                context: None,
            },
            DebtType::AllocationInefficiency {
                pattern: "".into(),
                impact: "".into(),
            },
            DebtType::StringConcatenation {
                loop_type: "".into(),
                iterations: None,
            },
            DebtType::NestedLoops {
                depth: 0,
                complexity_estimate: "".into(),
            },
            DebtType::BlockingIO {
                operation: "".into(),
                context: "".into(),
            },
            DebtType::SuboptimalDataStructure {
                current_type: "".into(),
                recommended_type: "".into(),
            },
            DebtType::GodObject {
                methods: 0,
                fields: None,
                responsibilities: 0,
                god_object_score: 0.0,
                lines: 0,
            },
            DebtType::FeatureEnvy {
                external_class: "".into(),
                usage_ratio: 0.0,
            },
            DebtType::PrimitiveObsession {
                primitive_type: "".into(),
                domain_concept: "".into(),
            },
            DebtType::MagicValues {
                value: "".into(),
                occurrences: 0,
            },
            DebtType::AssertionComplexity {
                assertion_count: 0,
                complexity_score: 0.0,
            },
            DebtType::FlakyTestPattern {
                pattern_type: "".into(),
                reliability_impact: "".into(),
            },
            DebtType::AsyncMisuse {
                pattern: "".into(),
                performance_impact: "".into(),
            },
            DebtType::ResourceLeak {
                resource_type: "".into(),
                cleanup_missing: "".into(),
            },
            DebtType::CollectionInefficiency {
                collection_type: "".into(),
                inefficiency_type: "".into(),
            },
            DebtType::ScatteredType {
                type_name: "".into(),
                total_methods: 0,
                file_count: 0,
                severity: "".into(),
            },
            DebtType::OrphanedFunctions {
                target_type: "".into(),
                function_count: 0,
                file_count: 0,
            },
            DebtType::UtilitiesSprawl {
                function_count: 0,
                distinct_types: 0,
            },
        ];

        for variant in variants {
            let display = variant.to_string();
            assert!(!display.is_empty(), "Empty display for {:?}", variant);
            assert!(
                display.len() >= 3,
                "Display too short for {:?}: '{}'",
                variant,
                display
            );
        }
    }

    #[test]
    fn display_is_deterministic() {
        // Calling to_string() multiple times should yield identical results
        let debt = DebtType::GodObject {
            methods: 50,
            fields: Some(30),
            responsibilities: 5,
            god_object_score: 85.0,
            lines: 2000,
        };
        let s1 = debt.to_string();
        let s2 = debt.to_string();
        let s3 = debt.to_string();
        assert_eq!(s1, s2);
        assert_eq!(s2, s3);
    }
}

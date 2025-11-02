use debtmap::priority::file_metrics::{
    FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators,
};
use std::path::PathBuf;

#[test]
fn test_file_level_scoring_integration() {
    // Test that file-level scoring correctly aggregates function scores
    let mut metrics = FileDebtMetrics {
        path: PathBuf::from("src/complex_module.rs"),
        total_lines: 750,
        function_count: 45,
        class_count: 3,
        avg_complexity: 12.5,
        max_complexity: 35,
        total_complexity: 562,
        coverage_percent: 0.45,
        uncovered_lines: 412,
        god_object_indicators: GodObjectIndicators {
            methods_count: 45,
            fields_count: 20,
            responsibilities: 8,
            is_god_object: false,
            god_object_score: 0.0,
            responsibility_names: Vec::new(),
            recommended_splits: Vec::new(),
            module_structure: None,

            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: debtmap::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
        },
        function_scores: vec![],
        god_object_type: None,
        file_type: None,
    };

    // Add various function scores
    let function_scores = vec![
        3.5,  // Simple function
        8.2,  // Complex function
        15.0, // Very complex function
        2.1,  // Trivial function
        6.7,  // Moderate complexity
        12.3, // High complexity
        4.5,  // Below average
        9.8,  // Above average
        7.6,  // Moderate-high
        5.4,  // Average
    ];

    metrics.function_scores = function_scores.clone();

    let score = metrics.calculate_score();

    // Verify score is influenced by function scores
    assert!(score > 0.0, "Score should be positive");

    // Test with empty function scores
    metrics.function_scores = vec![];
    let score_without_functions = metrics.calculate_score();

    metrics.function_scores = function_scores;
    let score_with_functions = metrics.calculate_score();

    assert!(
        score_with_functions > score_without_functions,
        "Score with function scores should be higher than without"
    );
}

#[test]
fn test_file_scoring_with_god_object_detection() {
    // Test integration between god object detection and file scoring
    let metrics = FileDebtMetrics {
        path: PathBuf::from("src/god_class.rs"),
        total_lines: 2000,
        function_count: 80,
        class_count: 1,
        avg_complexity: 18.0,
        max_complexity: 60,
        total_complexity: 1440,
        coverage_percent: 0.25,
        uncovered_lines: 1500,
        god_object_indicators: GodObjectIndicators {
            methods_count: 80,
            fields_count: 40,
            responsibilities: 12,
            is_god_object: true,
            god_object_score: 0.9,
            responsibility_names: Vec::new(),
            recommended_splits: Vec::new(),
            module_structure: None,

            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: debtmap::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
        },
        god_object_type: None,
        function_scores: vec![8.0; 80], // High scores for all functions
        file_type: None,
    };

    let score = metrics.calculate_score();
    let recommendation = metrics.generate_recommendation();

    // God object should have very high score
    assert!(
        score > 100.0,
        "God object with high complexity should have very high score"
    );
    assert!(
        recommendation.contains("Split") || recommendation.contains("URGENT"),
        "Should recommend breaking up god object, got: {}",
        recommendation
    );
    assert!(
        recommendation.contains("modules") || recommendation.contains("functions"),
        "Should suggest modularization, got: {}",
        recommendation
    );
}

#[test]
fn test_file_scoring_priorities() {
    // Test that files are correctly prioritized based on scores
    let files = vec![
        FileDebtMetrics {
            path: PathBuf::from("low_priority.rs"),
            total_lines: 50,
            function_count: 3,
            avg_complexity: 2.0,
            total_complexity: 6,
            coverage_percent: 0.9,
            function_scores: vec![1.0, 1.5, 2.0],
            god_object_type: None,
            ..Default::default()
        },
        FileDebtMetrics {
            path: PathBuf::from("medium_priority.rs"),
            total_lines: 300,
            function_count: 20,
            avg_complexity: 8.0,
            total_complexity: 160,
            coverage_percent: 0.6,
            function_scores: vec![5.0; 20],
            god_object_type: None,
            ..Default::default()
        },
        FileDebtMetrics {
            path: PathBuf::from("high_priority.rs"),
            total_lines: 800,
            function_count: 60,
            avg_complexity: 15.0,
            total_complexity: 900,
            coverage_percent: 0.3,
            god_object_indicators: GodObjectIndicators {
                is_god_object: true,
                god_object_score: 0.7,
                responsibility_names: Vec::new(),
                recommended_splits: Vec::new(),
                ..Default::default()
            },
            function_scores: vec![7.0; 60],
            god_object_type: None,
            ..Default::default()
        },
    ];

    let mut scores: Vec<(PathBuf, f64)> = files
        .iter()
        .map(|f| (f.path.clone(), f.calculate_score()))
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Verify correct prioritization
    assert_eq!(scores[0].0, PathBuf::from("high_priority.rs"));
    assert_eq!(scores[1].0, PathBuf::from("medium_priority.rs"));
    assert_eq!(scores[2].0, PathBuf::from("low_priority.rs"));

    // Verify score magnitudes make sense
    assert!(
        scores[0].1 > scores[1].1 * 2.0,
        "High priority should be significantly higher"
    );
    assert!(
        scores[1].1 > scores[2].1 * 2.0,
        "Medium should be significantly higher than low"
    );
}

#[test]
fn test_file_debt_item_creation() {
    // Test creating a complete FileDebtItem with metrics and recommendations
    let metrics = FileDebtMetrics {
        path: PathBuf::from("src/refactor_target.rs"),
        total_lines: 600,
        function_count: 35,
        class_count: 2,
        avg_complexity: 11.0,
        max_complexity: 28,
        total_complexity: 385,
        coverage_percent: 0.4,
        uncovered_lines: 360,
        god_object_indicators: GodObjectIndicators::default(),
        function_scores: vec![6.0; 35],
        god_object_type: None,
        file_type: None,
    };

    let score = metrics.calculate_score();
    let recommendation = metrics.generate_recommendation();

    let debt_item = FileDebtItem {
        metrics: metrics.clone(),
        score,
        priority_rank: 1,
        recommendation: recommendation.clone(),
        impact: FileImpact {
            complexity_reduction: 0.35,
            maintainability_improvement: 0.45,
            test_effort: 0.6,
        },
    };

    assert_eq!(debt_item.score, score);
    assert_eq!(debt_item.recommendation, recommendation);
    assert_eq!(debt_item.priority_rank, 1);
    assert!(debt_item.impact.complexity_reduction > 0.0);
    assert!(debt_item.impact.maintainability_improvement > 0.0);
    assert!(debt_item.impact.test_effort > 0.0);
}

#[test]
fn test_file_scoring_edge_cases() {
    // Test edge cases in file scoring

    // Empty file
    let empty_metrics = FileDebtMetrics::default();
    let empty_score = empty_metrics.calculate_score();
    assert_eq!(empty_score, 0.0, "Empty file should have zero score");

    // File with only coverage issues
    let coverage_only = FileDebtMetrics {
        path: PathBuf::from("untested.rs"),
        total_lines: 100,
        function_count: 5,
        avg_complexity: 1.0,
        total_complexity: 5,
        coverage_percent: 0.0,
        uncovered_lines: 100,
        function_scores: vec![1.0; 5],
        god_object_type: None,
        ..Default::default()
    };
    let coverage_score = coverage_only.calculate_score();
    assert!(
        coverage_score > 0.0,
        "Zero coverage should produce non-zero score"
    );

    // Perfect file (low complexity, high coverage)
    let perfect_file = FileDebtMetrics {
        path: PathBuf::from("perfect.rs"),
        total_lines: 100,
        function_count: 5,
        avg_complexity: 1.0,
        total_complexity: 5,
        coverage_percent: 1.0,
        uncovered_lines: 0,
        function_scores: vec![0.5; 5],
        god_object_type: None,
        ..Default::default()
    };
    let perfect_score = perfect_file.calculate_score();
    assert!(
        perfect_score < 1.0,
        "Perfect file should have very low score"
    );
}

#[test]
fn test_recommendation_generation_completeness() {
    // Test that all recommendation types are generated correctly

    let test_cases = vec![
        (
            FileDebtMetrics {
                god_object_indicators: GodObjectIndicators {
                    is_god_object: true,
                    ..Default::default()
                },
                function_count: 40,
                ..Default::default()
            },
            "Split", // Changed from "Break into" to match new recommendation format
        ),
        (
            FileDebtMetrics {
                total_lines: 1000,
                ..Default::default()
            },
            "Extract complex functions",
        ),
        (
            FileDebtMetrics {
                avg_complexity: 20.0,
                total_lines: 100,
                ..Default::default()
            },
            "Simplify complex functions",
        ),
        (
            FileDebtMetrics {
                coverage_percent: 0.2,
                total_lines: 100,
                avg_complexity: 2.0,
                ..Default::default()
            },
            "Increase test coverage",
        ),
        (
            FileDebtMetrics {
                total_lines: 200,
                avg_complexity: 5.0,
                coverage_percent: 0.8,
                ..Default::default()
            },
            "Refactor for better maintainability",
        ),
    ];

    for (metrics, expected_text) in test_cases {
        let recommendation = metrics.generate_recommendation();
        assert!(
            recommendation.contains(expected_text),
            "Recommendation '{}' should contain '{}'",
            recommendation,
            expected_text
        );
    }
}

#[test]
fn test_file_scoring_with_real_world_scenarios() {
    // Test realistic scenarios that might occur in production

    // Scenario 1: Legacy file with no tests
    let legacy_file = FileDebtMetrics {
        path: PathBuf::from("src/legacy/old_module.rs"),
        total_lines: 1500,
        function_count: 70,
        class_count: 5,
        avg_complexity: 25.0,
        max_complexity: 80,
        total_complexity: 1750,
        coverage_percent: 0.0,
        uncovered_lines: 1500,
        god_object_indicators: GodObjectIndicators {
            methods_count: 70,
            fields_count: 35,
            responsibilities: 15,
            is_god_object: true,
            god_object_score: 0.95,
            responsibility_names: Vec::new(),
            recommended_splits: Vec::new(),
            module_structure: None,

            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: debtmap::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
        },
        function_scores: vec![9.0; 70],
        god_object_type: None,
        file_type: None,
    };

    let legacy_score = legacy_file.calculate_score();
    assert!(
        legacy_score > 200.0,
        "Legacy file should have extremely high score"
    );

    // Scenario 2: Well-maintained utility file
    let util_file = FileDebtMetrics {
        path: PathBuf::from("src/utils/helpers.rs"),
        total_lines: 200,
        function_count: 15,
        class_count: 0,
        avg_complexity: 3.0,
        max_complexity: 6,
        total_complexity: 45,
        coverage_percent: 0.95,
        uncovered_lines: 10,
        god_object_indicators: GodObjectIndicators::default(),
        function_scores: vec![2.0; 15],
        god_object_type: None,
        file_type: None,
    };

    let util_score = util_file.calculate_score();
    assert!(
        util_score < 5.0,
        "Well-maintained utility should have low score"
    );

    // Scenario 3: Business logic with moderate issues
    let business_logic = FileDebtMetrics {
        path: PathBuf::from("src/business/order_processing.rs"),
        total_lines: 500,
        function_count: 30,
        class_count: 3,
        avg_complexity: 8.0,
        max_complexity: 20,
        total_complexity: 240,
        coverage_percent: 0.65,
        uncovered_lines: 175,
        god_object_indicators: GodObjectIndicators::default(),
        function_scores: vec![5.5; 30],
        god_object_type: None,
        file_type: None,
    };

    let business_score = business_logic.calculate_score();
    assert!(
        business_score > 10.0 && business_score < 300.0,
        "Business logic should have moderate score, got: {}",
        business_score
    );

    // Verify relative ordering
    assert!(legacy_score > business_score);
    assert!(business_score > util_score);
}

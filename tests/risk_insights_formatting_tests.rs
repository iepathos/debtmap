use debtmap::risk::insights::*;
use debtmap::risk::{Difficulty, TestEffort, TestingRecommendation};
use im::Vector;
use std::path::PathBuf;

#[test]
fn test_format_risk_reduction_less_than_half() {
    assert_eq!(format_risk_reduction(0.3), "<1");
    assert_eq!(format_risk_reduction(0.49), "<1");
}

#[test]
fn test_format_risk_reduction_half_or_greater() {
    assert_eq!(format_risk_reduction(0.5), "0"); // Rounds down
    assert_eq!(format_risk_reduction(1.0), "1");
    assert_eq!(format_risk_reduction(5.7), "6");
    assert_eq!(format_risk_reduction(10.2), "10");
}

#[test]
fn test_format_roi_display_high_values() {
    assert_eq!(format_roi_display(10.0), "10");
    assert_eq!(format_roi_display(15.5), "16");
    assert_eq!(format_roi_display(100.9), "101");
}

#[test]
fn test_format_roi_display_low_values() {
    assert_eq!(format_roi_display(0.1), "0.1");
    assert_eq!(format_roi_display(5.5), "5.5");
    assert_eq!(format_roi_display(9.99), "10.0");
}

#[test]
fn test_determine_risk_level() {
    assert_eq!(determine_risk_level(10.0), "HIGH");
    assert_eq!(determine_risk_level(8.0), "HIGH");
    assert_eq!(determine_risk_level(7.9), "MEDIUM");
    assert_eq!(determine_risk_level(5.0), "MEDIUM");
    assert_eq!(determine_risk_level(4.9), "LOW");
    assert_eq!(determine_risk_level(0.0), "LOW");
}

#[test]
fn test_format_difficulty_all_levels() {
    assert_eq!(format_difficulty(&Difficulty::Trivial), "trivial");
    assert_eq!(format_difficulty(&Difficulty::Simple), "simple");
    assert_eq!(format_difficulty(&Difficulty::Moderate), "moderate");
    assert_eq!(format_difficulty(&Difficulty::Complex), "complex");
    assert_eq!(format_difficulty(&Difficulty::VeryComplex), "very complex");
}

#[test]
fn test_format_complexity_info() {
    assert_eq!(format_complexity_info(0, 0), "est_branches=0, cognitive=0");
    assert_eq!(
        format_complexity_info(5, 10),
        "est_branches=5, cognitive=10"
    );
    assert_eq!(
        format_complexity_info(100, 200),
        "est_branches=100, cognitive=200"
    );
}

#[test]
fn test_format_dependency_info() {
    assert_eq!(format_dependency_info(0, 0), "0 upstream, 0 downstream");
    assert_eq!(format_dependency_info(3, 5), "3 upstream, 5 downstream");
    assert_eq!(format_dependency_info(10, 0), "10 upstream, 0 downstream");
}

#[test]
fn test_calculate_dash_count() {
    // Total width is 82, accounting for "┌─ " (3) + " ─┐" (3) + 2 spaces = 8
    assert_eq!(calculate_dash_count(2, 7), 65); // #1 + Priority: 1.0
    assert_eq!(calculate_dash_count(3, 10), 61); // #10 + Priority: 100.0
}

#[test]
fn test_format_recommendation_box_header() {
    let header = format_recommendation_box_header(0, "5.0");
    assert!(header.contains("#1"));
    assert!(header.contains("Priority: 5.0"));
    assert!(header.starts_with("┌─"));
    assert!(header.ends_with("─┐\n"));
}

#[test]
fn test_format_recommendations_empty() {
    let recommendations = Vector::new();
    let result = format_recommendations(&recommendations);
    assert_eq!(result, "");
}

#[test]
fn test_format_recommendations_with_single_item() {
    let mut recommendations = Vector::new();
    recommendations.push_back(TestingRecommendation {
        function: "test_func".to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        current_risk: 8.5,
        potential_risk_reduction: 2.5,
        roi: Some(5.0),
        test_effort_estimate: TestEffort {
            estimated_difficulty: Difficulty::Simple,
            branch_count: 3,
            cognitive_load: 5,
            recommended_test_cases: 3,
        },
        dependencies: vec![],
        dependents: vec![],
        rationale: "Test rationale".to_string(),
    });

    let result = format_recommendations(&recommendations);

    assert!(result.contains("[TARGET] TOP 5 TESTING RECOMMENDATIONS"));
    assert!(result.contains("Priority: 5.0"));
    assert!(result.contains("test_func()"));
    assert!(result.contains("test.rs:10"));
    assert!(result.contains("Risk: HIGH (8.5)"));
    assert!(result.contains("Impact: -2%")); // 2.5 rounds down to 2
    assert!(result.contains("Complexity: simple"));
    assert!(result.contains("est_branches=3, cognitive=5"));
    assert!(result.contains("0 upstream, 0 downstream"));
    assert!(result.contains("Test rationale"));
}

#[test]
fn test_format_recommendations_with_dependents() {
    let mut recommendations = Vector::new();
    recommendations.push_back(TestingRecommendation {
        function: "used_func".to_string(),
        file: PathBuf::from("lib.rs"),
        line: 20,
        current_risk: 6.0,
        potential_risk_reduction: 1.0,
        roi: Some(2.5),
        test_effort_estimate: TestEffort {
            estimated_difficulty: Difficulty::Moderate,
            branch_count: 5,
            cognitive_load: 8,
            recommended_test_cases: 5,
        },
        dependencies: vec!["dep1".to_string(), "dep2".to_string()],
        dependents: vec!["caller1".to_string(), "caller2".to_string()],
        rationale: "Important function".to_string(),
    });

    let result = format_recommendations(&recommendations);

    assert!(result.contains("Risk: MEDIUM"));
    assert!(result.contains("2 upstream, 2 downstream"));
    assert!(result.contains("Used by: caller1, caller2"));
}

#[test]
fn test_format_recommendations_truncates_to_five() {
    let mut recommendations = Vector::new();
    for i in 0..10 {
        recommendations.push_back(TestingRecommendation {
            function: format!("func_{i}"),
            file: PathBuf::from(format!("file_{i}.rs")),
            line: i * 10,
            current_risk: 5.0,
            potential_risk_reduction: 1.0,
            roi: Some(1.0),
            test_effort_estimate: TestEffort {
                estimated_difficulty: Difficulty::Simple,
                branch_count: 1,
                cognitive_load: 1,
                recommended_test_cases: 2,
            },
            dependencies: vec![],
            dependents: vec![],
            rationale: format!("Rationale {i}"),
        });
    }

    let result = format_recommendations(&recommendations);

    // Should only show first 5
    for i in 0..5 {
        assert!(result.contains(&format!("func_{i}()")));
    }
    // Should not show items 6-10
    for i in 5..10 {
        assert!(!result.contains(&format!("func_{i}()")));
    }
}

#[test]
fn test_format_recommendations_with_low_risk() {
    let mut recommendations = Vector::new();
    recommendations.push_back(TestingRecommendation {
        function: "low_risk_func".to_string(),
        file: PathBuf::from("safe.rs"),
        line: 5,
        current_risk: 2.0,
        potential_risk_reduction: 0.2,
        roi: Some(0.5),
        test_effort_estimate: TestEffort {
            estimated_difficulty: Difficulty::Trivial,
            branch_count: 1,
            cognitive_load: 0,
            recommended_test_cases: 1,
        },
        dependencies: vec![],
        dependents: vec![],
        rationale: "Low risk function".to_string(),
    });

    let result = format_recommendations(&recommendations);

    assert!(result.contains("Risk: LOW"));
    assert!(result.contains("Impact: -<1%"));
    assert!(result.contains("Priority: 0.5"));
    assert!(result.contains("Complexity: trivial"));
}

#[test]
fn test_format_recommendations_with_high_roi() {
    let mut recommendations = Vector::new();
    recommendations.push_back(TestingRecommendation {
        function: "high_roi_func".to_string(),
        file: PathBuf::from("critical.rs"),
        line: 100,
        current_risk: 10.0,
        potential_risk_reduction: 8.0,
        roi: Some(15.5),
        test_effort_estimate: TestEffort {
            estimated_difficulty: Difficulty::VeryComplex,
            branch_count: 20,
            cognitive_load: 50,
            recommended_test_cases: 20,
        },
        dependencies: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        dependents: vec![],
        rationale: "Critical high-risk function needing immediate attention".to_string(),
    });

    let result = format_recommendations(&recommendations);

    assert!(result.contains("Priority: 16")); // 15.5 rounds to 16 for display
    assert!(result.contains("Risk: HIGH (10.0)"));
    assert!(result.contains("Impact: -8%"));
    assert!(result.contains("Complexity: very complex"));
    assert!(result.contains("3 upstream, 0 downstream"));
}

#[test]
fn test_format_recommendations_with_none_roi() {
    let mut recommendations = Vector::new();
    recommendations.push_back(TestingRecommendation {
        function: "no_roi_func".to_string(),
        file: PathBuf::from("unknown.rs"),
        line: 1,
        current_risk: 5.0,
        potential_risk_reduction: 1.0,
        roi: None, // No ROI value
        test_effort_estimate: TestEffort {
            estimated_difficulty: Difficulty::Simple,
            branch_count: 2,
            cognitive_load: 3,
            recommended_test_cases: 2,
        },
        dependencies: vec![],
        dependents: vec![],
        rationale: "Function with unknown ROI".to_string(),
    });

    let result = format_recommendations(&recommendations);

    assert!(result.contains("Priority: 0.1")); // Default value when None
}

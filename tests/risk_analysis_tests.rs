use chrono::Utc;
use debtmap::core::{
    AnalysisResults, ComplexityReport, ComplexitySummary, DependencyReport, FunctionMetrics,
    TechnicalDebtReport,
};
use debtmap::utils::risk_analyzer::{analyze_risk_with_coverage, analyze_risk_without_coverage};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_analyze_risk_with_coverage_success() {
    // Create test data
    let temp_dir = TempDir::new().unwrap();
    let lcov_path = temp_dir.path().join("test.lcov");

    // Create a simple LCOV file
    let lcov_content = r#"TN:
SF:src/test.rs
FN:10,test_func
FNDA:5,test_func
FNF:1
FNH:1
DA:10,5
DA:11,5
DA:12,0
DA:13,0
LF:4
LH:2
end_of_record
"#;
    fs::write(&lcov_path, lcov_content).unwrap();

    // Create analysis results with test functions
    let results = AnalysisResults {
        project_path: temp_dir.path().to_path_buf(),
        timestamp: Utc::now(),
        complexity: ComplexityReport {
            metrics: vec![FunctionMetrics {
                name: "test_func".to_string(),
                file: PathBuf::from("src/test.rs"),
                line: 10,
                cyclomatic: 4,
                cognitive: 3,
                nesting: 2,
                length: 4,
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
                purity_reason: None,
                call_dependencies: None,
                purity_level: None,
            }],
            summary: ComplexitySummary {
                total_functions: 1,
                average_complexity: 4.0,
                max_complexity: 4,
                high_complexity_count: 0,
            },
        },
        technical_debt: TechnicalDebtReport {
            items: vec![],
            by_type: std::collections::HashMap::new(),
            priorities: vec![],
            duplications: vec![],
        },
        dependencies: DependencyReport {
            modules: vec![],
            circular: vec![],
        },
        duplications: vec![],
        file_contexts: HashMap::new(),
    };

    // Test with coverage analysis
    let result =
        analyze_risk_with_coverage(&results, &lcov_path, temp_dir.path(), false, None, None);

    assert!(result.is_ok());
    let insight = result.unwrap();
    assert!(insight.is_some());

    let insight = insight.unwrap();
    // Should have analyzed one function
    assert!(!insight.top_risks.is_empty());
    // Coverage should be calculated (50% in our test LCOV)
    assert!(insight.top_risks[0].coverage_percentage.is_some());
}

#[test]
fn test_analyze_risk_with_coverage_invalid_lcov_path() {
    let temp_dir = TempDir::new().unwrap();
    let non_existent_lcov = temp_dir.path().join("missing.lcov");

    let results = AnalysisResults {
        project_path: temp_dir.path().to_path_buf(),
        timestamp: Utc::now(),
        complexity: ComplexityReport {
            metrics: vec![FunctionMetrics {
                name: "test_func".to_string(),
                file: PathBuf::from("src/test.rs"),
                line: 10,
                cyclomatic: 3,
                cognitive: 2,
                nesting: 1,
                length: 10,
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
                purity_reason: None,
                call_dependencies: None,
                purity_level: None,
            }],
            summary: ComplexitySummary {
                total_functions: 1,
                average_complexity: 3.0,
                max_complexity: 3,
                high_complexity_count: 0,
            },
        },
        technical_debt: TechnicalDebtReport {
            items: vec![],
            by_type: std::collections::HashMap::new(),
            priorities: vec![],
            duplications: vec![],
        },
        dependencies: DependencyReport {
            modules: vec![],
            circular: vec![],
        },
        duplications: vec![],
        file_contexts: HashMap::new(),
    };

    // Should fail when LCOV file doesn't exist
    let result = analyze_risk_with_coverage(
        &results,
        &non_existent_lcov,
        temp_dir.path(),
        false,
        None,
        None,
    );

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to parse LCOV file"));
}

#[test]
fn test_analyze_risk_without_coverage() {
    let temp_dir = TempDir::new().unwrap();

    let results = AnalysisResults {
        project_path: temp_dir.path().to_path_buf(),
        timestamp: Utc::now(),
        complexity: ComplexityReport {
            metrics: vec![FunctionMetrics {
                name: "main".to_string(),
                file: PathBuf::from("src/test.rs"),
                line: 10,
                cyclomatic: 2,
                cognitive: 1,
                nesting: 0,
                length: 2,
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
                purity_reason: None,
                call_dependencies: None,
                purity_level: None,
            }],
            summary: ComplexitySummary {
                total_functions: 1,
                average_complexity: 2.0,
                max_complexity: 2,
                high_complexity_count: 0,
            },
        },
        technical_debt: TechnicalDebtReport {
            items: vec![],
            by_type: std::collections::HashMap::new(),
            priorities: vec![],
            duplications: vec![],
        },
        dependencies: DependencyReport {
            modules: vec![],
            circular: vec![],
        },
        duplications: vec![],
        file_contexts: HashMap::new(),
    };

    // Test without coverage
    let result = analyze_risk_without_coverage(
        &results,
        false, // enable_context
        None,  // context_providers
        None,  // disable_context
        temp_dir.path(),
    );

    assert!(result.is_ok());
    let insight = result.unwrap();
    assert!(insight.is_some());

    let insight = insight.unwrap();
    // Should have analyzed one function
    assert!(!insight.top_risks.is_empty());
    // No coverage data
    assert_eq!(insight.top_risks[0].coverage_percentage, None);
}

#[test]
fn test_analyze_risk_with_coverage_empty_metrics() {
    let temp_dir = TempDir::new().unwrap();
    let lcov_path = temp_dir.path().join("test.lcov");

    // Create valid LCOV file
    let lcov_content = r#"TN:
SF:src/test.rs
FNF:0
FNH:0
LF:0
LH:0
end_of_record
"#;
    fs::write(&lcov_path, lcov_content).unwrap();

    let results = AnalysisResults {
        project_path: temp_dir.path().to_path_buf(),
        timestamp: Utc::now(),
        complexity: ComplexityReport {
            metrics: vec![], // Empty metrics
            summary: ComplexitySummary {
                total_functions: 0,
                average_complexity: 0.0,
                max_complexity: 0,
                high_complexity_count: 0,
            },
        },
        technical_debt: TechnicalDebtReport {
            items: vec![],
            by_type: std::collections::HashMap::new(),
            priorities: vec![],
            duplications: vec![],
        },
        dependencies: DependencyReport {
            modules: vec![],
            circular: vec![],
        },
        duplications: vec![],
        file_contexts: HashMap::new(),
    };

    let result =
        analyze_risk_with_coverage(&results, &lcov_path, temp_dir.path(), false, None, None);

    assert!(result.is_ok());
    let insights = result.unwrap();
    assert!(insights.is_some());

    let insights = insights.unwrap();
    // No functions means no risks
    assert!(insights.top_risks.is_empty());
    assert_eq!(insights.risk_distribution.total_functions, 0);
}

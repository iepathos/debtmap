use chrono::Utc;
use debtmap::*;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_output_json_format() {
    let results = AnalysisResults {
        project_path: PathBuf::from("/test/project"),
        timestamp: Utc::now(),
        complexity: ComplexityReport {
            metrics: vec![FunctionMetrics {
                name: "test_func".to_string(),
                file: PathBuf::from("test.rs"),
                line: 10,
                cyclomatic: 5,
                cognitive: 7,
                nesting: 2,
                length: 25,
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
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            }],
            summary: ComplexitySummary {
                total_functions: 1,
                average_complexity: 5.0,
                max_complexity: 5,
                high_complexity_count: 0,
            },
        },
        technical_debt: TechnicalDebtReport {
            items: vec![DebtItem {
                id: "test-1".to_string(),
                debt_type: DebtType::Todo { reason: None },
                priority: Priority::Medium,
                file: PathBuf::from("test.rs"),
                line: 5,
                column: None,
                message: "TODO: Implement feature".to_string(), // debtmap:ignore -- Test fixture
                context: None,
            }],
            by_type: {
                let mut map = HashMap::new();
                map.insert(DebtType::Todo { reason: None }, vec![]);
                map
            },
            priorities: vec![Priority::Medium],
            duplications: vec![],
        },
        dependencies: DependencyReport {
            modules: vec![],
            circular: vec![],
        },
        duplications: vec![],
        file_contexts: HashMap::new(),
    };

    let mut writer = create_writer(OutputFormat::Json);
    let result = writer.write_results(&results);
    assert!(result.is_ok(), "JSON output should succeed");
}

#[test]
fn test_output_markdown_format() {
    let results = AnalysisResults {
        project_path: PathBuf::from("/test/project"),
        timestamp: Utc::now(),
        complexity: ComplexityReport {
            metrics: vec![],
            summary: ComplexitySummary {
                total_functions: 10,
                average_complexity: 5.5,
                max_complexity: 15,
                high_complexity_count: 2,
            },
        },
        technical_debt: TechnicalDebtReport {
            items: vec![],
            by_type: HashMap::new(),
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

    let mut writer = create_writer(OutputFormat::Markdown);
    let result = writer.write_results(&results);
    assert!(result.is_ok(), "Markdown output should succeed");
}

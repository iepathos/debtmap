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
                debt_type: DebtType::Todo,
                priority: Priority::Medium,
                file: PathBuf::from("test.rs"),
                line: 5,
                message: "TODO: Implement feature".to_string(), // debtmap:ignore -- Test fixture
                context: None,
            }],
            by_type: {
                let mut map = HashMap::new();
                map.insert(DebtType::Todo, vec![]);
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
    };

    let mut writer = create_writer(OutputFormat::Markdown);
    let result = writer.write_results(&results);
    assert!(result.is_ok(), "Markdown output should succeed");
}

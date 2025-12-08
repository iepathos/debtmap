//! Integration test verifying error swallowing items flow through the full pipeline.
//!
//! This test verifies:
//! 1. detect_error_swallowing() returns items (unit level)
//! 2. FileMetrics.debt_items contains error swallowing items after analysis
//! 3. AnalysisResults.technical_debt.items contains error swallowing items
//! 4. UnifiedAnalysis.items contains error swallowing items

use std::path::Path;

/// Test that detect_error_swallowing returns items for known patterns
#[test]
fn test_detect_error_swallowing_returns_items() {
    let code = r#"
fn example() {
    if let Ok(value) = some_function() {
        println!("{}", value);
    }
}

fn some_function() -> Result<i32, std::io::Error> {
    Ok(42)
}
"#;

    let file = syn::parse_str::<syn::File>(code).expect("Failed to parse");
    let items =
        debtmap::debt::error_swallowing::detect_error_swallowing(&file, Path::new("test.rs"), None);

    assert!(
        !items.is_empty(),
        "detect_error_swallowing should return items for if let Ok pattern"
    );

    // Verify the debt type is ErrorSwallowing
    for item in &items {
        assert!(
            matches!(item.debt_type, debtmap::core::DebtType::ErrorSwallowing { .. }),
            "Expected ErrorSwallowing debt type, got {:?}",
            item.debt_type
        );
    }
}

/// Test that FileMetrics contains error swallowing items after rust analysis
#[test]
fn test_file_metrics_contains_error_swallowing() {
    use debtmap::analyzers::{analyze_file, get_analyzer};
    use debtmap::core::Language;
    use std::path::PathBuf;

    let code = r#"
fn example() {
    if let Ok(value) = some_function() {
        println!("{}", value);
    }
}

fn some_function() -> Result<i32, std::io::Error> {
    Ok(42)
}
"#;

    let analyzer = get_analyzer(Language::Rust);
    let path = PathBuf::from("test.rs");
    let metrics = analyze_file(code.to_string(), path, &*analyzer).expect("Failed to analyze");

    // Check if any debt items are ErrorSwallowing
    let error_swallowing_items: Vec<_> = metrics
        .debt_items
        .iter()
        .filter(|item| matches!(item.debt_type, debtmap::core::DebtType::ErrorSwallowing { .. }))
        .collect();

    assert!(
        !error_swallowing_items.is_empty(),
        "FileMetrics.debt_items should contain ErrorSwallowing items. Found debt types: {:?}",
        metrics
            .debt_items
            .iter()
            .map(|i| format!("{:?}", i.debt_type))
            .collect::<Vec<_>>()
    );
}

/// Test that error swallowing items flow through to UnifiedAnalysis
#[test]
fn test_unified_analysis_contains_error_swallowing() {
    use debtmap::analyzers::{analyze_file, get_analyzer};
    use debtmap::builders::unified_analysis::perform_unified_analysis;
    use debtmap::core::{
        AnalysisResults, ComplexityReport, ComplexitySummary, DependencyReport, Language,
        TechnicalDebtReport,
    };
    use debtmap::priority::DebtType;
    use std::collections::HashMap;
    use std::path::PathBuf;

    let code = r#"
fn example() {
    if let Ok(value) = some_function() {
        println!("{}", value);
    }
}

fn some_function() -> Result<i32, std::io::Error> {
    Ok(42)
}
"#;

    let analyzer = get_analyzer(Language::Rust);
    let path = PathBuf::from("test.rs");
    let metrics = analyze_file(code.to_string(), path.clone(), &*analyzer).expect("Failed to analyze");

    // Build AnalysisResults with the debt items
    let all_functions = metrics.complexity.functions.clone();
    let mut by_type = HashMap::new();
    for item in &metrics.debt_items {
        by_type
            .entry(item.debt_type.clone())
            .or_insert_with(Vec::new)
            .push(item.clone());
    }
    let priorities = metrics.debt_items.iter().map(|i| i.priority).collect();

    let results = AnalysisResults {
        project_path: PathBuf::from("."),
        timestamp: chrono::Utc::now(),
        complexity: ComplexityReport {
            metrics: all_functions,
            summary: ComplexitySummary {
                total_functions: 2,
                average_complexity: 1.0,
                max_complexity: 1,
                high_complexity_count: 0,
            },
        },
        technical_debt: TechnicalDebtReport {
            items: metrics.debt_items.clone(),
            by_type,
            priorities,
            duplications: vec![],
        },
        dependencies: DependencyReport {
            modules: vec![],
            circular: vec![],
        },
        duplications: vec![],
        file_contexts: HashMap::new(),
    };

    // Perform unified analysis
    let unified = perform_unified_analysis(&results, None, false, &PathBuf::from("."), false, false)
        .expect("Failed to perform unified analysis");

    // Check for ErrorSwallowing items in unified analysis
    let error_swallowing_items: Vec<_> = unified
        .items
        .iter()
        .filter(|item| {
            matches!(
                &item.debt_type,
                DebtType::ErrorSwallowing { .. }
            )
        })
        .collect();

    assert!(
        !error_swallowing_items.is_empty(),
        "UnifiedAnalysis.items should contain ErrorSwallowing items. Found {} items total with types: {:?}",
        unified.items.len(),
        unified.items.iter().map(|i| format!("{:?}", std::mem::discriminant(&i.debt_type))).collect::<Vec<_>>()
    );
}

/// Test that AnalysisResults.technical_debt.items contains error swallowing items
#[test]
fn test_analysis_results_contains_error_swallowing() {
    use debtmap::analysis_utils;
    use debtmap::analyzers::{analyze_file, get_analyzer};
    use debtmap::core::Language;
    use debtmap::utils::analysis_helpers::build_technical_debt_report;
    use std::path::PathBuf;

    let code = r#"
fn example() {
    if let Ok(value) = some_function() {
        println!("{}", value);
    }
}

fn some_function() -> Result<i32, std::io::Error> {
    Ok(42)
}
"#;

    let analyzer = get_analyzer(Language::Rust);
    let path = PathBuf::from("test.rs");
    let metrics = analyze_file(code.to_string(), path, &*analyzer).expect("Failed to analyze");

    // Simulate the pipeline: extract debt items and build technical debt report
    let file_metrics = vec![metrics];
    let all_debt_items = analysis_utils::extract_all_debt_items(&file_metrics);

    // Check extracted debt items contain ErrorSwallowing
    let error_swallowing_items: Vec<_> = all_debt_items
        .iter()
        .filter(|item| matches!(item.debt_type, debtmap::core::DebtType::ErrorSwallowing { .. }))
        .collect();

    assert!(
        !error_swallowing_items.is_empty(),
        "extract_all_debt_items should include ErrorSwallowing items. Found: {:?}",
        all_debt_items
            .iter()
            .map(|i| format!("{:?}", i.debt_type))
            .collect::<Vec<_>>()
    );

    // Build technical debt report
    let technical_debt = build_technical_debt_report(all_debt_items, vec![]);

    // Check technical debt report contains ErrorSwallowing
    let report_error_swallowing: Vec<_> = technical_debt
        .items
        .iter()
        .filter(|item| matches!(item.debt_type, debtmap::core::DebtType::ErrorSwallowing { .. }))
        .collect();

    assert!(
        !report_error_swallowing.is_empty(),
        "TechnicalDebtReport.items should contain ErrorSwallowing items"
    );
}

// Analysis helper functions for direct library API testing
use anyhow::Result;
use chrono::Utc;
use debtmap::analyzers::{analyze_file, get_analyzer, get_analyzer_with_context};
use debtmap::core::{
    AnalysisResults, ComplexityReport, ComplexitySummary, DebtItem, DebtType, DependencyReport,
    FileMetrics, Language, Priority, TechnicalDebtReport,
};
use debtmap::debt::{
    patterns::{find_code_smells_with_suppression, find_todos_and_fixmes_with_suppression},
    suppression::parse_suppression_comments,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Analyze a code snippet directly using the library API
pub fn analyze_code_snippet(code: &str, language: Language) -> Result<FileMetrics> {
    let analyzer = get_analyzer(language);
    let path = PathBuf::from(match language {
        Language::Rust => "test.rs",
        Language::Python => "test.py",
        Language::JavaScript => "test.js",
        Language::TypeScript => "test.ts",
        _ => "test.txt",
    });

    analyze_file(code.to_string(), path, &*analyzer)
}

/// Analyze a file directly using the library API with full analysis pipeline
pub fn analyze_file_directly(file_path: &Path) -> Result<AnalysisResults> {
    // Read and analyze the file
    let content = std::fs::read_to_string(file_path)?;
    let language = detect_language(file_path);

    // Check if context-aware analysis is enabled via environment variable
    let context_aware = std::env::var("DEBTMAP_CONTEXT_AWARE")
        .map(|v| v == "true")
        .unwrap_or(false);

    let analyzer = get_analyzer_with_context(language, context_aware);

    // Parse and analyze the file
    let metrics = analyze_file(content.clone(), file_path.to_path_buf(), &*analyzer)?;

    // Parse suppression comments
    let suppression_comments = parse_suppression_comments(&content, language, file_path);

    // Find TODOs and FIXMEs
    let todos =
        find_todos_and_fixmes_with_suppression(&content, file_path, Some(&suppression_comments));

    // Find code smells
    let smells =
        find_code_smells_with_suppression(&content, file_path, Some(&suppression_comments));

    // Combine all debt items and add IDs
    let mut all_debt_items = Vec::new();
    for (i, mut item) in todos.into_iter().chain(smells.into_iter()).enumerate() {
        item.id = format!("debt_{}", i);
        all_debt_items.push(item);
    }

    // Create by_type map
    let mut by_type: HashMap<DebtType, Vec<DebtItem>> = HashMap::new();
    for item in &all_debt_items {
        by_type
            .entry(item.debt_type)
            .or_default()
            .push(item.clone());
    }

    // Extract priorities
    let priorities: Vec<Priority> = all_debt_items.iter().map(|item| item.priority).collect();

    // Create a simple ComplexityReport based on FileMetrics
    let complexity_report = ComplexityReport {
        metrics: vec![], // No function-level metrics available from FileMetrics
        summary: ComplexitySummary {
            total_functions: 0, // FileMetrics doesn't have functions list
            average_complexity: metrics.complexity.cyclomatic_complexity as f64,
            max_complexity: metrics.complexity.cyclomatic_complexity,
            high_complexity_count: if metrics.complexity.cyclomatic_complexity > 10 {
                1
            } else {
                0
            },
        },
    };

    // Create AnalysisResults
    Ok(AnalysisResults {
        project_path: file_path.parent().unwrap_or(Path::new(".")).to_path_buf(),
        timestamp: Utc::now(),
        complexity: complexity_report,
        technical_debt: TechnicalDebtReport {
            items: all_debt_items,
            by_type,
            priorities,
            duplications: vec![],
            file_contexts: std::collections::HashMap::new(),
        file_contexts: HashMap::new(),
        },
        dependencies: DependencyReport {
            modules: vec![],
            circular: vec![],
        },
        duplications: vec![],
            file_contexts: std::collections::HashMap::new(),
        file_contexts: HashMap::new(),
    })
}

/// Run a function with a timeout
pub fn run_with_timeout<F, T>(f: F, timeout: Duration) -> Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = f();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => Ok(result),
        Err(_) => Err(anyhow::anyhow!("Operation timed out after {:?}", timeout)),
    }
}

/// Perform unified analysis on results (simplified version for tests)
pub fn perform_unified_test_analysis(
    results: &AnalysisResults,
    _coverage_file: Option<&Path>,
) -> Result<Vec<(String, f64, DebtItem)>> {
    // Simplified version for tests - just create basic priorities
    let mut priorities = Vec::new();

    // Extract functions from complexity report if available
    for item in &results.technical_debt.items {
        let score = match item.priority {
            Priority::Critical => 10.0,
            Priority::High => 7.0,
            Priority::Medium => 4.0,
            Priority::Low => 1.0,
        };

        priorities.push((format!("item_{}", priorities.len()), score, item.clone()));
    }

    // Sort by score (highest first)
    priorities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    Ok(priorities)
}

fn detect_language(path: &Path) -> Language {
    match path.extension().and_then(|s| s.to_str()) {
        Some("rs") => Language::Rust,
        Some("py") => Language::Python,
        Some("js") => Language::JavaScript,
        Some("ts") => Language::TypeScript,
        _ => Language::Rust, // Default
    }
}

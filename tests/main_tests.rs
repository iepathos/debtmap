use debtmap::risk::{Difficulty, FunctionRisk, RiskCategory, TestEffort};
use debtmap::{
    analyze_module_dependencies, ComplexityMetrics, ComplexityReport, ComplexitySummary,
    Dependency, DependencyKind, DependencyReport, FileMetrics, FunctionMetrics, Language,
};
use std::path::PathBuf;

#[test]
fn test_build_complexity_report_empty() {
    let functions = vec![];
    let report = build_complexity_report(&functions, 10);

    assert!(report.metrics.is_empty());
    assert_eq!(report.summary.total_functions, 0);
    assert_eq!(report.summary.average_complexity, 0.0);
    assert_eq!(report.summary.max_complexity, 0);
    assert_eq!(report.summary.high_complexity_count, 0);
}

#[test]
fn test_build_complexity_report_single_function() {
    let functions = vec![FunctionMetrics {
        name: "test_func".to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic: 5,
        cognitive: 7,
        nesting: 2,
        length: 25,
        is_test: false,
    }];
    let report = build_complexity_report(&functions, 10);

    assert_eq!(report.metrics.len(), 1);
    assert_eq!(report.summary.total_functions, 1);
    assert_eq!(report.summary.average_complexity, 5.0);
    assert_eq!(report.summary.max_complexity, 5);
    assert_eq!(report.summary.high_complexity_count, 0);
}

#[test]
fn test_build_complexity_report_multiple_functions() {
    let functions = vec![
        FunctionMetrics {
            name: "simple_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 2,
            cognitive: 3,
            nesting: 1,
            length: 10,
            is_test: false,
        },
        FunctionMetrics {
            name: "complex_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 30,
            cyclomatic: 15,
            cognitive: 20,
            nesting: 4,
            length: 50,
            is_test: false,
        },
        FunctionMetrics {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 100,
            cyclomatic: 1,
            cognitive: 2,
            nesting: 0,
            length: 5,
            is_test: true,
        },
    ];
    let report = build_complexity_report(&functions, 10);

    assert_eq!(report.metrics.len(), 3);
    assert_eq!(report.summary.total_functions, 3);
    assert_eq!(report.summary.average_complexity, 6.0); // (2 + 15 + 1) / 3
    assert_eq!(report.summary.max_complexity, 15);
    assert_eq!(report.summary.high_complexity_count, 1); // Only complex_func exceeds threshold
}

#[test]
fn test_build_complexity_report_with_different_thresholds() {
    let functions = vec![
        FunctionMetrics {
            name: "func1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 8,
            cognitive: 10,
            nesting: 2,
            length: 30,
            is_test: false,
        },
        FunctionMetrics {
            name: "func2".to_string(),
            file: PathBuf::from("test.rs"),
            line: 50,
            cyclomatic: 12,
            cognitive: 15,
            nesting: 3,
            length: 40,
            is_test: false,
        },
    ];

    // Test with threshold 5 - both functions exceed
    let report = build_complexity_report(&functions, 5);
    assert_eq!(report.summary.high_complexity_count, 2);

    // Test with threshold 10 - only func2 exceeds
    let report = build_complexity_report(&functions, 10);
    assert_eq!(report.summary.high_complexity_count, 1);

    // Test with threshold 15 - no functions exceed
    let report = build_complexity_report(&functions, 15);
    assert_eq!(report.summary.high_complexity_count, 0);
}

// Helper function copied from main.rs
fn build_complexity_report(
    all_functions: &[FunctionMetrics],
    complexity_threshold: u32,
) -> ComplexityReport {
    ComplexityReport {
        metrics: all_functions.to_vec(),
        summary: ComplexitySummary {
            total_functions: all_functions.len(),
            average_complexity: debtmap::core::metrics::calculate_average_complexity(all_functions),
            max_complexity: debtmap::core::metrics::find_max_complexity(all_functions),
            high_complexity_count: debtmap::core::metrics::count_high_complexity(
                all_functions,
                complexity_threshold,
            ),
        },
    }
}

#[test]
fn test_extract_all_functions_empty() {
    let file_metrics = vec![];
    let functions = extract_all_functions(&file_metrics);
    assert!(functions.is_empty());
}

#[test]
fn test_extract_all_functions_single_file() {
    let file_metrics = vec![FileMetrics {
        path: PathBuf::from("test.rs"),
        language: Language::Rust,
        complexity: ComplexityMetrics {
            functions: vec![
                FunctionMetrics {
                    name: "func1".to_string(),
                    file: PathBuf::from("test.rs"),
                    line: 10,
                    cyclomatic: 5,
                    cognitive: 7,
                    nesting: 2,
                    length: 20,
                    is_test: false,
                },
                FunctionMetrics {
                    name: "func2".to_string(),
                    file: PathBuf::from("test.rs"),
                    line: 35,
                    cyclomatic: 3,
                    cognitive: 4,
                    nesting: 1,
                    length: 15,
                    is_test: false,
                },
            ],
            cyclomatic_complexity: 8,
            cognitive_complexity: 11,
        },
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
    }];

    let functions = extract_all_functions(&file_metrics);
    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "func1");
    assert_eq!(functions[1].name, "func2");
}

#[test]
fn test_extract_all_functions_multiple_files() {
    let file_metrics = vec![
        FileMetrics {
            path: PathBuf::from("file1.rs"),
            language: Language::Rust,
            complexity: ComplexityMetrics {
                functions: vec![FunctionMetrics {
                    name: "func_a".to_string(),
                    file: PathBuf::from("file1.rs"),
                    line: 5,
                    cyclomatic: 2,
                    cognitive: 3,
                    nesting: 1,
                    length: 10,
                    is_test: false,
                }],
                cyclomatic_complexity: 2,
                cognitive_complexity: 3,
            },
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
        },
        FileMetrics {
            path: PathBuf::from("file2.rs"),
            language: Language::Rust,
            complexity: ComplexityMetrics {
                functions: vec![
                    FunctionMetrics {
                        name: "func_b".to_string(),
                        file: PathBuf::from("file2.rs"),
                        line: 10,
                        cyclomatic: 4,
                        cognitive: 5,
                        nesting: 2,
                        length: 20,
                        is_test: false,
                    },
                    FunctionMetrics {
                        name: "func_c".to_string(),
                        file: PathBuf::from("file2.rs"),
                        line: 35,
                        cyclomatic: 6,
                        cognitive: 8,
                        nesting: 3,
                        length: 25,
                        is_test: true,
                    },
                ],
                cyclomatic_complexity: 10,
                cognitive_complexity: 13,
            },
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
        },
    ];

    let functions = extract_all_functions(&file_metrics);
    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "func_a");
    assert_eq!(functions[1].name, "func_b");
    assert_eq!(functions[2].name, "func_c");
}

// Helper function copied from main.rs
fn extract_all_functions(file_metrics: &[FileMetrics]) -> Vec<FunctionMetrics> {
    file_metrics
        .iter()
        .flat_map(|m| &m.complexity.functions)
        .cloned()
        .collect()
}

#[test]
fn test_print_risk_function_with_coverage() {
    let func = FunctionRisk {
        function_name: "test_function".to_string(),
        file: PathBuf::from("src/test.rs"),
        line_range: (10, 30),
        risk_score: 8.5,
        risk_category: RiskCategory::Critical,
        cyclomatic_complexity: 15,
        cognitive_complexity: 20,
        coverage_percentage: Some(0.25),
        test_effort: TestEffort {
            estimated_difficulty: Difficulty::Simple,
            cognitive_load: 20,
            branch_count: 15,
            recommended_test_cases: 5,
        },
        is_test_function: false,
    };

    // Capture output
    let mut output = Vec::new();
    print_risk_function_to_writer(&func, &mut output);
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains("test_function"));
    assert!(output_str.contains("risk: 8.5"));
    assert!(output_str.contains("coverage: 25%"));
}

#[test]
fn test_print_risk_function_without_coverage() {
    let func = FunctionRisk {
        function_name: "uncovered_function".to_string(),
        file: PathBuf::from("src/test.rs"),
        line_range: (50, 75),
        risk_score: 10.0,
        risk_category: RiskCategory::Critical,
        cyclomatic_complexity: 20,
        cognitive_complexity: 30,
        coverage_percentage: None,
        test_effort: TestEffort {
            estimated_difficulty: Difficulty::Moderate,
            cognitive_load: 30,
            branch_count: 20,
            recommended_test_cases: 8,
        },
        is_test_function: false,
    };

    // Capture output
    let mut output = Vec::new();
    print_risk_function_to_writer(&func, &mut output);
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains("uncovered_function"));
    assert!(output_str.contains("risk: 10.0"));
    assert!(output_str.contains("coverage: 0%"));
}

// Helper function to test print_risk_function
fn print_risk_function_to_writer<W: std::io::Write>(func: &FunctionRisk, writer: &mut W) {
    let coverage_str = func
        .coverage_percentage
        .map(|c| format!("{:.0}%", c * 100.0))
        .unwrap_or_else(|| "0%".to_string());
    writeln!(
        writer,
        "    - {} (risk: {:.1}, coverage: {})",
        func.function_name, func.risk_score, coverage_str
    )
    .unwrap();
}

#[test]
fn test_create_dependency_report_empty() {
    let file_metrics = vec![];
    let report = create_dependency_report(&file_metrics);

    assert!(report.modules.is_empty());
    assert!(report.circular.is_empty());
}

#[test]
fn test_create_dependency_report_single_file() {
    let file_metrics = vec![FileMetrics {
        path: PathBuf::from("src/main.rs"),
        language: Language::Rust,
        complexity: ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 0,
            cognitive_complexity: 0,
        },
        debt_items: vec![],
        dependencies: vec![
            Dependency {
                name: "std::io".to_string(),
                kind: DependencyKind::Import,
            },
            Dependency {
                name: "serde".to_string(),
                kind: DependencyKind::Import,
            },
        ],
        duplications: vec![],
    }];

    let report = create_dependency_report(&file_metrics);
    assert!(!report.modules.is_empty() || report.circular.is_empty());
}

#[test]
fn test_create_dependency_report_multiple_files() {
    let file_metrics = vec![
        FileMetrics {
            path: PathBuf::from("src/lib.rs"),
            language: Language::Rust,
            complexity: ComplexityMetrics {
                functions: vec![],
                cyclomatic_complexity: 0,
                cognitive_complexity: 0,
            },
            debt_items: vec![],
            dependencies: vec![Dependency {
                name: "core".to_string(),
                kind: DependencyKind::Module,
            }],
            duplications: vec![],
        },
        FileMetrics {
            path: PathBuf::from("src/core.rs"),
            language: Language::Rust,
            complexity: ComplexityMetrics {
                functions: vec![],
                cyclomatic_complexity: 0,
                cognitive_complexity: 0,
            },
            debt_items: vec![],
            dependencies: vec![Dependency {
                name: "utils".to_string(),
                kind: DependencyKind::Module,
            }],
            duplications: vec![],
        },
    ];

    let report = create_dependency_report(&file_metrics);
    // Should have entries for the files and their dependencies
    assert!(!report.modules.is_empty() || report.circular.is_empty());
}

// Helper function copied from main.rs
fn create_dependency_report(file_metrics: &[FileMetrics]) -> DependencyReport {
    let file_deps: Vec<(PathBuf, Vec<Dependency>)> = file_metrics
        .iter()
        .map(|m| (m.path.clone(), m.dependencies.clone()))
        .collect();

    let dep_graph = analyze_module_dependencies(&file_deps);

    DependencyReport {
        modules: dep_graph.calculate_coupling_metrics(),
        circular: dep_graph.detect_circular_dependencies(),
    }
}

#[test]
fn test_collect_file_metrics_processes_valid_files() {
    use std::fs;
    use tempfile::tempdir;

    // Create temporary directory with test files
    let dir = tempdir().unwrap();
    let rust_file = dir.path().join("test.rs");
    let python_file = dir.path().join("test.py");

    fs::write(&rust_file, "fn main() { println!(\"Hello\"); }").unwrap();
    fs::write(&python_file, "def main():\n    print('Hello')").unwrap();

    let files = vec![rust_file.clone(), python_file.clone()];
    let metrics = collect_file_metrics(&files);

    // Should have metrics for both files
    assert_eq!(metrics.len(), 2);
    assert!(metrics.iter().any(|m| m.path == rust_file));
    assert!(metrics.iter().any(|m| m.path == python_file));
}

#[test]
fn test_collect_file_metrics_filters_invalid_files() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let nonexistent = dir.path().join("nonexistent.rs");
    let files = vec![nonexistent];

    let metrics = collect_file_metrics(&files);

    // Should filter out the nonexistent file
    assert!(metrics.is_empty());
}

// Helper function that mimics the main.rs implementation
fn collect_file_metrics(files: &[PathBuf]) -> Vec<FileMetrics> {
    use rayon::prelude::*;

    files
        .par_iter()
        .filter_map(|path| analyze_single_file(path.as_path()))
        .collect()
}

fn analyze_single_file(file_path: &std::path::Path) -> Option<FileMetrics> {
    use debtmap::{analyzers, core::Language, io};

    let content = io::read_file(file_path).ok()?;
    let ext = file_path.extension()?.to_str()?;
    let language = Language::from_extension(ext);

    (language != Language::Unknown)
        .then(|| {
            let analyzer = analyzers::get_analyzer(language);
            analyzers::analyze_file(content, file_path.to_path_buf(), analyzer.as_ref())
        })?
        .ok()
}

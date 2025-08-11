use crate::core::{
    ComplexityReport, ComplexitySummary, DebtItem, DependencyReport, DuplicationBlock, FileMetrics,
    FunctionMetrics, TechnicalDebtReport,
};
use crate::debt;
use crate::debt::circular::analyze_module_dependencies;
use crate::{analyzers, core::Language, io};
use rayon::prelude::*;
use std::path::{Path, PathBuf};

pub fn collect_file_metrics(files: &[PathBuf]) -> Vec<FileMetrics> {
    files
        .par_iter()
        .filter_map(|path| analyze_single_file(path.as_path()))
        .collect()
}

pub fn extract_all_functions(file_metrics: &[FileMetrics]) -> Vec<FunctionMetrics> {
    file_metrics
        .iter()
        .flat_map(|m| &m.complexity.functions)
        .cloned()
        .collect()
}

pub fn extract_all_debt_items(file_metrics: &[FileMetrics]) -> Vec<DebtItem> {
    file_metrics
        .iter()
        .flat_map(|m| &m.debt_items)
        .cloned()
        .collect()
}

pub fn build_complexity_report(
    all_functions: &[FunctionMetrics],
    complexity_threshold: u32,
) -> ComplexityReport {
    ComplexityReport {
        metrics: all_functions.to_vec(),
        summary: ComplexitySummary {
            total_functions: all_functions.len(),
            average_complexity: crate::core::metrics::calculate_average_complexity(all_functions),
            max_complexity: crate::core::metrics::find_max_complexity(all_functions),
            high_complexity_count: crate::core::metrics::count_high_complexity(
                all_functions,
                complexity_threshold,
            ),
        },
    }
}

pub fn build_technical_debt_report(
    all_debt_items: Vec<DebtItem>,
    duplications: Vec<DuplicationBlock>,
) -> TechnicalDebtReport {
    let debt_by_type = debt::categorize_debt(all_debt_items.clone());
    let priorities = debt::prioritize_debt(all_debt_items.clone())
        .into_iter()
        .map(|item| item.priority)
        .collect();

    TechnicalDebtReport {
        items: all_debt_items,
        by_type: debt_by_type,
        priorities,
        duplications,
    }
}

pub fn create_dependency_report(file_metrics: &[FileMetrics]) -> DependencyReport {
    let file_deps: Vec<(PathBuf, Vec<crate::core::Dependency>)> = file_metrics
        .iter()
        .map(|m| (m.path.clone(), m.dependencies.clone()))
        .collect();

    let dep_graph = analyze_module_dependencies(&file_deps);

    DependencyReport {
        modules: dep_graph.calculate_coupling_metrics(),
        circular: dep_graph.detect_circular_dependencies(),
    }
}

pub fn analyze_single_file(file_path: &Path) -> Option<FileMetrics> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ComplexityMetrics, Dependency, DependencyKind};

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
    fn test_extract_all_functions_empty() {
        let file_metrics = vec![];
        let functions = extract_all_functions(&file_metrics);
        assert!(functions.is_empty());
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

    #[test]
    fn test_create_dependency_report_empty() {
        let file_metrics = vec![];
        let report = create_dependency_report(&file_metrics);

        assert!(report.modules.is_empty());
        assert!(report.circular.is_empty());
    }

    #[test]
    fn test_create_dependency_report_with_dependencies() {
        let file_metrics = vec![FileMetrics {
            path: PathBuf::from("src/main.rs"),
            language: Language::Rust,
            complexity: ComplexityMetrics::default(),
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
}

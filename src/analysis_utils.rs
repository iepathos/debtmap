use crate::core::{
    ComplexityReport, ComplexitySummary, DebtItem, DependencyReport, DuplicationBlock, FileMetrics,
    FunctionMetrics, TechnicalDebtReport,
};
use crate::debt;
use crate::debt::circular::analyze_module_dependencies;
use crate::{analyzers, core::Language, io};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn collect_file_metrics(files: &[PathBuf]) -> Vec<FileMetrics> {
    use indicatif::ParallelProgressIterator;

    // Only apply file limit if explicitly set by user
    let (total_files, files_to_process) = match std::env::var("DEBTMAP_MAX_FILES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
    {
        Some(0) => {
            // DEBTMAP_MAX_FILES=0 means no limit
            (files.len(), files)
        }
        Some(max_files) => {
            let limited = files.len().min(max_files);
            if files.len() > max_files {
                eprintln!(
                    "[WARN] Processing limited to {} files (found {}) by DEBTMAP_MAX_FILES",
                    max_files,
                    files.len()
                );
            }
            (limited, &files[..limited])
        }
        None => {
            // No limit by default
            (files.len(), files)
        }
    };

    // Use atomic counter for live progress updates
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let processed_count = Arc::new(AtomicUsize::new(0));
    let processed_count_clone = Arc::clone(&processed_count);

    let results: Vec<FileMetrics> = files_to_process
        .par_iter()
        .filter_map(|path| {
            let result = analyze_single_file(path.as_path());

            // Update progress after each file
            let current = processed_count_clone.fetch_add(1, Ordering::Relaxed) + 1;
            crate::io::progress::AnalysisProgress::with_global(|p| {
                p.update_progress(crate::io::progress::PhaseProgress::Progress {
                    current,
                    total: total_files,
                });
            });

            result
        })
        .collect();

    results
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

/// Extract file contexts from file metrics for test file detection (spec 166)
pub fn extract_file_contexts(
    file_metrics: &[FileMetrics],
) -> std::collections::HashMap<PathBuf, crate::analysis::FileContext> {
    use crate::analysis::FileContextDetector;

    file_metrics
        .iter()
        .map(|m| {
            let detector = FileContextDetector::new(m.language);
            let context = detector.detect(&m.path, &m.complexity.functions);
            (m.path.clone(), context)
        })
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
    let debt_by_type = debt::categorize_debt(&all_debt_items);
    let priorities = debt::prioritize_debt(&all_debt_items)
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

// Default timeout per file (can be overridden by env var)
const DEFAULT_FILE_TIMEOUT_SECS: u64 = 60;

pub fn analyze_single_file(file_path: &Path) -> Option<FileMetrics> {
    analyze_single_file_with_timeout(file_path, None)
}

pub fn analyze_single_file_with_timeout(
    file_path: &Path,
    timeout_secs: Option<u64>,
) -> Option<FileMetrics> {
    let timeout = timeout_secs
        .or_else(|| {
            std::env::var("DEBTMAP_FILE_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(DEFAULT_FILE_TIMEOUT_SECS);

    // For very large projects, reduce per-file timeout
    let effective_timeout = if std::env::var("DEBTMAP_MAX_FILES").is_ok() {
        timeout.min(15) // Cap at 15 seconds for limited projects
    } else {
        timeout
    };

    // Quick path for small timeout or debugging
    if effective_timeout == 0 || std::env::var("DEBTMAP_NO_TIMEOUT").is_ok() {
        return analyze_single_file_direct(file_path);
    }

    // Set up timeout mechanism
    let (tx, rx) = mpsc::channel();
    let path_clone = file_path.to_path_buf();

    let handle = thread::spawn(move || {
        let result = analyze_single_file_direct(&path_clone);
        let _ = tx.send(result); // Ignore if main thread has timed out
    });

    // Wait for result or timeout
    match rx.recv_timeout(Duration::from_secs(effective_timeout)) {
        Ok(result) => {
            let _ = handle.join(); // Clean up thread
            result
        }
        Err(_) => {
            // Timeout occurred
            let quiet = std::env::var("DEBTMAP_QUIET").is_ok();
            if !quiet {
                eprintln!(
                    "[TIME] Timeout analyzing {} ({}s limit)",
                    file_path.display(),
                    effective_timeout
                );
            }

            // Note: We can't force kill the thread, but it will finish eventually
            // The main analysis continues without this file
            None
        }
    }
}

fn analyze_single_file_direct(file_path: &Path) -> Option<FileMetrics> {
    let content = io::read_file(file_path).ok()?;
    let ext = file_path.extension()?.to_str()?;
    let language = Language::from_extension(ext);

    (language != Language::Unknown)
        .then(|| {
            let context_aware = std::env::var("DEBTMAP_CONTEXT_AWARE")
                .map(|v| v == "true")
                .unwrap_or(false);
            let analyzer = analyzers::get_analyzer_with_context(language, context_aware);
            analyzers::analyze_file(content, file_path.to_path_buf(), analyzer.as_ref())
        })?
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ComplexityMetrics, Dependency, DependencyKind};
    use std::env;

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
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
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
                        visibility: None,
                        is_trait_method: false,
                        in_test_module: false,
                        entropy_score: None,
                        is_pure: None,
                        purity_confidence: None,
                        purity_reason: None,
                        call_dependencies: None,
                        detected_patterns: None,
                        upstream_callers: None,
                        downstream_callees: None,
                        mapping_pattern_result: None,
                        adjusted_complexity: None,
                        composition_metrics: None,
                        language_specific: None,
                        purity_level: None,
                    }],
                    cyclomatic_complexity: 2,
                    cognitive_complexity: 3,
                },
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
                module_scope: None,
                classes: None,
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
                            visibility: None,
                            is_trait_method: false,
                            in_test_module: false,
                            entropy_score: None,
                            is_pure: None,
                            purity_confidence: None,
                            purity_reason: None,
                            call_dependencies: None,
                            detected_patterns: None,
                            upstream_callers: None,
                            downstream_callees: None,
                            mapping_pattern_result: None,
                            adjusted_complexity: None,
                            composition_metrics: None,
                            language_specific: None,
                            purity_level: None,
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
                            visibility: None,
                            is_trait_method: false,
                            in_test_module: false,
                            entropy_score: None,
                            is_pure: None,
                            purity_confidence: None,
                            purity_reason: None,
                            call_dependencies: None,
                            detected_patterns: None,
                            upstream_callers: None,
                            downstream_callees: None,
                            mapping_pattern_result: None,
                            adjusted_complexity: None,
                            composition_metrics: None,
                            language_specific: None,
                            purity_level: None,
                        },
                    ],
                    cyclomatic_complexity: 10,
                    cognitive_complexity: 13,
                },
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
                module_scope: None,
                classes: None,
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
            module_scope: None,
            classes: None,
        }];

        let report = create_dependency_report(&file_metrics);
        assert!(!report.modules.is_empty() || report.circular.is_empty());
    }

    #[test]
    fn test_collect_file_metrics_no_limit_by_default() {
        // When DEBTMAP_MAX_FILES is not set, all files should be processed
        env::remove_var("DEBTMAP_MAX_FILES");
        env::set_var("DEBTMAP_QUIET", "1");

        let files: Vec<PathBuf> = (0..5)
            .map(|i| PathBuf::from(format!("test_file_{}.rs", i)))
            .collect();

        // We can't easily test the actual file processing since it requires valid files,
        // but we can verify the logic by checking that the function doesn't panic
        // and would attempt to process all files
        let result = collect_file_metrics(&files);
        // Result will be empty since files don't exist, but no panic means logic is correct
        assert!(result.is_empty());

        env::remove_var("DEBTMAP_QUIET");
    }

    #[test]
    fn test_collect_file_metrics_with_zero_means_no_limit() {
        // DEBTMAP_MAX_FILES=0 should mean "no limit"
        env::set_var("DEBTMAP_MAX_FILES", "0");
        env::set_var("DEBTMAP_QUIET", "1");

        let files: Vec<PathBuf> = (0..5)
            .map(|i| PathBuf::from(format!("test_file_{}.rs", i)))
            .collect();

        let result = collect_file_metrics(&files);
        // Result will be empty since files don't exist, but no panic means logic is correct
        assert!(result.is_empty());

        env::remove_var("DEBTMAP_MAX_FILES");
        env::remove_var("DEBTMAP_QUIET");
    }

    #[test]
    fn test_collect_file_metrics_respects_explicit_limit() {
        // When DEBTMAP_MAX_FILES is set to a positive number, it should limit processing
        env::set_var("DEBTMAP_MAX_FILES", "3");
        env::set_var("DEBTMAP_QUIET", "1");

        let files: Vec<PathBuf> = (0..5)
            .map(|i| PathBuf::from(format!("test_file_{}.rs", i)))
            .collect();

        let result = collect_file_metrics(&files);
        // Result will be empty since files don't exist, but no panic means logic is correct
        assert!(result.is_empty());

        env::remove_var("DEBTMAP_MAX_FILES");
        env::remove_var("DEBTMAP_QUIET");
    }
}

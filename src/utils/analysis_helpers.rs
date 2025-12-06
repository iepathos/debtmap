use crate::{
    analysis_utils, config,
    core::{
        AnalysisResults, ComplexityReport, DependencyReport, DuplicationBlock, FileMetrics,
        FunctionMetrics, Language, TechnicalDebtReport,
    },
    debt, io,
};
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};

const DEFAULT_SIMILARITY_THRESHOLD: f64 = 0.8;

pub fn analyze_project(
    path: PathBuf,
    languages: Vec<Language>,
    complexity_threshold: u32,
    duplication_threshold: usize,
) -> Result<AnalysisResults> {
    let config = config::get_config();
    let files = io::walker::find_project_files_with_config(&path, languages.clone(), config)
        .context("Failed to find project files")?;

    let file_metrics = analysis_utils::collect_file_metrics(&files);
    let all_functions = analysis_utils::extract_all_functions(&file_metrics);
    let all_debt_items = analysis_utils::extract_all_debt_items(&file_metrics);
    let duplications = detect_duplications(&files, duplication_threshold);
    let file_contexts = analysis_utils::extract_file_contexts(&file_metrics);

    let complexity_report = build_complexity_report(&all_functions, complexity_threshold);
    let technical_debt = build_technical_debt_report(all_debt_items, duplications.clone());
    let dependencies = create_dependency_report(&file_metrics);

    Ok(AnalysisResults {
        project_path: path,
        timestamp: Utc::now(),
        complexity: complexity_report,
        technical_debt,
        dependencies,
        duplications,
        file_contexts,
    })
}

/// Detect duplications with progress callback for TUI updates
pub fn detect_duplications_with_progress<F>(
    files: &[PathBuf],
    threshold: usize,
    mut progress_callback: F,
) -> Vec<DuplicationBlock>
where
    F: FnMut(usize, usize),
{
    let total_files = files.len();
    let mut last_update = std::time::Instant::now();

    // Prepare files with progress tracking
    let files_with_content: Vec<(PathBuf, String)> = files
        .iter()
        .enumerate()
        .filter_map(|(idx, path)| {
            let result = match io::read_file(path) {
                Ok(content) => Some((path.clone(), content)),
                Err(e) => {
                    log::debug!(
                        "Skipping file {} for duplication check: {}",
                        path.display(),
                        e
                    );
                    None
                }
            };

            // Throttled progress updates (every 10 files or 100ms)
            if (idx + 1) % 10 == 0 || last_update.elapsed() > std::time::Duration::from_millis(100)
            {
                progress_callback(idx + 1, total_files);
                last_update = std::time::Instant::now();
            }

            result
        })
        .collect();

    // Final progress update
    progress_callback(total_files, total_files);

    debt::duplication::detect_duplication(
        files_with_content,
        threshold,
        DEFAULT_SIMILARITY_THRESHOLD,
    )
}

/// Detect duplications without progress tracking (compatibility wrapper)
pub fn detect_duplications(files: &[PathBuf], threshold: usize) -> Vec<DuplicationBlock> {
    detect_duplications_with_progress(files, threshold, |_, _| {})
}

pub fn prepare_files_for_duplication_check(files: &[PathBuf]) -> Vec<(PathBuf, String)> {
    files
        .iter()
        .filter_map(|path| match io::read_file(path) {
            Ok(content) => Some((path.clone(), content)),
            Err(e) => {
                log::debug!(
                    "Skipping file {} for duplication check: {}",
                    path.display(),
                    e
                );
                None
            }
        })
        .collect()
}

pub fn build_complexity_report(
    all_functions: &[FunctionMetrics],
    complexity_threshold: u32,
) -> ComplexityReport {
    analysis_utils::build_complexity_report(all_functions, complexity_threshold)
}

pub fn build_technical_debt_report(
    all_debt_items: Vec<crate::core::DebtItem>,
    duplications: Vec<DuplicationBlock>,
) -> TechnicalDebtReport {
    analysis_utils::build_technical_debt_report(all_debt_items, duplications)
}

pub fn create_dependency_report(file_metrics: &[FileMetrics]) -> DependencyReport {
    analysis_utils::create_dependency_report(file_metrics)
}

/// Check if a file path is within the current project directory.
///
/// Returns false for files outside the project (e.g., in sibling directories like ../prodigy).
///
/// This is used to filter parsing warnings - we only show warnings for files in the current
/// project to avoid cluttering the TUI with errors from external codebases. External files
/// may appear in the analysis due to cross-project references or symlinks.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use debtmap::utils::is_in_current_project;
///
/// // Files in the current project
/// assert!(is_in_current_project(Path::new("src/lib.rs")));
/// assert!(is_in_current_project(Path::new("./tests/integration.rs")));
///
/// // Files outside the current project
/// assert!(!is_in_current_project(Path::new("../other-project/src/lib.rs")));
/// ```
pub fn is_in_current_project(path: &Path) -> bool {
    // Quick check: if path starts with "../", it's clearly outside
    if path.starts_with("..") {
        return false;
    }

    // If it's already an absolute path, check if it's under current directory
    if path.is_absolute() {
        if let Ok(current_dir) = std::env::current_dir() {
            if let (Ok(canonical_path), Ok(canonical_cwd)) =
                (path.canonicalize(), current_dir.canonicalize())
            {
                return canonical_path.starts_with(canonical_cwd);
            }
        }
        // If we can't determine, assume it's external for safety
        return false;
    }

    // Relative paths that don't start with ".." are assumed to be in project
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_in_current_project_relative_path() {
        let path = Path::new("src/lib.rs");
        assert!(is_in_current_project(path));
    }

    #[test]
    fn test_is_in_current_project_parent_directory() {
        let path = Path::new("../other-project/file.rs");
        assert!(!is_in_current_project(path));
    }

    #[test]
    fn test_is_in_current_project_nested_parent() {
        let path = Path::new("../../another-project/src/file.rs");
        assert!(!is_in_current_project(path));
    }

    #[test]
    fn test_is_in_current_project_nested_relative() {
        let path = Path::new("src/subdir/nested/file.rs");
        assert!(is_in_current_project(path));
    }

    #[test]
    fn test_detect_duplications_with_progress_calls_callback() {
        use std::sync::{Arc, Mutex};

        // Create temporary test files
        let temp_dir = tempfile::tempdir().unwrap();
        let file1 = temp_dir.path().join("file1.rs");
        let file2 = temp_dir.path().join("file2.rs");
        let file3 = temp_dir.path().join("file3.rs");

        std::fs::write(&file1, "fn test() { println!(\"hello\"); }").unwrap();
        std::fs::write(&file2, "fn test() { println!(\"world\"); }").unwrap();
        std::fs::write(&file3, "fn test() { println!(\"foo\"); }").unwrap();

        let files = vec![file1, file2, file3];

        // Track progress callback invocations
        let progress_calls = Arc::new(Mutex::new(Vec::new()));
        let progress_calls_clone = progress_calls.clone();

        detect_duplications_with_progress(&files, 50, |current, total| {
            progress_calls_clone.lock().unwrap().push((current, total));
        });

        let calls = progress_calls.lock().unwrap();

        // Should have at least one call (final update)
        assert!(!calls.is_empty(), "Progress callback should be called");

        // Final call should report completion
        let final_call = calls.last().unwrap();
        assert_eq!(final_call.0, 3, "Final current should be total files");
        assert_eq!(final_call.1, 3, "Final total should be total files");
    }

    #[test]
    fn test_detect_duplications_with_progress_throttling() {
        use std::sync::{Arc, Mutex};

        // Create many temporary test files to test throttling
        let temp_dir = tempfile::tempdir().unwrap();
        let mut files = Vec::new();

        for i in 0..25 {
            let file = temp_dir.path().join(format!("file{}.rs", i));
            std::fs::write(&file, format!("fn test{}() {{}}", i)).unwrap();
            files.push(file);
        }

        // Track progress callback invocations
        let progress_calls = Arc::new(Mutex::new(Vec::new()));
        let progress_calls_clone = progress_calls.clone();

        detect_duplications_with_progress(&files, 50, |current, total| {
            progress_calls_clone.lock().unwrap().push((current, total));
        });

        let calls = progress_calls.lock().unwrap();

        // With 25 files and throttling every 10 files, we expect:
        // - Update at file 10
        // - Update at file 20
        // - Final update at file 25
        // (May have more if 100ms elapsed between files)
        assert!(
            calls.len() >= 3,
            "Should have at least 3 progress updates with throttling"
        );
        assert!(
            calls.len() <= 10,
            "Throttling should limit excessive updates"
        );

        // Verify counts are monotonically increasing
        for i in 1..calls.len() {
            assert!(
                calls[i].0 >= calls[i - 1].0,
                "Progress should be monotonically increasing"
            );
        }
    }

    #[test]
    fn test_detect_duplications_with_progress_correct_values() {
        use std::sync::{Arc, Mutex};

        // Create temporary test files
        let temp_dir = tempfile::tempdir().unwrap();
        let mut files = Vec::new();

        for i in 0..15 {
            let file = temp_dir.path().join(format!("file{}.rs", i));
            std::fs::write(&file, format!("fn test{}() {{}}", i)).unwrap();
            files.push(file);
        }

        let total_files = files.len();
        let progress_calls = Arc::new(Mutex::new(Vec::new()));
        let progress_calls_clone = progress_calls.clone();

        detect_duplications_with_progress(&files, 50, |current, total| {
            progress_calls_clone.lock().unwrap().push((current, total));
        });

        let calls = progress_calls.lock().unwrap();

        // All calls should report correct total
        for (current, total) in calls.iter() {
            assert_eq!(
                *total, total_files,
                "Total should always be {}",
                total_files
            );
            assert!(*current <= total_files, "Current should never exceed total");
            assert!(*current > 0, "Current should be positive");
        }
    }

    #[test]
    fn test_detect_duplications_without_progress_works() {
        // Test the compatibility wrapper
        let temp_dir = tempfile::tempdir().unwrap();
        let file1 = temp_dir.path().join("file1.rs");
        let file2 = temp_dir.path().join("file2.rs");

        std::fs::write(&file1, "fn test() { println!(\"hello\"); }").unwrap();
        std::fs::write(&file2, "fn test() { println!(\"hello\"); }").unwrap();

        let files = vec![file1, file2];

        // Should not panic or fail
        let _result = detect_duplications(&files, 50);
    }
}

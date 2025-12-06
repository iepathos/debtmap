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

pub fn detect_duplications(files: &[PathBuf], threshold: usize) -> Vec<DuplicationBlock> {
    let files_with_content = prepare_files_for_duplication_check(files);
    debt::duplication::detect_duplication(
        files_with_content,
        threshold,
        DEFAULT_SIMILARITY_THRESHOLD,
    )
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
}

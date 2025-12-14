//! Composed effect operations.
//!
//! This module provides higher-level composed effects that combine
//! multiple basic operations:
//! - Read file if exists (optional file reading)
//! - Read multiple files in sequence
//! - Walk and analyze directories
//! - Walk and validate directories
//!
//! # Design Philosophy
//!
//! These effects compose basic operations into higher-level workflows,
//! demonstrating the power of Effect-based composition for building
//! complex analysis pipelines from simple building blocks.

use crate::effects::AnalysisEffect;
use std::path::PathBuf;
use stillwater::effect::prelude::*;

use super::directory::walk_dir_with_config_effect;
use super::file::{file_exists_effect, read_file_effect};

/// Read a file only if it exists, returning None otherwise.
///
/// This is a composed effect that combines existence check with reading.
/// Useful when a file is optional.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::read_file_if_exists_effect;
///
/// let effect = read_file_if_exists_effect("optional.toml".into());
/// let content = run_effect(effect, config)?; // Ok(None) if file doesn't exist
/// ```
pub fn read_file_if_exists_effect(path: PathBuf) -> AnalysisEffect<Option<String>> {
    let path_clone = path.clone();
    file_exists_effect(path.clone())
        .and_then(move |exists| {
            if exists {
                read_file_effect(path_clone).map(Some).boxed()
            } else {
                pure(None).boxed()
            }
        })
        .boxed()
}

/// Read multiple files in sequence, collecting all contents.
///
/// If any file fails to read, the entire operation fails.
/// For parallel reading, use `read_files_parallel_effect`.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::read_files_effect;
///
/// let paths = vec!["src/main.rs".into(), "src/lib.rs".into()];
/// let effect = read_files_effect(paths);
/// let contents = run_effect(effect, config)?; // Vec<String>
/// ```
pub fn read_files_effect(paths: Vec<PathBuf>) -> AnalysisEffect<Vec<String>> {
    let mut effects: Vec<AnalysisEffect<String>> =
        paths.into_iter().map(read_file_effect).collect();

    // Sequence all effects
    if effects.is_empty() {
        return pure(Vec::new()).boxed();
    }

    let first = effects.remove(0);
    effects
        .into_iter()
        .fold(first.map(|s| vec![s]).boxed(), |acc, eff| {
            acc.and_then(move |mut results| {
                eff.map(move |s| {
                    results.push(s);
                    results
                })
                .boxed()
            })
            .boxed()
        })
}

/// Walk a directory and analyze all files in parallel.
///
/// This is a composed effect that combines directory walking with
/// parallel file analysis using stillwater's traverse pattern.
///
/// # Arguments
///
/// * `path` - Root directory to analyze
/// * `languages` - Languages to include in analysis
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::walk_and_analyze_effect;
/// use debtmap::core::Language;
///
/// let effect = walk_and_analyze_effect(
///     "src".into(),
///     vec![Language::Rust],
/// );
/// let results = run_effect(effect, config)?;
/// for result in results {
///     println!("{}: {} functions", result.path.display(),
///         result.metrics.complexity.functions.len());
/// }
/// ```
pub fn walk_and_analyze_effect(
    path: PathBuf,
    languages: Vec<crate::core::Language>,
) -> AnalysisEffect<Vec<crate::analyzers::batch::FileAnalysisResult>> {
    walk_dir_with_config_effect(path, languages)
        .and_then(crate::analyzers::batch::analyze_files_effect)
        .boxed()
}

/// Walk a directory and validate all files, accumulating errors.
///
/// This effect validates all discovered files and accumulates ALL
/// validation errors instead of failing at the first one.
///
/// # Arguments
///
/// * `path` - Root directory to validate
/// * `languages` - Languages to include
///
/// # Returns
///
/// An Effect that produces a Validation result with either:
/// - All successfully validated files
/// - All validation errors accumulated
pub fn walk_and_validate_effect(
    path: PathBuf,
    languages: Vec<crate::core::Language>,
) -> AnalysisEffect<crate::effects::AnalysisValidation<Vec<crate::analyzers::batch::ValidatedFile>>>
{
    walk_dir_with_config_effect(path, languages)
        .map(|files| crate::analyzers::batch::validate_files(&files))
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::effects::run_effect;
    use tempfile::TempDir;

    fn create_test_env() -> (TempDir, DebtmapConfig) {
        let temp_dir = TempDir::new().unwrap();
        (temp_dir, DebtmapConfig::default())
    }

    #[test]
    fn test_read_file_if_exists_effect() {
        let (temp_dir, config) = create_test_env();
        let file_path = temp_dir.path().join("optional.txt");
        std::fs::write(&file_path, "content").unwrap();

        // File exists
        let effect = read_file_if_exists_effect(file_path);
        let result = run_effect(effect, config.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("content".to_string()));

        // File doesn't exist
        let effect = read_file_if_exists_effect(temp_dir.path().join("missing.txt"));
        let result = run_effect(effect, config);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_read_files_effect() {
        let (temp_dir, config) = create_test_env();
        std::fs::write(temp_dir.path().join("a.txt"), "A").unwrap();
        std::fs::write(temp_dir.path().join("b.txt"), "B").unwrap();

        let paths = vec![temp_dir.path().join("a.txt"), temp_dir.path().join("b.txt")];
        let effect = read_files_effect(paths);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn test_read_files_effect_empty() {
        let config = DebtmapConfig::default();
        let effect = read_files_effect(vec![]);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}

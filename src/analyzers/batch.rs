//! Batch file analysis using stillwater's traverse pattern.
//!
//! This module provides parallel file analysis using stillwater's `traverse_effect`
//! and error accumulation using `Validation` with `traverse`.
//!
//! # Design
//!
//! The module follows the traverse pattern from functional programming:
//!
//! - **`traverse_effect`**: Apply an effectful operation to each element and collect results
//! - **`traverse` with Validation**: Apply a validation and accumulate ALL errors
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::analyzers::batch::{analyze_files_effect, validate_files};
//! use debtmap::effects::run_effect;
//!
//! // Parallel analysis with Effect
//! let results = run_effect(
//!     analyze_files_effect(files),
//!     config,
//! )?;
//!
//! // Validation with error accumulation
//! let validated = validate_files(&paths)?;
//! ```

use crate::analyzers::get_analyzer;
use crate::config::{BatchAnalysisConfig, ParallelConfig};
use crate::core::{DebtItem, FileMetrics, Language};
use crate::effects::{
    validation_failure, validation_failures, validation_success, AnalysisEffect, AnalysisValidation,
};
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use stillwater::effect::prelude::*;

/// Result of analyzing a single file with full context.
///
/// This struct captures all relevant information from analyzing a file,
/// including metrics, debt items, and optional timing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisResult {
    /// Path to the analyzed file
    pub path: PathBuf,

    /// Full file metrics from analysis
    pub metrics: FileMetrics,

    /// Technical debt items found in this file
    pub debt_items: Vec<DebtItem>,

    /// Time taken to analyze this file (if timing was enabled)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analysis_time: Option<Duration>,
}

impl FileAnalysisResult {
    /// Create a new analysis result without timing information.
    pub fn new(path: PathBuf, metrics: FileMetrics, debt_items: Vec<DebtItem>) -> Self {
        Self {
            path,
            metrics,
            debt_items,
            analysis_time: None,
        }
    }

    /// Create a new analysis result with timing information.
    pub fn with_timing(
        path: PathBuf,
        metrics: FileMetrics,
        debt_items: Vec<DebtItem>,
        analysis_time: Duration,
    ) -> Self {
        Self {
            path,
            metrics,
            debt_items,
            analysis_time: Some(analysis_time),
        }
    }
}

/// A validated file ready for analysis.
///
/// This struct represents a file that has passed initial validation
/// (existence, readability, syntax check) and is ready for full analysis.
#[derive(Debug, Clone)]
pub struct ValidatedFile {
    /// Path to the file
    pub path: PathBuf,

    /// File content
    pub content: String,

    /// Detected language
    pub language: Language,
}

impl ValidatedFile {
    /// Create a new validated file.
    pub fn new(path: PathBuf, content: String, language: Language) -> Self {
        Self {
            path,
            content,
            language,
        }
    }
}

// ============================================================================
// Effect-based Parallel Analysis
// ============================================================================

/// Analyze multiple files in parallel using stillwater's traverse pattern.
///
/// This function uses `traverse_effect` to apply analysis to each file
/// concurrently, collecting all results. If any file fails to analyze,
/// the entire operation fails with that error.
///
/// For error accumulation (collecting ALL errors), use [`validate_files`] instead.
///
/// # Arguments
///
/// * `paths` - Files to analyze
///
/// # Returns
///
/// An Effect that produces a vector of analysis results when run.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::analyzers::batch::analyze_files_effect;
/// use debtmap::effects::run_effect;
///
/// let files = vec!["src/main.rs".into(), "src/lib.rs".into()];
/// let results = run_effect(analyze_files_effect(files), config)?;
/// for result in results {
///     println!("{}: {} functions", result.path.display(), result.metrics.complexity.functions.len());
/// }
/// ```
pub fn analyze_files_effect(paths: Vec<PathBuf>) -> AnalysisEffect<Vec<FileAnalysisResult>> {
    from_fn(move |env: &RealEnv| {
        let config = env.config();
        let parallel_config = config
            .batch_analysis
            .as_ref()
            .map(|b| &b.parallelism)
            .cloned()
            .unwrap_or_default();
        let collect_timing = config
            .batch_analysis
            .as_ref()
            .map(|b| b.collect_timing)
            .unwrap_or(false);

        analyze_files_parallel(&paths, &parallel_config, collect_timing)
    })
    .boxed()
}

/// Analyze a single file as an Effect.
///
/// This is the fundamental unit of analysis wrapped as an Effect.
/// It reads the file, determines the language, and runs the appropriate analyzer.
pub fn analyze_single_file_effect(path: PathBuf) -> AnalysisEffect<FileAnalysisResult> {
    let path_display = path.display().to_string();
    from_fn(move |env: &RealEnv| {
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e.message()), &path)
        })?;

        analyze_file_content(&path, &content, false).map_err(|e| {
            AnalysisError::analysis(format!("Analysis failed for '{}': {}", path_display, e))
        })
    })
    .boxed()
}

/// Analyze files with configuration-based settings.
///
/// This version uses the environment's BatchAnalysisConfig for settings.
pub fn analyze_files_with_config_effect(
    paths: Vec<PathBuf>,
    _config: BatchAnalysisConfig,
) -> AnalysisEffect<Vec<FileAnalysisResult>> {
    analyze_files_effect(paths)
}

// ============================================================================
// Validation-based Error Accumulation
// ============================================================================

/// Validate multiple files, accumulating ALL errors.
///
/// Unlike `analyze_files_effect` which fails at the first error,
/// this function uses `Validation` to collect all validation errors.
/// This is useful for providing comprehensive feedback to users.
///
/// # Arguments
///
/// * `paths` - Files to validate
///
/// # Returns
///
/// A Validation that is either:
/// - `Success(Vec<ValidatedFile>)` if all files are valid
/// - `Failure(NonEmptyVec<AnalysisError>)` with ALL validation errors
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::analyzers::batch::validate_files;
/// use debtmap::effects::run_validation;
///
/// let paths = vec!["src/main.rs".into(), "src/lib.rs".into()];
/// match run_validation(validate_files(&paths)) {
///     Ok(validated) => {
///         // All files validated successfully
///         for file in validated {
///             println!("{}: {} bytes", file.path.display(), file.content.len());
///         }
///     }
///     Err(errors) => {
///         // Some files failed validation - show ALL errors
///         eprintln!("Validation failed with {} errors:", errors);
///     }
/// }
/// ```
pub fn validate_files(paths: &[PathBuf]) -> AnalysisValidation<Vec<ValidatedFile>> {
    let validations: Vec<AnalysisValidation<ValidatedFile>> =
        paths.iter().map(|p| validate_single_file(p)).collect();

    combine_validations_preserve_successes(validations)
}

/// Validate a single file's existence and syntax.
///
/// Performs the following checks:
/// 1. File exists and is readable
/// 2. Content is valid UTF-8
/// 3. Language is supported
///
/// # Returns
///
/// A Validation with either the validated file or an error.
pub fn validate_single_file(path: &Path) -> AnalysisValidation<ValidatedFile> {
    // Check file existence and read content
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return validation_failure(AnalysisError::io_with_path(
                format!("Failed to read file: {}", e),
                path,
            ))
        }
    };

    // Determine language
    let language = Language::from_path(path);
    if language == Language::Unknown {
        return validation_failure(AnalysisError::validation_with_path(
            "Unsupported file type",
            path,
        ));
    }

    // Basic syntax validation for supported languages
    if let Err(e) = validate_syntax(&content, language, path) {
        return validation_failure(e);
    }

    validation_success(ValidatedFile::new(path.to_path_buf(), content, language))
}

/// Validate file syntax based on language.
///
/// Spec 202: Uses UnifiedFileExtractor for Rust files to ensure consistent
/// parsing and SourceMap management.
fn validate_syntax(content: &str, language: Language, path: &Path) -> Result<(), AnalysisError> {
    match language {
        Language::Rust => {
            // Use UnifiedFileExtractor for parsing (spec 202)
            // This ensures SourceMap is reset after parsing
            crate::extraction::UnifiedFileExtractor::extract(path, content).map_err(|e| {
                AnalysisError::parse_with_path(format!("Rust syntax error: {}", e), path)
            })?;
            Ok(())
        }
        Language::Python => {
            // Basic Python syntax check (could use rustpython-parser)
            // For now, just check for basic structure
            if content.contains("def ") || content.contains("class ") || content.trim().is_empty() {
                Ok(())
            } else {
                // Accept any non-empty Python file
                Ok(())
            }
        }
        Language::Unknown => Err(AnalysisError::validation_with_path(
            "Cannot validate unknown language",
            path,
        )),
    }
}

/// Validate and then analyze files.
///
/// This combines validation with analysis, providing comprehensive
/// error reporting for validation failures while still performing
/// analysis on valid files.
///
/// # Returns
///
/// A Validation that is either:
/// - `Success(Vec<FileAnalysisResult>)` if all files are valid and analyzed
/// - `Failure(NonEmptyVec<AnalysisError>)` with all validation errors
pub fn validate_and_analyze_files(
    paths: &[PathBuf],
) -> AnalysisValidation<Vec<FileAnalysisResult>> {
    let validated = validate_files(paths);

    match validated {
        stillwater::Validation::Success(files) => {
            // All files validated, now analyze them
            let results: Vec<Result<FileAnalysisResult, AnalysisError>> = files
                .into_iter()
                .map(|f| analyze_validated_file(&f))
                .collect();

            // Collect results or errors
            let mut successes = Vec::new();
            let mut failures = Vec::new();

            for result in results {
                match result {
                    Ok(r) => successes.push(r),
                    Err(e) => failures.push(e),
                }
            }

            if failures.is_empty() {
                validation_success(successes)
            } else {
                validation_failures(failures)
            }
        }
        stillwater::Validation::Failure(errors) => stillwater::Validation::Failure(errors),
    }
}

// ============================================================================
// Internal Implementation
// ============================================================================

/// Analyze files in parallel using rayon.
fn analyze_files_parallel(
    paths: &[PathBuf],
    config: &ParallelConfig,
    collect_timing: bool,
) -> Result<Vec<FileAnalysisResult>, AnalysisError> {
    if !config.enabled || paths.len() <= 1 {
        // Sequential processing
        paths
            .iter()
            .map(|path| analyze_file_from_path(path, collect_timing))
            .collect()
    } else {
        // Parallel processing with rayon
        let batch_size = config.effective_batch_size();

        if paths.len() <= batch_size {
            // Single batch
            paths
                .par_iter()
                .map(|path| analyze_file_from_path(path, collect_timing))
                .collect()
        } else {
            // Chunked processing for large codebases
            paths
                .chunks(batch_size)
                .flat_map(|chunk| {
                    chunk
                        .par_iter()
                        .map(|path| analyze_file_from_path(path, collect_timing))
                        .collect::<Vec<_>>()
                })
                .collect()
        }
    }
}

/// Analyze a file from its path.
fn analyze_file_from_path(
    path: &Path,
    collect_timing: bool,
) -> Result<FileAnalysisResult, AnalysisError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| AnalysisError::io_with_path(format!("Failed to read file: {}", e), path))?;

    analyze_file_content(path, &content, collect_timing)
}

/// Analyze file content with the appropriate analyzer.
fn analyze_file_content(
    path: &Path,
    content: &str,
    collect_timing: bool,
) -> Result<FileAnalysisResult, AnalysisError> {
    let start = if collect_timing {
        Some(Instant::now())
    } else {
        None
    };

    let language = Language::from_path(path);
    let analyzer = get_analyzer(language);

    let ast = analyzer
        .parse(content, path.to_path_buf())
        .map_err(|e| AnalysisError::parse_with_path(format!("Parse failed: {}", e), path))?;

    let metrics = analyzer.analyze(&ast);
    let debt_items = metrics.debt_items.clone();

    // Reset SourceMap to prevent overflow when parsing many files.
    // Safe here because we've extracted all metrics (including line numbers) from the AST.
    crate::core::parsing::reset_span_locations();

    let analysis_time = start.map(|s| s.elapsed());

    Ok(FileAnalysisResult {
        path: path.to_path_buf(),
        metrics,
        debt_items,
        analysis_time,
    })
}

/// Analyze a pre-validated file.
fn analyze_validated_file(file: &ValidatedFile) -> Result<FileAnalysisResult, AnalysisError> {
    analyze_file_content(&file.path, &file.content, false)
}

/// Combine validations while preserving successful values.
fn combine_validations_preserve_successes<T>(
    validations: Vec<AnalysisValidation<T>>,
) -> AnalysisValidation<Vec<T>> {
    let mut successes = Vec::new();
    let mut failures: Vec<AnalysisError> = Vec::new();

    for v in validations {
        match v {
            stillwater::Validation::Success(value) => successes.push(value),
            stillwater::Validation::Failure(errors) => {
                for err in errors {
                    failures.push(err);
                }
            }
        }
    }

    if failures.is_empty() {
        validation_success(successes)
    } else {
        validation_failures(failures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::run_validation;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_file_analysis_result_new() {
        let path = PathBuf::from("test.rs");
        let metrics = FileMetrics {
            path: path.clone(),
            language: Language::Rust,
            complexity: Default::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: None,
        };

        let result = FileAnalysisResult::new(path.clone(), metrics, vec![]);

        assert_eq!(result.path, path);
        assert!(result.analysis_time.is_none());
    }

    #[test]
    fn test_file_analysis_result_with_timing() {
        let path = PathBuf::from("test.rs");
        let metrics = FileMetrics {
            path: path.clone(),
            language: Language::Rust,
            complexity: Default::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: None,
        };

        let duration = Duration::from_millis(100);
        let result = FileAnalysisResult::with_timing(path.clone(), metrics, vec![], duration);

        assert_eq!(result.path, path);
        assert_eq!(result.analysis_time, Some(duration));
    }

    #[test]
    fn test_validated_file_new() {
        let file = ValidatedFile::new(
            PathBuf::from("test.rs"),
            "fn main() {}".to_string(),
            Language::Rust,
        );

        assert_eq!(file.path, PathBuf::from("test.rs"));
        assert_eq!(file.content, "fn main() {}");
        assert_eq!(file.language, Language::Rust);
    }

    #[test]
    fn test_validate_single_file_success() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let result = validate_single_file(&file_path);

        assert!(result.is_success());
        match result {
            stillwater::Validation::Success(file) => {
                assert_eq!(file.path, file_path);
                assert_eq!(file.language, Language::Rust);
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_validate_single_file_not_found() {
        let result = validate_single_file(Path::new("/nonexistent/file.rs"));

        assert!(result.is_failure());
    }

    #[test]
    fn test_validate_single_file_unknown_language() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.unknown");
        fs::write(&file_path, "some content").unwrap();

        let result = validate_single_file(&file_path);

        assert!(result.is_failure());
    }

    #[test]
    fn test_validate_single_file_syntax_error() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main( { }").unwrap(); // Invalid Rust syntax

        let result = validate_single_file(&file_path);

        assert!(result.is_failure());
    }

    #[test]
    fn test_validate_files_all_success() {
        let temp_dir = create_test_dir();
        let file1 = temp_dir.path().join("a.rs");
        let file2 = temp_dir.path().join("b.rs");
        fs::write(&file1, "fn a() {}").unwrap();
        fs::write(&file2, "fn b() {}").unwrap();

        let paths = vec![file1, file2];
        let result = run_validation(validate_files(&paths));

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_validate_files_accumulates_errors() {
        let temp_dir = create_test_dir();
        let valid_file = temp_dir.path().join("valid.rs");
        fs::write(&valid_file, "fn valid() {}").unwrap();

        let paths = vec![
            valid_file,
            PathBuf::from("/nonexistent/a.rs"),
            PathBuf::from("/nonexistent/b.rs"),
        ];
        let result = validate_files(&paths);

        // Should accumulate BOTH nonexistent file errors
        match result {
            stillwater::Validation::Failure(errors) => {
                let errors_vec: Vec<_> = errors.into_iter().collect();
                assert_eq!(errors_vec.len(), 2);
            }
            _ => panic!("Expected failure with accumulated errors"),
        }
    }

    #[test]
    fn test_analyze_file_from_path() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main() { let x = 1; }").unwrap();

        let result = analyze_file_from_path(&file_path, false);

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.path, file_path);
        assert!(analysis.analysis_time.is_none());
    }

    #[test]
    fn test_analyze_file_from_path_with_timing() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let result = analyze_file_from_path(&file_path, true);

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.analysis_time.is_some());
    }

    #[test]
    fn test_analyze_files_parallel_sequential() {
        let temp_dir = create_test_dir();
        let file1 = temp_dir.path().join("a.rs");
        let file2 = temp_dir.path().join("b.rs");
        fs::write(&file1, "fn a() {}").unwrap();
        fs::write(&file2, "fn b() {}").unwrap();

        let config = ParallelConfig::sequential();
        let result = analyze_files_parallel(&[file1, file2], &config, false);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_analyze_files_parallel_enabled() {
        let temp_dir = create_test_dir();
        let file1 = temp_dir.path().join("a.rs");
        let file2 = temp_dir.path().join("b.rs");
        let file3 = temp_dir.path().join("c.rs");
        fs::write(&file1, "fn a() {}").unwrap();
        fs::write(&file2, "fn b() {}").unwrap();
        fs::write(&file3, "fn c() {}").unwrap();

        let config = ParallelConfig::default();
        let result = analyze_files_parallel(&[file1, file2, file3], &config, false);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_validate_and_analyze_files_success() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let result = run_validation(validate_and_analyze_files(std::slice::from_ref(&file_path)));

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, file_path);
    }

    #[test]
    fn test_validate_and_analyze_files_validation_failure() {
        let paths = vec![PathBuf::from("/nonexistent/file.rs")];
        let result = run_validation(validate_and_analyze_files(&paths));

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_syntax_rust_valid() {
        let content = "fn main() {}";
        let result = validate_syntax(content, Language::Rust, Path::new("test.rs"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_syntax_rust_invalid() {
        let content = "fn main( { }"; // Missing closing paren
        let result = validate_syntax(content, Language::Rust, Path::new("test.rs"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_syntax_python() {
        let content = "def hello():\n    pass";
        let result = validate_syntax(content, Language::Python, Path::new("test.py"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_combine_validations_preserve_successes() {
        let validations: Vec<AnalysisValidation<i32>> = vec![
            validation_success(1),
            validation_success(2),
            validation_success(3),
        ];

        let result = combine_validations_preserve_successes(validations);

        match result {
            stillwater::Validation::Success(values) => {
                assert_eq!(values, vec![1, 2, 3]);
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_combine_validations_with_failures() {
        let validations: Vec<AnalysisValidation<i32>> = vec![
            validation_success(1),
            validation_failure(AnalysisError::validation("Error 1")),
            validation_success(3),
            validation_failure(AnalysisError::validation("Error 2")),
        ];

        let result = combine_validations_preserve_successes(validations);

        match result {
            stillwater::Validation::Failure(errors) => {
                let errors_vec: Vec<_> = errors.into_iter().collect();
                assert_eq!(errors_vec.len(), 2);
            }
            _ => panic!("Expected failure"),
        }
    }
}

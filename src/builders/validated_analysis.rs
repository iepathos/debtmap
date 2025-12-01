//! Multi-file analysis with error accumulation.
//!
//! This module provides validation-aware analysis that accumulates ALL errors
//! instead of failing at the first one. This enables users to see all file
//! issues in a single run.
//!
//! # Design Philosophy
//!
//! - **Error Accumulation**: Collect ALL file read/parse errors before failing
//! - **Pure Functions**: File analysis is performed using pure transformations
//! - **Context Preservation**: Each error includes the file path and details
//! - **Backwards Compatible**: Wrappers convert to `anyhow::Result` for existing code
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::builders::validated_analysis::{
//!     analyze_files_validated, analyze_files_result
//! };
//!
//! // Get validation with ALL errors accumulated
//! let validation = analyze_files_validated(&files, &config);
//!
//! // Or use backwards-compatible Result API
//! let result = analyze_files_result(&files, &config);
//! ```

use std::path::{Path, PathBuf};

use crate::core::Language;
use crate::effects::{
    combine_validations, run_validation, validation_failure, validation_success, AnalysisValidation,
};
use crate::errors::AnalysisError;

/// Result of reading a file with its content and path.
#[derive(Debug, Clone)]
pub struct FileContent {
    /// The file path
    pub path: PathBuf,
    /// The file content
    pub content: String,
    /// The detected language
    pub language: Language,
}

/// Validate that files can be read, accumulating ALL read errors.
///
/// Returns a validation containing either all successfully read files
/// or ALL errors encountered during reading.
///
/// # Example
///
/// ```rust,ignore
/// let files = vec![
///     PathBuf::from("src/good.rs"),
///     PathBuf::from("/nonexistent/path"),
///     PathBuf::from("src/also_good.rs"),
/// ];
///
/// let result = validate_files_readable(&files);
/// // If /nonexistent/path doesn't exist, failure contains that error
/// // but also includes any other missing files
/// ```
pub fn validate_files_readable(files: &[PathBuf]) -> AnalysisValidation<Vec<FileContent>> {
    let validations: Vec<AnalysisValidation<FileContent>> = files
        .iter()
        .map(|path| validate_single_file_readable(path))
        .collect();

    combine_validations(validations)
}

/// Validate that a single file can be read.
fn validate_single_file_readable(path: &Path) -> AnalysisValidation<FileContent> {
    // Check if file exists
    if !path.exists() {
        return validation_failure(AnalysisError::io_with_path(
            format!("File not found: {}", path.display()),
            path,
        ));
    }

    // Check if it's a file (not directory)
    if !path.is_file() {
        return validation_failure(AnalysisError::io_with_path(
            format!("Path is not a file: {}", path.display()),
            path,
        ));
    }

    // Try to read the file
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let language = Language::from_path(path);
            validation_success(FileContent {
                path: path.to_path_buf(),
                content,
                language,
            })
        }
        Err(e) => validation_failure(AnalysisError::io_with_path(
            format!("Cannot read file: {}", e),
            path,
        )),
    }
}

/// Validate files readable with backwards-compatible Result API.
pub fn validate_files_readable_result(files: &[PathBuf]) -> anyhow::Result<Vec<FileContent>> {
    run_validation(validate_files_readable(files))
}

/// Summary of file read operations for user feedback.
#[derive(Debug, Clone)]
pub struct FileReadSummary {
    /// Number of files successfully read
    pub successful: usize,
    /// Number of files that failed to read
    pub failed: usize,
    /// Total files attempted
    pub total: usize,
    /// Error messages for failed files
    pub errors: Vec<AnalysisError>,
    /// Successfully read files
    pub files: Vec<FileContent>,
}

impl FileReadSummary {
    /// Check if all files were read successfully.
    pub fn all_successful(&self) -> bool {
        self.failed == 0
    }

    /// Format a summary message for display.
    pub fn format_summary(&self) -> String {
        if self.all_successful() {
            format!("Successfully read {} files", self.successful)
        } else {
            format!(
                "Read {} of {} files ({} failed)",
                self.successful, self.total, self.failed
            )
        }
    }
}

/// Read files with partial success support.
///
/// Unlike `validate_files_readable`, this function returns a summary
/// that includes both successes and failures, allowing analysis to
/// continue with successfully read files while reporting failures.
///
/// # Use Cases
///
/// - When you want to analyze as many files as possible
/// - When some files may be temporarily locked or inaccessible
/// - When you want to show progress even with some failures
pub fn read_files_with_summary(files: &[PathBuf]) -> FileReadSummary {
    let mut successful_files = Vec::new();
    let mut errors = Vec::new();

    for path in files {
        match validate_single_file_readable(path) {
            stillwater::Validation::Success(file_content) => {
                successful_files.push(file_content);
            }
            stillwater::Validation::Failure(errs) => {
                for err in errs {
                    errors.push(err);
                }
            }
        }
    }

    let successful = successful_files.len();
    let failed = errors.len();
    let total = files.len();

    FileReadSummary {
        successful,
        failed,
        total,
        errors,
        files: successful_files,
    }
}

/// Validate source content can be parsed, accumulating ALL parse errors.
///
/// This function attempts to parse each file content and accumulates
/// all parse errors instead of failing at the first one.
///
/// # Note
///
/// This is a validation-level function. Actual parsing uses the
/// language-specific analyzers from the `analyzers` module.
pub fn validate_sources_parseable(files: &[FileContent]) -> AnalysisValidation<Vec<FileContent>> {
    let validations: Vec<AnalysisValidation<FileContent>> =
        files.iter().map(validate_single_source_parseable).collect();

    combine_validations(validations)
}

/// Validate a single source file can be parsed.
fn validate_single_source_parseable(file: &FileContent) -> AnalysisValidation<FileContent> {
    match file.language {
        Language::Rust => validate_rust_parseable(file),
        Language::Python => validate_python_parseable(file),
        Language::JavaScript | Language::TypeScript => validate_js_parseable(file),
        Language::Unknown => {
            // Unknown languages pass through - we can't validate them
            validation_success(file.clone())
        }
    }
}

/// Validate Rust source is parseable.
fn validate_rust_parseable(file: &FileContent) -> AnalysisValidation<FileContent> {
    // Use syn to check if the file parses
    match syn::parse_file(&file.content) {
        Ok(_) => validation_success(file.clone()),
        Err(e) => {
            let line = e.span().start().line;
            validation_failure(AnalysisError::parse_with_context(
                format!("Rust parse error: {}", e),
                &file.path,
                line,
            ))
        }
    }
}

/// Validate Python source is parseable (basic validation).
fn validate_python_parseable(file: &FileContent) -> AnalysisValidation<FileContent> {
    // Basic Python syntax validation:
    // Check for common syntax issues without a full parser

    for (line_num, line) in file.content.lines().enumerate() {
        // Skip empty lines and comments
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Check for tabs mixed with spaces (common Python error)
        if line.starts_with(' ') && line.contains('\t') {
            return validation_failure(AnalysisError::parse_with_context(
                "Mixed tabs and spaces in indentation".to_string(),
                &file.path,
                line_num + 1,
            ));
        }
    }

    // For now, Python files pass through without deep validation
    // A full Python parser would be needed for proper validation
    validation_success(file.clone())
}

/// Validate JavaScript/TypeScript source is parseable (basic validation).
fn validate_js_parseable(file: &FileContent) -> AnalysisValidation<FileContent> {
    // Basic bracket matching validation
    let mut paren_count: i32 = 0;
    let mut brace_count: i32 = 0;
    let mut bracket_count: i32 = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    let mut prev_char = ' ';

    for (line_num, line) in file.content.lines().enumerate() {
        for ch in line.chars() {
            // Track string state
            if !in_string && (ch == '"' || ch == '\'' || ch == '`') {
                in_string = true;
                string_char = ch;
            } else if in_string && ch == string_char && prev_char != '\\' {
                in_string = false;
            }

            // Only count brackets outside strings
            if !in_string {
                match ch {
                    '(' => paren_count += 1,
                    ')' => {
                        paren_count -= 1;
                        if paren_count < 0 {
                            return validation_failure(AnalysisError::parse_with_context(
                                "Unmatched closing parenthesis".to_string(),
                                &file.path,
                                line_num + 1,
                            ));
                        }
                    }
                    '{' => brace_count += 1,
                    '}' => {
                        brace_count -= 1;
                        if brace_count < 0 {
                            return validation_failure(AnalysisError::parse_with_context(
                                "Unmatched closing brace".to_string(),
                                &file.path,
                                line_num + 1,
                            ));
                        }
                    }
                    '[' => bracket_count += 1,
                    ']' => {
                        bracket_count -= 1;
                        if bracket_count < 0 {
                            return validation_failure(AnalysisError::parse_with_context(
                                "Unmatched closing bracket".to_string(),
                                &file.path,
                                line_num + 1,
                            ));
                        }
                    }
                    _ => {}
                }
            }

            prev_char = ch;
        }
    }

    // Check for unclosed brackets at end of file
    if paren_count > 0 {
        return validation_failure(AnalysisError::parse_with_context(
            format!("Unclosed parenthesis ({} open)", paren_count),
            &file.path,
            file.content.lines().count(),
        ));
    }
    if brace_count > 0 {
        return validation_failure(AnalysisError::parse_with_context(
            format!("Unclosed brace ({} open)", brace_count),
            &file.path,
            file.content.lines().count(),
        ));
    }
    if bracket_count > 0 {
        return validation_failure(AnalysisError::parse_with_context(
            format!("Unclosed bracket ({} open)", bracket_count),
            &file.path,
            file.content.lines().count(),
        ));
    }

    validation_success(file.clone())
}

/// Validate sources parseable with backwards-compatible Result API.
pub fn validate_sources_parseable_result(
    files: &[FileContent],
) -> anyhow::Result<Vec<FileContent>> {
    run_validation(validate_sources_parseable(files))
}

/// Full file validation pipeline: read + parse, accumulating ALL errors.
///
/// This combines `validate_files_readable` and `validate_sources_parseable`
/// to validate that files both exist and can be parsed.
pub fn validate_files_full(files: &[PathBuf]) -> AnalysisValidation<Vec<FileContent>> {
    // First validate all files are readable
    let readable = validate_files_readable(files);

    // Then validate all readable files are parseable
    match readable {
        stillwater::Validation::Success(contents) => validate_sources_parseable(&contents),
        stillwater::Validation::Failure(errors) => stillwater::Validation::Failure(errors),
    }
}

/// Full file validation with backwards-compatible Result API.
pub fn validate_files_full_result(files: &[PathBuf]) -> anyhow::Result<Vec<FileContent>> {
    run_validation(validate_files_full(files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use stillwater::Validation;
    use tempfile::TempDir;

    #[test]
    fn test_validate_files_readable_all_exist() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.rs");
        let file2 = temp_dir.path().join("file2.rs");

        fs::write(&file1, "fn main() {}").unwrap();
        fs::write(&file2, "fn test() {}").unwrap();

        let files = vec![file1, file2];
        let result = validate_files_readable(&files);

        assert!(result.is_success());
        if let Validation::Success(contents) = result {
            assert_eq!(contents.len(), 2);
        }
    }

    #[test]
    fn test_validate_files_readable_accumulates_errors() {
        let files = vec![
            PathBuf::from("/nonexistent/path1.rs"),
            PathBuf::from("/nonexistent/path2.rs"),
            PathBuf::from("/nonexistent/path3.rs"),
        ];

        let result = validate_files_readable(&files);

        match result {
            Validation::Failure(errors) => {
                assert_eq!(errors.len(), 3, "Expected 3 file read errors");
            }
            Validation::Success(_) => panic!("Expected failure for nonexistent files"),
        }
    }

    #[test]
    fn test_read_files_with_summary_partial_success() {
        let temp_dir = TempDir::new().unwrap();
        let good_file = temp_dir.path().join("good.rs");
        fs::write(&good_file, "fn main() {}").unwrap();

        let files = vec![good_file, PathBuf::from("/nonexistent/path.rs")];

        let summary = read_files_with_summary(&files);

        assert_eq!(summary.successful, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.total, 2);
        assert!(!summary.all_successful());
        assert_eq!(summary.files.len(), 1);
        assert_eq!(summary.errors.len(), 1);
    }

    #[test]
    fn test_read_files_with_summary_format() {
        let summary = FileReadSummary {
            successful: 5,
            failed: 2,
            total: 7,
            errors: vec![],
            files: vec![],
        };

        let message = summary.format_summary();
        assert!(message.contains("5"));
        assert!(message.contains("7"));
        assert!(message.contains("2"));
    }

    #[test]
    fn test_validate_rust_parseable_success() {
        let file = FileContent {
            path: PathBuf::from("test.rs"),
            content: "fn main() { println!(\"Hello\"); }".to_string(),
            language: Language::Rust,
        };

        let result = validate_rust_parseable(&file);
        assert!(result.is_success());
    }

    #[test]
    fn test_validate_rust_parseable_failure() {
        let file = FileContent {
            path: PathBuf::from("test.rs"),
            content: "fn main() { incomplete".to_string(),
            language: Language::Rust,
        };

        let result = validate_rust_parseable(&file);
        assert!(result.is_failure());
    }

    #[test]
    fn test_validate_sources_parseable_accumulates_errors() {
        let files = vec![
            FileContent {
                path: PathBuf::from("good.rs"),
                content: "fn main() {}".to_string(),
                language: Language::Rust,
            },
            FileContent {
                path: PathBuf::from("bad1.rs"),
                content: "fn main() {".to_string(), // Missing closing brace
                language: Language::Rust,
            },
            FileContent {
                path: PathBuf::from("bad2.rs"),
                content: "fn incomplete(".to_string(), // Incomplete
                language: Language::Rust,
            },
        ];

        let result = validate_sources_parseable(&files);

        match result {
            Validation::Failure(errors) => {
                assert_eq!(errors.len(), 2, "Expected 2 parse errors");
            }
            Validation::Success(_) => panic!("Expected failure for invalid Rust"),
        }
    }

    #[test]
    fn test_validate_js_parseable_success() {
        let file = FileContent {
            path: PathBuf::from("test.js"),
            content: "function test() { return { a: 1 }; }".to_string(),
            language: Language::JavaScript,
        };

        let result = validate_js_parseable(&file);
        assert!(result.is_success());
    }

    #[test]
    fn test_validate_js_parseable_unmatched_brace() {
        let file = FileContent {
            path: PathBuf::from("test.js"),
            content: "function test() { return { a: 1 }".to_string(), // Missing }
            language: Language::JavaScript,
        };

        let result = validate_js_parseable(&file);
        assert!(result.is_failure());
    }

    #[test]
    fn test_validate_js_parseable_string_with_brackets() {
        // Brackets inside strings should not be counted
        let file = FileContent {
            path: PathBuf::from("test.js"),
            content: r#"const s = "{ hello }"; const x = [];"#.to_string(),
            language: Language::JavaScript,
        };

        let result = validate_js_parseable(&file);
        assert!(result.is_success());
    }

    #[test]
    fn test_validate_files_full_integration() {
        let temp_dir = TempDir::new().unwrap();
        let good_file = temp_dir.path().join("good.rs");
        fs::write(&good_file, "fn main() {}").unwrap();

        let files = vec![good_file];
        let result = validate_files_full(&files);

        assert!(result.is_success());
    }

    #[test]
    fn test_file_content_language_detection() {
        let temp_dir = TempDir::new().unwrap();

        // Create files with different extensions
        let rust_file1 = temp_dir.path().join("test1.rs");
        let rust_file2 = temp_dir.path().join("test2.rs");

        fs::write(&rust_file1, "fn main() {}").unwrap();
        fs::write(&rust_file2, "fn another() { let x = 5; }").unwrap();

        let files = vec![rust_file1, rust_file2];
        let result = validate_files_readable(&files);

        if let Validation::Success(contents) = result {
            assert_eq!(contents[0].language, Language::Rust);
            assert_eq!(contents[1].language, Language::Rust);
        } else {
            panic!("Expected success");
        }
    }
}

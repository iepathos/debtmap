//! Error collection data structures for batch operations.
//!
//! This module implements the Stillwater "fail completely" pattern for
//! independent operations like file analysis. Instead of stopping at the
//! first error, we collect ALL errors and present them together.

use anyhow::Error;
use std::path::PathBuf;

/// Results from batch analysis operations.
///
/// Follows the Stillwater "fail completely" pattern: each item is
/// analyzed independently, and we return BOTH successes and failures.
#[derive(Debug, Clone)]
pub struct AnalysisResults<T> {
    pub successes: Vec<T>,
    pub failures: Vec<AnalysisFailure>,
}

impl<T> AnalysisResults<T> {
    pub fn new(successes: Vec<T>, failures: Vec<AnalysisFailure>) -> Self {
        Self {
            successes,
            failures,
        }
    }

    pub fn success_count(&self) -> usize {
        self.successes.len()
    }

    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    pub fn total_count(&self) -> usize {
        self.success_count() + self.failure_count()
    }

    pub fn is_complete_success(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_count() == 0 {
            return 1.0;
        }
        self.success_count() as f64 / self.total_count() as f64
    }
}

/// Information about a failed analysis operation.
#[derive(Debug, Clone)]
pub struct AnalysisFailure {
    pub path: PathBuf,
    pub operation: OperationType,
    pub error: String, // String for Clone, preserves error message
}

impl AnalysisFailure {
    pub fn new(path: PathBuf, operation: OperationType, error: Error) -> Self {
        Self {
            path,
            operation,
            error: format!("{:#}", error), // Pretty error format
        }
    }

    pub fn file_read(path: PathBuf, error: Error) -> Self {
        Self::new(path, OperationType::FileRead, error)
    }

    pub fn file_parse(path: PathBuf, error: Error) -> Self {
        Self::new(path, OperationType::FileParse, error)
    }

    pub fn directory_access(path: PathBuf, error: Error) -> Self {
        Self::new(path, OperationType::DirectoryAccess, error)
    }

    pub fn analysis(path: PathBuf, error: Error) -> Self {
        Self::new(path, OperationType::Analysis, error)
    }
}

/// Type of operation that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationType {
    FileRead,
    FileParse,
    DirectoryAccess,
    Analysis,
    Other,
}

impl OperationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileRead => "File read",
            Self::FileParse => "File parse",
            Self::DirectoryAccess => "Directory access",
            Self::Analysis => "Analysis",
            Self::Other => "Other",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn test_analysis_results_success_count() {
        let results = AnalysisResults {
            successes: vec![1, 2, 3],
            failures: vec![],
        };

        assert_eq!(results.success_count(), 3);
        assert_eq!(results.failure_count(), 0);
        assert_eq!(results.total_count(), 3);
        assert!(results.is_complete_success());
        assert_eq!(results.success_rate(), 1.0);
    }

    #[test]
    fn test_analysis_results_with_failures() {
        let results = AnalysisResults {
            successes: vec![1, 2, 3],
            failures: vec![
                AnalysisFailure::file_read(PathBuf::from("a.rs"), anyhow!("Permission denied")),
                AnalysisFailure::file_parse(PathBuf::from("b.rs"), anyhow!("Parse error")),
            ],
        };

        assert_eq!(results.success_count(), 3);
        assert_eq!(results.failure_count(), 2);
        assert_eq!(results.total_count(), 5);
        assert!(!results.is_complete_success());
        assert_eq!(results.success_rate(), 0.6);
    }

    #[test]
    fn test_analysis_failure_creation() {
        let failure =
            AnalysisFailure::file_read(PathBuf::from("test.rs"), anyhow!("File not found"));

        assert_eq!(failure.path, PathBuf::from("test.rs"));
        assert_eq!(failure.operation, OperationType::FileRead);
        assert!(failure.error.contains("File not found"));
    }

    #[test]
    fn test_operation_type_as_str() {
        assert_eq!(OperationType::FileRead.as_str(), "File read");
        assert_eq!(OperationType::FileParse.as_str(), "File parse");
        assert_eq!(OperationType::Analysis.as_str(), "Analysis");
    }
}

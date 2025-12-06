//! Error summary generation for batch operations.

use super::collection::{AnalysisFailure, OperationType};
use std::collections::HashMap;
use std::path::PathBuf;

/// Summary of errors from batch operations.
#[derive(Debug)]
pub struct ErrorSummary {
    pub total: usize,
    pub by_operation: HashMap<OperationType, usize>,
    pub by_error_kind: HashMap<String, Vec<PathBuf>>,
    pub sample_errors: Vec<AnalysisFailure>,
}

impl ErrorSummary {
    pub fn from_failures(failures: &[AnalysisFailure]) -> Self {
        let total = failures.len();

        // Group by operation type
        let mut by_operation: HashMap<OperationType, usize> = HashMap::new();
        for failure in failures {
            *by_operation.entry(failure.operation).or_insert(0) += 1;
        }

        // Group by error kind (first line of error message)
        let mut by_error_kind: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for failure in failures {
            let error_kind = extract_error_kind(&failure.error);
            by_error_kind
                .entry(error_kind)
                .or_default()
                .push(failure.path.clone());
        }

        // Take samples (up to 10 total)
        let sample_errors = failures.iter().take(10).cloned().collect();

        Self {
            total,
            by_operation,
            by_error_kind,
            sample_errors,
        }
    }

    pub fn report(&self) -> String {
        let mut report = String::new();

        report.push_str("\nFailure breakdown:\n");

        // Report by operation type
        for (op_type, count) in &self.by_operation {
            report.push_str(&format!("  {}: {} file(s)\n", op_type.as_str(), count));
        }

        // Report by error kind
        report.push_str("\nError categories:\n");
        for (error_kind, paths) in &self.by_error_kind {
            report.push_str(&format!("  {}: {} file(s)\n", error_kind, paths.len()));

            // Show first few examples
            for path in paths.iter().take(3) {
                report.push_str(&format!("    - {}\n", path.display()));
            }

            if paths.len() > 3 {
                report.push_str(&format!("    ... and {} more\n", paths.len() - 3));
            }
        }

        report
    }
}

/// Extracts error kind from error message (first line or error type).
fn extract_error_kind(error: &str) -> String {
    // Try to extract error category
    if error.contains("Permission denied") {
        "Permission denied".to_string()
    } else if error.contains("No such file") {
        "File not found".to_string()
    } else if error.contains("parse") || error.contains("expected") {
        "Parse error".to_string()
    } else if error.contains("timeout") || error.contains("Timeout") {
        "Timeout".to_string()
    } else {
        // Use first line of error
        error.lines().next().unwrap_or("Unknown error").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn test_error_summary_groups_by_operation() {
        let failures = vec![
            AnalysisFailure::file_read(PathBuf::from("a.rs"), anyhow!("Error 1")),
            AnalysisFailure::file_read(PathBuf::from("b.rs"), anyhow!("Error 2")),
            AnalysisFailure::file_parse(PathBuf::from("c.rs"), anyhow!("Error 3")),
        ];

        let summary = ErrorSummary::from_failures(&failures);

        assert_eq!(summary.total, 3);
        assert_eq!(summary.by_operation[&OperationType::FileRead], 2);
        assert_eq!(summary.by_operation[&OperationType::FileParse], 1);
    }

    #[test]
    fn test_error_summary_groups_by_kind() {
        let failures = vec![
            AnalysisFailure::file_read(
                PathBuf::from("a.rs"),
                anyhow!("Permission denied (os error 13)"),
            ),
            AnalysisFailure::file_read(
                PathBuf::from("b.rs"),
                anyhow!("Permission denied (os error 13)"),
            ),
            AnalysisFailure::file_parse(
                PathBuf::from("c.rs"),
                anyhow!("expected item, found `}}`"),
            ),
        ];

        let summary = ErrorSummary::from_failures(&failures);

        assert_eq!(
            summary
                .by_error_kind
                .get("Permission denied")
                .map(|v| v.len()),
            Some(2)
        );
        assert_eq!(
            summary.by_error_kind.get("Parse error").map(|v| v.len()),
            Some(1)
        );
    }

    #[test]
    fn test_error_summary_report_format() {
        let failures = vec![
            AnalysisFailure::file_read(
                PathBuf::from("a.rs"),
                anyhow!("Permission denied (os error 13)"),
            ),
            AnalysisFailure::file_read(
                PathBuf::from("b.rs"),
                anyhow!("Permission denied (os error 13)"),
            ),
            AnalysisFailure::file_parse(
                PathBuf::from("c.rs"),
                anyhow!("expected item, found `}}`"),
            ),
        ];

        let summary = ErrorSummary::from_failures(&failures);
        let report = summary.report();

        assert!(report.contains("Failure breakdown:"));
        assert!(report.contains("Error categories:"));
        assert!(report.contains("Permission denied"));
        assert!(report.contains("Parse error"));
    }
}

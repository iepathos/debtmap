//! Pure formatting functions for validation output.
//!
//! This module contains pure string-building functions with no I/O.
//! Each formatter takes a ValidationResult and returns a formatted string.

use anyhow::{Context, Result};

use super::types::ValidationResult;

/// Pure: Format validation result as JSON.
pub fn format_json(result: &ValidationResult) -> Result<String> {
    serde_json::to_string_pretty(result).context("Failed to serialize validation result")
}

/// Pure: Format validation result for terminal display.
pub fn format_terminal(result: &ValidationResult) -> String {
    let mut output = String::new();

    output.push_str("═══ Validation Results ═══\n");
    output.push_str(&format!(
        "Completion: {:.1}%\n",
        result.completion_percentage
    ));
    output.push_str(&format!("Status: {}\n\n", result.status));

    if !result.improvements.is_empty() {
        output.push_str("✓ Improvements:\n");
        for improvement in &result.improvements {
            output.push_str(&format!("  • {}\n", improvement));
        }
        output.push('\n');
    }

    if !result.remaining_issues.is_empty() {
        output.push_str("⚠ Remaining Issues:\n");
        for issue in &result.remaining_issues {
            output.push_str(&format!("  • {}\n", issue));
        }
    }

    output
}

/// Pure: Format validation result as Markdown.
pub fn format_markdown(result: &ValidationResult) -> String {
    let mut output = String::new();

    output.push_str("# Validation Results\n\n");
    output.push_str(&format!(
        "**Completion**: {:.1}%\n",
        result.completion_percentage
    ));
    output.push_str(&format!("**Status**: {}\n\n", result.status));

    if !result.improvements.is_empty() {
        output.push_str("## Improvements\n\n");
        for improvement in &result.improvements {
            output.push_str(&format!("- {}\n", improvement));
        }
        output.push('\n');
    }

    if !result.remaining_issues.is_empty() {
        output.push_str("## Remaining Issues\n\n");
        for issue in &result.remaining_issues {
            output.push_str(&format!("- {}\n", issue));
        }
        output.push('\n');
    }

    if !result.gaps.is_empty() {
        output.push_str("## Gaps\n\n");
        for (key, gap) in &result.gaps {
            output.push_str(&format!("### {}\n\n", key));
            output.push_str(&format!("- **Description**: {}\n", gap.description));
            output.push_str(&format!("- **Location**: {}\n", gap.location));
            output.push_str(&format!("- **Severity**: {}\n", gap.severity));
            output.push_str(&format!("- **Suggested Fix**: {}\n\n", gap.suggested_fix));
        }
    }

    output
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::validate_improvement::types::{GapDetail, ProjectSummary};
    use std::collections::HashMap;

    fn create_test_result() -> ValidationResult {
        ValidationResult {
            completion_percentage: 75.0,
            status: "complete".to_string(),
            improvements: vec!["Improved target".to_string()],
            remaining_issues: vec![],
            gaps: HashMap::new(),
            target_summary: None,
            project_summary: ProjectSummary {
                total_debt_before: 100.0,
                total_debt_after: 50.0,
                improvement_percent: 50.0,
                items_resolved: 5,
                items_new: 0,
            },
            trend_analysis: None,
            attempt_number: None,
        }
    }

    #[test]
    fn test_format_terminal_includes_completion() {
        let result = create_test_result();
        let output = format_terminal(&result);

        assert!(output.contains("75.0%"));
        assert!(output.contains("complete"));
        assert!(output.contains("Improved target"));
    }

    #[test]
    fn test_format_markdown_includes_headers() {
        let result = create_test_result();
        let output = format_markdown(&result);

        assert!(output.contains("# Validation Results"));
        assert!(output.contains("## Improvements"));
        assert!(output.contains("**Completion**: 75.0%"));
    }

    #[test]
    fn test_format_json_parses_back() {
        let result = create_test_result();
        let json = format_json(&result).unwrap();
        let parsed: ValidationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.completion_percentage, 75.0);
        assert_eq!(parsed.status, "complete");
    }

    #[test]
    fn test_format_markdown_includes_gaps() {
        let mut result = create_test_result();
        result.gaps.insert(
            "test_gap".to_string(),
            GapDetail {
                description: "Test description".to_string(),
                location: "test.rs:10".to_string(),
                severity: "high".to_string(),
                suggested_fix: "Fix it".to_string(),
                score_before: None,
                score_after: None,
                current_score: None,
            },
        );

        let output = format_markdown(&result);

        assert!(output.contains("## Gaps"));
        assert!(output.contains("### test_gap"));
        assert!(output.contains("Test description"));
    }
}

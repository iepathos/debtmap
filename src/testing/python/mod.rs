pub mod analyzer;
pub mod assertion_patterns;
pub mod complexity_analyzer;
pub mod flaky_patterns;
pub mod framework_detector;

use crate::core::{DebtItem, DebtType, Priority};
use rustpython_parser::ast;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum TestFramework {
    Unittest,
    Pytest,
    Nose,
    Doctest,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct TestQualityIssue {
    pub issue_type: TestIssueType,
    pub test_name: String,
    pub line: usize,
    pub severity: Severity,
    pub suggestion: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestIssueType {
    NoAssertions,
    OverlyComplex(u32),
    FlakyPattern(FlakinessType),
    ExcessiveMocking(usize),
    PoorIsolation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlakinessType {
    TimingDependency,
    RandomValues,
    ExternalDependency,
    FilesystemDependency,
    NetworkDependency,
    ThreadingIssue,
}

pub fn convert_test_issue_to_debt_item(issue: TestQualityIssue, path: &PathBuf) -> DebtItem {
    let (priority, message, context, debt_type) = match issue.issue_type {
        TestIssueType::NoAssertions => (
            Priority::High,
            format!("Test '{}' has no assertions", issue.test_name),
            Some(issue.suggestion),
            DebtType::TestQuality,
        ),
        TestIssueType::OverlyComplex(score) => (
            Priority::Medium,
            format!(
                "Test '{}' is overly complex (score: {})",
                issue.test_name, score
            ),
            Some(issue.suggestion),
            DebtType::TestComplexity,
        ),
        TestIssueType::FlakyPattern(ref flakiness_type) => (
            Priority::High,
            format!(
                "Test '{}' has flaky pattern: {:?}",
                issue.test_name, flakiness_type
            ),
            Some(issue.suggestion),
            DebtType::TestQuality,
        ),
        TestIssueType::ExcessiveMocking(count) => (
            Priority::Medium,
            format!(
                "Test '{}' has excessive mocking ({} mocks/patches)",
                issue.test_name, count
            ),
            Some(issue.suggestion),
            DebtType::TestComplexity,
        ),
        TestIssueType::PoorIsolation => (
            Priority::High,
            format!("Test '{}' has poor isolation", issue.test_name),
            Some(issue.suggestion),
            DebtType::TestQuality,
        ),
    };

    DebtItem {
        id: format!("python-test-{}-{}", path.display(), issue.line),
        debt_type,
        priority,
        file: path.clone(),
        line: issue.line,
        column: None,
        message,
        context,
    }
}

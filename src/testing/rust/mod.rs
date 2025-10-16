//! # Rust Test Quality Analysis Module
//!
//! Provides comprehensive analysis of Rust test code quality, detecting patterns
//! that indicate potential issues in test suites.
//!
//! ## Components
//!
//! - **analyzer**: Main orchestrator for Rust test analysis
//! - **assertion_detector**: Detects missing or weak assertions
//! - **complexity_scorer**: Measures test complexity
//! - **flaky_detector**: Identifies flaky test patterns
//! - **framework_detector**: Detects test frameworks (std, criterion, proptest, rstest)
//! - **test_classifier**: Classifies tests by type
//!
//! ## Test Quality Metrics
//!
//! ### Assertion Analysis
//! - Counts assertions per test (`assert!`, `assert_eq!`, `assert_ne!`)
//! - Detects tests with no assertions
//! - Identifies Result-based tests
//! - Tracks `#[should_panic]` usage
//!
//! ### Complexity Scoring
//! - Conditional statements: +2 per if/match
//! - Loops: +3 per loop
//! - Assertions: +1 per assertion beyond 5
//! - Nesting depth: +2 per level > 2
//! - Line count: +(lines-30)/10 for tests > 30 lines
//!
//! ### Flaky Pattern Detection
//! - **Timing**: `std::thread::sleep`, `Instant::now()`
//! - **Random**: `rand` crate usage
//! - **External**: Network calls, filesystem hardcoded paths
//! - **Threading**: Unsynchronized concurrent access
//! - **Hash ordering**: HashMap iteration
//!
//! ### Framework Detection
//! - Standard `#[test]` attribute
//! - Criterion benchmarks
//! - Proptest property tests
//! - Rstest parameterized tests

pub mod analyzer;
pub mod assertion_detector;
pub mod complexity_scorer;
pub mod flaky_detector;
pub mod framework_detector;
pub mod test_classifier;

use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;

/// Rust test framework types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustTestFramework {
    /// Standard library #[test]
    Std,
    /// Criterion benchmarks
    Criterion,
    /// Proptest property tests
    Proptest,
    /// Quickcheck property tests
    Quickcheck,
    /// Rstest parameterized tests
    Rstest,
}

/// Types of Rust tests
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustTestType {
    UnitTest,
    IntegrationTest,
    BenchmarkTest,
    PropertyTest,
    DocTest,
}

/// Rust-specific assertion types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustAssertionType {
    /// assert!(condition)
    Assert,
    /// assert_eq!(left, right)
    AssertEq,
    /// assert_ne!(left, right)
    AssertNe,
    /// matches!(value, pattern)
    Matches,
    /// #[should_panic]
    ShouldPanic,
    /// Ok(()) return from test
    ResultOk,
    /// Custom assertion macro
    Custom(String),
}

/// Flaky pattern types specific to Rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustFlakinessType {
    TimingDependency,
    RandomValue,
    ExternalDependency,
    FileSystemDependency,
    NetworkDependency,
    ThreadingIssue,
    HashOrdering,
}

/// Test quality issue severity
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustTestSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Rust test quality issue
#[derive(Debug, Clone)]
pub struct RustTestQualityIssue {
    pub issue_type: RustTestIssueType,
    pub test_name: String,
    pub line: usize,
    pub severity: RustTestSeverity,
    pub confidence: f32,
    pub explanation: String,
    pub suggestion: String,
}

/// Types of test quality issues
#[derive(Debug, Clone, PartialEq)]
pub enum RustTestIssueType {
    NoAssertions,
    TooComplex(u32),
    FlakyPattern(RustFlakinessType),
    ExcessiveMocking(usize),
    IsolationIssue,
    TestsTooMuch,
    SlowTest,
}

/// Convert Rust test quality issue to debt item
pub fn convert_rust_test_issue_to_debt_item(issue: RustTestQualityIssue, path: &Path) -> DebtItem {
    let (priority, message, context, debt_type) = match issue.issue_type {
        RustTestIssueType::NoAssertions => (
            Priority::High,
            format!("Test '{}' has no assertions", issue.test_name),
            Some(format!(
                "{}\n\nSuggestion: {}",
                issue.explanation, issue.suggestion
            )),
            DebtType::TestQuality,
        ),
        RustTestIssueType::TooComplex(score) => (
            Priority::Medium,
            format!(
                "Test '{}' is overly complex (score: {})",
                issue.test_name, score
            ),
            Some(format!(
                "{}\n\nSuggestion: {}",
                issue.explanation, issue.suggestion
            )),
            DebtType::TestComplexity,
        ),
        RustTestIssueType::FlakyPattern(ref flakiness_type) => (
            Priority::High,
            format!(
                "Test '{}' has flaky pattern: {:?}",
                issue.test_name, flakiness_type
            ),
            Some(format!(
                "{}\n\nSuggestion: {}",
                issue.explanation, issue.suggestion
            )),
            DebtType::TestQuality,
        ),
        RustTestIssueType::ExcessiveMocking(count) => (
            Priority::Medium,
            format!(
                "Test '{}' has excessive mocking ({} mocks)",
                issue.test_name, count
            ),
            Some(format!(
                "{}\n\nSuggestion: {}",
                issue.explanation, issue.suggestion
            )),
            DebtType::TestComplexity,
        ),
        RustTestIssueType::IsolationIssue => (
            Priority::High,
            format!("Test '{}' has isolation issues", issue.test_name),
            Some(format!(
                "{}\n\nSuggestion: {}",
                issue.explanation, issue.suggestion
            )),
            DebtType::TestQuality,
        ),
        RustTestIssueType::TestsTooMuch => (
            Priority::Medium,
            format!("Test '{}' tests too many concerns", issue.test_name),
            Some(format!(
                "{}\n\nSuggestion: {}",
                issue.explanation, issue.suggestion
            )),
            DebtType::TestQuality,
        ),
        RustTestIssueType::SlowTest => (
            Priority::Low,
            format!("Test '{}' may be slow", issue.test_name),
            Some(format!(
                "{}\n\nSuggestion: {}",
                issue.explanation, issue.suggestion
            )),
            DebtType::TestComplexity,
        ),
    };

    DebtItem {
        id: format!("rust-test-{}-{}", path.display(), issue.line),
        debt_type,
        priority,
        file: path.to_path_buf(),
        line: issue.line,
        column: None,
        message,
        context,
    }
}

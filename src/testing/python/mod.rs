//! # Python Test Quality Analysis Module
//!
//! This module provides comprehensive analysis of Python test code quality,
//! detecting various patterns that indicate potential issues in test suites.
//!
//! ## Components
//!
//! - **analyzer**: Main entry point for test analysis, orchestrates all detection patterns
//! - **assertion_patterns**: Detects missing or weak assertions in test functions
//! - **complexity_analyzer**: Measures test complexity using various metrics
//! - **flaky_patterns**: Identifies patterns that can cause test flakiness
//! - **framework_detector**: Automatically detects the testing framework being used
//!
//! ## Test Quality Metrics
//!
//! ### Assertion Patterns
//! Tests without assertions provide no value. This detector identifies:
//! - Tests with setup/action code but no assertions
//! - Framework-specific assertion patterns (unittest, pytest, nose)
//! - Suggests appropriate assertions based on the detected framework
//!
//! ### Complexity Analysis
//! Complex tests are hard to understand and maintain. Complexity is calculated using:
//! - **Conditional statements**: Each if/else adds 2 points
//! - **Loops**: Each for/while loop adds 3 points
//! - **Assertions**: More than 5 assertions add 1 point each
//! - **Mocking**: Each mock/patch adds 2 points
//! - **Nesting depth**: Depth > 2 adds 2 points per level
//! - **Line count**: Lines > 20 add (lines-20)/5 points
//!
//! Default threshold is 10, configurable via `TestComplexityAnalyzer::with_threshold()`
//!
//! ### Flaky Pattern Detection
//! Identifies patterns that cause non-deterministic test behavior:
//!
//! #### Timing Dependencies
//! - `time.sleep()` calls
//! - `datetime.now()` without mocking
//! - Performance timing assertions
//! - **Fix**: Use time mocking libraries like freezegun
//!
//! #### Random Values
//! - Usage of `random` module without seed
//! - UUID generation
//! - `secrets` module usage
//! - **Fix**: Use fixed seeds or deterministic test data
//!
//! #### External Dependencies
//! - HTTP requests to external services
//! - API calls without mocking
//! - **Fix**: Mock external services or use test doubles
//!
//! #### Filesystem Dependencies
//! - Direct file I/O without temp directories
//! - Hardcoded file paths
//! - **Fix**: Use tempfile module or mock filesystem
//!
//! #### Network Dependencies
//! - Socket connections
//! - Network I/O operations
//! - **Fix**: Mock network calls or use test servers
//!
//! #### Threading Issues
//! - Thread/Process creation without synchronization
//! - Concurrent operations without locks
//! - **Fix**: Use proper synchronization primitives or avoid threading in tests
//!
//! ## Framework Detection
//! Automatically detects the testing framework based on:
//! 1. Class inheritance from `unittest.TestCase` (highest priority)
//! 2. Pytest fixtures decorators
//! 3. Import statements
//! 4. Default to pytest for simple test functions
//!
//! ## Severity Levels
//!
//! - **Critical**: Issues that almost certainly cause test failures or flakiness
//! - **High**: Serious issues that significantly impact test quality
//! - **Medium**: Issues that should be addressed but may not cause immediate problems
//! - **Low**: Minor issues or style improvements
//!
//! ## Usage Example
//!
//! ```rust
//! use rustpython_parser::{ast, Parse};
//! use debtmap::testing::python::analyzer::PythonTestAnalyzer;
//! use std::path::PathBuf;
//!
//! let code = r#"
//! def test_example():
//!     result = calculate_something()
//!     # Missing assertion!
//! "#;
//!
//! let module = ast::Mod::parse(code, "test.py").unwrap();
//! let mut analyzer = PythonTestAnalyzer::new();
//! let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
//!
//! for issue in issues {
//!     println!("Found issue: {:?}", issue);
//! }
//! ```

pub mod analyzer;
pub mod assertion_patterns;
pub mod complexity_analyzer;
pub mod config;
pub mod flaky_patterns;
pub mod framework_detector;

#[cfg(test)]
mod analyzer_test;
#[cfg(test)]
mod assertion_patterns_test;
#[cfg(test)]
mod complexity_analyzer_test;
#[cfg(test)]
mod flaky_patterns_test;
#[cfg(test)]
mod framework_detector_test;

use crate::core::{DebtItem, DebtType, Priority};
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

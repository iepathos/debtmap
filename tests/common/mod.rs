// Test utility module for debtmap integration tests
#![allow(dead_code)]

pub mod analysis_helpers;
pub mod subprocess_converter;

use std::path::PathBuf;
use std::time::Duration;

// Re-export commonly used types
#[allow(unused_imports)]
pub use analysis_helpers::{analyze_code_snippet, analyze_file_directly};

// Test fixture management
#[derive(Debug, Clone)]
pub struct TestFixture {
    pub code: String,
    pub language: debtmap::core::Language,
    pub expected_issues: Vec<debtmap::core::DebtItem>,
    pub config: Option<debtmap::config::DebtmapConfig>,
}

// Binary execution result for tests that need to verify CLI behavior
#[derive(Debug)]
pub struct BinaryResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration: Duration,
}

impl TestFixture {
    pub fn new(code: &str, language: debtmap::core::Language) -> Self {
        Self {
            code: code.to_string(),
            language,
            expected_issues: Vec::new(),
            config: None,
        }
    }

    pub fn with_expected_issues(mut self, issues: Vec<debtmap::core::DebtItem>) -> Self {
        self.expected_issues = issues;
        self
    }

    pub fn with_config(mut self, config: debtmap::config::DebtmapConfig) -> Self {
        self.config = Some(config);
        self
    }
}

// Helper to create temporary test files
pub fn create_test_file(content: &str, extension: &str) -> (tempfile::TempDir, PathBuf) {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join(format!("test.{}", extension));
    std::fs::write(&file_path, content).expect("Failed to write test file");
    (temp_dir, file_path)
}

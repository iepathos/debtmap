//! Data types for coverage explanation.
//!
//! These types define the configuration input, strategy attempts,
//! and result structures used throughout the coverage explanation module.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Output format for coverage explanation results.
#[derive(Debug, Clone, Copy)]
pub enum DebugFormat {
    Text,
    Json,
}

/// Configuration for the explain_coverage command.
#[derive(Debug)]
pub struct ExplainCoverageConfig {
    pub path: PathBuf,
    pub coverage_file: PathBuf,
    pub function_name: String,
    pub file_path: Option<PathBuf>,
    pub verbose: bool,
    pub format: DebugFormat,
}

/// Records a single strategy attempt when matching coverage data.
#[derive(Debug, Serialize, Deserialize)]
pub struct StrategyAttempt {
    pub strategy: String,
    pub success: bool,
    pub matched_function: Option<String>,
    pub matched_file: Option<String>,
    pub coverage_percentage: Option<f64>,
}

impl StrategyAttempt {
    /// Create a successful strategy attempt.
    pub fn success(
        strategy: impl Into<String>,
        matched_function: String,
        matched_file: String,
        coverage_percentage: f64,
    ) -> Self {
        Self {
            strategy: strategy.into(),
            success: true,
            matched_function: Some(matched_function),
            matched_file: Some(matched_file),
            coverage_percentage: Some(coverage_percentage),
        }
    }

    /// Create a failed strategy attempt.
    pub fn failure(strategy: impl Into<String>) -> Self {
        Self {
            strategy: strategy.into(),
            success: false,
            matched_function: None,
            matched_file: None,
            coverage_percentage: None,
        }
    }
}

/// Complete result of coverage explanation including all attempts.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExplainCoverageResult {
    pub function_name: String,
    pub file_path: Option<String>,
    pub coverage_found: bool,
    pub coverage_percentage: Option<f64>,
    pub matched_by_strategy: Option<String>,
    pub attempts: Vec<StrategyAttempt>,
    pub available_functions: Vec<String>,
    pub available_files: Vec<String>,
}

impl ExplainCoverageResult {
    /// Create a new result with initial values.
    pub fn new(function_name: String, file_path: Option<PathBuf>) -> Self {
        Self {
            function_name,
            file_path: file_path.map(|p| p.display().to_string()),
            coverage_found: false,
            coverage_percentage: None,
            matched_by_strategy: None,
            attempts: Vec::new(),
            available_functions: Vec::new(),
            available_files: Vec::new(),
        }
    }

    /// Record a successful match from an attempt.
    pub fn record_match(&mut self, attempt: &StrategyAttempt) {
        if attempt.success && !self.coverage_found {
            self.matched_by_strategy = Some(attempt.strategy.clone());
            self.coverage_found = true;
            self.coverage_percentage = attempt.coverage_percentage;
        }
    }

    /// Add an attempt and record match if successful.
    pub fn add_attempt(&mut self, attempt: StrategyAttempt) {
        let is_success = attempt.success;
        self.attempts.push(attempt);
        if is_success {
            if let Some(last) = self.attempts.last() {
                let strategy = last.strategy.clone();
                let coverage = last.coverage_percentage;
                if !self.coverage_found {
                    self.matched_by_strategy = Some(strategy);
                    self.coverage_found = true;
                    self.coverage_percentage = coverage;
                }
            }
        }
    }
}

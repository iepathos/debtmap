//! Data structures for validation of technical debt improvements.
//!
//! This module contains pure data types with no behavior beyond
//! serialization and construction.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Output format for validation results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Terminal,
    Markdown,
}

/// Configuration for validation.
#[derive(Debug, Clone)]
pub struct ValidateImprovementConfig {
    pub comparison_path: PathBuf,
    pub output_path: PathBuf,
    pub previous_validation: Option<PathBuf>,
    pub threshold: f64,
    pub format: OutputFormat,
    pub quiet: bool,
}

/// Validation result structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub completion_percentage: f64,
    pub status: String,
    pub improvements: Vec<String>,
    pub remaining_issues: Vec<String>,
    pub gaps: HashMap<String, GapDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_summary: Option<TargetSummary>,
    pub project_summary: ProjectSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend_analysis: Option<TrendAnalysis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_number: Option<u32>,
}

/// Gap detail structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapDetail {
    pub description: String,
    pub location: String,
    pub severity: String,
    pub suggested_fix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_before: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_after: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_score: Option<f64>,
}

/// Target summary structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSummary {
    pub location: String,
    pub score_before: f64,
    pub score_after: Option<f64>,
    pub improvement_percent: f64,
    pub status: String,
}

/// Project summary structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub total_debt_before: f64,
    pub total_debt_after: f64,
    pub improvement_percent: f64,
    pub items_resolved: usize,
    pub items_new: usize,
}

/// Trend analysis structure for tracking progress across attempts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub direction: String,
    pub previous_completion: Option<f64>,
    pub change: Option<f64>,
    pub recommendation: String,
}

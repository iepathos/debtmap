//! Data types for debtmap comparison and validation.
//!
//! This module defines all the data structures used in comparing
//! before and after debtmap analysis results.

use crate::priority::{DebtItem, ImpactMetrics};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Internal type for parsing debtmap JSON files during comparison.
/// This supports parsing the unified JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtmapJsonInput {
    pub items: Vec<DebtItem>,
    #[serde(default = "default_impact_metrics")]
    pub total_impact: ImpactMetrics,
    #[serde(default)]
    pub total_debt_score: f64,
    #[serde(default)]
    pub debt_density: f64,
    #[serde(default)]
    pub total_lines_of_code: usize,
    #[serde(default)]
    pub overall_coverage: Option<f64>,
}

fn default_impact_metrics() -> ImpactMetrics {
    ImpactMetrics {
        complexity_reduction: 0.0,
        coverage_improvement: 0.0,
        risk_reduction: 0.0,
        lines_reduction: 0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub completion_percentage: f64,
    pub status: String,
    pub improvements: Vec<String>,
    pub remaining_issues: Vec<String>,
    pub gaps: HashMap<String, GapDetail>,
    pub before_summary: AnalysisSummary,
    pub after_summary: AnalysisSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapDetail {
    pub description: String,
    pub location: String,
    pub severity: String,
    pub suggested_fix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_complexity: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_complexity: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_complexity: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub total_items: usize,
    pub high_priority_items: usize,
    pub average_score: f64,
}

pub struct CompareConfig {
    pub before_path: PathBuf,
    pub after_path: PathBuf,
    pub output_path: PathBuf,
}

// =============================================================================
// Internal Analysis Types
// =============================================================================

/// Holds all identified changes between before and after states
pub(crate) struct IdentifiedChanges {
    pub resolved: ResolvedItems,
    pub improved: ImprovedItems,
    pub new_items: NewItems,
    pub unchanged_critical: UnchangedCritical,
}

pub(crate) struct ResolvedItems {
    pub high_priority_count: usize,
    #[allow(dead_code)]
    pub total_count: usize,
}

pub(crate) struct ImprovedItems {
    pub complexity_reduction: f64,
    pub coverage_improvement: f64,
    pub coverage_improvement_count: usize,
}

pub(crate) struct NewItems {
    pub critical_count: usize,
    pub items: Vec<ItemInfo>,
}

pub(crate) struct UnchangedCritical {
    pub count: usize,
    pub items: Vec<ItemInfo>,
}

pub(crate) struct ItemInfo {
    pub file: PathBuf,
    pub function: String,
    pub line: usize,
    pub score: f64,
}

/// Scoring component weights for weighted average calculation
pub(crate) struct ScoringComponents {
    pub high_priority: f64,
    pub improvement: f64,
    pub complexity: f64,
    pub regression: f64,
}

// =============================================================================
// Iterator Helpers
// =============================================================================

use crate::priority::unified_scorer::UnifiedDebtItem;

/// Extract function items from a slice of DebtItems.
///
/// This eliminates repeated DebtItem::Function/DebtItem::File pattern matching
/// throughout the codebase.
pub fn extract_functions(items: &[DebtItem]) -> impl Iterator<Item = &UnifiedDebtItem> {
    items.iter().filter_map(|item| match item {
        DebtItem::Function(f) => Some(f.as_ref()),
        DebtItem::File(_) => None,
    })
}

/// Extract function items with their keys (file, function) for lookup operations.
pub fn extract_function_keys(
    items: &[DebtItem],
) -> impl Iterator<Item = ((PathBuf, String), &UnifiedDebtItem)> {
    items.iter().filter_map(|item| match item {
        DebtItem::Function(f) => Some((
            (f.location.file.clone(), f.location.function.clone()),
            f.as_ref(),
        )),
        DebtItem::File(_) => None,
    })
}

/// Extract just the location keys from function items.
pub fn extract_location_keys(items: &[DebtItem]) -> impl Iterator<Item = (PathBuf, String)> + '_ {
    items.iter().filter_map(|item| match item {
        DebtItem::Function(f) => Some((f.location.file.clone(), f.location.function.clone())),
        DebtItem::File(_) => None,
    })
}

// =============================================================================
// Threshold Constants
// =============================================================================

/// Threshold for identifying critical debt items
pub const CRITICAL_SCORE_THRESHOLD: f64 = 8.0;

/// Tolerance for considering scores as "unchanged"
pub const SCORE_CHANGE_TOLERANCE: f64 = 0.5;

/// Minimum score improvement to count as "improved"
pub const SCORE_IMPROVEMENT_THRESHOLD: f64 = 0.5;

// =============================================================================
// Predicate Functions
// =============================================================================

/// Check if a score is considered critical (>= threshold)
pub fn is_critical(score: f64) -> bool {
    score >= CRITICAL_SCORE_THRESHOLD
}

/// Check if two scores are considered unchanged (absolute difference < tolerance)
pub fn is_score_unchanged(before: f64, after: f64) -> bool {
    (before - after).abs() < SCORE_CHANGE_TOLERANCE
}

/// Check if there was significant score improvement
pub fn is_significantly_improved(before_score: f64, after_score: f64) -> bool {
    before_score - after_score > SCORE_IMPROVEMENT_THRESHOLD
}

/// Extract maximum coverage value from transitive coverage
pub fn extract_max_coverage(coverage: &Option<crate::priority::coverage_propagation::TransitiveCoverage>) -> f64 {
    coverage
        .as_ref()
        .map(|tc| tc.direct.max(tc.transitive))
        .unwrap_or(0.0)
}

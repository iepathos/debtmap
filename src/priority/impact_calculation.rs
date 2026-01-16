//! Pure helper functions for impact calculation.
//!
//! This module contains pure functions extracted from `UnifiedAnalysis::calculate_total_impact`
//! to reduce complexity and improve testability.

use super::{DebtType, FileDebtItem, UnifiedDebtItem};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Accumulated metrics from function-level debt items.
#[derive(Debug, Default)]
pub struct ItemAccumulatedMetrics {
    pub total_debt_score: f64,
    pub coverage_improvement: f64,
    pub lines_reduction: u32,
    pub complexity_reduction: f64,
    pub risk_reduction: f64,
    pub functions_to_test: usize,
    pub unique_files: HashMap<PathBuf, usize>,
}

/// Accumulated metrics from file-level debt items.
#[derive(Debug, Default)]
pub struct FileAccumulatedMetrics {
    pub additional_debt_score: f64,
    pub coverage_improvement: f64,
    pub lines_reduction: u32,
    pub complexity_reduction: f64,
    pub unique_files: HashMap<PathBuf, usize>,
}

/// Calculate debt density (per 1000 lines of code).
///
/// This is a pure function that computes the debt density metric.
///
/// # Arguments
/// * `total_debt_score` - The total debt score
/// * `total_lines_of_code` - The total lines of code
///
/// # Returns
/// The debt density (debt score per 1000 LOC)
pub fn calculate_debt_density(total_debt_score: f64, total_lines_of_code: usize) -> f64 {
    if total_lines_of_code > 0 {
        (total_debt_score / total_lines_of_code as f64) * 1000.0
    } else {
        0.0
    }
}

/// Accumulate metrics from function-level debt items.
///
/// This is a pure function that iterates over debt items and accumulates
/// their scores and impact metrics.
///
/// # Arguments
/// * `items` - Slice of unified debt items
/// * `analyzed_files` - Pre-analyzed file line counts
///
/// # Returns
/// Accumulated metrics from all items
pub fn accumulate_item_metrics(
    items: &im::Vector<UnifiedDebtItem>,
    analyzed_files: &HashMap<PathBuf, usize>,
) -> ItemAccumulatedMetrics {
    let mut result = ItemAccumulatedMetrics {
        unique_files: analyzed_files.clone(),
        ..Default::default()
    };

    for item in items {
        result.total_debt_score += item.unified_score.final_score;

        // Use cached file line count if available (spec 204)
        if let Some(line_count) = item.file_line_count {
            result
                .unique_files
                .insert(item.location.file.clone(), line_count);
        }

        // Only count functions that actually need testing
        if item.expected_impact.coverage_improvement > 0.0 {
            result.functions_to_test += 1;
            result.coverage_improvement += item.expected_impact.coverage_improvement / 100.0;
        }

        result.lines_reduction += item.expected_impact.lines_reduction;
        result.complexity_reduction += item.expected_impact.complexity_reduction;
        result.risk_reduction += item.expected_impact.risk_reduction;
    }

    result
}

/// Collect files that have god object items.
///
/// This is used to avoid double-counting when file items also exist
/// for god object files.
///
/// # Arguments
/// * `items` - Slice of unified debt items
///
/// # Returns
/// Set of file paths that contain god objects
pub fn collect_god_object_files(items: &im::Vector<UnifiedDebtItem>) -> HashSet<PathBuf> {
    items
        .iter()
        .filter(|item| matches!(item.debt_type, DebtType::GodObject { .. }))
        .map(|item| item.location.file.clone())
        .collect()
}

/// Accumulate metrics from file-level debt items.
///
/// This is a pure function that processes file items, avoiding double-counting
/// for files that are also represented as god objects.
///
/// # Arguments
/// * `file_items` - Slice of file debt items
/// * `god_object_files` - Files already counted as god objects
///
/// # Returns
/// Accumulated metrics from file items
pub fn accumulate_file_metrics(
    file_items: &im::Vector<FileDebtItem>,
    god_object_files: &HashSet<PathBuf>,
) -> FileAccumulatedMetrics {
    let mut result = FileAccumulatedMetrics::default();

    for file_item in file_items {
        // Skip adding score if this file already has a god object item
        if !god_object_files.contains(&file_item.metrics.path) {
            result.additional_debt_score += file_item.score;
        }

        // Track file and its actual total lines
        result.unique_files.insert(
            file_item.metrics.path.clone(),
            file_item.metrics.total_lines,
        );

        // File-level impacts are typically larger
        result.complexity_reduction += file_item.impact.complexity_reduction;
        result.lines_reduction += (file_item.metrics.total_lines / 10) as u32;

        // Coverage improvement from fixing file-level issues
        if file_item.metrics.coverage_percent < 0.8 {
            result.coverage_improvement += (0.8 - file_item.metrics.coverage_percent) * 10.0;
        }
    }

    result
}

/// Scale raw coverage improvement to a displayable percentage.
///
/// # Arguments
/// * `raw_coverage` - Unscaled coverage improvement value
///
/// # Returns
/// Scaled coverage improvement (0-100)
pub fn scale_coverage_improvement(raw_coverage: f64) -> f64 {
    (raw_coverage * 5.0).min(100.0)
}

/// Calculate total lines of code from unique files.
///
/// # Arguments
/// * `unique_files` - Map of file paths to line counts
///
/// # Returns
/// Total lines of code across all unique files
pub fn calculate_total_loc(unique_files: &HashMap<PathBuf, usize>) -> usize {
    unique_files.values().sum()
}

/// Merge unique file maps, with the second map taking precedence.
///
/// # Arguments
/// * `base` - Base file map
/// * `overlay` - Overlay file map (values take precedence)
///
/// # Returns
/// Merged file map
pub fn merge_unique_files(
    base: HashMap<PathBuf, usize>,
    overlay: HashMap<PathBuf, usize>,
) -> HashMap<PathBuf, usize> {
    let mut result = base;
    result.extend(overlay);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_debt_density_normal() {
        assert_eq!(calculate_debt_density(100.0, 1000), 100.0);
        assert_eq!(calculate_debt_density(50.0, 500), 100.0);
        assert_eq!(calculate_debt_density(250.0, 5000), 50.0);
    }

    #[test]
    fn test_calculate_debt_density_zero_loc() {
        assert_eq!(calculate_debt_density(100.0, 0), 0.0);
        assert_eq!(calculate_debt_density(0.0, 0), 0.0);
    }

    #[test]
    fn test_calculate_debt_density_zero_score() {
        assert_eq!(calculate_debt_density(0.0, 1000), 0.0);
    }

    #[test]
    fn test_scale_coverage_improvement() {
        assert_eq!(scale_coverage_improvement(10.0), 50.0);
        assert_eq!(scale_coverage_improvement(20.0), 100.0);
        assert_eq!(scale_coverage_improvement(30.0), 100.0); // Capped at 100
        assert_eq!(scale_coverage_improvement(0.0), 0.0);
    }

    #[test]
    fn test_calculate_total_loc() {
        let mut files = HashMap::new();
        files.insert(PathBuf::from("a.rs"), 100);
        files.insert(PathBuf::from("b.rs"), 200);
        files.insert(PathBuf::from("c.rs"), 50);
        assert_eq!(calculate_total_loc(&files), 350);
    }

    #[test]
    fn test_calculate_total_loc_empty() {
        let files = HashMap::new();
        assert_eq!(calculate_total_loc(&files), 0);
    }

    #[test]
    fn test_merge_unique_files() {
        let mut base = HashMap::new();
        base.insert(PathBuf::from("a.rs"), 100);
        base.insert(PathBuf::from("b.rs"), 200);

        let mut overlay = HashMap::new();
        overlay.insert(PathBuf::from("b.rs"), 250); // Override
        overlay.insert(PathBuf::from("c.rs"), 50); // New

        let merged = merge_unique_files(base, overlay);
        assert_eq!(merged.get(&PathBuf::from("a.rs")), Some(&100));
        assert_eq!(merged.get(&PathBuf::from("b.rs")), Some(&250)); // Overlay wins
        assert_eq!(merged.get(&PathBuf::from("c.rs")), Some(&50));
    }
}

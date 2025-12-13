//! Coverage data loading functions.
//!
//! This module provides functions for loading coverage data.
//! These are I/O operations that live at the boundary of the system.

use crate::risk::lcov::LcovData;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Load coverage data from an LCOV file (I/O operation).
///
/// This function performs I/O to load coverage data from disk.
/// It should only be called at system boundaries.
pub fn load_coverage_file(lcov_path: &Path) -> Result<LcovData> {
    crate::risk::lcov::parse_lcov_file(lcov_path).context("Failed to parse LCOV file")
}

/// Load coverage data with optional path (I/O operation).
///
/// Returns None if no coverage file is specified.
pub fn load_coverage_data(coverage_file: Option<PathBuf>) -> Result<Option<LcovData>> {
    match coverage_file {
        Some(path) => load_coverage_file(&path).map(Some),
        None => Ok(None),
    }
}

/// Calculate coverage percentage from coverage data (pure).
pub fn calculate_coverage_percent(coverage_data: Option<&LcovData>) -> f64 {
    coverage_data.map_or(0.0, |data| {
        if data.total_lines > 0 {
            (data.lines_hit as f64 / data.total_lines as f64) * 100.0
        } else {
            0.0
        }
    })
}

/// Check if coverage data is available (pure).
pub fn has_coverage_data(coverage_data: Option<&LcovData>) -> bool {
    coverage_data.is_some()
}

/// Get overall coverage from data (pure).
pub fn get_overall_coverage(coverage_data: Option<&LcovData>) -> Option<f64> {
    coverage_data.map(|lcov| lcov.get_overall_coverage())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_coverage_percent_none() {
        let percent = calculate_coverage_percent(None);
        assert_eq!(percent, 0.0);
    }

    #[test]
    fn test_has_coverage_data() {
        assert!(!has_coverage_data(None));
    }

    #[test]
    fn test_get_overall_coverage_none() {
        assert!(get_overall_coverage(None).is_none());
    }
}

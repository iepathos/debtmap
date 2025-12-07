//! Sort functionality for results.

use crate::priority::UnifiedAnalysis;
use std::cmp::Ordering;

/// Sort criteria
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortCriteria {
    /// Sort by total score (default, descending)
    Score,
    /// Sort by coverage (ascending - worst coverage first)
    Coverage,
    /// Sort by complexity (descending - highest complexity first)
    Complexity,
    /// Sort by file path (alphabetical)
    FilePath,
    /// Sort by function name (alphabetical)
    FunctionName,
}

impl SortCriteria {
    /// Get display name (minimal lowercase style)
    pub fn display_name(&self) -> &'static str {
        match self {
            SortCriteria::Score => "score",
            SortCriteria::Coverage => "coverage",
            SortCriteria::Complexity => "complexity",
            SortCriteria::FilePath => "file path",
            SortCriteria::FunctionName => "function name",
        }
    }

    /// Get all sort criteria
    pub fn all() -> &'static [SortCriteria] {
        &[
            SortCriteria::Score,
            SortCriteria::Coverage,
            SortCriteria::Complexity,
            SortCriteria::FilePath,
            SortCriteria::FunctionName,
        ]
    }
}

/// Stable tiebreaker for items with equal primary sort values.
/// Compares by file path, then line number to ensure consistent ordering.
fn tiebreaker(analysis: &UnifiedAnalysis, a: usize, b: usize) -> Ordering {
    let item_a = analysis.items.get(a);
    let item_b = analysis.items.get(b);

    match (item_a, item_b) {
        (Some(a), Some(b)) => {
            // First compare by file path
            match a.location.file.cmp(&b.location.file) {
                Ordering::Equal => {
                    // If same file, compare by line number
                    a.location.line.cmp(&b.location.line)
                }
                other => other,
            }
        }
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
    }
}

/// Sort item indices based on criteria
pub fn sort_indices(indices: &mut [usize], analysis: &UnifiedAnalysis, criteria: SortCriteria) {
    match criteria {
        SortCriteria::Score => {
            // Sort by score descending (highest score first), with stable tiebreaker
            indices.sort_by(|&a, &b| {
                let score_a = analysis
                    .items
                    .get(a)
                    .map(|item| item.unified_score.final_score.value())
                    .unwrap_or(0.0);
                let score_b = analysis
                    .items
                    .get(b)
                    .map(|item| item.unified_score.final_score.value())
                    .unwrap_or(0.0);

                match score_b.partial_cmp(&score_a).unwrap_or(Ordering::Equal) {
                    Ordering::Equal => tiebreaker(analysis, a, b),
                    other => other,
                }
            });
        }
        SortCriteria::Coverage => {
            // Sort by coverage ascending (lowest coverage first, None first), with stable tiebreaker
            indices.sort_by(|&a, &b| {
                let cov_a = analysis
                    .items
                    .get(a)
                    .and_then(|item| item.transitive_coverage.as_ref().map(|c| c.direct));
                let cov_b = analysis
                    .items
                    .get(b)
                    .and_then(|item| item.transitive_coverage.as_ref().map(|c| c.direct));

                let primary = match (cov_a, cov_b) {
                    (None, None) => Ordering::Equal,
                    (None, Some(_)) => Ordering::Less, // No coverage is worst
                    (Some(_), None) => Ordering::Greater,
                    (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
                };

                match primary {
                    Ordering::Equal => tiebreaker(analysis, a, b),
                    other => other,
                }
            });
        }
        SortCriteria::Complexity => {
            // Sort by complexity descending (highest complexity first), with stable tiebreaker
            indices.sort_by(|&a, &b| {
                let comp_a = analysis.items.get(a).map(|item| item.cyclomatic_complexity);
                let comp_b = analysis.items.get(b).map(|item| item.cyclomatic_complexity);

                match comp_b.cmp(&comp_a) {
                    Ordering::Equal => tiebreaker(analysis, a, b),
                    other => other,
                }
            });
        }
        SortCriteria::FilePath => {
            // Sort by file path alphabetically, then by line number
            indices.sort_by(|&a, &b| {
                let path_a = analysis.items.get(a).map(|item| &item.location.file);
                let path_b = analysis.items.get(b).map(|item| &item.location.file);

                let primary = match (path_a, path_b) {
                    (Some(a), Some(b)) => a.cmp(b),
                    (None, None) => Ordering::Equal,
                    (None, Some(_)) => Ordering::Less,
                    (Some(_), None) => Ordering::Greater,
                };

                match primary {
                    Ordering::Equal => {
                        // If same file, sort by line number
                        let line_a = analysis.items.get(a).map(|item| item.location.line);
                        let line_b = analysis.items.get(b).map(|item| item.location.line);
                        line_a.cmp(&line_b)
                    }
                    other => other,
                }
            });
        }
        SortCriteria::FunctionName => {
            // Sort by function name alphabetically, then by file path and line
            indices.sort_by(|&a, &b| {
                let name_a = analysis.items.get(a).map(|item| &item.location.function);
                let name_b = analysis.items.get(b).map(|item| &item.location.function);

                let primary = match (name_a, name_b) {
                    (Some(a), Some(b)) => a.cmp(b),
                    (None, None) => Ordering::Equal,
                    (None, Some(_)) => Ordering::Less,
                    (Some(_), None) => Ordering::Greater,
                };

                match primary {
                    Ordering::Equal => tiebreaker(analysis, a, b),
                    other => other,
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_criteria_display() {
        assert_eq!(SortCriteria::Score.display_name(), "score");
        assert_eq!(SortCriteria::Coverage.display_name(), "coverage");
        assert_eq!(SortCriteria::Complexity.display_name(), "complexity");
        assert_eq!(SortCriteria::FilePath.display_name(), "file path");
        assert_eq!(SortCriteria::FunctionName.display_name(), "function name");
    }

    #[test]
    fn test_all_sort_criteria() {
        let all = SortCriteria::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&SortCriteria::Score));
        assert!(all.contains(&SortCriteria::Coverage));
        assert!(all.contains(&SortCriteria::Complexity));
    }
}

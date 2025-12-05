//! Sort functionality for results.

use crate::priority::UnifiedAnalysis;

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
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            SortCriteria::Score => "Score (High to Low)",
            SortCriteria::Coverage => "Coverage (Low to High)",
            SortCriteria::Complexity => "Complexity (High to Low)",
            SortCriteria::FilePath => "File Path (A-Z)",
            SortCriteria::FunctionName => "Function Name (A-Z)",
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

/// Sort item indices based on criteria
pub fn sort_indices(indices: &mut [usize], analysis: &UnifiedAnalysis, criteria: SortCriteria) {
    match criteria {
        SortCriteria::Score => {
            // Sort by score descending (highest score first)
            indices.sort_by(|&a, &b| {
                let score_a = analysis
                    .items
                    .get(a)
                    .map(|item| item.unified_score.final_score)
                    .unwrap_or(0.0);
                let score_b = analysis
                    .items
                    .get(b)
                    .map(|item| item.unified_score.final_score)
                    .unwrap_or(0.0);
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortCriteria::Coverage => {
            // Sort by coverage ascending (lowest coverage first, None first)
            indices.sort_by(|&a, &b| {
                let cov_a = analysis
                    .items
                    .get(a)
                    .and_then(|item| item.transitive_coverage.as_ref().map(|c| c.direct));
                let cov_b = analysis
                    .items
                    .get(b)
                    .and_then(|item| item.transitive_coverage.as_ref().map(|c| c.direct));

                match (cov_a, cov_b) {
                    (None, None) => std::cmp::Ordering::Equal,
                    (None, Some(_)) => std::cmp::Ordering::Less, // No coverage is worst
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
                }
            });
        }
        SortCriteria::Complexity => {
            // Sort by complexity descending (highest complexity first)
            indices.sort_by(|&a, &b| {
                let comp_a = analysis.items.get(a).map(|item| item.cyclomatic_complexity);
                let comp_b = analysis.items.get(b).map(|item| item.cyclomatic_complexity);

                comp_b.cmp(&comp_a)
            });
        }
        SortCriteria::FilePath => {
            // Sort by file path alphabetically
            indices.sort_by(|&a, &b| {
                let path_a = analysis.items.get(a).map(|item| &item.location.file);
                let path_b = analysis.items.get(b).map(|item| &item.location.file);

                match (path_a, path_b) {
                    (Some(a), Some(b)) => a.cmp(b),
                    (None, None) => std::cmp::Ordering::Equal,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (Some(_), None) => std::cmp::Ordering::Greater,
                }
            });
        }
        SortCriteria::FunctionName => {
            // Sort by function name alphabetically
            indices.sort_by(|&a, &b| {
                let name_a = analysis.items.get(a).map(|item| &item.location.function);
                let name_b = analysis.items.get(b).map(|item| &item.location.function);

                match (name_a, name_b) {
                    (Some(a), Some(b)) => a.cmp(b),
                    (None, None) => std::cmp::Ordering::Equal,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (Some(_), None) => std::cmp::Ordering::Greater,
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
        assert_eq!(SortCriteria::Score.display_name(), "Score (High to Low)");
        assert_eq!(
            SortCriteria::Coverage.display_name(),
            "Coverage (Low to High)"
        );
        assert_eq!(
            SortCriteria::Complexity.display_name(),
            "Complexity (High to Low)"
        );
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

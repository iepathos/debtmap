//! File-level cohesion metrics (spec 198)
//!
//! Measures how tightly related the functions within a file are by analyzing
//! function call patterns. High cohesion indicates functions work together frequently.

use super::format::round_ratio;
use serde::{Deserialize, Serialize};

/// File-level cohesion metrics (spec 198)
///
/// Measures how tightly related the functions within a file are by analyzing
/// function call patterns. High cohesion indicates functions work together frequently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohesionOutput {
    /// Cohesion score between 0.0 (no cohesion) and 1.0 (perfect cohesion)
    pub score: f64,
    /// Number of internal function calls (within the same file)
    pub internal_calls: usize,
    /// Number of external function calls (to other files)
    pub external_calls: usize,
    /// Classification based on cohesion thresholds
    pub classification: CohesionClassification,
    /// Number of functions analyzed
    pub functions_analyzed: usize,
}

/// Cohesion classification based on score thresholds (spec 198)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CohesionClassification {
    /// Cohesion >= 0.7
    High,
    /// Cohesion 0.4 - 0.7
    Medium,
    /// Cohesion < 0.4
    Low,
}

impl CohesionClassification {
    /// Classify cohesion score into high/medium/low
    pub fn from_score(score: f64) -> Self {
        if score >= 0.7 {
            CohesionClassification::High
        } else if score >= 0.4 {
            CohesionClassification::Medium
        } else {
            CohesionClassification::Low
        }
    }
}

impl std::fmt::Display for CohesionClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CohesionClassification::High => write!(f, "High"),
            CohesionClassification::Medium => write!(f, "Medium"),
            CohesionClassification::Low => write!(f, "Low"),
        }
    }
}

/// Codebase-wide cohesion statistics (spec 198)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohesionSummary {
    /// Average cohesion score across all analyzed files
    pub average: f64,
    /// Number of files with high cohesion (>= 0.7)
    pub high_cohesion_files: usize,
    /// Number of files with medium cohesion (0.4 - 0.7)
    pub medium_cohesion_files: usize,
    /// Number of files with low cohesion (< 0.4)
    pub low_cohesion_files: usize,
}

/// Build CohesionOutput from FileCohesionResult (spec 198)
pub fn build_cohesion_output(result: &crate::organization::FileCohesionResult) -> CohesionOutput {
    CohesionOutput {
        score: round_ratio(result.score),
        internal_calls: result.internal_calls,
        external_calls: result.external_calls,
        classification: CohesionClassification::from_score(result.score),
        functions_analyzed: result.functions_analyzed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cohesion_classification_from_score() {
        assert_eq!(
            CohesionClassification::from_score(0.8),
            CohesionClassification::High
        );
        assert_eq!(
            CohesionClassification::from_score(0.7),
            CohesionClassification::High
        );
        assert_eq!(
            CohesionClassification::from_score(0.5),
            CohesionClassification::Medium
        );
        assert_eq!(
            CohesionClassification::from_score(0.4),
            CohesionClassification::Medium
        );
        assert_eq!(
            CohesionClassification::from_score(0.3),
            CohesionClassification::Low
        );
        assert_eq!(
            CohesionClassification::from_score(0.0),
            CohesionClassification::Low
        );
    }

    #[test]
    fn test_cohesion_classification_display() {
        assert_eq!(format!("{}", CohesionClassification::High), "High");
        assert_eq!(format!("{}", CohesionClassification::Medium), "Medium");
        assert_eq!(format!("{}", CohesionClassification::Low), "Low");
    }
}

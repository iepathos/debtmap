//! # Complexity Pattern Detection
//!
//! Classifies complexity hotspots by their primary driver:
//! - **High Nesting**: Cognitive >> Cyclomatic (deep conditionals)
//! - **High Branching**: Many decision points, moderate depth
//! - **Mixed Complexity**: Both nesting and branching high
//! - **Chaotic Structure**: High entropy, inconsistent patterns
//! - **Moderate Complexity**: Approaching thresholds
//!
//! Each pattern gets tailored refactoring recommendations based on
//! the root cause identified through metric ratio analysis.

use serde::{Deserialize, Serialize};

/// Complexity pattern classification based on metric ratios
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplexityPattern {
    /// Deep nesting drives complexity (cognitive >> cyclomatic)
    HighNesting {
        nesting_depth: u32,
        cognitive_score: u32,
        ratio: f64, // cognitive/cyclomatic
    },
    /// Many decision points (high cyclomatic, moderate cognitive)
    HighBranching { branch_count: u32, cyclomatic: u32 },
    /// Both nesting and branching contribute to complexity
    MixedComplexity {
        nesting_depth: u32,
        cyclomatic: u32,
        cognitive: u32,
    },
    /// Inconsistent structure (high entropy)
    ChaoticStructure { entropy: f64, cyclomatic: u32 },
    /// Approaching complexity thresholds
    ModerateComplexity { cyclomatic: u32, cognitive: u32 },
}

/// Complexity metrics for pattern detection
#[derive(Debug, Clone)]
pub struct ComplexityMetrics {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub entropy_score: Option<f64>,
}

impl ComplexityPattern {
    /// Detect complexity pattern from metrics.
    ///
    /// # Pattern Detection Logic
    ///
    /// 1. **Chaotic Structure** (checked first): entropy >= 0.45
    ///    - High entropy indicates inconsistent patterns that make refactoring risky
    ///    - Should be standardized before other refactorings
    ///
    /// 2. **High Nesting**: cognitive/cyclomatic > 3.0 AND nesting >= 4
    ///    - Cognitive dominates cyclomatic (high ratio)
    ///    - Deep nesting (4+ levels) is the primary driver
    ///    - Refactoring: early returns, guard clauses, extract conditionals
    ///
    /// 3. **High Branching**: cyclomatic >= 15 AND ratio < 2.5
    ///    - Many decision points with moderate nesting
    ///    - Refactoring: extract functions, lookup tables, strategy pattern
    ///
    /// 4. **Mixed Complexity**: cyclomatic >= 12 AND cognitive >= 40 AND 2.5 <= ratio <= 3.5
    ///    - Both nesting and branching contribute significantly
    ///    - Refactoring: two-phase approach (flatten then extract)
    ///
    /// 5. **Moderate Complexity**: default
    ///    - Approaching thresholds but not critical
    ///    - Preventive refactoring recommended
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::priority::complexity_patterns::{ComplexityPattern, ComplexityMetrics};
    ///
    /// // High nesting example
    /// let metrics = ComplexityMetrics {
    ///     cyclomatic: 12,
    ///     cognitive: 50,  // 4.2x ratio
    ///     nesting: 5,
    ///     entropy_score: Some(0.35),
    /// };
    /// let pattern = ComplexityPattern::detect(&metrics);
    /// assert!(matches!(pattern, ComplexityPattern::HighNesting { .. }));
    ///
    /// // High branching example
    /// let metrics = ComplexityMetrics {
    ///     cyclomatic: 18,
    ///     cognitive: 35,  // 1.9x ratio
    ///     nesting: 2,
    ///     entropy_score: Some(0.30),
    /// };
    /// let pattern = ComplexityPattern::detect(&metrics);
    /// assert!(matches!(pattern, ComplexityPattern::HighBranching { .. }));
    /// ```
    pub fn detect(metrics: &ComplexityMetrics) -> Self {
        let ratio = metrics.cognitive as f64 / metrics.cyclomatic.max(1) as f64;

        // Chaotic: high entropy (check first - requires standardization before refactoring)
        if let Some(entropy) = metrics.entropy_score {
            if entropy >= 0.45 {
                return ComplexityPattern::ChaoticStructure {
                    entropy,
                    cyclomatic: metrics.cyclomatic,
                };
            }
        }

        // High nesting: cognitive dominates
        if ratio > 3.0 && metrics.nesting >= 4 {
            return ComplexityPattern::HighNesting {
                nesting_depth: metrics.nesting,
                cognitive_score: metrics.cognitive,
                ratio,
            };
        }

        // High branching: cyclomatic high, ratio moderate
        if metrics.cyclomatic >= 15 && ratio < 2.5 {
            return ComplexityPattern::HighBranching {
                branch_count: metrics.cyclomatic,
                cyclomatic: metrics.cyclomatic,
            };
        }

        // Mixed: both high
        if metrics.cyclomatic >= 12 && metrics.cognitive >= 40 && (2.5..=3.5).contains(&ratio) {
            return ComplexityPattern::MixedComplexity {
                nesting_depth: metrics.nesting,
                cyclomatic: metrics.cyclomatic,
                cognitive: metrics.cognitive,
            };
        }

        // Default: moderate
        ComplexityPattern::ModerateComplexity {
            cyclomatic: metrics.cyclomatic,
            cognitive: metrics.cognitive,
        }
    }

    /// Get a human-readable description of the pattern
    pub fn description(&self) -> &'static str {
        match self {
            ComplexityPattern::HighNesting { .. } => "Deep nesting drives complexity",
            ComplexityPattern::HighBranching { .. } => "Many decision points",
            ComplexityPattern::MixedComplexity { .. } => "Both nesting and branching high",
            ComplexityPattern::ChaoticStructure { .. } => "Inconsistent structure patterns",
            ComplexityPattern::ModerateComplexity { .. } => "Approaching complexity thresholds",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_high_nesting_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 12,
            cognitive: 50, // 4.2x ratio
            nesting: 5,
            entropy_score: Some(0.35),
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(pattern, ComplexityPattern::HighNesting { .. }));

        if let ComplexityPattern::HighNesting {
            nesting_depth,
            cognitive_score,
            ratio,
        } = pattern
        {
            assert_eq!(nesting_depth, 5);
            assert_eq!(cognitive_score, 50);
            assert!((ratio - 4.17).abs() < 0.01);
        }
    }

    #[test]
    fn detect_high_branching_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 18,
            cognitive: 35, // 1.9x ratio
            nesting: 2,
            entropy_score: Some(0.30),
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(pattern, ComplexityPattern::HighBranching { .. }));
    }

    #[test]
    fn detect_mixed_complexity_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 15,
            cognitive: 45, // 3.0x ratio
            nesting: 3,
            entropy_score: Some(0.32),
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(pattern, ComplexityPattern::MixedComplexity { .. }));
    }

    #[test]
    fn detect_chaotic_structure_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 12,
            cognitive: 30,
            nesting: 3,
            entropy_score: Some(0.50), // High entropy
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(
            pattern,
            ComplexityPattern::ChaoticStructure { .. }
        ));
    }

    #[test]
    fn detect_moderate_complexity_pattern() {
        let metrics = ComplexityMetrics {
            cyclomatic: 11,
            cognitive: 18,
            nesting: 2,
            entropy_score: Some(0.30),
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(matches!(
            pattern,
            ComplexityPattern::ModerateComplexity { .. }
        ));
    }

    #[test]
    fn chaotic_takes_precedence_over_nesting() {
        // High nesting metrics BUT high entropy
        let metrics = ComplexityMetrics {
            cyclomatic: 12,
            cognitive: 50,
            nesting: 5,
            entropy_score: Some(0.48), // High entropy takes precedence
        };

        let pattern = ComplexityPattern::detect(&metrics);
        assert!(
            matches!(pattern, ComplexityPattern::ChaoticStructure { .. }),
            "Chaotic structure should be detected before high nesting"
        );
    }

    #[test]
    fn ratio_boundary_conditions() {
        // Exactly at high nesting threshold
        let metrics = ComplexityMetrics {
            cyclomatic: 10,
            cognitive: 30, // Exactly 3.0x
            nesting: 4,
            entropy_score: Some(0.30),
        };

        let pattern = ComplexityPattern::detect(&metrics);
        // ratio > 3.0 requires strictly greater, so this should NOT be HighNesting
        assert!(
            !matches!(pattern, ComplexityPattern::HighNesting { .. }),
            "Exactly 3.0 ratio should not trigger HighNesting (requires > 3.0)"
        );
    }

    #[test]
    fn handles_zero_cyclomatic() {
        // Edge case: cyclomatic = 0 (shouldn't happen but test defensive coding)
        let metrics = ComplexityMetrics {
            cyclomatic: 0,
            cognitive: 10,
            nesting: 2,
            entropy_score: Some(0.30),
        };

        let pattern = ComplexityPattern::detect(&metrics);
        // Should use max(1) to avoid division by zero
        assert!(matches!(
            pattern,
            ComplexityPattern::ModerateComplexity { .. }
        ));
    }

    #[test]
    fn pattern_descriptions() {
        assert_eq!(
            ComplexityPattern::HighNesting {
                nesting_depth: 5,
                cognitive_score: 50,
                ratio: 4.0
            }
            .description(),
            "Deep nesting drives complexity"
        );

        assert_eq!(
            ComplexityPattern::HighBranching {
                branch_count: 18,
                cyclomatic: 18
            }
            .description(),
            "Many decision points"
        );
    }
}

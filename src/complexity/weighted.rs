// Weighted complexity scoring (spec 121)
//
// This module implements cognitive complexity weighted scoring to improve prioritization
// by emphasizing cognitive complexity over cyclomatic complexity. Research shows cognitive
// complexity correlates better with bug density and maintenance difficulty.

use serde::{Deserialize, Serialize};

/// Weights for combining cyclomatic and cognitive complexity
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComplexityWeights {
    pub cyclomatic: f64,
    pub cognitive: f64,
}

impl Default for ComplexityWeights {
    fn default() -> Self {
        Self {
            cyclomatic: 0.3,
            cognitive: 0.7,
        }
    }
}

impl ComplexityWeights {
    /// Validate that weights sum to 1.0
    pub fn validate(&self) -> Result<(), String> {
        let sum = self.cyclomatic + self.cognitive;
        if (sum - 1.0).abs() > 0.001 {
            return Err(format!("Complexity weights must sum to 1.0, got {}", sum));
        }
        if self.cyclomatic < 0.0 || self.cognitive < 0.0 {
            return Err("Complexity weights must be non-negative".to_string());
        }
        Ok(())
    }

    /// Determine which metric is dominant
    pub fn dominant_metric(&self) -> ComplexityMetric {
        if self.cognitive > self.cyclomatic {
            ComplexityMetric::Cognitive
        } else {
            ComplexityMetric::Cyclomatic
        }
    }

    /// Create weights adjusted for function role
    /// Pure functions balance both metrics (50/50)
    /// Business logic emphasizes cognitive complexity (25/75)
    pub fn for_role(role: crate::priority::FunctionRole) -> Self {
        use crate::priority::FunctionRole;

        match role {
            // Pure functions: balance cyclomatic and cognitive equally
            FunctionRole::PureLogic => Self {
                cyclomatic: 0.5,
                cognitive: 0.5,
            },
            // Orchestrators and entry points: heavily favor cognitive
            FunctionRole::Orchestrator | FunctionRole::EntryPoint => Self {
                cyclomatic: 0.25,
                cognitive: 0.75,
            },
            // I/O wrappers and pattern matching: default weights
            FunctionRole::IOWrapper | FunctionRole::PatternMatch | FunctionRole::Unknown => {
                Self::default()
            }
        }
    }
}

/// Which complexity metric is dominant in scoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityMetric {
    Cyclomatic,
    Cognitive,
}

/// Normalization parameters for complexity metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComplexityNormalization {
    pub max_cyclomatic: f64,
    pub max_cognitive: f64,
}

impl Default for ComplexityNormalization {
    fn default() -> Self {
        Self {
            max_cyclomatic: 50.0,
            max_cognitive: 100.0,
        }
    }
}

impl ComplexityNormalization {
    /// Create normalization parameters from actual codebase analysis
    /// Calculates max values from the codebase with 20% headroom
    pub fn from_analysis<I>(complexity_pairs: I) -> Self
    where
        I: Iterator<Item = (u32, u32)>,
    {
        let mut max_cyclomatic = 0u32;
        let mut max_cognitive = 0u32;

        for (cyclomatic, cognitive) in complexity_pairs {
            max_cyclomatic = max_cyclomatic.max(cyclomatic);
            max_cognitive = max_cognitive.max(cognitive);
        }

        // Add 20% headroom and ensure minimums
        Self {
            max_cyclomatic: ((max_cyclomatic as f64 * 1.2).max(10.0)),
            max_cognitive: ((max_cognitive as f64 * 1.2).max(10.0)),
        }
    }

    /// Normalize cyclomatic complexity to 0-100 scale
    pub fn normalize_cyclomatic(&self, value: u32) -> f64 {
        (value as f64 / self.max_cyclomatic).min(1.0) * 100.0
    }

    /// Normalize cognitive complexity to 0-100 scale
    pub fn normalize_cognitive(&self, value: u32) -> f64 {
        (value as f64 / self.max_cognitive).min(1.0) * 100.0
    }
}

/// Combined weighted complexity score
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedComplexity {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub weighted_score: f64,
    pub weights_used: ComplexityWeights,
}

impl WeightedComplexity {
    /// Calculate weighted complexity score
    pub fn calculate(
        cyclomatic: u32,
        cognitive: u32,
        weights: ComplexityWeights,
        normalization: &ComplexityNormalization,
    ) -> Self {
        let normalized_cyclomatic = normalization.normalize_cyclomatic(cyclomatic);
        let normalized_cognitive = normalization.normalize_cognitive(cognitive);

        let weighted_score =
            weights.cyclomatic * normalized_cyclomatic + weights.cognitive * normalized_cognitive;

        Self {
            cyclomatic,
            cognitive,
            weighted_score,
            weights_used: weights,
        }
    }

    /// Determine which metric is dominant in this score
    pub fn dominant_metric(&self) -> ComplexityMetric {
        self.weights_used.dominant_metric()
    }

    /// Check if metrics diverge significantly (one high, one low)
    pub fn metrics_diverge(&self) -> bool {
        let ratio = if self.cyclomatic > self.cognitive {
            self.cyclomatic as f64 / (self.cognitive.max(1) as f64)
        } else {
            self.cognitive as f64 / (self.cyclomatic.max(1) as f64)
        };
        ratio >= 3.0
    }

    /// Format dominant metric for display
    pub fn dominant_metric_name(&self) -> &'static str {
        match self.dominant_metric() {
            ComplexityMetric::Cognitive => "cognitive-driven",
            ComplexityMetric::Cyclomatic => "cyclomatic-driven",
        }
    }

    /// Format complexity information with weighted score
    /// Returns: "cyclomatic=15, cognitive=3 → weighted=11.1 (cognitive-driven)"
    pub fn format_complexity_info(&self) -> String {
        format!(
            "cyclomatic={}, cognitive={} → weighted={:.1} ({})",
            self.cyclomatic,
            self.cognitive,
            self.weighted_score,
            self.dominant_metric_name()
        )
    }

    /// Format complexity details for verbose output
    /// Returns multi-line breakdown of the scoring
    pub fn format_complexity_details(&self) -> String {
        format!(
            "Cyclomatic: {} (weight: {:.0}%)\nCognitive: {} (weight: {:.0}%)\nWeighted Score: {:.1} ({})",
            self.cyclomatic,
            self.weights_used.cyclomatic * 100.0,
            self.cognitive,
            self.weights_used.cognitive * 100.0,
            self.weighted_score,
            self.dominant_metric_name()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_weights_sum_to_one() {
        let weights = ComplexityWeights::default();
        assert!(weights.validate().is_ok());
        assert!((weights.cyclomatic + weights.cognitive - 1.0).abs() < 0.001);
    }

    #[test]
    fn default_weights_favor_cognitive() {
        let weights = ComplexityWeights::default();
        assert!(weights.cognitive > weights.cyclomatic);
        assert_eq!(weights.cognitive, 0.7);
        assert_eq!(weights.cyclomatic, 0.3);
    }

    #[test]
    fn weights_validation_rejects_invalid_sum() {
        let weights = ComplexityWeights {
            cyclomatic: 0.5,
            cognitive: 0.6,
        };
        assert!(weights.validate().is_err());
    }

    #[test]
    fn weights_validation_rejects_negative() {
        let weights = ComplexityWeights {
            cyclomatic: -0.1,
            cognitive: 1.1,
        };
        assert!(weights.validate().is_err());
    }

    #[test]
    fn normalization_scales_to_0_100() {
        let norm = ComplexityNormalization::default();

        assert_eq!(norm.normalize_cyclomatic(0), 0.0);
        assert_eq!(norm.normalize_cyclomatic(25), 50.0);
        assert_eq!(norm.normalize_cyclomatic(50), 100.0);

        assert_eq!(norm.normalize_cognitive(0), 0.0);
        assert_eq!(norm.normalize_cognitive(50), 50.0);
        assert_eq!(norm.normalize_cognitive(100), 100.0);
    }

    #[test]
    fn normalization_caps_at_100() {
        let norm = ComplexityNormalization::default();

        assert_eq!(norm.normalize_cyclomatic(100), 100.0);
        assert_eq!(norm.normalize_cognitive(200), 100.0);
    }

    #[test]
    fn from_analysis_calculates_with_headroom() {
        let complexity_pairs = vec![(10, 20), (15, 30), (20, 40)];
        let norm = ComplexityNormalization::from_analysis(complexity_pairs.into_iter());

        // Max cyclomatic is 20, with 20% headroom = 24
        assert!((norm.max_cyclomatic - 24.0).abs() < 0.1);
        // Max cognitive is 40, with 20% headroom = 48
        assert!((norm.max_cognitive - 48.0).abs() < 0.1);
    }

    #[test]
    fn from_analysis_ensures_minimum_values() {
        let complexity_pairs = vec![(1, 2), (2, 3)];
        let norm = ComplexityNormalization::from_analysis(complexity_pairs.into_iter());

        // Even with small values, should use minimums of 10.0
        assert!(norm.max_cyclomatic >= 10.0);
        assert!(norm.max_cognitive >= 10.0);
    }

    #[test]
    fn from_analysis_handles_empty_iterator() {
        let complexity_pairs: Vec<(u32, u32)> = vec![];
        let norm = ComplexityNormalization::from_analysis(complexity_pairs.into_iter());

        // Should use minimum values
        assert_eq!(norm.max_cyclomatic, 10.0);
        assert_eq!(norm.max_cognitive, 10.0);
    }

    #[test]
    fn cognitive_weighted_reduces_mapping_pattern_score() {
        let weights = ComplexityWeights::default(); // 0.3 cyclo, 0.7 cognitive
        let norm = ComplexityNormalization::default();

        let weighted = WeightedComplexity::calculate(15, 3, weights, &norm);

        // 15/50 * 100 * 0.3 + 3/100 * 100 * 0.7 = 9.0 + 2.1 = 11.1
        assert!((weighted.weighted_score - 11.1).abs() < 0.1);
    }

    #[test]
    fn high_cognitive_scores_higher_than_high_cyclomatic() {
        let weights = ComplexityWeights::default();
        let norm = ComplexityNormalization::default();

        let high_cyclo_low_cog = WeightedComplexity::calculate(20, 5, weights, &norm);
        let low_cyclo_high_cog = WeightedComplexity::calculate(8, 25, weights, &norm);

        // With 70% cognitive weight, high cognitive should score higher
        assert!(
            low_cyclo_high_cog.weighted_score > high_cyclo_low_cog.weighted_score,
            "Expected {} > {}",
            low_cyclo_high_cog.weighted_score,
            high_cyclo_low_cog.weighted_score
        );
    }

    #[test]
    fn dominant_metric_identifies_cognitive() {
        let weights = ComplexityWeights::default();
        assert_eq!(weights.dominant_metric(), ComplexityMetric::Cognitive);
    }

    #[test]
    fn dominant_metric_identifies_cyclomatic() {
        let weights = ComplexityWeights {
            cyclomatic: 0.6,
            cognitive: 0.4,
        };
        assert_eq!(weights.dominant_metric(), ComplexityMetric::Cyclomatic);
    }

    #[test]
    fn for_role_pure_logic_balances_metrics() {
        use crate::priority::FunctionRole;
        let weights = ComplexityWeights::for_role(FunctionRole::PureLogic);
        assert_eq!(weights.cyclomatic, 0.5);
        assert_eq!(weights.cognitive, 0.5);
        assert!(weights.validate().is_ok());
    }

    #[test]
    fn for_role_orchestrator_favors_cognitive() {
        use crate::priority::FunctionRole;
        let weights = ComplexityWeights::for_role(FunctionRole::Orchestrator);
        assert_eq!(weights.cyclomatic, 0.25);
        assert_eq!(weights.cognitive, 0.75);
        assert!(weights.validate().is_ok());
    }

    #[test]
    fn for_role_entry_point_favors_cognitive() {
        use crate::priority::FunctionRole;
        let weights = ComplexityWeights::for_role(FunctionRole::EntryPoint);
        assert_eq!(weights.cyclomatic, 0.25);
        assert_eq!(weights.cognitive, 0.75);
        assert!(weights.validate().is_ok());
    }

    #[test]
    fn for_role_io_wrapper_uses_defaults() {
        use crate::priority::FunctionRole;
        let weights = ComplexityWeights::for_role(FunctionRole::IOWrapper);
        let defaults = ComplexityWeights::default();
        assert_eq!(weights.cyclomatic, defaults.cyclomatic);
        assert_eq!(weights.cognitive, defaults.cognitive);
    }

    #[test]
    fn metrics_diverge_detects_large_difference() {
        let weights = ComplexityWeights::default();
        let norm = ComplexityNormalization::default();

        // 15 cyclo vs 3 cognitive = 5x ratio
        let divergent = WeightedComplexity::calculate(15, 3, weights, &norm);
        assert!(divergent.metrics_diverge());

        // 10 cyclo vs 12 cognitive = 1.2x ratio
        let similar = WeightedComplexity::calculate(10, 12, weights, &norm);
        assert!(!similar.metrics_diverge());
    }

    #[test]
    fn format_complexity_info_includes_all_metrics() {
        let weights = ComplexityWeights::default();
        let norm = ComplexityNormalization::default();
        let weighted = WeightedComplexity::calculate(15, 3, weights, &norm);

        let formatted = weighted.format_complexity_info();

        assert!(formatted.contains("cyclomatic=15"));
        assert!(formatted.contains("cognitive=3"));
        assert!(formatted.contains("weighted="));
        assert!(formatted.contains("cognitive-driven"));
    }

    #[test]
    fn format_complexity_details_shows_weights() {
        let weights = ComplexityWeights::default();
        let norm = ComplexityNormalization::default();
        let weighted = WeightedComplexity::calculate(15, 3, weights, &norm);

        let formatted = weighted.format_complexity_details();

        assert!(formatted.contains("Cyclomatic: 15"));
        assert!(formatted.contains("Cognitive: 3"));
        assert!(formatted.contains("weight: 30%"));
        assert!(formatted.contains("weight: 70%"));
        assert!(formatted.contains("Weighted Score:"));
    }
}

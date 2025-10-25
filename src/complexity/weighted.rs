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
}

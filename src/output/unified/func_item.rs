//! Function-level debt item output types and conversions (spec 108)
//!
//! Provides `FunctionDebtItemOutput` struct and conversion from `UnifiedDebtItem`.

use super::dependencies::{Dependencies, PurityAnalysis, RecommendationOutput};
use super::format::{assert_ratio_invariants, assert_score_invariants};
use super::format::{round_ratio, round_score};
use super::location::UnifiedLocation;
use super::patterns::{extract_complexity_pattern, extract_pattern_data};
use super::priority::{assert_priority_invariants, Priority};
use crate::priority::{DebtType, FunctionRole, UnifiedDebtItem};
use serde::{Deserialize, Serialize};

/// Function-level debt item in unified format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDebtItemOutput {
    pub score: f64,
    pub category: String,
    pub priority: Priority,
    pub location: UnifiedLocation,
    pub metrics: FunctionMetricsOutput,
    pub debt_type: DebtType,
    pub function_role: FunctionRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purity_analysis: Option<PurityAnalysis>,
    pub dependencies: Dependencies,
    pub recommendation: RecommendationOutput,
    pub impact: FunctionImpactOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoring_details: Option<FunctionScoringDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjusted_complexity: Option<AdjustedComplexity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complexity_pattern: Option<String>,
    /// "state_machine" | "coordinator"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_confidence: Option<f64>,
    /// Pattern-specific metrics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_details: Option<serde_json::Value>,
}

impl FunctionDebtItemOutput {
    /// Assert all invariants hold for this function debt item (spec 230)
    #[cfg(debug_assertions)]
    pub fn assert_invariants(&self) {
        assert_score_invariants(self.score, "function.score");
        assert_priority_invariants(&self.priority, self.score);

        if let Some(coverage) = self.metrics.coverage {
            assert_ratio_invariants(coverage, "function.metrics.coverage");
        }

        if let Some(entropy) = self.metrics.entropy_score {
            assert_ratio_invariants(entropy, "function.metrics.entropy_score");
        }

        if let Some(ref purity) = self.purity_analysis {
            assert_ratio_invariants(
                purity.confidence as f64,
                "function.purity_analysis.confidence",
            );
        }

        if let Some(confidence) = self.pattern_confidence {
            assert_ratio_invariants(confidence, "function.pattern_confidence");
        }
    }

    /// No-op in release builds
    #[cfg(not(debug_assertions))]
    #[inline]
    pub fn assert_invariants(&self) {}

    pub fn from_function_item(item: &UnifiedDebtItem, include_scoring_details: bool) -> Self {
        // Apply rounding for clean output (spec 230)
        let rounded_score = round_score(item.unified_score.final_score.value());
        let complexity_pattern = extract_complexity_pattern(
            &item.recommendation.rationale,
            &item.recommendation.primary_action,
        );
        let (pattern_type, pattern_confidence, pattern_details) =
            extract_pattern_data(&item.language_specific);

        // Round coverage and entropy if present
        let rounded_coverage = item
            .transitive_coverage
            .as_ref()
            .map(|c| round_ratio(c.transitive));
        let rounded_entropy = item
            .entropy_details
            .as_ref()
            .map(|e| round_ratio(e.entropy_score));

        // Round pattern confidence if present
        let rounded_pattern_confidence = pattern_confidence.map(round_ratio);

        FunctionDebtItemOutput {
            score: rounded_score,
            category: crate::priority::DebtCategory::from_debt_type(&item.debt_type).to_string(),
            priority: Priority::from_score(rounded_score),
            location: UnifiedLocation {
                file: item.location.file.to_string_lossy().to_string(),
                line: Some(item.location.line),
                function: Some(item.location.function.clone()),
                file_context_label: item.file_context.as_ref().map(|ctx| {
                    use crate::priority::scoring::file_context_scoring::context_label;
                    context_label(ctx).to_string()
                }),
            },
            metrics: FunctionMetricsOutput {
                cyclomatic_complexity: item.cyclomatic_complexity,
                cognitive_complexity: item.cognitive_complexity,
                length: item.function_length,
                nesting_depth: item.nesting_depth,
                coverage: rounded_coverage,
                uncovered_lines: None, // Not currently tracked
                entropy_score: rounded_entropy,
            },
            debt_type: item.debt_type.clone(),
            function_role: item.function_role,
            purity_analysis: item.is_pure.map(|is_pure| PurityAnalysis {
                is_pure,
                confidence: item.purity_confidence.unwrap_or(0.0),
                side_effects: None,
            }),
            dependencies: Dependencies {
                upstream_count: item.upstream_dependencies,
                downstream_count: item.downstream_dependencies,
                upstream_callers: item.upstream_callers.clone(),
                downstream_callees: item.downstream_callees.clone(),
            },
            recommendation: RecommendationOutput {
                action: item.recommendation.primary_action.clone(),
                priority: None,
                implementation_steps: item.recommendation.implementation_steps.clone(),
            },
            impact: FunctionImpactOutput {
                coverage_improvement: round_ratio(item.expected_impact.coverage_improvement),
                complexity_reduction: round_ratio(item.expected_impact.complexity_reduction),
                risk_reduction: round_ratio(item.expected_impact.risk_reduction),
            },
            scoring_details: if include_scoring_details {
                Some(FunctionScoringDetails {
                    coverage_score: round_score(item.unified_score.coverage_factor),
                    complexity_score: round_score(item.unified_score.complexity_factor),
                    dependency_score: round_score(item.unified_score.dependency_factor),
                    base_score: round_score(
                        item.unified_score.complexity_factor
                            + item.unified_score.coverage_factor
                            + item.unified_score.dependency_factor,
                    ),
                    entropy_dampening: item
                        .entropy_details
                        .as_ref()
                        .map(|e| round_ratio(e.dampening_factor)),
                    role_multiplier: round_ratio(item.unified_score.role_multiplier),
                    final_score: rounded_score,
                    purity_factor: item.unified_score.purity_factor.map(round_ratio),
                    refactorability_factor: item
                        .unified_score
                        .refactorability_factor
                        .map(round_ratio),
                    pattern_factor: item.unified_score.pattern_factor.map(round_ratio),
                })
            } else {
                None
            },
            adjusted_complexity: item.entropy_details.as_ref().map(|e| AdjustedComplexity {
                // Dampened cyclomatic = cyclomatic * dampening_factor (spec 232)
                // When dampening_factor = 1.0, dampened_cyclomatic equals original cyclomatic
                dampened_cyclomatic: round_score(
                    item.cyclomatic_complexity as f64 * e.dampening_factor,
                ),
                dampening_factor: round_ratio(e.dampening_factor),
            }),
            complexity_pattern,
            pattern_type,
            pattern_confidence: rounded_pattern_confidence,
            pattern_details,
        }
    }
}

/// Adjusted complexity based on entropy analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedComplexity {
    pub dampened_cyclomatic: f64,
    pub dampening_factor: f64,
}

/// Function metrics in unified format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetricsOutput {
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub length: usize,
    pub nesting_depth: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uncovered_lines: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy_score: Option<f64>,
}

/// Function impact metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionImpactOutput {
    pub coverage_improvement: f64,
    pub complexity_reduction: f64,
    pub risk_reduction: f64,
}

/// Function scoring details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionScoringDetails {
    pub coverage_score: f64,
    pub complexity_score: f64,
    pub dependency_score: f64,
    pub base_score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy_dampening: Option<f64>,
    pub role_multiplier: f64,
    pub final_score: f64,
    // Data flow factors (spec 218)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purity_factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refactorability_factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_factor: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_debt_item_serialization_roundtrip() {
        let item = FunctionDebtItemOutput {
            score: 42.57,
            category: "Testing".to_string(),
            priority: Priority::from_score(42.57),
            location: UnifiedLocation {
                file: "test.rs".to_string(),
                line: Some(10),
                function: Some("test_fn".to_string()),
                file_context_label: None,
            },
            metrics: FunctionMetricsOutput {
                cyclomatic_complexity: 5,
                cognitive_complexity: 3,
                length: 20,
                nesting_depth: 2,
                coverage: Some(0.8),
                uncovered_lines: None,
                entropy_score: Some(0.5),
            },
            debt_type: DebtType::TestingGap {
                coverage: 0.8,
                cyclomatic: 5,
                cognitive: 3,
            },
            function_role: FunctionRole::Unknown,
            purity_analysis: Some(PurityAnalysis {
                is_pure: true,
                confidence: 0.9,
                side_effects: None,
            }),
            dependencies: Dependencies {
                upstream_count: 2,
                downstream_count: 3,
                upstream_callers: vec!["caller1".to_string()],
                downstream_callees: vec!["callee1".to_string()],
            },
            recommendation: RecommendationOutput {
                action: "Add tests".to_string(),
                priority: None,
                implementation_steps: vec![],
            },
            impact: FunctionImpactOutput {
                coverage_improvement: 0.2,
                complexity_reduction: 0.1,
                risk_reduction: 0.15,
            },
            scoring_details: None,
            adjusted_complexity: None,
            complexity_pattern: None,
            pattern_type: None,
            pattern_confidence: None,
            pattern_details: None,
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: FunctionDebtItemOutput = serde_json::from_str(&json).unwrap();

        // Key fields should be preserved
        assert_eq!(item.score, deserialized.score);
        assert!(matches!(deserialized.priority, Priority::Medium));
        assert_eq!(item.metrics.coverage, deserialized.metrics.coverage);
    }

    #[test]
    fn test_no_floating_point_noise_in_output() {
        let item = FunctionDebtItemOutput {
            score: 42.57, // Already rounded
            category: "Testing".to_string(),
            priority: Priority::Medium,
            location: UnifiedLocation {
                file: "test.rs".to_string(),
                line: Some(10),
                function: Some("test_fn".to_string()),
                file_context_label: None,
            },
            metrics: FunctionMetricsOutput {
                cyclomatic_complexity: 5,
                cognitive_complexity: 3,
                length: 20,
                nesting_depth: 2,
                coverage: Some(0.8), // Already rounded
                uncovered_lines: None,
                entropy_score: Some(0.5), // Already rounded
            },
            debt_type: crate::priority::DebtType::TestingGap {
                coverage: 0.8,
                cyclomatic: 5,
                cognitive: 3,
            },
            function_role: FunctionRole::Unknown,
            purity_analysis: None,
            dependencies: Dependencies {
                upstream_count: 0,
                downstream_count: 0,
                upstream_callers: vec![],
                downstream_callees: vec![],
            },
            recommendation: RecommendationOutput {
                action: "Add tests".to_string(),
                priority: None,
                implementation_steps: vec![],
            },
            impact: FunctionImpactOutput {
                coverage_improvement: 0.2,
                complexity_reduction: 0.1,
                risk_reduction: 0.15,
            },
            scoring_details: None,
            adjusted_complexity: None,
            complexity_pattern: None,
            pattern_type: None,
            pattern_confidence: None,
            pattern_details: None,
        };

        let json = serde_json::to_string(&item).unwrap();

        // Check for typical floating-point noise patterns
        let noise_patterns = ["9999999999", "0000000001"];

        for pattern in noise_patterns {
            assert!(
                !json.contains(pattern),
                "Found floating-point noise '{}' in: {}",
                pattern,
                json
            );
        }
    }

    #[test]
    fn test_adjusted_complexity_serialization() {
        let adjusted = AdjustedComplexity {
            dampened_cyclomatic: 11.0,
            dampening_factor: 1.0,
        };
        let json = serde_json::to_string(&adjusted).unwrap();
        assert!(json.contains("\"dampened_cyclomatic\":11.0"));
        assert!(json.contains("\"dampening_factor\":1.0"));
    }
}

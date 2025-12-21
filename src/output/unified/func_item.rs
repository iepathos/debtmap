//! Function-level debt item output types and conversions (spec 108)
//!
//! Provides `FunctionDebtItemOutput` struct and conversion from `UnifiedDebtItem`.

use super::dependencies::{Dependencies, PurityAnalysis, RecommendationOutput};
use super::format::{assert_ratio_invariants, assert_score_invariants};
use super::format::{round_ratio, round_score};
use super::location::UnifiedLocation;
use super::patterns::{extract_complexity_pattern, extract_pattern_data};
use super::priority::{assert_priority_invariants, Priority};
use crate::core::PurityLevel;
use crate::priority::{DebtType, FunctionRole, UnifiedDebtItem};
use serde::{Deserialize, Serialize};

/// Generate side effects description based on purity level.
///
/// This provides human-readable reasons for why a function is not strictly pure,
/// derived from the `PurityLevel` classification.
fn generate_side_effects_from_purity(
    is_pure: bool,
    purity_level: Option<PurityLevel>,
) -> Option<Vec<String>> {
    if is_pure {
        return None;
    }

    let effects = match purity_level {
        Some(PurityLevel::Impure) => {
            vec!["Has side effects (I/O, mutations, or external state modification)".to_string()]
        }
        Some(PurityLevel::ReadOnly) => {
            vec!["Reads external state (but does not modify it)".to_string()]
        }
        Some(PurityLevel::LocallyPure) => {
            vec!["Has local mutations only (no external side effects)".to_string()]
        }
        Some(PurityLevel::StrictlyPure) => {
            // Shouldn't happen if is_pure is false, but handle gracefully
            return None;
        }
        None => {
            vec!["Function may have side effects".to_string()]
        }
    };

    Some(effects)
}

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
    /// Context window suggestion for AI agents (spec 263)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextSuggestionOutput>,
    /// Git history context for understanding code stability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_history: Option<GitHistoryOutput>,
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
        let rounded_score = round_score(item.unified_score.final_score);
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
            .entropy_analysis
            .as_ref()
            .map(|e| round_ratio(e.entropy_score));

        // Extract pattern_repetition and branch_similarity from entropy analysis
        let (pattern_repetition, branch_similarity) =
            if let Some(ref analysis) = item.entropy_analysis {
                (
                    Some(round_ratio(analysis.pattern_repetition)),
                    Some(round_ratio(analysis.branch_similarity)),
                )
            } else {
                (None, None)
            };

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
                pattern_repetition,
                branch_similarity,
                entropy_adjusted_cognitive: item
                    .entropy_analysis
                    .as_ref()
                    .map(|e| e.adjusted_complexity),
                transitive_coverage: item
                    .transitive_coverage
                    .as_ref()
                    .map(|c| round_ratio(c.transitive)),
            },
            debt_type: item.debt_type.clone(),
            function_role: item.function_role,
            purity_analysis: item.is_pure.map(|is_pure| {
                let purity_level = item
                    .purity_level
                    .as_ref()
                    .map(|level| format!("{:?}", level));
                let side_effects = generate_side_effects_from_purity(is_pure, item.purity_level);
                PurityAnalysis {
                    is_pure,
                    confidence: item.purity_confidence.unwrap_or(0.0),
                    purity_level,
                    side_effects,
                }
            }),
            dependencies: {
                let upstream = item.upstream_dependencies;
                let downstream = item.downstream_dependencies;
                let blast_radius = upstream + downstream;
                let critical_path = upstream > 5 || downstream > 10;
                let instability = if blast_radius > 0 {
                    Some(round_ratio(downstream as f64 / blast_radius as f64))
                } else {
                    None
                };
                let coupling_classification =
                    derive_coupling_classification(upstream, downstream, instability);
                Dependencies {
                    upstream_count: upstream,
                    downstream_count: downstream,
                    upstream_callers: item.upstream_callers.clone(),
                    downstream_callees: item.downstream_callees.clone(),
                    blast_radius,
                    critical_path,
                    coupling_classification,
                    instability,
                }
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
                        .entropy_analysis
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
                    // Additional multipliers for LLM output (Spec 264)
                    structural_multiplier: item
                        .unified_score
                        .structural_multiplier
                        .map(round_ratio),
                    context_multiplier: item.context_multiplier.map(round_ratio),
                    contextual_risk_multiplier: item
                        .unified_score
                        .contextual_risk_multiplier
                        .map(round_ratio),
                    pre_normalization_score: item
                        .unified_score
                        .pre_normalization_score
                        .map(round_score),
                })
            } else {
                None
            },
            adjusted_complexity: item.entropy_analysis.as_ref().map(|e| AdjustedComplexity {
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
            context: item
                .context_suggestion
                .as_ref()
                .map(ContextSuggestionOutput::from_context_suggestion),
            git_history: item
                .contextual_risk
                .as_ref()
                .and_then(GitHistoryOutput::from_contextual_risk),
        }
    }
}

/// Derive coupling classification based on upstream/downstream dependencies
///
/// Classifications based on dependency patterns:
/// - "Leaf Module": No downstream dependencies (stable, pure consumer)
/// - "Stable Core": High upstream, low instability (heavily depended upon)
/// - "Hub": High upstream and downstream (central integration point)
/// - "Connector": Balanced upstream/downstream (mediator)
/// - None: Low dependency counts, no significant classification
fn derive_coupling_classification(
    upstream: usize,
    downstream: usize,
    instability: Option<f64>,
) -> Option<String> {
    let blast_radius = upstream + downstream;

    // Low dependency counts - no meaningful classification
    if blast_radius < 3 {
        return None;
    }

    // Leaf module: no downstream, only consumes
    if downstream == 0 && upstream > 0 {
        return Some("Leaf Module".to_string());
    }

    let inst = instability.unwrap_or(0.5);

    // Stable core: high upstream (many callers), low instability
    if upstream >= 5 && inst < 0.3 {
        return Some("Stable Core".to_string());
    }

    // Hub: high both upstream and downstream
    if upstream >= 5 && downstream >= 5 {
        return Some("Hub".to_string());
    }

    // Connector: moderate dependencies in both directions
    if upstream >= 2 && downstream >= 2 {
        return Some("Connector".to_string());
    }

    None
}

/// Adjusted complexity based on entropy analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedComplexity {
    pub dampened_cyclomatic: f64,
    pub dampening_factor: f64,
}

/// Function metrics in unified format
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// Pattern repetition score (0.0-1.0, higher = more repetitive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_repetition: Option<f64>,
    /// Branch similarity score (0.0-1.0, higher = similar branches)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_similarity: Option<f64>,
    /// Entropy-adjusted cognitive complexity (Spec 264)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy_adjusted_cognitive: Option<u32>,
    /// Transitive coverage from callers (Spec 264)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transitive_coverage: Option<f64>,
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
    // Additional multipliers for LLM output (Spec 264)
    /// Structural multiplier for deep nesting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structural_multiplier: Option<f64>,
    /// Context multiplier based on file context (production vs test)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_multiplier: Option<f64>,
    /// Contextual risk multiplier from git history analysis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contextual_risk_multiplier: Option<f64>,
    /// Score before normalization/clamping was applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_normalization_score: Option<f64>,
}

/// Context suggestion output for AI agents (spec 263)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSuggestionOutput {
    /// Primary code range to read
    pub primary: FileRangeOutput,
    /// Related context ranges
    pub related: Vec<RelatedContextOutput>,
    /// Estimated total lines to read
    pub total_lines: u32,
    /// Confidence that this context is sufficient (0.0-1.0)
    pub completeness_confidence: f64,
}

/// File range output (spec 263)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRangeOutput {
    pub file: String,
    pub start_line: u32,
    pub end_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

/// Related context output (spec 263)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedContextOutput {
    pub range: FileRangeOutput,
    pub relationship: String,
    pub reason: String,
}

impl ContextSuggestionOutput {
    pub fn from_context_suggestion(ctx: &crate::priority::context::ContextSuggestion) -> Self {
        Self {
            primary: FileRangeOutput {
                file: ctx.primary.file.to_string_lossy().to_string(),
                start_line: ctx.primary.start_line,
                end_line: ctx.primary.end_line,
                symbol: ctx.primary.symbol.clone(),
            },
            related: ctx
                .related
                .iter()
                .map(|r| RelatedContextOutput {
                    range: FileRangeOutput {
                        file: r.range.file.to_string_lossy().to_string(),
                        start_line: r.range.start_line,
                        end_line: r.range.end_line,
                        symbol: r.range.symbol.clone(),
                    },
                    relationship: r.relationship.to_string(),
                    reason: r.reason.clone(),
                })
                .collect(),
            total_lines: ctx.total_lines,
            completeness_confidence: round_ratio(ctx.completeness_confidence as f64),
        }
    }
}

/// Git history context output for LLM consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHistoryOutput {
    /// Change frequency (changes per month)
    pub change_frequency: f64,
    /// Bug density as ratio of bug fixes to total commits (0.0-1.0)
    pub bug_density: f64,
    /// Age of the code in days
    pub age_days: u32,
    /// Number of unique authors who have modified this code
    pub author_count: usize,
    /// Stability classification based on churn and bug patterns
    pub stability: String,
}

impl GitHistoryOutput {
    /// Extract git history from contextual risk if available
    pub fn from_contextual_risk(risk: &crate::risk::context::ContextualRisk) -> Option<Self> {
        use crate::risk::context::ContextDetails;

        risk.contexts
            .iter()
            .find(|c| c.provider == "git_history")
            .and_then(|git_context| {
                if let ContextDetails::Historical {
                    change_frequency,
                    bug_density,
                    age_days,
                    author_count,
                } = git_context.details
                {
                    let stability = classify_stability(change_frequency, bug_density, age_days);
                    Some(GitHistoryOutput {
                        change_frequency: round_ratio(change_frequency),
                        bug_density: round_ratio(bug_density),
                        age_days,
                        author_count,
                        stability,
                    })
                } else {
                    None
                }
            })
    }
}

/// Classify stability based on change patterns
fn classify_stability(change_frequency: f64, bug_density: f64, age_days: u32) -> String {
    if change_frequency > 5.0 && bug_density > 0.3 {
        "Highly Unstable".to_string()
    } else if change_frequency > 2.0 {
        "Frequently Changed".to_string()
    } else if bug_density > 0.2 {
        "Bug Prone".to_string()
    } else if age_days > 365 {
        "Mature Stable".to_string()
    } else {
        "Relatively Stable".to_string()
    }
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
                ..Default::default()
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
                purity_level: None,
                side_effects: None,
            }),
            dependencies: Dependencies {
                upstream_count: 2,
                downstream_count: 3,
                upstream_callers: vec!["caller1".to_string()],
                downstream_callees: vec!["callee1".to_string()],
                ..Default::default()
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
            context: None,
            git_history: None,
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
                ..Default::default()
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
                ..Default::default()
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
            context: None,
            git_history: None,
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

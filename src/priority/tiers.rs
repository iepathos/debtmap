/// Tier classification for recommendation prioritization
///
/// This module implements a tiered prioritization strategy to surface
/// architectural issues above testing gaps, preventing "walls of similar-scored items".
use crate::priority::{DebtType, UnifiedDebtItem};
use serde::{Deserialize, Serialize};

/// Recommendation tier for strategic remediation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RecommendationTier {
    /// Tier 1: Critical Architecture (God Objects, God Modules, excessive complexity)
    /// Must address before adding new features - high impact on maintainability
    T1CriticalArchitecture,

    /// Tier 2: Complex Untested (Untested code with high complexity or dependencies)
    /// Risk of bugs in critical paths - should be tested before refactoring
    T2ComplexUntested,

    /// Tier 3: Testing Gaps (Untested code with moderate complexity)
    /// Improve coverage to prevent future issues - lower priority than architectural debt
    T3TestingGaps,

    /// Tier 4: Maintenance (Low-complexity issues)
    /// Address opportunistically - minimal impact on system health
    T4Maintenance,
}

impl RecommendationTier {
    /// Get tier weight for score adjustment
    pub fn weight(&self, config: &TierConfig) -> f64 {
        match self {
            RecommendationTier::T1CriticalArchitecture => config.t1_weight,
            RecommendationTier::T2ComplexUntested => config.t2_weight,
            RecommendationTier::T3TestingGaps => config.t3_weight,
            RecommendationTier::T4Maintenance => config.t4_weight,
        }
    }

    /// Get tier label for display
    pub fn label(&self) -> &'static str {
        match self {
            RecommendationTier::T1CriticalArchitecture => "Tier 1: Critical Architecture",
            RecommendationTier::T2ComplexUntested => "Tier 2: Complex Untested",
            RecommendationTier::T3TestingGaps => "Tier 3: Testing Gaps",
            RecommendationTier::T4Maintenance => "Tier 4: Maintenance",
        }
    }

    /// Get short tier label
    pub fn short_label(&self) -> &'static str {
        match self {
            RecommendationTier::T1CriticalArchitecture => "T1",
            RecommendationTier::T2ComplexUntested => "T2",
            RecommendationTier::T3TestingGaps => "T3",
            RecommendationTier::T4Maintenance => "T4",
        }
    }
}

/// Configuration for tier thresholds and weights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    /// Tier 2: Complexity threshold for complex untested code
    pub t2_complexity_threshold: u32,

    /// Tier 2: Dependency threshold for complex untested code
    pub t2_dependency_threshold: usize,

    /// Tier 3: Complexity threshold for testing gaps
    pub t3_complexity_threshold: u32,

    /// Show Tier 4 items in main report
    pub show_t4_in_main_report: bool,

    /// Tier 1 weight (boost architectural issues)
    pub t1_weight: f64,

    /// Tier 2 weight
    pub t2_weight: f64,

    /// Tier 3 weight
    pub t3_weight: f64,

    /// Tier 4 weight
    pub t4_weight: f64,
}

impl Default for TierConfig {
    fn default() -> Self {
        Self {
            t2_complexity_threshold: 15,
            t2_dependency_threshold: 10,
            t3_complexity_threshold: 10,
            show_t4_in_main_report: false,
            t1_weight: 1.5,
            t2_weight: 1.0,
            t3_weight: 0.7,
            t4_weight: 0.3,
        }
    }
}

impl TierConfig {
    /// Create strict tier configuration
    pub fn strict() -> Self {
        Self {
            t2_complexity_threshold: 10,
            t2_dependency_threshold: 7,
            t3_complexity_threshold: 7,
            ..Default::default()
        }
    }

    /// Create balanced tier configuration (default)
    pub fn balanced() -> Self {
        Self::default()
    }

    /// Create lenient tier configuration
    pub fn lenient() -> Self {
        Self {
            t2_complexity_threshold: 20,
            t2_dependency_threshold: 15,
            t3_complexity_threshold: 15,
            ..Default::default()
        }
    }
}

/// Classify a debt item into a recommendation tier
/// Uses sophisticated scoring metrics (weighted complexity, entropy, cognitive load)
/// rather than raw cyclomatic complexity
pub fn classify_tier(item: &UnifiedDebtItem, config: &TierConfig) -> RecommendationTier {
    // Tier 1: Architectural issues OR very high scores
    // Use final_score (includes exponential scaling, risk boosts, all sophisticated analysis)
    if is_architectural_issue(item, &item.debt_type, config) {
        return RecommendationTier::T1CriticalArchitecture;
    }

    // Tier 2: Complex untested code
    if is_complex_untested(item, config) {
        return RecommendationTier::T2ComplexUntested;
    }

    // Tier 2: Moderate complexity hotspots using sophisticated scoring
    // This now considers weighted complexity, cognitive load, nesting, entropy dampening
    if is_moderate_complexity_hotspot(item, config) {
        return RecommendationTier::T2ComplexUntested;
    }

    // Tier 3: Moderate testing gaps
    if is_moderate_untested(item, config) {
        return RecommendationTier::T3TestingGaps;
    }

    // Tier 4: Everything else
    RecommendationTier::T4Maintenance
}

/// Check if debt type is an architectural issue
/// Uses sophisticated scoring: final_score, complexity_factor, cognitive, nesting
fn is_architectural_issue(
    item: &UnifiedDebtItem,
    debt_type: &DebtType,
    _config: &TierConfig,
) -> bool {
    // God objects and modules are always T1
    if matches!(
        debt_type,
        DebtType::GodObject { .. } | DebtType::GodModule { .. }
    ) {
        return true;
    }

    // Critical patterns are always T1
    if matches!(
        debt_type,
        DebtType::AsyncMisuse { .. } | DebtType::ErrorSwallowing { .. }
    ) {
        return true;
    }

    // For complexity hotspots, use SOPHISTICATED METRICS not raw cyclomatic
    if let DebtType::ComplexityHotspot {
        adjusted_cyclomatic,
        cyclomatic,
        cognitive,
    } = debt_type
    {
        // Use entropy-dampened complexity if available
        let effective_cyclomatic = adjusted_cyclomatic.unwrap_or(*cyclomatic);

        // T1 if: extreme cyclomatic (raw) OR high weighted score OR extreme cognitive OR extreme nesting
        // The final_score already includes weighted complexity, so check it first
        if item.unified_score.final_score > 10.0 {
            return true; // High final score after exponential scaling = critical
        }

        // Also check individual metrics for extreme values
        if effective_cyclomatic > 50 {
            return true; // Extremely high cyclomatic even after dampening
        }

        if *cognitive >= 20 {
            return true; // Extreme cognitive load
        }

        if item.nesting_depth >= 5 {
            return true; // Very deep nesting
        }

        // Check complexity_factor (includes weighted scoring: 30% cyclo + 70% cognitive)
        if item.unified_score.complexity_factor > 5.0 {
            return true; // High weighted complexity score
        }
    }

    false
}

/// Check if item is complex untested code
fn is_complex_untested(item: &UnifiedDebtItem, config: &TierConfig) -> bool {
    // Must be a testing gap
    let is_testing_gap = matches!(item.debt_type, DebtType::TestingGap { .. });
    if !is_testing_gap {
        return false;
    }

    // High complexity threshold
    let high_complexity = item.cyclomatic_complexity >= config.t2_complexity_threshold;

    // High dependency count
    let total_deps = item.upstream_dependencies + item.downstream_dependencies;
    let high_dependencies = total_deps >= config.t2_dependency_threshold;

    // Entry point function
    let is_critical_function = matches!(
        item.function_role,
        crate::priority::FunctionRole::EntryPoint
    );

    high_complexity || high_dependencies || is_critical_function
}

/// Check if item is moderate untested code
fn is_moderate_untested(item: &UnifiedDebtItem, config: &TierConfig) -> bool {
    // Must be a testing gap
    let is_testing_gap = matches!(item.debt_type, DebtType::TestingGap { .. });
    if !is_testing_gap {
        return false;
    }

    // Moderate complexity
    item.cyclomatic_complexity >= config.t3_complexity_threshold
}

/// Check if item is a moderate complexity hotspot (T2, not extreme enough for T1)
/// Uses sophisticated metrics: weighted complexity, cognitive load, nesting, entropy dampening
fn is_moderate_complexity_hotspot(item: &UnifiedDebtItem, _config: &TierConfig) -> bool {
    // Only apply to complexity hotspots
    if !matches!(&item.debt_type, DebtType::ComplexityHotspot { .. }) {
        return false;
    }

    // T2 if the item has meaningful complexity that warrants attention
    // Use multiple sophisticated signals, not just raw cyclomatic

    // Signal 1: complexity_factor (weighted: 30% cyclo + 70% cognitive, scaled 0-10)
    // Threshold: >= 2.0 indicates meaningful complexity
    let has_meaningful_weighted_complexity = item.unified_score.complexity_factor >= 2.0;

    // Signal 2: High cognitive complexity (mental load)
    // Threshold: >= 12 indicates moderate to high cognitive load
    let has_high_cognitive = item.cognitive_complexity >= 12;

    // Signal 3: Deep nesting (indicates nested conditionals / loops)
    // Threshold: >= 3 indicates meaningful nesting
    let has_deep_nesting = item.nesting_depth >= 3;

    // Signal 4: Adjusted cyclomatic after entropy dampening
    // This respects the entropy analysis that identified repetitive patterns
    if let DebtType::ComplexityHotspot {
        adjusted_cyclomatic,
        cyclomatic,
        ..
    } = &item.debt_type
    {
        let effective_cyclomatic = adjusted_cyclomatic.unwrap_or(*cyclomatic);

        // For dampened complexity, use lower threshold since dampening already filtered out noise
        let has_meaningful_dampened_complexity = (8..=50).contains(&effective_cyclomatic);

        // T2 if ANY of the sophisticated signals indicate meaningful complexity
        has_meaningful_weighted_complexity
            || has_high_cognitive
            || has_deep_nesting
            || has_meaningful_dampened_complexity
    } else {
        // Fallback: use weighted and cognitive signals
        has_meaningful_weighted_complexity || has_high_cognitive || has_deep_nesting
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        ActionableRecommendation, FunctionRole, ImpactMetrics, Location, UnifiedScore,
    };

    fn create_test_item(debt_type: DebtType, complexity: u32, deps: usize) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: "test.rs".into(),
                function: "test_fn".into(),
                line: 1,
            },
            debt_type,
            unified_score: UnifiedScore {
                complexity_factor: 0.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                final_score: 0.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Test".into(),
                rationale: "Test".into(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                risk_reduction: 0.0,
                complexity_reduction: 0.0,
                coverage_improvement: 0.0,
                lines_reduction: 0,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: deps,
            downstream_dependencies: deps,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: complexity,
            cognitive_complexity: complexity,
            entropy_details: None,
            is_pure: Some(false),
            purity_confidence: Some(0.0),
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None, // spec 203
        }
    }

    #[test]
    fn test_tier_classification_god_object() {
        let item = create_test_item(
            DebtType::GodObject {
                methods: 100,
                fields: 50,
                responsibilities: 5,
                god_object_score: 95.0,
            },
            10,
            5,
        );
        let config = TierConfig::default();
        assert_eq!(
            classify_tier(&item, &config),
            RecommendationTier::T1CriticalArchitecture
        );
    }

    #[test]
    fn test_tier_classification_complex_untested() {
        let item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 20,
                cognitive: 25,
            },
            20,
            5,
        );
        let config = TierConfig::default();
        assert_eq!(
            classify_tier(&item, &config),
            RecommendationTier::T2ComplexUntested
        );
    }

    #[test]
    fn test_tier_classification_simple_untested_filtered() {
        let item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 5,
                cognitive: 6,
            },
            5,
            2,
        );
        let config = TierConfig::default();
        assert_eq!(
            classify_tier(&item, &config),
            RecommendationTier::T4Maintenance
        );
    }

    #[test]
    fn test_tier_classification_moderate_untested() {
        let item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 12,
                cognitive: 14,
            },
            12,
            5,
        );
        let config = TierConfig::default();
        // With 5 upstream + 5 downstream deps = 10 total, meets t2_dependency_threshold (10)
        // Therefore should classify as T2ComplexUntested, not T3TestingGaps
        assert_eq!(
            classify_tier(&item, &config),
            RecommendationTier::T2ComplexUntested
        );
    }
}

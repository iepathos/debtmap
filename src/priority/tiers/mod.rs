/// Tier classification for recommendation prioritization
///
/// This module implements a tiered prioritization strategy using pure functional
/// composition of predicate functions, following the "Pure Core, Imperative Shell"
/// principle from the Stillwater philosophy.
///
/// ## Architecture
///
/// - **predicates.rs**: Pure predicate functions (2-5 lines each, single responsibility)
/// - **pure.rs**: Pure classification logic (composes predicates)
/// - **mod.rs**: Public API and configuration
///
/// ## Design Principles
///
/// - Small, pure functions with no side effects
/// - Clear predicate composition using boolean operators
/// - All functions < 10 lines, cyclomatic complexity < 5
/// - 100% testable without mocks
use serde::{Deserialize, Serialize};

pub mod predicates;
pub mod pure;

// Re-export main classification function for backward compatibility
pub use pure::classify_tier;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::score_types::Score0To100;
    use crate::priority::{
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, UnifiedDebtItem,
        UnifiedScore,
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
                final_score: Score0To100::new(0.0),
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                // Spec 260: Score transparency fields
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
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
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
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
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }
    }

    #[test]
    fn test_tier_classification_god_object() {
        let item = create_test_item(
            DebtType::GodObject {
                methods: 100,
                fields: Some(50),
                responsibilities: 5,
                god_object_score: Score0To100::new(95.0),
                lines: 500,
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

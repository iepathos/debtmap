use serde::{Deserialize, Serialize};

// Re-export types from sub-modules
use super::classification::ClassificationConfig;
use super::detection::{ErrorHandlingConfig, OrchestratorDetectionConfig};
use super::display::{DisplayConfig, GodObjectConfig};
use super::languages::{EntropyConfig, LanguagesConfig};
use super::scoring::{
    ComplexityWeightsConfig, NormalizationConfig, RebalancedScoringConfig, RoleCoverageWeights,
    RoleMultiplierConfig, RoleMultipliers, ScoringWeights,
};
use super::thresholds::ThresholdsConfig;
use crate::complexity::pure_mapping_patterns::MappingPatternConfig;

/// Root configuration structure for debtmap
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebtmapConfig {
    /// Scoring weights configuration
    #[serde(default)]
    pub scoring: Option<ScoringWeights>,

    /// Display configuration for output formatting
    #[serde(default)]
    pub display: Option<DisplayConfig>,

    /// External API detection configuration
    #[serde(default)]
    pub external_api: Option<crate::priority::external_api_detector::ExternalApiConfig>,

    /// God object detection configuration
    #[serde(default)]
    pub god_object_detection: Option<GodObjectConfig>,

    /// Thresholds configuration
    #[serde(default)]
    pub thresholds: Option<ThresholdsConfig>,

    /// Language configuration
    #[serde(default)]
    pub languages: Option<LanguagesConfig>,

    /// Ignore patterns
    #[serde(default)]
    pub ignore: Option<IgnoreConfig>,

    /// Output configuration
    #[serde(default)]
    pub output: Option<OutputConfig>,

    /// Context-aware detection configuration
    #[serde(default)]
    pub context: Option<crate::config::classification::ContextConfig>,

    /// Entropy-based complexity scoring configuration
    #[serde(default)]
    pub entropy: Option<EntropyConfig>,

    /// Role multipliers for semantic classification
    #[serde(default)]
    pub role_multipliers: Option<RoleMultipliers>,

    /// Complexity thresholds for enhanced detection
    #[serde(default)]
    pub complexity_thresholds: Option<crate::complexity::threshold_manager::ComplexityThresholds>,

    /// Error handling detection configuration
    #[serde(default)]
    pub error_handling: Option<ErrorHandlingConfig>,

    /// Score normalization configuration
    #[serde(default)]
    pub normalization: Option<NormalizationConfig>,

    /// Lines of code counting configuration
    #[serde(default)]
    pub loc: Option<crate::metrics::LocCountingConfig>,

    /// Tier configuration for prioritization
    #[serde(default)]
    pub tiers: Option<crate::priority::TierConfig>,

    /// Role-based coverage weight multipliers
    #[serde(default)]
    pub role_coverage_weights: Option<RoleCoverageWeights>,

    /// Role multiplier clamping configuration (spec 119)
    #[serde(default)]
    pub role_multiplier_config: Option<RoleMultiplierConfig>,

    /// Orchestrator detection configuration
    #[serde(default)]
    pub orchestrator_detection: Option<OrchestratorDetectionConfig>,

    /// Orchestration score adjustment configuration (spec 110)
    #[serde(default)]
    pub orchestration_adjustment:
        Option<crate::priority::scoring::orchestration_adjustment::OrchestrationAdjustmentConfig>,

    /// Constructor detection configuration (spec 117)
    #[serde(default, rename = "classification")]
    pub classification: Option<ClassificationConfig>,

    /// Pure mapping pattern detection configuration (spec 118)
    #[serde(default)]
    pub mapping_patterns: Option<MappingPatternConfig>,

    /// Role-based coverage expectations (spec 119)
    #[serde(default)]
    pub coverage_expectations: Option<crate::priority::scoring::CoverageExpectations>,

    /// Complexity weights configuration (spec 121)
    #[serde(default)]
    pub complexity_weights: Option<ComplexityWeightsConfig>,

    /// AST-based functional pattern analysis configuration (spec 111)
    #[serde(default)]
    pub functional_analysis: Option<crate::analysis::FunctionalAnalysisConfig>,

    /// Boilerplate detection configuration (spec 131)
    #[serde(default)]
    pub boilerplate_detection:
        Option<crate::organization::boilerplate_detector::BoilerplateDetectionConfig>,

    /// Rebalanced scoring configuration (spec 136)
    #[serde(default, rename = "scoring_rebalanced")]
    pub scoring_rebalanced: Option<RebalancedScoringConfig>,
}

impl DebtmapConfig {
    /// Get ignore patterns from configuration
    ///
    /// Returns a vector of glob patterns that should be excluded from analysis.
    /// If no configuration is found or no patterns are specified, returns an empty vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::config::DebtmapConfig;
    /// let config = DebtmapConfig::default();
    /// let patterns = config.get_ignore_patterns();
    /// // patterns might contain ["tests/**/*", "*.test.rs"]
    /// ```
    pub fn get_ignore_patterns(&self) -> Vec<String> {
        self.ignore
            .as_ref()
            .map(|ig| ig.patterns.clone())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreConfig {
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub default_format: Option<String>,
    /// Enable colored output (default: auto-detect based on TTY)
    #[serde(default)]
    pub use_color: Option<bool>,
}

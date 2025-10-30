//! Scoring configuration for technical debt prioritization
//!
//! This module contains all scoring-related configuration types including:
//! - Weight configurations for different factors (coverage, complexity, etc.)
//! - Role multipliers for semantic classification
//! - Complexity weight configurations
//! - Role coverage weights
//! - Role multiplier clamping
//! - Rebalanced scoring presets

use serde::{Deserialize, Serialize};

/// Scoring weights configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Weight for coverage factor (0.0-1.0)
    #[serde(default = "default_coverage_weight")]
    pub coverage: f64,

    /// Weight for complexity factor (0.0-1.0)
    #[serde(default = "default_complexity_weight")]
    pub complexity: f64,

    /// Weight for semantic factor (0.0-1.0)
    #[serde(default = "default_semantic_weight")]
    pub semantic: f64,

    /// Weight for dependency criticality factor (0.0-1.0)
    #[serde(default = "default_dependency_weight")]
    pub dependency: f64,

    /// Weight for security issues factor (0.0-1.0)
    #[serde(default = "default_security_weight")]
    pub security: f64,

    /// Weight for code organization issues factor (0.0-1.0)
    #[serde(default = "default_organization_weight")]
    pub organization: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            coverage: default_coverage_weight(),
            complexity: default_complexity_weight(),
            semantic: default_semantic_weight(),
            dependency: default_dependency_weight(),
            security: default_security_weight(),
            organization: default_organization_weight(),
        }
    }
}

impl ScoringWeights {
    // Pure function: Check if a weight is in valid range
    pub fn is_valid_weight(weight: f64) -> bool {
        (0.0..=1.0).contains(&weight)
    }

    // Pure function: Validate a single weight with name
    pub fn validate_weight(weight: f64, name: &str) -> Result<(), String> {
        if Self::is_valid_weight(weight) {
            Ok(())
        } else {
            Err(format!("{} weight must be between 0.0 and 1.0", name))
        }
    }

    // Pure function: Validate active weights sum to 1.0
    pub fn validate_active_weights_sum(
        coverage: f64,
        complexity: f64,
        dependency: f64,
    ) -> Result<(), String> {
        let sum = coverage + complexity + dependency;
        if (sum - 1.0).abs() > 0.001 {
            Err(format!(
                "Active scoring weights (coverage, complexity, dependency) must sum to 1.0, but sum to {:.3}",
                sum
            ))
        } else {
            Ok(())
        }
    }

    // Pure function: Collect all weight validations
    pub fn collect_weight_validations(&self) -> Vec<Result<(), String>> {
        vec![
            Self::validate_weight(self.coverage, "Coverage"),
            Self::validate_weight(self.complexity, "Complexity"),
            Self::validate_weight(self.semantic, "Semantic"),
            Self::validate_weight(self.dependency, "Dependency"),
            Self::validate_weight(self.security, "Security"),
            Self::validate_weight(self.organization, "Organization"),
        ]
    }

    /// Validate that weights sum to 1.0 (with small tolerance for floating point)
    pub fn validate(&self) -> Result<(), String> {
        // Validate active weights sum
        Self::validate_active_weights_sum(self.coverage, self.complexity, self.dependency)?;

        // Validate all individual weights
        for validation in self.collect_weight_validations() {
            validation?;
        }

        Ok(())
    }

    /// Normalize weights to ensure they sum to 1.0
    pub fn normalize(&mut self) {
        // Only normalize the weights we actually use
        let sum = self.coverage + self.complexity + self.dependency;
        if sum > 0.0 && (sum - 1.0).abs() > 0.001 {
            self.coverage /= sum;
            self.complexity /= sum;
            self.dependency /= sum;
            // Set unused weights to 0
            self.semantic = 0.0;
            self.security = 0.0;
            self.organization = 0.0;
        }
    }
}

// Default weights for weighted sum model - prioritizing coverage gaps
pub fn default_coverage_weight() -> f64 {
    0.50 // 50% weight to prioritize untested code
}
pub fn default_complexity_weight() -> f64 {
    0.35 // 35% weight for complexity within coverage tiers
}
pub fn default_semantic_weight() -> f64 {
    0.00 // Not used in weighted sum model
}
pub fn default_dependency_weight() -> f64 {
    0.15 // 15% weight for impact radius
}
pub fn default_security_weight() -> f64 {
    0.00 // Not used in weighted sum model
}
pub fn default_organization_weight() -> f64 {
    0.00 // Not used in weighted sum model
}

/// Role multipliers configuration for semantic classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMultipliers {
    /// Multiplier for PureLogic functions (default: 1.5)
    #[serde(default = "default_pure_logic_multiplier")]
    pub pure_logic: f64,

    /// Multiplier for Orchestrator functions (default: 0.6)
    #[serde(default = "default_orchestrator_multiplier")]
    pub orchestrator: f64,

    /// Multiplier for IOWrapper functions (default: 0.5)
    #[serde(default = "default_io_wrapper_multiplier")]
    pub io_wrapper: f64,

    /// Multiplier for EntryPoint functions (default: 0.8)
    #[serde(default = "default_entry_point_multiplier")]
    pub entry_point: f64,

    /// Multiplier for PatternMatch functions (default: 0.4)
    #[serde(default = "default_pattern_match_multiplier")]
    pub pattern_match: f64,

    /// Multiplier for Debug functions (default: 0.3)
    #[serde(default = "default_debug_multiplier")]
    pub debug: f64,

    /// Multiplier for Unknown functions (default: 1.0)
    #[serde(default = "default_unknown_multiplier")]
    pub unknown: f64,
}

impl Default for RoleMultipliers {
    fn default() -> Self {
        Self {
            pure_logic: default_pure_logic_multiplier(),
            orchestrator: default_orchestrator_multiplier(),
            io_wrapper: default_io_wrapper_multiplier(),
            entry_point: default_entry_point_multiplier(),
            pattern_match: default_pattern_match_multiplier(),
            debug: default_debug_multiplier(),
            unknown: default_unknown_multiplier(),
        }
    }
}

pub fn default_pure_logic_multiplier() -> f64 {
    1.2 // Prioritized but not extreme (was 1.5)
}

pub fn default_orchestrator_multiplier() -> f64 {
    0.8 // Reduced but not severely (was 0.6)
}

pub fn default_io_wrapper_multiplier() -> f64 {
    0.7 // Minor reduction (was 0.5)
}

pub fn default_entry_point_multiplier() -> f64 {
    0.9 // Slight reduction (was 0.8)
}

pub fn default_pattern_match_multiplier() -> f64 {
    0.6 // Moderate reduction (was 0.4)
}

pub fn default_debug_multiplier() -> f64 {
    0.3 // Debug/diagnostic functions have lowest test priority (spec 119)
}

pub fn default_unknown_multiplier() -> f64 {
    1.0 // No adjustment for unknown functions
}

/// Complexity weights configuration (spec 121)
/// Controls how cyclomatic and cognitive complexity are combined in scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityWeightsConfig {
    /// Weight for cyclomatic complexity (default: 0.3)
    #[serde(default = "default_cyclomatic_weight")]
    pub cyclomatic: f64,

    /// Weight for cognitive complexity (default: 0.7)
    #[serde(default = "default_cognitive_weight")]
    pub cognitive: f64,

    /// Maximum cyclomatic complexity for normalization (default: 50)
    #[serde(default = "default_max_cyclomatic")]
    pub max_cyclomatic: f64,

    /// Maximum cognitive complexity for normalization (default: 100)
    #[serde(default = "default_max_cognitive")]
    pub max_cognitive: f64,
}

impl Default for ComplexityWeightsConfig {
    fn default() -> Self {
        Self {
            cyclomatic: default_cyclomatic_weight(),
            cognitive: default_cognitive_weight(),
            max_cyclomatic: default_max_cyclomatic(),
            max_cognitive: default_max_cognitive(),
        }
    }
}

pub fn default_cyclomatic_weight() -> f64 {
    0.3 // 30% weight - cyclomatic as proxy for test cases
}

pub fn default_cognitive_weight() -> f64 {
    0.7 // 70% weight - cognitive correlates better with bugs
}

pub fn default_max_cyclomatic() -> f64 {
    50.0 // Reasonable maximum for cyclomatic complexity
}

pub fn default_max_cognitive() -> f64 {
    100.0 // Cognitive complexity can go higher
}

/// Role-based coverage weight multipliers for scoring adjustment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleCoverageWeights {
    /// Coverage weight multiplier for EntryPoint functions (default: 0.6)
    #[serde(default = "default_entry_point_coverage_weight")]
    pub entry_point: f64,

    /// Coverage weight multiplier for Orchestrator functions (default: 0.8)
    #[serde(default = "default_orchestrator_coverage_weight")]
    pub orchestrator: f64,

    /// Coverage weight multiplier for PureLogic functions (default: 1.0)
    #[serde(default = "default_pure_logic_coverage_weight")]
    pub pure_logic: f64,

    /// Coverage weight multiplier for IOWrapper functions (default: 1.0)
    #[serde(default = "default_io_wrapper_coverage_weight")]
    pub io_wrapper: f64,

    /// Coverage weight multiplier for PatternMatch functions (default: 1.0)
    #[serde(default = "default_pattern_match_coverage_weight")]
    pub pattern_match: f64,

    /// Coverage weight multiplier for Debug functions (default: 0.3)
    #[serde(default = "default_debug_coverage_weight")]
    pub debug: f64,

    /// Coverage weight multiplier for Unknown functions (default: 1.0)
    #[serde(default = "default_unknown_coverage_weight")]
    pub unknown: f64,
}

impl Default for RoleCoverageWeights {
    fn default() -> Self {
        Self {
            entry_point: default_entry_point_coverage_weight(),
            orchestrator: default_orchestrator_coverage_weight(),
            pure_logic: default_pure_logic_coverage_weight(),
            io_wrapper: default_io_wrapper_coverage_weight(),
            pattern_match: default_pattern_match_coverage_weight(),
            debug: default_debug_coverage_weight(),
            unknown: default_unknown_coverage_weight(),
        }
    }
}

pub fn default_entry_point_coverage_weight() -> f64 {
    0.6 // Entry points are often integration tested, reduce unit coverage penalty
}

pub fn default_orchestrator_coverage_weight() -> f64 {
    0.8 // Orchestrators are often tested via higher-level tests
}

pub fn default_pure_logic_coverage_weight() -> f64 {
    1.0 // Pure logic should have unit tests, no reduction
}

pub fn default_io_wrapper_coverage_weight() -> f64 {
    0.5 // I/O wrappers are integration tested, reduce unit coverage penalty (spec 119)
}

pub fn default_pattern_match_coverage_weight() -> f64 {
    1.0 // Pattern matching should have unit tests, no reduction
}

pub fn default_debug_coverage_weight() -> f64 {
    0.3 // Debug/diagnostic functions have lowest coverage expectations (spec 119)
}

pub fn default_unknown_coverage_weight() -> f64 {
    1.0 // Unknown functions get normal coverage expectations
}

/// Role multiplier clamping configuration (spec 119)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMultiplierConfig {
    /// Minimum clamp value for role multipliers (default: 0.3)
    #[serde(default = "default_role_clamp_min")]
    pub clamp_min: f64,

    /// Maximum clamp value for role multipliers (default: 1.8)
    #[serde(default = "default_role_clamp_max")]
    pub clamp_max: f64,

    /// Whether to enable clamping (default: true)
    #[serde(default = "default_enable_role_clamping")]
    pub enable_clamping: bool,
}

impl Default for RoleMultiplierConfig {
    fn default() -> Self {
        Self {
            clamp_min: default_role_clamp_min(),
            clamp_max: default_role_clamp_max(),
            enable_clamping: default_enable_role_clamping(),
        }
    }
}

pub fn default_role_clamp_min() -> f64 {
    0.3 // Allow IOWrapper to get 50% reduction (0.5 multiplier)
}

pub fn default_role_clamp_max() -> f64 {
    1.8 // Allow EntryPoint to get 50% increase (1.5 multiplier)
}

pub fn default_enable_role_clamping() -> bool {
    true // Enable by default
}

/// Rebalanced scoring configuration (spec 136)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancedScoringConfig {
    /// Preset name (balanced, quality-focused, size-focused, test-coverage)
    #[serde(default)]
    pub preset: Option<String>,

    /// Custom complexity weight (overrides preset if specified)
    #[serde(default)]
    pub complexity_weight: Option<f64>,

    /// Custom coverage weight (overrides preset if specified)
    #[serde(default)]
    pub coverage_weight: Option<f64>,

    /// Custom structural weight (overrides preset if specified)
    #[serde(default)]
    pub structural_weight: Option<f64>,

    /// Custom size weight (overrides preset if specified)
    #[serde(default)]
    pub size_weight: Option<f64>,

    /// Custom smell weight (overrides preset if specified)
    #[serde(default)]
    pub smell_weight: Option<f64>,
}

impl RebalancedScoringConfig {
    /// Convert to ScoreWeights
    pub fn to_weights(&self) -> crate::priority::scoring::ScoreWeights {
        use crate::priority::scoring::ScoreWeights;

        // Start with preset if specified, otherwise default
        let mut weights = self
            .preset
            .as_ref()
            .and_then(|p| ScoreWeights::from_preset(p))
            .unwrap_or_default();

        // Override with custom values if specified
        if let Some(w) = self.complexity_weight {
            weights.complexity_weight = w;
        }
        if let Some(w) = self.coverage_weight {
            weights.coverage_weight = w;
        }
        if let Some(w) = self.structural_weight {
            weights.structural_weight = w;
        }
        if let Some(w) = self.size_weight {
            weights.size_weight = w;
        }
        if let Some(w) = self.smell_weight {
            weights.smell_weight = w;
        }

        weights
    }
}

/// Score normalization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationConfig {
    /// Threshold for linear scaling (default: 10.0)
    #[serde(default = "default_linear_threshold")]
    pub linear_threshold: f64,

    /// Threshold for logarithmic scaling (default: 100.0)
    #[serde(default = "default_logarithmic_threshold")]
    pub logarithmic_threshold: f64,

    /// Multiplier for square root scaling (default: 3.33)
    #[serde(default = "default_sqrt_multiplier")]
    pub sqrt_multiplier: f64,

    /// Multiplier for logarithmic scaling (default: 10.0)
    #[serde(default = "default_log_multiplier")]
    pub log_multiplier: f64,

    /// Whether to show raw scores alongside normalized scores
    #[serde(default = "default_show_raw_scores")]
    pub show_raw_scores: bool,
}

impl Default for NormalizationConfig {
    fn default() -> Self {
        Self {
            linear_threshold: default_linear_threshold(),
            logarithmic_threshold: default_logarithmic_threshold(),
            sqrt_multiplier: default_sqrt_multiplier(),
            log_multiplier: default_log_multiplier(),
            show_raw_scores: default_show_raw_scores(),
        }
    }
}

pub fn default_linear_threshold() -> f64 {
    10.0
}

pub fn default_logarithmic_threshold() -> f64 {
    100.0
}

pub fn default_sqrt_multiplier() -> f64 {
    3.33
}

pub fn default_log_multiplier() -> f64 {
    10.0
}

pub fn default_show_raw_scores() -> bool {
    true
}

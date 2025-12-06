//! Configuration presets for common analysis scenarios.
//!
//! This module provides pre-configured settings for different code quality standards:
//! - **Strict**: High code quality standards, low tolerance for complexity
//! - **Balanced**: Reasonable defaults for most projects
//! - **Permissive**: Lenient settings for legacy or complex domains
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::config::presets::PresetLevel;
//!
//! let config = PresetLevel::Strict.to_config();
//! ```

use serde::{Deserialize, Serialize};

use super::core::DebtmapConfig;
use super::scoring::ScoringWeights;
use super::thresholds::ThresholdsConfig;

/// Preset configuration levels for analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetLevel {
    /// Strict thresholds for high code quality standards
    Strict,
    /// Balanced thresholds for typical projects (recommended)
    Balanced,
    /// Permissive thresholds for legacy or complex domains
    Permissive,
}

impl PresetLevel {
    /// Convert preset level to full configuration.
    ///
    /// This returns a complete `DebtmapConfig` with all sections configured
    /// according to the preset level.
    pub fn to_config(self) -> DebtmapConfig {
        match self {
            PresetLevel::Strict => strict_preset(),
            PresetLevel::Balanced => balanced_preset(),
            PresetLevel::Permissive => permissive_preset(),
        }
    }

    /// Get scoring weights for this preset.
    pub fn scoring_weights(self) -> ScoringWeights {
        match self {
            PresetLevel::Strict => ScoringWeights {
                coverage: 0.55, // Higher coverage weight for strict mode
                complexity: 0.30,
                semantic: 0.00,
                dependency: 0.15,
                security: 0.00,
                organization: 0.00,
            },
            PresetLevel::Balanced => ScoringWeights::default(),
            PresetLevel::Permissive => ScoringWeights {
                coverage: 0.40,   // Lower coverage weight for permissive mode
                complexity: 0.40, // Higher complexity weight
                semantic: 0.00,
                dependency: 0.20,
                security: 0.00,
                organization: 0.00,
            },
        }
    }

    /// Get threshold configuration for this preset.
    pub fn thresholds(self) -> ThresholdsConfig {
        match self {
            PresetLevel::Strict => ThresholdsConfig {
                complexity: Some(20),
                duplication: Some(5),
                max_file_length: Some(500),
                max_function_length: Some(30),
                minimum_cyclomatic_complexity: Some(3),
                minimum_cognitive_complexity: Some(5),
                min_score_threshold: Some(2.0),
                ..Default::default()
            },
            PresetLevel::Balanced => ThresholdsConfig {
                complexity: Some(50),
                duplication: Some(10),
                max_file_length: Some(1000),
                max_function_length: Some(50),
                minimum_cyclomatic_complexity: Some(5),
                minimum_cognitive_complexity: Some(10),
                min_score_threshold: Some(3.0),
                ..Default::default()
            },
            PresetLevel::Permissive => ThresholdsConfig {
                complexity: Some(100),
                duplication: Some(20),
                max_file_length: Some(2000),
                max_function_length: Some(100),
                minimum_cyclomatic_complexity: Some(10),
                minimum_cognitive_complexity: Some(15),
                min_score_threshold: Some(5.0),
                ..Default::default()
            },
        }
    }

    /// Parse preset from string name (returns Option instead of Result).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "strict" => Some(PresetLevel::Strict),
            "balanced" => Some(PresetLevel::Balanced),
            "permissive" | "lenient" => Some(PresetLevel::Permissive),
            _ => None,
        }
    }

    /// Get the string name of this preset.
    pub fn as_str(self) -> &'static str {
        match self {
            PresetLevel::Strict => "strict",
            PresetLevel::Balanced => "balanced",
            PresetLevel::Permissive => "permissive",
        }
    }
}

impl std::fmt::Display for PresetLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for PresetLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PresetLevel::parse(s).ok_or_else(|| {
            format!(
                "Invalid preset level: '{}'. Valid options: strict, balanced, permissive",
                s
            )
        })
    }
}

/// Create strict preset configuration.
fn strict_preset() -> DebtmapConfig {
    DebtmapConfig {
        scoring: Some(PresetLevel::Strict.scoring_weights()),
        thresholds: Some(PresetLevel::Strict.thresholds()),
        ..Default::default()
    }
}

/// Create balanced preset configuration (default).
fn balanced_preset() -> DebtmapConfig {
    DebtmapConfig {
        scoring: Some(PresetLevel::Balanced.scoring_weights()),
        thresholds: Some(PresetLevel::Balanced.thresholds()),
        ..Default::default()
    }
}

/// Create permissive preset configuration.
fn permissive_preset() -> DebtmapConfig {
    DebtmapConfig {
        scoring: Some(PresetLevel::Permissive.scoring_weights()),
        thresholds: Some(PresetLevel::Permissive.thresholds()),
        ..Default::default()
    }
}

/// Merge a preset configuration with an existing config.
///
/// Values from `config` override values from the preset.
/// This allows users to start with a preset and customize specific values.
pub fn merge_preset_with_config(preset: PresetLevel, config: DebtmapConfig) -> DebtmapConfig {
    let preset_config = preset.to_config();

    DebtmapConfig {
        scoring: config.scoring.or(preset_config.scoring),
        display: config.display.or(preset_config.display),
        external_api: config.external_api.or(preset_config.external_api),
        god_object_detection: config
            .god_object_detection
            .or(preset_config.god_object_detection),
        thresholds: config.thresholds.or(preset_config.thresholds),
        languages: config.languages.or(preset_config.languages),
        ignore: config.ignore.or(preset_config.ignore),
        output: config.output.or(preset_config.output),
        context: config.context.or(preset_config.context),
        entropy: config.entropy.or(preset_config.entropy),
        role_multipliers: config.role_multipliers.or(preset_config.role_multipliers),
        complexity_thresholds: config
            .complexity_thresholds
            .or(preset_config.complexity_thresholds),
        error_handling: config.error_handling.or(preset_config.error_handling),
        normalization: config.normalization.or(preset_config.normalization),
        loc: config.loc.or(preset_config.loc),
        tiers: config.tiers.or(preset_config.tiers),
        role_coverage_weights: config
            .role_coverage_weights
            .or(preset_config.role_coverage_weights),
        role_multiplier_config: config
            .role_multiplier_config
            .or(preset_config.role_multiplier_config),
        orchestrator_detection: config
            .orchestrator_detection
            .or(preset_config.orchestrator_detection),
        orchestration_adjustment: config
            .orchestration_adjustment
            .or(preset_config.orchestration_adjustment),
        classification: config.classification.or(preset_config.classification),
        mapping_patterns: config.mapping_patterns.or(preset_config.mapping_patterns),
        coverage_expectations: config
            .coverage_expectations
            .or(preset_config.coverage_expectations),
        complexity_weights: config
            .complexity_weights
            .or(preset_config.complexity_weights),
        functional_analysis: config
            .functional_analysis
            .or(preset_config.functional_analysis),
        boilerplate_detection: config
            .boilerplate_detection
            .or(preset_config.boilerplate_detection),
        scoring_rebalanced: config
            .scoring_rebalanced
            .or(preset_config.scoring_rebalanced),
        context_multipliers: config
            .context_multipliers
            .or(preset_config.context_multipliers),
        batch_analysis: config.batch_analysis.or(preset_config.batch_analysis),
        retry: config.retry.or(preset_config.retry),
        analysis: config.analysis.or(preset_config.analysis),
        state_detection: config.state_detection.or(preset_config.state_detection),
        data_flow_scoring: config.data_flow_scoring.or(preset_config.data_flow_scoring),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_levels_distinct() {
        let strict = PresetLevel::Strict.thresholds();
        let balanced = PresetLevel::Balanced.thresholds();
        let permissive = PresetLevel::Permissive.thresholds();

        // Strict should have lowest thresholds
        assert!(strict.complexity.unwrap() < balanced.complexity.unwrap());
        assert!(balanced.complexity.unwrap() < permissive.complexity.unwrap());
    }

    #[test]
    fn test_preset_scoring_weights_valid() {
        let presets = vec![
            PresetLevel::Strict,
            PresetLevel::Balanced,
            PresetLevel::Permissive,
        ];

        for preset in presets {
            let weights = preset.scoring_weights();
            assert!(weights.validate().is_ok());

            // Check sum is approximately 1.0
            let sum = weights.coverage
                + weights.complexity
                + weights.semantic
                + weights.dependency
                + weights.security
                + weights.organization;
            assert!((sum - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_preset_parse() {
        assert_eq!(PresetLevel::parse("strict"), Some(PresetLevel::Strict));
        assert_eq!(PresetLevel::parse("Strict"), Some(PresetLevel::Strict));
        assert_eq!(PresetLevel::parse("STRICT"), Some(PresetLevel::Strict));
        assert_eq!(
            PresetLevel::parse("balanced"),
            Some(PresetLevel::Balanced)
        );
        assert_eq!(
            PresetLevel::parse("permissive"),
            Some(PresetLevel::Permissive)
        );
        assert_eq!(
            PresetLevel::parse("lenient"),
            Some(PresetLevel::Permissive)
        );
        assert_eq!(PresetLevel::parse("invalid"), None);
    }

    #[test]
    fn test_preset_display() {
        assert_eq!(PresetLevel::Strict.to_string(), "strict");
        assert_eq!(PresetLevel::Balanced.to_string(), "balanced");
        assert_eq!(PresetLevel::Permissive.to_string(), "permissive");
    }

    #[test]
    fn test_merge_preset_with_config() {
        let preset = PresetLevel::Strict;
        let custom_config = DebtmapConfig {
            scoring: Some(ScoringWeights {
                coverage: 0.60,
                complexity: 0.25,
                semantic: 0.00,
                dependency: 0.15,
                security: 0.00,
                organization: 0.00,
            }),
            ..Default::default()
        };

        let merged = merge_preset_with_config(preset, custom_config);

        // Custom scoring should override preset
        assert_eq!(merged.scoring.unwrap().coverage, 0.60);

        // Thresholds should come from preset (not in custom_config)
        assert!(merged.thresholds.is_some());
    }

    #[test]
    fn test_strict_preset_characteristics() {
        let config = PresetLevel::Strict.to_config();
        let thresholds = config.thresholds.unwrap();
        let scoring = config.scoring.unwrap();

        // Strict should have low complexity thresholds
        assert_eq!(thresholds.complexity, Some(20));
        assert_eq!(thresholds.duplication, Some(5));

        // Strict should emphasize coverage
        assert!(scoring.coverage > 0.50);
    }

    #[test]
    fn test_permissive_preset_characteristics() {
        let config = PresetLevel::Permissive.to_config();
        let thresholds = config.thresholds.unwrap();
        let scoring = config.scoring.unwrap();

        // Permissive should have high complexity thresholds
        assert_eq!(thresholds.complexity, Some(100));
        assert_eq!(thresholds.duplication, Some(20));

        // Permissive should de-emphasize coverage
        assert!(scoring.coverage < 0.50);
    }

    #[test]
    fn test_balanced_is_default() {
        let balanced = PresetLevel::Balanced.to_config();

        // Balanced scoring should match defaults
        assert_eq!(
            balanced.scoring.unwrap().coverage,
            ScoringWeights::default().coverage
        );
    }
}

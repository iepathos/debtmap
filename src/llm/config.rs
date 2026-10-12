//! `[llm_editability]` configuration section.

use super::churn::ChurnConfig;
use super::features::Feature;
use super::languages::LanguageOverride;
use super::profiles::ProfileSpec;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Models evaluated when the user does not list any: one per capability tier.
pub const DEFAULT_MODELS: [&str; 4] = [
    "tier:frontier_reasoning",
    "tier:frontier",
    "tier:mid",
    "tier:small_local",
];

/// Score cutoffs (0–100) for the `feasible` and `review` verdicts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Thresholds {
    pub feasible: f64,
    pub review: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            feasible: 75.0,
            review: 50.0,
        }
    }
}

impl Thresholds {
    pub fn validate(&self, context: &str) -> Result<(), String> {
        let in_range = |v: f64| (0.0..=100.0).contains(&v);
        if !in_range(self.feasible) || !in_range(self.review) {
            return Err(format!("{context}: thresholds must be between 0 and 100"));
        }
        if self.review > self.feasible {
            return Err(format!(
                "{context}: review threshold ({}) must not exceed feasible threshold ({})",
                self.review, self.feasible
            ));
        }
        Ok(())
    }
}

/// LLM editability estimate configuration. Every field has a default.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmEditabilityConfig {
    pub enabled: bool,
    /// Model ids or aliases to evaluate; empty means [`DEFAULT_MODELS`].
    pub models: Vec<String>,
    pub thresholds: Thresholds,
    /// Global per-feature weights; missing features use [`Feature::default_weight`].
    pub weights: BTreeMap<Feature, f64>,
    /// Estimated tokens needed to understand each callee.
    pub tokens_per_dependency: f64,
    /// Reasoning load added per nesting level.
    pub nesting_weight: f64,
    /// Coverage assumed when no coverage data was provided.
    pub unknown_coverage: f64,
    /// Overrides for built-in language profiles, keyed by language or `default`.
    pub languages: BTreeMap<String, LanguageOverride>,
    /// Overrides for built-in model profiles, or new custom profiles.
    pub profiles: BTreeMap<String, ProfileSpec>,
    /// Recent LLM churn detection from git history.
    pub churn: ChurnConfig,
}

impl Default for LlmEditabilityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            models: Vec::new(),
            thresholds: Thresholds::default(),
            weights: BTreeMap::new(),
            tokens_per_dependency: 150.0,
            nesting_weight: 2.0,
            unknown_coverage: 0.5,
            languages: BTreeMap::new(),
            profiles: BTreeMap::new(),
            churn: ChurnConfig::default(),
        }
    }
}

impl LlmEditabilityConfig {
    pub fn weight(&self, feature: Feature) -> f64 {
        self.weights
            .get(&feature)
            .copied()
            .unwrap_or_else(|| feature.default_weight())
    }

    /// Model names to evaluate, falling back to one model per tier.
    pub fn model_names(&self) -> Vec<String> {
        if self.models.is_empty() {
            DEFAULT_MODELS.iter().map(|m| m.to_string()).collect()
        } else {
            self.models.clone()
        }
    }

    /// Validate scalar settings; profiles and languages are validated on resolution.
    pub fn validate(&self) -> Result<(), String> {
        self.thresholds.validate("llm_editability")?;
        if let Some((feature, w)) = self.weights.iter().find(|(_, w)| **w < 0.0) {
            return Err(format!(
                "llm_editability.weights.{}: must be >= 0, got {w}",
                feature.as_str()
            ));
        }
        if self.tokens_per_dependency < 0.0 || self.nesting_weight < 0.0 {
            return Err(
                "llm_editability: tokens_per_dependency and nesting_weight must be >= 0".into(),
            );
        }
        if !(0.0..=1.0).contains(&self.unknown_coverage) {
            return Err("llm_editability.unknown_coverage must be between 0 and 1".into());
        }
        self.churn.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_toml_keeps_defaults() {
        let config: LlmEditabilityConfig = toml::from_str(
            r#"
enabled = true
thresholds = { feasible = 80.0 }
weights = { ripple = 2.0 }
churn = { window_days = 7, agents = ["*our-bot*"] }
"#,
        )
        .unwrap();
        assert!(config.enabled);
        assert_eq!(config.thresholds.feasible, 80.0);
        assert_eq!(config.thresholds.review, 50.0);
        assert_eq!(config.weight(Feature::Ripple), 2.0);
        assert_eq!(config.weight(Feature::Reasoning), 1.5);
        assert_eq!(config.tokens_per_dependency, 150.0);
        assert_eq!(config.churn.window_days, 7);
        assert!(config.churn.builtin_agents);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn empty_model_list_uses_one_model_per_tier() {
        assert_eq!(LlmEditabilityConfig::default().model_names().len(), 4);
    }

    #[test]
    fn rejects_review_above_feasible() {
        let config = LlmEditabilityConfig {
            thresholds: Thresholds {
                feasible: 40.0,
                review: 60.0,
            },
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_negative_weight_and_bad_unknown_coverage() {
        let negative = LlmEditabilityConfig {
            weights: BTreeMap::from([(Feature::Ripple, -1.0)]),
            ..Default::default()
        };
        assert!(negative.validate().is_err());
        let coverage = LlmEditabilityConfig {
            unknown_coverage: 1.5,
            ..Default::default()
        };
        assert!(coverage.validate().is_err());
    }
}

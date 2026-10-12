//! Editability features extracted from existing per-function analysis data.

use super::churn::ChurnSummary;
use super::config::LlmEditabilityConfig;
use super::languages::LanguageProfile;
use crate::core::PurityLevel;
use crate::priority::UnifiedDebtItem;
use crate::risk::crap::{CoverageBasis, item_coverage};
use serde::{Deserialize, Serialize};

/// A signal that makes a function harder for an LLM to edit correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    /// Size of the function itself, verbosity-normalized tokens.
    UnitTokens,
    /// Function plus the callee signatures needed to understand it.
    ContextTokens,
    /// Entropy-adjusted cognitive complexity plus nesting.
    Reasoning,
    /// Production callers that may need updating after an edit.
    Ripple,
    /// How likely a wrong edit goes unnoticed (tests and type checking), 0–1.
    VerificationGap,
    /// Side effects from purity analysis, 0–1.
    SideEffects,
    /// Recent LLM rework from git history (see [`super::churn`]).
    Rework,
}

impl Feature {
    pub const ALL: [Feature; 7] = [
        Feature::UnitTokens,
        Feature::ContextTokens,
        Feature::Reasoning,
        Feature::Ripple,
        Feature::VerificationGap,
        Feature::SideEffects,
        Feature::Rework,
    ];

    /// Ratio features are already in 0–1; count features need a tolerance.
    pub fn is_ratio(self) -> bool {
        matches!(self, Feature::VerificationGap | Feature::SideEffects)
    }

    pub fn default_weight(self) -> f64 {
        match self {
            Feature::UnitTokens | Feature::ContextTokens => 1.0,
            Feature::Reasoning | Feature::VerificationGap => 1.5,
            Feature::Ripple | Feature::Rework => 1.2,
            Feature::SideEffects => 0.8,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Feature::UnitTokens => "unit_tokens",
            Feature::ContextTokens => "context_tokens",
            Feature::Reasoning => "reasoning",
            Feature::Ripple => "ripple",
            Feature::VerificationGap => "verification_gap",
            Feature::SideEffects => "side_effects",
            Feature::Rework => "rework",
        }
    }
}

/// Raw inputs kept for writing concrete, human-readable actions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeatureInputs {
    pub lines: f64,
    pub fan_out: f64,
    pub coverage: Option<f64>,
    /// Multiplier on the test gap: 0.5 with a strong type checker, 1.0 with none.
    pub verification_factor: f64,
    pub tokens_per_line: f64,
    pub verbosity: f64,
    /// `None` without function-level git history.
    pub churn: Option<ChurnSummary>,
}

/// Feature values for one function.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EditabilityFeatures {
    pub unit_tokens: f64,
    pub context_tokens: f64,
    /// Un-normalized token estimate, checked against model context limits.
    pub raw_context_tokens: f64,
    pub reasoning: f64,
    pub ripple: f64,
    pub verification_gap: f64,
    pub side_effects: f64,
    pub rework: f64,
    /// Older LLM edits never followed by a fix; raises the score, capped.
    pub proven_edits: f64,
    #[serde(skip)]
    pub inputs: FeatureInputs,
}

impl EditabilityFeatures {
    pub fn from_item(
        item: &UnifiedDebtItem,
        config: &LlmEditabilityConfig,
        language: &LanguageProfile,
        basis: CoverageBasis,
    ) -> Self {
        let lines = item.function_length as f64;
        let fan_out = item
            .downstream_dependencies
            .max(item.downstream_callees.len()) as f64;
        let raw_unit = lines * language.tokens_per_line;
        let raw_context = raw_unit + fan_out * config.tokens_per_dependency;
        let coverage = item_coverage(item, basis);
        let verification_factor = 0.5 + 0.5 * language.type_check_gap;
        let tested = coverage.unwrap_or(config.unknown_coverage).clamp(0.0, 1.0);
        Self {
            unit_tokens: raw_unit / language.verbosity,
            context_tokens: raw_context / language.verbosity,
            raw_context_tokens: raw_context,
            reasoning: reasoning_load(item, config.nesting_weight),
            ripple: production_fan_in(item) as f64,
            verification_gap: (1.0 - tested) * verification_factor,
            side_effects: side_effect_level(item),
            rework: 0.0,
            proven_edits: 0.0,
            inputs: FeatureInputs {
                lines,
                fan_out,
                coverage,
                verification_factor,
                tokens_per_line: language.tokens_per_line,
                verbosity: language.verbosity,
                churn: None,
            },
        }
    }

    /// Add the git-history churn signals.
    pub fn with_churn(self, churn: Option<ChurnSummary>) -> Self {
        Self {
            rework: churn.as_ref().map(ChurnSummary::rework).unwrap_or(0.0),
            proven_edits: churn
                .as_ref()
                .map(|c| c.proven_llm_edits as f64)
                .unwrap_or(0.0),
            inputs: FeatureInputs {
                churn,
                ..self.inputs
            },
            ..self
        }
    }

    pub fn get(&self, feature: Feature) -> f64 {
        match feature {
            Feature::UnitTokens => self.unit_tokens,
            Feature::ContextTokens => self.context_tokens,
            Feature::Reasoning => self.reasoning,
            Feature::Ripple => self.ripple,
            Feature::VerificationGap => self.verification_gap,
            Feature::SideEffects => self.side_effects,
            Feature::Rework => self.rework,
        }
    }

    /// Copy with one feature replaced.
    pub fn with(&self, feature: Feature, value: f64) -> Self {
        let mut next = self.clone();
        match feature {
            Feature::UnitTokens => next.unit_tokens = value,
            Feature::ContextTokens => next.context_tokens = value,
            Feature::Reasoning => next.reasoning = value,
            Feature::Ripple => next.ripple = value,
            Feature::VerificationGap => next.verification_gap = value,
            Feature::SideEffects => next.side_effects = value,
            Feature::Rework => next.rework = value,
        }
        next
    }
}

fn reasoning_load(item: &UnifiedDebtItem, nesting_weight: f64) -> f64 {
    let cognitive = item
        .entropy_analysis
        .as_ref()
        .map(|e| e.adjusted_complexity)
        .unwrap_or(item.cognitive_complexity);
    cognitive as f64 + nesting_weight * item.nesting_depth as f64
}

/// Production callers when caller classification ran, otherwise all callers.
fn production_fan_in(item: &UnifiedDebtItem) -> usize {
    if item.upstream_production_callers.is_empty() && item.upstream_test_callers.is_empty() {
        item.upstream_dependencies
    } else {
        item.upstream_production_callers.len()
    }
}

fn side_effect_level(item: &UnifiedDebtItem) -> f64 {
    match (item.purity_level, item.is_pure) {
        (Some(PurityLevel::StrictlyPure), _) => 0.0,
        (Some(PurityLevel::LocallyPure), _) => 0.2,
        (Some(PurityLevel::ReadOnly), _) => 0.4,
        (Some(PurityLevel::Impure), _) | (None, Some(false)) => 1.0,
        (None, Some(true)) => 0.0,
        (None, None) => 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_and_with_round_trip_every_feature() {
        let base = EditabilityFeatures::default();
        for (i, feature) in Feature::ALL.into_iter().enumerate() {
            let value = i as f64 + 1.0;
            assert_eq!(base.with(feature, value).get(feature), value);
        }
    }

    #[test]
    fn feature_names_match_serde() {
        for feature in Feature::ALL {
            let json = serde_json::to_string(&feature).unwrap();
            assert_eq!(json, format!("\"{}\"", feature.as_str()));
        }
    }

    #[test]
    fn only_verification_and_side_effects_are_ratios() {
        let ratios: Vec<_> = Feature::ALL.into_iter().filter(|f| f.is_ratio()).collect();
        assert_eq!(ratios, [Feature::VerificationGap, Feature::SideEffects]);
    }
}

//! LLM editability estimate.
//!
//! Estimates, per function and per configured model, how likely an LLM is to
//! edit the function correctly, recommends the cheapest model likely to
//! succeed, and lists concrete changes that would make the function easier to
//! edit. The estimate is a heuristic meant to drive agent rules ("refactor
//! functions debtmap flags as hard for LLMs"): it ranks functions and explains
//! itself, but is not a calibrated probability.
//!
//! All inputs come from existing analysis data: size, complexity, nesting,
//! callers and callees, coverage, purity, per-language verbosity and, with
//! git history, recent LLM-co-authored churn.

mod churn;
mod config;
mod features;
mod languages;
mod profiles;
mod score;

pub use churn::{
    AgentMatcher, BUILTIN_AGENTS, ChurnConfig, ChurnSummary, FixGroup, TopicConfig, summarize,
};
pub use config::{DEFAULT_MODELS, LlmEditabilityConfig, Thresholds};
pub use features::{EditabilityFeatures, Feature, FeatureInputs};
pub use languages::{
    LanguageOverride, LanguageProfile, LanguageProfiles, language_key, profile_for,
    resolve_languages,
};
pub use profiles::{ModelProfile, ProfileSpec, Registry};
pub use score::{
    Action, ModelScore, Verdict, actions, assess, cross_scope_text, fix_burst_text, recommend,
    round1, score,
};

use crate::core::Language;
use crate::priority::UnifiedDebtItem;
use crate::risk::context::ContextDetails;
use crate::risk::context::git_history::activity::CommitActivity;
use crate::risk::crap::CoverageBasis;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Per-function editability result, as emitted in JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmEditabilityOutput {
    /// `heuristic` until calibration support exists.
    pub basis: String,
    pub language: String,
    pub features: EditabilityFeatures,
    pub models: Vec<ModelScore>,
    pub recommended_model: Option<String>,
    pub actions: Vec<Action>,
    /// Recent LLM churn; `null` without function-level git history.
    #[serde(default)]
    pub churn: Option<ChurnSummary>,
}

/// Resolved configuration, language profiles and model profiles.
#[derive(Debug, Clone)]
pub struct Evaluator {
    config: LlmEditabilityConfig,
    languages: LanguageProfiles,
    models: Vec<ModelProfile>,
    coverage_basis: CoverageBasis,
    agents: AgentMatcher,
    /// Reference time for the churn window.
    now: DateTime<Utc>,
}

impl Evaluator {
    pub fn new(
        config: &LlmEditabilityConfig,
        coverage_basis: CoverageBasis,
    ) -> Result<Self, String> {
        config.validate()?;
        let registry = Registry::load(&config.profiles)?;
        let models = config
            .model_names()
            .iter()
            .map(|name| registry.find(name).and_then(|id| registry.resolve(&id)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            config: config.clone(),
            languages: resolve_languages(&config.languages)?,
            models,
            coverage_basis,
            agents: AgentMatcher::new(&config.churn)?,
            now: Utc::now(),
        })
    }

    pub fn models(&self) -> &[ModelProfile] {
        &self.models
    }

    pub fn evaluate(&self, item: &UnifiedDebtItem) -> LlmEditabilityOutput {
        let language = Language::from_path(&item.location.file);
        let profile = profile_for(&self.languages, language);
        let churn = commit_activity(item)
            .map(|a| summarize(a, &self.agents, self.now, self.config.churn.window_days));
        let features =
            EditabilityFeatures::from_item(item, &self.config, &profile, self.coverage_basis)
                .with_churn(churn);
        self.evaluate_features(features, language_key(language))
    }

    pub fn evaluate_features(
        &self,
        features: EditabilityFeatures,
        language: &str,
    ) -> LlmEditabilityOutput {
        let scores: Vec<ModelScore> = self
            .models
            .iter()
            .map(|p| assess(&features, p, &self.config))
            .collect();
        let recommended_model = recommend(&self.models, &scores);
        LlmEditabilityOutput {
            basis: "heuristic".to_string(),
            language: language.to_string(),
            actions: actions(&features, &self.models, &self.config),
            churn: features.inputs.churn.clone(),
            features: rounded(features),
            models: scores
                .into_iter()
                .map(|s| ModelScore {
                    score: round1(s.score),
                    ..s
                })
                .collect(),
            recommended_model,
        }
    }
}

fn rounded(f: EditabilityFeatures) -> EditabilityFeatures {
    let r2 = |v: f64| (v * 100.0).round() / 100.0;
    EditabilityFeatures {
        unit_tokens: f.unit_tokens.round(),
        context_tokens: f.context_tokens.round(),
        raw_context_tokens: f.raw_context_tokens.round(),
        reasoning: r2(f.reasoning),
        ripple: f.ripple,
        verification_gap: r2(f.verification_gap),
        side_effects: r2(f.side_effects),
        rework: f.rework,
        proven_edits: f.proven_edits,
        inputs: f.inputs,
    }
}

/// Function-level commit facts; `None` without them (file-level history only).
fn commit_activity(item: &UnifiedDebtItem) -> Option<&CommitActivity> {
    item.contextual_risk
        .as_ref()?
        .contexts
        .iter()
        .find_map(|c| match &c.details {
            ContextDetails::Historical { activity, .. } if !activity.commits.is_empty() => {
                Some(activity)
            }
            _ => None,
        })
}

/// Build the evaluator for a configuration, or `None` when the feature is disabled.
pub fn evaluator_for(config: &crate::config::DebtmapConfig) -> Result<Option<Evaluator>, String> {
    let basis = config.crap.clone().unwrap_or_default().coverage_source;
    config
        .llm_editability
        .as_ref()
        .filter(|llm| llm.enabled)
        .map(|llm| Evaluator::new(llm, basis))
        .transpose()
}

/// Process-wide evaluator from the installed configuration.
pub fn evaluator() -> Option<&'static Evaluator> {
    static EVALUATOR: OnceLock<Option<Evaluator>> = OnceLock::new();
    EVALUATOR
        .get_or_init(|| {
            evaluator_for(crate::config::get_config())
                .map_err(|e| log::warn!("LLM editability disabled: {e}"))
                .ok()
                .flatten()
        })
        .as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evaluator() -> Evaluator {
        Evaluator::new(&LlmEditabilityConfig::default(), CoverageBasis::Direct).unwrap()
    }

    #[test]
    fn default_config_evaluates_one_model_per_tier() {
        assert_eq!(evaluator().models().len(), 4);
    }

    #[test]
    fn unknown_model_name_fails_construction() {
        let config = LlmEditabilityConfig {
            models: vec!["no-such-model".to_string()],
            ..Default::default()
        };
        assert!(Evaluator::new(&config, CoverageBasis::Direct).is_err());
    }

    #[test]
    fn named_models_resolve_through_aliases() {
        let config = LlmEditabilityConfig {
            models: vec!["claude-sonnet-4-5".to_string(), "gpt-5-mini".to_string()],
            ..Default::default()
        };
        let ids: Vec<_> = Evaluator::new(&config, CoverageBasis::Direct)
            .unwrap()
            .models()
            .iter()
            .map(|m| m.id.clone())
            .collect();
        assert_eq!(ids, ["claude-sonnet", "gpt-mini"]);
    }

    #[test]
    fn disabled_config_has_no_evaluator() {
        let config = crate::config::DebtmapConfig::default();
        assert!(evaluator_for(&config).unwrap().is_none());
    }

    #[test]
    fn output_serializes_scores_and_actions() {
        let features = EditabilityFeatures {
            unit_tokens: 830.0,
            context_tokens: 1750.0,
            raw_context_tokens: 2280.0,
            reasoning: 40.0,
            ripple: 12.0,
            verification_gap: 0.55,
            side_effects: 1.0,
            rework: 0.0,
            proven_edits: 0.0,
            inputs: FeatureInputs {
                lines: 120.0,
                fan_out: 8.0,
                coverage: None,
                verification_factor: 0.55,
                tokens_per_line: 9.0,
                verbosity: 1.3,
                churn: None,
            },
        };
        let output = evaluator().evaluate_features(features, "rust");
        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["basis"], "heuristic");
        assert!(json.as_object().unwrap().contains_key("churn"));
        assert!(json["churn"].is_null());
        assert_eq!(json["features"]["rework"], 0.0);
        assert_eq!(json["models"].as_array().unwrap().len(), 4);
        assert!(json["recommended_model"].is_string() || json["recommended_model"].is_null());
        assert!(!json["actions"].as_array().unwrap().is_empty());
        assert!(json["features"].get("inputs").is_none());
        let back: LlmEditabilityOutput = serde_json::from_value(json).unwrap();
        assert_eq!(back.models.len(), 4);
    }
}

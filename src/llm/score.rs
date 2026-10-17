//! Scoring: saturating per-feature difficulty combined through a logistic.
//!
//! For count features `d = x / (x + tolerance)`; ratio features use `d = x`.
//! `score = 100 · σ(capability − Σ weightᵢ · sensitivityᵢ · dᵢ)`.
//! The score ranks functions and explains itself; it is not a calibrated
//! probability.

use super::churn::{ChurnSummary, FixGroup};
use super::config::{LlmEditabilityConfig, Thresholds};
use super::features::{EditabilityFeatures, Feature};
use super::profiles::ModelProfile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const ACTION_MIN_GAIN: f64 = 1.0;
const TARGET_COVERAGE: f64 = 0.8;
const TARGET_SIDE_EFFECTS: f64 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Feasible,
    Review,
    Human,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Feasible => "feasible",
            Verdict::Review => "review",
            Verdict::Human => "human",
        }
    }
}

/// One model's assessment of one function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelScore {
    pub model: String,
    pub score: f64,
    pub verdict: Verdict,
    /// Features with the largest penalties, largest first.
    pub top_factors: Vec<Feature>,
    /// Set when the verdict was forced, e.g. `context_overflow`.
    #[serde(default)]
    pub reason: Option<String>,
}

/// A concrete change that would make the function easier to edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub feature: Feature,
    pub text: String,
    /// Estimated score per model after the change; empty for advice-only actions.
    pub score_after: BTreeMap<String, f64>,
}

pub fn difficulty(feature: Feature, value: f64, profile: &ModelProfile) -> f64 {
    let value = value.max(0.0);
    if feature.is_ratio() {
        value.min(1.0)
    } else {
        value / (value + profile.tolerance(feature))
    }
}

/// Weighted penalty contributed by each feature.
pub fn penalties(
    features: &EditabilityFeatures,
    profile: &ModelProfile,
    config: &LlmEditabilityConfig,
) -> Vec<(Feature, f64)> {
    Feature::ALL
        .into_iter()
        .map(|f| {
            let d = difficulty(f, features.get(f), profile);
            (f, config.weight(f) * profile.sensitivity(f) * d)
        })
        .collect()
}

pub fn score(
    features: &EditabilityFeatures,
    profile: &ModelProfile,
    config: &LlmEditabilityConfig,
) -> f64 {
    let total: f64 = penalties(features, profile, config)
        .iter()
        .map(|(_, p)| p)
        .sum();
    let capability = profile.capability + config.churn.proven_bonus(features.proven_edits);
    100.0 / (1.0 + (total - capability).exp())
}

pub fn assess(
    features: &EditabilityFeatures,
    profile: &ModelProfile,
    config: &LlmEditabilityConfig,
) -> ModelScore {
    let value = score(features, profile, config);
    let overflow = features.raw_context_tokens > profile.effective_context_tokens as f64;
    let thresholds = profile.thresholds.unwrap_or(config.thresholds);
    ModelScore {
        model: profile.id.clone(),
        score: value,
        verdict: if overflow {
            Verdict::Human
        } else {
            verdict(value, thresholds)
        },
        top_factors: top_factors(penalties(features, profile, config), 2),
        reason: overflow.then(|| "context_overflow".to_string()),
    }
}

fn verdict(score: f64, thresholds: Thresholds) -> Verdict {
    if score >= thresholds.feasible {
        Verdict::Feasible
    } else if score >= thresholds.review {
        Verdict::Review
    } else {
        Verdict::Human
    }
}

fn top_factors(mut penalties: Vec<(Feature, f64)>, n: usize) -> Vec<Feature> {
    penalties.sort_by(|a, b| b.1.total_cmp(&a.1));
    penalties
        .into_iter()
        .filter(|(_, p)| *p > 0.0)
        .take(n)
        .map(|(f, _)| f)
        .collect()
}

/// Cheapest model whose verdict is `feasible`; ties go to the higher score.
pub fn recommend(profiles: &[ModelProfile], scores: &[ModelScore]) -> Option<String> {
    profiles
        .iter()
        .zip(scores)
        .filter(|(_, s)| s.verdict == Verdict::Feasible)
        .min_by(|(pa, sa), (pb, sb)| {
            pa.cost_tier
                .cmp(&pb.cost_tier)
                .then_with(|| sb.score.total_cmp(&sa.score))
        })
        .map(|(p, _)| p.id.clone())
}

/// Fixes the agent can make inside the function, most impactful first.
/// Rework and ripple can't be refactored away directly, so they are
/// advice-only and listed last, ripple last of all.
pub fn actions(
    features: &EditabilityFeatures,
    profiles: &[ModelProfile],
    config: &LlmEditabilityConfig,
) -> Vec<Action> {
    let mut scored: Vec<(f64, Action)> = [
        Feature::UnitTokens,
        Feature::Reasoning,
        Feature::SideEffects,
        Feature::VerificationGap,
        Feature::ContextTokens,
    ]
    .into_iter()
    .filter_map(|f| scored_action(f, features, profiles, config))
    .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored
        .into_iter()
        .map(|(_, a)| a)
        .chain(rework_advice(features))
        .chain(ripple_advice(features, profiles))
        .collect()
}

fn scored_action(
    feature: Feature,
    features: &EditabilityFeatures,
    profiles: &[ModelProfile],
    config: &LlmEditabilityConfig,
) -> Option<(f64, Action)> {
    let target = target_value(feature, features, profiles)?;
    if features.get(feature) <= target {
        return None;
    }
    let improved = features.with(feature, target);
    let gains: Vec<(String, f64, f64)> = profiles
        .iter()
        .map(|p| {
            let after = score(&improved, p, config);
            (p.id.clone(), after, after - score(features, p, config))
        })
        .collect();
    let max_gain = gains.iter().map(|g| g.2).fold(0.0, f64::max);
    (max_gain >= ACTION_MIN_GAIN).then(|| {
        let total_gain = gains.iter().map(|g| g.2).sum();
        let score_after = gains.into_iter().map(|(id, after, _)| (id, round1(after)));
        (
            total_gain,
            Action {
                feature,
                text: action_text(feature, features, target),
                score_after: score_after.collect(),
            },
        )
    })
}

/// Value the feature is lowered to when estimating an action's effect.
fn target_value(
    feature: Feature,
    features: &EditabilityFeatures,
    profiles: &[ModelProfile],
) -> Option<f64> {
    match feature {
        Feature::SideEffects => Some(TARGET_SIDE_EFFECTS),
        Feature::VerificationGap => {
            Some((1.0 - TARGET_COVERAGE) * features.inputs.verification_factor)
        }
        Feature::Ripple | Feature::Rework => None,
        Feature::ContextTokens => profiles
            .iter()
            .map(|p| p.tolerance(feature))
            .min_by(f64::total_cmp)
            .map(|target| target.max(features.unit_tokens)),
        _ => profiles
            .iter()
            .map(|p| p.tolerance(feature))
            .min_by(f64::total_cmp),
    }
}

fn action_text(feature: Feature, features: &EditabilityFeatures, target: f64) -> String {
    let inputs = &features.inputs;
    match feature {
        Feature::UnitTokens => {
            let target_lines = target * inputs.verbosity / inputs.tokens_per_line;
            format!(
                "Split into smaller functions ({:.0} lines now; aim for under {:.0} lines)",
                inputs.lines, target_lines
            )
        }
        Feature::Reasoning => format!(
            "Flatten nesting with early returns and extract branch bodies into named functions \
             (reasoning load {:.0}; aim for under {target:.0})",
            features.reasoning
        ),
        Feature::SideEffects => {
            "Move I/O and mutation to the edges and keep a pure core".to_string()
        }
        Feature::VerificationGap => match inputs.coverage {
            Some(c) => format!(
                "Add tests covering this function (coverage {:.0}%; aim for {:.0}%)",
                c * 100.0,
                TARGET_COVERAGE * 100.0
            ),
            None => format!(
                "Add tests covering this function (no coverage data; aim for {:.0}%)",
                TARGET_COVERAGE * 100.0
            ),
        },
        Feature::ContextTokens => format!(
            "Reduce the number of callees ({:.0} now) or group them behind one interface",
            inputs.fan_out
        ),
        Feature::Ripple => ripple_text(features.ripple),
        Feature::Rework => rework_text(inputs.churn.as_ref().unwrap_or(&Default::default())),
    }
}

fn rework_text(churn: &ChurnSummary) -> String {
    let signals: Vec<String> = [
        (churn.llm_edits > 1).then(|| format!("{} LLM edits", churn.llm_edits)),
        (churn.fixes_after_llm_edit > 0)
            .then(|| format!("{} fixes after LLM edits", churn.fixes_after_llm_edit)),
        (churn.reverts_of_llm_edits > 0)
            .then(|| format!("{} reverted LLM edits", churn.reverts_of_llm_edits)),
        (churn.fix_burst > 1).then(|| fix_burst_text(churn)),
        (churn.corroborated_fixes > 0).then(|| {
            format!(
                "{} fixes matching both scope and topic",
                churn.corroborated_fixes
            )
        }),
        cross_scope_text(churn),
    ]
    .into_iter()
    .flatten()
    .collect();
    format!(
        "LLMs keep reworking this ({} in the last {} days): pin the intended behavior \
         with a failing test and simplify it before the next LLM edit",
        signals.join(", "),
        churn.window_days
    )
}

/// e.g. "3 fixes on this topic under other scopes (lexer: 2, tokens: 1)".
pub fn cross_scope_text(churn: &ChurnSummary) -> Option<String> {
    let total: u32 = churn.cross_scope_fixes.values().sum();
    let scopes: Vec<String> = churn
        .cross_scope_fixes
        .iter()
        .map(|(scope, n)| format!("{scope}: {n}"))
        .collect();
    (total > 0).then(|| {
        format!(
            "{total} fixes on this topic under other scopes ({})",
            scopes.join(", ")
        )
    })
}

pub fn fix_burst_text(churn: &ChurnSummary) -> String {
    let n = churn.fix_burst;
    match &churn.fix_burst_group {
        Some(FixGroup::Scope(scope)) => format!("{n} fix({scope}) commits"),
        Some(FixGroup::File) => format!("{n} fix commits in this file"),
        Some(FixGroup::Topic) => format!("{n} fix commits on the same topic"),
        None => format!("{n} fix commits"),
    }
}

fn rework_advice(features: &EditabilityFeatures) -> Option<Action> {
    (features.rework >= 1.0).then(|| Action {
        feature: Feature::Rework,
        text: action_text(Feature::Rework, features, 0.0),
        score_after: BTreeMap::new(),
    })
}

fn ripple_text(ripple: f64) -> String {
    format!(
        "Keep the signature stable, add a wrapper, or narrow the public surface \
         ({ripple:.0} production callers)"
    )
}

fn ripple_advice(features: &EditabilityFeatures, profiles: &[ModelProfile]) -> Option<Action> {
    let threshold = profiles
        .iter()
        .map(|p| p.tolerance(Feature::Ripple))
        .min_by(f64::total_cmp)?;
    (features.ripple >= threshold).then(|| Action {
        feature: Feature::Ripple,
        text: ripple_text(features.ripple),
        score_after: BTreeMap::new(),
    })
}

pub fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::features::FeatureInputs;
    use crate::llm::profiles::Registry;
    use proptest::prelude::*;

    fn profiles() -> Vec<ModelProfile> {
        let registry = Registry::load(&BTreeMap::new()).unwrap();
        crate::llm::config::DEFAULT_MODELS
            .iter()
            .map(|id| registry.resolve(id).unwrap())
            .collect()
    }

    fn features(
        lines: f64,
        reasoning: f64,
        ripple: f64,
        gap: f64,
        side: f64,
    ) -> EditabilityFeatures {
        let raw_unit = lines * 9.0;
        EditabilityFeatures {
            unit_tokens: raw_unit / 1.3,
            context_tokens: (raw_unit + 1200.0) / 1.3,
            raw_context_tokens: raw_unit + 1200.0,
            reasoning,
            ripple,
            verification_gap: gap,
            side_effects: side,
            rework: 0.0,
            proven_edits: 0.0,
            inputs: FeatureInputs {
                lines,
                fan_out: 8.0,
                coverage: Some(0.0),
                verification_factor: 0.55,
                tokens_per_line: 9.0,
                verbosity: 1.3,
                churn: None,
            },
        }
    }

    fn churn(llm_edits: u32, fixes: u32, proven: u32) -> ChurnSummary {
        ChurnSummary {
            window_days: 14,
            llm_edits,
            fixes_after_llm_edit: fixes,
            proven_llm_edits: proven,
            ..Default::default()
        }
    }

    #[test]
    fn recent_rework_lowers_score_and_adds_advice_before_ripple() {
        let config = LlmEditabilityConfig::default();
        let base = hard();
        let reworked = base.clone().with_churn(Some(churn(4, 2, 0)));
        for p in profiles() {
            assert!(score(&reworked, &p, &config) < score(&base, &p, &config));
        }
        let list = actions(&reworked, &profiles(), &config);
        let n = list.len();
        assert_eq!(list[n - 2].feature, Feature::Rework);
        assert!(
            list[n - 2]
                .text
                .contains("4 LLM edits, 2 fixes after LLM edits in the last 14 days"),
            "{}",
            list[n - 2].text
        );
        assert_eq!(list[n - 1].feature, Feature::Ripple);
    }

    #[test]
    fn proven_llm_edits_raise_score_within_the_cap() {
        let config = LlmEditabilityConfig::default();
        let base = hard();
        let proven = base.clone().with_churn(Some(churn(0, 0, 5)));
        assert_eq!(proven.rework, 0.0);
        for p in profiles() {
            let (before, after) = (score(&base, &p, &config), score(&proven, &p, &config));
            assert!(after > before);
            let mut capped = p.clone();
            capped.capability += config.churn.proven_edit_bonus;
            assert!(after < score(&base, &capped, &config));
        }
        assert!(
            actions(&proven, &profiles(), &config)
                .iter()
                .all(|a| a.feature != Feature::Rework)
        );
    }

    fn hard() -> EditabilityFeatures {
        features(120.0, 45.0, 15.0, 0.55, 1.0)
    }

    fn easy() -> EditabilityFeatures {
        features(8.0, 2.0, 1.0, 0.0, 0.0)
    }

    #[test]
    fn easy_functions_outscore_hard_ones_for_every_model() {
        let config = LlmEditabilityConfig::default();
        for p in profiles() {
            assert!(score(&easy(), &p, &config) > score(&hard(), &p, &config));
        }
    }

    #[test]
    fn stronger_tiers_score_higher() {
        let config = LlmEditabilityConfig::default();
        let scores: Vec<f64> = profiles()
            .iter()
            .map(|p| score(&hard(), p, &config))
            .collect();
        assert!(scores.windows(2).all(|w| w[0] > w[1]), "{scores:?}");
    }

    #[test]
    fn easy_function_is_feasible_even_for_small_models() {
        let config = LlmEditabilityConfig::default();
        assert!(
            profiles()
                .iter()
                .all(|p| assess(&easy(), p, &config).verdict == Verdict::Feasible)
        );
    }

    #[test]
    fn context_overflow_forces_human() {
        let config = LlmEditabilityConfig::default();
        let small = profiles().pop().unwrap();
        let mut f = easy();
        f.raw_context_tokens = small.effective_context_tokens as f64 + 1.0;
        let assessed = assess(&f, &small, &config);
        assert_eq!(assessed.verdict, Verdict::Human);
        assert_eq!(assessed.reason.as_deref(), Some("context_overflow"));
    }

    #[test]
    fn recommends_cheapest_feasible_model() {
        let config = LlmEditabilityConfig::default();
        let profiles = profiles();
        let easy_scores: Vec<_> = profiles
            .iter()
            .map(|p| assess(&easy(), p, &config))
            .collect();
        assert_eq!(
            recommend(&profiles, &easy_scores).as_deref(),
            Some("tier:small_local")
        );
        let impossible = features(2000.0, 300.0, 200.0, 1.0, 1.0);
        let scores: Vec<_> = profiles
            .iter()
            .map(|p| assess(&impossible, p, &config))
            .collect();
        assert_eq!(recommend(&profiles, &scores), None);
    }

    #[test]
    fn hard_function_gets_actions_with_ripple_last() {
        let config = LlmEditabilityConfig::default();
        let list = actions(&hard(), &profiles(), &config);
        assert!(list.len() >= 3, "{list:?}");
        assert_eq!(list.last().map(|a| a.feature), Some(Feature::Ripple));
        assert!(list.last().unwrap().score_after.is_empty());
    }

    #[test]
    fn callee_action_is_absent_when_no_callees_remain() {
        let config = LlmEditabilityConfig::default();
        let mut features = features(500.0, 2.0, 1.0, 0.0, 0.0);
        features.inputs.fan_out = 0.0;
        features.context_tokens = features.unit_tokens;
        features.raw_context_tokens = features.unit_tokens * features.inputs.verbosity;
        let list = actions(&features, &profiles(), &config);
        assert!(
            list.iter()
                .all(|action| action.feature != Feature::ContextTokens)
        );
    }

    #[test]
    fn easy_function_gets_no_actions() {
        let config = LlmEditabilityConfig::default();
        assert!(actions(&easy(), &profiles(), &config).is_empty());
    }

    #[test]
    fn unit_token_action_speaks_in_lines() {
        let config = LlmEditabilityConfig::default();
        let list = actions(&hard(), &profiles(), &config);
        let split = list
            .iter()
            .find(|a| a.feature == Feature::UnitTokens)
            .unwrap();
        assert!(split.text.contains("120 lines now"), "{}", split.text);
    }

    #[test]
    fn same_logic_in_go_and_python_gets_similar_size_difficulty() {
        let config = LlmEditabilityConfig::default();
        let p = &profiles()[1];
        // 100 Python lines ≈ 150 Go lines for the same logic (verbosity 1.0 vs 1.5).
        let python = 100.0 * 8.0 / 1.0;
        let go = 150.0 * 8.0 / 1.5;
        let d_py = difficulty(Feature::UnitTokens, python, p);
        let d_go = difficulty(Feature::UnitTokens, go, p);
        assert!((d_py - d_go).abs() < 1e-9);
        assert!(config.weight(Feature::UnitTokens) > 0.0);
    }

    fn arb_features() -> impl Strategy<Value = EditabilityFeatures> {
        (
            1.0f64..2000.0,
            0.0f64..200.0,
            0.0f64..100.0,
            0.0f64..=1.0,
            0.0f64..=1.0,
        )
            .prop_map(|(lines, reasoning, ripple, gap, side)| {
                features(lines, reasoning, ripple, gap, side)
            })
    }

    proptest! {
        #[test]
        fn score_is_strictly_between_0_and_100(f in arb_features()) {
            let config = LlmEditabilityConfig::default();
            for p in profiles() {
                let s = score(&f, &p, &config);
                prop_assert!(s > 0.0 && s < 100.0);
            }
        }

        #[test]
        fn worse_feature_never_raises_score(f in arb_features(), i in 0usize..7, bump in 0.0f64..50.0) {
            let config = LlmEditabilityConfig::default();
            let feature = Feature::ALL[i];
            let worse = f.with(feature, f.get(feature) + bump);
            for p in profiles() {
                prop_assert!(score(&worse, &p, &config) <= score(&f, &p, &config) + 1e-9);
            }
        }

        #[test]
        fn higher_tolerance_never_lowers_score(f in arb_features(), factor in 1.0f64..4.0) {
            let config = LlmEditabilityConfig::default();
            for p in profiles() {
                let mut tolerant = p.clone();
                for t in tolerant.tolerance.values_mut() {
                    *t *= factor;
                }
                prop_assert!(score(&f, &tolerant, &config) >= score(&f, &p, &config) - 1e-9);
            }
        }

        #[test]
        fn every_listed_action_raises_or_keeps_every_score(f in arb_features()) {
            let config = LlmEditabilityConfig::default();
            let profiles = profiles();
            for action in actions(&f, &profiles, &config) {
                for p in &profiles {
                    if let Some(after) = action.score_after.get(&p.id) {
                        prop_assert!(*after + 0.05 >= score(&f, p, &config));
                    }
                }
            }
        }

        #[test]
        fn actions_are_ordered_and_ripple_is_last(f in arb_features()) {
            let config = LlmEditabilityConfig::default();
            let list = actions(&f, &profiles(), &config);
            let ripple_positions: Vec<_> = list.iter().enumerate()
                .filter(|(_, a)| a.feature == Feature::Ripple).map(|(i, _)| i).collect();
            prop_assert!(ripple_positions.iter().all(|i| *i == list.len() - 1));
        }
    }
}

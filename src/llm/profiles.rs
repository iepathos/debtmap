//! Model profile registry: built-in tiers and families plus user overrides.

use super::config::Thresholds;
use super::features::Feature;
use glob::{MatchOptions, Pattern};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const BUILTIN: &str = include_str!("profiles.toml");
const TIER_PREFIX: &str = "tier:";
const DEFAULT_PARENT: &str = "tier:mid";
const MAX_INHERITANCE_DEPTH: usize = 8;

/// A partial profile, as written in the registry or user configuration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileSpec {
    /// Parent profile: `tier:<name>` or another model id.
    pub inherits: Option<String>,
    pub provider: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub capability: Option<f64>,
    pub cost_tier: Option<u8>,
    pub context_window: Option<u64>,
    pub effective_context_tokens: Option<u64>,
    pub tolerance: BTreeMap<Feature, f64>,
    pub sensitivity: BTreeMap<Feature, f64>,
    pub thresholds: Option<Thresholds>,
}

impl ProfileSpec {
    /// Fields set on `self` win over `base`.
    fn over(self, base: ProfileSpec) -> ProfileSpec {
        ProfileSpec {
            inherits: self.inherits.or(base.inherits),
            provider: self.provider.or(base.provider),
            aliases: self.aliases.or(base.aliases),
            capability: self.capability.or(base.capability),
            cost_tier: self.cost_tier.or(base.cost_tier),
            context_window: self.context_window.or(base.context_window),
            effective_context_tokens: self
                .effective_context_tokens
                .or(base.effective_context_tokens),
            tolerance: base.tolerance.into_iter().chain(self.tolerance).collect(),
            sensitivity: base
                .sensitivity
                .into_iter()
                .chain(self.sensitivity)
                .collect(),
            thresholds: self.thresholds.or(base.thresholds),
        }
    }
}

/// A fully resolved model profile.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModelProfile {
    pub id: String,
    pub provider: String,
    pub tier: String,
    pub aliases: Vec<String>,
    pub capability: f64,
    pub cost_tier: u8,
    pub context_window: u64,
    pub effective_context_tokens: u64,
    pub tolerance: BTreeMap<Feature, f64>,
    pub sensitivity: BTreeMap<Feature, f64>,
    pub thresholds: Option<Thresholds>,
}

impl ModelProfile {
    pub fn tolerance(&self, feature: Feature) -> f64 {
        self.tolerance.get(&feature).copied().unwrap_or(1.0)
    }

    pub fn sensitivity(&self, feature: Feature) -> f64 {
        self.sensitivity.get(&feature).copied().unwrap_or(1.0)
    }
}

#[derive(Debug, Deserialize)]
struct RegistryFile {
    tiers: BTreeMap<String, ProfileSpec>,
    models: BTreeMap<String, ProfileSpec>,
}

/// Built-in profiles merged with user profiles.
#[derive(Debug, Clone)]
pub struct Registry {
    tiers: BTreeMap<String, ProfileSpec>,
    models: BTreeMap<String, ProfileSpec>,
}

impl Registry {
    pub fn load(user: &BTreeMap<String, ProfileSpec>) -> Result<Self, String> {
        let file: RegistryFile = toml::from_str(BUILTIN)
            .map_err(|e| format!("built-in model profiles are invalid: {e}"))?;
        let mut registry = Registry {
            tiers: file.tiers,
            models: file.models,
        };
        for (id, spec) in user {
            registry.add_user_profile(id, spec.clone());
        }
        Ok(registry)
    }

    fn add_user_profile(&mut self, id: &str, spec: ProfileSpec) {
        let (map, key) = match id.strip_prefix(TIER_PREFIX) {
            Some(tier) => (&mut self.tiers, tier),
            None => (&mut self.models, id),
        };
        let merged = match map.remove(key) {
            Some(existing) => spec.over(existing),
            None => spec,
        };
        map.insert(key.to_string(), merged);
    }

    /// All selectable ids: tiers first, then model families.
    pub fn ids(&self) -> Vec<String> {
        self.tiers
            .keys()
            .map(|t| format!("{TIER_PREFIX}{t}"))
            .chain(self.models.keys().cloned())
            .collect()
    }

    /// Map a user-supplied name (id or alias) to a profile id.
    pub fn find(&self, name: &str) -> Result<String, String> {
        let wanted = name.trim().to_ascii_lowercase();
        if let Some(id) = self
            .ids()
            .into_iter()
            .find(|id| id.eq_ignore_ascii_case(&wanted))
        {
            return Ok(id);
        }
        self.models
            .iter()
            .flat_map(|(id, spec)| {
                spec.aliases
                    .iter()
                    .flatten()
                    .filter(|alias| alias_matches(alias, &wanted))
                    .map(move |alias| (specificity(alias), id))
            })
            .max_by(|(a, id_a), (b, id_b)| a.cmp(b).then_with(|| id_b.cmp(id_a)))
            .map(|(_, id)| id.clone())
            .ok_or_else(|| {
                format!("unknown LLM model `{name}`; run `debtmap llm-models` to list known models")
            })
    }

    /// Resolve a profile id into a complete, validated profile.
    pub fn resolve(&self, id: &str) -> Result<ModelProfile, String> {
        let spec = self.resolve_spec(id, 0)?;
        into_profile(id, spec, self.tier_of(id, 0))
    }

    fn resolve_spec(&self, id: &str, depth: usize) -> Result<ProfileSpec, String> {
        if depth > MAX_INHERITANCE_DEPTH {
            return Err(format!(
                "LLM profile `{id}`: inheritance cycle or chain too deep"
            ));
        }
        if let Some(tier) = id.strip_prefix(TIER_PREFIX) {
            return self
                .tiers
                .get(tier)
                .cloned()
                .ok_or_else(|| format!("unknown LLM tier `{id}`"));
        }
        let spec = self
            .models
            .get(id)
            .cloned()
            .ok_or_else(|| format!("unknown LLM model profile `{id}`"))?;
        let parent = spec.inherits.clone().unwrap_or(DEFAULT_PARENT.to_string());
        Ok(spec.over(self.resolve_spec(&parent, depth + 1)?))
    }

    fn tier_of(&self, id: &str, depth: usize) -> String {
        if id.starts_with(TIER_PREFIX) || depth > MAX_INHERITANCE_DEPTH {
            return id.to_string();
        }
        let parent = self
            .models
            .get(id)
            .and_then(|s| s.inherits.clone())
            .unwrap_or(DEFAULT_PARENT.to_string());
        self.tier_of(&parent, depth + 1)
    }
}

fn alias_matches(alias: &str, name: &str) -> bool {
    let options = MatchOptions {
        case_sensitive: false,
        ..MatchOptions::new()
    };
    Pattern::new(alias).is_ok_and(|p| p.matches_with(name, options))
}

/// Literal characters in a glob; a `[...]` class counts as one.
fn specificity(alias: &str) -> usize {
    alias
        .split('[')
        .enumerate()
        .map(|(i, part)| {
            let literal = match (i, part.split_once(']')) {
                (0, _) | (_, None) => part,
                (_, Some((_, rest))) => rest,
            };
            literal.chars().filter(|c| !matches!(c, '*' | '?')).count() + usize::from(i > 0)
        })
        .sum()
}

fn into_profile(id: &str, spec: ProfileSpec, tier: String) -> Result<ModelProfile, String> {
    let missing = |field: &str| format!("LLM profile `{id}` is missing `{field}`");
    let profile = ModelProfile {
        id: id.to_string(),
        provider: spec.provider.unwrap_or_else(|| "custom".to_string()),
        tier,
        aliases: spec.aliases.unwrap_or_default(),
        capability: spec.capability.ok_or_else(|| missing("capability"))?,
        cost_tier: spec.cost_tier.ok_or_else(|| missing("cost_tier"))?,
        context_window: spec.context_window.unwrap_or(0),
        effective_context_tokens: spec
            .effective_context_tokens
            .ok_or_else(|| missing("effective_context_tokens"))?,
        tolerance: spec.tolerance,
        sensitivity: spec.sensitivity,
        thresholds: spec.thresholds,
    };
    validate(&profile)?;
    Ok(profile)
}

fn validate(p: &ModelProfile) -> Result<(), String> {
    let context = format!("llm_editability.profiles.{}", p.id);
    if let Some(feature) = Feature::ALL
        .into_iter()
        .filter(|f| !f.is_ratio())
        .find(|f| p.tolerance.get(f).is_none_or(|t| *t <= 0.0))
    {
        return Err(format!(
            "{context}.tolerance.{}: must be set and > 0",
            feature.as_str()
        ));
    }
    if let Some((feature, _)) = p.sensitivity.iter().find(|(_, s)| **s < 0.0) {
        return Err(format!(
            "{context}.sensitivity.{}: must be >= 0",
            feature.as_str()
        ));
    }
    p.thresholds.map(|t| t.validate(&context)).unwrap_or(Ok(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        Registry::load(&BTreeMap::new()).unwrap()
    }

    #[test]
    fn every_builtin_profile_resolves() {
        let registry = registry();
        for id in registry.ids() {
            registry
                .resolve(&id)
                .unwrap_or_else(|e| panic!("{id}: {e}"));
        }
    }

    #[test]
    fn builtin_covers_major_providers() {
        let registry = registry();
        let providers: std::collections::BTreeSet<_> = registry
            .ids()
            .iter()
            .filter_map(|id| registry.resolve(id).ok())
            .map(|p| p.provider)
            .collect();
        for provider in [
            "anthropic",
            "openai",
            "google",
            "xai",
            "meta",
            "mistral",
            "alibaba",
            "deepseek",
        ] {
            assert!(providers.contains(provider), "missing {provider}");
        }
    }

    #[test]
    fn finds_models_by_alias() {
        let registry = registry();
        let cases = [
            ("claude-sonnet-4-5", "claude-sonnet"),
            ("claude-3-5-sonnet-20241022", "claude-sonnet"),
            ("Claude-Opus-4-1", "claude-opus"),
            ("gpt-5", "gpt-frontier"),
            ("gpt-5-mini", "gpt-mini"),
            ("gpt-4o-mini", "gpt-mini"),
            ("gpt-5-codex", "openai-reasoning"),
            ("o4-mini", "openai-reasoning-mini"),
            ("o3", "openai-reasoning"),
            ("gemini-2.5-pro", "gemini-pro"),
            ("gemini-2.5-flash", "gemini-flash"),
            ("grok-4", "grok"),
            ("grok-code-fast-1", "grok-code"),
            ("llama-3.1-8b-instruct", "llama-small"),
            ("qwen3-coder-480b-a35b", "qwen-coder-large"),
            ("qwen2.5-coder-7b-instruct", "qwen-coder-small"),
            ("deepseek-r1", "deepseek-reasoner"),
            ("deepseek-v3", "deepseek-chat"),
            ("tier:mid", "tier:mid"),
            ("claude-haiku", "claude-haiku"),
        ];
        for (name, expected) in cases {
            assert_eq!(registry.find(name).unwrap(), expected, "{name}");
        }
    }

    #[test]
    fn unknown_model_is_an_error() {
        assert!(registry().find("definitely-not-a-model").is_err());
    }

    #[test]
    fn family_inherits_tier_and_applies_overrides() {
        let sonnet = registry().resolve("claude-sonnet").unwrap();
        let frontier = registry().resolve("tier:frontier").unwrap();
        assert_eq!(sonnet.tier, "tier:frontier");
        assert_eq!(sonnet.capability, 3.7);
        assert_eq!(sonnet.tolerance, frontier.tolerance);
    }

    #[test]
    fn user_override_merges_field_by_field() {
        let user = BTreeMap::from([(
            "claude-sonnet".to_string(),
            ProfileSpec {
                tolerance: BTreeMap::from([(Feature::Reasoning, 30.0)]),
                ..Default::default()
            },
        )]);
        let sonnet = Registry::load(&user)
            .unwrap()
            .resolve("claude-sonnet")
            .unwrap();
        assert_eq!(sonnet.tolerance(Feature::Reasoning), 30.0);
        assert_eq!(sonnet.tolerance(Feature::UnitTokens), 1200.0);
        assert_eq!(sonnet.provider, "anthropic");
    }

    #[test]
    fn custom_profile_id_matches_ignoring_case() {
        let user = BTreeMap::from([(
            "MyCoder".to_string(),
            ProfileSpec {
                cost_tier: Some(1),
                ..Default::default()
            },
        )]);
        let registry = Registry::load(&user).unwrap();
        assert_eq!(registry.find("MyCoder").unwrap(), "MyCoder");
        assert_eq!(registry.find("mycoder").unwrap(), "MyCoder");
    }

    #[test]
    fn custom_profile_defaults_to_mid_tier_and_is_findable_by_alias() {
        let user = BTreeMap::from([(
            "my-finetune".to_string(),
            ProfileSpec {
                aliases: Some(vec!["acme-coder-*".to_string()]),
                cost_tier: Some(1),
                ..Default::default()
            },
        )]);
        let registry = Registry::load(&user).unwrap();
        assert_eq!(registry.find("acme-coder-7b").unwrap(), "my-finetune");
        let profile = registry.resolve("my-finetune").unwrap();
        assert_eq!(profile.tier, "tier:mid");
        assert_eq!(profile.cost_tier, 1);
    }

    #[test]
    fn inheritance_cycle_is_an_error() {
        let user = BTreeMap::from([
            (
                "a".to_string(),
                ProfileSpec {
                    inherits: Some("b".to_string()),
                    ..Default::default()
                },
            ),
            (
                "b".to_string(),
                ProfileSpec {
                    inherits: Some("a".to_string()),
                    ..Default::default()
                },
            ),
        ]);
        assert!(Registry::load(&user).unwrap().resolve("a").is_err());
    }

    #[test]
    fn rejects_non_positive_tolerance() {
        let user = BTreeMap::from([(
            "claude-sonnet".to_string(),
            ProfileSpec {
                tolerance: BTreeMap::from([(Feature::Ripple, 0.0)]),
                ..Default::default()
            },
        )]);
        assert!(
            Registry::load(&user)
                .unwrap()
                .resolve("claude-sonnet")
                .is_err()
        );
    }

    #[test]
    fn specificity_counts_literals_and_classes() {
        assert_eq!(specificity("gpt-5*"), 5);
        assert_eq!(specificity("gpt-*-mini*"), 9);
        assert_eq!(specificity("grok-[0-9]*"), 6);
    }
}

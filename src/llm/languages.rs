//! Per-language verbosity and type-checking profiles.

use crate::core::Language;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const BUILTIN: &str = include_str!("languages.toml");
const DEFAULT_KEY: &str = "default";

/// Resolved profile for one language.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LanguageProfile {
    pub tokens_per_line: f64,
    pub verbosity: f64,
    pub type_check_gap: f64,
}

/// User override; unset fields keep the built-in value.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LanguageOverride {
    pub tokens_per_line: Option<f64>,
    pub verbosity: Option<f64>,
    pub type_check_gap: Option<f64>,
}

pub type LanguageProfiles = BTreeMap<String, LanguageProfile>;

/// Profile key for a language. New `Language` variants must be added here.
pub fn language_key(language: Language) -> &'static str {
    match language {
        Language::Rust => "rust",
        Language::Python => "python",
        Language::JavaScript => "javascript",
        Language::TypeScript => "typescript",
        Language::Go => "go",
        Language::Solidity => "solidity",
        Language::Unknown => DEFAULT_KEY,
    }
}

/// Built-in profiles merged with user overrides.
pub fn resolve_languages(
    overrides: &BTreeMap<String, LanguageOverride>,
) -> Result<LanguageProfiles, String> {
    let builtin: LanguageProfiles = toml::from_str(BUILTIN)
        .map_err(|e| format!("built-in language profiles are invalid: {e}"))?;
    let default = *builtin
        .get(DEFAULT_KEY)
        .ok_or("built-in language profiles lack a `default` entry")?;
    let merged: LanguageProfiles = builtin
        .keys()
        .chain(overrides.keys())
        .map(|key| {
            let base = builtin.get(key).copied().unwrap_or(default);
            let profile = overrides.get(key).map(|o| apply(base, o)).unwrap_or(base);
            (key.clone(), profile)
        })
        .collect();
    for (key, profile) in &merged {
        validate(key, profile)?;
    }
    Ok(merged)
}

/// Profile for a language, falling back to `default`.
pub fn profile_for(profiles: &LanguageProfiles, language: Language) -> LanguageProfile {
    profiles
        .get(language_key(language))
        .or_else(|| profiles.get(DEFAULT_KEY))
        .copied()
        .unwrap_or(LanguageProfile {
            tokens_per_line: 9.0,
            verbosity: 1.2,
            type_check_gap: 0.6,
        })
}

fn apply(base: LanguageProfile, o: &LanguageOverride) -> LanguageProfile {
    LanguageProfile {
        tokens_per_line: o.tokens_per_line.unwrap_or(base.tokens_per_line),
        verbosity: o.verbosity.unwrap_or(base.verbosity),
        type_check_gap: o.type_check_gap.unwrap_or(base.type_check_gap),
    }
}

fn validate(key: &str, p: &LanguageProfile) -> Result<(), String> {
    if p.tokens_per_line <= 0.0 || p.verbosity <= 0.0 {
        return Err(format!(
            "llm_editability.languages.{key}: tokens_per_line and verbosity must be > 0"
        ));
    }
    if !(0.0..=1.0).contains(&p.type_check_gap) {
        return Err(format!(
            "llm_editability.languages.{key}.type_check_gap must be between 0 and 1"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANALYZED: [Language; 6] = [
        Language::Rust,
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
        Language::Go,
        Language::Solidity,
    ];

    #[test]
    fn every_analyzed_language_has_an_explicit_profile() {
        let builtin: LanguageProfiles = toml::from_str(BUILTIN).unwrap();
        for language in ANALYZED {
            assert!(
                builtin.contains_key(language_key(language)),
                "missing built-in LLM language profile for {language}"
            );
        }
        assert!(builtin.contains_key(DEFAULT_KEY));
    }

    #[test]
    fn unknown_language_uses_default() {
        let profiles = resolve_languages(&BTreeMap::new()).unwrap();
        assert_eq!(
            profile_for(&profiles, Language::Unknown),
            profiles[DEFAULT_KEY]
        );
    }

    #[test]
    fn go_is_more_verbose_than_python() {
        let profiles = resolve_languages(&BTreeMap::new()).unwrap();
        assert!(
            profile_for(&profiles, Language::Go).verbosity
                > profile_for(&profiles, Language::Python).verbosity
        );
    }

    #[test]
    fn overrides_merge_field_by_field() {
        let overrides = BTreeMap::from([(
            "go".to_string(),
            LanguageOverride {
                verbosity: Some(1.6),
                ..Default::default()
            },
        )]);
        let profiles = resolve_languages(&overrides).unwrap();
        let go = profile_for(&profiles, Language::Go);
        assert_eq!(go.verbosity, 1.6);
        assert_eq!(go.type_check_gap, 0.25);
    }

    #[test]
    fn new_language_override_starts_from_default() {
        let overrides = BTreeMap::from([(
            "kotlin".to_string(),
            LanguageOverride {
                type_check_gap: Some(0.2),
                ..Default::default()
            },
        )]);
        let profiles = resolve_languages(&overrides).unwrap();
        assert_eq!(
            profiles["kotlin"].verbosity,
            profiles[DEFAULT_KEY].verbosity
        );
        assert_eq!(profiles["kotlin"].type_check_gap, 0.2);
    }

    #[test]
    fn rejects_invalid_override() {
        let overrides = BTreeMap::from([(
            "rust".to_string(),
            LanguageOverride {
                verbosity: Some(0.0),
                ..Default::default()
            },
        )]);
        assert!(resolve_languages(&overrides).is_err());
    }
}

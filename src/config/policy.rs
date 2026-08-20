//! Immutable, language-aware analysis policy.

use super::{DebtmapConfig, GeneratedCodeMode, LanguageFeatures};
use crate::core::{FileMetrics, Language};
use crate::priority::DebtType;
use serde::{Deserialize, Serialize};

const SUPPORTED_LANGUAGES: [Language; 6] = [
    Language::Rust,
    Language::Python,
    Language::JavaScript,
    Language::TypeScript,
    Language::Go,
    Language::Solidity,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisFeature {
    Complexity,
    DeadCode,
    Duplication,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveLanguagePolicy {
    pub language: Language,
    pub enabled: bool,
    pub features: LanguageFeatures,
    pub generated_code: GeneratedCodeMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicationPolicy {
    pub minimum_lines: usize,
    pub similarity_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisPolicy {
    pub languages: Vec<EffectiveLanguagePolicy>,
    pub duplication: DuplicationPolicy,
}

impl AnalysisPolicy {
    pub fn from_config(config: &DebtmapConfig) -> Self {
        let enabled = config
            .languages
            .as_ref()
            .map(|languages| languages.enabled.as_slice())
            .unwrap_or_default();
        let languages = SUPPORTED_LANGUAGES
            .into_iter()
            .map(|language| language_policy(config, language, enabled))
            .collect();
        let thresholds = config.thresholds.as_ref();
        Self {
            languages,
            duplication: DuplicationPolicy {
                minimum_lines: thresholds.and_then(|value| value.duplication).unwrap_or(50)
                    as usize,
                similarity_threshold: thresholds
                    .and_then(|value| value.duplication_similarity)
                    .unwrap_or(1.0),
            },
        }
    }

    pub fn for_language(&self, language: Language) -> Option<&EffectiveLanguagePolicy> {
        self.languages
            .iter()
            .find(|policy| policy.language == language)
    }

    pub fn allows(&self, language: Language, feature: AnalysisFeature) -> bool {
        self.for_language(language)
            .is_some_and(|policy| policy.enabled && feature_enabled(&policy.features, feature))
    }

    pub fn allows_debt_type(&self, language: Language, debt_type: &DebtType) -> bool {
        match debt_type {
            DebtType::Complexity { .. }
            | DebtType::ComplexityHotspot { .. }
            | DebtType::TestComplexity { .. }
            | DebtType::TestComplexityHotspot { .. } => {
                self.allows(language, AnalysisFeature::Complexity)
            }
            DebtType::DeadCode { .. } => self.allows(language, AnalysisFeature::DeadCode),
            DebtType::Duplication { .. } | DebtType::TestDuplication { .. } => {
                self.allows(language, AnalysisFeature::Duplication)
            }
            _ => self
                .for_language(language)
                .is_none_or(|policy| policy.enabled),
        }
    }

    pub fn filter_file_metrics(&self, metrics: FileMetrics) -> FileMetrics {
        let fallback_language = metrics.language;
        let debt_items = metrics
            .debt_items
            .into_iter()
            .filter(|item| {
                let detected = Language::from_path(&item.file);
                let language = if detected != Language::Unknown {
                    detected
                } else {
                    fallback_language
                };
                self.allows_debt_type(language, &item.debt_type)
            })
            .collect();
        FileMetrics {
            debt_items,
            ..metrics
        }
    }

    pub fn apply_file_metrics(&self, metrics: FileMetrics, source: &str) -> Option<FileMetrics> {
        let language = metrics.language;
        let generated = crate::analysis::generated_code::is_generated_or_vendor(
            &metrics.path,
            language,
            source,
        );
        self.apply_file_metrics_with_generated(metrics, generated)
    }

    pub fn apply_file_metrics_with_generated(
        &self,
        metrics: FileMetrics,
        generated: bool,
    ) -> Option<FileMetrics> {
        let language = metrics.language;
        let mode = self
            .for_language(language)
            .map(|policy| policy.generated_code)
            .unwrap_or(GeneratedCodeMode::Analyze);
        if generated && mode == GeneratedCodeMode::Exclude {
            return None;
        }
        let mut metrics = self.filter_file_metrics(metrics);
        if generated && mode == GeneratedCodeMode::SuppressDebt {
            metrics.debt_items.clear();
            metrics.duplications.clear();
        }
        Some(metrics)
    }

    pub fn allows_generated_findings(&self, path: &std::path::Path, source: &str) -> bool {
        let language = Language::from_path(path);
        let generated =
            crate::analysis::generated_code::is_generated_or_vendor(path, language, source);
        !generated
            || self
                .for_language(language)
                .is_none_or(|policy| policy.generated_code == GeneratedCodeMode::Analyze)
    }
}

fn feature_enabled(features: &LanguageFeatures, feature: AnalysisFeature) -> bool {
    match feature {
        AnalysisFeature::Complexity => features.detect_complexity,
        AnalysisFeature::DeadCode => features.detect_dead_code,
        AnalysisFeature::Duplication => features.detect_duplication,
    }
}

fn language_policy(
    config: &DebtmapConfig,
    language: Language,
    enabled: &[String],
) -> EffectiveLanguagePolicy {
    let languages = config.languages.as_ref();
    let (features, generated_code) = match language {
        Language::Rust => languages
            .and_then(|value| value.rust.clone())
            .map(|value| (value.features, value.generated_code))
            .unwrap_or_default(),
        Language::Python => languages
            .and_then(|value| value.python.clone())
            .map(|value| (value.features, value.generated_code))
            .unwrap_or_default(),
        Language::JavaScript => languages
            .and_then(|value| value.javascript.clone())
            .map(|value| (value.features, value.generated_code))
            .unwrap_or_default(),
        Language::TypeScript => languages
            .and_then(|value| value.typescript.clone())
            .map(|value| (value.features, value.generated_code))
            .unwrap_or_default(),
        Language::Go => languages
            .and_then(|value| value.go.clone())
            .map(|value| (value.features, value.generated_code))
            .unwrap_or_default(),
        Language::Solidity => languages
            .and_then(|value| value.solidity.clone())
            .map(|value| (value.features, value.vendor_code))
            .unwrap_or_default(),
        Language::Unknown => (LanguageFeatures::default(), GeneratedCodeMode::Analyze),
    };
    EffectiveLanguagePolicy {
        language,
        enabled: enabled.is_empty()
            || enabled
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&language.to_string())),
        features,
        generated_code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        GoLanguageConfig, LanguagesConfig, SolidityLanguageConfig, ThresholdsConfig,
    };

    fn features(complexity: bool, dead_code: bool, duplication: bool) -> LanguageFeatures {
        LanguageFeatures {
            detect_complexity: complexity,
            detect_dead_code: dead_code,
            detect_duplication: duplication,
        }
    }

    fn standard(features: LanguageFeatures) -> crate::config::StandardLanguageConfig {
        crate::config::StandardLanguageConfig {
            features,
            ..Default::default()
        }
    }

    fn feature_matrix_policy() -> AnalysisPolicy {
        AnalysisPolicy::from_config(&DebtmapConfig {
            languages: Some(LanguagesConfig {
                rust: Some(standard(features(true, false, false))),
                python: Some(standard(features(false, true, false))),
                javascript: Some(standard(features(false, false, true))),
                typescript: Some(standard(features(true, true, false))),
                go: Some(GoLanguageConfig {
                    features: features(true, false, true),
                    ..GoLanguageConfig::default()
                }),
                solidity: Some(SolidityLanguageConfig {
                    features: features(false, true, true),
                    ..SolidityLanguageConfig::default()
                }),
                ..LanguagesConfig::default()
            }),
            ..DebtmapConfig::default()
        })
    }

    #[test]
    fn resolves_feature_matrix_for_all_supported_languages() {
        let policy = feature_matrix_policy();
        let expected = [
            (Language::Rust, [true, false, false]),
            (Language::Python, [false, true, false]),
            (Language::JavaScript, [false, false, true]),
            (Language::TypeScript, [true, true, false]),
            (Language::Go, [true, false, true]),
            (Language::Solidity, [false, true, true]),
        ];
        let features = [
            AnalysisFeature::Complexity,
            AnalysisFeature::DeadCode,
            AnalysisFeature::Duplication,
        ];

        for (language, decisions) in expected {
            for (feature, expected) in features.into_iter().zip(decisions) {
                assert_eq!(policy.allows(language, feature), expected);
            }
        }
    }

    #[test]
    fn javascript_and_typescript_have_independent_features() {
        let config = DebtmapConfig {
            languages: Some(LanguagesConfig {
                javascript: Some(standard(LanguageFeatures {
                    detect_complexity: false,
                    ..LanguageFeatures::default()
                })),
                typescript: Some(standard(LanguageFeatures {
                    detect_complexity: true,
                    ..LanguageFeatures::default()
                })),
                ..LanguagesConfig::default()
            }),
            ..DebtmapConfig::default()
        };
        let policy = AnalysisPolicy::from_config(&config);

        assert!(!policy.allows(Language::JavaScript, AnalysisFeature::Complexity));
        assert!(policy.allows(Language::TypeScript, AnalysisFeature::Complexity));
    }

    #[test]
    fn standard_language_generated_modes_round_trip_from_toml() {
        let config: DebtmapConfig = toml::from_str(
            r#"
[languages.rust]
generated_code = "exclude"

[languages.python]
generated_code = "suppress_debt"

[languages.javascript]
generated_code = "analyze"

[languages.typescript]
generated_code = "exclude"
"#,
        )
        .unwrap();
        let policy = AnalysisPolicy::from_config(&config);

        let expected = [
            (Language::Rust, GeneratedCodeMode::Exclude),
            (Language::Python, GeneratedCodeMode::SuppressDebt),
            (Language::JavaScript, GeneratedCodeMode::Analyze),
            (Language::TypeScript, GeneratedCodeMode::Exclude),
        ];
        for (language, mode) in expected {
            assert_eq!(policy.for_language(language).unwrap().generated_code, mode);
        }
        let serialized = toml::to_string(&config).unwrap();
        let reparsed: DebtmapConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(
            AnalysisPolicy::from_config(&reparsed).languages,
            policy.languages
        );
    }

    #[test]
    fn generated_suppression_applies_to_all_six_languages() {
        let config: DebtmapConfig = toml::from_str(
            r#"
[languages.rust]
generated_code = "suppress_debt"
[languages.python]
generated_code = "suppress_debt"
[languages.javascript]
generated_code = "suppress_debt"
[languages.typescript]
generated_code = "suppress_debt"
[languages.go]
generated_code = "suppress_debt"
[languages.solidity]
vendor_code = "suppress_debt"
"#,
        )
        .unwrap();
        let policy = AnalysisPolicy::from_config(&config);
        let fixtures = [
            ("generated/model.rs", Language::Rust, "// generated"),
            ("generated/model.py", Language::Python, "# generated"),
            ("generated/model.js", Language::JavaScript, "// generated"),
            ("generated/model.ts", Language::TypeScript, "// generated"),
            (
                "service.pb.go",
                Language::Go,
                "// Code generated by protoc. DO NOT EDIT.\npackage api",
            ),
            (
                "Generated.sol",
                Language::Solidity,
                "// Generated. DO NOT EDIT\npragma solidity 0.8.20;",
            ),
        ];

        for (path, language, source) in fixtures {
            let filtered = policy
                .apply_file_metrics(file_metrics(path, language), source)
                .unwrap();
            assert!(filtered.debt_items.is_empty(), "language: {language}");
        }
    }

    fn file_metrics(path: &str, language: Language) -> FileMetrics {
        let path = std::path::PathBuf::from(path);
        FileMetrics {
            path: path.clone(),
            language,
            complexity: Default::default(),
            debt_items: vec![crate::core::DebtItem {
                id: "todo".into(),
                debt_type: DebtType::Todo { reason: None },
                priority: crate::core::Priority::Low,
                file: path,
                line: 1,
                column: None,
                message: "todo".into(),
                context: None,
            }],
            dependencies: Vec::new(),
            duplications: Vec::new(),
            total_lines: 1,
            test_lines: 0,
            module_scope: None,
            classes: None,
        }
    }

    #[test]
    fn disabled_go_complexity_is_rejected_at_finding_boundary() {
        let config = DebtmapConfig {
            languages: Some(LanguagesConfig {
                go: Some(crate::config::GoLanguageConfig {
                    features: LanguageFeatures {
                        detect_complexity: false,
                        ..LanguageFeatures::default()
                    },
                    generated_code: GeneratedCodeMode::SuppressDebt,
                }),
                ..LanguagesConfig::default()
            }),
            ..DebtmapConfig::default()
        };
        let policy = AnalysisPolicy::from_config(&config);
        let debt = DebtType::Complexity {
            cyclomatic: 20,
            cognitive: 20,
        };

        assert!(!policy.allows_debt_type(Language::Go, &debt));
    }

    #[test]
    fn file_policy_removes_disabled_debt_and_preserves_other_findings() {
        let policy = AnalysisPolicy::from_config(&DebtmapConfig {
            languages: Some(LanguagesConfig {
                go: Some(GoLanguageConfig {
                    features: features(false, true, true),
                    ..GoLanguageConfig::default()
                }),
                ..LanguagesConfig::default()
            }),
            ..DebtmapConfig::default()
        });
        let path = std::path::PathBuf::from("main.go");
        let debt_items = vec![
            crate::core::DebtItem {
                id: "complexity".into(),
                debt_type: DebtType::Complexity {
                    cyclomatic: 20,
                    cognitive: 20,
                },
                priority: crate::core::Priority::High,
                file: path.clone(),
                line: 1,
                column: None,
                message: "complex".into(),
                context: None,
            },
            crate::core::DebtItem {
                id: "todo".into(),
                debt_type: DebtType::Todo { reason: None },
                priority: crate::core::Priority::Low,
                file: path.clone(),
                line: 2,
                column: None,
                message: "todo".into(),
                context: None,
            },
        ];
        let metrics = FileMetrics {
            path,
            language: Language::Go,
            complexity: Default::default(),
            debt_items,
            dependencies: Vec::new(),
            duplications: Vec::new(),
            total_lines: 2,
            test_lines: 0,
            module_scope: None,
            classes: None,
        };

        let filtered = policy.filter_file_metrics(metrics);

        assert_eq!(filtered.debt_items.len(), 1);
        assert!(matches!(
            filtered.debt_items[0].debt_type,
            DebtType::Todo { .. }
        ));
    }

    #[test]
    fn duplication_settings_are_resolved_once() {
        let config = DebtmapConfig {
            thresholds: Some(ThresholdsConfig {
                duplication: Some(12),
                duplication_similarity: Some(0.8),
                ..ThresholdsConfig::default()
            }),
            ..DebtmapConfig::default()
        };

        assert_eq!(
            AnalysisPolicy::from_config(&config).duplication,
            DuplicationPolicy {
                minimum_lines: 12,
                similarity_threshold: 0.8,
            }
        );
    }
}

//! Strict diagnostics for the user-facing configuration contract.

use super::DebtmapConfig;
use super::validation::validate_config;
use stillwater::Validation;

mod schema;
use schema::{GO_KEYS, OUTPUT_FORMAT_VALUES, SOLIDITY_KEYS, suggest_key};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigCheckDiagnostic {
    pub path: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

impl ConfigCheckDiagnostic {
    fn parse(message: String) -> Self {
        Self {
            path: None,
            message,
            suggestion: None,
        }
    }

    fn unknown(path: String) -> Self {
        let suggestion = suggest_key(&path);
        Self {
            message: format!("unknown configuration key `{path}`"),
            path: Some(path),
            suggestion,
        }
    }

    pub(crate) fn render(&self) -> String {
        match &self.suggestion {
            Some(suggestion) => format!("{}; did you mean `{suggestion}`?", self.message),
            None => self.message.clone(),
        }
    }
}

pub(crate) fn check_config_contents(
    contents: &str,
) -> Result<DebtmapConfig, Vec<ConfigCheckDiagnostic>> {
    let mut unknown_paths = Vec::new();
    let deserializer = toml::de::Deserializer::parse(contents)
        .map_err(|error| vec![ConfigCheckDiagnostic::parse(error.to_string())])?;
    let config: DebtmapConfig = serde_ignored::deserialize(deserializer, |path| {
        unknown_paths.push(normalize_ignored_path(&path.to_string()));
    })
    .map_err(|error| vec![ConfigCheckDiagnostic::parse(error.to_string())])?;
    collect_flattened_language_unknowns(contents, &mut unknown_paths)?;

    unknown_paths.sort();
    unknown_paths.dedup();
    let mut diagnostics: Vec<_> = unknown_paths
        .into_iter()
        .map(ConfigCheckDiagnostic::unknown)
        .collect();
    if let Validation::Failure(errors) = validate_config(&config) {
        diagnostics.extend(
            errors
                .into_iter()
                .map(|error| ConfigCheckDiagnostic::parse(error.to_string())),
        );
    }
    diagnostics.extend(validate_contract_values(&config));

    if diagnostics.is_empty() {
        Ok(config)
    } else {
        Err(diagnostics)
    }
}

fn collect_flattened_language_unknowns(
    contents: &str,
    unknown_paths: &mut Vec<String>,
) -> Result<(), Vec<ConfigCheckDiagnostic>> {
    let document = toml::from_str::<toml::Value>(contents)
        .map_err(|error| vec![ConfigCheckDiagnostic::parse(error.to_string())])?;
    collect_unknown_table_keys(
        &document,
        &["languages", "go"],
        "languages.go",
        GO_KEYS,
        unknown_paths,
    );
    collect_unknown_table_keys(
        &document,
        &["languages", "solidity"],
        "languages.solidity",
        SOLIDITY_KEYS,
        unknown_paths,
    );
    Ok(())
}

fn collect_unknown_table_keys(
    document: &toml::Value,
    segments: &[&str],
    parent: &str,
    candidates: &[&str],
    unknown_paths: &mut Vec<String>,
) {
    let table = segments
        .iter()
        .try_fold(document, |value, segment| value.get(*segment))
        .and_then(toml::Value::as_table);
    if let Some(table) = table {
        unknown_paths.extend(
            table
                .keys()
                .filter(|key| !candidates.contains(&key.as_str()))
                .map(|key| format!("{parent}.{key}")),
        );
    }
}

fn validate_contract_values(config: &DebtmapConfig) -> Vec<ConfigCheckDiagnostic> {
    let mut diagnostics = Vec::new();
    if let Some(risk) = config
        .thresholds
        .as_ref()
        .and_then(|thresholds| thresholds.minimum_risk_score)
        && !(0.0..=10.0).contains(&risk)
    {
        diagnostics.push(value_error(
            "thresholds.minimum_risk_score",
            format!("must be between 0 and 10, got {risk}"),
        ));
    }
    if let Some(output) = config.output.as_ref() {
        if let Some(confidence) = output.min_confidence_warning
            && !(0.0..=1.0).contains(&confidence)
        {
            diagnostics.push(value_error(
                "output.min_confidence_warning",
                format!("must be between 0 and 1, got {confidence}"),
            ));
        }
        validate_optional_enum(
            "output.detail_level",
            output.detail_level.as_deref(),
            &["summary", "standard", "comprehensive", "debug"],
            &mut diagnostics,
        );
        validate_optional_enum(
            "output.default_format",
            output.default_format.as_deref(),
            OUTPUT_FORMAT_VALUES,
            &mut diagnostics,
        );
        validate_optional_enum(
            "output.format",
            output.format.as_deref(),
            OUTPUT_FORMAT_VALUES,
            &mut diagnostics,
        );
    }
    if let Some(entropy) = config.entropy.as_ref() {
        let values = [
            ("weight", entropy.weight),
            ("pattern_threshold", entropy.pattern_threshold),
            ("entropy_threshold", entropy.entropy_threshold),
            ("branch_threshold", entropy.branch_threshold),
            ("max_repetition_reduction", entropy.max_repetition_reduction),
            ("max_entropy_reduction", entropy.max_entropy_reduction),
            ("max_branch_reduction", entropy.max_branch_reduction),
            ("max_combined_reduction", entropy.max_combined_reduction),
        ];
        diagnostics.extend(
            values
                .into_iter()
                .filter(|(_, value)| !(0.0..=1.0).contains(value))
                .map(|(field, value)| {
                    value_error(
                        &format!("entropy.{field}"),
                        format!("must be between 0 and 1, got {value}"),
                    )
                }),
        );
    }
    diagnostics
}

fn value_error(path: &str, message: String) -> ConfigCheckDiagnostic {
    ConfigCheckDiagnostic {
        path: Some(path.to_string()),
        message: format!("invalid value for `{path}`: {message}"),
        suggestion: None,
    }
}

fn validate_optional_enum(
    path: &str,
    value: Option<&str>,
    allowed: &[&str],
    diagnostics: &mut Vec<ConfigCheckDiagnostic>,
) {
    if let Some(value) = value
        && !allowed.contains(&value)
    {
        diagnostics.push(value_error(
            path,
            format!("expected one of {}, got `{value}`", allowed.join(", ")),
        ));
    }
}

fn normalize_ignored_path(path: &str) -> String {
    path.split('.')
        .filter(|segment| *segment != "?")
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_configuration() {
        let contents = r#"
[thresholds]
complexity = 10

[thresholds.validation]
max_debt_density = 50.0
"#;

        let result = check_config_contents(contents);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn accumulates_unknown_keys_with_scoped_suggestions() {
        let contents = r#"
threshold = 10

[thresholds]
complexty = 10
"#;

        let diagnostics = check_config_contents(contents).unwrap_err();
        let rendered: Vec<_> = diagnostics.iter().map(|item| item.render()).collect();
        assert_eq!(rendered.len(), 2);
        assert!(rendered[0].contains("`threshold`"));
        assert!(rendered[0].contains("`thresholds`"));
        assert!(
            rendered[1].contains("`thresholds.complexty`"),
            "{rendered:?}"
        );
        assert!(rendered[1].contains("`thresholds.complexity`"));
    }

    #[test]
    fn combines_unknown_key_and_value_diagnostics() {
        let contents = r#"
[thresholds]
complexty = 10
complexity = 0
"#;

        let diagnostics = check_config_contents(contents).unwrap_err();
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().any(|item| item.path.is_some()));
        assert!(
            diagnostics
                .iter()
                .any(|item| item.message.contains("cannot be zero"))
        );
    }

    #[test]
    fn rejects_type_errors() {
        let diagnostics = check_config_contents("[thresholds]\ncomplexity = \"high\"").unwrap_err();
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("invalid type"));
    }

    #[test]
    fn catches_unknown_flattened_language_keys() {
        let contents = r#"
[languages]
enabled = ["go", "solidity"]

[languages.go]
detect_dead_cod = true
generated_cod = "exclude"

[languages.solidity]
detect_complexit = true
vendor_cod = "exclude"
"#;

        let diagnostics = check_config_contents(contents).unwrap_err();
        let paths: Vec<_> = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.path.as_deref())
            .collect();
        assert!(
            paths.contains(&"languages.go.detect_dead_cod"),
            "{diagnostics:?}"
        );
        assert!(paths.contains(&"languages.go.generated_cod"));
        assert!(paths.contains(&"languages.solidity.detect_complexit"));
        assert!(paths.contains(&"languages.solidity.vendor_cod"));
    }

    #[test]
    fn suggests_keys_in_common_scopes() {
        let cases = [
            ("[scoring]\ncoverge = 0.5", "scoring.coverage"),
            ("[display]\ntired = true", "display.tiered"),
            ("[ignore]\npatterns = []\npattrens = []", "ignore.patterns"),
            (
                "[external_api]\ndetect_externl_api = true",
                "external_api.detect_external_api",
            ),
            (
                "[god_object_detection.rust]\nmax_methds = 20",
                "god_object_detection.rust.max_methods",
            ),
        ];

        for (contents, expected) in cases {
            let diagnostics = check_config_contents(contents).unwrap_err();
            assert_eq!(
                diagnostics[0].suggestion.as_deref(),
                Some(expected),
                "{contents}: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn rejects_documented_out_of_range_and_enum_values() {
        let contents = r#"
[thresholds]
minimum_risk_score = 11.0

[output]
min_confidence_warning = 1.5
format = "bogus"
detail_level = "everything"
"#;

        let diagnostics = check_config_contents(contents).unwrap_err();
        assert_eq!(diagnostics.len(), 4);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message.starts_with("invalid value"))
        );
    }

    #[test]
    fn rejects_out_of_range_validation_and_entropy_values() {
        let contents = r#"
[thresholds.validation]
max_codebase_risk_score = 11.0

[entropy]
weight = 2.0
pattern_threshold = -0.1
entropy_threshold = 1.1
"#;

        let diagnostics = check_config_contents(contents).unwrap_err();
        assert_eq!(diagnostics.len(), 4);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("risk score must be 0-10"))
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.message.contains("entropy."))
                .count(),
            3
        );
    }

    #[test]
    fn rejects_nan_risk_threshold() {
        let diagnostics =
            check_config_contents("[thresholds.validation]\nmax_codebase_risk_score = nan")
                .unwrap_err();

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("risk score must be 0-10"));
    }
}

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// Sub-modules
mod classification;
mod detection;
mod scoring;
mod thresholds;
mod languages;
mod display;

// Re-export scoring types for backward compatibility
pub use scoring::{
    default_cognitive_weight, default_complexity_weight, default_coverage_weight,
    default_cyclomatic_weight, default_debug_coverage_weight, default_debug_multiplier,
    default_dependency_weight, default_enable_role_clamping, default_entry_point_coverage_weight,
    default_entry_point_multiplier, default_io_wrapper_coverage_weight,
    default_io_wrapper_multiplier, default_linear_threshold, default_log_multiplier,
    default_logarithmic_threshold, default_max_cognitive, default_max_cyclomatic,
    default_orchestrator_coverage_weight, default_orchestrator_multiplier,
    default_organization_weight, default_pattern_match_coverage_weight,
    default_pattern_match_multiplier, default_pure_logic_coverage_weight,
    default_pure_logic_multiplier, default_role_clamp_max, default_role_clamp_min,
    default_security_weight, default_semantic_weight, default_show_raw_scores,
    default_sqrt_multiplier, default_unknown_coverage_weight, default_unknown_multiplier,
    ComplexityWeightsConfig, NormalizationConfig, RebalancedScoringConfig, RoleCoverageWeights,
    RoleMultiplierConfig, RoleMultipliers, ScoringWeights,
};

// Re-export threshold types for backward compatibility
pub use thresholds::{
    FileSizeThresholds, GodObjectThresholds, ThresholdsConfig, ValidationThresholds,
};

// Re-export detection types for backward compatibility
pub use detection::{
    AccessorDetectionConfig, ConstructorDetectionConfig, DataFlowClassificationConfig,
    ErrorHandlingConfig, ErrorPatternConfig, OrchestratorDetectionConfig, SeverityOverride,
};

// Re-export classification types for backward compatibility
pub use classification::{
    CallerCalleeConfig, ClassificationConfig, ContextConfig, ContextMatcherConfig,
    ContextRuleConfig, FunctionPatternConfig,
};

// Re-export language types for backward compatibility
pub use languages::{EntropyConfig, LanguageFeatures, LanguagesConfig};

// Re-export display types for backward compatibility
pub use display::{DisplayConfig, GodObjectConfig, VerbosityLevel};

// Pure mapping pattern detection config (spec 118)
pub use crate::complexity::pure_mapping_patterns::MappingPatternConfig;

/// Root configuration structure for debtmap
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebtmapConfig {
    /// Scoring weights configuration
    #[serde(default)]
    pub scoring: Option<ScoringWeights>,

    /// Display configuration for output formatting
    #[serde(default)]
    pub display: Option<DisplayConfig>,

    /// External API detection configuration
    #[serde(default)]
    pub external_api: Option<crate::priority::external_api_detector::ExternalApiConfig>,

    /// God object detection configuration
    #[serde(default)]
    pub god_object_detection: Option<GodObjectConfig>,

    /// Thresholds configuration
    #[serde(default)]
    pub thresholds: Option<ThresholdsConfig>,

    /// Language configuration
    #[serde(default)]
    pub languages: Option<LanguagesConfig>,

    /// Ignore patterns
    #[serde(default)]
    pub ignore: Option<IgnoreConfig>,

    /// Output configuration
    #[serde(default)]
    pub output: Option<OutputConfig>,

    /// Context-aware detection configuration
    #[serde(default)]
    pub context: Option<ContextConfig>,

    /// Entropy-based complexity scoring configuration
    #[serde(default)]
    pub entropy: Option<EntropyConfig>,

    /// Role multipliers for semantic classification
    #[serde(default)]
    pub role_multipliers: Option<RoleMultipliers>,

    /// Complexity thresholds for enhanced detection
    #[serde(default)]
    pub complexity_thresholds: Option<crate::complexity::threshold_manager::ComplexityThresholds>,

    /// Error handling detection configuration
    #[serde(default)]
    pub error_handling: Option<ErrorHandlingConfig>,

    /// Score normalization configuration
    #[serde(default)]
    pub normalization: Option<NormalizationConfig>,

    /// Lines of code counting configuration
    #[serde(default)]
    pub loc: Option<crate::metrics::LocCountingConfig>,

    /// Tier configuration for prioritization
    #[serde(default)]
    pub tiers: Option<crate::priority::TierConfig>,

    /// Role-based coverage weight multipliers
    #[serde(default)]
    pub role_coverage_weights: Option<RoleCoverageWeights>,

    /// Role multiplier clamping configuration (spec 119)
    #[serde(default)]
    pub role_multiplier_config: Option<RoleMultiplierConfig>,

    /// Orchestrator detection configuration
    #[serde(default)]
    pub orchestrator_detection: Option<OrchestratorDetectionConfig>,

    /// Orchestration score adjustment configuration (spec 110)
    #[serde(default)]
    pub orchestration_adjustment:
        Option<crate::priority::scoring::orchestration_adjustment::OrchestrationAdjustmentConfig>,

    /// Constructor detection configuration (spec 117)
    #[serde(default, rename = "classification")]
    pub classification: Option<ClassificationConfig>,

    /// Pure mapping pattern detection configuration (spec 118)
    #[serde(default)]
    pub mapping_patterns: Option<MappingPatternConfig>,

    /// Role-based coverage expectations (spec 119)
    #[serde(default)]
    pub coverage_expectations: Option<crate::priority::scoring::CoverageExpectations>,

    /// Complexity weights configuration (spec 121)
    #[serde(default)]
    pub complexity_weights: Option<ComplexityWeightsConfig>,

    /// AST-based functional pattern analysis configuration (spec 111)
    #[serde(default)]
    pub functional_analysis: Option<crate::analysis::FunctionalAnalysisConfig>,

    /// Boilerplate detection configuration (spec 131)
    #[serde(default)]
    pub boilerplate_detection:
        Option<crate::organization::boilerplate_detector::BoilerplateDetectionConfig>,

    /// Rebalanced scoring configuration (spec 136)
    #[serde(default, rename = "scoring_rebalanced")]
    pub scoring_rebalanced: Option<RebalancedScoringConfig>,
}

impl DebtmapConfig {
    /// Get ignore patterns from configuration
    ///
    /// Returns a vector of glob patterns that should be excluded from analysis.
    /// If no configuration is found or no patterns are specified, returns an empty vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::config::DebtmapConfig;
    /// let config = DebtmapConfig::default();
    /// let patterns = config.get_ignore_patterns();
    /// // patterns might contain ["tests/**/*", "*.test.rs"]
    /// ```
    pub fn get_ignore_patterns(&self) -> Vec<String> {
        self.ignore
            .as_ref()
            .map(|ig| ig.patterns.clone())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreConfig {
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub default_format: Option<String>,
    /// Enable colored output (default: auto-detect based on TTY)
    #[serde(default)]
    pub use_color: Option<bool>,
}

/// Cache the configuration
static CONFIG: OnceLock<DebtmapConfig> = OnceLock::new();
static SCORING_WEIGHTS: OnceLock<ScoringWeights> = OnceLock::new();

/// Load configuration from .debtmap.toml if it exists
/// Pure function to read and parse config file contents
fn read_config_file(path: &Path) -> Result<String, std::io::Error> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Pure function to parse and validate config from TOML string
#[cfg(test)]
pub(crate) fn parse_and_validate_config(contents: &str) -> Result<DebtmapConfig, String> {
    parse_and_validate_config_impl(contents)
}

fn parse_and_validate_config_impl(contents: &str) -> Result<DebtmapConfig, String> {
    let mut config = toml::from_str::<DebtmapConfig>(contents)
        .map_err(|e| format!("Failed to parse .debtmap.toml: {}", e))?;

    // Validate and normalize scoring weights if present
    if let Some(ref mut scoring) = config.scoring {
        if let Err(e) = scoring.validate() {
            eprintln!("Warning: Invalid scoring weights: {}. Using defaults.", e);
            config.scoring = Some(ScoringWeights::default());
        } else {
            scoring.normalize(); // Ensure exact sum of 1.0
        }
    }

    Ok(config)
}

/// Pure function to try loading config from a specific path
fn try_load_config_from_path(config_path: &Path) -> Option<DebtmapConfig> {
    let contents = match read_config_file(config_path) {
        Ok(contents) => contents,
        Err(e) => {
            handle_read_error(config_path, &e);
            return None;
        }
    };

    match parse_and_validate_config_impl(&contents) {
        Ok(config) => {
            log::debug!("Loaded config from {}", config_path.display());
            Some(config)
        }
        Err(e) => {
            eprintln!("Warning: {}. Using defaults.", e);
            None
        }
    }
}

/// Handle file read errors with appropriate logging
fn handle_read_error(config_path: &Path, error: &std::io::Error) {
    // Only log actual errors, not "file not found"
    if error.kind() != std::io::ErrorKind::NotFound {
        log::warn!(
            "Failed to read config file {}: {}",
            config_path.display(),
            error
        );
    }
}

/// Pure function to generate directory ancestors up to a depth limit
#[cfg(test)]
pub(crate) fn directory_ancestors(
    start: PathBuf,
    max_depth: usize,
) -> impl Iterator<Item = PathBuf> {
    directory_ancestors_impl(start, max_depth)
}

fn directory_ancestors_impl(start: PathBuf, max_depth: usize) -> impl Iterator<Item = PathBuf> {
    std::iter::successors(Some(start), |dir| {
        let mut parent = dir.clone();
        if parent.pop() {
            Some(parent)
        } else {
            None
        }
    })
    .take(max_depth)
}

pub fn load_config() -> DebtmapConfig {
    const MAX_TRAVERSAL_DEPTH: usize = 10;

    // Get current directory or return default
    let current = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!(
                "Failed to get current directory: {}. Using default config.",
                e
            );
            return DebtmapConfig::default();
        }
    };

    // Search for config file in directory hierarchy
    directory_ancestors_impl(current, MAX_TRAVERSAL_DEPTH)
        .map(|dir| dir.join(".debtmap.toml"))
        .find_map(|path| try_load_config_from_path(&path))
        .unwrap_or_else(|| {
            log::debug!(
                "No config found after checking {} directories. Using default config.",
                MAX_TRAVERSAL_DEPTH
            );
            DebtmapConfig::default()
        })
}

/// Get the cached configuration
pub fn get_config() -> &'static DebtmapConfig {
    CONFIG.get_or_init(load_config)
}

/// Get the configuration without panicking on errors
pub fn get_config_safe() -> Result<DebtmapConfig, std::io::Error> {
    Ok(load_config())
}

/// Get the scoring weights (with defaults if not configured)
pub fn get_scoring_weights() -> &'static ScoringWeights {
    SCORING_WEIGHTS.get_or_init(|| get_config().scoring.clone().unwrap_or_default())
}

/// Get entropy-based complexity scoring configuration
pub fn get_entropy_config() -> EntropyConfig {
    get_config().entropy.clone().unwrap_or_default()
}

/// Get role multipliers configuration
pub fn get_role_multipliers() -> RoleMultipliers {
    get_config().role_multipliers.clone().unwrap_or_default()
}

/// Get minimum debt score threshold (default: 1.0)
pub fn get_minimum_debt_score() -> f64 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_debt_score)
        .unwrap_or(1.0)
}

/// Get minimum cyclomatic complexity threshold (default: 2)
pub fn get_minimum_cyclomatic_complexity() -> u32 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_cyclomatic_complexity)
        .unwrap_or(2)
}

/// Get minimum cognitive complexity threshold (default: 3)
pub fn get_minimum_cognitive_complexity() -> u32 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_cognitive_complexity)
        .unwrap_or(3)
}

/// Get minimum risk score threshold (default: 1.0)
pub fn get_minimum_risk_score() -> f64 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_risk_score)
        .unwrap_or(1.0)
}

/// Get display configuration (with defaults)
pub fn get_display_config() -> DisplayConfig {
    get_config().display.clone().unwrap_or_default()
}

/// Get validation thresholds (with defaults)
pub fn get_validation_thresholds() -> ValidationThresholds {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.validation.clone())
        .unwrap_or_default()
}

/// Get language-specific feature configuration
pub fn get_language_features(language: &crate::core::Language) -> LanguageFeatures {
    use crate::core::Language;

    let config = get_config();
    let languages_config = config.languages.as_ref();

    match language {
        Language::Rust => {
            languages_config
                .and_then(|lc| lc.rust.clone())
                .unwrap_or(LanguageFeatures {
                    detect_dead_code: false, // Rust dead code detection disabled by default
                    detect_complexity: true,
                    detect_duplication: true,
                })
        }
        Language::Python => languages_config
            .and_then(|lc| lc.python.clone())
            .unwrap_or_default(),
        Language::JavaScript => languages_config
            .and_then(|lc| lc.javascript.clone())
            .unwrap_or_default(),
        Language::TypeScript => languages_config
            .and_then(|lc| lc.typescript.clone())
            .unwrap_or_default(),
        Language::Unknown => LanguageFeatures::default(),
    }
}

/// Get complexity thresholds configuration
pub fn get_complexity_thresholds() -> crate::complexity::threshold_manager::ComplexityThresholds {
    get_config()
        .complexity_thresholds
        .clone()
        .unwrap_or_default()
}

/// Get error handling configuration
pub fn get_error_handling_config() -> ErrorHandlingConfig {
    get_config().error_handling.clone().unwrap_or_default()
}

/// Get role-based coverage weight multipliers
pub fn get_role_coverage_weights() -> RoleCoverageWeights {
    get_config()
        .role_coverage_weights
        .clone()
        .unwrap_or_default()
}

/// Get role-based coverage expectations (spec 119)
pub fn get_coverage_expectations() -> crate::priority::scoring::CoverageExpectations {
    get_config()
        .coverage_expectations
        .clone()
        .unwrap_or_default()
}

/// Get role multiplier clamping configuration (spec 119)
pub fn get_role_multiplier_config() -> RoleMultiplierConfig {
    get_config()
        .role_multiplier_config
        .clone()
        .unwrap_or_default()
}

/// Get orchestrator detection configuration
pub fn get_orchestrator_detection_config() -> OrchestratorDetectionConfig {
    get_config()
        .orchestrator_detection
        .clone()
        .unwrap_or_default()
}

/// Get orchestration adjustment configuration (spec 110)
pub fn get_orchestration_adjustment_config(
) -> crate::priority::scoring::orchestration_adjustment::OrchestrationAdjustmentConfig {
    get_config()
        .orchestration_adjustment
        .clone()
        .unwrap_or_default()
}

/// Get constructor detection configuration (spec 117)
pub fn get_constructor_detection_config() -> ConstructorDetectionConfig {
    get_config()
        .classification
        .as_ref()
        .and_then(|c| c.constructors.clone())
        .unwrap_or_default()
}

/// Get accessor detection configuration (spec 125)
pub fn get_accessor_detection_config() -> AccessorDetectionConfig {
    get_config()
        .classification
        .as_ref()
        .and_then(|c| c.accessors.clone())
        .unwrap_or_default()
}

/// Get data flow classification configuration (spec 126)
pub fn get_data_flow_classification_config() -> DataFlowClassificationConfig {
    get_config()
        .classification
        .as_ref()
        .and_then(|c| c.data_flow.clone())
        .unwrap_or_default()
}

/// Get functional analysis configuration (spec 111)
pub fn get_functional_analysis_config() -> crate::analysis::FunctionalAnalysisConfig {
    get_config().functional_analysis.clone().unwrap_or_default()
}

/// Get smart performance configuration
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_ignore_patterns_with_patterns() {
        let config = DebtmapConfig {
            ignore: Some(IgnoreConfig {
                patterns: vec![
                    "tests/**/*".to_string(),
                    "*.test.rs".to_string(),
                    "**/fixtures/**".to_string(),
                ],
            }),
            ..Default::default()
        };

        let patterns = config.get_ignore_patterns();
        assert_eq!(patterns.len(), 3);
        assert!(patterns.contains(&"tests/**/*".to_string()));
        assert!(patterns.contains(&"*.test.rs".to_string()));
        assert!(patterns.contains(&"**/fixtures/**".to_string()));
    }

    #[test]
    fn test_get_ignore_patterns_without_config() {
        let config = DebtmapConfig::default();
        let patterns = config.get_ignore_patterns();
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn test_get_ignore_patterns_with_empty_patterns() {
        let config = DebtmapConfig {
            ignore: Some(IgnoreConfig { patterns: vec![] }),
            ..Default::default()
        };

        let patterns = config.get_ignore_patterns();
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn test_parse_and_validate_config_valid_toml() {
        let toml_content = r#"
[context]
critical_paths = ["/src/main.rs"]

[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15
"#;
        let result = super::parse_and_validate_config(toml_content);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.scoring.is_some());
        let scoring = config.scoring.unwrap();
        // Active weights should sum to 1.0
        let active_sum = scoring.coverage + scoring.complexity + scoring.dependency;
        assert!((active_sum - 1.0).abs() < 0.001);
        // Check the values with floating point tolerance
        assert!((scoring.coverage - 0.50).abs() < 0.001);
        assert!((scoring.complexity - 0.35).abs() < 0.001);
        assert!((scoring.dependency - 0.15).abs() < 0.001);
        // Unused weights should be 0
        assert!((scoring.semantic - 0.0).abs() < 0.001);
        assert!((scoring.security - 0.0).abs() < 0.001);
        assert!((scoring.organization - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_and_validate_config_invalid_toml() {
        let toml_content = "invalid toml [[ content";
        let result = super::parse_and_validate_config(toml_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse"));
    }

    #[test]
    fn test_parse_and_validate_config_invalid_weights_replaced_with_defaults() {
        let toml_content = r#"
[scoring]
coverage = 0.5
complexity = 0.5
semantic = 0.5
dependency = 0.5
security = 0.5
organization = 0.5
"#;
        let result = super::parse_and_validate_config(toml_content);
        assert!(result.is_ok());
        let config = result.unwrap();
        let scoring = config.scoring.unwrap();
        // Invalid weights (sum > 1.0) should be replaced with defaults
        assert_eq!(scoring.coverage, 0.50);
        assert_eq!(scoring.complexity, 0.35);
        assert_eq!(scoring.semantic, 0.00);
        assert_eq!(scoring.dependency, 0.15);
        assert_eq!(scoring.security, 0.00);
        assert_eq!(scoring.organization, 0.00);
        let active_sum = scoring.coverage + scoring.complexity + scoring.dependency;
        assert!((active_sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_directory_ancestors_generates_correct_sequence() {
        use std::path::PathBuf;

        let start = PathBuf::from("/a/b/c/d");
        let ancestors: Vec<PathBuf> = super::directory_ancestors(start, 3).collect();

        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0], PathBuf::from("/a/b/c/d"));
        assert_eq!(ancestors[1], PathBuf::from("/a/b/c"));
        assert_eq!(ancestors[2], PathBuf::from("/a/b"));
    }

    #[test]
    fn test_directory_ancestors_respects_max_depth() {
        use std::path::PathBuf;

        let start = PathBuf::from("/a/b/c/d/e/f/g/h");
        let ancestors: Vec<PathBuf> = super::directory_ancestors(start, 2).collect();

        assert_eq!(ancestors.len(), 2);
    }

    #[test]
    fn test_directory_ancestors_handles_root() {
        use std::path::PathBuf;

        let start = PathBuf::from("/");
        let ancestors: Vec<PathBuf> = super::directory_ancestors(start, 5).collect();

        // Root directory has no parent, so we only get the root itself
        assert_eq!(ancestors.len(), 1);
        assert_eq!(ancestors[0], PathBuf::from("/"));
    }

    #[test]
    fn test_try_load_config_from_path_with_valid_config() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("debtmap.toml");

        // Write a valid config file
        fs::write(
            &config_path,
            r#"
[thresholds]
complexity = 15
max_file_length = 1000

[scoring]
complexity_weight = 0.4
coverage_weight = 0.3
inheritance_weight = 0.15
interface_weight = 0.15
"#,
        )
        .unwrap();

        let result = try_load_config_from_path(&config_path);
        assert!(result.is_some());

        let config = result.unwrap();
        assert_eq!(config.thresholds.as_ref().unwrap().complexity, Some(15));
        assert_eq!(
            config.thresholds.as_ref().unwrap().max_file_length,
            Some(1000)
        );
    }

    #[test]
    fn test_try_load_config_from_path_with_invalid_config() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("debtmap.toml");

        // Write an invalid config file
        fs::write(&config_path, "invalid toml content").unwrap();

        let result = try_load_config_from_path(&config_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_try_load_config_from_path_with_nonexistent_file() {
        use std::path::PathBuf;

        let config_path = PathBuf::from("/nonexistent/path/to/config.toml");
        let result = try_load_config_from_path(&config_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_read_error_with_not_found() {
        use std::io;
        use std::path::PathBuf;

        let path = PathBuf::from("/test/path");
        let error = io::Error::new(io::ErrorKind::NotFound, "File not found");

        // This should not panic and should not log a warning for NotFound
        handle_read_error(&path, &error);
    }

    #[test]
    fn test_handle_read_error_with_permission_denied() {
        use std::io;
        use std::path::PathBuf;

        let path = PathBuf::from("/test/path");
        let error = io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied");

        // This should log a warning but not panic
        handle_read_error(&path, &error);
    }

    #[test]
    fn test_get_validation_thresholds_with_defaults() {
        // Test that get_validation_thresholds returns expected values
        // The config might override these, so we test flexible values
        let thresholds = get_validation_thresholds();

        // Primary quality metrics
        assert_eq!(thresholds.max_average_complexity, 10.0);
        assert_eq!(thresholds.max_debt_density, 50.0);
        assert_eq!(thresholds.max_codebase_risk_score, 7.0);
        assert_eq!(thresholds.min_coverage_percentage, 0.0);

        // Safety net - high ceiling
        assert_eq!(thresholds.max_total_debt_score, 10000);

        // Deprecated metrics should be None by default
        #[allow(deprecated)]
        {
            assert_eq!(thresholds.max_high_complexity_count, None);
            assert_eq!(thresholds.max_debt_items, None);
            assert_eq!(thresholds.max_high_risk_functions, None);
        }
    }

    #[test]
    fn test_default_linear_threshold() {
        assert_eq!(default_linear_threshold(), 10.0);
    }

    #[test]
    fn test_default_logarithmic_threshold() {
        assert_eq!(default_logarithmic_threshold(), 100.0);
    }

    #[test]
    fn test_default_sqrt_multiplier() {
        assert_eq!(default_sqrt_multiplier(), 3.33);
    }

    #[test]
    fn test_default_log_multiplier() {
        assert_eq!(default_log_multiplier(), 10.0);
    }

    #[test]
    fn test_default_show_raw_scores() {
        assert!(default_show_raw_scores());
    }

    #[test]
    fn test_normalization_config_default() {
        let config = NormalizationConfig::default();
        assert_eq!(config.linear_threshold, 10.0);
        assert_eq!(config.logarithmic_threshold, 100.0);
        assert_eq!(config.sqrt_multiplier, 3.33);
        assert_eq!(config.log_multiplier, 10.0);
        assert!(config.show_raw_scores);
    }

    #[test]
    fn test_role_multipliers_default() {
        let multipliers = RoleMultipliers::default();
        assert_eq!(multipliers.pure_logic, 1.2);
        assert_eq!(multipliers.orchestrator, 0.8);
        assert_eq!(multipliers.io_wrapper, 0.7);
        assert_eq!(multipliers.entry_point, 0.9);
        assert_eq!(multipliers.pattern_match, 0.6);
        assert_eq!(multipliers.unknown, 1.0);
    }

    #[test]
    fn test_scoring_weights_default() {
        let weights = ScoringWeights::default();
        assert_eq!(weights.coverage, 0.50);
        assert_eq!(weights.complexity, 0.35);
        assert_eq!(weights.semantic, 0.00);
        assert_eq!(weights.dependency, 0.15);
        assert_eq!(weights.security, 0.00);
        assert_eq!(weights.organization, 0.00);
    }

    #[test]
    fn test_scoring_weights_validate_success() {
        let weights = ScoringWeights {
            coverage: 0.50,
            complexity: 0.35,
            semantic: 0.0,
            dependency: 0.15,
            security: 0.0,
            organization: 0.0,
        };
        assert!(weights.validate().is_ok());
    }

    #[test]
    fn test_scoring_weights_validate_invalid_sum() {
        let weights = ScoringWeights {
            coverage: 0.60,
            complexity: 0.60,
            semantic: 0.0,
            dependency: 0.0,
            security: 0.0,
            organization: 0.0,
        };
        assert!(weights.validate().is_err());
    }

    #[test]
    fn test_scoring_weights_normalize() {
        let mut weights = ScoringWeights {
            coverage: 0.40,
            complexity: 0.30,
            semantic: 0.0,
            dependency: 0.10,
            security: 0.0,
            organization: 0.0,
        };
        weights.normalize();
        // After normalization, active weights should sum to 1.0
        let sum = weights.coverage + weights.complexity + weights.dependency;
        assert!((sum - 1.0).abs() < 0.001);
        // Check proportions are maintained
        assert!((weights.coverage - 0.50).abs() < 0.001);
        assert!((weights.complexity - 0.375).abs() < 0.001);
        assert!((weights.dependency - 0.125).abs() < 0.001);
    }

    #[test]
    fn test_entropy_config_default() {
        let config = EntropyConfig::default();
        assert!(config.enabled);
        assert_eq!(config.weight, 1.0);
        assert_eq!(config.min_tokens, 20);
        assert_eq!(config.pattern_threshold, 0.7);
        assert_eq!(config.entropy_threshold, 0.4);
        assert_eq!(config.branch_threshold, 0.8);
        assert_eq!(config.max_repetition_reduction, 0.20);
        assert_eq!(config.max_entropy_reduction, 0.15);
        assert_eq!(config.max_branch_reduction, 0.25);
        assert_eq!(config.max_combined_reduction, 0.30);
    }

    #[test]
    fn test_error_handling_config_default() {
        let config = ErrorHandlingConfig::default();
        assert!(config.detect_async_errors);
        assert!(config.detect_context_loss);
        assert!(config.detect_propagation);
        assert!(config.detect_panic_patterns);
        assert!(config.detect_swallowing);
        assert_eq!(config.custom_patterns.len(), 0);
        assert_eq!(config.severity_overrides.len(), 0);
    }

    #[test]
    fn test_god_object_config_default() {
        let config = GodObjectConfig::default();
        assert!(config.enabled);
        // Test Rust defaults
        assert_eq!(config.rust.max_methods, 20);
        assert_eq!(config.rust.max_fields, 15);
        // Test Python defaults
        assert_eq!(config.python.max_methods, 15);
        assert_eq!(config.python.max_fields, 10);
        // Test JavaScript defaults
        assert_eq!(config.javascript.max_methods, 15);
        assert_eq!(config.javascript.max_fields, 20);
    }

    #[test]
    fn test_context_config_default() {
        let config = ContextConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.rules.len(), 0);
        assert!(config.function_patterns.is_none());
    }

    #[test]
    fn test_language_features_default() {
        let features = LanguageFeatures::default();
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_minimum_debt_score() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let score = get_minimum_debt_score();
        // The default is 1.0 but config might override it to 2.0
        assert!(score >= 1.0);
    }

    #[test]
    fn test_get_minimum_cyclomatic_complexity() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let complexity = get_minimum_cyclomatic_complexity();
        // The default is 2 but config might override it to 3
        assert!(complexity >= 2);
    }

    #[test]
    fn test_get_minimum_cognitive_complexity() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let complexity = get_minimum_cognitive_complexity();
        // The default is 3 but config might override it to 5
        assert!(complexity >= 3);
    }

    #[test]
    fn test_get_minimum_risk_score() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let score = get_minimum_risk_score();
        // The default is 1.0 but config might override it to 2.0
        assert!(score >= 1.0);
    }

    #[test]
    fn test_validation_thresholds_default() {
        let thresholds = ValidationThresholds::default();

        // Primary quality metrics
        assert_eq!(thresholds.max_average_complexity, 10.0);
        assert_eq!(thresholds.max_debt_density, 50.0);
        assert_eq!(thresholds.max_codebase_risk_score, 7.0);
        assert_eq!(thresholds.min_coverage_percentage, 0.0);

        // Safety net - high ceiling
        assert_eq!(thresholds.max_total_debt_score, 10000);

        // Deprecated metrics should be None by default
        #[allow(deprecated)]
        {
            assert_eq!(thresholds.max_high_complexity_count, None);
            assert_eq!(thresholds.max_debt_items, None);
            assert_eq!(thresholds.max_high_risk_functions, None);
        }
    }

    #[test]
    fn test_get_language_features_rust() {
        use crate::core::Language;
        let features = get_language_features(&Language::Rust);
        assert!(!features.detect_dead_code); // Rust has dead code detection disabled
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_language_features_python() {
        use crate::core::Language;
        let features = get_language_features(&Language::Python);
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_language_features_javascript() {
        use crate::core::Language;
        let features = get_language_features(&Language::JavaScript);
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_language_features_typescript() {
        use crate::core::Language;
        let features = get_language_features(&Language::TypeScript);
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_language_features_unknown() {
        use crate::core::Language;
        let features = get_language_features(&Language::Unknown);
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_entropy_config() {
        let config = get_entropy_config();
        // Config might override these values
        assert!(config.enabled);
        // Weight might be configured to 0.5 in .debtmap.toml
        assert!(config.weight > 0.0);
    }

    #[test]
    fn test_get_role_multipliers() {
        let multipliers = get_role_multipliers();
        assert_eq!(multipliers.pure_logic, 1.2);
        assert_eq!(multipliers.orchestrator, 0.8);
    }

    #[test]
    fn test_get_error_handling_config() {
        let config = get_error_handling_config();
        assert!(config.detect_async_errors);
        assert!(config.detect_context_loss);
    }

    #[test]
    fn test_get_scoring_weights() {
        let weights = get_scoring_weights();
        assert_eq!(weights.coverage, 0.50);
        assert_eq!(weights.complexity, 0.35);
        assert_eq!(weights.dependency, 0.15);
    }

    #[test]
    fn test_default_weight_functions() {
        assert_eq!(default_coverage_weight(), 0.50);
        assert_eq!(default_complexity_weight(), 0.35);
        assert_eq!(default_semantic_weight(), 0.00);
        assert_eq!(default_dependency_weight(), 0.15);
        assert_eq!(default_security_weight(), 0.00);
        assert_eq!(default_organization_weight(), 0.00);
    }

    #[test]
    fn test_default_multiplier_functions() {
        assert_eq!(default_pure_logic_multiplier(), 1.2);
        assert_eq!(default_orchestrator_multiplier(), 0.8);
        assert_eq!(default_io_wrapper_multiplier(), 0.7);
        assert_eq!(default_entry_point_multiplier(), 0.9);
        assert_eq!(default_pattern_match_multiplier(), 0.6);
        assert_eq!(default_unknown_multiplier(), 1.0);
    }

    #[test]
    fn test_default_language_feature_functions() {
        use crate::config::languages::*;
        assert!(default_detect_dead_code());
        assert!(default_detect_complexity());
        assert!(default_detect_duplication());
    }

    #[test]
    fn test_default_entropy_functions() {
        use crate::config::languages::*;
        assert!(default_entropy_enabled());
        assert_eq!(default_entropy_weight(), 1.0);
        assert_eq!(default_entropy_min_tokens(), 20);
        assert_eq!(default_entropy_pattern_threshold(), 0.7);
        assert_eq!(default_entropy_threshold(), 0.4);
        assert_eq!(default_branch_threshold(), 0.8);
        assert_eq!(default_max_repetition_reduction(), 0.20);
        assert_eq!(default_max_entropy_reduction(), 0.15);
        assert_eq!(default_max_branch_reduction(), 0.25);
        assert_eq!(default_max_combined_reduction(), 0.30);
    }

    #[test]
    fn test_default_error_handling_functions() {
        // Test through the ErrorHandlingConfig::default() instead of private functions
        let config = ErrorHandlingConfig::default();
        assert!(config.detect_async_errors);
        assert!(config.detect_context_loss);
        assert!(config.detect_propagation);
        assert!(config.detect_panic_patterns);
        assert!(config.detect_swallowing);
    }

    // Tests for extracted pure functions (spec 93)

    #[test]
    fn test_is_valid_weight() {
        // Test valid weights
        assert!(ScoringWeights::is_valid_weight(0.0));
        assert!(ScoringWeights::is_valid_weight(0.5));
        assert!(ScoringWeights::is_valid_weight(1.0));

        // Test invalid weights
        assert!(!ScoringWeights::is_valid_weight(-0.1));
        assert!(!ScoringWeights::is_valid_weight(1.1));
        assert!(!ScoringWeights::is_valid_weight(2.0));
        assert!(!ScoringWeights::is_valid_weight(-10.0));
    }

    #[test]
    fn test_validate_weight() {
        // Test valid weight
        assert!(ScoringWeights::validate_weight(0.5, "Test").is_ok());
        assert!(ScoringWeights::validate_weight(0.0, "Min").is_ok());
        assert!(ScoringWeights::validate_weight(1.0, "Max").is_ok());

        // Test invalid weight
        let result = ScoringWeights::validate_weight(1.5, "Invalid");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Invalid weight must be between 0.0 and 1.0"
        );
    }

    #[test]
    fn test_validate_active_weights_sum() {
        // Test valid sum (exactly 1.0)
        assert!(ScoringWeights::validate_active_weights_sum(0.5, 0.3, 0.2).is_ok());

        // Test valid sum (within tolerance)
        assert!(ScoringWeights::validate_active_weights_sum(0.5, 0.3, 0.2001).is_ok());
        assert!(ScoringWeights::validate_active_weights_sum(0.5, 0.3, 0.1999).is_ok());

        // Test invalid sum (too high)
        let result = ScoringWeights::validate_active_weights_sum(0.6, 0.5, 0.3);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("must sum to 1.0, but sum to 1.400"));

        // Test invalid sum (too low)
        let result = ScoringWeights::validate_active_weights_sum(0.2, 0.2, 0.2);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("must sum to 1.0, but sum to 0.600"));
    }

    #[test]
    fn test_collect_weight_validations() {
        // Test with all valid weights
        let weights = ScoringWeights {
            coverage: 0.5,
            complexity: 0.3,
            semantic: 0.0,
            dependency: 0.2,
            security: 0.0,
            organization: 0.0,
        };
        let validations = weights.collect_weight_validations();
        assert_eq!(validations.len(), 6);
        for validation in validations {
            assert!(validation.is_ok());
        }

        // Test with invalid weights
        let weights = ScoringWeights {
            coverage: 1.5,    // Invalid
            complexity: -0.1, // Invalid
            semantic: 0.0,
            dependency: 0.2,
            security: 2.0, // Invalid
            organization: 0.0,
        };
        let validations = weights.collect_weight_validations();
        assert_eq!(validations.len(), 6);
        assert!(validations[0].is_err()); // coverage
        assert!(validations[1].is_err()); // complexity
        assert!(validations[2].is_ok()); // semantic
        assert!(validations[3].is_ok()); // dependency
        assert!(validations[4].is_err()); // security
        assert!(validations[5].is_ok()); // organization
    }

    #[test]
    fn test_read_config_file() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        // Write test config
        fs::write(&config_path, "[thresholds]\ncomplexity = 15\n").unwrap();

        // Test reading existing file
        let contents = read_config_file(&config_path).unwrap();
        assert_eq!(contents, "[thresholds]\ncomplexity = 15\n");

        // Test reading non-existent file
        let non_existent = temp_dir.path().join("non_existent.toml");
        assert!(read_config_file(&non_existent).is_err());
    }

    #[test]
    fn test_parse_and_validate_config_impl() {
        // Test valid config
        let valid_toml = r#"
[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15
"#;
        let config = parse_and_validate_config_impl(valid_toml).unwrap();
        let scoring = config.scoring.unwrap();
        assert_eq!(scoring.coverage, 0.50);
        assert_eq!(scoring.complexity, 0.35);
        assert_eq!(scoring.dependency, 0.15);

        // Test invalid TOML
        let invalid_toml = "invalid [[ toml";
        assert!(parse_and_validate_config_impl(invalid_toml).is_err());

        // Test config with invalid weights (should be normalized)
        let invalid_weights = r#"
[scoring]
coverage = 0.6
complexity = 0.6
dependency = 0.6
"#;
        let config = parse_and_validate_config_impl(invalid_weights).unwrap();
        // Should use defaults due to invalid sum
        let scoring = config.scoring.unwrap();
        assert_eq!(scoring.coverage, 0.50);
        assert_eq!(scoring.complexity, 0.35);
        assert_eq!(scoring.dependency, 0.15);
    }

    #[test]
    fn test_directory_ancestors_impl() {
        use std::path::PathBuf;

        // Test normal path traversal
        let start = PathBuf::from("/a/b/c/d");
        let ancestors: Vec<PathBuf> = directory_ancestors_impl(start.clone(), 3).collect();
        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0], PathBuf::from("/a/b/c/d"));
        assert_eq!(ancestors[1], PathBuf::from("/a/b/c"));
        assert_eq!(ancestors[2], PathBuf::from("/a/b"));

        // Test with depth limit
        let ancestors: Vec<PathBuf> = directory_ancestors_impl(start.clone(), 2).collect();
        assert_eq!(ancestors.len(), 2);

        // Test with root path
        let root = PathBuf::from("/");
        let ancestors: Vec<PathBuf> = directory_ancestors_impl(root, 5).collect();
        assert_eq!(ancestors.len(), 1);
        assert_eq!(ancestors[0], PathBuf::from("/"));

        // Test with zero depth
        let ancestors: Vec<PathBuf> = directory_ancestors_impl(start, 0).collect();
        assert_eq!(ancestors.len(), 0);
    }

    #[test]
    fn test_handle_read_error() {
        use std::io;
        use std::path::PathBuf;

        let path = PathBuf::from("/test/path.toml");

        // Test NotFound error (should not log warning)
        let not_found = io::Error::new(io::ErrorKind::NotFound, "File not found");
        handle_read_error(&path, &not_found); // Should not panic

        // Test PermissionDenied error (should log warning)
        let permission = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        handle_read_error(&path, &permission); // Should not panic

        // Test other errors
        let other = io::Error::other("Unknown error");
        handle_read_error(&path, &other); // Should not panic
    }
}

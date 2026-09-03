//! Multi-source configuration loading with precedence and source tracking.
//!
//! This module implements configuration loading from multiple sources with
//! layered precedence, as specified in Spec 201:
//!
//! 1. Built-in defaults (lowest priority)
//! 2. User config (`~/.config/debtmap/config.toml`)
//! 3. Project config (`.debtmap.toml`)
//! 4. Environment variables (`DEBTMAP_*`)
//! 5. CLI arguments (highest priority - handled at call site)
//!
//! # Features
//!
//! - **Multi-source loading**: Load from files, environment, and defaults
//! - **Source tracking**: Know where each config value came from
//! - **Error accumulation**: Show ALL config errors at once
//! - **Backwards compatible**: Optional config files, works without them
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::config::multi_source::{load_multi_source_config, ConfigSource};
//!
//! // Load config from all sources
//! let result = load_multi_source_config();
//! match result {
//!     Ok(traced) => {
//!         println!("Loaded config from: {:?}", traced.sources());
//!         let config = traced.config();
//!         // Use config...
//!     }
//!     Err(errors) => {
//!         for error in errors {
//!             eprintln!("Config error: {}", error);
//!         }
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};

use super::core::DebtmapConfig;
use super::loader::{directory_ancestors_impl, parse_runtime_config, read_config_file};
use super::scoring::ScoringWeights;
use super::thresholds::ThresholdsConfig;
use super::validation::validate_config;

/// Macro to merge an optional config field from source to target.
///
/// This eliminates repetitive merge patterns by providing a consistent way
/// to merge Option fields while tracking their source.
///
/// Following Stillwater philosophy: composition over complexity, DRY principle.
macro_rules! merge_optional_field {
    ($target:expr_2021, $source:expr_2021, $field:ident, $field_name:literal, $source_id:expr_2021, $field_sources:expr_2021) => {
        if $source.$field.is_some() {
            $target.$field = $source.$field.clone();
            $field_sources.insert($field_name.to_string(), $source_id.clone());
        }
    };
}
use crate::effects::{
    AnalysisValidation, validation_failure, validation_failures, validation_success,
};
use crate::errors::AnalysisError;

/// Configuration source identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigSource {
    /// Built-in default values
    Default,
    /// User config file (~/.config/debtmap/config.toml)
    UserConfig(PathBuf),
    /// Project config file (.debtmap.toml)
    ProjectConfig(PathBuf),
    /// Environment variable
    Environment(String),
    /// Custom config path (from DEBTMAP_CONFIG env var)
    CustomPath(PathBuf),
}

impl fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigSource::Default => write!(f, "built-in defaults"),
            ConfigSource::UserConfig(path) => write!(f, "user config: {}", path.display()),
            ConfigSource::ProjectConfig(path) => write!(f, "project config: {}", path.display()),
            ConfigSource::Environment(var) => write!(f, "environment variable: {}", var),
            ConfigSource::CustomPath(path) => write!(f, "custom config: {}", path.display()),
        }
    }
}

/// A traced configuration value with source information.
#[derive(Debug, Clone)]
pub struct TracedValue<T> {
    /// The actual value
    pub value: T,
    /// Where this value came from
    pub source: ConfigSource,
    /// Whether this value was overridden from an earlier source
    pub was_overridden: bool,
    /// Previous sources that were overridden (for debugging)
    pub previous_sources: Vec<ConfigSource>,
}

impl<T> TracedValue<T> {
    /// Create a new traced value
    pub fn new(value: T, source: ConfigSource) -> Self {
        Self {
            value,
            source,
            was_overridden: false,
            previous_sources: Vec::new(),
        }
    }

    /// Mark this value as overridden from an earlier source
    pub fn override_from(mut self, previous: ConfigSource) -> Self {
        self.was_overridden = true;
        self.previous_sources.push(previous);
        self
    }
}

/// Traced configuration with source tracking for all values.
#[derive(Debug, Clone)]
pub struct TracedConfig {
    /// The merged configuration
    config: DebtmapConfig,
    /// Sources that contributed to the final config (in order of application)
    sources: Vec<ConfigSource>,
    /// Per-field source tracking for common fields
    field_sources: HashMap<String, ConfigSource>,
}

impl TracedConfig {
    /// Get the merged configuration
    pub fn config(&self) -> &DebtmapConfig {
        &self.config
    }

    /// Consume and return the merged configuration
    pub fn into_config(self) -> DebtmapConfig {
        self.config
    }

    /// Get the sources that contributed to this config (in order of application)
    pub fn sources(&self) -> &[ConfigSource] {
        &self.sources
    }

    /// Get the source for a specific field path (e.g., "scoring.coverage")
    pub fn field_source(&self, path: &str) -> Option<&ConfigSource> {
        self.field_sources.get(path)
    }

    /// Get all field sources for display
    pub fn all_field_sources(&self) -> &HashMap<String, ConfigSource> {
        &self.field_sources
    }

    /// Check if a specific source was used
    pub fn has_source(&self, source: &ConfigSource) -> bool {
        self.sources.contains(source)
    }
}

#[derive(Debug, Default)]
struct ConfigEnvironment {
    custom_config_path: Option<PathBuf>,
    complexity_threshold: Option<u32>,
    coverage_weight: Option<f64>,
    complexity_weight: Option<f64>,
    dependency_weight: Option<f64>,
}

impl ConfigEnvironment {
    fn from_process() -> Self {
        Self {
            custom_config_path: env::var_os("DEBTMAP_CONFIG").map(PathBuf::from),
            complexity_threshold: parse_env_value("DEBTMAP_COMPLEXITY_THRESHOLD"),
            coverage_weight: parse_env_value("DEBTMAP_COVERAGE_WEIGHT"),
            complexity_weight: parse_env_value("DEBTMAP_COMPLEXITY_WEIGHT"),
            dependency_weight: parse_env_value("DEBTMAP_DEPENDENCY_WEIGHT"),
        }
    }
}

fn parse_env_value<T: std::str::FromStr>(name: &str) -> Option<T> {
    env::var(name).ok()?.parse().ok()
}

/// Load configuration from multiple sources with precedence.
///
/// Sources are loaded in order of precedence (lowest to highest):
/// 1. Built-in defaults
/// 2. User config (~/.config/debtmap/config.toml)
/// 3. Project config (.debtmap.toml in current dir or parent)
/// 4. Custom config (DEBTMAP_CONFIG env var)
/// 5. Environment variables (DEBTMAP_*)
///
/// # Returns
///
/// Returns a `TracedConfig` with source tracking, or accumulated errors
/// if any config file fails to parse.
pub fn load_multi_source_config() -> Result<TracedConfig, Vec<AnalysisError>> {
    load_multi_source_config_from(std::env::current_dir().unwrap_or_default())
}

/// Load configuration from multiple sources, starting from a specific directory.
pub fn load_multi_source_config_from(
    start_dir: PathBuf,
) -> Result<TracedConfig, Vec<AnalysisError>> {
    load_multi_source_config_with_inputs(
        start_dir,
        user_config_path(),
        ConfigEnvironment::from_process(),
    )
}

fn load_multi_source_config_with_inputs(
    start_dir: PathBuf,
    user_config_path: Option<PathBuf>,
    environment: ConfigEnvironment,
) -> Result<TracedConfig, Vec<AnalysisError>> {
    let mut errors = Vec::new();
    let mut sources = Vec::new();
    let mut field_sources = HashMap::new();

    // 1. Start with defaults
    let mut config = DebtmapConfig::default();
    sources.push(ConfigSource::Default);

    // 2. Load user config if it exists
    if let Some(user_config_path) = user_config_path {
        match load_config_from_path(&user_config_path) {
            Ok(user_config) => {
                let source = ConfigSource::UserConfig(user_config_path);
                merge_config(&mut config, &user_config, &source, &mut field_sources);
                sources.push(source);
            }
            Err(e) => {
                // Only report errors for files that exist but fail to parse
                if user_config_path.exists() {
                    errors.push(e);
                }
            }
        }
    }

    // 3. Load project config if it exists
    if let Some(project_config_path) = find_project_config(&start_dir) {
        match load_config_from_path(&project_config_path) {
            Ok(project_config) => {
                let source = ConfigSource::ProjectConfig(project_config_path);
                merge_config(&mut config, &project_config, &source, &mut field_sources);
                sources.push(source);
            }
            Err(e) => errors.push(e),
        }
    }

    // 4. Load custom config if DEBTMAP_CONFIG is set
    if let Some(custom_path) = environment.custom_config_path.as_ref() {
        match load_config_from_path(custom_path) {
            Ok(custom_config) => {
                let source = ConfigSource::CustomPath(custom_path.clone());
                merge_config(&mut config, &custom_config, &source, &mut field_sources);
                sources.push(source);
            }
            Err(e) => errors.push(e),
        }
    }

    // 5. Apply environment variable overrides
    apply_env_overrides(&mut config, &environment, &mut field_sources, &mut sources);

    // Return errors if any config files failed
    if !errors.is_empty() {
        return Err(errors);
    }

    // Validate the final merged config
    match validate_config(&config) {
        stillwater::Validation::Success(_) => {}
        stillwater::Validation::Failure(validation_errors) => {
            // Convert NonEmptyVec to Vec
            return Err(validation_errors.into_iter().collect());
        }
    }

    Ok(TracedConfig {
        config,
        sources,
        field_sources,
    })
}

/// Load configuration with validation, returning AnalysisValidation for error accumulation.
pub fn load_multi_source_config_validated() -> AnalysisValidation<TracedConfig> {
    match load_multi_source_config() {
        Ok(traced) => validation_success(traced),
        Err(errors) if errors.len() == 1 => validation_failure(errors.into_iter().next().unwrap()),
        Err(errors) => validation_failures(errors),
    }
}

/// Get the path to the user's config file.
///
/// Returns the platform configuration directory joined with `debtmap/config.toml`.
pub fn user_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("debtmap").join("config.toml"))
}

/// Find the project config file (.debtmap.toml) by searching up the directory tree.
fn find_project_config(start_dir: &Path) -> Option<PathBuf> {
    const MAX_TRAVERSAL_DEPTH: usize = 10;

    directory_ancestors_impl(start_dir.to_path_buf(), MAX_TRAVERSAL_DEPTH)
        .map(|dir| dir.join(".debtmap.toml"))
        .find(|path| path.exists())
}

/// Load and parse a config file from a specific path.
fn load_config_from_path(path: &Path) -> Result<DebtmapConfig, AnalysisError> {
    let contents = read_config_file(path).map_err(|e| {
        AnalysisError::io_with_path(format!("Cannot read config file: {}", e), path)
    })?;

    parse_runtime_config(&contents).map_err(|e| AnalysisError::config_with_path(e, path))
}

/// Merge source config into target config, tracking field sources.
///
/// Uses `merge_optional_field!` macro to eliminate repetitive merge patterns.
/// Following Stillwater philosophy: composition over complexity, DRY principle.
fn merge_config(
    target: &mut DebtmapConfig,
    source: &DebtmapConfig,
    source_id: &ConfigSource,
    field_sources: &mut HashMap<String, ConfigSource>,
) {
    // Merge scoring weights (with sub-field tracking)
    if source.scoring.is_some() {
        target.scoring = source.scoring.clone();
        field_sources.insert("scoring".to_string(), source_id.clone());
        if source.scoring.is_some() {
            field_sources.insert("scoring.coverage".to_string(), source_id.clone());
            field_sources.insert("scoring.complexity".to_string(), source_id.clone());
            field_sources.insert("scoring.dependency".to_string(), source_id.clone());
        }
    }

    // Merge all other optional fields using the macro
    merge_optional_field!(
        target,
        source,
        thresholds,
        "thresholds",
        source_id,
        field_sources
    );
    merge_optional_field!(target, source, display, "display", source_id, field_sources);
    merge_optional_field!(target, source, ignore, "ignore", source_id, field_sources);
    merge_optional_field!(target, source, output, "output", source_id, field_sources);
    merge_optional_field!(target, source, entropy, "entropy", source_id, field_sources);
    merge_optional_field!(
        target,
        source,
        role_multipliers,
        "role_multipliers",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        languages,
        "languages",
        source_id,
        field_sources
    );
    merge_optional_field!(target, source, context, "context", source_id, field_sources);
    merge_optional_field!(
        target,
        source,
        error_handling,
        "error_handling",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        normalization,
        "normalization",
        source_id,
        field_sources
    );
    merge_optional_field!(target, source, loc, "loc", source_id, field_sources);
    merge_optional_field!(target, source, tiers, "tiers", source_id, field_sources);
    merge_optional_field!(
        target,
        source,
        god_object_detection,
        "god_object_detection",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        external_api,
        "external_api",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        complexity_thresholds,
        "complexity_thresholds",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        role_coverage_weights,
        "role_coverage_weights",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        role_multiplier_config,
        "role_multiplier_config",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        orchestrator_detection,
        "orchestrator_detection",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        orchestration_adjustment,
        "orchestration_adjustment",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        classification,
        "classification",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        mapping_patterns,
        "mapping_patterns",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        coverage_expectations,
        "coverage_expectations",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        complexity_weights,
        "complexity_weights",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        functional_analysis,
        "functional_analysis",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        boilerplate_detection,
        "boilerplate_detection",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        scoring_rebalanced,
        "scoring_rebalanced",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        context_multipliers,
        "context_multipliers",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        batch_analysis,
        "batch_analysis",
        source_id,
        field_sources
    );
    merge_optional_field!(target, source, retry, "retry", source_id, field_sources);
    merge_optional_field!(
        target,
        source,
        analysis,
        "analysis",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        state_detection,
        "state_detection",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        data_flow_scoring,
        "data_flow_scoring",
        source_id,
        field_sources
    );
    merge_optional_field!(
        target,
        source,
        context_suggestion,
        "context_suggestion",
        source_id,
        field_sources
    );
}

/// Apply environment variable overrides to the config.
///
/// Supported environment variables:
/// - DEBTMAP_COMPLEXITY_THRESHOLD: Override complexity threshold
/// - DEBTMAP_COVERAGE_WEIGHT: Override coverage weight
/// - DEBTMAP_COMPLEXITY_WEIGHT: Override complexity weight
/// - DEBTMAP_DEPENDENCY_WEIGHT: Override dependency weight
fn apply_env_overrides(
    config: &mut DebtmapConfig,
    environment: &ConfigEnvironment,
    field_sources: &mut HashMap<String, ConfigSource>,
    sources: &mut Vec<ConfigSource>,
) {
    let mut any_env_override = false;

    // DEBTMAP_COMPLEXITY_THRESHOLD
    if let Some(threshold) = environment.complexity_threshold {
        let thresholds = config
            .thresholds
            .get_or_insert_with(ThresholdsConfig::default);
        thresholds.complexity = Some(threshold);
        field_sources.insert(
            "thresholds.complexity".to_string(),
            ConfigSource::Environment("DEBTMAP_COMPLEXITY_THRESHOLD".to_string()),
        );
        any_env_override = true;
    }

    // DEBTMAP_COVERAGE_WEIGHT
    if let Some(weight) = environment.coverage_weight {
        let scoring = config.scoring.get_or_insert_with(ScoringWeights::default);
        scoring.coverage = weight;
        field_sources.insert(
            "scoring.coverage".to_string(),
            ConfigSource::Environment("DEBTMAP_COVERAGE_WEIGHT".to_string()),
        );
        any_env_override = true;
    }

    // DEBTMAP_COMPLEXITY_WEIGHT
    if let Some(weight) = environment.complexity_weight {
        let scoring = config.scoring.get_or_insert_with(ScoringWeights::default);
        scoring.complexity = weight;
        field_sources.insert(
            "scoring.complexity".to_string(),
            ConfigSource::Environment("DEBTMAP_COMPLEXITY_WEIGHT".to_string()),
        );
        any_env_override = true;
    }

    // DEBTMAP_DEPENDENCY_WEIGHT
    if let Some(weight) = environment.dependency_weight {
        let scoring = config.scoring.get_or_insert_with(ScoringWeights::default);
        scoring.dependency = weight;
        field_sources.insert(
            "scoring.dependency".to_string(),
            ConfigSource::Environment("DEBTMAP_DEPENDENCY_WEIGHT".to_string()),
        );
        any_env_override = true;
    }

    if any_env_override {
        sources.push(ConfigSource::Environment("DEBTMAP_*".to_string()));
    }
}

/// Display configuration sources in a user-friendly format.
pub fn display_config_sources(traced: &TracedConfig) {
    println!("Configuration sources:");
    println!();

    let mut fields: Vec<_> = traced.all_field_sources().iter().collect();
    fields.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (path, source) in fields {
        println!("  {} = <value>", path);
        println!("    from: {}", source);
        println!();
    }

    println!("Source priority (lowest to highest):");
    for (i, source) in traced.sources().iter().enumerate() {
        println!("  {}. {}", i + 1, source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn merge_config_preserves_every_recent_root_section() {
        let source = DebtmapConfig {
            batch_analysis: Some(crate::config::BatchAnalysisConfig::default()),
            retry: Some(crate::config::RetryConfig::default()),
            analysis: Some(crate::config::AnalysisSettings::default()),
            state_detection: Some(
                crate::analyzers::state_field_detector::StateDetectionConfig::default(),
            ),
            data_flow_scoring: Some(crate::config::DataFlowScoringConfig::default()),
            context_suggestion: Some(crate::priority::context::ContextConfig::default()),
            ..DebtmapConfig::default()
        };
        let mut merged = DebtmapConfig::default();
        let mut fields = HashMap::new();
        let origin = ConfigSource::CustomPath(PathBuf::from("custom.toml"));

        merge_config(&mut merged, &source, &origin, &mut fields);

        assert!(merged.batch_analysis.is_some());
        assert!(merged.retry.is_some());
        assert!(merged.analysis.is_some());
        assert!(merged.state_detection.is_some());
        assert!(merged.data_flow_scoring.is_some());
        assert!(merged.context_suggestion.is_some());
        for name in [
            "batch_analysis",
            "retry",
            "analysis",
            "state_detection",
            "data_flow_scoring",
            "context_suggestion",
        ] {
            assert_eq!(fields.get(name), Some(&origin));
        }
    }

    #[test]
    fn test_user_config_path() {
        let path = user_config_path();
        // Should return Some on all platforms with a home directory
        if dirs::config_dir().is_some() {
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(
                path.ends_with("debtmap/config.toml") || path.ends_with("debtmap\\config.toml")
            );
        }
    }

    #[test]
    fn test_find_project_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".debtmap.toml");

        // No config file yet
        assert!(find_project_config(temp_dir.path()).is_none());

        // Create config file
        fs::write(&config_path, "[thresholds]\ncomplexity = 15\n").unwrap();

        // Should find it now
        let found = find_project_config(temp_dir.path());
        assert!(found.is_some());
        assert_eq!(found.unwrap(), config_path);
    }

    #[test]
    fn test_find_project_config_in_parent() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".debtmap.toml");
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        // Create config in parent
        fs::write(&config_path, "[thresholds]\ncomplexity = 15\n").unwrap();

        // Should find it from subdir
        let found = find_project_config(&subdir);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), config_path);
    }

    #[test]
    fn test_load_config_from_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.toml");

        fs::write(
            &config_path,
            r#"
[thresholds]
complexity = 20

[scoring]
coverage = 0.5
complexity = 0.35
dependency = 0.15
"#,
        )
        .unwrap();

        let config = load_config_from_path(&config_path).unwrap();
        assert_eq!(config.thresholds.as_ref().unwrap().complexity, Some(20));
        assert!((config.scoring.as_ref().unwrap().coverage - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_load_config_from_path_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.toml");

        fs::write(&config_path, "invalid [[ toml content").unwrap();

        let result = load_config_from_path(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_config() {
        let mut target = DebtmapConfig::default();
        let source = DebtmapConfig {
            thresholds: Some(ThresholdsConfig {
                complexity: Some(25),
                ..Default::default()
            }),
            ..Default::default()
        };
        let source_id = ConfigSource::ProjectConfig(PathBuf::from("/test/.debtmap.toml"));
        let mut field_sources = HashMap::new();

        merge_config(&mut target, &source, &source_id, &mut field_sources);

        assert_eq!(target.thresholds.as_ref().unwrap().complexity, Some(25));
        assert_eq!(field_sources.get("thresholds"), Some(&source_id));
    }

    #[test]
    fn test_config_source_display() {
        assert_eq!(ConfigSource::Default.to_string(), "built-in defaults");
        assert!(
            ConfigSource::UserConfig(PathBuf::from("/home/user/.config/debtmap/config.toml"))
                .to_string()
                .contains("user config")
        );
        assert!(
            ConfigSource::ProjectConfig(PathBuf::from("/project/.debtmap.toml"))
                .to_string()
                .contains("project config")
        );
        assert!(
            ConfigSource::Environment("DEBTMAP_COMPLEXITY_THRESHOLD".to_string())
                .to_string()
                .contains("environment variable")
        );
    }

    #[test]
    fn test_traced_config_sources() {
        let config = DebtmapConfig::default();
        let sources = vec![
            ConfigSource::Default,
            ConfigSource::ProjectConfig(PathBuf::from("/test")),
        ];
        let field_sources = HashMap::new();

        let traced = TracedConfig {
            config,
            sources,
            field_sources,
        };

        assert_eq!(traced.sources().len(), 2);
        assert!(traced.has_source(&ConfigSource::Default));
    }

    #[test]
    fn test_load_multi_source_config_from_empty_dir() {
        let temp_dir = TempDir::new().unwrap();

        // Should work with no config files (uses defaults)
        let result = load_multi_source_config_with_inputs(
            temp_dir.path().to_path_buf(),
            None,
            ConfigEnvironment::default(),
        );
        assert!(result.is_ok());

        let traced = result.unwrap();
        assert!(traced.has_source(&ConfigSource::Default));
    }

    #[test]
    fn test_load_multi_source_config_with_project_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".debtmap.toml");

        fs::write(
            &config_path,
            r#"
[thresholds]
complexity = 30
"#,
        )
        .unwrap();

        let result = load_multi_source_config_with_inputs(
            temp_dir.path().to_path_buf(),
            None,
            ConfigEnvironment::default(),
        );
        assert!(result.is_ok());

        let traced = result.unwrap();
        assert_eq!(
            traced.config().thresholds.as_ref().unwrap().complexity,
            Some(30)
        );
        assert!(traced.has_source(&ConfigSource::ProjectConfig(config_path)));
    }

    #[test]
    fn explicit_user_config_path_is_loaded() {
        let temp_dir = TempDir::new().unwrap();
        let user_config = temp_dir.path().join("user-config.toml");
        fs::write(&user_config, "[ignore]\npatterns = [\"*.rs\"]\n").unwrap();

        let traced = load_multi_source_config_with_inputs(
            temp_dir.path().to_path_buf(),
            Some(user_config.clone()),
            ConfigEnvironment::default(),
        )
        .unwrap();

        assert_eq!(traced.config().get_ignore_patterns(), vec!["*.rs"]);
        assert!(traced.has_source(&ConfigSource::UserConfig(user_config)));
    }

    #[test]
    fn test_env_overrides() {
        let mut config = DebtmapConfig::default();
        let mut field_sources = HashMap::new();
        let mut sources = Vec::new();
        let environment = ConfigEnvironment {
            complexity_threshold: Some(42),
            ..ConfigEnvironment::default()
        };

        apply_env_overrides(&mut config, &environment, &mut field_sources, &mut sources);

        assert_eq!(config.thresholds.as_ref().unwrap().complexity, Some(42));
        assert!(field_sources.contains_key("thresholds.complexity"));
    }
}

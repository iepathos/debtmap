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
use super::loader::{directory_ancestors_impl, parse_and_validate_config_impl, read_config_file};
use super::scoring::ScoringWeights;
use super::thresholds::ThresholdsConfig;
use super::validation::validate_config;
use crate::effects::{
    validation_failure, validation_failures, validation_success, AnalysisValidation,
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
    let mut errors = Vec::new();
    let mut sources = Vec::new();
    let mut field_sources = HashMap::new();

    // 1. Start with defaults
    let mut config = DebtmapConfig::default();
    sources.push(ConfigSource::Default);

    // 2. Load user config if it exists
    if let Some(user_config_path) = user_config_path() {
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
    if let Ok(custom_path) = env::var("DEBTMAP_CONFIG") {
        let custom_path = PathBuf::from(custom_path);
        match load_config_from_path(&custom_path) {
            Ok(custom_config) => {
                let source = ConfigSource::CustomPath(custom_path);
                merge_config(&mut config, &custom_config, &source, &mut field_sources);
                sources.push(source);
            }
            Err(e) => errors.push(e),
        }
    }

    // 5. Apply environment variable overrides
    apply_env_overrides(&mut config, &mut field_sources, &mut sources);

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
/// Returns `~/.config/debtmap/config.toml` on Unix/macOS,
/// or the equivalent on Windows.
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

    parse_and_validate_config_impl(&contents).map_err(|e| AnalysisError::config_with_path(e, path))
}

/// Merge source config into target config, tracking field sources.
fn merge_config(
    target: &mut DebtmapConfig,
    source: &DebtmapConfig,
    source_id: &ConfigSource,
    field_sources: &mut HashMap<String, ConfigSource>,
) {
    // Merge scoring weights
    if source.scoring.is_some() {
        target.scoring = source.scoring.clone();
        field_sources.insert("scoring".to_string(), source_id.clone());
        if let Some(ref scoring) = source.scoring {
            field_sources.insert("scoring.coverage".to_string(), source_id.clone());
            field_sources.insert("scoring.complexity".to_string(), source_id.clone());
            field_sources.insert("scoring.dependency".to_string(), source_id.clone());
            let _ = scoring; // Suppress unused warning
        }
    }

    // Merge thresholds
    if source.thresholds.is_some() {
        target.thresholds = source.thresholds.clone();
        field_sources.insert("thresholds".to_string(), source_id.clone());
    }

    // Merge display config
    if source.display.is_some() {
        target.display = source.display.clone();
        field_sources.insert("display".to_string(), source_id.clone());
    }

    // Merge ignore patterns
    if source.ignore.is_some() {
        target.ignore = source.ignore.clone();
        field_sources.insert("ignore".to_string(), source_id.clone());
    }

    // Merge output config
    if source.output.is_some() {
        target.output = source.output.clone();
        field_sources.insert("output".to_string(), source_id.clone());
    }

    // Merge entropy config
    if source.entropy.is_some() {
        target.entropy = source.entropy.clone();
        field_sources.insert("entropy".to_string(), source_id.clone());
    }

    // Merge role multipliers
    if source.role_multipliers.is_some() {
        target.role_multipliers = source.role_multipliers.clone();
        field_sources.insert("role_multipliers".to_string(), source_id.clone());
    }

    // Merge languages config
    if source.languages.is_some() {
        target.languages = source.languages.clone();
        field_sources.insert("languages".to_string(), source_id.clone());
    }

    // Merge context config
    if source.context.is_some() {
        target.context = source.context.clone();
        field_sources.insert("context".to_string(), source_id.clone());
    }

    // Merge error handling config
    if source.error_handling.is_some() {
        target.error_handling = source.error_handling.clone();
        field_sources.insert("error_handling".to_string(), source_id.clone());
    }

    // Merge normalization config
    if source.normalization.is_some() {
        target.normalization = source.normalization.clone();
        field_sources.insert("normalization".to_string(), source_id.clone());
    }

    // Merge LOC config
    if source.loc.is_some() {
        target.loc = source.loc.clone();
        field_sources.insert("loc".to_string(), source_id.clone());
    }

    // Merge tiers config
    if source.tiers.is_some() {
        target.tiers = source.tiers.clone();
        field_sources.insert("tiers".to_string(), source_id.clone());
    }

    // Merge god object detection config
    if source.god_object_detection.is_some() {
        target.god_object_detection = source.god_object_detection.clone();
        field_sources.insert("god_object_detection".to_string(), source_id.clone());
    }

    // Merge external API config
    if source.external_api.is_some() {
        target.external_api = source.external_api.clone();
        field_sources.insert("external_api".to_string(), source_id.clone());
    }

    // Merge complexity thresholds
    if source.complexity_thresholds.is_some() {
        target.complexity_thresholds = source.complexity_thresholds.clone();
        field_sources.insert("complexity_thresholds".to_string(), source_id.clone());
    }

    // Merge role coverage weights
    if source.role_coverage_weights.is_some() {
        target.role_coverage_weights = source.role_coverage_weights.clone();
        field_sources.insert("role_coverage_weights".to_string(), source_id.clone());
    }

    // Merge role multiplier config
    if source.role_multiplier_config.is_some() {
        target.role_multiplier_config = source.role_multiplier_config.clone();
        field_sources.insert("role_multiplier_config".to_string(), source_id.clone());
    }

    // Merge orchestrator detection config
    if source.orchestrator_detection.is_some() {
        target.orchestrator_detection = source.orchestrator_detection.clone();
        field_sources.insert("orchestrator_detection".to_string(), source_id.clone());
    }

    // Merge orchestration adjustment config
    if source.orchestration_adjustment.is_some() {
        target.orchestration_adjustment = source.orchestration_adjustment.clone();
        field_sources.insert("orchestration_adjustment".to_string(), source_id.clone());
    }

    // Merge classification config
    if source.classification.is_some() {
        target.classification = source.classification.clone();
        field_sources.insert("classification".to_string(), source_id.clone());
    }

    // Merge mapping patterns config
    if source.mapping_patterns.is_some() {
        target.mapping_patterns = source.mapping_patterns.clone();
        field_sources.insert("mapping_patterns".to_string(), source_id.clone());
    }

    // Merge coverage expectations config
    if source.coverage_expectations.is_some() {
        target.coverage_expectations = source.coverage_expectations.clone();
        field_sources.insert("coverage_expectations".to_string(), source_id.clone());
    }

    // Merge complexity weights config
    if source.complexity_weights.is_some() {
        target.complexity_weights = source.complexity_weights.clone();
        field_sources.insert("complexity_weights".to_string(), source_id.clone());
    }

    // Merge functional analysis config
    if source.functional_analysis.is_some() {
        target.functional_analysis = source.functional_analysis.clone();
        field_sources.insert("functional_analysis".to_string(), source_id.clone());
    }

    // Merge boilerplate detection config
    if source.boilerplate_detection.is_some() {
        target.boilerplate_detection = source.boilerplate_detection.clone();
        field_sources.insert("boilerplate_detection".to_string(), source_id.clone());
    }

    // Merge rebalanced scoring config
    if source.scoring_rebalanced.is_some() {
        target.scoring_rebalanced = source.scoring_rebalanced.clone();
        field_sources.insert("scoring_rebalanced".to_string(), source_id.clone());
    }

    // Merge context multipliers config
    if source.context_multipliers.is_some() {
        target.context_multipliers = source.context_multipliers.clone();
        field_sources.insert("context_multipliers".to_string(), source_id.clone());
    }
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
    field_sources: &mut HashMap<String, ConfigSource>,
    sources: &mut Vec<ConfigSource>,
) {
    let mut any_env_override = false;

    // DEBTMAP_COMPLEXITY_THRESHOLD
    if let Ok(value) = env::var("DEBTMAP_COMPLEXITY_THRESHOLD") {
        if let Ok(threshold) = value.parse::<u32>() {
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
    }

    // DEBTMAP_COVERAGE_WEIGHT
    if let Ok(value) = env::var("DEBTMAP_COVERAGE_WEIGHT") {
        if let Ok(weight) = value.parse::<f64>() {
            let scoring = config.scoring.get_or_insert_with(ScoringWeights::default);
            scoring.coverage = weight;
            field_sources.insert(
                "scoring.coverage".to_string(),
                ConfigSource::Environment("DEBTMAP_COVERAGE_WEIGHT".to_string()),
            );
            any_env_override = true;
        }
    }

    // DEBTMAP_COMPLEXITY_WEIGHT
    if let Ok(value) = env::var("DEBTMAP_COMPLEXITY_WEIGHT") {
        if let Ok(weight) = value.parse::<f64>() {
            let scoring = config.scoring.get_or_insert_with(ScoringWeights::default);
            scoring.complexity = weight;
            field_sources.insert(
                "scoring.complexity".to_string(),
                ConfigSource::Environment("DEBTMAP_COMPLEXITY_WEIGHT".to_string()),
            );
            any_env_override = true;
        }
    }

    // DEBTMAP_DEPENDENCY_WEIGHT
    if let Ok(value) = env::var("DEBTMAP_DEPENDENCY_WEIGHT") {
        if let Ok(weight) = value.parse::<f64>() {
            let scoring = config.scoring.get_or_insert_with(ScoringWeights::default);
            scoring.dependency = weight;
            field_sources.insert(
                "scoring.dependency".to_string(),
                ConfigSource::Environment("DEBTMAP_DEPENDENCY_WEIGHT".to_string()),
            );
            any_env_override = true;
        }
    }

    if any_env_override {
        sources.push(ConfigSource::Environment("DEBTMAP_*".to_string()));
    }
}

/// Display configuration sources in a user-friendly format.
pub fn display_config_sources(traced: &TracedConfig) {
    println!("Configuration sources:");
    println!();

    for (path, source) in traced.all_field_sources() {
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
        let result = load_multi_source_config_from(temp_dir.path().to_path_buf());
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

        let result = load_multi_source_config_from(temp_dir.path().to_path_buf());
        assert!(result.is_ok());

        let traced = result.unwrap();
        assert_eq!(
            traced.config().thresholds.as_ref().unwrap().complexity,
            Some(30)
        );
        assert!(traced.has_source(&ConfigSource::ProjectConfig(config_path)));
    }

    #[test]
    fn test_env_overrides() {
        // Save original env vars
        let orig_threshold = env::var("DEBTMAP_COMPLEXITY_THRESHOLD").ok();

        // Set env var
        env::set_var("DEBTMAP_COMPLEXITY_THRESHOLD", "42");

        let mut config = DebtmapConfig::default();
        let mut field_sources = HashMap::new();
        let mut sources = Vec::new();

        apply_env_overrides(&mut config, &mut field_sources, &mut sources);

        assert_eq!(config.thresholds.as_ref().unwrap().complexity, Some(42));
        assert!(field_sources.contains_key("thresholds.complexity"));

        // Restore original env var
        match orig_threshold {
            Some(v) => env::set_var("DEBTMAP_COMPLEXITY_THRESHOLD", v),
            None => env::remove_var("DEBTMAP_COMPLEXITY_THRESHOLD"),
        }
    }
}

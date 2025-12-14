//! Options builder for unified analysis.
//!
//! This module provides a builder pattern for constructing analysis options
//! with type-safe defaults and validation.

use crate::core::AnalysisResults;
use crate::formatting::FormattingConfig;
use std::path::{Path, PathBuf};

/// Configuration error types.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Options for unified analysis (borrowed version for compatibility).
pub struct UnifiedAnalysisOptions<'a> {
    pub results: &'a AnalysisResults,
    pub coverage_file: Option<&'a PathBuf>,
    pub semantic_off: bool,
    pub project_path: &'a Path,
    pub verbose_macro_warnings: bool,
    pub show_macro_stats: bool,
    pub parallel: bool,
    pub jobs: usize,
    pub multi_pass: bool,
    pub show_attribution: bool,
    pub aggregate_only: bool,
    pub no_aggregation: bool,
    pub aggregation_method: Option<String>,
    pub min_problematic: Option<usize>,
    pub no_god_object: bool,
    pub suppress_coverage_tip: bool,
    pub _formatting_config: FormattingConfig,
    pub enable_context: bool,
    pub context_providers: Option<Vec<String>>,
    pub disable_context: Option<Vec<String>>,
    /// Pre-discovered Rust files from stage 0 (avoids re-walking filesystem)
    pub rust_files: Option<Vec<PathBuf>>,
}

/// Owned options for unified analysis.
#[derive(Debug, Clone)]
pub struct UnifiedAnalysisConfig {
    pub coverage_file: Option<PathBuf>,
    pub semantic_off: bool,
    pub project_path: PathBuf,
    pub verbose_macro_warnings: bool,
    pub show_macro_stats: bool,
    pub parallel: bool,
    pub jobs: usize,
    pub multi_pass: bool,
    pub show_attribution: bool,
    pub aggregate_only: bool,
    pub no_aggregation: bool,
    pub aggregation_method: Option<String>,
    pub min_problematic: Option<usize>,
    pub no_god_object: bool,
    pub suppress_coverage_tip: bool,
    pub formatting_config: FormattingConfig,
    pub enable_context: bool,
    pub context_providers: Option<Vec<String>>,
    pub disable_context: Option<Vec<String>>,
}

impl Default for UnifiedAnalysisConfig {
    fn default() -> Self {
        Self {
            coverage_file: None,
            semantic_off: false,
            project_path: PathBuf::from("."),
            verbose_macro_warnings: false,
            show_macro_stats: false,
            parallel: false,
            jobs: 0,
            multi_pass: false,
            show_attribution: false,
            aggregate_only: false,
            no_aggregation: false,
            aggregation_method: None,
            min_problematic: None,
            no_god_object: false,
            suppress_coverage_tip: false,
            formatting_config: FormattingConfig::from_env(),
            enable_context: false,
            context_providers: None,
            disable_context: None,
        }
    }
}

/// Builder for UnifiedAnalysisConfig.
#[derive(Default)]
pub struct UnifiedAnalysisConfigBuilder {
    coverage_file: Option<PathBuf>,
    semantic_off: bool,
    project_path: Option<PathBuf>,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    parallel: bool,
    jobs: usize,
    multi_pass: bool,
    show_attribution: bool,
    aggregate_only: bool,
    no_aggregation: bool,
    aggregation_method: Option<String>,
    min_problematic: Option<usize>,
    no_god_object: bool,
    suppress_coverage_tip: bool,
    formatting_config: Option<FormattingConfig>,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
}

impl UnifiedAnalysisConfigBuilder {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the project path.
    pub fn project_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.project_path = Some(path.into());
        self
    }

    /// Set the coverage file path.
    pub fn coverage_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.coverage_file = Some(path.into());
        self
    }

    /// Set optional coverage file.
    pub fn coverage_file_opt(mut self, path: Option<PathBuf>) -> Self {
        self.coverage_file = path;
        self
    }

    /// Disable semantic analysis.
    pub fn semantic_off(mut self) -> Self {
        self.semantic_off = true;
        self
    }

    /// Enable verbose macro warnings.
    pub fn verbose_macro_warnings(mut self) -> Self {
        self.verbose_macro_warnings = true;
        self
    }

    /// Enable macro stats display.
    pub fn show_macro_stats(mut self) -> Self {
        self.show_macro_stats = true;
        self
    }

    /// Enable parallel processing.
    pub fn parallel(mut self) -> Self {
        self.parallel = true;
        self
    }

    /// Set the number of parallel jobs.
    pub fn jobs(mut self, jobs: usize) -> Self {
        self.jobs = jobs;
        self
    }

    /// Enable multi-pass analysis.
    pub fn multi_pass(mut self) -> Self {
        self.multi_pass = true;
        self
    }

    /// Enable attribution display.
    pub fn show_attribution(mut self) -> Self {
        self.show_attribution = true;
        self
    }

    /// Enable aggregate-only mode.
    pub fn aggregate_only(mut self) -> Self {
        self.aggregate_only = true;
        self
    }

    /// Disable aggregation.
    pub fn no_aggregation(mut self) -> Self {
        self.no_aggregation = true;
        self
    }

    /// Set the aggregation method.
    pub fn aggregation_method(mut self, method: impl Into<String>) -> Self {
        self.aggregation_method = Some(method.into());
        self
    }

    /// Set minimum problematic threshold.
    pub fn min_problematic(mut self, count: usize) -> Self {
        self.min_problematic = Some(count);
        self
    }

    /// Disable god object detection.
    pub fn no_god_object(mut self) -> Self {
        self.no_god_object = true;
        self
    }

    /// Suppress coverage tip message.
    pub fn suppress_coverage_tip(mut self) -> Self {
        self.suppress_coverage_tip = true;
        self
    }

    /// Set formatting configuration.
    pub fn formatting_config(mut self, config: FormattingConfig) -> Self {
        self.formatting_config = Some(config);
        self
    }

    /// Enable context analysis.
    pub fn enable_context(mut self) -> Self {
        self.enable_context = true;
        self
    }

    /// Set context providers.
    pub fn context_providers(mut self, providers: Vec<String>) -> Self {
        self.context_providers = Some(providers);
        self
    }

    /// Set disabled context providers.
    pub fn disable_context(mut self, providers: Vec<String>) -> Self {
        self.disable_context = Some(providers);
        self
    }

    /// Build the configuration.
    ///
    /// Environment variables are read here at the edge, not in pure computation.
    pub fn build(self) -> Result<UnifiedAnalysisConfig, ConfigError> {
        let project_path = self
            .project_path
            .ok_or(ConfigError::MissingField("project_path"))?;

        // Environment variables only read here, at the edge
        let parallel = self.parallel
            || std::env::var("DEBTMAP_PARALLEL")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false);

        let jobs = if self.jobs > 0 {
            self.jobs
        } else {
            std::env::var("DEBTMAP_JOBS")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0)
        };

        Ok(UnifiedAnalysisConfig {
            coverage_file: self.coverage_file,
            semantic_off: self.semantic_off,
            project_path,
            verbose_macro_warnings: self.verbose_macro_warnings,
            show_macro_stats: self.show_macro_stats,
            parallel,
            jobs,
            multi_pass: self.multi_pass,
            show_attribution: self.show_attribution,
            aggregate_only: self.aggregate_only,
            no_aggregation: self.no_aggregation,
            aggregation_method: self.aggregation_method,
            min_problematic: self.min_problematic,
            no_god_object: self.no_god_object,
            suppress_coverage_tip: self.suppress_coverage_tip,
            formatting_config: self
                .formatting_config
                .unwrap_or_else(FormattingConfig::from_env),
            enable_context: self.enable_context,
            context_providers: self.context_providers,
            disable_context: self.disable_context,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_requires_project_path() {
        let result = UnifiedAnalysisConfigBuilder::new().build();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::MissingField("project_path")
        ));
    }

    #[test]
    fn test_builder_with_project_path() {
        let result = UnifiedAnalysisConfigBuilder::new()
            .project_path("/tmp/test")
            .build();
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.project_path, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_builder_chaining() {
        let config = UnifiedAnalysisConfigBuilder::new()
            .project_path("/tmp/test")
            .parallel()
            .jobs(4)
            .no_god_object()
            .build()
            .unwrap();

        assert!(config.parallel);
        assert_eq!(config.jobs, 4);
        assert!(config.no_god_object);
    }

    #[test]
    fn test_default_config() {
        let config = UnifiedAnalysisConfig::default();
        assert!(!config.parallel);
        assert_eq!(config.jobs, 0);
        assert!(!config.no_god_object);
        assert!(!config.multi_pass);
    }
}

//! Type-state pattern for validating configuration before execution
//!
//! This module provides compile-time guarantees that configuration
//! is validated before being executed. Using phantom types, we encode
//! validation states in the type system itself.
//!
//! # Example
//!
//! ```ignore
//! use debtmap::commands::state::{AnalyzeConfig, Unvalidated, Validated};
//!
//! // Create unvalidated config
//! let config: AnalyzeConfig<Unvalidated> = AnalyzeConfig::new(path);
//!
//! // This won't compile - unvalidated config cannot be executed:
//! // config.execute(); // ERROR: method not found
//!
//! // Must validate first:
//! let validated = config.validate()?;
//! validated.execute()?; // OK - validated config can execute
//! ```

use std::marker::PhantomData;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli;
use crate::formatting::FormattingConfig;

/// Marker type representing unvalidated state
#[derive(Debug, Clone, Copy)]
pub struct Unvalidated;

/// Marker type representing validated state
#[derive(Debug, Clone, Copy)]
pub struct Validated;

/// Configuration with type-state validation
///
/// Generic parameter `State` encodes whether config has been validated.
/// Default is `Unvalidated` to ensure validation is explicit.
#[derive(Debug, Clone)]
pub struct AnalyzeConfig<State = Unvalidated> {
    // Configuration fields
    pub path: PathBuf,
    pub format: cli::OutputFormat,
    pub output: Option<PathBuf>,
    pub threshold_complexity: u32,
    pub threshold_duplication: usize,
    pub languages: Option<Vec<String>>,
    pub coverage_file: Option<PathBuf>,
    pub enable_context: bool,
    pub context_providers: Option<Vec<String>>,
    pub disable_context: Option<Vec<String>>,
    pub top: Option<usize>,
    pub tail: Option<usize>,
    pub summary: bool,
    pub semantic_off: bool,
    pub verbosity: u8,
    pub verbose_macro_warnings: bool,
    pub show_macro_stats: bool,
    pub group_by_category: bool,
    pub min_priority: Option<String>,
    pub min_score: Option<f64>,
    pub filter_categories: Option<Vec<String>>,
    pub no_context_aware: bool,
    pub threshold_preset: Option<cli::ThresholdPreset>,
    pub _formatting_config: FormattingConfig,
    pub parallel: bool,
    pub jobs: usize,
    pub multi_pass: bool,
    pub show_attribution: bool,
    pub detail_level: Option<String>,
    pub aggregate_only: bool,
    pub no_aggregation: bool,
    pub aggregation_method: Option<String>,
    pub min_problematic: Option<usize>,
    pub no_god_object: bool,
    pub max_files: Option<usize>,
    pub validate_loc: bool,
    pub no_public_api_detection: bool,
    pub public_api_threshold: f32,
    pub no_pattern_detection: bool,
    pub patterns: Option<Vec<String>>,
    pub pattern_threshold: f32,
    pub show_pattern_warnings: bool,
    pub debug_call_graph: bool,
    pub trace_functions: Option<Vec<String>>,
    pub call_graph_stats_only: bool,
    pub debug_format: cli::DebugFormatArg,
    pub validate_call_graph: bool,
    pub show_dependencies: bool,
    pub no_dependencies: bool,
    pub max_callers: usize,
    pub max_callees: usize,
    pub show_external: bool,
    pub show_std_lib: bool,
    pub ast_functional_analysis: bool,
    pub functional_analysis_profile: Option<cli::FunctionalAnalysisProfile>,
    pub min_split_methods: usize,
    pub min_split_lines: usize,
    pub no_tui: bool,
    pub show_filter_stats: bool,

    // Phantom type to encode validation state
    _state: PhantomData<State>,
}

impl AnalyzeConfig<Unvalidated> {
    /// Create a new unvalidated configuration
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        path: PathBuf,
        format: cli::OutputFormat,
        output: Option<PathBuf>,
        threshold_complexity: u32,
        threshold_duplication: usize,
        languages: Option<Vec<String>>,
        coverage_file: Option<PathBuf>,
        enable_context: bool,
        context_providers: Option<Vec<String>>,
        disable_context: Option<Vec<String>>,
        top: Option<usize>,
        tail: Option<usize>,
        summary: bool,
        semantic_off: bool,
        verbosity: u8,
        verbose_macro_warnings: bool,
        show_macro_stats: bool,
        group_by_category: bool,
        min_priority: Option<String>,
        min_score: Option<f64>,
        filter_categories: Option<Vec<String>>,
        no_context_aware: bool,
        threshold_preset: Option<cli::ThresholdPreset>,
        _formatting_config: FormattingConfig,
        parallel: bool,
        jobs: usize,
        multi_pass: bool,
        show_attribution: bool,
        detail_level: Option<String>,
        aggregate_only: bool,
        no_aggregation: bool,
        aggregation_method: Option<String>,
        min_problematic: Option<usize>,
        no_god_object: bool,
        max_files: Option<usize>,
        validate_loc: bool,
        no_public_api_detection: bool,
        public_api_threshold: f32,
        no_pattern_detection: bool,
        patterns: Option<Vec<String>>,
        pattern_threshold: f32,
        show_pattern_warnings: bool,
        debug_call_graph: bool,
        trace_functions: Option<Vec<String>>,
        call_graph_stats_only: bool,
        debug_format: cli::DebugFormatArg,
        validate_call_graph: bool,
        show_dependencies: bool,
        no_dependencies: bool,
        max_callers: usize,
        max_callees: usize,
        show_external: bool,
        show_std_lib: bool,
        ast_functional_analysis: bool,
        functional_analysis_profile: Option<cli::FunctionalAnalysisProfile>,
        min_split_methods: usize,
        min_split_lines: usize,
        no_tui: bool,
        show_filter_stats: bool,
    ) -> Self {
        AnalyzeConfig {
            path,
            format,
            output,
            threshold_complexity,
            threshold_duplication,
            languages,
            coverage_file,
            enable_context,
            context_providers,
            disable_context,
            top,
            tail,
            summary,
            semantic_off,
            verbosity,
            verbose_macro_warnings,
            show_macro_stats,
            group_by_category,
            min_priority,
            min_score,
            filter_categories,
            no_context_aware,
            threshold_preset,
            _formatting_config,
            parallel,
            jobs,
            multi_pass,
            show_attribution,
            detail_level,
            aggregate_only,
            no_aggregation,
            aggregation_method,
            min_problematic,
            no_god_object,
            max_files,
            validate_loc,
            no_public_api_detection,
            public_api_threshold,
            no_pattern_detection,
            patterns,
            pattern_threshold,
            show_pattern_warnings,
            debug_call_graph,
            trace_functions,
            call_graph_stats_only,
            debug_format,
            validate_call_graph,
            show_dependencies,
            no_dependencies,
            max_callers,
            max_callees,
            show_external,
            show_std_lib,
            ast_functional_analysis,
            functional_analysis_profile,
            min_split_methods,
            min_split_lines,
            no_tui,
            show_filter_stats,
            _state: PhantomData,
        }
    }

    /// Validate configuration and transition to validated state
    ///
    /// This method performs validation checks and returns a validated
    /// configuration on success. The return type guarantees at compile-time
    /// that only validated config can be executed.
    pub fn validate(self) -> Result<AnalyzeConfig<Validated>> {
        // Validate path exists
        if !self.path.exists() {
            anyhow::bail!("Path does not exist: {}", self.path.display());
        }

        // Validate thresholds are reasonable
        if self.threshold_complexity == 0 {
            anyhow::bail!("Complexity threshold must be greater than 0");
        }

        // Validate output path if specified
        if let Some(ref output) = self.output {
            if let Some(parent) = output.parent() {
                // Empty parent means current directory (e.g., "file.json" has parent "")
                // which is always valid
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    anyhow::bail!("Output directory does not exist: {}", parent.display());
                }
            }
        }

        // Validate coverage file if specified
        if let Some(ref coverage_file) = self.coverage_file {
            if !coverage_file.exists() {
                anyhow::bail!("Coverage file does not exist: {}", coverage_file.display());
            }
        }

        // Validate job count
        if self.jobs == 0 && self.parallel {
            anyhow::bail!("Job count must be greater than 0 when parallel is enabled");
        }

        // Validate min_score if specified
        if let Some(min_score) = self.min_score {
            if !(0.0..=100.0).contains(&min_score) {
                anyhow::bail!("Minimum score must be between 0.0 and 100.0");
            }
        }

        // Validation successful - transition to validated state
        Ok(AnalyzeConfig {
            path: self.path,
            format: self.format,
            output: self.output,
            threshold_complexity: self.threshold_complexity,
            threshold_duplication: self.threshold_duplication,
            languages: self.languages,
            coverage_file: self.coverage_file,
            enable_context: self.enable_context,
            context_providers: self.context_providers,
            disable_context: self.disable_context,
            top: self.top,
            tail: self.tail,
            summary: self.summary,
            semantic_off: self.semantic_off,
            verbosity: self.verbosity,
            verbose_macro_warnings: self.verbose_macro_warnings,
            show_macro_stats: self.show_macro_stats,
            group_by_category: self.group_by_category,
            min_priority: self.min_priority,
            min_score: self.min_score,
            filter_categories: self.filter_categories,
            no_context_aware: self.no_context_aware,
            threshold_preset: self.threshold_preset,
            _formatting_config: self._formatting_config,
            parallel: self.parallel,
            jobs: self.jobs,
            multi_pass: self.multi_pass,
            show_attribution: self.show_attribution,
            detail_level: self.detail_level,
            aggregate_only: self.aggregate_only,
            no_aggregation: self.no_aggregation,
            aggregation_method: self.aggregation_method,
            min_problematic: self.min_problematic,
            no_god_object: self.no_god_object,
            max_files: self.max_files,
            validate_loc: self.validate_loc,
            no_public_api_detection: self.no_public_api_detection,
            public_api_threshold: self.public_api_threshold,
            no_pattern_detection: self.no_pattern_detection,
            patterns: self.patterns,
            pattern_threshold: self.pattern_threshold,
            show_pattern_warnings: self.show_pattern_warnings,
            debug_call_graph: self.debug_call_graph,
            trace_functions: self.trace_functions,
            call_graph_stats_only: self.call_graph_stats_only,
            debug_format: self.debug_format,
            validate_call_graph: self.validate_call_graph,
            show_dependencies: self.show_dependencies,
            no_dependencies: self.no_dependencies,
            max_callers: self.max_callers,
            max_callees: self.max_callees,
            show_external: self.show_external,
            show_std_lib: self.show_std_lib,
            ast_functional_analysis: self.ast_functional_analysis,
            functional_analysis_profile: self.functional_analysis_profile,
            min_split_methods: self.min_split_methods,
            min_split_lines: self.min_split_lines,
            no_tui: self.no_tui,
            show_filter_stats: self.show_filter_stats,
            _state: PhantomData,
        })
    }
}

impl AnalyzeConfig<Validated> {
    /// Execute the analysis with validated configuration
    ///
    /// This method is only available on validated configs, providing
    /// compile-time guarantee that validation has occurred.
    pub fn execute(self) -> Result<()> {
        // Convert to old-style config for backward compatibility
        let old_config = super::analyze::AnalyzeConfig {
            path: self.path,
            format: self.format,
            output: self.output,
            threshold_complexity: self.threshold_complexity,
            threshold_duplication: self.threshold_duplication,
            languages: self.languages,
            coverage_file: self.coverage_file,
            enable_context: self.enable_context,
            context_providers: self.context_providers,
            disable_context: self.disable_context,
            top: self.top,
            tail: self.tail,
            summary: self.summary,
            semantic_off: self.semantic_off,
            verbosity: self.verbosity,
            verbose_macro_warnings: self.verbose_macro_warnings,
            show_macro_stats: self.show_macro_stats,
            group_by_category: self.group_by_category,
            min_priority: self.min_priority,
            min_score: self.min_score,
            filter_categories: self.filter_categories,
            no_context_aware: self.no_context_aware,
            threshold_preset: self.threshold_preset,
            _formatting_config: self._formatting_config,
            parallel: self.parallel,
            jobs: self.jobs,
            multi_pass: self.multi_pass,
            show_attribution: self.show_attribution,
            detail_level: self.detail_level,
            aggregate_only: self.aggregate_only,
            no_aggregation: self.no_aggregation,
            aggregation_method: self.aggregation_method,
            min_problematic: self.min_problematic,
            no_god_object: self.no_god_object,
            max_files: self.max_files,
            validate_loc: self.validate_loc,
            no_public_api_detection: self.no_public_api_detection,
            public_api_threshold: self.public_api_threshold,
            no_pattern_detection: self.no_pattern_detection,
            patterns: self.patterns,
            pattern_threshold: self.pattern_threshold,
            show_pattern_warnings: self.show_pattern_warnings,
            debug_call_graph: self.debug_call_graph,
            trace_functions: self.trace_functions,
            call_graph_stats_only: self.call_graph_stats_only,
            debug_format: self.debug_format,
            validate_call_graph: self.validate_call_graph,
            show_dependencies: self.show_dependencies,
            no_dependencies: self.no_dependencies,
            max_callers: self.max_callers,
            max_callees: self.max_callees,
            show_external: self.show_external,
            show_std_lib: self.show_std_lib,
            ast_functional_analysis: self.ast_functional_analysis,
            functional_analysis_profile: self.functional_analysis_profile,
            min_split_methods: self.min_split_methods,
            min_split_lines: self.min_split_lines,
            no_tui: self.no_tui,
            show_filter_stats: self.show_filter_stats,
        };

        super::analyze::handle_analyze(old_config).context("Analysis execution failed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_transition() {
        // Create unvalidated config
        let config = AnalyzeConfig::new(
            PathBuf::from("."),
            cli::OutputFormat::Terminal,
            None,
            10,
            5,
            None,
            None,
            false,
            None,
            None,
            None,
            None,
            false,
            false,
            0,
            false,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            FormattingConfig::default(),
            true,
            4,
            false,
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            None,
            false,
            false,
            0.5,
            false,
            None,
            0.5,
            false,
            false,
            None,
            false,
            cli::DebugFormatArg::Text,
            false,
            false,
            false,
            100,
            100,
            false,
            false,
            false,
            None,
            3,
            20,
            false,
            false,
        );

        // Validate should succeed for current directory
        let validated = config.validate();
        assert!(validated.is_ok());
    }

    #[test]
    fn test_validation_fails_for_nonexistent_path() {
        let config = AnalyzeConfig::new(
            PathBuf::from("/nonexistent/path/that/should/not/exist"),
            cli::OutputFormat::Terminal,
            None,
            10,
            5,
            None,
            None,
            false,
            None,
            None,
            None,
            None,
            false,
            false,
            0,
            false,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            FormattingConfig::default(),
            true,
            4,
            false,
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            None,
            false,
            false,
            0.5,
            false,
            None,
            0.5,
            false,
            false,
            None,
            false,
            cli::DebugFormatArg::Text,
            false,
            false,
            false,
            100,
            100,
            false,
            false,
            false,
            None,
            3,
            20,
            false,
            false,
        );

        // Validation should fail
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_fails_for_zero_complexity() {
        let config = AnalyzeConfig::new(
            PathBuf::from("."),
            cli::OutputFormat::Terminal,
            None,
            0, // Invalid: zero complexity
            5,
            None,
            None,
            false,
            None,
            None,
            None,
            None,
            false,
            false,
            0,
            false,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            FormattingConfig::default(),
            true,
            4,
            false,
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            None,
            false,
            false,
            0.5,
            false,
            None,
            0.5,
            false,
            false,
            None,
            false,
            cli::DebugFormatArg::Text,
            false,
            false,
            false,
            100,
            100,
            false,
            false,
            false,
            None,
            3,
            20,
            false,
            false,
        );

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_fails_for_invalid_score() {
        let config = AnalyzeConfig::new(
            PathBuf::from("."),
            cli::OutputFormat::Terminal,
            None,
            10,
            5,
            None,
            None,
            false,
            None,
            None,
            None,
            None,
            false,
            false,
            0,
            false,
            false,
            false,
            None,
            Some(150.0), // Invalid: > 100
            None,
            false,
            None,
            FormattingConfig::default(),
            true,
            4,
            false,
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            None,
            false,
            false,
            0.5,
            false,
            None,
            0.5,
            false,
            false,
            None,
            false,
            cli::DebugFormatArg::Text,
            false,
            false,
            false,
            100,
            100,
            false,
            false,
            false,
            None,
            3,
            20,
            false,
            false,
        );

        let result = config.validate();
        assert!(result.is_err());
    }
}

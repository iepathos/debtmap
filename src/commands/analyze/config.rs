//! Configuration module for the analyze command.
//!
//! This module contains the `AnalyzeConfig` struct and environment setup functions.
//! It follows the "Shell" pattern - handling I/O for environment configuration.

use crate::{
    cli, formatting::FormattingConfig, progress::ProgressConfig, progress::ProgressManager,
};
use std::path::PathBuf;

/// Configuration for the analyze command.
pub struct AnalyzeConfig {
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
}

impl AnalyzeConfig {
    /// Check if diagnostics mode is enabled.
    pub fn needs_diagnostics(&self) -> bool {
        self.debug_call_graph || self.validate_call_graph || self.call_graph_stats_only
    }
}

/// Set up environment based on configuration (I/O).
pub fn setup_environment(config: &AnalyzeConfig) {
    configure_output(config);
    set_threshold_preset(config.threshold_preset);
    setup_env_vars(config);
}

/// Configure output color settings.
fn configure_output(config: &AnalyzeConfig) {
    if config._formatting_config.color.should_use_color() {
        colored::control::set_override(true);
    } else {
        colored::control::set_override(false);
    }
}

/// Set threshold preset environment variable.
fn set_threshold_preset(preset: Option<cli::ThresholdPreset>) {
    if let Some(preset) = preset {
        let value = match preset {
            cli::ThresholdPreset::Strict => "strict",
            cli::ThresholdPreset::Balanced => "balanced",
            cli::ThresholdPreset::Lenient => "lenient",
        };
        std::env::set_var("DEBTMAP_THRESHOLD_PRESET", value);
    }
}

/// Set up environment variables from configuration.
fn setup_env_vars(config: &AnalyzeConfig) {
    setup_max_files(config.max_files);
    setup_min_score(config.min_score);
    setup_jobs(config.jobs);
    setup_functional_analysis(config);
}

/// Set max files environment variable if specified.
fn setup_max_files(max_files: Option<usize>) {
    if let Some(max_files) = max_files {
        std::env::set_var("DEBTMAP_MAX_FILES", max_files.to_string());
    }
}

/// Set minimum score threshold environment variable if specified.
fn setup_min_score(min_score: Option<f64>) {
    if let Some(min_score) = min_score {
        std::env::set_var("DEBTMAP_MIN_SCORE_THRESHOLD", min_score.to_string());
    }
}

/// Set jobs environment variable for parallel processing.
fn setup_jobs(jobs: usize) {
    if jobs > 0 {
        std::env::set_var("DEBTMAP_JOBS", jobs.to_string());
    }
}

/// Set functional analysis environment variables.
fn setup_functional_analysis(config: &AnalyzeConfig) {
    if !config.ast_functional_analysis {
        return;
    }

    std::env::set_var("DEBTMAP_FUNCTIONAL_ANALYSIS", "true");

    if let Some(profile) = config.functional_analysis_profile {
        let profile_str = match profile {
            cli::FunctionalAnalysisProfile::Strict => "strict",
            cli::FunctionalAnalysisProfile::Balanced => "balanced",
            cli::FunctionalAnalysisProfile::Lenient => "lenient",
        };
        std::env::set_var("DEBTMAP_FUNCTIONAL_ANALYSIS_PROFILE", profile_str);
    }
}

/// Initialize global progress manager with TUI support (I/O).
pub fn setup_progress_manager(verbosity: u8) {
    let quiet = std::env::var("DEBTMAP_QUIET").is_ok();
    let progress_config = ProgressConfig::from_env(quiet, verbosity);
    ProgressManager::init_global(progress_config);

    // Start TUI rendering if available
    if let Some(manager) = ProgressManager::global() {
        manager.tui_start_stage(0);
    }
}

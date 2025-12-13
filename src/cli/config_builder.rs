//! Configuration builders for CLI commands
//!
//! This module contains configuration structs and builder functions for
//! converting CLI arguments into typed configuration objects.

use crate::cli::args::{DebugFormatArg, FunctionalAnalysisProfile, OutputFormat, ThresholdPreset};
use crate::formatting::FormattingConfig;
use std::path::PathBuf;

/// Path and file configuration for analysis
#[derive(Debug, Clone)]
pub struct PathConfig {
    pub path: PathBuf,
    pub output: Option<PathBuf>,
    pub coverage_file: Option<PathBuf>,
    pub max_files: Option<usize>,
    pub min_priority: Option<String>,
    pub min_score: Option<f64>,
    pub filter_categories: Option<Vec<String>>,
    pub min_problematic: Option<usize>,
}

impl PathConfig {
    #[allow(dead_code)]
    pub fn builder(path: PathBuf) -> PathConfigBuilder {
        PathConfigBuilder {
            path,
            output: None,
            coverage_file: None,
            max_files: None,
            min_priority: None,
            min_score: None,
            filter_categories: None,
            min_problematic: None,
        }
    }
}

#[allow(dead_code)]
pub struct PathConfigBuilder {
    path: PathBuf,
    output: Option<PathBuf>,
    coverage_file: Option<PathBuf>,
    max_files: Option<usize>,
    min_priority: Option<String>,
    min_score: Option<f64>,
    filter_categories: Option<Vec<String>>,
    min_problematic: Option<usize>,
}

#[allow(dead_code)]
impl PathConfigBuilder {
    pub fn output(mut self, output: Option<PathBuf>) -> Self {
        self.output = output;
        self
    }
    pub fn coverage_file(mut self, file: Option<PathBuf>) -> Self {
        self.coverage_file = file;
        self
    }
    pub fn max_files(mut self, max: Option<usize>) -> Self {
        self.max_files = max;
        self
    }
    pub fn min_priority(mut self, priority: Option<String>) -> Self {
        self.min_priority = priority;
        self
    }
    pub fn min_score(mut self, score: Option<f64>) -> Self {
        self.min_score = score;
        self
    }
    pub fn filter_categories(mut self, categories: Option<Vec<String>>) -> Self {
        self.filter_categories = categories;
        self
    }
    pub fn min_problematic(mut self, min: Option<usize>) -> Self {
        self.min_problematic = min;
        self
    }
    pub fn build(self) -> PathConfig {
        PathConfig {
            path: self.path,
            output: self.output,
            coverage_file: self.coverage_file,
            max_files: self.max_files,
            min_priority: self.min_priority,
            min_score: self.min_score,
            filter_categories: self.filter_categories,
            min_problematic: self.min_problematic,
        }
    }
}

/// Analysis thresholds configuration
#[derive(Debug, Clone)]
pub struct ThresholdConfig {
    pub complexity: u32,
    pub duplication: usize,
    pub preset: Option<ThresholdPreset>,
    pub public_api_threshold: f32,
}

impl ThresholdConfig {
    #[allow(dead_code)]
    pub fn builder(complexity: u32, duplication: usize) -> ThresholdConfigBuilder {
        ThresholdConfigBuilder {
            complexity,
            duplication,
            preset: None,
            public_api_threshold: 0.5,
        }
    }
}

#[allow(dead_code)]
pub struct ThresholdConfigBuilder {
    complexity: u32,
    duplication: usize,
    preset: Option<ThresholdPreset>,
    public_api_threshold: f32,
}

#[allow(dead_code)]
impl ThresholdConfigBuilder {
    pub fn preset(mut self, preset: Option<ThresholdPreset>) -> Self {
        self.preset = preset;
        self
    }
    pub fn public_api_threshold(mut self, threshold: f32) -> Self {
        self.public_api_threshold = threshold;
        self
    }
    pub fn build(self) -> ThresholdConfig {
        ThresholdConfig {
            complexity: self.complexity,
            duplication: self.duplication,
            preset: self.preset,
            public_api_threshold: self.public_api_threshold,
        }
    }
}

/// Feature flags for analysis options
#[derive(Debug, Clone)]
pub struct AnalysisFeatureConfig {
    pub enable_context: bool,
    pub context_providers: Option<Vec<String>>,
    pub disable_context: Option<Vec<String>>,
    pub semantic_off: bool,
    pub no_pattern_detection: bool,
    pub patterns: Option<Vec<String>>,
    pub pattern_threshold: f32,
    pub no_god_object: bool,
    pub no_public_api_detection: bool,
    pub ast_functional_analysis: bool,
    pub functional_analysis_profile: Option<FunctionalAnalysisProfile>,
    pub min_split_methods: usize,
    pub min_split_lines: usize,
    pub validate_loc: bool,
    pub validate_call_graph: bool,
}

impl AnalysisFeatureConfig {
    #[allow(dead_code)]
    pub fn builder() -> AnalysisFeatureConfigBuilder {
        AnalysisFeatureConfigBuilder::default()
    }
}

#[derive(Default)]
#[allow(dead_code)]
pub struct AnalysisFeatureConfigBuilder {
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
    semantic_off: bool,
    no_pattern_detection: bool,
    patterns: Option<Vec<String>>,
    pattern_threshold: f32,
    no_god_object: bool,
    no_public_api_detection: bool,
    ast_functional_analysis: bool,
    functional_analysis_profile: Option<FunctionalAnalysisProfile>,
    min_split_methods: usize,
    min_split_lines: usize,
    validate_loc: bool,
    validate_call_graph: bool,
}

#[allow(dead_code)]
impl AnalysisFeatureConfigBuilder {
    pub fn enable_context(mut self, enable: bool) -> Self {
        self.enable_context = enable;
        self
    }
    pub fn context_providers(mut self, providers: Option<Vec<String>>) -> Self {
        self.context_providers = providers;
        self
    }
    pub fn disable_context(mut self, disable: Option<Vec<String>>) -> Self {
        self.disable_context = disable;
        self
    }
    pub fn semantic_off(mut self, off: bool) -> Self {
        self.semantic_off = off;
        self
    }
    pub fn no_pattern_detection(mut self, no: bool) -> Self {
        self.no_pattern_detection = no;
        self
    }
    pub fn patterns(mut self, patterns: Option<Vec<String>>) -> Self {
        self.patterns = patterns;
        self
    }
    pub fn pattern_threshold(mut self, threshold: f32) -> Self {
        self.pattern_threshold = threshold;
        self
    }
    pub fn no_god_object(mut self, no: bool) -> Self {
        self.no_god_object = no;
        self
    }
    pub fn no_public_api_detection(mut self, no: bool) -> Self {
        self.no_public_api_detection = no;
        self
    }
    pub fn ast_functional_analysis(mut self, enable: bool) -> Self {
        self.ast_functional_analysis = enable;
        self
    }
    pub fn functional_analysis_profile(
        mut self,
        profile: Option<FunctionalAnalysisProfile>,
    ) -> Self {
        self.functional_analysis_profile = profile;
        self
    }
    pub fn min_split_methods(mut self, min: usize) -> Self {
        self.min_split_methods = min;
        self
    }
    pub fn min_split_lines(mut self, min: usize) -> Self {
        self.min_split_lines = min;
        self
    }
    pub fn validate_loc(mut self, validate: bool) -> Self {
        self.validate_loc = validate;
        self
    }
    pub fn validate_call_graph(mut self, validate: bool) -> Self {
        self.validate_call_graph = validate;
        self
    }
    pub fn build(self) -> AnalysisFeatureConfig {
        AnalysisFeatureConfig {
            enable_context: self.enable_context,
            context_providers: self.context_providers,
            disable_context: self.disable_context,
            semantic_off: self.semantic_off,
            no_pattern_detection: self.no_pattern_detection,
            patterns: self.patterns,
            pattern_threshold: self.pattern_threshold,
            no_god_object: self.no_god_object,
            no_public_api_detection: self.no_public_api_detection,
            ast_functional_analysis: self.ast_functional_analysis,
            functional_analysis_profile: self.functional_analysis_profile,
            min_split_methods: self.min_split_methods,
            min_split_lines: self.min_split_lines,
            validate_loc: self.validate_loc,
            validate_call_graph: self.validate_call_graph,
        }
    }
}

/// Display and output formatting configuration
#[derive(Debug, Clone)]
pub struct DisplayConfig {
    pub format: OutputFormat,
    pub verbosity: u8,
    pub summary: bool,
    pub top: Option<usize>,
    pub tail: Option<usize>,
    pub group_by_category: bool,
    pub show_attribution: bool,
    pub detail_level: Option<String>,
    pub no_tui: bool,
    pub show_filter_stats: bool,
    pub formatting_config: FormattingConfig,
    pub no_context_aware: bool,
}

/// Performance and parallelization settings
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    pub parallel: bool,
    pub jobs: usize,
    pub multi_pass: bool,
    pub aggregate_only: bool,
    pub no_aggregation: bool,
}

/// Debug and diagnostic settings
#[derive(Debug, Clone)]
pub struct DebugConfig {
    pub verbose_macro_warnings: bool,
    pub show_macro_stats: bool,
    pub debug_call_graph: bool,
    pub trace_functions: Option<Vec<String>>,
    pub call_graph_stats_only: bool,
    pub debug_format: DebugFormatArg,
    pub show_pattern_warnings: bool,
    pub show_dependencies: bool,
    pub no_dependencies: bool,
}

/// Language-specific settings
#[derive(Debug, Clone)]
pub struct LanguageConfig {
    pub languages: Option<Vec<String>>,
    pub aggregation_method: Option<String>,
    pub max_callers: usize,
    pub max_callees: usize,
    pub show_external: bool,
    pub show_std_lib: bool,
}

// ============================================================================
// Pure builder functions for configuration groups
// ============================================================================

/// Builds PathConfig from command-line parameters (pure, spec 182)
#[allow(clippy::too_many_arguments)]
pub fn build_path_config(
    path: PathBuf,
    output: Option<PathBuf>,
    coverage_file: Option<PathBuf>,
    max_files: Option<usize>,
    min_priority: Option<String>,
    min_score: Option<f64>,
    filter_categories: Option<Vec<String>>,
    min_problematic: Option<usize>,
) -> PathConfig {
    PathConfig {
        path,
        output,
        coverage_file,
        max_files,
        min_priority,
        min_score,
        filter_categories,
        min_problematic,
    }
}

/// Builds ThresholdConfig from command-line parameters (pure, spec 182)
pub fn build_threshold_config(
    complexity: u32,
    duplication: usize,
    preset: Option<ThresholdPreset>,
    public_api_threshold: f32,
) -> ThresholdConfig {
    ThresholdConfig {
        complexity,
        duplication,
        preset,
        public_api_threshold,
    }
}

/// Builds AnalysisFeatureConfig from command-line parameters (pure, spec 182)
#[allow(clippy::too_many_arguments)]
pub fn build_feature_config(
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
    semantic_off: bool,
    no_pattern_detection: bool,
    patterns: Option<Vec<String>>,
    pattern_threshold: f32,
    no_god_object: bool,
    no_public_api_detection: bool,
    ast_functional_analysis: bool,
    functional_analysis_profile: Option<FunctionalAnalysisProfile>,
    min_split_methods: usize,
    min_split_lines: usize,
    validate_loc: bool,
    validate_call_graph: bool,
) -> AnalysisFeatureConfig {
    AnalysisFeatureConfig {
        enable_context,
        context_providers,
        disable_context,
        semantic_off,
        no_pattern_detection,
        patterns,
        pattern_threshold,
        no_god_object,
        no_public_api_detection,
        ast_functional_analysis,
        functional_analysis_profile,
        min_split_methods,
        min_split_lines,
        validate_loc,
        validate_call_graph,
    }
}

/// Builds DisplayConfig from command-line parameters (pure, spec 182)
#[allow(clippy::too_many_arguments)]
pub fn build_display_config(
    format: OutputFormat,
    verbosity: u8,
    summary: bool,
    top: Option<usize>,
    tail: Option<usize>,
    group_by_category: bool,
    show_attribution: bool,
    detail_level: Option<String>,
    no_tui: bool,
    show_filter_stats: bool,
    formatting_config: FormattingConfig,
    no_context_aware: bool,
) -> DisplayConfig {
    DisplayConfig {
        format,
        verbosity,
        summary,
        top,
        tail,
        group_by_category,
        show_attribution,
        detail_level,
        no_tui,
        show_filter_stats,
        formatting_config,
        no_context_aware,
    }
}

/// Builds PerformanceConfig from command-line parameters (pure, spec 182)
pub fn build_performance_config(
    parallel: bool,
    jobs: usize,
    multi_pass: bool,
    aggregate_only: bool,
    no_aggregation: bool,
) -> PerformanceConfig {
    PerformanceConfig {
        parallel,
        jobs,
        multi_pass,
        aggregate_only,
        no_aggregation,
    }
}

/// Builds DebugConfig from command-line parameters (pure, spec 182)
#[allow(clippy::too_many_arguments)]
pub fn build_debug_config(
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    debug_call_graph: bool,
    trace_functions: Option<Vec<String>>,
    call_graph_stats_only: bool,
    debug_format: DebugFormatArg,
    show_pattern_warnings: bool,
    show_dependencies: bool,
    no_dependencies: bool,
) -> DebugConfig {
    DebugConfig {
        verbose_macro_warnings,
        show_macro_stats,
        debug_call_graph,
        trace_functions,
        call_graph_stats_only,
        debug_format,
        show_pattern_warnings,
        show_dependencies,
        no_dependencies,
    }
}

/// Builds LanguageConfig from command-line parameters (pure, spec 182)
pub fn build_language_config(
    languages: Option<Vec<String>>,
    aggregation_method: Option<String>,
    max_callers: usize,
    max_callees: usize,
    show_external: bool,
    show_std_lib: bool,
) -> LanguageConfig {
    LanguageConfig {
        languages,
        aggregation_method,
        max_callers,
        max_callees,
        show_external,
        show_std_lib,
    }
}

// ============================================================================
// Pure conversion functions
// ============================================================================

/// Pure function to determine parallel mode
pub fn should_use_parallel(no_parallel: bool) -> bool {
    !no_parallel
}

/// Pure function to compute effective verbosity (spec 204)
pub fn compute_verbosity(verbosity: u8, compact: bool) -> u8 {
    if compact {
        0 // Compact mode uses minimum verbosity
    } else {
        verbosity
    }
}

/// Pure function to check if single-pass mode is enabled via env var (spec 202)
pub fn is_single_pass_env_enabled() -> bool {
    std::env::var("DEBTMAP_SINGLE_PASS")
        .ok()
        .and_then(|v| v.parse::<bool>().ok().or_else(|| Some(v == "1")))
        .unwrap_or(false)
}

/// Pure function to compute multi-pass setting (spec 204)
pub fn compute_multi_pass(no_multi_pass: bool) -> bool {
    if no_multi_pass {
        return false;
    }
    !is_single_pass_env_enabled()
}

/// Pure function to create formatting configuration
#[allow(clippy::too_many_arguments)]
pub fn create_formatting_config(
    plain: bool,
    _show_dependencies: bool,
    _no_dependencies: bool,
    max_callers: usize,
    max_callees: usize,
    show_external: bool,
    show_std_lib: bool,
    show_splits: bool,
) -> FormattingConfig {
    use crate::config::CallerCalleeConfig;
    use crate::formatting::ColorMode;

    let color_mode = if plain {
        ColorMode::Never
    } else {
        // Get color mode from environment
        let base_config = FormattingConfig::from_env();
        base_config.color
    };

    let caller_callee = CallerCalleeConfig {
        max_callers,
        max_callees,
        show_external,
        show_std_lib,
    };

    FormattingConfig::with_caller_callee(color_mode, caller_callee).with_show_splits(show_splits)
}

// ============================================================================
// Conversion helper functions
// ============================================================================

pub fn convert_min_priority(priority: Option<String>) -> Option<String> {
    priority
}

pub fn convert_filter_categories(categories: Option<Vec<String>>) -> Option<Vec<String>> {
    categories.filter(|v| !v.is_empty())
}

pub fn convert_context_providers(providers: Option<Vec<String>>) -> Option<Vec<String>> {
    providers.filter(|v| !v.is_empty())
}

pub fn convert_disable_context(disable_context: Option<Vec<String>>) -> Option<Vec<String>> {
    disable_context
}

pub fn convert_languages(languages: Option<Vec<String>>) -> Option<Vec<String>> {
    languages.filter(|v| !v.is_empty())
}

pub fn convert_threshold_preset(preset: Option<ThresholdPreset>) -> Option<ThresholdPreset> {
    preset
}

pub fn convert_output_format(format: OutputFormat) -> OutputFormat {
    format
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_verbosity_compact() {
        assert_eq!(compute_verbosity(1, true), 0);
    }

    #[test]
    fn test_compute_verbosity_explicit() {
        assert_eq!(compute_verbosity(2, false), 2);
    }

    #[test]
    fn test_compute_verbosity_default() {
        assert_eq!(compute_verbosity(1, false), 1);
    }

    #[test]
    fn test_compute_multi_pass_disabled() {
        assert!(!compute_multi_pass(true));
    }

    #[test]
    fn test_compute_multi_pass_enabled() {
        std::env::remove_var("DEBTMAP_SINGLE_PASS");
        assert!(compute_multi_pass(false));
    }

    #[test]
    fn test_is_single_pass_env_disabled() {
        std::env::remove_var("DEBTMAP_SINGLE_PASS");
        assert!(!is_single_pass_env_enabled());
    }

    #[test]
    fn test_is_single_pass_env_true() {
        std::env::set_var("DEBTMAP_SINGLE_PASS", "true");
        assert!(is_single_pass_env_enabled());
        std::env::remove_var("DEBTMAP_SINGLE_PASS");
    }

    #[test]
    fn test_is_single_pass_env_numeric() {
        std::env::set_var("DEBTMAP_SINGLE_PASS", "1");
        assert!(is_single_pass_env_enabled());
        std::env::remove_var("DEBTMAP_SINGLE_PASS");
    }

    #[test]
    fn test_path_config_builder() {
        let config = PathConfig::builder(PathBuf::from("/test"))
            .output(Some(PathBuf::from("/output")))
            .max_files(Some(100))
            .build();
        assert_eq!(config.path, PathBuf::from("/test"));
        assert_eq!(config.output, Some(PathBuf::from("/output")));
        assert_eq!(config.max_files, Some(100));
    }

    #[test]
    fn test_threshold_config_builder() {
        let config = ThresholdConfig::builder(10, 50)
            .public_api_threshold(0.8)
            .build();
        assert_eq!(config.complexity, 10);
        assert_eq!(config.duplication, 50);
        assert_eq!(config.public_api_threshold, 0.8);
    }

    #[test]
    fn test_analysis_feature_config_builder() {
        let config = AnalysisFeatureConfig::builder()
            .enable_context(true)
            .semantic_off(false)
            .ast_functional_analysis(true)
            .build();
        assert!(config.enable_context);
        assert!(!config.semantic_off);
        assert!(config.ast_functional_analysis);
    }

    #[test]
    fn test_convert_filter_categories_empty() {
        assert_eq!(convert_filter_categories(Some(vec![])), None);
    }

    #[test]
    fn test_convert_filter_categories_non_empty() {
        let cats = vec!["test".to_string()];
        assert_eq!(convert_filter_categories(Some(cats.clone())), Some(cats));
    }

    #[test]
    fn test_convert_languages_empty() {
        assert_eq!(convert_languages(Some(vec![])), None);
    }

    #[test]
    fn test_convert_languages_non_empty() {
        let langs = vec!["rust".to_string()];
        assert_eq!(convert_languages(Some(langs.clone())), Some(langs));
    }

    #[test]
    fn test_should_use_parallel() {
        assert!(should_use_parallel(false));
        assert!(!should_use_parallel(true));
    }
}

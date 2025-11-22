use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ThresholdPreset {
    /// Strict thresholds for high code quality standards
    Strict,
    /// Balanced thresholds for typical projects (default)
    Balanced,
    /// Lenient thresholds for legacy or complex domains
    Lenient,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DebugFormatArg {
    /// Human-readable text format
    Text,
    /// JSON format for programmatic analysis
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FunctionalAnalysisProfile {
    /// Strict profile for codebases emphasizing functional purity
    Strict,
    /// Balanced profile for typical Rust codebases (default)
    Balanced,
    /// Lenient profile for imperative-heavy codebases
    Lenient,
}

#[derive(Parser, Debug)]
#[command(name = "debtmap")]
#[command(about = "Code complexity and technical debt analyzer", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
    /// Analyze code for complexity and technical debt
    Analyze {
        /// Path to analyze
        path: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "terminal")]
        format: OutputFormat,

        /// JSON output structure format (legacy or unified)
        /// 'legacy': Current format with {File: {...}} and {Function: {...}} wrappers
        /// 'unified': New format with consistent structure and 'type' field (spec 108)
        #[arg(long = "output-format", value_enum, default_value = "legacy")]
        json_format: JsonFormat,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Complexity threshold
        #[arg(long, default_value = "10")]
        threshold_complexity: u32,

        /// Duplication threshold (lines)
        #[arg(long, default_value = "50")]
        threshold_duplication: usize,

        /// Languages to analyze
        #[arg(long, value_delimiter = ',')]
        languages: Option<Vec<String>>,

        /// LCOV coverage file for risk analysis and score dampening.
        /// Coverage data dampens debt scores for well-tested code (multiplier = 1.0 - coverage),
        /// surfacing untested complex functions. Total debt score with coverage ≤ score without.
        #[arg(long = "coverage-file", visible_alias = "lcov")]
        coverage_file: Option<PathBuf>,

        /// Enable context-aware risk analysis
        #[arg(long = "context", visible_alias = "enable-context")]
        enable_context: bool,

        /// Context providers to use (critical_path, dependency, git_history)
        #[arg(long = "context-providers", value_delimiter = ',')]
        context_providers: Option<Vec<String>>,

        /// Disable specific context providers
        #[arg(long = "disable-context", value_delimiter = ',')]
        disable_context: Option<Vec<String>>,

        /// Show only top N priority items
        #[arg(long = "top", visible_alias = "head")]
        top: Option<usize>,

        /// Show only bottom N priority items (lowest priority)
        #[arg(long = "tail")]
        tail: Option<usize>,

        /// Use summary format with tiered priority display (compact output)
        #[arg(long = "summary", short = 's')]
        summary: bool,

        /// Disable semantic analysis (fallback mode)
        #[arg(long = "semantic-off")]
        semantic_off: bool,

        /// Show score breakdown for debugging (deprecated: use -v instead)
        #[arg(long = "explain-score", hide = true)]
        explain_score: bool,

        /// Increase verbosity level (can be repeated: -v, -vv, -vvv)
        /// -v: Show main score factors
        /// -vv: Show detailed calculations
        /// -vvv: Show all debug information
        #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
        verbosity: u8,

        /// Use compact output format (minimal details, top metrics only)
        #[arg(short = 'c', long = "compact", conflicts_with = "verbosity")]
        compact: bool,

        /// Show verbose macro parsing warnings
        #[arg(long = "verbose-macro-warnings")]
        verbose_macro_warnings: bool,

        /// Show macro expansion statistics at the end of analysis
        #[arg(long = "show-macro-stats")]
        show_macro_stats: bool,

        /// Group output by debt category
        #[arg(long = "group-by-category")]
        group_by_category: bool,

        /// Minimum priority to display (low, medium, high, critical)
        #[arg(long = "min-priority")]
        min_priority: Option<String>,

        /// Filter by debt categories (comma-separated)
        #[arg(long = "filter", value_delimiter = ',')]
        filter_categories: Option<Vec<String>>,

        /// Disable context-aware false positive reduction (enabled by default)
        #[arg(long = "no-context-aware")]
        no_context_aware: bool,

        /// Complexity threshold preset (strict, balanced, lenient)
        #[arg(long = "threshold-preset", value_enum)]
        threshold_preset: Option<ThresholdPreset>,

        /// Plain output mode: ASCII-only, no colors, no emoji, machine-parseable
        #[arg(long = "plain")]
        plain: bool,

        /// Disable parallel call graph construction (enabled by default)
        #[arg(long = "no-parallel")]
        no_parallel: bool,

        /// Number of threads for parallel processing (0 = use all cores)
        #[arg(long = "jobs", short = 'j', default_value = "0")]
        jobs: usize,

        /// Enable caching (deprecated: caching is now enabled by default)
        #[arg(long = "cache", hide = true)]
        use_cache: bool,

        /// Disable caching for this run (caching is enabled by default)
        #[arg(long = "no-cache")]
        no_cache: bool,

        /// Clear cache before running analysis
        #[arg(long = "clear-cache")]
        clear_cache: bool,

        /// Force cache rebuild (same as --clear-cache)
        #[arg(long = "force-cache-rebuild", conflicts_with = "clear_cache")]
        force_cache_rebuild: bool,

        /// Show cache statistics and location
        #[arg(long = "cache-stats")]
        cache_stats: bool,

        /// Migrate cache from local to shared location
        #[arg(long = "migrate-cache")]
        migrate_cache: bool,

        /// Cache location strategy (local, shared, or path)
        #[arg(long = "cache-location", env = "DEBTMAP_CACHE_DIR")]
        cache_location: Option<String>,

        /// Enable multi-pass analysis with attribution
        #[arg(long = "multi-pass")]
        multi_pass: bool,

        /// Show complexity attribution details
        #[arg(long = "attribution")]
        show_attribution: bool,

        /// Detail level for diagnostic reports (summary, standard, comprehensive, debug)
        #[arg(long = "detail-level", default_value = "standard")]
        detail_level: Option<String>,

        /// Show only aggregated file-level scores
        #[arg(long = "aggregate-only")]
        aggregate_only: bool,

        /// Disable file-level aggregation
        #[arg(long = "no-aggregation")]
        no_aggregation: bool,

        /// File aggregation method (sum, weighted_sum, logarithmic_sum, max_plus_average)
        #[arg(long = "aggregation-method", default_value = "weighted_sum")]
        aggregation_method: Option<String>,

        /// Minimum number of problematic functions for file aggregation
        #[arg(long = "min-problematic")]
        min_problematic: Option<usize>,

        /// Disable god object detection
        #[arg(long = "no-god-object")]
        no_god_object: bool,

        /// Minimum methods per god object split recommendation (Spec 190)
        #[arg(long = "min-split-methods", default_value = "10")]
        min_split_methods: usize,

        /// Minimum lines per god object split recommendation (Spec 190)
        #[arg(long = "min-split-lines", default_value = "150")]
        min_split_lines: usize,

        /// Maximum number of files to analyze (0 = no limit, default: no limit)
        #[arg(long = "max-files")]
        max_files: Option<usize>,

        /// Validate LOC consistency across analysis modes (with/without coverage)
        #[arg(long = "validate-loc")]
        validate_loc: bool,

        /// Disable public API detection heuristics for dead code analysis
        #[arg(long = "no-public-api-detection")]
        no_public_api_detection: bool,

        /// Public API confidence threshold (0.0-1.0) - functions above this are considered public APIs
        #[arg(long = "public-api-threshold", default_value = "0.7")]
        public_api_threshold: f32,

        /// Disable pattern recognition
        #[arg(long = "no-pattern-detection")]
        no_pattern_detection: bool,

        /// Enable specific patterns only (comma-separated: observer,singleton,factory,strategy,callback,template_method)
        #[arg(long = "patterns", value_delimiter = ',')]
        patterns: Option<Vec<String>>,

        /// Pattern confidence threshold (0.0 - 1.0)
        #[arg(long = "pattern-threshold", default_value = "0.7")]
        pattern_threshold: f32,

        /// Show pattern warnings for uncertain detections
        #[arg(long = "show-pattern-warnings")]
        show_pattern_warnings: bool,

        /// Explain metric definitions and formulas (measured vs estimated)
        #[arg(long = "explain-metrics")]
        explain_metrics: bool,

        /// Enable call graph debugging with detailed resolution information
        #[arg(long = "debug-call-graph")]
        debug_call_graph: bool,

        /// Trace specific functions during call resolution (comma-separated)
        #[arg(long = "trace-function", value_delimiter = ',')]
        trace_functions: Option<Vec<String>>,

        /// Show only call graph statistics (no detailed failure list)
        #[arg(long = "call-graph-stats")]
        call_graph_stats_only: bool,

        /// Debug output format (text or json)
        #[arg(long = "debug-format", value_enum, default_value = "text")]
        debug_format: DebugFormatArg,

        /// Validate call graph structure and report issues
        #[arg(long = "validate-call-graph")]
        validate_call_graph: bool,

        /// Show dependency information (callers/callees) in output
        #[arg(long = "show-dependencies")]
        show_dependencies: bool,

        /// Hide dependency information (callers/callees) in output
        #[arg(long = "no-dependencies", conflicts_with = "show_dependencies")]
        no_dependencies: bool,

        /// Maximum number of callers to display (default: 5)
        #[arg(long = "max-callers", default_value = "5")]
        max_callers: usize,

        /// Maximum number of callees to display (default: 5)
        #[arg(long = "max-callees", default_value = "5")]
        max_callees: usize,

        /// Show external crate calls in dependencies
        #[arg(long = "show-external-calls")]
        show_external: bool,

        /// Show standard library calls in dependencies
        #[arg(long = "show-std-lib-calls")]
        show_std_lib: bool,

        /// Enable AST-based functional composition analysis (spec 111)
        #[arg(long = "ast-functional-analysis")]
        ast_functional_analysis: bool,

        /// Functional analysis profile (strict, balanced, lenient)
        #[arg(long = "functional-analysis-profile", value_enum)]
        functional_analysis_profile: Option<FunctionalAnalysisProfile>,
    },

    /// Initialize configuration file
    Init {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },

    /// Validate code against thresholds
    Validate {
        /// Path to analyze
        path: PathBuf,

        /// Configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// LCOV coverage file for risk analysis and score dampening.
        /// Coverage data dampens debt scores for well-tested code (multiplier = 1.0 - coverage),
        /// surfacing untested complex functions. Total debt score with coverage ≤ score without.
        #[arg(long = "coverage-file", visible_alias = "lcov")]
        coverage_file: Option<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum)]
        format: Option<OutputFormat>,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Enable context-aware risk analysis
        #[arg(long = "context", visible_alias = "enable-context")]
        enable_context: bool,

        /// Context providers to use (critical_path, dependency, git_history)
        #[arg(long = "context-providers", value_delimiter = ',')]
        context_providers: Option<Vec<String>>,

        /// Disable specific context providers
        #[arg(long = "disable-context", value_delimiter = ',')]
        disable_context: Option<Vec<String>>,

        /// Maximum debt density allowed (per 1000 LOC)
        #[arg(long = "max-debt-density")]
        max_debt_density: Option<f64>,

        /// Show only top N priority items
        #[arg(long = "top", visible_alias = "head")]
        top: Option<usize>,

        /// Show only bottom N priority items (lowest priority)
        #[arg(long = "tail")]
        tail: Option<usize>,

        /// Use summary format with tiered priority display (compact output)
        #[arg(long = "summary", short = 's')]
        summary: bool,

        /// Disable semantic analysis (fallback mode)
        #[arg(long = "semantic-off")]
        semantic_off: bool,

        /// Show score breakdown for debugging (deprecated: use -v instead)
        #[arg(long = "explain-score", hide = true)]
        explain_score: bool,

        /// Increase verbosity level (can be repeated: -v, -vv, -vvv)
        /// -v: Show main score factors
        /// -vv: Show detailed calculations
        /// -vvv: Show all debug information
        #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
        verbosity: u8,

        /// Disable parallel processing (enabled by default).
        /// Parallel processing utilizes all CPU cores for call graph construction
        /// and unified analysis, providing 70-90% performance improvement on multi-core systems.
        /// Use this flag to force sequential processing for debugging or compatibility.
        #[arg(long = "no-parallel")]
        no_parallel: bool,

        /// Number of threads for parallel processing (0 = use all cores).
        /// Controls thread pool size for parallel call graph construction.
        /// Examples: --jobs 4 (use 4 threads), --jobs 0 (use all available cores).
        /// Environment variable DEBTMAP_JOBS can also be used to set this value.
        #[arg(long = "jobs", short = 'j', default_value = "0")]
        jobs: usize,
    },

    /// Compare two analysis results and generate diff
    Compare {
        /// Path to "before" analysis JSON
        #[arg(long, value_name = "FILE")]
        before: PathBuf,

        /// Path to "after" analysis JSON
        #[arg(long, value_name = "FILE")]
        after: PathBuf,

        /// Path to implementation plan (to extract target location)
        #[arg(long, value_name = "FILE")]
        plan: Option<PathBuf>,

        /// Target location (alternative to --plan)
        /// Format: file:function:line
        #[arg(long, value_name = "LOCATION", conflicts_with = "plan")]
        target_location: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "json")]
        format: OutputFormat,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate technical debt improvement from comparison results
    ValidateImprovement {
        /// Path to comparison JSON file from 'debtmap compare'
        #[arg(long, value_name = "FILE")]
        comparison: PathBuf,

        /// Output file path for validation results
        #[arg(
            long,
            short = 'o',
            value_name = "FILE",
            default_value = ".prodigy/debtmap-validation.json"
        )]
        output: PathBuf,

        /// Path to previous validation for progress tracking
        #[arg(long, value_name = "FILE")]
        previous_validation: Option<PathBuf>,

        /// Improvement threshold percentage (0-100)
        #[arg(long, default_value = "75.0")]
        threshold: f64,

        /// Output format
        #[arg(short, long, value_enum, default_value = "json")]
        format: OutputFormat,

        /// Suppress progress output (automation mode)
        #[arg(long, short = 'q')]
        quiet: bool,
    },

    /// Explain coverage detection for a specific function (debugging tool)
    ExplainCoverage {
        /// Path to the codebase to analyze
        path: PathBuf,

        /// LCOV coverage file
        #[arg(long = "coverage-file", visible_alias = "lcov")]
        coverage_file: PathBuf,

        /// Function name to explain (e.g., "create_auto_commit")
        #[arg(long = "function")]
        function_name: String,

        /// File path containing the function (optional, helps narrow search)
        #[arg(long = "file")]
        file_path: Option<PathBuf>,

        /// Show all attempted matching strategies
        #[arg(long = "verbose", short = 'v')]
        verbose: bool,

        /// Output format
        #[arg(short = 'f', long = "format", value_enum, default_value = "text")]
        format: DebugFormatArg,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OutputFormat {
    Json,
    Markdown,
    Terminal,
    Html,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum JsonFormat {
    /// Legacy format with {File: {...}} and {Function: {...}} wrappers
    Legacy,
    /// Unified format with consistent structure (spec 108)
    Unified,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl From<Priority> for crate::core::Priority {
    fn from(p: Priority) -> Self {
        match p {
            Priority::Low => crate::core::Priority::Low,
            Priority::Medium => crate::core::Priority::Medium,
            Priority::High => crate::core::Priority::High,
            Priority::Critical => crate::core::Priority::Critical,
        }
    }
}

impl From<OutputFormat> for crate::io::output::OutputFormat {
    fn from(f: OutputFormat) -> Self {
        match f {
            OutputFormat::Json => crate::io::output::OutputFormat::Json,
            OutputFormat::Markdown => crate::io::output::OutputFormat::Markdown,
            OutputFormat::Terminal => crate::io::output::OutputFormat::Terminal,
            OutputFormat::Html => crate::io::output::OutputFormat::Html,
        }
    }
}

pub fn parse_args() -> Cli {
    Cli::parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_conversion() {
        // Test conversion from CLI Priority to core Priority
        assert_eq!(
            crate::core::Priority::from(Priority::Low),
            crate::core::Priority::Low
        );
        assert_eq!(
            crate::core::Priority::from(Priority::Medium),
            crate::core::Priority::Medium
        );
        assert_eq!(
            crate::core::Priority::from(Priority::High),
            crate::core::Priority::High
        );
        assert_eq!(
            crate::core::Priority::from(Priority::Critical),
            crate::core::Priority::Critical
        );
    }

    #[test]
    fn test_output_format_conversion() {
        // Test conversion from CLI OutputFormat to io::output::OutputFormat
        assert_eq!(
            crate::io::output::OutputFormat::from(OutputFormat::Json),
            crate::io::output::OutputFormat::Json
        );
        assert_eq!(
            crate::io::output::OutputFormat::from(OutputFormat::Markdown),
            crate::io::output::OutputFormat::Markdown
        );
        assert_eq!(
            crate::io::output::OutputFormat::from(OutputFormat::Terminal),
            crate::io::output::OutputFormat::Terminal
        );
    }

    #[test]
    fn test_cli_parsing_analyze_command() {
        use clap::Parser;

        let args = vec![
            "debtmap",
            "analyze",
            "/test/path",
            "--format",
            "json",
            "--threshold-complexity",
            "15",
            "--threshold-duplication",
            "100",
        ];

        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Analyze {
                path,
                format,
                threshold_complexity,
                threshold_duplication,
                ..
            } => {
                assert_eq!(path, PathBuf::from("/test/path"));
                assert_eq!(format, OutputFormat::Json);
                assert_eq!(threshold_complexity, 15);
                assert_eq!(threshold_duplication, 100);
            }
            _ => panic!("Expected Analyze command"),
        }
    }

    #[test]
    fn test_cli_parsing_init_command() {
        use clap::Parser;

        let args = vec!["debtmap", "init", "--force"];

        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Init { force } => {
                assert!(force);
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_cli_parsing_validate_command() {
        use clap::Parser;

        let args = vec![
            "debtmap",
            "validate",
            "/test/path",
            "--config",
            "/config/path",
        ];

        let cli = Cli::parse_from(args);

        match cli.command {
            Commands::Validate { path, config, .. } => {
                assert_eq!(path, PathBuf::from("/test/path"));
                assert_eq!(config, Some(PathBuf::from("/config/path")));
            }
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_priority_ordering() {
        // Test that Priority enum ordering is correct
        assert!(Priority::Low < Priority::Medium);
        assert!(Priority::Medium < Priority::High);
        assert!(Priority::High < Priority::Critical);
    }

    #[test]
    fn test_output_format_equality() {
        // Test OutputFormat equality
        assert_eq!(OutputFormat::Json, OutputFormat::Json);
        assert_ne!(OutputFormat::Json, OutputFormat::Markdown);
        assert_ne!(OutputFormat::Terminal, OutputFormat::Json);
    }

    #[test]
    fn test_parse_args_wrapper() {
        // Since parse_args() calls Cli::parse() which requires actual CLI args,
        // we'll test it indirectly through the Cli structure
        use clap::Parser;

        // Verify that the parse_args function would work with valid arguments
        let test_args = vec!["debtmap", "analyze", "."];
        let cli = Cli::parse_from(test_args);

        // Verify the CLI was parsed correctly
        match cli.command {
            Commands::Analyze { .. } => {
                // Success - the structure was created properly
            }
            _ => panic!("Expected Analyze command from test args"),
        }
    }
}

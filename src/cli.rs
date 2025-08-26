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

#[derive(Parser, Debug)]
#[command(name = "debtmap")]
#[command(about = "Code complexity and technical debt analyzer", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Analyze code for complexity and technical debt
    Analyze {
        /// Path to analyze
        path: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "terminal")]
        format: OutputFormat,

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

        /// Optional LCOV coverage file for risk analysis
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

        /// Show verbose macro parsing warnings
        #[arg(long = "verbose-macro-warnings")]
        verbose_macro_warnings: bool,

        /// Show macro expansion statistics at the end of analysis
        #[arg(long = "show-macro-stats")]
        show_macro_stats: bool,

        /// Enable enhanced security analysis with additional detectors
        #[arg(long = "security-enhanced")]
        security_enhanced: bool,

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

        /// Optional LCOV coverage file for risk analysis
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

        /// Show only top N priority items
        #[arg(long = "top", visible_alias = "head")]
        top: Option<usize>,

        /// Show only bottom N priority items (lowest priority)
        #[arg(long = "tail")]
        tail: Option<usize>,

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
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OutputFormat {
    Json,
    Markdown,
    Terminal,
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

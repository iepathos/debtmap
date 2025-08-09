use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

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
    },

    /// Analyze only complexity metrics
    Complexity {
        /// Path to analyze
        path: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "terminal")]
        format: OutputFormat,

        /// Complexity threshold
        #[arg(long, default_value = "10")]
        threshold: u32,
    },

    /// Analyze only technical debt
    Debt {
        /// Path to analyze
        path: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "terminal")]
        format: OutputFormat,

        /// Minimum priority to report
        #[arg(long, value_enum)]
        min_priority: Option<Priority>,
    },

    /// Analyze dependencies
    Deps {
        /// Path to analyze
        path: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "terminal")]
        format: OutputFormat,
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

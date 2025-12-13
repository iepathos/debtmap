//! CLI module for debtmap
//!
//! This module provides the command-line interface for debtmap, including:
//! - Argument parsing and validation (`args`)
//! - Command handlers (`commands`)
//! - Configuration building (`config_builder`)
//! - Runtime setup (`setup`)

pub mod args;
pub mod commands;
pub mod config_builder;
pub mod setup;

// Re-export commonly used types for convenience
pub use args::{
    Cli, Commands, DebugFormatArg, FunctionalAnalysisProfile, OutputFormat, Priority,
    ThresholdPreset,
};
pub use commands::{handle_analyze_command, handle_compare_command, handle_validate_command};
pub use config_builder::{
    AnalysisFeatureConfig, DebugConfig, DisplayConfig, LanguageConfig, PathConfig,
    PerformanceConfig, ThresholdConfig,
};
pub use setup::{
    apply_environment_setup, configure_thread_pool, get_worker_count, is_automation_mode,
    print_metrics_explanation, show_config_sources, MAIN_STACK_SIZE,
};

/// Parse CLI arguments using Clap
pub fn parse_args() -> Cli {
    args::parse_args()
}

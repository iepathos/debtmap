//! Debtmap CLI entry point
//!
//! This is the main entry point for the debtmap CLI. It handles:
//! - Thread pool configuration for parallel processing
//! - CLI argument parsing and command dispatching
//! - Top-level error handling
//!
//! The actual command implementations are in `cli::commands`.

use anyhow::Result;
use clap::Parser;
use debtmap::cli::{
    configure_thread_pool, get_worker_count, handle_analyze_command_with_profiling,
    handle_compare_command, handle_explain_coverage_command, handle_validate_command,
    handle_validate_improvement_command, show_config_sources, Cli, Commands, MAIN_STACK_SIZE,
};
use debtmap::di::create_app_container;
use debtmap::observability::{extract_thread_panic_message, init_tracing, install_panic_hook};
use std::sync::Arc;

/// Extract the number of jobs from a command, defaulting to 0 for commands that don't support it.
fn extract_jobs(command: &Commands) -> usize {
    match command {
        Commands::Analyze { jobs, .. } | Commands::Validate { jobs, .. } => *jobs,
        _ => 0,
    }
}

/// Parse CLI arguments, supporting ARGUMENTS environment variable for backward compatibility.
///
/// This is a pure function that encapsulates the CLI parsing logic.
fn parse_cli() -> Cli {
    if let Ok(args_str) = std::env::var("ARGUMENTS") {
        let args: Vec<String> = args_str.split_whitespace().map(String::from).collect();
        let mut full_args = vec![std::env::args()
            .next()
            .unwrap_or_else(|| "debtmap".to_string())];
        full_args.extend(args);
        Cli::parse_from(full_args)
    } else {
        Cli::parse()
    }
}

fn main() -> Result<()> {
    // Install custom panic hook FIRST for structured crash reports (spec 207)
    install_panic_hook();

    // Initialize tracing early for structured logging (spec 208)
    // Controlled by RUST_LOG environment variable (default: warn)
    init_tracing();

    // Spawn the actual main logic on a thread with a larger stack (16MB)
    // to handle deeply nested AST traversals without stack overflow.
    // The default main thread stack is often ~1MB which is insufficient
    // for recursive syn::visit patterns on large/complex Rust files.
    std::thread::Builder::new()
        .stack_size(MAIN_STACK_SIZE)
        .spawn(main_inner)?
        .join()
        .map_err(|e| anyhow::anyhow!("Thread panic: {}", extract_thread_panic_message(&e)))?
}

fn main_inner() -> Result<()> {
    let cli = parse_cli();

    // Configure rayon thread pool early based on CLI arguments
    configure_thread_pool(get_worker_count(extract_jobs(&cli.command)));

    // Handle --show-config-sources flag (spec 201)
    if cli.show_config_sources {
        show_config_sources()?;
        return Ok(());
    }

    // If custom config path provided, set environment variable for loaders
    if let Some(ref config_path) = cli.config {
        std::env::set_var("DEBTMAP_CONFIG", config_path);
    }

    // Create the dependency injection container once at startup
    let _container = Arc::new(create_app_container()?);

    // Dispatch to command handlers
    match cli.command {
        command @ Commands::Analyze { .. } => {
            handle_analyze_command_with_profiling(command)
                .map_err(|e| anyhow::anyhow!("Analyze command failed: {}", e))?;
            Ok(())
        }
        Commands::Init { force } => {
            debtmap::commands::init::init_config(force)?;
            Ok(())
        }
        command @ Commands::Validate { .. } => {
            handle_validate_command(command)?;
            Ok(())
        }
        Commands::Compare {
            before,
            after,
            plan,
            target_location,
            format,
            output,
        } => {
            handle_compare_command(
                &before,
                &after,
                plan.as_deref(),
                target_location,
                format,
                output.as_deref(),
            )?;
            Ok(())
        }
        command @ Commands::ValidateImprovement { .. } => {
            handle_validate_improvement_command(command)?;
            Ok(())
        }
        Commands::DiagnoseCoverage {
            coverage_file,
            format,
        } => {
            debtmap::commands::diagnose_coverage::diagnose_coverage_file(&coverage_file, &format)?;
            Ok(())
        }
        command @ Commands::ExplainCoverage { .. } => {
            handle_explain_coverage_command(command)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use debtmap::cli::Cli;

    /// Helper to parse CLI args and extract the command
    fn parse_command(args: &[&str]) -> Commands {
        let mut full_args = vec!["debtmap"];
        full_args.extend(args);
        Cli::parse_from(full_args).command
    }

    #[test]
    fn test_extract_jobs_from_analyze_command_default() {
        let cmd = parse_command(&["analyze", "."]);
        // Default jobs is 0
        assert_eq!(extract_jobs(&cmd), 0);
    }

    #[test]
    fn test_extract_jobs_from_analyze_command_custom() {
        let cmd = parse_command(&["analyze", ".", "--jobs", "8"]);
        assert_eq!(extract_jobs(&cmd), 8);
    }

    #[test]
    fn test_extract_jobs_from_analyze_command_short_flag() {
        let cmd = parse_command(&["analyze", ".", "-j", "4"]);
        assert_eq!(extract_jobs(&cmd), 4);
    }

    #[test]
    fn test_extract_jobs_from_validate_command_default() {
        let cmd = parse_command(&["validate", "."]);
        // Default jobs is 0
        assert_eq!(extract_jobs(&cmd), 0);
    }

    #[test]
    fn test_extract_jobs_from_validate_command_custom() {
        let cmd = parse_command(&["validate", ".", "--jobs", "16"]);
        assert_eq!(extract_jobs(&cmd), 16);
    }

    #[test]
    fn test_extract_jobs_from_init_command() {
        let cmd = parse_command(&["init"]);
        assert_eq!(extract_jobs(&cmd), 0);
    }

    #[test]
    fn test_extract_jobs_from_compare_command() {
        let cmd = parse_command(&["compare", "--before", "a.json", "--after", "b.json"]);
        assert_eq!(extract_jobs(&cmd), 0);
    }

    #[test]
    fn test_extract_jobs_from_validate_improvement_command() {
        let cmd = parse_command(&["validate-improvement", "--comparison", "comp.json"]);
        assert_eq!(extract_jobs(&cmd), 0);
    }

    #[test]
    fn test_extract_jobs_from_diagnose_coverage_command() {
        let cmd = parse_command(&["diagnose-coverage", "lcov.info"]);
        assert_eq!(extract_jobs(&cmd), 0);
    }

    #[test]
    fn test_extract_jobs_from_explain_coverage_command() {
        let cmd = parse_command(&[
            "explain-coverage",
            ".",
            "--coverage-file",
            "lcov.info",
            "--function",
            "test_func",
        ]);
        assert_eq!(extract_jobs(&cmd), 0);
    }
}

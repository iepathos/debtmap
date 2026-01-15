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
    configure_thread_pool, get_worker_count, handle_analyze_command, handle_compare_command,
    handle_validate_command, is_automation_mode, show_config_sources, Cli, Commands,
    MAIN_STACK_SIZE,
};
use debtmap::di::create_app_container;
use debtmap::observability::{
    enable_profiling, extract_thread_panic_message, get_timing_report, init_tracing,
    install_panic_hook,
};
use std::path::PathBuf;
use std::sync::Arc;

/// Output profiling report to file or stderr.
///
/// This function handles the I/O for profiling output, keeping it separate from
/// the main command dispatch logic.
fn output_profiling_report(output_path: Option<PathBuf>) -> Result<()> {
    let report = get_timing_report();
    match output_path {
        Some(path) => {
            std::fs::write(&path, report.to_json())
                .map_err(|e| anyhow::anyhow!("Failed to write profile output: {}", e))?;
            eprintln!("Profiling data written to: {}", path.display());
        }
        None => {
            eprintln!("{}", report.to_summary());
        }
    }
    Ok(())
}

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
        Commands::Analyze {
            profile,
            profile_output,
            ..
        } => {
            if profile {
                enable_profiling();
            }
            // Re-parse to get full command for handler (profiling options consumed above)
            handle_analyze_command(parse_cli().command)
                .map_err(|e| anyhow::anyhow!("Analyze command failed: {}", e))?;
            if profile {
                output_profiling_report(profile_output)?;
            }
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
        Commands::ValidateImprovement {
            comparison,
            output,
            previous_validation,
            threshold,
            format,
            quiet,
        } => {
            let config = debtmap::commands::validate_improvement::ValidateImprovementConfig {
                comparison_path: comparison,
                output_path: output,
                previous_validation,
                threshold,
                format: format.into(),
                quiet: quiet || is_automation_mode(),
            };
            debtmap::commands::validate_improvement::validate_improvement(config)?;
            Ok(())
        }
        Commands::DiagnoseCoverage {
            coverage_file,
            format,
        } => {
            debtmap::commands::diagnose_coverage::diagnose_coverage_file(&coverage_file, &format)?;
            Ok(())
        }
        Commands::ExplainCoverage {
            path,
            coverage_file,
            function_name,
            file_path,
            verbose,
            format,
        } => {
            let config = debtmap::commands::explain_coverage::ExplainCoverageConfig {
                path,
                coverage_file,
                function_name,
                file_path,
                verbose,
                format: format.into(),
            };
            debtmap::commands::explain_coverage::explain_coverage(config)?;
            Ok(())
        }
    }
}

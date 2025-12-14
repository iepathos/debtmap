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
use std::sync::Arc;

fn main() -> Result<()> {
    // Spawn the actual main logic on a thread with a larger stack (16MB)
    // to handle deeply nested AST traversals without stack overflow.
    // The default main thread stack is often ~1MB which is insufficient
    // for recursive syn::visit patterns on large/complex Rust files.
    std::thread::Builder::new()
        .stack_size(MAIN_STACK_SIZE)
        .spawn(main_inner)?
        .join()
        .map_err(|e| anyhow::anyhow!("Thread panic: {:?}", e))?
}

fn main_inner() -> Result<()> {
    // Support ARGUMENTS environment variable for backward compatibility
    let cli = if let Ok(args_str) = std::env::var("ARGUMENTS") {
        let args: Vec<String> = args_str.split_whitespace().map(String::from).collect();
        let mut full_args = vec![std::env::args()
            .next()
            .unwrap_or_else(|| "debtmap".to_string())];
        full_args.extend(args);
        Cli::parse_from(full_args)
    } else {
        Cli::parse()
    };

    // Configure rayon thread pool early based on CLI arguments
    let jobs = match &cli.command {
        Commands::Analyze { jobs, .. } => *jobs,
        Commands::Validate { jobs, .. } => *jobs,
        _ => 0,
    };
    configure_thread_pool(get_worker_count(jobs));

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
            handle_analyze_command(command)
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
                format: match format {
                    debtmap::cli::OutputFormat::Json => {
                        debtmap::commands::validate_improvement::OutputFormat::Json
                    }
                    debtmap::cli::OutputFormat::Markdown => {
                        debtmap::commands::validate_improvement::OutputFormat::Markdown
                    }
                    debtmap::cli::OutputFormat::Terminal => {
                        debtmap::commands::validate_improvement::OutputFormat::Terminal
                    }
                    debtmap::cli::OutputFormat::Html => {
                        debtmap::commands::validate_improvement::OutputFormat::Terminal
                    }
                    debtmap::cli::OutputFormat::Dot => {
                        debtmap::commands::validate_improvement::OutputFormat::Terminal
                    }
                },
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
                format: match format {
                    debtmap::cli::DebugFormatArg::Text => {
                        debtmap::commands::explain_coverage::DebugFormat::Text
                    }
                    debtmap::cli::DebugFormatArg::Json => {
                        debtmap::commands::explain_coverage::DebugFormat::Json
                    }
                },
            };
            debtmap::commands::explain_coverage::explain_coverage(config)?;
            Ok(())
        }
    }
}

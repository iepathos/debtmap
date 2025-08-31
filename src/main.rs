use debtmap::builders;
use debtmap::cli;
use debtmap::commands;
use debtmap::formatting::{ColorMode, EmojiMode, FormattingConfig};
use debtmap::output;
use debtmap::utils;

use anyhow::Result;
use cli::Commands;
use std::process;

fn main() -> Result<()> {
    let cli = cli::parse_args();

    let result = match cli.command {
        Commands::Analyze {
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
            semantic_off,
            explain_score: _,
            verbosity,
            verbose_macro_warnings,
            show_macro_stats,
            group_by_category,
            min_priority,
            filter_categories,
            no_context_aware,
            threshold_preset,
            plain,
        } => {
            // Enhanced scoring is always enabled (no need for environment variable)

            // Set context-aware environment variable (enabled by default)
            if !no_context_aware {
                std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
            }

            // Parse formatting configuration
            let formatting_config = if plain {
                // Plain mode: no colors, no emoji, ASCII-only
                FormattingConfig::new(ColorMode::Never, EmojiMode::Never)
            } else {
                // Auto-detect from environment
                FormattingConfig::from_env()
            };

            let config = debtmap::commands::analyze::AnalyzeConfig {
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
                semantic_off,
                verbosity,
                verbose_macro_warnings,
                show_macro_stats,
                group_by_category,
                min_priority,
                filter_categories,
                no_context_aware,
                threshold_preset,
                formatting_config,
            };
            debtmap::commands::analyze::handle_analyze(config)
        }
        Commands::Init { force } => debtmap::commands::init::init_config(force),
        Commands::Validate {
            path,
            config,
            coverage_file,
            format,
            output,
            enable_context,
            context_providers,
            disable_context,
            top,
            tail,
            semantic_off,
            explain_score: _,
            verbosity,
        } => {
            let config = debtmap::commands::validate::ValidateConfig {
                path,
                config,
                coverage_file,
                verbosity,
                format,
                output,
                enable_context,
                context_providers,
                disable_context,
                top,
                tail,
                semantic_off,
            };
            debtmap::commands::validate::validate_project(config)
        }
    };

    // Exit with appropriate code based on result
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

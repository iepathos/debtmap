use debtmap::cli;
use debtmap::formatting::{ColorMode, EmojiMode, FormattingConfig};

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
            parallel,
            jobs,
            use_cache,
            no_cache,
            clear_cache,
            cache_stats,
            migrate_cache,
            cache_location,
            multi_pass,
            show_attribution,
            detail_level,
        } => {
            // Enhanced scoring is always enabled (no need for environment variable)

            // Handle cache location environment variable
            if let Some(ref location) = cache_location {
                std::env::set_var("DEBTMAP_CACHE_DIR", location);
            }

            // Handle cache stats display
            if cache_stats {
                let cache = debtmap::cache::SharedCache::new(Some(&path))?;
                let stats = cache.get_full_stats()?;
                println!("{}", stats);
                return Ok(());
            }

            // Handle cache migration
            if migrate_cache {
                let cache = debtmap::cache::SharedCache::new(Some(&path))?;
                let local_cache = path.join(".debtmap_cache");
                if local_cache.exists() {
                    println!("Migrating cache from local to shared location...");
                    cache.migrate_from_local(&local_cache)?;
                    println!("Cache migration complete!");
                    // Optionally remove local cache after successful migration
                    if std::fs::remove_dir_all(&local_cache).is_ok() {
                        println!("Local cache removed.");
                    }
                }
                return Ok(());
            }

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
                parallel,
                jobs,
                use_cache,
                no_cache,
                clear_cache,
                cache_stats: false,   // Already handled above
                migrate_cache: false, // Already handled above
                cache_location: cache_location.clone(),
                multi_pass,
                show_attribution,
                detail_level,
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

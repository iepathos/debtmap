use anyhow::Result;
use clap::Parser;
use debtmap::cli::{Cli, Commands};
use debtmap::formatting::{ColorMode, EmojiMode, FormattingConfig};

// Main orchestrator function
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        command @ Commands::Analyze { .. } => handle_analyze_command(command)?,
        Commands::Init { force } => {
            debtmap::commands::init::init_config(force)?;
            Ok(())
        }
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
            let validate_config = debtmap::commands::validate::ValidateConfig {
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
                verbosity,
            };
            debtmap::commands::validate::validate_project(validate_config)?;
            Ok(())
        }
    }
}

// Pure function to handle analyze command with all its complexity
fn handle_analyze_command(command: Commands) -> Result<Result<()>> {
    if let Commands::Analyze {
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
        no_parallel,
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
        aggregate_only,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
    } = command
    {
        // Apply side effects first
        let cache_location_path = cache_location.as_ref().map(|s| std::path::PathBuf::from(s));
        apply_environment_setup(&cache_location_path, no_context_aware)?;

        // Handle early returns for cache operations
        if let Some(result) = handle_cache_operations(cache_stats, migrate_cache, &path)? {
            return Ok(result);
        }

        // Build configuration from pure data transformation
        let formatting_config = create_formatting_config(plain);
        let config = build_analyze_config(
            path, format, output, threshold_complexity, threshold_duplication,
            languages, coverage_file, enable_context, context_providers,
            disable_context, top, tail, semantic_off, verbosity,
            verbose_macro_warnings, show_macro_stats, group_by_category,
            min_priority, filter_categories, no_context_aware, threshold_preset,
            formatting_config, no_parallel, jobs, use_cache, no_cache,
            clear_cache, cache_location, multi_pass, show_attribution,
            detail_level, aggregate_only, no_aggregation, aggregation_method,
            min_problematic, no_god_object,
        );

        Ok(debtmap::commands::analyze::handle_analyze(config))
    } else {
        Err(anyhow::anyhow!("Invalid command"))
    }
}

// Pure function to check cache operations
fn handle_cache_operations(
    cache_stats: bool,
    migrate_cache: bool,
    path: &std::path::PathBuf,
) -> Result<Option<Result<()>>> {
    if cache_stats {
        return Ok(Some(handle_cache_stats(path)));
    }
    if migrate_cache {
        return Ok(Some(handle_cache_migration(path)?));
    }
    Ok(None)
}

// Side effect handler for cache stats
fn handle_cache_stats(path: &std::path::PathBuf) -> Result<()> {
    let cache = debtmap::cache::SharedCache::new(Some(path))?;
    let stats = cache.get_stats();
    println!("Cache Statistics:");
    println!("  Entries: {}", stats.entry_count);
    println!("  Size: {} bytes", stats.total_size);
    Ok(())
}

// Side effect handler for cache migration
fn handle_cache_migration(path: &std::path::PathBuf) -> Result<Result<()>> {
    use debtmap::cache::CacheStrategy;

    println!("Migrating cache to shared location");

    // Get the shared cache location
    let dst_location = debtmap::cache::CacheLocation::resolve_with_strategy(
        Some(path),
        CacheStrategy::Shared,
    )?;

    // Create a new cache at the shared location
    let _dst_cache = debtmap::cache::SharedCache::new_with_cache_dir(
        Some(path),
        dst_location.get_cache_path().to_path_buf(),
    )?;

    // For now, just report that we've set up the cache in the shared location
    println!("Cache now configured at shared location: {:?}", dst_location.get_cache_path());
    Ok(Ok(()))
}

// Side effect function for environment setup (I/O at edges)
fn apply_environment_setup(
    cache_location: &Option<std::path::PathBuf>,
    no_context_aware: bool,
) -> Result<()> {
    if let Some(ref location) = cache_location {
        std::env::set_var("DEBTMAP_CACHE_DIR", location);
    }

    if !no_context_aware {
        std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
    }

    Ok(())
}

// Pure function to check condition
fn should_use_cache(use_cache: bool, no_cache: bool) -> bool {
    !no_cache || use_cache
}

// Pure function to determine parallel mode
fn should_use_parallel(no_parallel: bool) -> bool {
    !no_parallel
}

// Pure function to get worker count
fn get_worker_count(jobs: usize) -> usize {
    if jobs == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    } else {
        jobs
    }
}

// Pure functions for data transformation
fn convert_min_priority(priority: Option<String>) -> Option<String> {
    priority
}

fn convert_filter_categories(categories: Option<Vec<String>>) -> Option<Vec<String>> {
    categories.filter(|v| !v.is_empty())
}

fn convert_context_providers(providers: Option<Vec<String>>) -> Option<Vec<String>> {
    providers.filter(|v| !v.is_empty())
}

fn convert_disable_context(disable_context: Option<Vec<String>>) -> Option<Vec<String>> {
    disable_context
}

fn convert_languages(languages: Option<Vec<String>>) -> Option<Vec<String>> {
    languages.filter(|v| !v.is_empty())
}

fn convert_cache_location(location: &Option<String>) -> Option<String> {
    location.clone()
}

fn convert_threshold_preset(preset: Option<debtmap::cli::ThresholdPreset>) -> Option<debtmap::cli::ThresholdPreset> {
    preset
}

fn convert_output_format(format: debtmap::cli::OutputFormat) -> debtmap::cli::OutputFormat {
    format
}

// Pure function to create formatting configuration
fn create_formatting_config(plain: bool) -> FormattingConfig {
    if plain {
        FormattingConfig::new(ColorMode::Never, EmojiMode::Never)
    } else {
        FormattingConfig::from_env()
    }
}

// Pure function to build analyze configuration
#[allow(clippy::too_many_arguments)]
fn build_analyze_config(
    path: std::path::PathBuf,
    format: debtmap::cli::OutputFormat,
    output: Option<std::path::PathBuf>,
    threshold_complexity: u32,
    threshold_duplication: usize,
    languages: Option<Vec<String>>,
    coverage_file: Option<std::path::PathBuf>,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
    top: Option<usize>,
    tail: Option<usize>,
    semantic_off: bool,
    verbosity: u8,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    group_by_category: bool,
    min_priority: Option<String>,
    filter_categories: Option<Vec<String>>,
    no_context_aware: bool,
    threshold_preset: Option<debtmap::cli::ThresholdPreset>,
    formatting_config: FormattingConfig,
    no_parallel: bool,
    jobs: usize,
    use_cache: bool,
    no_cache: bool,
    clear_cache: bool,
    cache_location: Option<String>,
    multi_pass: bool,
    show_attribution: bool,
    detail_level: Option<String>,
    aggregate_only: bool,
    no_aggregation: bool,
    aggregation_method: Option<String>,
    min_problematic: Option<usize>,
    no_god_object: bool,
) -> debtmap::commands::analyze::AnalyzeConfig {
    debtmap::commands::analyze::AnalyzeConfig {
        path,
        format: convert_output_format(format),
        output,
        threshold_complexity,
        threshold_duplication,
        languages: convert_languages(languages),
        coverage_file,
        enable_context,
        context_providers: convert_context_providers(context_providers),
        disable_context: convert_disable_context(disable_context),
        top,
        tail,
        semantic_off,
        verbosity,
        verbose_macro_warnings,
        show_macro_stats,
        group_by_category,
        min_priority: convert_min_priority(min_priority),
        filter_categories: convert_filter_categories(filter_categories),
        no_context_aware,
        threshold_preset: convert_threshold_preset(threshold_preset),
        formatting_config,
        parallel: should_use_parallel(no_parallel),
        jobs: get_worker_count(jobs),
        use_cache: should_use_cache(use_cache, no_cache),
        no_cache,
        clear_cache,
        cache_stats: false,
        migrate_cache: false,
        cache_location: convert_cache_location(&cache_location),
        multi_pass,
        show_attribution,
        detail_level,
        aggregate_only,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
    }
}
use debtmap::cli;
use debtmap::formatting::{ColorMode, EmojiMode, FormattingConfig};

use anyhow::Result;
use cli::Commands;
use std::process;

fn main() -> Result<()> {
    let cli = cli::parse_args();
    let result = execute_command(cli.command)?;
    handle_result(result)
}

// Pure function to execute a command
fn execute_command(command: Commands) -> Result<Result<()>> {
    match command {
        Commands::Analyze { .. } => handle_analyze_command(command),
        Commands::Init { force } => Ok(debtmap::commands::init::init_config(force)),
        Commands::Validate { .. } => handle_validate_command(command),
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
        apply_environment_setup(&cache_location, no_context_aware)?;

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
        unreachable!("handle_analyze_command called with non-Analyze command")
    }
}

// Pure function to handle validate command
fn handle_validate_command(command: Commands) -> Result<Result<()>> {
    if let Commands::Validate {
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
    } = command
    {
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
        Ok(debtmap::commands::validate::validate_project(config))
    } else {
        unreachable!("handle_validate_command called with non-Validate command")
    }
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

// Pure function for cache operations that may return early
fn handle_cache_operations(
    cache_stats: bool,
    migrate_cache: bool,
    path: &std::path::PathBuf,
) -> Result<Option<Result<()>>> {
    if cache_stats {
        return Ok(Some(display_cache_stats(path)));
    }

    if migrate_cache {
        return Ok(Some(perform_cache_migration(path)));
    }

    Ok(None)
}

// I/O function for cache stats
fn display_cache_stats(path: &std::path::PathBuf) -> Result<()> {
    let cache = debtmap::cache::SharedCache::new(Some(path))?;
    let stats = cache.get_full_stats()?;
    println!("Shared cache stats:\n{}", stats);

    use debtmap::cache::UnifiedAnalysisCache;
    if let Ok(unified_cache) = UnifiedAnalysisCache::new(Some(path)) {
        println!("\n{}", unified_cache.stats());
    }

    Ok(())
}

// I/O function for cache migration
fn perform_cache_migration(path: &std::path::PathBuf) -> Result<()> {
    let cache = debtmap::cache::SharedCache::new(Some(path))?;
    let local_cache = path.join(".debtmap_cache");

    if local_cache.exists() {
        println!("Migrating cache from local to shared location...");
        cache.migrate_from_local(&local_cache)?;
        println!("Cache migration complete!");

        if std::fs::remove_dir_all(&local_cache).is_ok() {
            println!("Local cache removed.");
        }
    }

    Ok(())
}

// Pure function for formatting configuration
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
    format: Option<String>,
    output: Option<std::path::PathBuf>,
    threshold_complexity: Option<u32>,
    threshold_duplication: Option<u32>,
    languages: Vec<String>,
    coverage_file: Option<std::path::PathBuf>,
    enable_context: bool,
    context_providers: Vec<String>,
    disable_context: bool,
    top: Option<usize>,
    tail: Option<usize>,
    semantic_off: bool,
    verbosity: u8,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    group_by_category: bool,
    min_priority: Option<f64>,
    filter_categories: Vec<String>,
    no_context_aware: bool,
    threshold_preset: Option<String>,
    formatting_config: FormattingConfig,
    no_parallel: bool,
    jobs: Option<usize>,
    use_cache: bool,
    no_cache: bool,
    clear_cache: bool,
    cache_location: Option<std::path::PathBuf>,
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
        parallel: !no_parallel,
        jobs,
        use_cache: !no_cache || use_cache,
        no_cache,
        clear_cache,
        cache_stats: false,
        migrate_cache: false,
        cache_location,
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

// Pure function for result handling
fn handle_result(result: Result<()>) -> Result<()> {
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

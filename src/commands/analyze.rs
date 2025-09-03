use super::super::builders::unified_analysis;
use super::super::output;
use super::super::utils::{analysis_helpers, language_parser};
use crate::{
    analysis_utils, cli, config,
    core::{self, *},
    formatting::FormattingConfig,
    io,
};
use anyhow::{Context, Result};
use chrono::Utc;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct AnalyzeConfig {
    pub path: PathBuf,
    pub format: crate::cli::OutputFormat,
    pub output: Option<PathBuf>,
    pub threshold_complexity: u32,
    pub threshold_duplication: usize,
    pub languages: Option<Vec<String>>,
    pub coverage_file: Option<PathBuf>,
    pub enable_context: bool,
    pub context_providers: Option<Vec<String>>,
    pub disable_context: Option<Vec<String>>,
    pub top: Option<usize>,
    pub tail: Option<usize>,
    pub semantic_off: bool,
    pub verbosity: u8,
    pub verbose_macro_warnings: bool,
    pub show_macro_stats: bool,
    pub group_by_category: bool,
    pub min_priority: Option<String>,
    pub filter_categories: Option<Vec<String>>,
    pub no_context_aware: bool,
    pub threshold_preset: Option<cli::ThresholdPreset>,
    pub formatting_config: FormattingConfig,
    pub parallel: bool,
    pub jobs: usize,
    pub use_cache: bool,
    pub no_cache: bool,
    pub clear_cache: bool,
    pub cache_stats: bool,
    pub migrate_cache: bool,
    pub cache_location: Option<String>,
    pub multi_pass: bool,
    pub show_attribution: bool,
    pub detail_level: Option<String>,
}

pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    configure_output(&config);
    set_threshold_preset(config.threshold_preset);

    // Handle cache flags
    if config.clear_cache {
        // Clear cache using the shared cache system
        if let Ok(mut cache) = core::cache::AnalysisCache::new(Some(&config.path)) {
            cache.clear()?;
            log::info!("Cache cleared");
        }
    }

    if config.no_cache {
        std::env::set_var("DEBTMAP_NO_CACHE", "1");
    }

    let languages = language_parser::parse_languages(config.languages);
    let results = analyze_project(
        config.path.clone(),
        languages,
        config.threshold_complexity,
        config.threshold_duplication,
    )?;

    let unified_analysis = unified_analysis::perform_unified_analysis_with_options(
        unified_analysis::UnifiedAnalysisOptions {
            results: &results,
            coverage_file: config.coverage_file.as_ref(),
            semantic_off: config.semantic_off,
            project_path: &config.path,
            verbose_macro_warnings: config.verbose_macro_warnings,
            show_macro_stats: config.show_macro_stats,
            parallel: config.parallel,
            jobs: config.jobs,
            use_cache: config.use_cache,
            multi_pass: config.multi_pass,
            show_attribution: config.show_attribution,
        },
    )?;

    let output_config = output::OutputConfig {
        top: config.top,
        tail: config.tail,
        verbosity: config.verbosity,
        output_file: config.output,
        output_format: Some(config.format),
        formatting_config: config.formatting_config,
    };

    output::output_unified_priorities_with_config(
        unified_analysis,
        output_config,
        &results,
        config.coverage_file.as_ref(),
    )?;

    Ok(())
}

fn configure_output(config: &AnalyzeConfig) {
    if config.formatting_config.color.should_use_color() {
        colored::control::set_override(true);
    } else {
        colored::control::set_override(false);
    }
}

fn set_threshold_preset(preset: Option<cli::ThresholdPreset>) {
    if let Some(preset) = preset {
        match preset {
            cli::ThresholdPreset::Strict => std::env::set_var("DEBTMAP_THRESHOLD_PRESET", "strict"),
            cli::ThresholdPreset::Balanced => {
                std::env::set_var("DEBTMAP_THRESHOLD_PRESET", "balanced")
            }
            cli::ThresholdPreset::Lenient => {
                std::env::set_var("DEBTMAP_THRESHOLD_PRESET", "lenient")
            }
        }
    }
}

fn analyze_project(
    path: PathBuf,
    languages: Vec<Language>,
    complexity_threshold: u32,
    duplication_threshold: usize,
) -> Result<AnalysisResults> {
    let config = config::get_config();
    let files = io::walker::find_project_files_with_config(&path, languages.clone(), config)
        .context("Failed to find project files")?;

    // Initialize cache (enabled by default unless DEBTMAP_NO_CACHE is set)
    let cache_enabled = std::env::var("DEBTMAP_NO_CACHE").is_err();
    let mut cache = if cache_enabled {
        match core::cache::AnalysisCache::new(Some(&path)) {
            Ok(c) => Some(c),
            Err(e) => {
                log::warn!("Failed to initialize cache: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Collect file metrics with or without cache
    let file_metrics = if let Some(ref mut cache) = cache {
        collect_file_metrics_with_cache(&files, cache)
    } else {
        analysis_utils::collect_file_metrics(&files)
    };

    // Print cache statistics in verbose mode
    if cache_enabled && log::log_enabled!(log::Level::Debug) {
        if let Some(cache) = &cache {
            log::info!("Cache stats: {}", cache.stats());
        }
    }

    let all_functions = analysis_utils::extract_all_functions(&file_metrics);
    let all_debt_items = analysis_utils::extract_all_debt_items(&file_metrics);
    let duplications = analysis_helpers::detect_duplications(&files, duplication_threshold);

    let complexity_report =
        analysis_helpers::build_complexity_report(&all_functions, complexity_threshold);
    let technical_debt =
        analysis_helpers::build_technical_debt_report(all_debt_items, duplications.clone());
    let dependencies = analysis_helpers::create_dependency_report(&file_metrics);

    Ok(AnalysisResults {
        project_path: path,
        timestamp: Utc::now(),
        complexity: complexity_report,
        technical_debt,
        dependencies,
        duplications,
    })
}

fn collect_file_metrics_with_cache(
    files: &[PathBuf],
    cache: &mut core::cache::AnalysisCache,
) -> Vec<FileMetrics> {
    let cache = Arc::new(Mutex::new(cache));

    files
        .par_iter()
        .filter_map(|path| {
            let mut cache = cache.lock().unwrap();
            cache
                .get_or_compute(path, || {
                    analysis_utils::analyze_single_file(path)
                        .ok_or_else(|| anyhow::anyhow!("Failed to analyze file"))
                })
                .ok()
        })
        .collect()
}

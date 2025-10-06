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
use std::io::Write;
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
    pub summary: bool,
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
    pub force_cache_rebuild: bool,
    pub cache_stats: bool,
    pub migrate_cache: bool,
    pub cache_location: Option<String>,
    pub multi_pass: bool,
    pub show_attribution: bool,
    pub detail_level: Option<String>,
    pub aggregate_only: bool,
    pub no_aggregation: bool,
    pub aggregation_method: Option<String>,
    pub min_problematic: Option<usize>,
    pub no_god_object: bool,
    pub max_files: Option<usize>,
    pub validate_loc: bool,
}

pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    configure_output(&config);
    set_threshold_preset(config.threshold_preset);

    // Set max files environment variable if specified
    if let Some(max_files) = config.max_files {
        std::env::set_var("DEBTMAP_MAX_FILES", max_files.to_string());
    }

    // Set jobs environment variable for parallel processing
    if config.jobs > 0 {
        std::env::set_var("DEBTMAP_JOBS", config.jobs.to_string());
    }

    // Handle cache flags
    if config.clear_cache || config.force_cache_rebuild {
        // Clear cache using the shared cache system
        if let Ok(mut cache) = core::cache::AnalysisCache::new(Some(&config.path)) {
            cache.clear()?;
            log::info!("File metrics cache cleared");
        }

        // Also clear unified analysis cache
        use crate::cache::UnifiedAnalysisCache;
        if let Ok(mut unified_cache) = UnifiedAnalysisCache::new(Some(&config.path)) {
            unified_cache.clear()?;
            log::info!("Unified analysis cache cleared");
        }

        if config.force_cache_rebuild {
            log::info!("Force cache rebuild requested - all caches cleared");
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
        config.parallel,
        config.formatting_config,
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
            aggregate_only: config.aggregate_only,
            no_aggregation: config.no_aggregation,
            aggregation_method: config.aggregation_method.clone(),
            min_problematic: config.min_problematic,
            no_god_object: config.no_god_object,
            formatting_config: config.formatting_config,
        },
    )?;

    let output_config = output::OutputConfig {
        top: config.top,
        tail: config.tail,
        summary: config.summary,
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

    // Set emoji mode environment variable for components that can't access FormattingConfig
    if !config.formatting_config.emoji.should_use_emoji() {
        std::env::set_var("DEBTMAP_NO_EMOJI", "1");
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
    parallel_enabled: bool,
    formatting_config: FormattingConfig,
) -> Result<AnalysisResults> {
    // Set environment variables for parallel processing
    if parallel_enabled {
        std::env::set_var("DEBTMAP_PARALLEL", "true");
    }
    let config = config::get_config();
    let files = io::walker::find_project_files_with_config(&path, languages.clone(), config)
        .context("Failed to find project files")?;

    // Analyze project size and apply graduated optimizations
    analyze_and_configure_project_size(&files, parallel_enabled, formatting_config)?;

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

/// Analyze project size and configure optimizations based on scale
fn analyze_and_configure_project_size(
    files: &[PathBuf],
    parallel_enabled: bool,
    formatting_config: FormattingConfig,
) -> Result<()> {
    let file_count = files.len();
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();
    let use_emoji = formatting_config.emoji.should_use_emoji();

    if !quiet_mode {
        match file_count {
            0..=100 => {
                // Small project - no warnings needed
                if use_emoji {
                    eprintln!("📁 Analyzing {} files (small project)", file_count);
                } else {
                    eprintln!("Analyzing {} files (small project)", file_count);
                }
            }
            101..=500 => {
                // Medium project - inform user
                if use_emoji {
                    eprintln!("📁 Analyzing {} files (medium project)", file_count);
                } else {
                    eprintln!("Analyzing {} files (medium project)", file_count);
                }
                if parallel_enabled {
                    if use_emoji {
                        eprintln!("💡 Parallel processing enabled for better performance");
                    } else {
                        eprintln!("Parallel processing enabled for better performance");
                    }
                } else if use_emoji {
                    eprintln!(
                        "💡 Using sequential processing (use default for better performance)"
                    );
                } else {
                    eprintln!("Using sequential processing (use default for better performance)");
                }
            }
            501..=1000 => {
                // Large project - inform user
                if use_emoji {
                    eprintln!("📁 Analyzing {} files (large project)", file_count);
                } else {
                    eprintln!("Analyzing {} files (large project)", file_count);
                }

                // Enable parallel processing by default
                std::env::set_var("RUST_BACKTRACE", "0"); // Reduce noise
            }
            1001..=2000 => {
                // Very large project - inform user
                if use_emoji {
                    eprintln!("📁 Analyzing {} files (very large project)", file_count);
                    eprint!("⏱️  Starting analysis...");
                } else {
                    eprintln!("Analyzing {} files (very large project)", file_count);
                    eprint!("Starting analysis...");
                }

                // Enable all performance optimizations
                std::env::set_var("RUST_BACKTRACE", "0");
                std::io::stderr().flush().unwrap();
            }
            _ => {
                // Massive project - inform user
                if use_emoji {
                    eprintln!("📁 Analyzing {} files (massive project)", file_count);
                    eprintln!();
                    eprintln!("💡 Suggestions:");
                    eprintln!("   • Use .debtmapignore to exclude test/vendor directories");
                    eprintln!("   • Focus analysis on specific modules with targeted paths");
                    eprintln!();
                    eprint!("⏱️  Starting analysis...");
                } else {
                    eprintln!("Analyzing {} files (massive project)", file_count);
                    eprintln!();
                    eprintln!("Suggestions:");
                    eprintln!("   • Use .debtmapignore to exclude test/vendor directories");
                    eprintln!("   • Focus analysis on specific modules with targeted paths");
                    eprintln!();
                    eprint!("Starting analysis...");
                }

                std::env::set_var("RUST_BACKTRACE", "0");
                std::io::stderr().flush().unwrap();
            }
        }
    }

    Ok(())
}

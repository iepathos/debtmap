use super::super::builders::unified_analysis;
use super::super::output;
use super::super::utils::{analysis_helpers, language_parser};
use crate::{analysis_utils, cli, config, core::*, formatting::FormattingConfig, io};
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::PathBuf;

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
}

pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    configure_output(&config);
    set_threshold_preset(config.threshold_preset);

    let languages = language_parser::parse_languages(config.languages);
    let results = analyze_project(
        config.path.clone(),
        languages,
        config.threshold_complexity,
        config.threshold_duplication,
    )?;

    let unified_analysis = unified_analysis::perform_unified_analysis(
        &results,
        config.coverage_file.as_ref(),
        config.semantic_off,
        &config.path,
        config.verbose_macro_warnings,
        config.show_macro_stats,
    )?;

    output::output_unified_priorities_with_config(
        unified_analysis,
        config.top,
        config.tail,
        config.verbosity,
        config.output,
        Some(config.format),
        &results,
        config.coverage_file.as_ref(),
        config.formatting_config,
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

    let file_metrics = analysis_utils::collect_file_metrics(&files);
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

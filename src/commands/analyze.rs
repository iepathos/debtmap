use super::super::builders::unified_analysis;
use super::super::output;
use super::super::utils::{analysis_helpers, language_parser};
use crate::{
    analysis_utils, cli, config,
    core::*,
    formatting::FormattingConfig,
    io,
    progress::{ProgressConfig, ProgressManager},
    tui::app::StageStatus,
};
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
    pub summary: bool,
    pub semantic_off: bool,
    pub verbosity: u8,
    pub verbose_macro_warnings: bool,
    pub show_macro_stats: bool,
    pub group_by_category: bool,
    pub min_priority: Option<String>,
    pub min_score: Option<f64>,
    pub filter_categories: Option<Vec<String>>,
    pub no_context_aware: bool,
    pub threshold_preset: Option<cli::ThresholdPreset>,
    pub _formatting_config: FormattingConfig,
    pub parallel: bool,
    pub jobs: usize,
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
    pub no_public_api_detection: bool,
    pub public_api_threshold: f32,
    pub no_pattern_detection: bool,
    pub patterns: Option<Vec<String>>,
    pub pattern_threshold: f32,
    pub show_pattern_warnings: bool,
    pub debug_call_graph: bool,
    pub trace_functions: Option<Vec<String>>,
    pub call_graph_stats_only: bool,
    pub debug_format: crate::cli::DebugFormatArg,
    pub validate_call_graph: bool,
    pub show_dependencies: bool,
    pub no_dependencies: bool,
    pub max_callers: usize,
    pub max_callees: usize,
    pub show_external: bool,
    pub show_std_lib: bool,
    pub ast_functional_analysis: bool,
    pub functional_analysis_profile: Option<crate::cli::FunctionalAnalysisProfile>,
    pub min_split_methods: usize,
    pub min_split_lines: usize,
    pub no_tui: bool,
    pub show_filter_stats: bool,
}

pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    configure_output(&config);
    set_threshold_preset(config.threshold_preset);

    // Initialize global progress manager with TUI support
    let quiet = std::env::var("DEBTMAP_QUIET").is_ok();
    let progress_config = ProgressConfig::from_env(quiet, config.verbosity);
    ProgressManager::init_global(progress_config);

    // Start TUI rendering if available
    if let Some(manager) = ProgressManager::global() {
        manager.tui_start_stage(0); // files stage
    }

    // Set max files environment variable if specified
    if let Some(max_files) = config.max_files {
        std::env::set_var("DEBTMAP_MAX_FILES", max_files.to_string());
    }

    // Set minimum score threshold if specified (spec 193)
    if let Some(min_score) = config.min_score {
        std::env::set_var("DEBTMAP_MIN_SCORE_THRESHOLD", min_score.to_string());
    }

    // Set jobs environment variable for parallel processing
    if config.jobs > 0 {
        std::env::set_var("DEBTMAP_JOBS", config.jobs.to_string());
    }

    // Set functional analysis environment variables (spec 111)
    if config.ast_functional_analysis {
        std::env::set_var("DEBTMAP_FUNCTIONAL_ANALYSIS", "true");

        // Set the profile if specified
        if let Some(profile) = config.functional_analysis_profile {
            let profile_str = match profile {
                crate::cli::FunctionalAnalysisProfile::Strict => "strict",
                crate::cli::FunctionalAnalysisProfile::Balanced => "balanced",
                crate::cli::FunctionalAnalysisProfile::Lenient => "lenient",
            };
            std::env::set_var("DEBTMAP_FUNCTIONAL_ANALYSIS_PROFILE", profile_str);
        }
    }

    let languages = language_parser::parse_languages(config.languages.clone());
    let results = analyze_project(
        config.path.clone(),
        languages,
        config.threshold_complexity,
        config.threshold_duplication,
        config.parallel,
        config._formatting_config,
    )?;

    // Note: Phase 2 (Building call graph) is tracked inside perform_unified_analysis_with_options
    let mut unified_analysis = unified_analysis::perform_unified_analysis_with_options(
        unified_analysis::UnifiedAnalysisOptions {
            results: &results,
            coverage_file: config.coverage_file.as_ref(),
            semantic_off: config.semantic_off,
            project_path: &config.path,
            verbose_macro_warnings: config.verbose_macro_warnings,
            show_macro_stats: config.show_macro_stats,
            parallel: config.parallel,
            jobs: config.jobs,
            multi_pass: config.multi_pass,
            show_attribution: config.show_attribution,
            aggregate_only: config.aggregate_only,
            no_aggregation: config.no_aggregation,
            aggregation_method: config.aggregation_method.clone(),
            min_problematic: config.min_problematic,
            no_god_object: config.no_god_object,
            suppress_coverage_tip: false, // Show coverage TIP for analyze command
            _formatting_config: config._formatting_config,
            enable_context: config.enable_context,
            context_providers: config.context_providers.clone(),
            disable_context: config.disable_context.clone(),
        },
    )?;

    // Apply file context adjustments for test file scoring (spec 166)
    use crate::priority::UnifiedAnalysisUtils;
    unified_analysis.apply_file_context_adjustments(&results.file_contexts);

    // Handle call graph debug and validation flags
    if config.debug_call_graph || config.validate_call_graph || config.call_graph_stats_only {
        handle_call_graph_diagnostics(&unified_analysis, &config)?;
    }

    // Apply category filtering if specified
    let filtered_analysis = if let Some(ref filter_cats) = config.filter_categories {
        let categories: Vec<crate::priority::DebtCategory> = filter_cats
            .iter()
            .filter_map(|s| crate::priority::DebtCategory::from_string(s))
            .collect();

        if !categories.is_empty() {
            unified_analysis.filter_by_categories(&categories)
        } else {
            unified_analysis
        }
    } else {
        unified_analysis
    };

    // Items in unified_analysis are final (spec 243: single-stage filtering)
    // No post-filtering needed - all filtering happens during add_item

    // Cleanup TUI BEFORE writing output (alternate screen would discard output)
    if let Some(manager) = ProgressManager::global() {
        manager.tui_set_progress(1.0);
        manager.tui_cleanup();
    }

    // Show total analysis time (spec 195)
    io::progress::AnalysisProgress::with_global(|p| p.finish());

    // Determine output mode: use interactive TUI or traditional output
    if should_use_tui(&config) {
        // Launch interactive TUI results explorer
        use crate::tui::results::ResultsExplorer;
        let mut explorer = ResultsExplorer::new(filtered_analysis)?;
        explorer.run()?;
    } else {
        // Use traditional text/JSON/markdown output
        let output_config = output::OutputConfig {
            top: config.top,
            tail: config.tail,
            summary: config.summary,
            verbosity: config.verbosity,
            output_file: config.output,
            output_format: Some(config.format),
            formatting_config: config._formatting_config,
            show_filter_stats: config.show_filter_stats,
        };

        output::output_unified_priorities_with_config(
            filtered_analysis,
            output_config,
            &results,
            config.coverage_file.as_ref(),
        )?;
    }

    Ok(())
}

/// Determine if interactive TUI should be used
fn should_use_tui(config: &AnalyzeConfig) -> bool {
    use std::io::IsTerminal;

    // Don't use TUI if:
    // 1. Explicitly disabled with --no-tui
    // 2. Non-terminal format specified (JSON, Markdown, HTML)
    // 3. Output file specified
    // 4. stdout is not a terminal (piped/redirected)
    // 5. CI environment detected
    !config.no_tui
        && matches!(config.format, cli::OutputFormat::Terminal)
        && config.output.is_none()
        && std::io::stdout().is_terminal()
        && std::env::var("CI").is_err()
}

fn configure_output(config: &AnalyzeConfig) {
    if config._formatting_config.color.should_use_color() {
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

pub fn analyze_project(
    path: PathBuf,
    languages: Vec<Language>,
    complexity_threshold: u32,
    duplication_threshold: usize,
    parallel_enabled: bool,
    _formatting_config: FormattingConfig,
) -> Result<AnalysisResults> {
    // Set environment variables for parallel processing
    if parallel_enabled {
        std::env::set_var("DEBTMAP_PARALLEL", "true");
    }
    let config = config::get_config();

    // Initialize global unified progress tracker (spec 195)
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();
    if !quiet_mode {
        io::progress::AnalysisProgress::init_global();
    }

    // Phase 1: files parse (discovery + parsing combined)
    io::progress::AnalysisProgress::with_global(|p| p.start_phase(0));
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_start_stage(0);
    }

    // Subtask 0: discover files
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(0, 0, StageStatus::Active, None);
    }

    let files = io::walker::find_project_files_with_config(&path, languages.clone(), config)
        .context("Failed to find project files")?;

    // Update progress with file count (still in phase 0)
    io::progress::AnalysisProgress::with_global(|p| {
        p.update_progress(io::progress::PhaseProgress::Count(files.len()));
    });

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(0, 0, StageStatus::Completed, None);
        std::thread::sleep(std::time::Duration::from_millis(150)); // Visual consistency
    }

    // Subtask 1: parse metrics
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(0, 1, StageStatus::Active, None);
    }

    // Analyze project size and apply graduated optimizations
    analyze_and_configure_project_size(&files, parallel_enabled, _formatting_config)?;

    // Continue phase 0 for parsing (no phase transition needed)
    // Collect file metrics directly without caching
    let file_metrics = analysis_utils::collect_file_metrics(&files);

    // Update progress to show parsing completion (still in phase 0)
    io::progress::AnalysisProgress::with_global(|p| {
        p.update_progress(io::progress::PhaseProgress::Progress {
            current: files.len(),
            total: files.len(),
        });
        p.complete_phase();
    });

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(0, 1, StageStatus::Completed, None);
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 2: extract data
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(0, 2, StageStatus::Active, None);
    }

    let all_functions = analysis_utils::extract_all_functions(&file_metrics);
    let all_debt_items = analysis_utils::extract_all_debt_items(&file_metrics);

    // Update TUI stats with function and debt counts
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_counts(all_functions.len(), all_debt_items.len());
    }

    // Extract file contexts for test file detection (spec 166)
    let file_contexts = analysis_utils::extract_file_contexts(&file_metrics);

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(0, 2, StageStatus::Completed, None);
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 3: detect duplications (THE SLOW ONE)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(0, 3, StageStatus::Active, Some((0, files.len())));
    }

    let duplications = analysis_helpers::detect_duplications_with_progress(
        &files,
        duplication_threshold,
        |current, total| {
            if let Some(manager) = crate::progress::ProgressManager::global() {
                manager.tui_update_subtask(0, 3, StageStatus::Active, Some((current, total)));
            }
        },
    );

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            0,
            3,
            StageStatus::Completed,
            Some((files.len(), files.len())),
        );
        manager.tui_complete_stage(0, format!("{} files parsed", files.len()));
        manager.tui_set_progress(0.22); // ~2/9 stages complete
    }

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
        file_contexts,
    })
}

/// Analyze project size and configure optimizations based on scale
fn analyze_and_configure_project_size(
    files: &[PathBuf],
    parallel_enabled: bool,
    _formatting_config: FormattingConfig,
) -> Result<()> {
    let file_count = files.len();
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();

    if !quiet_mode {
        match file_count {
            0..=100 => {
                // Small project - no warnings needed
                log::info!("Analyzing {} files (small project)", file_count);
            }
            101..=500 => {
                // Medium project - inform user
                log::info!("Analyzing {} files (medium project)", file_count);
                if parallel_enabled {
                    log::info!("Parallel processing enabled for better performance");
                } else {
                    log::warn!("Using sequential processing (use default for better performance)");
                }
            }
            501..=1000 => {
                // Large project - inform user
                log::info!("Analyzing {} files (large project)", file_count);

                // Enable parallel processing by default
                std::env::set_var("RUST_BACKTRACE", "0"); // Reduce noise
            }
            1001..=2000 => {
                // Very large project - inform user
                log::info!("Analyzing {} files (very large project)", file_count);

                // Enable all performance optimizations
                std::env::set_var("RUST_BACKTRACE", "0");
            }
            _ => {
                // Massive project - inform user
                log::warn!("Analyzing {} files (massive project)", file_count);
                log::warn!("Consider using .debtmapignore to exclude test/vendor directories");
                log::warn!("Focus analysis on specific modules with targeted paths");

                std::env::set_var("RUST_BACKTRACE", "0");
            }
        }
    }

    Ok(())
}

/// Handle call graph debug and validation diagnostics
fn handle_call_graph_diagnostics(
    unified_analysis: &crate::priority::UnifiedAnalysis,
    config: &AnalyzeConfig,
) -> Result<()> {
    use crate::analyzers::call_graph::debug::{CallGraphDebugger, DebugConfig, DebugFormat};
    use crate::analyzers::call_graph::validation::CallGraphValidator;

    // Get the call graph from unified analysis
    let call_graph = &unified_analysis.call_graph;

    // Run validation if requested
    if config.validate_call_graph {
        let validation_report = CallGraphValidator::validate(call_graph);

        eprintln!("\n=== Call Graph Validation Report ===");
        eprintln!("Health Score: {}/100", validation_report.health_score);

        // Display statistics
        eprintln!("\nStatistics:");
        eprintln!(
            "  Total Functions: {}",
            validation_report.statistics.total_functions
        );
        eprintln!(
            "  Entry Points: {}",
            validation_report.statistics.entry_points
        );
        eprintln!(
            "  Leaf Functions: {} (has callers, no callees)",
            validation_report.statistics.leaf_functions
        );
        eprintln!(
            "  Unreachable: {} (no callers, has callees)",
            validation_report.statistics.unreachable_functions
        );
        eprintln!(
            "  Isolated: {} (no callers, no callees)",
            validation_report.statistics.isolated_functions
        );
        if validation_report.statistics.recursive_functions > 0 {
            eprintln!(
                "  Recursive: {}",
                validation_report.statistics.recursive_functions
            );
        }

        eprintln!(
            "\nStructural Issues: {}",
            validation_report.structural_issues.len()
        );
        eprintln!("Warnings: {}", validation_report.warnings.len());

        if !validation_report.structural_issues.is_empty() {
            eprintln!("\nStructural Issues:");
            for issue in &validation_report.structural_issues {
                eprintln!("  - {:?}", issue);
            }
        }

        if !validation_report.warnings.is_empty() && config.verbosity > 0 {
            eprintln!("\nWarnings:");
            for warning in validation_report.warnings.iter().take(10) {
                eprintln!("  - {:?}", warning);
            }
            if validation_report.warnings.len() > 10 {
                eprintln!(
                    "  ... and {} more warnings",
                    validation_report.warnings.len() - 10
                );
            }
        }
    }

    // Run debug output if requested
    if config.debug_call_graph {
        let format = match config.debug_format {
            crate::cli::DebugFormatArg::Text => DebugFormat::Text,
            crate::cli::DebugFormatArg::Json => DebugFormat::Json,
        };

        let debug_config = DebugConfig {
            show_successes: config.verbosity > 1,
            show_timing: true,
            max_candidates_shown: 5,
            format,
            filter_functions: config
                .trace_functions
                .as_ref()
                .map(|funcs| funcs.iter().cloned().collect()),
        };

        let mut debugger = CallGraphDebugger::new(debug_config);

        // Add trace functions if specified
        if let Some(ref funcs) = config.trace_functions {
            for func in funcs {
                debugger.add_trace_function(func.clone());
            }
        }

        // Finalize statistics
        debugger.finalize_statistics();

        // Output debug report
        eprintln!("\n=== Call Graph Debug Report ===");
        let mut stdout = std::io::stdout();
        debugger.write_report(&mut stdout)?;
    }

    // Show call graph stats if requested
    if config.call_graph_stats_only {
        eprintln!("\n=== Call Graph Statistics ===");
        eprintln!("Total Functions: {}", call_graph.node_count());

        // Calculate total calls by summing callees for all functions
        let total_calls: usize = call_graph
            .get_all_functions()
            .map(|func| call_graph.get_callees(func).len())
            .sum();
        eprintln!("Total Calls: {}", total_calls);

        eprintln!(
            "Average Calls per Function: {:.2}",
            if call_graph.node_count() > 0 {
                total_calls as f64 / call_graph.node_count() as f64
            } else {
                0.0
            }
        );
    }

    // Print coverage matching statistics if diagnostic mode enabled (Spec 203 FR3)
    if std::env::var("DEBTMAP_COVERAGE_DEBUG").is_ok() {
        crate::risk::lcov::print_coverage_statistics();
    }

    Ok(())
}

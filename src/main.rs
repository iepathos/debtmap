use anyhow::Result;
use clap::Parser;
use debtmap::cli::{Cli, Commands};
use debtmap::core::injection::{AppContainer, AppContainerBuilder};
use debtmap::error::{CliError, ConfigError};
use debtmap::formatting::{ColorMode, FormattingConfig};
use std::path::Path;
use std::sync::Arc;

// Default implementations for DI container components
mod default_implementations {
    use anyhow::Result;
    use debtmap::core::traits::{
        ConfigProvider, Formatter, PriorityCalculator, PriorityFactor, Scorer,
    };
    use debtmap::core::types::{AnalysisResult, DebtCategory, DebtItem, Severity};
    use std::collections::HashMap;

    pub struct DefaultDebtScorer;

    impl DefaultDebtScorer {
        pub fn new() -> Self {
            Self
        }
    }

    impl Scorer for DefaultDebtScorer {
        type Item = DebtItem;

        fn score(&self, item: &Self::Item) -> f64 {
            let base_score = match item.category {
                DebtCategory::Complexity => 10.0,
                DebtCategory::Testing => 8.0,
                DebtCategory::Documentation => 6.0,
                DebtCategory::Organization => 7.0,
                DebtCategory::Performance => 9.0,
                DebtCategory::Security => 10.0,
                DebtCategory::Maintainability => 7.0,
                _ => 5.0,
            };

            let severity_multiplier = match item.severity {
                Severity::Critical => 3.0,
                Severity::Major => 2.0,
                Severity::Warning => 1.5,
                Severity::Info => 1.0,
            };

            base_score * severity_multiplier * item.effort.max(0.1)
        }

        fn methodology(&self) -> &str {
            "Default scoring based on category, severity, and estimated hours"
        }
    }

    pub struct DefaultConfigProvider {
        config: std::sync::RwLock<HashMap<String, String>>,
    }

    impl DefaultConfigProvider {
        pub fn new() -> Self {
            let mut config = HashMap::new();
            // Load default configuration values
            config.insert("complexity_threshold".to_string(), "10".to_string());
            config.insert("max_file_size".to_string(), "1000000".to_string());
            config.insert("parallel_processing".to_string(), "true".to_string());

            Self {
                config: std::sync::RwLock::new(config),
            }
        }
    }

    impl ConfigProvider for DefaultConfigProvider {
        fn get(&self, key: &str) -> Option<String> {
            let config = self.config.read().unwrap();
            config.get(key).cloned()
        }

        fn set(&mut self, key: String, value: String) {
            let mut config = self.config.write().unwrap();
            config.insert(key, value);
        }

        fn load_from_file(&self, _path: &std::path::Path) -> Result<()> {
            // In production, would read from actual config file
            // For now, just return Ok to satisfy the trait
            Ok(())
        }
    }

    pub struct DefaultPriorityCalculator;

    impl DefaultPriorityCalculator {
        pub fn new() -> Self {
            Self
        }
    }

    impl PriorityCalculator for DefaultPriorityCalculator {
        type Item = DebtItem;

        fn calculate_priority(&self, item: &Self::Item) -> f64 {
            let severity_weight = match item.severity {
                Severity::Critical => 1.0,
                Severity::Major => 0.75,
                Severity::Warning => 0.5,
                Severity::Info => 0.25,
            };

            let category_weight = match item.category {
                DebtCategory::Complexity => 0.9,
                DebtCategory::Testing => 0.85,
                DebtCategory::Performance => 0.8,
                DebtCategory::Organization => 0.7,
                DebtCategory::Documentation => 0.6,
                DebtCategory::Security => 0.95,
                _ => 0.5,
            };

            let effort_factor = 1.0 / (1.0 + item.effort);

            f64::min(
                severity_weight * 0.5 + category_weight * 0.3 + effort_factor * 0.2,
                1.0,
            )
        }

        fn get_factors(&self, item: &Self::Item) -> Vec<PriorityFactor> {
            vec![
                PriorityFactor {
                    name: "severity".to_string(),
                    weight: 0.5,
                    value: match item.severity {
                        Severity::Critical => 1.0,
                        Severity::Major => 0.75,
                        Severity::Warning => 0.5,
                        Severity::Info => 0.25,
                    },
                    description: format!("Severity: {:?}", item.severity),
                },
                PriorityFactor {
                    name: "category".to_string(),
                    weight: 0.3,
                    value: match item.category {
                        DebtCategory::Complexity => 0.9,
                        DebtCategory::Testing => 0.85,
                        _ => 0.5,
                    },
                    description: format!("Category: {:?}", item.category),
                },
                PriorityFactor {
                    name: "effort".to_string(),
                    weight: 0.2,
                    value: 1.0 / (1.0 + item.effort),
                    description: format!("Estimated effort: {} hours", item.effort),
                },
            ]
        }
    }

    pub struct JsonFormatter;

    impl JsonFormatter {
        pub fn new() -> Self {
            Self
        }
    }

    impl Formatter for JsonFormatter {
        type Report = AnalysisResult;

        fn format(&self, report: &Self::Report) -> Result<String> {
            serde_json::to_string_pretty(report)
                .map_err(|e| anyhow::anyhow!("JSON formatting error: {}", e))
        }

        fn format_name(&self) -> &str {
            "json"
        }
    }

    pub struct MarkdownFormatter;

    impl MarkdownFormatter {
        pub fn new() -> Self {
            Self
        }
    }

    impl Formatter for MarkdownFormatter {
        type Report = AnalysisResult;

        fn format(&self, report: &Self::Report) -> Result<String> {
            let mut output = String::new();
            output.push_str("# Code Analysis Report\n\n");
            output.push_str("## Summary\n\n");
            output.push_str(&format!("- Total Files: {}\n", report.metrics.total_files));
            output.push_str(&format!(
                "- Total Functions: {}\n",
                report.metrics.total_functions
            ));
            output.push_str(&format!("- Total Lines: {}\n", report.metrics.total_lines));
            output.push_str(&format!(
                "- Average Complexity: {:.2}\n",
                report.metrics.average_complexity
            ));
            output.push_str(&format!("- Debt Score: {:.2}\n\n", report.total_score));

            if !report.debt_items.is_empty() {
                output.push_str("## Technical Debt Items\n\n");
                for item in &report.debt_items {
                    output.push_str(&format!(
                        "- **{:?}** ({:?}): {}\n",
                        item.category, item.severity, item.description
                    ));
                }
            }

            Ok(output)
        }

        fn format_name(&self) -> &str {
            "markdown"
        }
    }

    pub struct TerminalFormatter;

    impl TerminalFormatter {
        pub fn new() -> Self {
            Self
        }
    }

    impl Formatter for TerminalFormatter {
        type Report = AnalysisResult;

        fn format(&self, report: &Self::Report) -> Result<String> {
            let mut output = String::new();
            output.push_str("═══════════════════════════════════════\n");
            output.push_str("         Code Analysis Report          \n");
            output.push_str("═══════════════════════════════════════\n\n");

            output.push_str(&format!(
                "Total Files:      {}\n",
                report.metrics.total_files
            ));
            output.push_str(&format!(
                "Total Functions:  {}\n",
                report.metrics.total_functions
            ));
            output.push_str(&format!(
                "Total Lines:      {}\n",
                report.metrics.total_lines
            ));
            output.push_str(&format!(
                "Avg Complexity:   {:.2}\n",
                report.metrics.average_complexity
            ));
            output.push_str(&format!("Debt Score:       {:.2}\n", report.total_score));

            if !report.debt_items.is_empty() {
                output.push_str("\n───────────────────────────────────────\n");
                output.push_str("Technical Debt Summary:\n");
                output.push_str(&format!("  {} items found\n", report.debt_items.len()));

                // Create severity counts
                let mut critical_count = 0;
                let mut major_count = 0;
                let mut warning_count = 0;
                let mut info_count = 0;

                for item in &report.debt_items {
                    match item.severity {
                        Severity::Critical => critical_count += 1,
                        Severity::Major => major_count += 1,
                        Severity::Warning => warning_count += 1,
                        Severity::Info => info_count += 1,
                    }
                }

                if critical_count > 0 {
                    output.push_str(&format!("  Critical: {}\n", critical_count));
                }
                if major_count > 0 {
                    output.push_str(&format!("  Major: {}\n", major_count));
                }
                if warning_count > 0 {
                    output.push_str(&format!("  Warning: {}\n", warning_count));
                }
                if info_count > 0 {
                    output.push_str(&format!("  Info: {}\n", info_count));
                }
            }

            Ok(output)
        }

        fn format_name(&self) -> &str {
            "terminal"
        }
    }
}

// Create and configure the dependency injection container
fn create_app_container() -> Result<AppContainer> {
    use default_implementations::*;

    // Build the container with all required dependencies
    // We need to create instances directly since the factory returns Box<dyn Analyzer>
    // But our AppContainerBuilder expects concrete types implementing the new trait
    use debtmap::core::injection::RustAnalyzerAdapter;

    let container = AppContainerBuilder::new()
        .with_rust_analyzer(RustAnalyzerAdapter::new())
        .with_debt_scorer(DefaultDebtScorer::new())
        .with_config(DefaultConfigProvider::new())
        .with_priority_calculator(DefaultPriorityCalculator::new())
        .with_json_formatter(JsonFormatter::new())
        .with_markdown_formatter(MarkdownFormatter::new())
        .with_terminal_formatter(TerminalFormatter::new())
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build container: {}", e))?;

    Ok(container)
}

/// Display configuration sources (spec 201)
fn show_config_sources() -> Result<()> {
    use debtmap::config::multi_source::{display_config_sources, load_multi_source_config};

    match load_multi_source_config() {
        Ok(traced) => {
            display_config_sources(&traced);
            Ok(())
        }
        Err(errors) => {
            eprintln!("Error loading configuration:");
            for error in errors {
                eprintln!("  - {}", error);
            }
            Err(anyhow::anyhow!("Configuration loading failed"))
        }
    }
}

/// Print explanation of metric definitions and formulas
fn print_metrics_explanation() {
    println!("\n=== Debtmap Metrics Reference ===\n");

    println!("## Metric Categories (Spec 118)\n");
    println!("Debtmap distinguishes between two types of metrics:\n");

    println!("### Measured Metrics");
    println!("These metrics are directly computed from the AST (Abstract Syntax Tree):");
    println!("  • cyclomatic_complexity: Count of decision points (if, match, while, etc.)");
    println!("  • cognitive_complexity: Weighted measure of code understandability");
    println!("  • nesting_depth: Maximum levels of nested control structures");
    println!("  • loc: Lines of code in the function");
    println!("  • parameter_count: Number of function parameters\n");

    println!("### Estimated Metrics");
    println!("These metrics are heuristic estimates, not precise AST measurements:");
    println!("  • est_branches: Estimated execution paths (formula-based approximation)");
    println!("    Formula: max(nesting_depth, 1) × cyclomatic_complexity ÷ 3");
    println!("    Purpose: Estimate test cases needed for branch coverage");
    println!("    Note: This is an ESTIMATE, not a count from the AST\n");

    println!("## Why the Distinction Matters\n");
    println!("• Measured metrics: Precise, repeatable, suitable for thresholds");
    println!("• Estimated metrics: Approximate, useful for prioritization and heuristics");
    println!("• Use cyclomatic_complexity for code quality gates");
    println!("• Use est_branches for estimating testing effort\n");

    println!("## Terminology Change (Spec 118)\n");
    println!("Previously called 'branches', now renamed to 'est_branches' to:");
    println!("  1. Make it clear this is an estimate, not a measured value");
    println!("  2. Avoid confusion with cyclomatic complexity (actual branches)");
    println!("  3. Set accurate expectations for users\n");

    println!("## Example Usage\n");
    println!("  debtmap analyze . --threshold-complexity 15    # Use measured cyclomatic");
    println!(
        "  debtmap analyze . --top 10                      # Uses est_branches for prioritization"
    );
    println!("  debtmap analyze . --lcov coverage.info          # Coverage vs complexity\n");

    println!("For more details, see: https://docs.debtmap.dev/metrics-reference\n");
}

// Configure rayon global thread pool once at startup
fn configure_thread_pool(jobs: usize) {
    if jobs > 0 {
        if let Err(e) = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build_global()
        {
            // Already configured - this is fine, just ignore
            eprintln!("Note: Thread pool already configured: {}", e);
        }
    }
}

// Main orchestrator function
fn main() -> Result<()> {
    // Support ARGUMENTS environment variable for backward compatibility
    let cli = if let Ok(args_str) = std::env::var("ARGUMENTS") {
        // Parse space-separated arguments from environment variable
        let args: Vec<String> = args_str.split_whitespace().map(String::from).collect();
        // Prepend program name (required by clap)
        let mut full_args = vec![std::env::args()
            .next()
            .unwrap_or_else(|| "debtmap".to_string())];
        full_args.extend(args);
        Cli::parse_from(full_args)
    } else {
        Cli::parse()
    };

    // Configure rayon thread pool early based on CLI arguments
    // Extract jobs parameter from whichever command is being run
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
        Commands::Validate {
            path,
            config,
            coverage_file,
            format,
            output,
            enable_context,
            context_providers,
            disable_context,
            max_debt_density,
            top,
            tail,
            summary: _, // Validate command doesn't use summary yet
            semantic_off,
            explain_score: _,
            verbosity,
            no_parallel,
            jobs,
            show_splits,
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
                max_debt_density,
                top,
                tail,
                semantic_off,
                verbosity,
                no_parallel,
                jobs,
                show_splits,
            };
            debtmap::commands::validate::validate_project(validate_config)?;
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

fn is_automation_mode() -> bool {
    std::env::var("PRODIGY_AUTOMATION")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true")
        || std::env::var("PRODIGY_VALIDATION")
            .unwrap_or_default()
            .eq_ignore_ascii_case("true")
}

/// Extracts and builds configuration from the Analyze command variant.
///
/// This function handles the destructuring of the 65 CLI parameters from the Commands enum
/// and builds all configuration groups. It's separated to keep the main handler focused
/// on coordination.
///
/// # Returns
///
/// Returns a tuple of configuration groups and special flags needed for handler coordination.
///
/// # Architecture Note
///
/// This is a necessary extraction function. While long, its complexity is structural
/// (destructuring + config building) rather than logical. The destructuring must happen
/// somewhere when working with clap's Commands enum.
#[allow(clippy::type_complexity)]
fn extract_analyze_params(
    command: Commands,
) -> Result<(
    analyze_config::PathConfig,
    analyze_config::ThresholdConfig,
    analyze_config::AnalysisFeatureConfig,
    analyze_config::DisplayConfig,
    analyze_config::PerformanceConfig,
    analyze_config::DebugConfig,
    analyze_config::LanguageConfig,
    bool, // explain_metrics flag
    bool, // no_context_aware flag
)> {
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
        summary,
        semantic_off,
        explain_score: _,
        verbosity,
        compact,
        verbose_macro_warnings,
        show_macro_stats,
        group_by_category,
        min_priority,
        min_score,
        filter_categories,
        no_context_aware,
        threshold_preset,
        plain,
        no_parallel,
        jobs,
        no_multi_pass,
        show_attribution,
        detail_level,
        aggregate_only,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
        max_files,
        validate_loc,
        no_public_api_detection,
        public_api_threshold,
        no_pattern_detection,
        patterns,
        pattern_threshold,
        show_pattern_warnings,
        explain_metrics,
        debug_call_graph,
        trace_functions,
        call_graph_stats_only,
        debug_format,
        validate_call_graph,
        show_dependencies,
        no_dependencies,
        max_callers,
        max_callees,
        show_external,
        show_std_lib,
        ast_functional_analysis,
        functional_analysis_profile,
        min_split_methods,
        min_split_lines,
        show_splits,
        no_tui,
        quiet: _,
        show_filter_stats,
    } = command
    {
        // Build configuration groups using pure builder functions
        let path_cfg = build_path_config(
            path,
            output,
            coverage_file,
            max_files,
            min_priority,
            min_score,
            filter_categories,
            min_problematic,
        );

        let threshold_cfg = build_threshold_config(
            threshold_complexity,
            threshold_duplication,
            threshold_preset,
            public_api_threshold,
        );

        let feature_cfg = build_feature_config(
            enable_context,
            context_providers,
            disable_context,
            semantic_off,
            no_pattern_detection,
            patterns,
            pattern_threshold,
            no_god_object,
            no_public_api_detection,
            ast_functional_analysis,
            functional_analysis_profile,
            min_split_methods,
            min_split_lines,
            validate_loc,
            validate_call_graph,
        );

        let formatting_config = create_formatting_config(
            plain,
            show_dependencies,
            no_dependencies,
            max_callers,
            max_callees,
            show_external,
            show_std_lib,
            show_splits,
        );

        let display_cfg = build_display_config(
            format,
            compute_verbosity(verbosity, compact),
            summary,
            top,
            tail,
            group_by_category,
            show_attribution,
            detail_level,
            no_tui,
            show_filter_stats,
            formatting_config,
            no_context_aware,
        );

        let perf_cfg = build_performance_config(
            should_use_parallel(no_parallel),
            get_worker_count(jobs),
            compute_multi_pass(no_multi_pass),
            aggregate_only,
            no_aggregation,
        );

        let debug_cfg = build_debug_config(
            verbose_macro_warnings,
            show_macro_stats,
            debug_call_graph,
            trace_functions,
            call_graph_stats_only,
            debug_format,
            show_pattern_warnings,
            show_dependencies,
            no_dependencies,
        );

        let lang_cfg = build_language_config(
            languages,
            aggregation_method,
            max_callers,
            max_callees,
            show_external,
            show_std_lib,
        );

        Ok((
            path_cfg,
            threshold_cfg,
            feature_cfg,
            display_cfg,
            perf_cfg,
            debug_cfg,
            lang_cfg,
            explain_metrics,
            no_context_aware,
        ))
    } else {
        Err(anyhow::anyhow!("Invalid command: expected Analyze variant"))
    }
}

/// Handles the analyze command (coordination only).
///
/// This is the entry point for the analyze command. It coordinates the three main steps:
/// 1. Extract parameters and build configuration
/// 2. Apply environment setup (side effects)
/// 3. Delegate to analysis handler
///
/// # Architecture
///
/// This function follows the "pure core, imperative shell" pattern and serves as a thin
/// coordination layer (30-40 lines). The heavy lifting is delegated to:
/// - `extract_analyze_params`: Parameter extraction and config building
/// - `apply_environment_setup`: Side effects at the boundary
/// - `handle_analyze`: Core analysis logic
///
/// # Returns
///
/// Returns `Result<(), CliError>` for all CLI-related errors (configuration, validation,
/// or analysis execution errors).
///
/// # Specification
///
/// Implements specs 182 and 206: Refactor handle_analyze_command into composable functions
/// with clear error types. This handler is now 30-40 lines (coordination only), with
/// parameter extraction delegated to `extract_analyze_params` and uses typed errors
/// instead of nested Results.
fn handle_analyze_command(command: Commands) -> Result<(), CliError> {
    // Extract parameters and build configuration groups
    let (
        path_cfg,
        threshold_cfg,
        feature_cfg,
        display_cfg,
        perf_cfg,
        debug_cfg,
        lang_cfg,
        explain_metrics,
        no_context_aware,
    ) = extract_analyze_params(command).map_err(|e| CliError::InvalidCommand(e.to_string()))?;

    // Apply side effects (I/O at edges)
    apply_environment_setup(no_context_aware)
        .map_err(|e| CliError::Config(ConfigError::ValidationFailed(e.to_string())))?;

    // Handle explain-metrics flag (early return for info display)
    if explain_metrics {
        print_metrics_explanation();
        return Ok(());
    }

    // Build final configuration from component configs
    let config = build_analyze_config(
        path_cfg,
        threshold_cfg,
        feature_cfg,
        display_cfg,
        perf_cfg,
        debug_cfg,
        lang_cfg,
    );

    // Delegate to analysis handler - map analysis errors to CLI errors
    debtmap::commands::analyze::handle_analyze(config)
        .map_err(|e| CliError::Config(ConfigError::ValidationFailed(e.to_string())))
}

// Side effect function for environment setup (I/O at edges)
fn apply_environment_setup(no_context_aware: bool) -> Result<()> {
    if !no_context_aware {
        std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
    }

    Ok(())
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

// Pure function to compute effective verbosity (spec 204)
fn compute_verbosity(verbosity: u8, compact: bool) -> u8 {
    if compact {
        0 // Compact mode uses minimum verbosity
    } else {
        verbosity
    }
}

// Pure function to check if single-pass mode is enabled via env var (spec 202)
fn is_single_pass_env_enabled() -> bool {
    std::env::var("DEBTMAP_SINGLE_PASS")
        .ok()
        .and_then(|v| v.parse::<bool>().ok().or_else(|| Some(v == "1")))
        .unwrap_or(false)
}

// Pure function to compute multi-pass setting (spec 204)
fn compute_multi_pass(no_multi_pass: bool) -> bool {
    if no_multi_pass {
        return false;
    }
    !is_single_pass_env_enabled()
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

fn convert_threshold_preset(
    preset: Option<debtmap::cli::ThresholdPreset>,
) -> Option<debtmap::cli::ThresholdPreset> {
    preset
}

fn convert_output_format(format: debtmap::cli::OutputFormat) -> debtmap::cli::OutputFormat {
    format
}

// Pure function to create formatting configuration
#[allow(clippy::too_many_arguments)]
fn create_formatting_config(
    plain: bool,
    _show_dependencies: bool,
    _no_dependencies: bool,
    max_callers: usize,
    max_callees: usize,
    show_external: bool,
    show_std_lib: bool,
    show_splits: bool,
) -> FormattingConfig {
    use debtmap::config::CallerCalleeConfig;

    let color_mode = if plain {
        ColorMode::Never
    } else {
        // Get color mode from environment
        let base_config = FormattingConfig::from_env();
        base_config.color
    };

    let caller_callee = CallerCalleeConfig {
        max_callers,
        max_callees,
        show_external,
        show_std_lib,
    };

    FormattingConfig::with_caller_callee(color_mode, caller_callee).with_show_splits(show_splits)
}

/// Configuration groups for analyze command (spec 204)
mod analyze_config {
    use std::path::PathBuf;

    /// Path and file configuration for analysis
    #[derive(Debug, Clone)]
    pub struct PathConfig {
        pub path: PathBuf,
        pub output: Option<PathBuf>,
        pub coverage_file: Option<PathBuf>,
        pub max_files: Option<usize>,
        pub min_priority: Option<String>,
        pub min_score: Option<f64>,
        pub filter_categories: Option<Vec<String>>,
        pub min_problematic: Option<usize>,
    }

    impl PathConfig {
        #[allow(dead_code)]
        pub fn builder(path: PathBuf) -> PathConfigBuilder {
            PathConfigBuilder {
                path,
                output: None,
                coverage_file: None,
                max_files: None,
                min_priority: None,
                min_score: None,
                filter_categories: None,
                min_problematic: None,
            }
        }
    }

    #[allow(dead_code)]
    pub struct PathConfigBuilder {
        path: PathBuf,
        output: Option<PathBuf>,
        coverage_file: Option<PathBuf>,
        max_files: Option<usize>,
        min_priority: Option<String>,
        min_score: Option<f64>,
        filter_categories: Option<Vec<String>>,
        min_problematic: Option<usize>,
    }

    #[allow(dead_code)]
    impl PathConfigBuilder {
        pub fn output(mut self, output: Option<PathBuf>) -> Self {
            self.output = output;
            self
        }
        pub fn coverage_file(mut self, file: Option<PathBuf>) -> Self {
            self.coverage_file = file;
            self
        }
        pub fn max_files(mut self, max: Option<usize>) -> Self {
            self.max_files = max;
            self
        }
        pub fn min_priority(mut self, priority: Option<String>) -> Self {
            self.min_priority = priority;
            self
        }
        pub fn min_score(mut self, score: Option<f64>) -> Self {
            self.min_score = score;
            self
        }
        pub fn filter_categories(mut self, categories: Option<Vec<String>>) -> Self {
            self.filter_categories = categories;
            self
        }
        pub fn min_problematic(mut self, min: Option<usize>) -> Self {
            self.min_problematic = min;
            self
        }
        pub fn build(self) -> PathConfig {
            PathConfig {
                path: self.path,
                output: self.output,
                coverage_file: self.coverage_file,
                max_files: self.max_files,
                min_priority: self.min_priority,
                min_score: self.min_score,
                filter_categories: self.filter_categories,
                min_problematic: self.min_problematic,
            }
        }
    }

    /// Analysis thresholds configuration
    #[derive(Debug, Clone)]
    pub struct ThresholdConfig {
        pub complexity: u32,
        pub duplication: usize,
        pub preset: Option<debtmap::cli::ThresholdPreset>,
        pub public_api_threshold: f32,
    }

    impl ThresholdConfig {
        #[allow(dead_code)]
        pub fn builder(complexity: u32, duplication: usize) -> ThresholdConfigBuilder {
            ThresholdConfigBuilder {
                complexity,
                duplication,
                preset: None,
                public_api_threshold: 0.5,
            }
        }
    }

    #[allow(dead_code)]
    pub struct ThresholdConfigBuilder {
        complexity: u32,
        duplication: usize,
        preset: Option<debtmap::cli::ThresholdPreset>,
        public_api_threshold: f32,
    }

    #[allow(dead_code)]
    impl ThresholdConfigBuilder {
        pub fn preset(mut self, preset: Option<debtmap::cli::ThresholdPreset>) -> Self {
            self.preset = preset;
            self
        }
        pub fn public_api_threshold(mut self, threshold: f32) -> Self {
            self.public_api_threshold = threshold;
            self
        }
        pub fn build(self) -> ThresholdConfig {
            ThresholdConfig {
                complexity: self.complexity,
                duplication: self.duplication,
                preset: self.preset,
                public_api_threshold: self.public_api_threshold,
            }
        }
    }

    /// Feature flags for analysis options
    #[derive(Debug, Clone)]
    pub struct AnalysisFeatureConfig {
        pub enable_context: bool,
        pub context_providers: Option<Vec<String>>,
        pub disable_context: Option<Vec<String>>,
        pub semantic_off: bool,
        pub no_pattern_detection: bool,
        pub patterns: Option<Vec<String>>,
        pub pattern_threshold: f32,
        pub no_god_object: bool,
        pub no_public_api_detection: bool,
        pub ast_functional_analysis: bool,
        pub functional_analysis_profile: Option<debtmap::cli::FunctionalAnalysisProfile>,
        pub min_split_methods: usize,
        pub min_split_lines: usize,
        pub validate_loc: bool,
        pub validate_call_graph: bool,
    }

    impl AnalysisFeatureConfig {
        #[allow(dead_code)]
        pub fn builder() -> AnalysisFeatureConfigBuilder {
            AnalysisFeatureConfigBuilder::default()
        }
    }

    #[derive(Default)]
    #[allow(dead_code)]
    pub struct AnalysisFeatureConfigBuilder {
        enable_context: bool,
        context_providers: Option<Vec<String>>,
        disable_context: Option<Vec<String>>,
        semantic_off: bool,
        no_pattern_detection: bool,
        patterns: Option<Vec<String>>,
        pattern_threshold: f32,
        no_god_object: bool,
        no_public_api_detection: bool,
        ast_functional_analysis: bool,
        functional_analysis_profile: Option<debtmap::cli::FunctionalAnalysisProfile>,
        min_split_methods: usize,
        min_split_lines: usize,
        validate_loc: bool,
        validate_call_graph: bool,
    }

    #[allow(dead_code)]
    impl AnalysisFeatureConfigBuilder {
        pub fn enable_context(mut self, enable: bool) -> Self {
            self.enable_context = enable;
            self
        }
        pub fn context_providers(mut self, providers: Option<Vec<String>>) -> Self {
            self.context_providers = providers;
            self
        }
        pub fn disable_context(mut self, disable: Option<Vec<String>>) -> Self {
            self.disable_context = disable;
            self
        }
        pub fn semantic_off(mut self, off: bool) -> Self {
            self.semantic_off = off;
            self
        }
        pub fn no_pattern_detection(mut self, no: bool) -> Self {
            self.no_pattern_detection = no;
            self
        }
        pub fn patterns(mut self, patterns: Option<Vec<String>>) -> Self {
            self.patterns = patterns;
            self
        }
        pub fn pattern_threshold(mut self, threshold: f32) -> Self {
            self.pattern_threshold = threshold;
            self
        }
        pub fn no_god_object(mut self, no: bool) -> Self {
            self.no_god_object = no;
            self
        }
        pub fn no_public_api_detection(mut self, no: bool) -> Self {
            self.no_public_api_detection = no;
            self
        }
        pub fn ast_functional_analysis(mut self, enable: bool) -> Self {
            self.ast_functional_analysis = enable;
            self
        }
        pub fn functional_analysis_profile(
            mut self,
            profile: Option<debtmap::cli::FunctionalAnalysisProfile>,
        ) -> Self {
            self.functional_analysis_profile = profile;
            self
        }
        pub fn min_split_methods(mut self, min: usize) -> Self {
            self.min_split_methods = min;
            self
        }
        pub fn min_split_lines(mut self, min: usize) -> Self {
            self.min_split_lines = min;
            self
        }
        pub fn validate_loc(mut self, validate: bool) -> Self {
            self.validate_loc = validate;
            self
        }
        pub fn validate_call_graph(mut self, validate: bool) -> Self {
            self.validate_call_graph = validate;
            self
        }
        pub fn build(self) -> AnalysisFeatureConfig {
            AnalysisFeatureConfig {
                enable_context: self.enable_context,
                context_providers: self.context_providers,
                disable_context: self.disable_context,
                semantic_off: self.semantic_off,
                no_pattern_detection: self.no_pattern_detection,
                patterns: self.patterns,
                pattern_threshold: self.pattern_threshold,
                no_god_object: self.no_god_object,
                no_public_api_detection: self.no_public_api_detection,
                ast_functional_analysis: self.ast_functional_analysis,
                functional_analysis_profile: self.functional_analysis_profile,
                min_split_methods: self.min_split_methods,
                min_split_lines: self.min_split_lines,
                validate_loc: self.validate_loc,
                validate_call_graph: self.validate_call_graph,
            }
        }
    }

    /// Display and output formatting configuration
    #[derive(Debug, Clone)]
    pub struct DisplayConfig {
        pub format: debtmap::cli::OutputFormat,
        pub verbosity: u8,
        pub summary: bool,
        pub top: Option<usize>,
        pub tail: Option<usize>,
        pub group_by_category: bool,
        pub show_attribution: bool,
        pub detail_level: Option<String>,
        pub no_tui: bool,
        pub show_filter_stats: bool,
        pub formatting_config: super::FormattingConfig,
        pub no_context_aware: bool,
    }

    /// Performance and parallelization settings
    #[derive(Debug, Clone)]
    pub struct PerformanceConfig {
        pub parallel: bool,
        pub jobs: usize,
        pub multi_pass: bool,
        pub aggregate_only: bool,
        pub no_aggregation: bool,
    }

    /// Debug and diagnostic settings
    #[derive(Debug, Clone)]
    pub struct DebugConfig {
        pub verbose_macro_warnings: bool,
        pub show_macro_stats: bool,
        pub debug_call_graph: bool,
        pub trace_functions: Option<Vec<String>>,
        pub call_graph_stats_only: bool,
        pub debug_format: debtmap::cli::DebugFormatArg,
        pub show_pattern_warnings: bool,
        pub show_dependencies: bool,
        pub no_dependencies: bool,
    }

    /// Language-specific settings
    #[derive(Debug, Clone)]
    pub struct LanguageConfig {
        pub languages: Option<Vec<String>>,
        pub aggregation_method: Option<String>,
        pub max_callers: usize,
        pub max_callees: usize,
        pub show_external: bool,
        pub show_std_lib: bool,
    }
}

/// Builds PathConfig from command-line parameters (pure, spec 182)
#[allow(clippy::too_many_arguments)]
fn build_path_config(
    path: std::path::PathBuf,
    output: Option<std::path::PathBuf>,
    coverage_file: Option<std::path::PathBuf>,
    max_files: Option<usize>,
    min_priority: Option<String>,
    min_score: Option<f64>,
    filter_categories: Option<Vec<String>>,
    min_problematic: Option<usize>,
) -> analyze_config::PathConfig {
    analyze_config::PathConfig {
        path,
        output,
        coverage_file,
        max_files,
        min_priority,
        min_score,
        filter_categories,
        min_problematic,
    }
}

/// Builds ThresholdConfig from command-line parameters (pure, spec 182)
fn build_threshold_config(
    complexity: u32,
    duplication: usize,
    preset: Option<debtmap::cli::ThresholdPreset>,
    public_api_threshold: f32,
) -> analyze_config::ThresholdConfig {
    analyze_config::ThresholdConfig {
        complexity,
        duplication,
        preset,
        public_api_threshold,
    }
}

/// Builds AnalysisFeatureConfig from command-line parameters (pure, spec 182)
#[allow(clippy::too_many_arguments)]
fn build_feature_config(
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
    semantic_off: bool,
    no_pattern_detection: bool,
    patterns: Option<Vec<String>>,
    pattern_threshold: f32,
    no_god_object: bool,
    no_public_api_detection: bool,
    ast_functional_analysis: bool,
    functional_analysis_profile: Option<debtmap::cli::FunctionalAnalysisProfile>,
    min_split_methods: usize,
    min_split_lines: usize,
    validate_loc: bool,
    validate_call_graph: bool,
) -> analyze_config::AnalysisFeatureConfig {
    analyze_config::AnalysisFeatureConfig {
        enable_context,
        context_providers,
        disable_context,
        semantic_off,
        no_pattern_detection,
        patterns,
        pattern_threshold,
        no_god_object,
        no_public_api_detection,
        ast_functional_analysis,
        functional_analysis_profile,
        min_split_methods,
        min_split_lines,
        validate_loc,
        validate_call_graph,
    }
}

/// Builds DisplayConfig from command-line parameters (pure, spec 182)
#[allow(clippy::too_many_arguments)]
fn build_display_config(
    format: debtmap::cli::OutputFormat,
    verbosity: u8,
    summary: bool,
    top: Option<usize>,
    tail: Option<usize>,
    group_by_category: bool,
    show_attribution: bool,
    detail_level: Option<String>,
    no_tui: bool,
    show_filter_stats: bool,
    formatting_config: FormattingConfig,
    no_context_aware: bool,
) -> analyze_config::DisplayConfig {
    analyze_config::DisplayConfig {
        format,
        verbosity,
        summary,
        top,
        tail,
        group_by_category,
        show_attribution,
        detail_level,
        no_tui,
        show_filter_stats,
        formatting_config,
        no_context_aware,
    }
}

/// Builds PerformanceConfig from command-line parameters (pure, spec 182)
fn build_performance_config(
    parallel: bool,
    jobs: usize,
    multi_pass: bool,
    aggregate_only: bool,
    no_aggregation: bool,
) -> analyze_config::PerformanceConfig {
    analyze_config::PerformanceConfig {
        parallel,
        jobs,
        multi_pass,
        aggregate_only,
        no_aggregation,
    }
}

/// Builds DebugConfig from command-line parameters (pure, spec 182)
#[allow(clippy::too_many_arguments)]
fn build_debug_config(
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    debug_call_graph: bool,
    trace_functions: Option<Vec<String>>,
    call_graph_stats_only: bool,
    debug_format: debtmap::cli::DebugFormatArg,
    show_pattern_warnings: bool,
    show_dependencies: bool,
    no_dependencies: bool,
) -> analyze_config::DebugConfig {
    analyze_config::DebugConfig {
        verbose_macro_warnings,
        show_macro_stats,
        debug_call_graph,
        trace_functions,
        call_graph_stats_only,
        debug_format,
        show_pattern_warnings,
        show_dependencies,
        no_dependencies,
    }
}

/// Builds LanguageConfig from command-line parameters (pure, spec 182)
fn build_language_config(
    languages: Option<Vec<String>>,
    aggregation_method: Option<String>,
    max_callers: usize,
    max_callees: usize,
    show_external: bool,
    show_std_lib: bool,
) -> analyze_config::LanguageConfig {
    analyze_config::LanguageConfig {
        languages,
        aggregation_method,
        max_callers,
        max_callees,
        show_external,
        show_std_lib,
    }
}

/// Build analyze configuration from grouped configuration structs (spec 204)
fn build_analyze_config(
    p: analyze_config::PathConfig,
    t: analyze_config::ThresholdConfig,
    f: analyze_config::AnalysisFeatureConfig,
    d: analyze_config::DisplayConfig,
    pf: analyze_config::PerformanceConfig,
    db: analyze_config::DebugConfig,
    l: analyze_config::LanguageConfig,
) -> debtmap::commands::analyze::AnalyzeConfig {
    debtmap::commands::analyze::AnalyzeConfig {
        path: p.path,
        output: p.output,
        coverage_file: p.coverage_file,
        max_files: p.max_files,
        min_priority: convert_min_priority(p.min_priority),
        min_score: p.min_score,
        filter_categories: convert_filter_categories(p.filter_categories),
        min_problematic: p.min_problematic,
        threshold_complexity: t.complexity,
        threshold_duplication: t.duplication,
        threshold_preset: convert_threshold_preset(t.preset),
        public_api_threshold: t.public_api_threshold,
        format: convert_output_format(d.format),
        verbosity: d.verbosity,
        summary: d.summary,
        top: d.top,
        tail: d.tail,
        group_by_category: d.group_by_category,
        show_attribution: d.show_attribution,
        detail_level: d.detail_level,
        no_tui: d.no_tui,
        show_filter_stats: d.show_filter_stats,
        _formatting_config: d.formatting_config,
        no_context_aware: d.no_context_aware,
        enable_context: f.enable_context,
        context_providers: convert_context_providers(f.context_providers),
        disable_context: convert_disable_context(f.disable_context),
        semantic_off: f.semantic_off,
        no_pattern_detection: f.no_pattern_detection,
        patterns: f.patterns,
        pattern_threshold: f.pattern_threshold,
        no_god_object: f.no_god_object,
        no_public_api_detection: f.no_public_api_detection,
        validate_loc: f.validate_loc,
        validate_call_graph: f.validate_call_graph,
        ast_functional_analysis: f.ast_functional_analysis,
        functional_analysis_profile: f.functional_analysis_profile,
        min_split_methods: f.min_split_methods,
        min_split_lines: f.min_split_lines,
        parallel: pf.parallel,
        jobs: pf.jobs,
        multi_pass: pf.multi_pass,
        aggregate_only: pf.aggregate_only,
        no_aggregation: pf.no_aggregation,
        verbose_macro_warnings: db.verbose_macro_warnings,
        show_macro_stats: db.show_macro_stats,
        debug_call_graph: db.debug_call_graph,
        trace_functions: db.trace_functions,
        call_graph_stats_only: db.call_graph_stats_only,
        debug_format: db.debug_format,
        show_pattern_warnings: db.show_pattern_warnings,
        show_dependencies: db.show_dependencies,
        no_dependencies: db.no_dependencies,
        languages: convert_languages(l.languages),
        aggregation_method: l.aggregation_method,
        max_callers: l.max_callers,
        max_callees: l.max_callees,
        show_external: l.show_external,
        show_std_lib: l.show_std_lib,
    }
}

/// Pure function to convert DebtmapJsonInput to UnifiedAnalysis
/// Splits merged DebtItem enum into separate function and file vectors
fn json_to_analysis(
    json: debtmap::commands::compare_debtmap::DebtmapJsonInput,
) -> debtmap::priority::UnifiedAnalysis {
    use debtmap::priority::{call_graph::CallGraph, DebtItem, UnifiedAnalysis};
    use im::Vector;

    let mut items = Vector::new();
    let mut file_items = Vector::new();

    // Split DebtItems into function and file items
    for item in json.items {
        match item {
            DebtItem::Function(func) => items.push_back(*func),
            DebtItem::File(file) => file_items.push_back(*file),
        }
    }

    // Create UnifiedAnalysis with empty call graph and data flow graph
    // These aren't serialized in JSON output anyway
    let call_graph = CallGraph::new();

    UnifiedAnalysis {
        items,
        file_items,
        total_impact: json.total_impact,
        total_debt_score: json.total_debt_score,
        debt_density: json.debt_density,
        total_lines_of_code: json.total_lines_of_code,
        call_graph: call_graph.clone(),
        data_flow_graph: debtmap::data_flow::DataFlowGraph::from_call_graph(call_graph),
        overall_coverage: json.overall_coverage,
        has_coverage_data: json.overall_coverage.is_some(),
        timings: None,
    }
}

// Handle compare command
fn handle_compare_command(
    before: &Path,
    after: &Path,
    plan: Option<&Path>,
    target_location: Option<String>,
    format: debtmap::cli::OutputFormat,
    output: Option<&Path>,
) -> Result<()> {
    use debtmap::commands::compare_debtmap::DebtmapJsonInput;
    use debtmap::comparison::{Comparator, PlanParser};
    use std::fs;

    // Extract target location from plan or use explicit location
    let target = if let Some(plan_path) = plan {
        Some(PlanParser::extract_target_location(plan_path)?)
    } else {
        target_location
    };

    // Load JSON output and convert to UnifiedAnalysis
    let before_content = fs::read_to_string(before)?;
    let before_json: DebtmapJsonInput = serde_json::from_str(&before_content)?;
    let before_results = json_to_analysis(before_json);

    let after_content = fs::read_to_string(after)?;
    let after_json: DebtmapJsonInput = serde_json::from_str(&after_content)?;
    let after_results = json_to_analysis(after_json);

    // Perform comparison
    let comparator = Comparator::new(before_results, after_results, target);
    let comparison = comparator.compare()?;

    // Output results
    let output_str = match format {
        debtmap::cli::OutputFormat::Json => serde_json::to_string_pretty(&comparison)?,
        debtmap::cli::OutputFormat::Markdown => format_comparison_markdown(&comparison),
        debtmap::cli::OutputFormat::Html => format_comparison_markdown(&comparison),
        debtmap::cli::OutputFormat::Terminal => {
            print_comparison_terminal(&comparison);
            return Ok(());
        }
    };

    // Write to file or stdout
    if let Some(output_path) = output {
        fs::write(output_path, output_str)?;
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

// Format comparison as markdown
fn format_comparison_markdown(comparison: &debtmap::comparison::ComparisonResult) -> String {
    use debtmap::comparison::{DebtTrend, TargetStatus};

    let mut md = String::new();

    md.push_str("# Debtmap Comparison Report\n\n");
    md.push_str(&format!(
        "**Date**: {}\n\n",
        comparison.metadata.comparison_date
    ));

    if let Some(target) = &comparison.target_item {
        md.push_str("## Target Item Analysis\n\n");

        let status_icon = match target.status {
            TargetStatus::Resolved => "[OK]",
            TargetStatus::Improved => "[OK]",
            TargetStatus::Unchanged => "[WARNING]",
            TargetStatus::Regressed => "[ERROR]",
            TargetStatus::NotFoundBefore | TargetStatus::NotFound => "[UNKNOWN]",
        };

        md.push_str(&format!(
            "{} **Status**: {:?}\n\n",
            status_icon, target.status
        ));
        md.push_str(&format!("**Location**: `{}`\n\n", target.location));

        md.push_str("### Before\n");
        md.push_str(&format!("- **Score**: {:.1}\n", target.before.score));
        md.push_str(&format!(
            "- **Complexity**: Cyclomatic {}, Cognitive {}\n",
            target.before.cyclomatic_complexity, target.before.cognitive_complexity
        ));
        md.push_str(&format!("- **Coverage**: {:.1}%\n", target.before.coverage));
        md.push_str(&format!(
            "- **Function Length**: {} lines\n\n",
            target.before.function_length
        ));

        if let Some(after_metrics) = &target.after {
            md.push_str("### After\n");
            md.push_str(&format!("- **Score**: {:.1}\n", after_metrics.score));
            md.push_str(&format!(
                "- **Complexity**: Cyclomatic {}, Cognitive {}\n",
                after_metrics.cyclomatic_complexity, after_metrics.cognitive_complexity
            ));
            md.push_str(&format!("- **Coverage**: {:.1}%\n", after_metrics.coverage));
            md.push_str(&format!(
                "- **Function Length**: {} lines\n\n",
                after_metrics.function_length
            ));
        }

        md.push_str("### Improvements\n");
        md.push_str(&format!(
            "- Score reduced by **{:.1}%**\n",
            target.improvements.score_reduction_pct
        ));
        md.push_str(&format!(
            "- Complexity reduced by **{:.1}%**\n",
            target.improvements.complexity_reduction_pct
        ));
        md.push_str(&format!(
            "- Coverage improved by **{:.1}%**\n\n",
            target.improvements.coverage_improvement_pct
        ));
    }

    md.push_str("## Project Health\n\n");

    let trend_icon = match comparison.summary.overall_debt_trend {
        DebtTrend::Improving => "[IMPROVING]",
        DebtTrend::Stable => "[STABLE]",
        DebtTrend::Regressing => "[REGRESSING]",
    };

    md.push_str(&format!(
        "### Overall Trend: {} {:?}\n\n",
        trend_icon, comparison.summary.overall_debt_trend
    ));
    md.push_str(&format!(
        "- Total debt: {:.1} → {:.1} ({:+.1}%)\n",
        comparison.project_health.before.total_debt_score,
        comparison.project_health.after.total_debt_score,
        comparison.project_health.changes.debt_score_change_pct
    ));
    md.push_str(&format!(
        "- Critical items: {} → {} ({:+})\n",
        comparison.project_health.before.critical_items,
        comparison.project_health.after.critical_items,
        comparison.project_health.changes.critical_items_change
    ));

    if !comparison.regressions.is_empty() {
        md.push_str(&format!(
            "\n[WARNING] {} new critical item(s) detected\n\n",
            comparison.regressions.len()
        ));

        md.push_str("### Regressions\n\n");
        for reg in &comparison.regressions {
            md.push_str(&format!("- `{}` (score: {:.1})\n", reg.location, reg.score));
        }
    } else {
        md.push_str("\n[OK] No new critical items introduced\n");
    }

    md.push_str("\n## Summary\n\n");
    if comparison.summary.target_improved {
        md.push_str("[OK] Target item significantly improved\n");
    }
    if comparison.summary.new_critical_count == 0 {
        md.push_str("[OK] No regressions detected\n");
    }
    match comparison.summary.overall_debt_trend {
        DebtTrend::Improving => md.push_str("[OK] Overall project health improved\n"),
        DebtTrend::Stable => md.push_str("[STABLE] Overall project health stable\n"),
        DebtTrend::Regressing => md.push_str("[WARNING] Overall project health declined\n"),
    }

    md
}

// Print comparison to terminal
fn print_comparison_terminal(comparison: &debtmap::comparison::ComparisonResult) {
    println!("{}", format_comparison_markdown(comparison));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Tests for pure functions (spec 204)
    #[test]
    fn test_compute_verbosity_compact() {
        assert_eq!(compute_verbosity(1, true), 0);
    }

    #[test]
    fn test_compute_verbosity_explicit() {
        assert_eq!(compute_verbosity(2, false), 2);
    }

    #[test]
    fn test_compute_verbosity_default() {
        assert_eq!(compute_verbosity(1, false), 1);
    }

    #[test]
    fn test_compute_multi_pass_disabled() {
        assert!(!compute_multi_pass(true));
    }

    #[test]
    fn test_compute_multi_pass_enabled() {
        std::env::remove_var("DEBTMAP_SINGLE_PASS");
        assert!(compute_multi_pass(false));
    }

    #[test]
    fn test_is_single_pass_env_disabled() {
        std::env::remove_var("DEBTMAP_SINGLE_PASS");
        assert!(!is_single_pass_env_enabled());
    }

    #[test]
    fn test_is_single_pass_env_true() {
        std::env::set_var("DEBTMAP_SINGLE_PASS", "true");
        assert!(is_single_pass_env_enabled());
        std::env::remove_var("DEBTMAP_SINGLE_PASS");
    }

    #[test]
    fn test_is_single_pass_env_numeric() {
        std::env::set_var("DEBTMAP_SINGLE_PASS", "1");
        assert!(is_single_pass_env_enabled());
        std::env::remove_var("DEBTMAP_SINGLE_PASS");
    }

    // Tests for configuration builders (spec 204)
    #[test]
    fn test_path_config_builder() {
        let config = analyze_config::PathConfig::builder(PathBuf::from("/test"))
            .output(Some(PathBuf::from("/output")))
            .max_files(Some(100))
            .build();
        assert_eq!(config.path, PathBuf::from("/test"));
        assert_eq!(config.output, Some(PathBuf::from("/output")));
        assert_eq!(config.max_files, Some(100));
    }

    #[test]
    fn test_threshold_config_builder() {
        let config = analyze_config::ThresholdConfig::builder(10, 50)
            .public_api_threshold(0.8)
            .build();
        assert_eq!(config.complexity, 10);
        assert_eq!(config.duplication, 50);
        assert_eq!(config.public_api_threshold, 0.8);
    }

    #[test]
    fn test_analysis_feature_config_builder() {
        let config = analyze_config::AnalysisFeatureConfig::builder()
            .enable_context(true)
            .semantic_off(false)
            .ast_functional_analysis(true)
            .build();
        assert!(config.enable_context);
        assert!(!config.semantic_off);
        assert!(config.ast_functional_analysis);
    }

    // Test conversion functions (spec 204)
    #[test]
    fn test_convert_filter_categories_empty() {
        assert_eq!(convert_filter_categories(Some(vec![])), None);
    }

    #[test]
    fn test_convert_filter_categories_non_empty() {
        let cats = vec!["test".to_string()];
        assert_eq!(convert_filter_categories(Some(cats.clone())), Some(cats));
    }

    #[test]
    fn test_convert_languages_empty() {
        assert_eq!(convert_languages(Some(vec![])), None);
    }

    #[test]
    fn test_convert_languages_non_empty() {
        let langs = vec!["rust".to_string()];
        assert_eq!(convert_languages(Some(langs.clone())), Some(langs));
    }
}

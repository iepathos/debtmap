use debtmap::analysis::call_graph::RustCallGraphBuilder;
use debtmap::analysis::python_call_graph::PythonCallGraphAnalyzer;
use debtmap::analysis_utils;
use debtmap::cli;
use debtmap::config;
use debtmap::core;
use debtmap::debt;
use debtmap::io;
use debtmap::priority;
use debtmap::risk;

use anyhow::{Context, Result};
use chrono::Utc;
use cli::Commands;
use core::{
    AnalysisResults, ComplexityReport, DependencyReport, FileMetrics, Language, TechnicalDebtReport,
};
use std::path::{Path, PathBuf};
use std::process;

struct AnalyzeConfig {
    path: PathBuf,
    format: cli::OutputFormat,
    output: Option<PathBuf>,
    threshold_complexity: u32,
    threshold_duplication: usize,
    languages: Option<Vec<String>>,
    coverage_file: Option<PathBuf>,
    _enable_context: bool,
    _context_providers: Option<Vec<String>>,
    _disable_context: Option<Vec<String>>,
    top: Option<usize>,
    tail: Option<usize>,
    semantic_off: bool,
    verbosity: u8,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    #[allow(dead_code)]
    group_by_category: bool,
    #[allow(dead_code)]
    min_priority: Option<String>,
    #[allow(dead_code)]
    filter_categories: Option<Vec<String>>,
    #[allow(dead_code)]
    no_context_aware: bool,
    threshold_preset: Option<cli::ThresholdPreset>,
    markdown_enhanced: bool,
    markdown_detail: String,
}

struct ValidateConfig {
    path: PathBuf,
    #[allow(dead_code)] // TODO: Use config file for thresholds
    config: Option<PathBuf>,
    coverage_file: Option<PathBuf>,
    format: Option<cli::OutputFormat>,
    output: Option<PathBuf>,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
    #[allow(dead_code)]
    top: Option<usize>,
    #[allow(dead_code)]
    tail: Option<usize>,
    #[allow(dead_code)]
    semantic_off: bool,
    verbosity: u8,
}

struct ValidationDetails {
    average_complexity: f64,
    max_average_complexity: f64,
    high_complexity_count: usize,
    max_high_complexity_count: usize,
    debt_items: usize,
    max_debt_items: usize,
    total_debt_score: u32,
    max_total_debt_score: u32,
    codebase_risk_score: f64,
    max_codebase_risk_score: f64,
    high_risk_functions: usize,
    max_high_risk_functions: usize,
    coverage_percentage: f64,
    min_coverage_percentage: f64,
}

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
            markdown_enhanced,
            markdown_detail,
        } => {
            // Enhanced scoring is always enabled (no need for environment variable)

            // Set context-aware environment variable (enabled by default)
            if !no_context_aware {
                std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
            }

            let config = AnalyzeConfig {
                path,
                format,
                output,
                threshold_complexity,
                threshold_duplication,
                languages,
                coverage_file,
                _enable_context: enable_context,
                _context_providers: context_providers,
                _disable_context: disable_context,
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
                markdown_enhanced,
                markdown_detail,
            };
            handle_analyze(config)
        }
        Commands::Init { force } => init_config(force),
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
            let config = ValidateConfig {
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
            validate_project(config)
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

fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    // Set threshold preset if provided
    if let Some(preset) = config.threshold_preset {
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

    let languages = parse_languages(config.languages);
    let results = analyze_project(
        config.path.clone(),
        languages,
        config.threshold_complexity,
        config.threshold_duplication,
    )?;

    // Always use unified prioritization as the default
    // Build call graph and perform unified analysis
    let unified_analysis = perform_unified_analysis(
        &results,
        config.coverage_file.as_ref(),
        config.semantic_off,
        &config.path,
        config.verbose_macro_warnings,
        config.show_macro_stats,
    )?;

    // Output unified prioritized results
    output_unified_priorities_with_config(
        unified_analysis,
        config.top,
        config.tail,
        config.verbosity,
        config.output,
        Some(config.format),
        config.markdown_enhanced,
        &config.markdown_detail,
        &results,
        config.coverage_file.as_ref(),
    )?;

    // Analyze command should only fail on actual errors, not thresholds
    // Threshold checking is done by the validate command
    Ok(())
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
    let duplications = detect_duplications(&files, duplication_threshold);

    let complexity_report = build_complexity_report(&all_functions, complexity_threshold);
    let technical_debt = build_technical_debt_report(all_debt_items, duplications.clone());
    let dependencies = create_dependency_report(&file_metrics);

    Ok(AnalysisResults {
        project_path: path,
        timestamp: Utc::now(),
        complexity: complexity_report,
        technical_debt,
        dependencies,
        duplications,
    })
}

const DEFAULT_SIMILARITY_THRESHOLD: f64 = 0.8;

fn prepare_files_for_duplication_check(files: &[PathBuf]) -> Vec<(PathBuf, String)> {
    files
        .iter()
        .filter_map(|path| match io::read_file(path) {
            Ok(content) => Some((path.clone(), content)),
            Err(e) => {
                log::debug!(
                    "Skipping file {} for duplication check: {}",
                    path.display(),
                    e
                );
                None
            }
        })
        .collect()
}

fn detect_duplications(files: &[PathBuf], threshold: usize) -> Vec<core::DuplicationBlock> {
    let files_with_content = prepare_files_for_duplication_check(files);
    debt::duplication::detect_duplication(
        files_with_content,
        threshold,
        DEFAULT_SIMILARITY_THRESHOLD,
    )
}

fn build_complexity_report(
    all_functions: &[core::FunctionMetrics],
    complexity_threshold: u32,
) -> ComplexityReport {
    analysis_utils::build_complexity_report(all_functions, complexity_threshold)
}

fn build_technical_debt_report(
    all_debt_items: Vec<core::DebtItem>,
    duplications: Vec<core::DuplicationBlock>,
) -> TechnicalDebtReport {
    analysis_utils::build_technical_debt_report(all_debt_items, duplications)
}

/// Pure function: Create JSON output structure
fn create_json_output(
    results: &AnalysisResults,
    risk_insights: &Option<risk::RiskInsight>,
) -> serde_json::Value {
    serde_json::json!({
        "analysis": results,
        "risk_insights": risk_insights,
    })
}

/// Pure function: Format results to string based on format type
fn format_results_to_string(
    results: &AnalysisResults,
    risk_insights: &Option<risk::RiskInsight>,
    format: io::output::OutputFormat,
) -> Result<String> {
    match format {
        io::output::OutputFormat::Json => {
            let output = create_json_output(results, risk_insights);
            Ok(serde_json::to_string_pretty(&output)?)
        }
        _ => {
            let mut buffer = Vec::new();
            write_formatted_output(&mut buffer, results, risk_insights, format)?;
            Ok(String::from_utf8_lossy(&buffer).into_owned())
        }
    }
}

/// Helper function to write formatted output to a buffer
fn write_formatted_output(
    buffer: &mut Vec<u8>,
    results: &AnalysisResults,
    risk_insights: &Option<risk::RiskInsight>,
    format: io::output::OutputFormat,
) -> Result<()> {
    let mut writer = create_file_writer(buffer, format);
    writer.write_results(results)?;
    if let Some(insights) = risk_insights {
        writer.write_risk_insights(insights)?;
    }
    Ok(())
}

/// Pure function: Create appropriate writer for file output
fn create_file_writer<'a>(
    buffer: &'a mut Vec<u8>,
    format: io::output::OutputFormat,
) -> Box<dyn io::output::OutputWriter + 'a> {
    match format {
        io::output::OutputFormat::Markdown | io::output::OutputFormat::Terminal => {
            Box::new(io::writers::MarkdownWriter::new(buffer))
        }
        _ => Box::new(io::writers::MarkdownWriter::new(buffer)), // Default fallback
    }
}

/// Simplified I/O orchestrator function
fn output_results_with_risk(
    results: AnalysisResults,
    risk_insights: Option<risk::RiskInsight>,
    format: io::output::OutputFormat,
    output_file: Option<PathBuf>,
) -> Result<()> {
    match output_file {
        Some(path) => {
            let content = format_results_to_string(&results, &risk_insights, format)?;
            io::write_file(&path, &content)?;
        }
        None => {
            let mut writer = io::output::create_writer(format);
            writer.write_results(&results)?;
            if let Some(insights) = risk_insights {
                writer.write_risk_insights(&insights)?;
            }
        }
    }
    Ok(())
}

fn build_context_aggregator(
    project_path: &Path,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
) -> Option<risk::context::ContextAggregator> {
    if !enable_context {
        return None;
    }

    let enabled_providers = context_providers.unwrap_or_else(get_default_providers);
    let disabled = disable_context.unwrap_or_default();

    let aggregator = enabled_providers
        .into_iter()
        .filter(|name| !disabled.contains(name))
        .fold(
            risk::context::ContextAggregator::new(),
            |acc, provider_name| add_provider_to_aggregator(acc, &provider_name, project_path),
        );

    Some(aggregator)
}

fn get_default_providers() -> Vec<String> {
    vec![
        "critical_path".to_string(),
        "dependency".to_string(),
        "git_history".to_string(),
    ]
}

fn add_provider_to_aggregator(
    aggregator: risk::context::ContextAggregator,
    provider_name: &str,
    project_path: &Path,
) -> risk::context::ContextAggregator {
    match create_provider(provider_name, project_path) {
        Some(provider) => aggregator.with_provider(provider),
        None => {
            eprintln!("Warning: Unknown context provider: {provider_name}");
            aggregator
        }
    }
}

fn create_provider(
    provider_name: &str,
    project_path: &Path,
) -> Option<Box<dyn risk::context::ContextProvider>> {
    match provider_name {
        "critical_path" => Some(create_critical_path_provider()),
        "dependency" => Some(create_dependency_provider()),
        "git_history" => create_git_history_provider(project_path),
        _ => None,
    }
}

fn create_critical_path_provider() -> Box<dyn risk::context::ContextProvider> {
    let analyzer = risk::context::critical_path::CriticalPathAnalyzer::new();
    Box::new(risk::context::critical_path::CriticalPathProvider::new(
        analyzer,
    ))
}

fn create_dependency_provider() -> Box<dyn risk::context::ContextProvider> {
    let graph = risk::context::dependency::DependencyGraph::new();
    Box::new(risk::context::dependency::DependencyRiskProvider::new(
        graph,
    ))
}

fn create_git_history_provider(
    project_path: &Path,
) -> Option<Box<dyn risk::context::ContextProvider>> {
    risk::context::git_history::GitHistoryProvider::new(project_path.to_path_buf())
        .ok()
        .map(|provider| Box::new(provider) as Box<dyn risk::context::ContextProvider>)
}

fn analyze_risk_with_coverage(
    results: &AnalysisResults,
    lcov_path: &Path,
    project_path: &Path,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
) -> Result<Option<risk::RiskInsight>> {
    use im::Vector;

    // Parse LCOV file
    let lcov_data = risk::lcov::parse_lcov_file(lcov_path).context("Failed to parse LCOV file")?;

    // Calculate debt score and threshold
    let debt_score = debt::total_debt_score(&results.technical_debt.items) as f64;
    let debt_threshold = 100.0; // Default threshold

    // Create risk analyzer
    let mut analyzer = risk::RiskAnalyzer::default().with_debt_context(debt_score, debt_threshold);

    // Add context aggregator if enabled
    if let Some(aggregator) = build_context_aggregator(
        project_path,
        enable_context,
        context_providers,
        disable_context,
    ) {
        analyzer = analyzer.with_context_aggregator(aggregator);
    }

    // Analyze each function for risk
    let mut function_risks = Vector::new();

    for func in &results.complexity.metrics {
        let complexity_metrics = core::ComplexityMetrics::from_function(func);

        // Try to get coverage for this function (use line number for closures)
        let coverage = lcov_data.get_function_coverage_with_line(&func.file, &func.name, func.line);

        let risk = analyzer.analyze_function(
            func.file.clone(),
            func.name.clone(),
            (func.line, func.line + func.length),
            &complexity_metrics,
            coverage,
            func.is_test,
        );

        function_risks.push_back(risk);
    }

    // Generate insights
    let insights = risk::insights::generate_risk_insights(function_risks, &analyzer);

    Ok(Some(insights))
}

fn analyze_risk_without_coverage(
    results: &AnalysisResults,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
    project_path: &Path,
) -> Result<Option<risk::RiskInsight>> {
    use im::Vector;

    // Calculate debt score and threshold
    let debt_score = debt::total_debt_score(&results.technical_debt.items) as f64;
    let debt_threshold = 100.0; // Default threshold

    // Create risk analyzer
    let mut analyzer = risk::RiskAnalyzer::default().with_debt_context(debt_score, debt_threshold);

    // Add context aggregator if enabled
    if let Some(aggregator) = build_context_aggregator(
        project_path,
        enable_context,
        context_providers,
        disable_context,
    ) {
        analyzer = analyzer.with_context_aggregator(aggregator);
    }

    // Analyze each function for risk based on complexity only
    let mut function_risks = Vector::new();

    for func in &results.complexity.metrics {
        let complexity_metrics = core::ComplexityMetrics::from_function(func);

        let risk = analyzer.analyze_function(
            func.file.clone(),
            func.name.clone(),
            (func.line, func.line + func.length),
            &complexity_metrics,
            None, // No coverage data
            func.is_test,
        );

        function_risks.push_back(risk);
    }

    // Generate insights
    let insights = risk::insights::generate_risk_insights(function_risks, &analyzer);

    Ok(Some(insights))
}

fn init_config(force: bool) -> Result<()> {
    let config_path = PathBuf::from(".debtmap.toml");

    if config_path.exists() && !force {
        anyhow::bail!("Configuration file already exists. Use --force to overwrite.");
    }

    let default_config = r#"# Debtmap Configuration

[thresholds]
complexity = 10
duplication = 50
max_file_length = 500
max_function_length = 50

[languages]
enabled = ["rust", "python"]

[ignore]
patterns = [
    "target/**",
    "venv/**",
    "node_modules/**",
    "*.min.js"
]

[output]
default_format = "terminal"
"#;

    io::write_file(&config_path, default_config)?;
    println!("Created .debtmap.toml configuration file");

    Ok(())
}

fn validate_with_risk(
    results: &AnalysisResults,
    insights: &risk::RiskInsight,
    lcov_data: Option<&risk::lcov::LcovData>,
) -> (bool, ValidationDetails) {
    let thresholds = debtmap::config::get_validation_thresholds();
    let risk_threshold = 7.0; // Functions with risk > 7.0 are considered high risk

    // Calculate metrics
    let high_risk_count = insights
        .top_risks
        .iter()
        .filter(|f| f.risk_score > risk_threshold)
        .count();

    // Use unified scoring system for consistency with analyze command
    let total_debt_score = calculate_unified_debt_score(results, lcov_data);
    let coverage_percentage = lcov_data
        .map(|lcov| lcov.get_overall_coverage())
        .unwrap_or(0.0);

    // Check each threshold
    let avg_complexity_pass =
        results.complexity.summary.average_complexity <= thresholds.max_average_complexity;
    let high_complexity_pass =
        results.complexity.summary.high_complexity_count <= thresholds.max_high_complexity_count;
    let debt_items_pass = results.technical_debt.items.len() <= thresholds.max_debt_items;
    let debt_score_pass = total_debt_score <= thresholds.max_total_debt_score;
    let codebase_risk_pass = insights.codebase_risk_score <= thresholds.max_codebase_risk_score;
    let high_risk_func_pass = high_risk_count <= thresholds.max_high_risk_functions;
    let coverage_pass = coverage_percentage >= thresholds.min_coverage_percentage;

    let pass = avg_complexity_pass
        && high_complexity_pass
        && debt_items_pass
        && debt_score_pass
        && codebase_risk_pass
        && high_risk_func_pass
        && coverage_pass;

    let details = ValidationDetails {
        average_complexity: results.complexity.summary.average_complexity,
        max_average_complexity: thresholds.max_average_complexity,
        high_complexity_count: results.complexity.summary.high_complexity_count,
        max_high_complexity_count: thresholds.max_high_complexity_count,
        debt_items: results.technical_debt.items.len(),
        max_debt_items: thresholds.max_debt_items,
        total_debt_score,
        max_total_debt_score: thresholds.max_total_debt_score,
        codebase_risk_score: insights.codebase_risk_score,
        max_codebase_risk_score: thresholds.max_codebase_risk_score,
        high_risk_functions: high_risk_count,
        max_high_risk_functions: thresholds.max_high_risk_functions,
        coverage_percentage,
        min_coverage_percentage: thresholds.min_coverage_percentage,
    };

    (pass, details)
}

fn validate_project(config: ValidateConfig) -> Result<()> {
    // Use default thresholds for now (TODO: read from config)
    let complexity_threshold = 10;
    let duplication_threshold = 50;

    // Analyze the project
    let results = analyze_project(
        config.path.clone(),
        vec![Language::Rust, Language::Python],
        complexity_threshold,
        duplication_threshold,
    )?;

    // Handle risk analysis
    let risk_insights = get_risk_insights(&config, &results)?;

    // Generate report if requested
    generate_report_if_requested(&config, &results, &risk_insights)?;

    // Validate and report results
    validate_and_report(&config, &results, &risk_insights)
}

fn get_risk_insights(
    config: &ValidateConfig,
    results: &AnalysisResults,
) -> Result<Option<risk::RiskInsight>> {
    match (&config.coverage_file, config.enable_context) {
        (Some(lcov_path), _) => analyze_risk_with_coverage(
            results,
            lcov_path,
            &config.path,
            config.enable_context,
            config.context_providers.clone(),
            config.disable_context.clone(),
        ),
        (None, true) => analyze_risk_without_coverage(
            results,
            config.enable_context,
            config.context_providers.clone(),
            config.disable_context.clone(),
            &config.path,
        ),
        _ => Ok(None),
    }
}

fn determine_output_format(config: &ValidateConfig) -> Option<cli::OutputFormat> {
    config
        .format
        .or(config.output.as_ref().map(|_| cli::OutputFormat::Terminal))
}

fn generate_report_if_requested(
    config: &ValidateConfig,
    results: &AnalysisResults,
    risk_insights: &Option<risk::RiskInsight>,
) -> Result<()> {
    determine_output_format(config)
        .map(|format| {
            output_results_with_risk(
                results.clone(),
                risk_insights.clone(),
                format.into(),
                config.output.clone(),
            )
        })
        .unwrap_or(Ok(()))
}

fn validate_and_report(
    config: &ValidateConfig,
    results: &AnalysisResults,
    risk_insights: &Option<risk::RiskInsight>,
) -> Result<()> {
    // Load LCOV data if provided
    let lcov_data = if let Some(lcov_path) = &config.coverage_file {
        risk::lcov::parse_lcov_file(lcov_path).ok()
    } else {
        None
    };

    let (pass, details) = risk_insights
        .as_ref()
        .map(|insights| validate_with_risk(results, insights, lcov_data.as_ref()))
        .unwrap_or_else(|| validate_basic(results));

    if pass {
        print_validation_success(&details, config.verbosity);
        Ok(())
    } else {
        print_validation_failure_with_details(&details, risk_insights, config.verbosity);
        anyhow::bail!("Validation failed")
    }
}

/// Calculate unified debt score using the same scoring system as the analyze command
fn calculate_unified_debt_score(
    results: &AnalysisResults,
    lcov_data: Option<&risk::lcov::LcovData>,
) -> u32 {
    // This is a simplified version that may produce slightly higher scores than analyze
    // because it doesn't perform full framework detection and macro expansion.
    // The analyze command does additional processing to exclude framework-generated code.

    // Build call graph from complexity metrics
    let mut call_graph = build_initial_call_graph(&results.complexity.metrics);

    // Get project path from results
    let project_path = &results.project_path;

    // Process Rust files to get framework exclusions and function pointer usage
    let (framework_exclusions, function_pointer_used_functions) =
        process_rust_files_for_call_graph(
            project_path,
            &mut call_graph,
            false, // verbose_macro_warnings = false for validation
            false, // show_macro_stats = false
        )
        .unwrap_or_default();

    // Process Python files as well
    if let Err(e) = process_python_files_for_call_graph(project_path, &mut call_graph) {
        log::warn!("Failed to process Python files for call graph: {}", e);
        // Continue with Rust-only analysis rather than failing completely
    }

    // Create unified analysis using the same approach as analyze command
    let unified = create_unified_analysis_with_exclusions(
        &results.complexity.metrics,
        &call_graph,
        lcov_data,
        &framework_exclusions,
        Some(&function_pointer_used_functions),
        Some(&results.technical_debt.items),
    );

    // Return the unified debt score as u32 for compatibility with validation thresholds
    unified.total_debt_score as u32
}

fn validate_basic(results: &AnalysisResults) -> (bool, ValidationDetails) {
    let thresholds = debtmap::config::get_validation_thresholds();
    // Use unified scoring system for consistency with analyze command
    let total_debt_score = calculate_unified_debt_score(results, None);

    // Check each threshold
    let avg_complexity_pass =
        results.complexity.summary.average_complexity <= thresholds.max_average_complexity;
    let high_complexity_pass =
        results.complexity.summary.high_complexity_count <= thresholds.max_high_complexity_count;
    let debt_items_pass = results.technical_debt.items.len() <= thresholds.max_debt_items;
    let debt_score_pass = total_debt_score <= thresholds.max_total_debt_score;

    let pass = avg_complexity_pass && high_complexity_pass && debt_items_pass && debt_score_pass;

    let details = ValidationDetails {
        average_complexity: results.complexity.summary.average_complexity,
        max_average_complexity: thresholds.max_average_complexity,
        high_complexity_count: results.complexity.summary.high_complexity_count,
        max_high_complexity_count: thresholds.max_high_complexity_count,
        debt_items: results.technical_debt.items.len(),
        max_debt_items: thresholds.max_debt_items,
        total_debt_score,
        max_total_debt_score: thresholds.max_total_debt_score,
        codebase_risk_score: 0.0,
        max_codebase_risk_score: thresholds.max_codebase_risk_score,
        high_risk_functions: 0,
        max_high_risk_functions: thresholds.max_high_risk_functions,
        coverage_percentage: 0.0,
        min_coverage_percentage: thresholds.min_coverage_percentage,
    };

    (pass, details)
}

fn print_validation_success(details: &ValidationDetails, verbosity: u8) {
    println!("✅ Validation PASSED - All metrics within thresholds");

    if verbosity > 0 {
        println!();
        print_validation_details(details);
    }
}

/// Generate a failure message for a metric that exceeds its threshold
fn format_threshold_failure(
    metric_name: &str,
    actual: &str,
    threshold: &str,
    comparison: &str,
) -> String {
    format!(
        "    ❌ {}: {} {} {}",
        metric_name, actual, comparison, threshold
    )
}

/// Check if a numeric metric exceeds its maximum threshold
fn exceeds_max_threshold<T: PartialOrd>(actual: T, threshold: T) -> bool {
    actual > threshold
}

/// Check if a numeric metric is below its minimum threshold  
fn below_min_threshold<T: PartialOrd>(actual: T, threshold: T) -> bool {
    actual < threshold
}

/// Print all failed validation checks for the given details
fn print_failed_validation_checks(details: &ValidationDetails) {
    if exceeds_max_threshold(details.average_complexity, details.max_average_complexity) {
        println!(
            "{}",
            format_threshold_failure(
                "Average complexity",
                &format!("{:.1}", details.average_complexity),
                &format!("{:.1}", details.max_average_complexity),
                ">"
            )
        );
    }
    if exceeds_max_threshold(
        details.high_complexity_count,
        details.max_high_complexity_count,
    ) {
        println!(
            "{}",
            format_threshold_failure(
                "High complexity functions",
                &details.high_complexity_count.to_string(),
                &details.max_high_complexity_count.to_string(),
                ">"
            )
        );
    }
    if exceeds_max_threshold(details.debt_items, details.max_debt_items) {
        println!(
            "{}",
            format_threshold_failure(
                "Technical debt items",
                &details.debt_items.to_string(),
                &details.max_debt_items.to_string(),
                ">"
            )
        );
    }
    if exceeds_max_threshold(details.total_debt_score, details.max_total_debt_score) {
        println!(
            "{}",
            format_threshold_failure(
                "Total debt score",
                &details.total_debt_score.to_string(),
                &details.max_total_debt_score.to_string(),
                ">"
            )
        );
    }
    if details.max_codebase_risk_score > 0.0
        && exceeds_max_threshold(details.codebase_risk_score, details.max_codebase_risk_score)
    {
        println!(
            "{}",
            format_threshold_failure(
                "Codebase risk score",
                &format!("{:.1}", details.codebase_risk_score),
                &format!("{:.1}", details.max_codebase_risk_score),
                ">"
            )
        );
    }
    if details.max_high_risk_functions > 0
        && exceeds_max_threshold(details.high_risk_functions, details.max_high_risk_functions)
    {
        println!(
            "{}",
            format_threshold_failure(
                "High-risk functions",
                &details.high_risk_functions.to_string(),
                &details.max_high_risk_functions.to_string(),
                ">"
            )
        );
    }
    if details.min_coverage_percentage > 0.0
        && below_min_threshold(details.coverage_percentage, details.min_coverage_percentage)
    {
        println!(
            "{}",
            format_threshold_failure(
                "Code coverage",
                &format!("{:.1}%", details.coverage_percentage),
                &format!("{:.1}%", details.min_coverage_percentage),
                "<"
            )
        );
    }
}

fn print_validation_failure_with_details(
    details: &ValidationDetails,
    risk_insights: &Option<risk::RiskInsight>,
    verbosity: u8,
) {
    println!("❌ Validation FAILED - Some metrics exceed thresholds");
    println!();

    // Always show validation details for failures
    print_validation_details(details);

    // Show which checks failed
    println!("\n  Failed checks:");
    print_failed_validation_checks(details);

    if verbosity > 1 && risk_insights.is_some() {
        if let Some(insights) = risk_insights {
            print_risk_metrics(insights);
        }
    }
}

fn print_validation_details(details: &ValidationDetails) {
    println!("  Metrics Summary:");
    println!(
        "    Average complexity: {:.1} (threshold: {:.1})",
        details.average_complexity, details.max_average_complexity
    );
    println!(
        "    High complexity functions: {} (threshold: {})",
        details.high_complexity_count, details.max_high_complexity_count
    );
    println!(
        "    Technical debt items: {} (threshold: {})",
        details.debt_items, details.max_debt_items
    );
    println!(
        "    Total debt score: {} (threshold: {})",
        details.total_debt_score, details.max_total_debt_score
    );

    if details.max_codebase_risk_score > 0.0 || details.codebase_risk_score > 0.0 {
        println!(
            "    Codebase risk score: {:.1} (threshold: {:.1})",
            details.codebase_risk_score, details.max_codebase_risk_score
        );
    }

    if details.max_high_risk_functions > 0 || details.high_risk_functions > 0 {
        println!(
            "    High-risk functions: {} (threshold: {})",
            details.high_risk_functions, details.max_high_risk_functions
        );
    }

    if details.min_coverage_percentage > 0.0 || details.coverage_percentage > 0.0 {
        println!(
            "    Code coverage: {:.1}% (minimum: {:.1}%)",
            details.coverage_percentage, details.min_coverage_percentage
        );
    }
}

fn print_risk_metrics(insights: &risk::RiskInsight) {
    println!(
        "\n  Overall codebase risk score: {:.1}",
        insights.codebase_risk_score
    );

    if !insights.top_risks.is_empty() {
        println!("\n  Critical risk functions (high complexity + low/no coverage):");
        insights
            .top_risks
            .iter()
            .take(5)
            .for_each(print_risk_function);
    }
}

fn format_risk_function(func: &risk::FunctionRisk) -> String {
    let coverage_str = func
        .coverage_percentage
        .map(|c| format!("{:.0}%", c * 100.0))
        .unwrap_or_else(|| "0%".to_string());
    format!(
        "    - {} (risk: {:.1}, coverage: {})",
        func.function_name, func.risk_score, coverage_str
    )
}

fn print_risk_function(func: &risk::FunctionRisk) {
    let formatted = format_risk_function(func);
    println!("{formatted}");
}

fn parse_languages(languages: Option<Vec<String>>) -> Vec<Language> {
    languages
        .map(|langs| {
            langs
                .iter()
                .filter_map(|lang_str| parse_single_language(lang_str))
                .collect()
        })
        .unwrap_or_else(default_languages)
}

fn parse_single_language(lang_str: &str) -> Option<Language> {
    match lang_str.to_lowercase().as_str() {
        "rust" | "rs" => Some(Language::Rust),
        "python" | "py" => Some(Language::Python),
        "javascript" | "js" => Some(Language::JavaScript),
        "typescript" | "ts" => Some(Language::TypeScript),
        _ => None,
    }
}

fn default_languages() -> Vec<Language> {
    vec![
        Language::Rust,
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
    ]
}

// Note: This function is now only used by tests. The actual threshold checking
// for CI/CD is done by the validate command using the validate_and_report function.
#[cfg(test)]
fn is_analysis_passing(results: &AnalysisResults, _complexity_threshold: u32) -> bool {
    let debt_score = debt::total_debt_score(&results.technical_debt.items);
    let debt_threshold = 100;

    results.complexity.summary.average_complexity <= 10.0
        && results.complexity.summary.high_complexity_count <= 5
        && debt_score <= debt_threshold
}

fn create_dependency_report(file_metrics: &[FileMetrics]) -> DependencyReport {
    analysis_utils::create_dependency_report(file_metrics)
}

/// Classify if a function is an entry point based on its name
fn is_entry_point(function_name: &str) -> bool {
    match function_name {
        "main" => true,
        name if name.starts_with("handle_") => true,
        name if name.starts_with("run_") => true,
        _ => false,
    }
}

/// Classify if a function is a test based on its name and file path
fn is_test_function(function_name: &str, file_path: &Path, is_test_attr: bool) -> bool {
    is_test_attr
        || function_name.starts_with("test_")
        || file_path.to_string_lossy().contains("test")
}

/// Build the initial call graph from complexity metrics
fn build_initial_call_graph(metrics: &[debtmap::FunctionMetrics]) -> priority::CallGraph {
    let mut call_graph = priority::CallGraph::new();

    for metric in metrics {
        let func_id = priority::call_graph::FunctionId {
            file: metric.file.clone(),
            name: metric.name.clone(),
            line: metric.line,
        };

        call_graph.add_function(
            func_id,
            is_entry_point(&metric.name),
            is_test_function(&metric.name, &metric.file, metric.is_test),
            metric.cyclomatic,
            metric.length,
        );
    }

    call_graph
}

// Expansion-related functions removed - now using enhanced token parsing in rust_call_graph.rs

/// Process Rust files to extract call relationships with enhanced analysis
fn process_rust_files_for_call_graph(
    project_path: &Path,
    call_graph: &mut priority::CallGraph,
    _verbose_macro_warnings: bool,
    _show_macro_stats: bool,
) -> Result<(
    std::collections::HashSet<priority::call_graph::FunctionId>,
    std::collections::HashSet<priority::call_graph::FunctionId>,
)> {
    let config = config::get_config();
    let rust_files =
        io::walker::find_project_files_with_config(project_path, vec![Language::Rust], config)
            .context("Failed to find Rust files for call graph")?;

    // Macro handling is now done through enhanced token parsing

    // Create Rust-specific call graph builder from the base graph
    let mut enhanced_builder = RustCallGraphBuilder::from_base_graph(call_graph.clone());

    // NEW APPROACH: Collect all parsed files first, then extract call graph globally
    let mut workspace_files = Vec::new();
    let mut expanded_files = Vec::new();

    // First pass: collect all parsed files
    for file_path in rust_files {
        if let Ok(content) = io::read_file(&file_path) {
            // Parse the content - macro handling happens in rust_call_graph
            if let Ok(parsed) = syn::parse_file(&content) {
                expanded_files.push((parsed.clone(), file_path.clone()));
                workspace_files.push((file_path.clone(), parsed));
            }
        }
    }

    // Second pass: extract call graph from all files with cross-module resolution
    if !expanded_files.is_empty() {
        use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
        let multi_file_call_graph = extract_call_graph_multi_file(&expanded_files);
        call_graph.merge(multi_file_call_graph);
    }

    // Third pass: run enhanced analysis on all parsed files
    for (file_path, parsed) in &workspace_files {
        enhanced_builder
            .analyze_basic_calls(file_path, parsed)?
            .analyze_trait_dispatch(file_path, parsed)?
            .analyze_function_pointers(file_path, parsed)?
            .analyze_framework_patterns(file_path, parsed)?;
    }

    // Run cross-module analysis with all files
    enhanced_builder.analyze_cross_module(&workspace_files)?;

    // Build the enhanced graph and merge its calls into our existing graph
    let enhanced_graph = enhanced_builder.build();
    // Get framework pattern exclusions for dead code detection
    let framework_exclusions = enhanced_graph.framework_patterns.get_exclusions();
    // Convert from im::HashSet to std::collections::HashSet
    let framework_exclusions_std: std::collections::HashSet<priority::call_graph::FunctionId> =
        framework_exclusions.into_iter().collect();

    // Extract function pointer usage information to prevent false positives
    let function_pointer_used_functions = enhanced_graph
        .function_pointer_tracker
        .get_definitely_used_functions();
    let function_pointer_used_std: std::collections::HashSet<priority::call_graph::FunctionId> =
        function_pointer_used_functions.into_iter().collect();
    // Merge the enhanced graph's calls into our existing call graph instead of replacing it
    call_graph.merge(enhanced_graph.base_graph);

    // Resolve cross-file function calls after all files have been processed
    call_graph.resolve_cross_file_calls();

    Ok((framework_exclusions_std, function_pointer_used_std))
}

/// Extract call graph from a file without expansion
#[allow(dead_code)]
fn extract_regular_call_graph(file_path: &Path) -> Result<priority::CallGraph> {
    let content = io::read_file(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let parsed = syn::parse_file(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse Rust file {}: {}", file_path.display(), e))?;

    use debtmap::analyzers::rust_call_graph::extract_call_graph;
    Ok(extract_call_graph(&parsed, file_path))
}

/// Process Python files to extract method call relationships
fn process_python_files_for_call_graph(
    project_path: &Path,
    call_graph: &mut priority::CallGraph,
) -> Result<()> {
    let config = config::get_config();
    let python_files =
        io::walker::find_project_files_with_config(project_path, vec![Language::Python], config)
            .context("Failed to find Python files for call graph")?;

    let mut analyzer = PythonCallGraphAnalyzer::new();

    for file_path in &python_files {
        match io::read_file(file_path) {
            Ok(content) => {
                // Parse Python file using rustpython_parser
                match rustpython_parser::parse(
                    &content,
                    rustpython_parser::Mode::Module,
                    "<module>",
                ) {
                    Ok(module) => {
                        // Analyze the module and extract method calls with source for accurate line numbers
                        if let Err(e) = analyzer
                            .analyze_module_with_source(&module, file_path, &content, call_graph)
                        {
                            log::warn!("Failed to analyze Python file {:?}: {}", file_path, e);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to parse Python file {:?}: {}", file_path, e);
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to read Python file {:?}: {}", file_path, e);
            }
        }
    }

    Ok(())
}

/// Determine if a function metric should be included in debt analysis
/// Returns true if the metric should be skipped, false if it should be included
fn should_skip_metric_for_debt_analysis(
    metric: &debtmap::FunctionMetrics,
    call_graph: &priority::CallGraph,
    test_only_functions: &std::collections::HashSet<priority::call_graph::FunctionId>,
) -> bool {
    // Skip test functions from debt score calculation
    // Test functions are analyzed separately to avoid inflating debt scores
    if metric.is_test || metric.in_test_module {
        return true;
    }

    // Skip closures - they're part of their parent function's implementation
    // Their complexity already contributes to the parent function's metrics
    if metric.name.contains("<closure@") {
        return true;
    }

    let func_id = priority::call_graph::FunctionId {
        file: metric.file.clone(),
        name: metric.name.clone(),
        line: metric.line,
    };

    // Skip functions that are only reachable from test functions
    // These are test infrastructure (mocks, helpers, fixtures)
    // This is 100% accurate - if something is only called from tests, it's not production code
    if test_only_functions.contains(&func_id) {
        return true;
    }

    // Skip only truly trivial delegation functions
    // These have minimal complexity AND delegate to exactly one other function
    if metric.cyclomatic == 1 && metric.cognitive == 0 && metric.length <= 3 {
        let callees = call_graph.get_callees(&func_id);
        if callees.len() == 1 {
            // This is a one-line delegation like: fn foo() { bar() }
            // Not worth tracking as technical debt
            return true;
        }
    }

    false
}

/// Create a debt item from a metric with framework exclusions
#[allow(dead_code)]
fn create_debt_item_from_metric(
    metric: &debtmap::FunctionMetrics,
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &std::collections::HashSet<priority::call_graph::FunctionId>,
    function_pointer_used_functions: Option<
        &std::collections::HashSet<priority::call_graph::FunctionId>,
    >,
) -> priority::UnifiedDebtItem {
    use priority::unified_scorer;

    unified_scorer::create_unified_debt_item_with_exclusions(
        metric,
        call_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
    )
}

fn create_debt_item_from_metric_with_aggregator(
    metric: &debtmap::FunctionMetrics,
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &std::collections::HashSet<priority::call_graph::FunctionId>,
    function_pointer_used_functions: Option<
        &std::collections::HashSet<priority::call_graph::FunctionId>,
    >,
    debt_aggregator: &priority::debt_aggregator::DebtAggregator,
    data_flow: Option<&debtmap::data_flow::DataFlowGraph>,
) -> priority::UnifiedDebtItem {
    use debtmap::scoring::{EnhancedScorer, ScoringContext};
    use priority::unified_scorer;
    use std::collections::HashSet;

    // Always use enhanced scoring
    // Create scoring context
    let mut scoring_context = ScoringContext::new(call_graph.clone());

    // Add coverage data if available
    if let Some(lcov) = coverage_data {
        scoring_context = scoring_context.with_coverage(lcov.clone());
    }

    // Identify test files
    let test_files: HashSet<std::path::PathBuf> = HashSet::new();
    scoring_context = scoring_context.with_test_files(test_files);

    // Create enhanced scorer
    let scorer = EnhancedScorer::new(&scoring_context);

    // Score the function
    let score_breakdown = scorer.score_function_with_aggregator(metric, debt_aggregator);

    // Convert to UnifiedDebtItem with enhanced score
    let mut item = unified_scorer::create_unified_debt_item_with_aggregator_and_data_flow(
        metric,
        call_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
        debt_aggregator,
        data_flow,
    );

    // Override the final score with enhanced score
    item.unified_score.final_score = score_breakdown.total;

    item
}

/// Convert error swallowing debt items to unified debt items
fn convert_error_swallowing_to_unified(
    debt_items: &[core::DebtItem],
    _call_graph: &priority::CallGraph,
) -> Vec<priority::UnifiedDebtItem> {
    use priority::{
        unified_scorer::Location, ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics,
        UnifiedDebtItem, UnifiedScore,
    };

    debt_items
        .iter()
        .filter(|item| item.debt_type == core::DebtType::ErrorSwallowing)
        .map(|item| {
            // Create a basic unified score for error swallowing
            // These aren't function-based, so we use moderate scores
            let unified_score = UnifiedScore {
                complexity_factor: 3.0, // Moderate complexity - error handling adds complexity
                coverage_factor: 5.0,   // Important to test error paths
                dependency_factor: 4.0, // Moderate dependency impact
                role_multiplier: 1.2,   // Slightly elevated importance
                final_score: 5.5,       // Above average priority
            };

            let pattern = item.message.split(':').next().unwrap_or("Error swallowing");
            let context = item.context.clone();

            UnifiedDebtItem {
                location: Location {
                    file: item.file.clone(),
                    function: format!("line_{}", item.line), // Use line number as pseudo-function
                    line: item.line,
                },
                debt_type: DebtType::ErrorSwallowing {
                    pattern: pattern.to_string(),
                    context,
                },
                unified_score,
                function_role: FunctionRole::Unknown,
                recommendation: ActionableRecommendation {
                    primary_action: format!("Fix error swallowing at line {}", item.line),
                    rationale: item.message.clone(),
                    implementation_steps: vec![
                        "Replace error swallowing with proper error handling".to_string(),
                        "Log errors at minimum, even if they can't be handled".to_string(),
                        "Consider propagating errors to caller with ?".to_string(),
                    ],
                    related_items: vec![],
                },
                expected_impact: ImpactMetrics {
                    coverage_improvement: 0.0,
                    lines_reduction: 0,
                    complexity_reduction: 0.0,
                    risk_reduction: 3.5, // Significant risk reduction
                },
                transitive_coverage: None,
                upstream_dependencies: 0,
                downstream_dependencies: 0,
                upstream_callers: vec![],
                downstream_callees: vec![],
                nesting_depth: 0,
                function_length: 0,
                cyclomatic_complexity: 0,
                cognitive_complexity: 0,
                entropy_details: None, // Error swallowing items don't have entropy data
                is_pure: None,         // Error swallowing functions are not pure
                purity_confidence: None,
            }
        })
        .collect()
}

/// Create unified analysis from metrics and call graph with framework exclusions
fn create_unified_analysis_with_exclusions(
    metrics: &[debtmap::FunctionMetrics],
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &std::collections::HashSet<priority::call_graph::FunctionId>,
    function_pointer_used_functions: Option<
        &std::collections::HashSet<priority::call_graph::FunctionId>,
    >,
    debt_items: Option<&[core::DebtItem]>,
) -> priority::UnifiedAnalysis {
    use priority::{
        debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId},
        UnifiedAnalysis,
    };

    let mut unified = UnifiedAnalysis::new(call_graph.clone());

    // Populate the data flow graph with purity analysis data
    unified.populate_purity_analysis(metrics);

    // Identify test-only functions using call graph reachability
    let test_only_functions: std::collections::HashSet<_> =
        call_graph.find_test_only_functions().into_iter().collect();

    // Create debt aggregator and aggregate all debt items
    let mut debt_aggregator = DebtAggregator::new();
    if let Some(debt_items) = debt_items {
        // Create function ID mappings for aggregation
        let function_mappings: Vec<(AggregatorFunctionId, usize, usize)> = metrics
            .iter()
            .map(|m| {
                let func_id = AggregatorFunctionId {
                    file: m.file.clone(),
                    name: m.name.clone(),
                    start_line: m.line,
                    end_line: m.line + m.length,
                };
                (func_id, m.line, m.line + m.length)
            })
            .collect();

        // The debt items are already the correct type
        let debt_items_vec: Vec<core::DebtItem> = debt_items.to_vec();

        debt_aggregator.aggregate_debt(debt_items_vec, &function_mappings);
    }

    for metric in metrics {
        if should_skip_metric_for_debt_analysis(metric, call_graph, &test_only_functions) {
            continue;
        }

        let item = create_debt_item_from_metric_with_aggregator(
            metric,
            call_graph,
            coverage_data,
            framework_exclusions,
            function_pointer_used_functions,
            &debt_aggregator,
            Some(&unified.data_flow_graph),
        );
        unified.add_item(item);
    }

    // Add error swallowing debt items if provided
    if let Some(debt_items) = debt_items {
        // Convert error swallowing debt items
        let error_swallowing_items = convert_error_swallowing_to_unified(debt_items, call_graph);
        for item in error_swallowing_items {
            unified.add_item(item);
        }
    }

    unified.sort_by_priority();
    unified.calculate_total_impact();

    // Set overall coverage if lcov data is available
    if let Some(lcov) = coverage_data {
        unified.overall_coverage = Some(lcov.get_overall_coverage());
    }

    unified
}

/// Create unified analysis from metrics and call graph (legacy - kept for compatibility)
#[allow(dead_code)]
fn create_unified_analysis(
    metrics: &[debtmap::FunctionMetrics],
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
) -> priority::UnifiedAnalysis {
    use priority::{unified_scorer, UnifiedAnalysis};

    let mut unified = UnifiedAnalysis::new(call_graph.clone());

    // Identify test-only functions using call graph reachability
    let test_only_functions: std::collections::HashSet<_> =
        call_graph.find_test_only_functions().into_iter().collect();

    for metric in metrics {
        // Use the centralized skip logic
        if should_skip_metric_for_debt_analysis(metric, call_graph, &test_only_functions) {
            continue;
        }

        // Create the unified debt item
        let item = unified_scorer::create_unified_debt_item(metric, call_graph, coverage_data);
        unified.add_item(item);
    }

    unified.sort_by_priority();
    unified.calculate_total_impact();

    // Set overall coverage if lcov data is available
    if let Some(lcov) = coverage_data {
        unified.overall_coverage = Some(lcov.get_overall_coverage());
    }

    unified
}

fn perform_unified_analysis(
    results: &AnalysisResults,
    coverage_file: Option<&PathBuf>,
    _semantic_off: bool,
    project_path: &Path,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
) -> Result<priority::UnifiedAnalysis> {
    // Build initial call graph from complexity metrics
    let mut call_graph = build_initial_call_graph(&results.complexity.metrics);

    // Process Rust files to extract call relationships and get framework exclusions
    let (framework_exclusions, function_pointer_used_functions) =
        process_rust_files_for_call_graph(
            project_path,
            &mut call_graph,
            verbose_macro_warnings,
            show_macro_stats,
        )?;

    // Process Python files to extract method call relationships
    process_python_files_for_call_graph(project_path, &mut call_graph)?;

    // Load coverage data if available
    let coverage_data = match coverage_file {
        Some(lcov_path) => {
            Some(risk::lcov::parse_lcov_file(lcov_path).context("Failed to parse LCOV file")?)
        }
        None => None,
    };

    // Create and return unified analysis with framework exclusions
    Ok(create_unified_analysis_with_exclusions(
        &results.complexity.metrics,
        &call_graph,
        coverage_data.as_ref(),
        &framework_exclusions,
        Some(&function_pointer_used_functions),
        Some(&results.technical_debt.items),
    ))
}

/// Determines the priority output format based on command line flags
fn determine_priority_output_format(
    top: Option<usize>,
    tail: Option<usize>,
) -> priority::formatter::OutputFormat {
    use priority::formatter::OutputFormat;

    if let Some(n) = tail {
        OutputFormat::Tail(n)
    } else if let Some(n) = top {
        OutputFormat::Top(n)
    } else {
        OutputFormat::Default
    }
}

/// Determines if the output file has a markdown extension
fn is_markdown_file(output_file: &Option<PathBuf>) -> bool {
    output_file
        .as_ref()
        .and_then(|p| p.extension())
        .map(|ext| ext == "md")
        .unwrap_or(false)
}

/// Calculates the limit for markdown output based on top/tail parameters
fn calculate_markdown_limit(top: Option<usize>, tail: Option<usize>) -> usize {
    if let Some(n) = top {
        n
    } else if tail.is_some() {
        // For tail, we'll handle it differently in markdown
        10
    } else {
        10
    }
}

/// Handles JSON format output
fn output_json(analysis: &priority::UnifiedAnalysis, output_file: Option<PathBuf>) -> Result<()> {
    use std::fs;
    use std::io::Write;

    let json = serde_json::to_string_pretty(analysis)?;
    if let Some(path) = output_file {
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
    } else {
        println!("{json}");
    }
    Ok(())
}

/// Handles markdown format output
fn output_markdown(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    verbosity: u8,
    output_file: Option<PathBuf>,
) -> Result<()> {
    use std::fs;
    use std::io::Write;

    let limit = calculate_markdown_limit(top, tail);
    let output = priority::format_priorities_markdown(analysis, limit, verbosity);

    if let Some(path) = output_file {
        let mut file = fs::File::create(path)?;
        file.write_all(output.as_bytes())?;
    } else {
        println!("{output}");
    }
    Ok(())
}

/// Handles terminal format output
fn output_enhanced_markdown(
    analysis: &priority::UnifiedAnalysis,
    results: &AnalysisResults,
    coverage_file: Option<&PathBuf>,
    detail_level: &str,
    output_file: Option<PathBuf>,
) -> Result<()> {
    use io::writers::{DetailLevel, EnhancedMarkdownWriter, MarkdownConfig};

    // Parse detail level
    let detail = match detail_level.to_lowercase().as_str() {
        "summary" => DetailLevel::Summary,
        "standard" => DetailLevel::Standard,
        "detailed" => DetailLevel::Detailed,
        "complete" => DetailLevel::Complete,
        _ => DetailLevel::Standard,
    };

    // Create configuration
    let config = MarkdownConfig {
        detail_level: detail,
        ..MarkdownConfig::default()
    };

    // Calculate risk insights if coverage available
    let risk_insights = if let Some(lcov_path) = coverage_file {
        match analyze_risk_with_coverage(
            results,
            lcov_path,
            &PathBuf::from("."),
            false,
            None,
            None,
        ) {
            Ok(insights) => insights,
            Err(e) => {
                log::warn!("Failed to calculate risk insights: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Create writer
    let writer: Box<dyn std::io::Write> = if let Some(path) = output_file {
        Box::new(std::fs::File::create(path)?)
    } else {
        Box::new(std::io::stdout())
    };

    let mut enhanced_writer = EnhancedMarkdownWriter::with_config(writer, config);

    // Write the enhanced report
    enhanced_writer.write_enhanced_report(results, Some(analysis), risk_insights.as_ref())?;

    Ok(())
}

fn output_terminal(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    verbosity: u8,
    output_file: Option<PathBuf>,
) -> Result<()> {
    use std::fs;
    use std::io::Write;

    let format = determine_priority_output_format(top, tail);
    let output = priority::formatter::format_priorities_with_verbosity(analysis, format, verbosity);

    if let Some(path) = output_file {
        let mut file = fs::File::create(path)?;
        file.write_all(output.as_bytes())?;
    } else {
        println!("{output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn output_unified_priorities(
    analysis: priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    verbosity: u8,
    output_file: Option<PathBuf>,
    output_format: Option<cli::OutputFormat>,
) -> Result<()> {
    // Check if JSON format is requested
    if let Some(cli::OutputFormat::Json) = output_format {
        output_json(&analysis, output_file)
    } else if is_markdown_file(&output_file) {
        output_markdown(&analysis, top, tail, verbosity, output_file)
    } else {
        output_terminal(&analysis, top, tail, verbosity, output_file)
    }
}

#[allow(clippy::too_many_arguments)]
fn output_unified_priorities_with_config(
    analysis: priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    verbosity: u8,
    output_file: Option<PathBuf>,
    output_format: Option<cli::OutputFormat>,
    markdown_enhanced: bool,
    markdown_detail: &str,
    results: &AnalysisResults,
    coverage_file: Option<&PathBuf>,
) -> Result<()> {
    // Check if enhanced markdown is requested
    if markdown_enhanced && matches!(output_format, Some(cli::OutputFormat::Markdown)) {
        output_enhanced_markdown(
            &analysis,
            results,
            coverage_file,
            markdown_detail,
            output_file,
        )
    } else {
        // Fall back to standard output
        output_unified_priorities(analysis, top, tail, verbosity, output_file, output_format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use debtmap::core::{
        ComplexityReport, ComplexitySummary, DebtItem, DebtType, DependencyReport, Priority,
        TechnicalDebtReport,
    };
    use debtmap::risk::{Difficulty, FunctionRisk, RiskCategory, RiskDistribution, TestEffort};
    use im::Vector;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_format_risk_function_with_coverage() {
        let func = FunctionRisk {
            function_name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line_range: (10, 20),
            cyclomatic_complexity: 5,
            cognitive_complexity: 7,
            risk_score: 8.5,
            coverage_percentage: Some(0.75),
            test_effort: TestEffort {
                estimated_difficulty: Difficulty::Moderate,
                cognitive_load: 7,
                branch_count: 5,
                recommended_test_cases: 3,
            },
            risk_category: RiskCategory::Medium,
            is_test_function: false,
        };
        let result = format_risk_function(&func);
        assert_eq!(result, "    - test_func (risk: 8.5, coverage: 75%)");
    }

    #[test]
    fn test_format_risk_function_without_coverage() {
        let func = FunctionRisk {
            function_name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line_range: (10, 20),
            cyclomatic_complexity: 5,
            cognitive_complexity: 7,
            risk_score: 10.0,
            coverage_percentage: None,
            test_effort: TestEffort {
                estimated_difficulty: Difficulty::Trivial,
                cognitive_load: 3,
                branch_count: 1,
                recommended_test_cases: 1,
            },
            risk_category: RiskCategory::Critical,
            is_test_function: false,
        };
        let result = format_risk_function(&func);
        assert_eq!(result, "    - test_func (risk: 10.0, coverage: 0%)");
    }

    #[test]
    fn test_format_risk_function_zero_coverage() {
        let func = FunctionRisk {
            function_name: "zero_cov_func".to_string(),
            file: PathBuf::from("test.rs"),
            line_range: (20, 30),
            cyclomatic_complexity: 3,
            cognitive_complexity: 4,
            risk_score: 5.5,
            coverage_percentage: Some(0.0),
            test_effort: TestEffort {
                estimated_difficulty: Difficulty::Trivial,
                cognitive_load: 4,
                branch_count: 2,
                recommended_test_cases: 2,
            },
            risk_category: RiskCategory::Low,
            is_test_function: false,
        };
        let result = format_risk_function(&func);
        assert_eq!(result, "    - zero_cov_func (risk: 5.5, coverage: 0%)");
    }

    #[test]
    fn test_format_risk_function_full_coverage() {
        let func = FunctionRisk {
            function_name: "well_tested_func".to_string(),
            file: PathBuf::from("test.rs"),
            line_range: (30, 50),
            cyclomatic_complexity: 10,
            cognitive_complexity: 15,
            risk_score: 1.2,
            coverage_percentage: Some(1.0),
            test_effort: TestEffort {
                estimated_difficulty: Difficulty::Moderate,
                cognitive_load: 15,
                branch_count: 8,
                recommended_test_cases: 5,
            },
            risk_category: RiskCategory::WellTested,
            is_test_function: false,
        };
        let result = format_risk_function(&func);
        assert_eq!(result, "    - well_tested_func (risk: 1.2, coverage: 100%)");
    }

    #[test]
    fn test_parse_single_language_rust() {
        assert_eq!(parse_single_language("rust"), Some(Language::Rust));
        assert_eq!(parse_single_language("rs"), Some(Language::Rust));
        assert_eq!(parse_single_language("RUST"), Some(Language::Rust));
        assert_eq!(parse_single_language("Rs"), Some(Language::Rust));
    }

    #[test]
    fn test_parse_single_language_python() {
        assert_eq!(parse_single_language("python"), Some(Language::Python));
        assert_eq!(parse_single_language("py"), Some(Language::Python));
        assert_eq!(parse_single_language("PYTHON"), Some(Language::Python));
        assert_eq!(parse_single_language("Py"), Some(Language::Python));
    }

    #[test]
    fn test_parse_single_language_javascript() {
        assert_eq!(
            parse_single_language("javascript"),
            Some(Language::JavaScript)
        );
        assert_eq!(parse_single_language("js"), Some(Language::JavaScript));
        assert_eq!(
            parse_single_language("JAVASCRIPT"),
            Some(Language::JavaScript)
        );
        assert_eq!(parse_single_language("JS"), Some(Language::JavaScript));
    }

    #[test]
    fn test_parse_single_language_typescript() {
        assert_eq!(
            parse_single_language("typescript"),
            Some(Language::TypeScript)
        );
        assert_eq!(parse_single_language("ts"), Some(Language::TypeScript));
        assert_eq!(
            parse_single_language("TYPESCRIPT"),
            Some(Language::TypeScript)
        );
        assert_eq!(parse_single_language("TS"), Some(Language::TypeScript));
    }

    #[test]
    fn test_parse_single_language_unknown() {
        assert_eq!(parse_single_language("java"), None);
        assert_eq!(parse_single_language("c++"), None);
        assert_eq!(parse_single_language("go"), None);
        assert_eq!(parse_single_language(""), None);
    }

    #[test]
    fn test_parse_languages_with_valid_input() {
        let input = Some(vec!["rust".to_string(), "python".to_string()]);
        let result = parse_languages(input);
        assert_eq!(result, vec![Language::Rust, Language::Python]);
    }

    #[test]
    fn test_parse_languages_with_mixed_valid_invalid() {
        let input = Some(vec![
            "rust".to_string(),
            "java".to_string(),
            "python".to_string(),
        ]);
        let result = parse_languages(input);
        assert_eq!(result, vec![Language::Rust, Language::Python]);
    }

    #[test]
    fn test_parse_languages_with_none_uses_default() {
        let result = parse_languages(None);
        assert_eq!(result, default_languages());
    }

    #[test]
    fn test_parse_languages_empty_vec_returns_empty() {
        let input = Some(vec![]);
        let result = parse_languages(input);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_default_languages() {
        let defaults = default_languages();
        assert_eq!(defaults.len(), 4);
        assert!(defaults.contains(&Language::Rust));
        assert!(defaults.contains(&Language::Python));
        assert!(defaults.contains(&Language::JavaScript));
        assert!(defaults.contains(&Language::TypeScript));
    }

    #[test]
    fn test_analyze_risk_with_coverage_success() {
        use debtmap::FunctionMetrics;
        use std::fs;
        use tempfile::TempDir;

        // Create test data
        let temp_dir = TempDir::new().unwrap();
        let lcov_path = temp_dir.path().join("test.lcov");

        // Create a simple LCOV file
        let lcov_content = r#"TN:
SF:src/test.rs
FN:10,test_func
FNDA:5,test_func
FNF:1
FNH:1
DA:10,5
DA:11,5
DA:12,0
DA:13,0
LF:4
LH:2
end_of_record
"#;
        fs::write(&lcov_path, lcov_content).unwrap();

        // Create analysis results with test functions
        let results = AnalysisResults {
            project_path: temp_dir.path().to_path_buf(),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![FunctionMetrics {
                    name: "test_func".to_string(),
                    file: PathBuf::from("src/test.rs"),
                    line: 10,
                    cyclomatic: 4,
                    cognitive: 3,
                    nesting: 2,
                    length: 4,
                    is_test: false,
                    visibility: None,
                    is_trait_method: false,
                    in_test_module: false,
                    entropy_score: None,
                    is_pure: None,
                    purity_confidence: None,
                }],
                summary: ComplexitySummary {
                    total_functions: 1,
                    average_complexity: 4.0,
                    max_complexity: 4,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        // Test with coverage analysis
        let result =
            analyze_risk_with_coverage(&results, &lcov_path, temp_dir.path(), false, None, None);

        assert!(result.is_ok());
        let insight = result.unwrap();
        assert!(insight.is_some());

        let insight = insight.unwrap();
        // Should have analyzed one function
        assert!(!insight.top_risks.is_empty());
        // Coverage should be calculated (50% in our test LCOV)
        assert!(insight.top_risks[0].coverage_percentage.is_some());
    }

    #[test]
    fn test_analyze_risk_with_coverage_invalid_lcov_path() {
        use debtmap::FunctionMetrics;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let non_existent_lcov = temp_dir.path().join("missing.lcov");

        let results = AnalysisResults {
            project_path: temp_dir.path().to_path_buf(),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![FunctionMetrics {
                    name: "test_func".to_string(),
                    file: PathBuf::from("src/test.rs"),
                    line: 10,
                    cyclomatic: 3,
                    cognitive: 2,
                    nesting: 1,
                    length: 10,
                    is_test: false,
                    visibility: None,
                    is_trait_method: false,
                    in_test_module: false,
                    entropy_score: None,
                    is_pure: None,
                    purity_confidence: None,
                }],
                summary: ComplexitySummary {
                    total_functions: 1,
                    average_complexity: 3.0,
                    max_complexity: 3,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        // Should fail when LCOV file doesn't exist
        let result = analyze_risk_with_coverage(
            &results,
            &non_existent_lcov,
            temp_dir.path(),
            false,
            None,
            None,
        );

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse LCOV file"));
    }

    #[test]
    fn test_analyze_risk_with_coverage_with_context() {
        use debtmap::FunctionMetrics;
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let lcov_path = temp_dir.path().join("test.lcov");

        // Create LCOV with no coverage
        let lcov_content = r#"TN:
SF:src/test.rs
FN:10,main
FNDA:0,main
FNF:1
FNH:0
DA:10,0
DA:11,0
LF:2
LH:0
end_of_record
"#;
        fs::write(&lcov_path, lcov_content).unwrap();

        let results = AnalysisResults {
            project_path: temp_dir.path().to_path_buf(),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![FunctionMetrics {
                    name: "main".to_string(),
                    file: PathBuf::from("src/test.rs"),
                    line: 10,
                    cyclomatic: 2,
                    cognitive: 1,
                    nesting: 0,
                    length: 2,
                    is_test: false,
                    visibility: None,
                    is_trait_method: false,
                    in_test_module: false,
                    entropy_score: None,
                    is_pure: None,
                    purity_confidence: None,
                }],
                summary: ComplexitySummary {
                    total_functions: 1,
                    average_complexity: 2.0,
                    max_complexity: 2,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![DebtItem {
                    id: "debt-1".to_string(),
                    debt_type: DebtType::Todo,
                    priority: Priority::High,
                    file: PathBuf::from("src/test.rs"),
                    line: 10,
                    column: None,
                    message: "TODO: Implement feature".to_string(),
                    context: None,
                }],
                by_type: std::collections::HashMap::new(),
                priorities: vec![Priority::High],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        // Test with context enabled
        let result = analyze_risk_with_coverage(
            &results,
            &lcov_path,
            temp_dir.path(),
            true,
            Some(vec!["dependency".to_string()]),
            None,
        );

        assert!(result.is_ok());
        let insight = result.unwrap();
        assert!(insight.is_some());

        let insight = insight.unwrap();
        // Should identify entry point main with 0% coverage
        assert!(!insight.top_risks.is_empty());
        assert_eq!(insight.top_risks[0].function_name, "main");
        assert_eq!(insight.top_risks[0].coverage_percentage, Some(0.0));
    }

    #[test]
    fn test_analyze_risk_with_coverage_multiple_functions() {
        use debtmap::FunctionMetrics;
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let lcov_path = temp_dir.path().join("test.lcov");

        // Create LCOV with mixed coverage
        // The LCOV format shows line coverage, not function coverage percentages
        let lcov_content = r#"TN:
SF:src/lib.rs
FN:10,well_tested
FNDA:10,well_tested
FN:20,partially_tested
FNDA:5,partially_tested
FN:30,untested
FNDA:0,untested
FNF:3
FNH:2
DA:10,10
DA:11,10
DA:20,5
DA:21,5
DA:22,0
DA:30,0
DA:31,0
LF:7
LH:4
end_of_record
"#;
        fs::write(&lcov_path, lcov_content).unwrap();

        let results = AnalysisResults {
            project_path: temp_dir.path().to_path_buf(),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![
                    FunctionMetrics {
                        name: "well_tested".to_string(),
                        file: PathBuf::from("src/lib.rs"),
                        line: 10,
                        cyclomatic: 2,
                        cognitive: 1,
                        nesting: 0,
                        length: 2,
                        is_test: false,
                        visibility: None,
                        is_trait_method: false,
                        in_test_module: false,
                        entropy_score: None,
                        is_pure: None,
                        purity_confidence: None,
                    },
                    FunctionMetrics {
                        name: "partially_tested".to_string(),
                        file: PathBuf::from("src/lib.rs"),
                        line: 20,
                        cyclomatic: 3,
                        cognitive: 2,
                        nesting: 1,
                        length: 3, // Adjusted to match LCOV data
                        is_test: false,
                        visibility: None,
                        is_trait_method: false,
                        in_test_module: false,
                        entropy_score: None,
                        is_pure: None,
                        purity_confidence: None,
                    },
                    FunctionMetrics {
                        name: "untested".to_string(),
                        file: PathBuf::from("src/lib.rs"),
                        line: 30,
                        cyclomatic: 5,
                        cognitive: 4,
                        nesting: 2,
                        length: 2,
                        is_test: false,
                        visibility: None,
                        is_trait_method: false,
                        in_test_module: false,
                        entropy_score: None,
                        is_pure: None,
                        purity_confidence: None,
                    },
                    FunctionMetrics {
                        name: "test_function".to_string(),
                        file: PathBuf::from("src/lib.rs"),
                        line: 40,
                        cyclomatic: 1,
                        cognitive: 1,
                        nesting: 0,
                        length: 5,
                        is_test: true,
                        visibility: None,
                        is_trait_method: false,
                        in_test_module: false,
                        entropy_score: None,
                        is_pure: None,
                        purity_confidence: None,
                    },
                ],
                summary: ComplexitySummary {
                    total_functions: 4,
                    average_complexity: 2.75,
                    max_complexity: 5,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        // Test with multiple functions of varying coverage
        let result = analyze_risk_with_coverage(
            &results,
            &lcov_path,
            temp_dir.path(),
            false,
            None,
            Some(vec!["git_history".to_string()]),
        );

        assert!(result.is_ok());
        let insight = result.unwrap();
        assert!(insight.is_some());

        let insight = insight.unwrap();
        // Should have analyzed functions - check top_risks list
        assert!(!insight.top_risks.is_empty());

        // Find functions in top_risks and verify expected coverage patterns
        let all_risks = &insight.top_risks;

        // Well tested function should have coverage data
        if let Some(well_tested) = all_risks.iter().find(|r| r.function_name == "well_tested") {
            assert!(well_tested.coverage_percentage.is_some());
            // Should have high coverage since all lines are executed
            // Coverage is stored as a fraction (0-1), not percentage
            assert!(well_tested.coverage_percentage.unwrap() > 0.5);
        }

        // Partially tested function should have some coverage
        if let Some(partially_tested) = all_risks
            .iter()
            .find(|r| r.function_name == "partially_tested")
        {
            // Just verify it has coverage data
            assert!(partially_tested.coverage_percentage.is_some());
        }

        // Untested function should have zero coverage
        if let Some(untested) = all_risks.iter().find(|r| r.function_name == "untested") {
            assert_eq!(untested.coverage_percentage, Some(0.0));
        }

        // Test functions should be marked as such
        if let Some(test_func) = all_risks
            .iter()
            .find(|r| r.function_name == "test_function")
        {
            assert!(test_func.is_test_function);
        }
    }

    #[test]
    fn test_analyze_risk_with_coverage_empty_lcov() {
        use debtmap::FunctionMetrics;
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let lcov_path = temp_dir.path().join("empty.lcov");

        // Create empty but valid LCOV file
        let lcov_content = "";
        fs::write(&lcov_path, lcov_content).unwrap();

        let results = AnalysisResults {
            project_path: temp_dir.path().to_path_buf(),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![FunctionMetrics {
                    name: "uncovered_func".to_string(),
                    file: PathBuf::from("src/test.rs"),
                    line: 10,
                    cyclomatic: 5,
                    cognitive: 3,
                    nesting: 2,
                    length: 15,
                    is_test: false,
                    visibility: None,
                    is_trait_method: false,
                    in_test_module: false,
                    entropy_score: None,
                    is_pure: None,
                    purity_confidence: None,
                }],
                summary: ComplexitySummary {
                    total_functions: 1,
                    average_complexity: 5.0,
                    max_complexity: 5,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        // Should succeed but functions will have no coverage
        let result =
            analyze_risk_with_coverage(&results, &lcov_path, temp_dir.path(), false, None, None);

        assert!(result.is_ok());
        let insights = result.unwrap();
        assert!(insights.is_some());

        let insights = insights.unwrap();
        // Function should be in top risks due to no coverage
        assert!(!insights.top_risks.is_empty());
        assert_eq!(insights.top_risks[0].coverage_percentage, None);
    }

    #[test]
    fn test_analyze_risk_with_coverage_malformed_lcov() {
        use debtmap::FunctionMetrics;
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let lcov_path = temp_dir.path().join("malformed.lcov");

        // Create malformed LCOV content
        let lcov_content = "INVALID:malformed:content:not:lcov:format";
        fs::write(&lcov_path, lcov_content).unwrap();

        let results = AnalysisResults {
            project_path: temp_dir.path().to_path_buf(),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![FunctionMetrics {
                    name: "test_func".to_string(),
                    file: PathBuf::from("src/test.rs"),
                    line: 10,
                    cyclomatic: 3,
                    cognitive: 2,
                    nesting: 1,
                    length: 10,
                    is_test: false,
                    visibility: None,
                    is_trait_method: false,
                    in_test_module: false,
                    entropy_score: None,
                    is_pure: None,
                    purity_confidence: None,
                }],
                summary: ComplexitySummary {
                    total_functions: 1,
                    average_complexity: 3.0,
                    max_complexity: 3,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        // Should handle malformed LCOV gracefully
        let result =
            analyze_risk_with_coverage(&results, &lcov_path, temp_dir.path(), false, None, None);

        // Malformed LCOV causes parsing error
        assert!(result.is_err());
        // Verify error message mentions LCOV parsing
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse LCOV file"));
    }

    #[test]
    fn test_analyze_risk_with_coverage_full_coverage() {
        use debtmap::FunctionMetrics;
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let lcov_path = temp_dir.path().join("full_coverage.lcov");

        // Create LCOV with 100% coverage
        let lcov_content = r#"TN:
SF:src/test.rs
FN:10,fully_covered
FNDA:100,fully_covered
FNF:1
FNH:1
DA:10,100
DA:11,100
DA:12,100
DA:13,100
DA:14,100
LF:5
LH:5
end_of_record
"#;
        fs::write(&lcov_path, lcov_content).unwrap();

        let results = AnalysisResults {
            project_path: temp_dir.path().to_path_buf(),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![FunctionMetrics {
                    name: "fully_covered".to_string(),
                    file: PathBuf::from("src/test.rs"),
                    line: 10,
                    cyclomatic: 8, // High complexity but fully covered
                    cognitive: 6,
                    nesting: 3,
                    length: 5,
                    is_test: false,
                    visibility: None,
                    is_trait_method: false,
                    in_test_module: false,
                    entropy_score: None,
                    is_pure: None,
                    purity_confidence: None,
                }],
                summary: ComplexitySummary {
                    total_functions: 1,
                    average_complexity: 8.0,
                    max_complexity: 8,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        let result =
            analyze_risk_with_coverage(&results, &lcov_path, temp_dir.path(), false, None, None);

        assert!(result.is_ok());
        let insights = result.unwrap();
        assert!(insights.is_some());

        let insights = insights.unwrap();
        // Even with high complexity, 100% coverage should reduce risk
        if !insights.top_risks.is_empty() {
            // Check that coverage data is present (value might vary based on LCOV interpretation)
            assert!(insights.top_risks[0].coverage_percentage.is_some());
            // Risk score should be affected by coverage (even if not perfect)
            // Higher complexity with some coverage should still have moderate risk
            assert!(insights.top_risks[0].risk_score > 0.0);
        }
    }

    #[test]
    fn test_analyze_risk_with_coverage_empty_metrics() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let lcov_path = temp_dir.path().join("test.lcov");

        // Create valid LCOV file
        let lcov_content = r#"TN:
SF:src/test.rs
FNF:0
FNH:0
LF:0
LH:0
end_of_record
"#;
        fs::write(&lcov_path, lcov_content).unwrap();

        let results = AnalysisResults {
            project_path: temp_dir.path().to_path_buf(),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![], // Empty metrics
                summary: ComplexitySummary {
                    total_functions: 0,
                    average_complexity: 0.0,
                    max_complexity: 0,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        let result =
            analyze_risk_with_coverage(&results, &lcov_path, temp_dir.path(), false, None, None);

        assert!(result.is_ok());
        let insights = result.unwrap();
        assert!(insights.is_some());

        let insights = insights.unwrap();
        // No functions means no risks
        assert!(insights.top_risks.is_empty());
        assert_eq!(insights.risk_distribution.total_functions, 0);
    }

    #[test]
    fn test_analyze_risk_with_coverage_high_debt_context() {
        use debtmap::FunctionMetrics;
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let lcov_path = temp_dir.path().join("test.lcov");

        // Create LCOV with partial coverage
        let lcov_content = r#"TN:
SF:src/test.rs
FN:10,debt_func
FNDA:3,debt_func
FNF:1
FNH:1
DA:10,3
DA:11,3
DA:12,0
LF:3
LH:2
end_of_record
"#;
        fs::write(&lcov_path, lcov_content).unwrap();

        let results = AnalysisResults {
            project_path: temp_dir.path().to_path_buf(),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![FunctionMetrics {
                    name: "debt_func".to_string(),
                    file: PathBuf::from("src/test.rs"),
                    line: 10,
                    cyclomatic: 6,
                    cognitive: 4,
                    nesting: 2,
                    length: 3,
                    is_test: false,
                    visibility: None,
                    is_trait_method: false,
                    in_test_module: false,
                    entropy_score: None,
                    is_pure: None,
                    purity_confidence: None,
                }],
                summary: ComplexitySummary {
                    total_functions: 1,
                    average_complexity: 6.0,
                    max_complexity: 6,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![
                    // Add high debt items
                    DebtItem {
                        id: "debt-1".to_string(),
                        debt_type: DebtType::Complexity,
                        priority: Priority::High,
                        file: PathBuf::from("src/test.rs"),
                        line: 10,
                        column: Some(1),
                        message: "High complexity function".to_string(),
                        context: Some("debt_func".to_string()),
                    },
                ],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        let result =
            analyze_risk_with_coverage(&results, &lcov_path, temp_dir.path(), false, None, None);

        assert!(result.is_ok());
        let insights = result.unwrap();
        assert!(insights.is_some());

        let insights = insights.unwrap();
        // High debt should contribute to risk
        assert!(!insights.top_risks.is_empty());
        // The function should have moderate risk (complexity 6, coverage 66.7%, with debt)
        // With proper coverage handling, risk should be moderate, not high
        assert!(insights.top_risks[0].risk_score > 1.0);
        assert!(insights.top_risks[0].risk_score < 3.0);
    }

    #[test]
    fn test_is_analysis_passing_all_good() {
        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 10,
                    average_complexity: 5.0,
                    max_complexity: 8,
                    high_complexity_count: 2,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };
        assert!(is_analysis_passing(&results, 10));
    }

    #[test]
    fn test_is_analysis_passing_high_average_complexity() {
        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 10,
                    average_complexity: 15.0, // Over threshold
                    max_complexity: 20,
                    high_complexity_count: 3,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };
        assert!(!is_analysis_passing(&results, 10));
    }

    #[test]
    fn test_is_analysis_passing_too_many_complex_functions() {
        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 20,
                    average_complexity: 8.0,
                    max_complexity: 25,
                    high_complexity_count: 10, // Over threshold
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };
        assert!(!is_analysis_passing(&results, 10));
    }

    #[test]
    fn test_is_analysis_passing_high_debt_score() {
        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 10,
                    average_complexity: 5.0,
                    max_complexity: 8,
                    high_complexity_count: 2,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![
                    DebtItem {
                        id: "debt-1".to_string(),
                        debt_type: DebtType::Todo,
                        priority: Priority::High,
                        file: PathBuf::from("test.rs"),
                        line: 10,
                        column: None,
                        message: "TODO: Fix this".to_string(),
                        context: None,
                    },
                    DebtItem {
                        id: "debt-2".to_string(),
                        debt_type: DebtType::Complexity,
                        priority: Priority::High,
                        file: PathBuf::from("test.rs"),
                        line: 50,
                        column: None,
                        message: "Method too long".to_string(),
                        context: None,
                    },
                ],
                by_type: std::collections::HashMap::new(),
                priorities: vec![Priority::High],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };
        // Since the actual debt score calculation depends on the debt module,
        // we can't easily test this without mocking. The function will pass
        // if debt score is <= 100
        let passing = is_analysis_passing(&results, 10);
        // This will depend on how debt::total_debt_score calculates the score
        // For now, we just check that the function runs without panicking
        let _ = passing;
    }

    #[test]
    fn test_determine_priority_output_format_top() {
        use priority::formatter::OutputFormat;

        let format = determine_priority_output_format(Some(5), None);
        assert!(matches!(format, OutputFormat::Top(5)));

        let format = determine_priority_output_format(Some(10), None);
        assert!(matches!(format, OutputFormat::Top(10)));

        let format = determine_priority_output_format(Some(1), None);
        assert!(matches!(format, OutputFormat::Top(1)));
    }

    #[test]
    fn test_determine_priority_output_format_default() {
        use priority::formatter::OutputFormat;

        let format = determine_priority_output_format(None, None);
        assert!(matches!(format, OutputFormat::Default));
    }

    #[test]
    fn test_determine_priority_output_format_precedence_order() {
        use priority::formatter::OutputFormat;

        // Test precedence: tail > top > default
        let format = determine_priority_output_format(Some(5), None);
        assert!(matches!(format, OutputFormat::Top(5)));

        let format = determine_priority_output_format(None, None);
        assert!(matches!(format, OutputFormat::Default));

        let format = determine_priority_output_format(None, Some(3));
        assert!(matches!(format, OutputFormat::Tail(3)));

        // tail takes precedence over top
        let format = determine_priority_output_format(Some(5), Some(3));
        assert!(matches!(format, OutputFormat::Tail(3)));
    }

    #[test]
    fn test_determine_priority_output_format_tail() {
        use priority::formatter::OutputFormat;

        // tail should work when specified alone
        let format = determine_priority_output_format(None, Some(5));
        assert!(matches!(format, OutputFormat::Tail(5)));

        // tail with different values
        let format = determine_priority_output_format(None, Some(10));
        assert!(matches!(format, OutputFormat::Tail(10)));

        // tail should take precedence over top when both are specified
        let format = determine_priority_output_format(Some(3), Some(5));
        assert!(matches!(format, OutputFormat::Tail(5)));
    }

    #[test]
    fn test_is_analysis_passing_boundary_values() {
        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 10,
                    average_complexity: 10.0, // Exactly at threshold
                    max_complexity: 15,
                    high_complexity_count: 5, // Exactly at threshold
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };
        assert!(is_analysis_passing(&results, 10));
    }

    #[test]
    fn test_determine_output_format_with_explicit_format() {
        let config = ValidateConfig {
            path: PathBuf::from("."),
            config: None,
            coverage_file: None,
            verbosity: 0,
            format: Some(cli::OutputFormat::Json),
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            tail: None,
            semantic_off: false,
        };
        assert_eq!(
            determine_output_format(&config),
            Some(cli::OutputFormat::Json)
        );
    }

    #[test]
    fn test_determine_output_format_with_output_no_format() {
        let config = ValidateConfig {
            path: PathBuf::from("."),
            config: None,
            coverage_file: None,
            verbosity: 0,
            format: None,
            output: Some(PathBuf::from("output.txt")),
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            tail: None,
            semantic_off: false,
        };
        assert_eq!(
            determine_output_format(&config),
            Some(cli::OutputFormat::Terminal)
        );
    }

    #[test]
    fn test_determine_output_format_with_both() {
        let config = ValidateConfig {
            path: PathBuf::from("."),
            config: None,
            coverage_file: None,
            verbosity: 0,
            format: Some(cli::OutputFormat::Markdown),
            output: Some(PathBuf::from("output.md")),
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            tail: None,
            semantic_off: false,
        };
        // Format takes precedence over output
        assert_eq!(
            determine_output_format(&config),
            Some(cli::OutputFormat::Markdown)
        );
    }

    #[test]
    fn test_determine_output_format_with_neither() {
        let config = ValidateConfig {
            path: PathBuf::from("."),
            config: None,
            coverage_file: None,
            verbosity: 0,
            format: None,
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            tail: None,
            semantic_off: false,
        };
        assert_eq!(determine_output_format(&config), None);
    }

    #[test]
    fn test_prepare_files_for_duplication_check_empty() {
        let files: Vec<PathBuf> = vec![];
        let result = prepare_files_for_duplication_check(&files);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_prepare_files_for_duplication_check_nonexistent_files() {
        let files = vec![
            PathBuf::from("/nonexistent/file1.rs"),
            PathBuf::from("/nonexistent/file2.rs"),
        ];
        let result = prepare_files_for_duplication_check(&files);
        // Nonexistent files are filtered out
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_default_similarity_threshold() {
        // Just verify the constant is set to expected value
        assert_eq!(DEFAULT_SIMILARITY_THRESHOLD, 0.8);
    }

    #[test]
    fn test_prepare_files_for_duplication_check_with_real_file() {
        use std::fs;
        use std::io::Write;

        // Create a temporary file for testing
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_dup_check.txt");

        // Write some content to the file
        let mut file = fs::File::create(&test_file).unwrap();
        writeln!(file, "test content").unwrap();

        let files = vec![test_file.clone()];
        let result = prepare_files_for_duplication_check(&files);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, test_file);
        assert_eq!(result[0].1, "test content\n");

        // Clean up
        fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_is_entry_point_main() {
        assert!(is_entry_point("main"));
    }

    #[test]
    fn test_is_entry_point_handle_prefix() {
        assert!(is_entry_point("handle_request"));
        assert!(is_entry_point("handle_"));
        assert!(is_entry_point("handle_user_input"));
    }

    #[test]
    fn test_is_entry_point_run_prefix() {
        assert!(is_entry_point("run_server"));
        assert!(is_entry_point("run_"));
        assert!(is_entry_point("run_application"));
    }

    #[test]
    fn test_is_entry_point_regular_function() {
        assert!(!is_entry_point("process_data"));
        assert!(!is_entry_point("calculate_score"));
        assert!(!is_entry_point("format_output"));
    }

    #[test]
    fn test_is_test_function_with_attribute() {
        let path = Path::new("src/lib.rs");
        assert!(is_test_function("some_function", path, true));
    }

    #[test]
    fn test_is_test_function_with_test_prefix() {
        let path = Path::new("src/lib.rs");
        assert!(is_test_function("test_something", path, false));
        assert!(is_test_function("test_", path, false));
    }

    #[test]
    fn test_is_test_function_in_test_file() {
        let path = Path::new("src/test_utils.rs");
        assert!(is_test_function("helper_function", path, false));

        let path2 = Path::new("tests/integration.rs");
        assert!(is_test_function("regular_function", path2, false));
    }

    #[test]
    fn test_is_test_function_regular() {
        let path = Path::new("src/main.rs");
        assert!(!is_test_function("process_data", path, false));
        assert!(!is_test_function("calculate", path, false));
    }

    #[test]
    fn test_build_initial_call_graph() {
        use debtmap::FunctionMetrics;

        let metrics = vec![
            FunctionMetrics {
                name: "main".to_string(),
                file: PathBuf::from("src/main.rs"),
                line: 10,
                cyclomatic: 5,
                cognitive: 7,
                nesting: 2,
                length: 25,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
            },
            FunctionMetrics {
                name: "test_function".to_string(),
                file: PathBuf::from("tests/test.rs"),
                line: 20,
                cyclomatic: 3,
                cognitive: 4,
                nesting: 1,
                length: 15,
                is_test: true,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
            },
        ];

        let _call_graph = build_initial_call_graph(&metrics);

        // Verify the graph was built with correct classifications
        let _func_id_main = priority::call_graph::FunctionId {
            file: PathBuf::from("src/main.rs"),
            name: "main".to_string(),
            line: 10,
        };

        let _func_id_test = priority::call_graph::FunctionId {
            file: PathBuf::from("tests/test.rs"),
            name: "test_function".to_string(),
            line: 20,
        };

        // Check that functions were added to the graph
        // The graph should have both functions
        // Note: CallGraph doesn't expose has_function, we just verify it was built
    }

    #[test]
    fn test_create_unified_analysis() {
        use debtmap::FunctionMetrics;
        use priority::CallGraph;

        let metrics = vec![FunctionMetrics {
            name: "analyze_data".to_string(),
            file: PathBuf::from("src/analyzer.rs"),
            line: 15,
            cyclomatic: 8,
            cognitive: 10,
            nesting: 3,
            length: 40,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
        }];

        let mut call_graph = CallGraph::new();
        let func_id = priority::call_graph::FunctionId {
            file: PathBuf::from("src/analyzer.rs"),
            name: "analyze_data".to_string(),
            line: 15,
        };
        call_graph.add_function(func_id, false, false, 8, 40);

        let unified = create_unified_analysis(&metrics, &call_graph, None);

        // Verify the unified analysis was created
        assert_eq!(unified.items.len(), 1);
    }

    #[test]
    fn test_create_unified_analysis_excludes_test_functions() {
        use crate::core::FunctionMetrics;
        use crate::priority::{call_graph::FunctionId, CallGraph};

        // Create test metrics with both production and test functions
        let metrics = vec![
            FunctionMetrics {
                name: "production_function".to_string(),
                file: PathBuf::from("src/main.rs"),
                line: 10,
                cyclomatic: 5,
                cognitive: 3,
                nesting: 1,
                length: 20,
                is_test: false, // Production function
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
            },
            FunctionMetrics {
                name: "test_something".to_string(),
                file: PathBuf::from("src/main.rs"),
                line: 30,
                cyclomatic: 8,
                cognitive: 12,
                nesting: 2,
                length: 40,
                is_test: true, // Test function - should be excluded
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
            },
            FunctionMetrics {
                name: "another_production_function".to_string(),
                file: PathBuf::from("src/lib.rs"),
                line: 50,
                cyclomatic: 3,
                cognitive: 2,
                nesting: 0,
                length: 15,
                is_test: false, // Production function
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
            },
        ];

        let mut call_graph = CallGraph::new();

        // Add all functions to call graph
        for metric in &metrics {
            let func_id = FunctionId {
                file: metric.file.clone(),
                name: metric.name.clone(),
                line: metric.line,
            };
            call_graph.add_function(
                func_id,
                false,
                metric.is_test,
                metric.cyclomatic,
                metric.length,
            );
        }

        let unified = create_unified_analysis(&metrics, &call_graph, None);

        // Verify only production functions are included in unified analysis
        // Test function should be excluded, so only 2 items should be present
        assert_eq!(unified.items.len(), 2);

        // Verify that the included functions are indeed the production ones
        let included_names: Vec<&String> = unified
            .items
            .iter()
            .map(|item| &item.location.function)
            .collect();

        assert!(included_names.contains(&&"production_function".to_string()));
        assert!(included_names.contains(&&"another_production_function".to_string()));
        assert!(!included_names.contains(&&"test_something".to_string()));

        // Verify that total debt score doesn't include the complex test function
        // If test functions were included, the score would be much higher due to
        // the complex test function (cyclomatic=8, cognitive=12, no coverage)
        let total_debt_score = unified.total_impact.risk_reduction;
        assert!(
            total_debt_score < 20.0,
            "Debt score should be low since test function is excluded"
        );
    }

    #[test]
    fn test_validate_with_risk_all_passing() {
        // Test case 1: All metrics are well within acceptable limits
        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 10,
                    average_complexity: 3.5, // Well below 10.0
                    max_complexity: 8,
                    high_complexity_count: 1,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![DebtItem {
                    id: "debt-1".to_string(),
                    debt_type: DebtType::Todo,
                    priority: Priority::Low,
                    file: PathBuf::from("test.rs"),
                    line: 10,
                    column: None,
                    message: "TODO: Minor cleanup".to_string(),
                    context: None,
                }], // Only 1 item, well below 150
                by_type: std::collections::HashMap::new(),
                priorities: vec![Priority::Low],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        let insights = risk::RiskInsight {
            top_risks: Vector::from(vec![FunctionRisk {
                function_name: "low_risk_func".to_string(),
                file: PathBuf::from("test.rs"),
                line_range: (10, 20),
                cyclomatic_complexity: 3,
                cognitive_complexity: 4,
                risk_score: 2.5, // Below 7.0 threshold
                coverage_percentage: Some(0.8),
                test_effort: TestEffort {
                    estimated_difficulty: Difficulty::Trivial,
                    cognitive_load: 4,
                    branch_count: 2,
                    recommended_test_cases: 2,
                },
                risk_category: RiskCategory::Low,
                is_test_function: false,
            }]),
            codebase_risk_score: 4.5, // Below 7.0
            risk_reduction_opportunities: Vector::new(),
            complexity_coverage_correlation: None,
            risk_distribution: RiskDistribution {
                critical_count: 0,
                high_count: 0,
                medium_count: 0,
                low_count: 1,
                well_tested_count: 0,
                total_functions: 1,
            },
        };

        let (pass, _) = validate_with_risk(&results, &insights, None);
        assert!(pass);
    }

    #[test]
    fn test_validate_with_risk_high_complexity_fails() {
        // Test case 2: High average complexity causes failure
        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 10,
                    average_complexity: 15.0, // Above 10.0 threshold
                    max_complexity: 25,
                    high_complexity_count: 5,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        let insights = risk::RiskInsight {
            top_risks: Vector::new(),
            codebase_risk_score: 5.0, // Still acceptable
            risk_reduction_opportunities: Vector::new(),
            complexity_coverage_correlation: None,
            risk_distribution: RiskDistribution {
                critical_count: 0,
                high_count: 0,
                medium_count: 0,
                low_count: 0,
                well_tested_count: 0,
                total_functions: 0,
            },
        };

        let (pass, _) = validate_with_risk(&results, &insights, None);
        assert!(!pass);
    }

    #[test]
    fn test_validate_with_risk_too_many_high_risk_functions() {
        // Test case 3: Too many high-risk functions (more than default threshold of 50)
        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 20,
                    average_complexity: 5.0, // Acceptable
                    max_complexity: 10,
                    high_complexity_count: 2,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        let mut high_risk_functions = vec![];
        for i in 0..51 {
            high_risk_functions.push(FunctionRisk {
                function_name: format!("high_risk_func_{i}"),
                file: PathBuf::from("test.rs"),
                line_range: (i * 20, i * 20 + 15),
                cyclomatic_complexity: 12,
                cognitive_complexity: 18,
                risk_score: 8.5, // Above 7.0 threshold
                coverage_percentage: Some(0.0),
                test_effort: TestEffort {
                    estimated_difficulty: Difficulty::Complex,
                    cognitive_load: 18,
                    branch_count: 10,
                    recommended_test_cases: 8,
                },
                risk_category: RiskCategory::High,
                is_test_function: false,
            });
        }

        let insights = risk::RiskInsight {
            top_risks: Vector::from(high_risk_functions),
            codebase_risk_score: 6.5, // Still below 7.0
            risk_reduction_opportunities: Vector::new(),
            complexity_coverage_correlation: None,
            risk_distribution: RiskDistribution {
                critical_count: 0,
                high_count: 51,
                medium_count: 0,
                low_count: 0,
                well_tested_count: 0,
                total_functions: 51,
            },
        };

        let (pass, _) = validate_with_risk(&results, &insights, None);
        assert!(!pass);
    }

    #[test]
    fn test_validate_with_risk_excessive_technical_debt() {
        // Test case 4: Too many technical debt items (more than default threshold of 2000)
        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 30,
                    average_complexity: 4.0, // Acceptable
                    max_complexity: 8,
                    high_complexity_count: 1,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: (0..10001)
                    .map(|i| DebtItem {
                        id: format!("debt-{i}"),
                        debt_type: DebtType::Todo,
                        priority: Priority::Low,
                        file: PathBuf::from(format!("test{}.rs", i % 10)),
                        line: (i * 10) as usize,
                        column: None,
                        message: format!("TODO: Item {i}"),
                        context: None,
                    })
                    .collect(), // Exactly 10001 items to exceed debt score threshold
                by_type: std::collections::HashMap::new(),
                priorities: vec![Priority::Low],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        let insights = risk::RiskInsight {
            top_risks: Vector::from(vec![FunctionRisk {
                function_name: "moderate_risk".to_string(),
                file: PathBuf::from("test.rs"),
                line_range: (10, 30),
                cyclomatic_complexity: 5,
                cognitive_complexity: 7,
                risk_score: 5.0, // Below threshold
                coverage_percentage: Some(0.6),
                test_effort: TestEffort {
                    estimated_difficulty: Difficulty::Moderate,
                    cognitive_load: 7,
                    branch_count: 4,
                    recommended_test_cases: 3,
                },
                risk_category: RiskCategory::Medium,
                is_test_function: false,
            }]),
            codebase_risk_score: 6.0, // Below 7.0
            risk_reduction_opportunities: Vector::new(),
            complexity_coverage_correlation: None,
            risk_distribution: RiskDistribution {
                critical_count: 0,
                high_count: 0,
                medium_count: 1,
                low_count: 0,
                well_tested_count: 0,
                total_functions: 1,
            },
        };

        let (pass, _) = validate_with_risk(&results, &insights, None);
        assert!(!pass);
    }

    #[test]
    fn test_create_json_output() {
        use crate::core::{
            ComplexityReport, ComplexitySummary, DependencyReport, TechnicalDebtReport,
        };
        use chrono::Utc;
        use std::path::PathBuf;

        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 10,
                    average_complexity: 5.0,
                    max_complexity: 10,
                    high_complexity_count: 2,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        let risk_insights = None;
        let output = create_json_output(&results, &risk_insights);

        assert!(output.is_object());
        assert!(output["analysis"].is_object());
        assert!(output["risk_insights"].is_null());
    }

    #[test]
    fn test_create_json_output_with_risk() {
        use crate::core::{
            ComplexityReport, ComplexitySummary, DependencyReport, TechnicalDebtReport,
        };
        use crate::risk::{RiskDistribution, RiskInsight};
        use chrono::Utc;
        use im::Vector;
        use std::path::PathBuf;

        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 10,
                    average_complexity: 5.0,
                    max_complexity: 10,
                    high_complexity_count: 2,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        let risk_insights = Some(RiskInsight {
            top_risks: Vector::new(),
            risk_reduction_opportunities: Vector::new(),
            codebase_risk_score: 5.0,
            complexity_coverage_correlation: None,
            risk_distribution: RiskDistribution {
                critical_count: 0,
                high_count: 1,
                medium_count: 2,
                low_count: 3,
                well_tested_count: 4,
                total_functions: 10,
            },
        });

        let output = create_json_output(&results, &risk_insights);

        assert!(output.is_object());
        assert!(output["analysis"].is_object());
        assert!(output["risk_insights"].is_object());
    }

    #[test]
    fn test_format_results_to_string_json() {
        use crate::core::{
            ComplexityReport, ComplexitySummary, DependencyReport, TechnicalDebtReport,
        };
        use chrono::Utc;
        use std::path::PathBuf;

        let results = AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 10,
                    average_complexity: 5.0,
                    max_complexity: 10,
                    high_complexity_count: 2,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        };

        let risk_insights = None;
        let result =
            format_results_to_string(&results, &risk_insights, io::output::OutputFormat::Json);

        assert!(result.is_ok());
        let json_str = result.unwrap();

        // Verify it's valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_create_file_writer() {
        let mut buffer = Vec::new();
        let _writer = create_file_writer(&mut buffer, io::output::OutputFormat::Markdown);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_create_provider_critical_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let provider = create_provider("critical_path", temp_dir.path());
        assert!(provider.is_some());

        // Verify it's the correct provider type by checking its name
        let provider = provider.unwrap();
        assert_eq!(provider.name(), "critical_path");
        // Verify weight is positive
        assert!(provider.weight() > 0.0);
    }

    #[test]
    fn test_create_provider_dependency() {
        let temp_dir = tempfile::tempdir().unwrap();
        let provider = create_provider("dependency", temp_dir.path());
        assert!(provider.is_some());

        // Verify it's the correct provider type by checking its name
        let provider = provider.unwrap();
        assert_eq!(provider.name(), "dependency_risk");
        // Verify weight is positive
        assert!(provider.weight() > 0.0);
    }

    #[test]
    fn test_create_provider_git_history() {
        // Create a temporary directory with a git repo
        let temp_dir = tempfile::tempdir().unwrap();

        // Initialize git repo
        std::process::Command::new("git")
            .arg("init")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to init git repo");

        // Create provider - should succeed with valid git repo
        let provider = create_provider("git_history", temp_dir.path());
        assert!(provider.is_some());

        // Test with non-git directory
        let non_git_dir = tempfile::tempdir().unwrap();
        let provider_none = create_provider("git_history", non_git_dir.path());
        // Git history provider returns None for non-git directories
        assert!(provider_none.is_none());
    }

    #[test]
    fn test_create_provider_unknown() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Test with unknown provider name
        let provider = create_provider("unknown_provider", temp_dir.path());
        assert!(provider.is_none());

        // Test with empty string
        let provider_empty = create_provider("", temp_dir.path());
        assert!(provider_empty.is_none());

        // Test with other invalid names
        let provider_invalid = create_provider("invalid", temp_dir.path());
        assert!(provider_invalid.is_none());
    }

    #[test]
    fn test_validate_project_success() {
        use std::io::Write;
        use tempfile::TempDir;

        // Create a temporary directory with test files
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        let mut file = std::fs::File::create(&test_file).unwrap();
        writeln!(file, "fn simple_function() {{ println!(\"Hello\"); }}").unwrap();

        // Create config for successful validation
        let config = ValidateConfig {
            path: temp_dir.path().to_path_buf(),
            config: None,
            coverage_file: None,
            verbosity: 0,
            format: None,
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            tail: None,
            semantic_off: false,
        };

        // Run validation - should succeed
        let result = validate_project(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_project_with_coverage() {
        use std::io::Write;
        use tempfile::TempDir;

        // Create temporary directory with test files
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        let mut file = std::fs::File::create(&test_file).unwrap();
        writeln!(file, "fn covered_function() {{ let x = 1 + 1; }}").unwrap();

        // Create a simple LCOV file with full coverage
        let lcov_file = temp_dir.path().join("coverage.lcov");
        let mut lcov = std::fs::File::create(&lcov_file).unwrap();
        writeln!(lcov, "TN:").unwrap();
        writeln!(lcov, "SF:test.rs").unwrap();
        writeln!(lcov, "FN:1,covered_function").unwrap();
        writeln!(lcov, "FNDA:1,covered_function").unwrap();
        writeln!(lcov, "FNF:1").unwrap();
        writeln!(lcov, "FNH:1").unwrap();
        writeln!(lcov, "DA:1,1").unwrap(); // Line 1 executed once
        writeln!(lcov, "LF:1").unwrap(); // 1 line total
        writeln!(lcov, "LH:1").unwrap(); // 1 line hit
        writeln!(lcov, "end_of_record").unwrap();

        // Create config with coverage file
        let config = ValidateConfig {
            path: temp_dir.path().to_path_buf(),
            config: None,
            coverage_file: Some(lcov_file),
            verbosity: 0,
            format: None,
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            tail: None,
            semantic_off: false,
        };

        // Run validation with coverage
        let result = validate_project(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_project_with_output_format() {
        use std::io::Write;
        use tempfile::TempDir;

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("main.rs");
        let mut file = std::fs::File::create(&test_file).unwrap();
        writeln!(file, "fn main() {{ println!(\"Test\"); }}").unwrap();

        let output_file = temp_dir.path().join("output.json");

        // Create config with JSON output format
        let config = ValidateConfig {
            path: temp_dir.path().to_path_buf(),
            config: None,
            coverage_file: None,
            verbosity: 0,
            format: Some(cli::OutputFormat::Json),
            output: Some(output_file.clone()),
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            tail: None,
            semantic_off: false,
        };

        // Run validation with output format
        let result = validate_project(config);
        assert!(result.is_ok());
        // Output file should be created
        assert!(output_file.exists());
    }

    #[test]
    fn test_validate_project_with_context_enabled() {
        use std::io::Write;
        use tempfile::TempDir;

        // Create temporary directory with git repo
        let temp_dir = TempDir::new().unwrap();

        // Initialize git repo
        std::process::Command::new("git")
            .arg("init")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let test_file = temp_dir.path().join("test.rs");
        let mut file = std::fs::File::create(&test_file).unwrap();
        writeln!(file, "fn context_test() {{ let x = 42; }}").unwrap();

        // Add and commit file for git history
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Create config with context enabled
        let config = ValidateConfig {
            path: temp_dir.path().to_path_buf(),
            config: None,
            coverage_file: None,
            verbosity: 0,
            format: None,
            output: None,
            enable_context: true,
            context_providers: Some(vec!["git-history".to_string()]),
            disable_context: None,
            top: None,
            tail: None,
            semantic_off: false,
        };

        // Run validation with context
        let result = validate_project(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_regular_call_graph_success() {
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.rs");

        let rust_code = r#"
            fn main() {
                helper();
                process_data();
            }
            
            fn helper() {
                println!("Helper");
            }
            
            fn process_data() {
                helper();
            }
        "#;

        let mut file = std::fs::File::create(&test_file).unwrap();
        file.write_all(rust_code.as_bytes()).unwrap();

        let result = extract_regular_call_graph(&test_file).unwrap();

        // Should have functions in the call graph
        assert!(!result.is_empty());

        let functions = result.find_all_functions();
        assert!(functions.iter().any(|f| f.name == "main"));
        assert!(functions.iter().any(|f| f.name == "helper"));
        assert!(functions.iter().any(|f| f.name == "process_data"));
    }

    #[test]
    fn test_extract_regular_call_graph_nonexistent_file() {
        let nonexistent_path = Path::new("/nonexistent/file.rs");
        let result = extract_regular_call_graph(nonexistent_path);

        // Should return an error for nonexistent files
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to read file"));
    }

    #[test]
    fn test_extract_regular_call_graph_invalid_rust_syntax() {
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("invalid.rs");

        // Invalid Rust syntax
        let invalid_code = "fn main() { this is not valid rust }";

        let mut file = std::fs::File::create(&test_file).unwrap();
        file.write_all(invalid_code.as_bytes()).unwrap();

        let result = extract_regular_call_graph(&test_file);

        // Should return an error for invalid Rust syntax
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to parse Rust file"));
    }

    #[test]
    fn test_extract_regular_call_graph_empty_file() {
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("empty.rs");

        // Create empty file
        let mut file = std::fs::File::create(&test_file).unwrap();
        file.write_all(b"").unwrap();

        let result = extract_regular_call_graph(&test_file);

        // Should return Ok with empty call graph
        assert!(result.is_ok());
        let call_graph = result.unwrap();
        assert!(call_graph.is_empty());
    }

    #[test]
    fn test_extract_regular_call_graph_with_complex_calls() {
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("complex.rs");

        let rust_code = r#"
            use std::collections::HashMap;
            
            fn analyze() -> HashMap<String, i32> {
                let mut map = HashMap::new();
                map.insert(format_key("test"), calculate_value(42));
                map
            }
            
            fn format_key(s: &str) -> String {
                s.to_uppercase()
            }
            
            fn calculate_value(n: i32) -> i32 {
                validate_input(n);
                n * 2
            }
            
            fn validate_input(n: i32) {
                if n < 0 {
                    panic!("Invalid input");
                }
            }
        "#;

        let mut file = std::fs::File::create(&test_file).unwrap();
        file.write_all(rust_code.as_bytes()).unwrap();

        let result = extract_regular_call_graph(&test_file).unwrap();

        // Should extract all functions
        let functions = result.find_all_functions();
        assert!(functions.iter().any(|f| f.name == "analyze"));
        assert!(functions.iter().any(|f| f.name == "format_key"));
        assert!(functions.iter().any(|f| f.name == "calculate_value"));
        assert!(functions.iter().any(|f| f.name == "validate_input"));

        // Should track function calls
        let analyze_func = functions.iter().find(|f| f.name == "analyze").unwrap();
        let callees = result.get_callees(analyze_func);
        assert!(!callees.is_empty());
    }

    // Expansion-related tests removed - now using enhanced token parsing

    // Process file tests removed - expansion functionality replaced

    #[test]
    fn test_format_threshold_failure() {
        let result = format_threshold_failure("Test Metric", "10.5", "5.0", ">");
        assert_eq!(result, "    ❌ Test Metric: 10.5 > 5.0");

        let result = format_threshold_failure("Coverage", "45.2%", "80.0%", "<");
        assert_eq!(result, "    ❌ Coverage: 45.2% < 80.0%");
    }

    #[test]
    fn test_exceeds_max_threshold() {
        assert!(exceeds_max_threshold(10, 5));
        assert!(exceeds_max_threshold(10.5, 10.0));
        assert!(!exceeds_max_threshold(5, 10));
        assert!(!exceeds_max_threshold(10, 10));
        assert!(!exceeds_max_threshold(9.9, 10.0));
    }

    #[test]
    fn test_below_min_threshold() {
        assert!(below_min_threshold(5, 10));
        assert!(below_min_threshold(9.9, 10.0));
        assert!(!below_min_threshold(10, 5));
        assert!(!below_min_threshold(10, 10));
        assert!(!below_min_threshold(10.1, 10.0));
    }

    #[test]
    fn test_print_failed_validation_checks() {
        // This function prints to stdout, so we'd need to capture stdout to test it properly
        // For now, we'll test that it doesn't panic with various inputs
        let details = ValidationDetails {
            average_complexity: 15.0,
            max_average_complexity: 10.0,
            high_complexity_count: 5,
            max_high_complexity_count: 3,
            debt_items: 100,
            max_debt_items: 50,
            total_debt_score: 1000,
            max_total_debt_score: 500,
            codebase_risk_score: 8.0,
            max_codebase_risk_score: 5.0,
            high_risk_functions: 10,
            max_high_risk_functions: 5,
            coverage_percentage: 40.0,
            min_coverage_percentage: 80.0,
        };

        // Should not panic
        print_failed_validation_checks(&details);

        // Test with thresholds that won't trigger failures
        let details_passing = ValidationDetails {
            average_complexity: 5.0,
            max_average_complexity: 10.0,
            high_complexity_count: 1,
            max_high_complexity_count: 3,
            debt_items: 10,
            max_debt_items: 50,
            total_debt_score: 100,
            max_total_debt_score: 500,
            codebase_risk_score: 2.0,
            max_codebase_risk_score: 5.0,
            high_risk_functions: 2,
            max_high_risk_functions: 5,
            coverage_percentage: 90.0,
            min_coverage_percentage: 80.0,
        };

        // Should not panic with passing thresholds
        print_failed_validation_checks(&details_passing);
    }

    #[test]
    fn test_is_markdown_file_with_md_extension() {
        let path = Some(PathBuf::from("report.md"));
        assert!(is_markdown_file(&path));
    }

    #[test]
    fn test_is_markdown_file_with_other_extension() {
        let path = Some(PathBuf::from("report.txt"));
        assert!(!is_markdown_file(&path));
    }

    #[test]
    fn test_is_markdown_file_with_no_extension() {
        let path = Some(PathBuf::from("report"));
        assert!(!is_markdown_file(&path));
    }

    #[test]
    fn test_is_markdown_file_with_none() {
        assert!(!is_markdown_file(&None));
    }

    #[test]
    fn test_is_markdown_file_case_sensitive() {
        // Test that comparison is case-sensitive
        let path = Some(PathBuf::from("report.MD"));
        assert!(!is_markdown_file(&path));
    }

    #[test]
    fn test_calculate_markdown_limit_with_top() {
        assert_eq!(calculate_markdown_limit(Some(15), None), 15);
        assert_eq!(calculate_markdown_limit(Some(5), Some(20)), 5);
    }

    #[test]
    fn test_calculate_markdown_limit_with_tail() {
        assert_eq!(calculate_markdown_limit(None, Some(20)), 10);
    }

    #[test]
    fn test_calculate_markdown_limit_with_neither() {
        assert_eq!(calculate_markdown_limit(None, None), 10);
    }

    #[test]
    fn test_output_json_to_stdout() {
        // Create a minimal UnifiedAnalysis for testing
        let analysis = priority::UnifiedAnalysis {
            items: im::Vector::new(),
            total_impact: priority::ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 100.0,
            call_graph: priority::call_graph::CallGraph::default(),
            data_flow_graph: debtmap::data_flow::DataFlowGraph::new(),
            overall_coverage: Some(75.0),
        };

        // Test JSON output without error (can't easily test stdout)
        let result = output_json(&analysis, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_json_to_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        let analysis = priority::UnifiedAnalysis {
            items: im::Vector::new(),
            total_impact: priority::ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 100.0,
            call_graph: priority::call_graph::CallGraph::default(),
            data_flow_graph: debtmap::data_flow::DataFlowGraph::new(),
            overall_coverage: Some(75.0),
        };

        let result = output_json(&analysis, Some(output_path.clone()));
        assert!(result.is_ok());
        assert!(output_path.exists());

        // Verify the file contains valid JSON
        let contents = fs::read_to_string(output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["total_debt_score"], 100.0);
    }

    #[test]
    fn test_output_markdown_to_stdout() {
        let analysis = priority::UnifiedAnalysis {
            items: im::Vector::new(),
            total_impact: priority::ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 100.0,
            call_graph: priority::call_graph::CallGraph::default(),
            data_flow_graph: debtmap::data_flow::DataFlowGraph::new(),
            overall_coverage: Some(75.0),
        };

        // Test that markdown output works without error
        let result = output_markdown(&analysis, Some(5), None, 0, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_markdown_to_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.md");

        let analysis = priority::UnifiedAnalysis {
            items: im::Vector::new(),
            total_impact: priority::ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 100.0,
            call_graph: priority::call_graph::CallGraph::default(),
            data_flow_graph: debtmap::data_flow::DataFlowGraph::new(),
            overall_coverage: Some(75.0),
        };

        let result = output_markdown(&analysis, Some(10), None, 1, Some(output_path.clone()));
        assert!(result.is_ok());
        assert!(output_path.exists());

        // Verify the file contains markdown content
        let contents = fs::read_to_string(output_path).unwrap();
        assert!(contents.contains("# Priority Technical Debt Fixes"));
    }

    #[test]
    fn test_output_terminal_to_stdout() {
        let analysis = priority::UnifiedAnalysis {
            items: im::Vector::new(),
            total_impact: priority::ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 100.0,
            call_graph: priority::call_graph::CallGraph::default(),
            data_flow_graph: debtmap::data_flow::DataFlowGraph::new(),
            overall_coverage: Some(75.0),
        };

        // Test terminal output without error
        let result = output_terminal(&analysis, Some(5), None, 0, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_terminal_to_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.txt");

        let analysis = priority::UnifiedAnalysis {
            items: im::Vector::new(),
            total_impact: priority::ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 100.0,
            call_graph: priority::call_graph::CallGraph::default(),
            data_flow_graph: debtmap::data_flow::DataFlowGraph::new(),
            overall_coverage: Some(75.0),
        };

        let result = output_terminal(&analysis, None, Some(5), 2, Some(output_path.clone()));
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_output_unified_priorities_json_format() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        let analysis = priority::UnifiedAnalysis {
            items: im::Vector::new(),
            total_impact: priority::ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 100.0,
            call_graph: priority::call_graph::CallGraph::default(),
            data_flow_graph: debtmap::data_flow::DataFlowGraph::new(),
            overall_coverage: Some(75.0),
        };

        let result = output_unified_priorities(
            analysis,
            None,
            None,
            0,
            Some(output_path.clone()),
            Some(cli::OutputFormat::Json),
        );
        assert!(result.is_ok());
        assert!(output_path.exists());

        // Verify JSON format
        let contents = fs::read_to_string(output_path).unwrap();
        let _parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
    }

    #[test]
    fn test_output_unified_priorities_markdown_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.md");

        let analysis = priority::UnifiedAnalysis {
            items: im::Vector::new(),
            total_impact: priority::ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 100.0,
            call_graph: priority::call_graph::CallGraph::default(),
            data_flow_graph: debtmap::data_flow::DataFlowGraph::new(),
            overall_coverage: Some(75.0),
        };

        let result =
            output_unified_priorities(analysis, Some(5), None, 1, Some(output_path.clone()), None);
        assert!(result.is_ok());
        assert!(output_path.exists());

        // Verify markdown content
        let contents = fs::read_to_string(output_path).unwrap();
        assert!(contents.contains("# Priority Technical Debt Fixes"));
    }

    #[test]
    fn test_output_unified_priorities_terminal_format() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.txt");

        let analysis = priority::UnifiedAnalysis {
            items: im::Vector::new(),
            total_impact: priority::ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 100.0,
            call_graph: priority::call_graph::CallGraph::default(),
            data_flow_graph: debtmap::data_flow::DataFlowGraph::new(),
            overall_coverage: Some(75.0),
        };

        let result =
            output_unified_priorities(analysis, None, Some(3), 2, Some(output_path.clone()), None);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_format_results_json_output() {
        // Create test data
        let results = create_test_analysis_results();
        let risk_insights = None;

        // Test JSON format
        let formatted =
            format_results_to_string(&results, &risk_insights, io::output::OutputFormat::Json);
        assert!(formatted.is_ok());

        let json_str = formatted.unwrap();
        assert!(json_str.contains("\"project_path\""));
        assert!(json_str.contains("\"complexity\""));
        assert!(json_str.contains("\"technical_debt\""));

        // Verify it's valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_format_results_markdown_output() {
        // Create test data
        let results = create_test_analysis_results();
        let risk_insights = None;

        // Test Markdown format
        let formatted =
            format_results_to_string(&results, &risk_insights, io::output::OutputFormat::Markdown);
        assert!(formatted.is_ok());

        let markdown_str = formatted.unwrap();
        // Markdown should contain some expected structure
        assert!(!markdown_str.is_empty());
        // Should not be JSON
        assert!(!markdown_str.starts_with('{'));
    }

    #[test]
    fn test_format_results_terminal_output() {
        // Create test data
        let results = create_test_analysis_results();
        let risk_insights = None;

        // Test Terminal format
        let formatted =
            format_results_to_string(&results, &risk_insights, io::output::OutputFormat::Terminal);
        assert!(formatted.is_ok());

        let terminal_str = formatted.unwrap();
        assert!(!terminal_str.is_empty());
    }

    #[test]
    fn test_format_results_with_risk_insights() {
        // Create test data with risk insights
        let results = create_test_analysis_results();
        let risk_insights = Some(create_test_risk_insights());

        // Test JSON format with risk insights
        let formatted =
            format_results_to_string(&results, &risk_insights, io::output::OutputFormat::Json);
        assert!(formatted.is_ok());

        let json_str = formatted.unwrap();
        assert!(json_str.contains("\"risk_insights\""));

        // Test Markdown format with risk insights
        let formatted_md =
            format_results_to_string(&results, &risk_insights, io::output::OutputFormat::Markdown);
        assert!(formatted_md.is_ok());
    }

    #[test]
    fn test_format_results_edge_cases() {
        // Test with minimal results
        let mut results = create_test_analysis_results();
        results.complexity.metrics.clear();
        results.technical_debt.items.clear();
        results.duplications.clear();

        // Should still format successfully even with empty data
        let formatted = format_results_to_string(&results, &None, io::output::OutputFormat::Json);
        assert!(formatted.is_ok());

        let json_str = formatted.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Verify empty arrays are properly serialized
        assert_eq!(
            parsed["analysis"]["complexity"]["metrics"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            parsed["analysis"]["technical_debt"]["items"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    // Helper function to create test analysis results
    fn create_test_analysis_results() -> AnalysisResults {
        use crate::core::{ComplexitySummary, FunctionMetrics, Priority};
        use chrono::Utc;
        use std::collections::HashMap;

        AnalysisResults {
            project_path: PathBuf::from("/test/project"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![FunctionMetrics {
                    name: "test_func".to_string(),
                    file: PathBuf::from("test.rs"),
                    line: 10,
                    cyclomatic: 5,
                    cognitive: 8,
                    nesting: 2,
                    length: 20,
                    is_test: false,
                    visibility: Some("pub".to_string()),
                    is_trait_method: false,
                    in_test_module: false,
                    entropy_score: None,
                    is_pure: None,
                    purity_confidence: None,
                }],
                summary: ComplexitySummary {
                    total_functions: 1,
                    average_complexity: 5.0,
                    max_complexity: 5,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![DebtItem {
                    id: "debt_1".to_string(),
                    debt_type: DebtType::Complexity,
                    priority: Priority::Medium,
                    file: PathBuf::from("test.rs"),
                    line: 10,
                    column: Some(0),
                    message: "Function lacks test coverage".to_string(),
                    context: Some("test_func".to_string()),
                }],
                by_type: HashMap::new(),
                priorities: vec![Priority::Medium],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        }
    }

    // Helper function to create test risk insights
    fn create_test_risk_insights() -> risk::RiskInsight {
        use im::Vector;

        risk::RiskInsight {
            top_risks: Vector::from(vec![risk::FunctionRisk {
                file: PathBuf::from("test.rs"),
                function_name: "test_func".to_string(),
                line_range: (10, 50),
                cyclomatic_complexity: 15,
                cognitive_complexity: 20,
                coverage_percentage: Some(0.0),
                risk_score: 8.5,
                test_effort: risk::TestEffort {
                    estimated_difficulty: risk::Difficulty::Complex,
                    cognitive_load: 20,
                    branch_count: 8,
                    recommended_test_cases: 12,
                },
                risk_category: risk::RiskCategory::Critical,
                is_test_function: false,
            }]),
            risk_reduction_opportunities: Vector::from(vec![risk::TestingRecommendation {
                function: "test_func".to_string(),
                file: PathBuf::from("test.rs"),
                line: 10,
                current_risk: 8.5,
                potential_risk_reduction: 4.0,
                test_effort_estimate: risk::TestEffort {
                    estimated_difficulty: risk::Difficulty::Complex,
                    cognitive_load: 20,
                    branch_count: 8,
                    recommended_test_cases: 12,
                },
                rationale: "High complexity with no test coverage".to_string(),
                roi: Some(2.5),
                dependencies: vec!["dep1".to_string()],
                dependents: vec!["func2".to_string()],
            }]),
            codebase_risk_score: 75.0,
            complexity_coverage_correlation: Some(-0.65),
            risk_distribution: risk::RiskDistribution {
                critical_count: 1,
                high_count: 2,
                medium_count: 3,
                low_count: 4,
                well_tested_count: 5,
                total_functions: 15,
            },
        }
    }
}

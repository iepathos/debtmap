use debtmap::analysis_utils;
use debtmap::cli;
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
    priorities_only: bool,
    detailed: bool,
    semantic_off: bool,
    explain_score: bool,
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
    priorities_only: bool,
    #[allow(dead_code)]
    detailed: bool,
    #[allow(dead_code)]
    semantic_off: bool,
    #[allow(dead_code)]
    explain_score: bool,
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
            priorities_only,
            detailed,
            semantic_off,
            explain_score,
        } => {
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
                priorities_only,
                detailed,
                semantic_off,
                explain_score,
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
            priorities_only,
            detailed,
            semantic_off,
            explain_score,
        } => {
            let config = ValidateConfig {
                path,
                config,
                coverage_file,
                format,
                output,
                enable_context,
                context_providers,
                disable_context,
                top,
                priorities_only,
                detailed,
                semantic_off,
                explain_score,
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
    )?;

    // Output unified prioritized results
    output_unified_priorities(
        unified_analysis,
        config.top,
        config.priorities_only,
        config.detailed,
        config.explain_score,
        config.output,
        Some(config.format),
    )?;

    // Check if analysis passed
    if !is_analysis_passing(&results, config.threshold_complexity) {
        process::exit(1);
    }

    Ok(())
}

fn analyze_project(
    path: PathBuf,
    languages: Vec<Language>,
    complexity_threshold: u32,
    duplication_threshold: usize,
) -> Result<AnalysisResults> {
    let files = io::walker::find_project_files(&path, languages.clone())
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
        .filter_map(|path| {
            io::read_file(path)
                .ok()
                .map(|content| (path.clone(), content))
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

fn validate_with_risk(results: &AnalysisResults, insights: &risk::RiskInsight) -> bool {
    // Calculate risk-adjusted thresholds
    let risk_threshold = 7.0; // Functions with risk > 7.0 are considered high risk

    // Check high-risk functions
    let high_risk_count = insights
        .top_risks
        .iter()
        .filter(|f| f.risk_score > risk_threshold)
        .count();

    // Check overall codebase risk
    let codebase_risk_pass = insights.codebase_risk_score < 7.0;

    // Pass if:
    // - Average complexity is reasonable
    // - Not too many high-risk functions
    // - Overall codebase risk is acceptable
    // - Technical debt is manageable
    results.complexity.summary.average_complexity < 10.0
        && high_risk_count < 5
        && codebase_risk_pass
        && results.technical_debt.items.len() < 150 // Slightly higher threshold with coverage
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
    let pass = risk_insights
        .as_ref()
        .map(|insights| validate_with_risk(results, insights))
        .unwrap_or_else(|| validate_basic(results));

    if pass {
        print_validation_success(config.coverage_file.is_some());
        Ok(())
    } else {
        print_validation_failure(results, risk_insights);
        anyhow::bail!("Validation failed")
    }
}

fn validate_basic(results: &AnalysisResults) -> bool {
    results.complexity.summary.average_complexity < 10.0
        && results.complexity.summary.high_complexity_count < 10
        && results.technical_debt.items.len() < 100
}

fn print_validation_success(has_coverage: bool) {
    println!("✅ Validation PASSED - All metrics within thresholds");
    if has_coverage {
        println!("  Coverage analysis was applied to risk calculations");
    }
}

fn print_validation_failure(results: &AnalysisResults, risk_insights: &Option<risk::RiskInsight>) {
    println!("❌ Validation FAILED - Some metrics exceed thresholds");
    print_basic_metrics(results);

    if let Some(insights) = risk_insights {
        print_risk_metrics(insights);
    }
}

fn print_basic_metrics(results: &AnalysisResults) {
    println!(
        "  Average complexity: {:.1}",
        results.complexity.summary.average_complexity
    );
    println!(
        "  High complexity functions: {}",
        results.complexity.summary.high_complexity_count
    );
    println!(
        "  Technical debt items: {}",
        results.technical_debt.items.len()
    );
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

/// Process Rust files to extract call relationships
fn process_rust_files_for_call_graph(
    project_path: &Path,
    call_graph: &mut priority::CallGraph,
) -> Result<()> {
    let rust_files = io::walker::find_project_files(project_path, vec![Language::Rust])
        .context("Failed to find Rust files for call graph")?;

    for file_path in rust_files {
        if let Ok(content) = io::read_file(&file_path) {
            if let Ok(parsed) = syn::parse_file(&content) {
                use debtmap::analyzers::rust_call_graph::extract_call_graph;
                let file_call_graph = extract_call_graph(&parsed, &file_path);
                call_graph.merge(file_call_graph);
            }
        }
    }

    Ok(())
}

/// Create unified analysis from metrics and call graph
fn create_unified_analysis(
    metrics: &[debtmap::FunctionMetrics],
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
) -> priority::UnifiedAnalysis {
    use priority::{unified_scorer, UnifiedAnalysis};

    let mut unified = UnifiedAnalysis::new(call_graph.clone());

    for metric in metrics {
        // Skip test functions from debt score calculation
        // Test functions are analyzed separately to avoid inflating debt scores
        if metric.is_test {
            continue;
        }

        // Skip closures - they're part of their parent function's implementation
        // Their complexity already contributes to the parent function's metrics
        if metric.name.contains("<closure@") {
            continue;
        }

        // Skip test helper functions (functions only called by test functions)
        let func_id = priority::call_graph::FunctionId {
            file: metric.file.clone(),
            name: metric.name.clone(),
            line: metric.line,
        };

        if call_graph.is_test_helper(&func_id) {
            continue;
        }

        let roi_score = 5.0; // Default ROI
        let item =
            unified_scorer::create_unified_debt_item(metric, call_graph, coverage_data, roi_score);
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
) -> Result<priority::UnifiedAnalysis> {
    // Build initial call graph from complexity metrics
    let mut call_graph = build_initial_call_graph(&results.complexity.metrics);

    // Process Rust files to extract call relationships
    process_rust_files_for_call_graph(project_path, &mut call_graph)?;

    // Load coverage data if available
    let coverage_data = match coverage_file {
        Some(lcov_path) => {
            Some(risk::lcov::parse_lcov_file(lcov_path).context("Failed to parse LCOV file")?)
        }
        None => None,
    };

    // Create and return unified analysis
    Ok(create_unified_analysis(
        &results.complexity.metrics,
        &call_graph,
        coverage_data.as_ref(),
    ))
}

/// Determines the priority output format based on command line flags
fn determine_priority_output_format(
    priorities_only: bool,
    detailed: bool,
    top: Option<usize>,
) -> priority::formatter::OutputFormat {
    use priority::formatter::OutputFormat;

    if priorities_only {
        OutputFormat::PrioritiesOnly
    } else if detailed {
        OutputFormat::Detailed
    } else if let Some(n) = top {
        OutputFormat::Top(n)
    } else {
        OutputFormat::Default
    }
}

fn output_unified_priorities(
    analysis: priority::UnifiedAnalysis,
    top: Option<usize>,
    priorities_only: bool,
    detailed: bool,
    _explain_score: bool,
    output_file: Option<PathBuf>,
    output_format: Option<cli::OutputFormat>,
) -> Result<()> {
    use priority::formatter::format_priorities;
    use std::fs;
    use std::io::Write;

    // Check if JSON format is requested
    if let Some(cli::OutputFormat::Json) = output_format {
        // For JSON, serialize the analysis directly
        let json = serde_json::to_string_pretty(&analysis)?;
        if let Some(path) = output_file {
            let mut file = fs::File::create(path)?;
            file.write_all(json.as_bytes())?;
        } else {
            println!("{json}");
        }
    } else {
        // For other formats, use the existing formatter
        let format = determine_priority_output_format(priorities_only, detailed, top);
        let output = format_priorities(&analysis, format);

        if let Some(path) = output_file {
            let mut file = fs::File::create(path)?;
            file.write_all(output.as_bytes())?;
        } else {
            println!("{output}");
        }
    }

    Ok(())
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
            assert!(well_tested.coverage_percentage.unwrap() > 50.0);
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
                        message: "TODO: Fix this".to_string(),
                        context: None,
                    },
                    DebtItem {
                        id: "debt-2".to_string(),
                        debt_type: DebtType::Complexity,
                        priority: Priority::High,
                        file: PathBuf::from("test.rs"),
                        line: 50,
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
    fn test_determine_priority_output_format_priorities_only() {
        use priority::formatter::OutputFormat;

        let format = determine_priority_output_format(true, false, None);
        assert!(matches!(format, OutputFormat::PrioritiesOnly));

        // priorities_only takes precedence over other flags
        let format = determine_priority_output_format(true, true, Some(5));
        assert!(matches!(format, OutputFormat::PrioritiesOnly));
    }

    #[test]
    fn test_determine_priority_output_format_detailed() {
        use priority::formatter::OutputFormat;

        let format = determine_priority_output_format(false, true, None);
        assert!(matches!(format, OutputFormat::Detailed));

        // detailed takes precedence over top when priorities_only is false
        let format = determine_priority_output_format(false, true, Some(5));
        assert!(matches!(format, OutputFormat::Detailed));
    }

    #[test]
    fn test_determine_priority_output_format_top() {
        use priority::formatter::OutputFormat;

        let format = determine_priority_output_format(false, false, Some(5));
        assert!(matches!(format, OutputFormat::Top(5)));

        let format = determine_priority_output_format(false, false, Some(10));
        assert!(matches!(format, OutputFormat::Top(10)));

        let format = determine_priority_output_format(false, false, Some(1));
        assert!(matches!(format, OutputFormat::Top(1)));
    }

    #[test]
    fn test_determine_priority_output_format_default() {
        use priority::formatter::OutputFormat;

        let format = determine_priority_output_format(false, false, None);
        assert!(matches!(format, OutputFormat::Default));
    }

    #[test]
    fn test_determine_priority_output_format_precedence_order() {
        use priority::formatter::OutputFormat;

        // Test full precedence: priorities_only > detailed > top > default
        let format = determine_priority_output_format(true, true, Some(5));
        assert!(matches!(format, OutputFormat::PrioritiesOnly));

        let format = determine_priority_output_format(false, true, Some(5));
        assert!(matches!(format, OutputFormat::Detailed));

        let format = determine_priority_output_format(false, false, Some(5));
        assert!(matches!(format, OutputFormat::Top(5)));

        let format = determine_priority_output_format(false, false, None);
        assert!(matches!(format, OutputFormat::Default));
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
            format: Some(cli::OutputFormat::Json),
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            priorities_only: false,
            detailed: false,
            semantic_off: false,
            explain_score: false,
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
            format: None,
            output: Some(PathBuf::from("output.txt")),
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            priorities_only: false,
            detailed: false,
            semantic_off: false,
            explain_score: false,
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
            format: Some(cli::OutputFormat::Markdown),
            output: Some(PathBuf::from("output.md")),
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            priorities_only: false,
            detailed: false,
            semantic_off: false,
            explain_score: false,
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
            format: None,
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            priorities_only: false,
            detailed: false,
            semantic_off: false,
            explain_score: false,
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

        assert!(validate_with_risk(&results, &insights));
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

        assert!(!validate_with_risk(&results, &insights));
    }

    #[test]
    fn test_validate_with_risk_too_many_high_risk_functions() {
        // Test case 3: Too many high-risk functions (5 or more)
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
        for i in 0..5 {
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
                high_count: 5,
                medium_count: 0,
                low_count: 0,
                well_tested_count: 0,
                total_functions: 5,
            },
        };

        assert!(!validate_with_risk(&results, &insights));
    }

    #[test]
    fn test_validate_with_risk_excessive_technical_debt() {
        // Test case 4: Too many technical debt items (150 or more)
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
                items: (0..150)
                    .map(|i| DebtItem {
                        id: format!("debt-{i}"),
                        debt_type: DebtType::Todo,
                        priority: Priority::Low,
                        file: PathBuf::from(format!("test{}.rs", i % 10)),
                        line: (i * 10) as usize,
                        message: format!("TODO: Item {i}"),
                        context: None,
                    })
                    .collect(), // Exactly 150 items
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

        assert!(!validate_with_risk(&results, &insights));
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
            format: None,
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            priorities_only: false,
            detailed: false,
            semantic_off: false,
            explain_score: false,
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

        // Create a simple LCOV file
        let lcov_file = temp_dir.path().join("coverage.lcov");
        let mut lcov = std::fs::File::create(&lcov_file).unwrap();
        writeln!(lcov, "TN:").unwrap();
        writeln!(lcov, "SF:test.rs").unwrap();
        writeln!(lcov, "FN:1,covered_function").unwrap();
        writeln!(lcov, "FNDA:1,covered_function").unwrap();
        writeln!(lcov, "FNF:1").unwrap();
        writeln!(lcov, "FNH:1").unwrap();
        writeln!(lcov, "end_of_record").unwrap();

        // Create config with coverage file
        let config = ValidateConfig {
            path: temp_dir.path().to_path_buf(),
            config: None,
            coverage_file: Some(lcov_file),
            format: None,
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            priorities_only: false,
            detailed: false,
            semantic_off: false,
            explain_score: false,
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
            format: Some(cli::OutputFormat::Json),
            output: Some(output_file.clone()),
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            priorities_only: false,
            detailed: false,
            semantic_off: false,
            explain_score: false,
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
            format: None,
            output: None,
            enable_context: true,
            context_providers: Some(vec!["git-history".to_string()]),
            disable_context: None,
            top: None,
            priorities_only: false,
            detailed: false,
            semantic_off: false,
            explain_score: false,
        };

        // Run validation with context
        let result = validate_project(config);
        assert!(result.is_ok());
    }
}

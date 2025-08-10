use debtmap::analyzers;
use debtmap::cli;
use debtmap::core;
use debtmap::debt;
use debtmap::io;
use debtmap::risk;

use anyhow::{Context, Result};
use chrono::Utc;
use cli::Commands;
use core::{
    AnalysisResults, ComplexityReport, ComplexitySummary, DependencyReport, FileMetrics, Language,
    TechnicalDebtReport,
};
use debt::circular::analyze_module_dependencies;
use rayon::prelude::*;
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
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
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
        } => {
            let config = AnalyzeConfig {
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

    // Handle risk analysis if coverage file provided
    let risk_insights = if let Some(lcov_path) = config.coverage_file {
        analyze_risk_with_coverage(
            &results,
            &lcov_path,
            &config.path,
            config.enable_context,
            config.context_providers,
            config.disable_context,
        )?
    } else {
        analyze_risk_without_coverage(
            &results,
            config.enable_context,
            config.context_providers,
            config.disable_context,
            &config.path,
        )?
    };

    // Output results
    output_results_with_risk(
        results.clone(),
        risk_insights,
        config.format.into(),
        config.output,
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

    let file_metrics = collect_file_metrics(&files);
    let all_functions = extract_all_functions(&file_metrics);
    let all_debt_items = extract_all_debt_items(&file_metrics);
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

fn collect_file_metrics(files: &[PathBuf]) -> Vec<FileMetrics> {
    files
        .par_iter()
        .filter_map(|path| analyze_single_file(path.as_path()))
        .collect()
}

fn extract_all_functions(file_metrics: &[FileMetrics]) -> Vec<core::FunctionMetrics> {
    file_metrics
        .iter()
        .flat_map(|m| &m.complexity.functions)
        .cloned()
        .collect()
}

fn extract_all_debt_items(file_metrics: &[FileMetrics]) -> Vec<core::DebtItem> {
    file_metrics
        .iter()
        .flat_map(|m| &m.debt_items)
        .cloned()
        .collect()
}

fn detect_duplications(files: &[PathBuf], threshold: usize) -> Vec<core::DuplicationBlock> {
    let files_with_content: Vec<(PathBuf, String)> = files
        .iter()
        .filter_map(|path| {
            io::read_file(path)
                .ok()
                .map(|content| (path.clone(), content))
        })
        .collect();

    debt::duplication::detect_duplication(files_with_content, threshold, 0.8)
}

fn build_complexity_report(
    all_functions: &[core::FunctionMetrics],
    complexity_threshold: u32,
) -> ComplexityReport {
    ComplexityReport {
        metrics: all_functions.to_vec(),
        summary: ComplexitySummary {
            total_functions: all_functions.len(),
            average_complexity: core::metrics::calculate_average_complexity(all_functions),
            max_complexity: core::metrics::find_max_complexity(all_functions),
            high_complexity_count: core::metrics::count_high_complexity(
                all_functions,
                complexity_threshold,
            ),
        },
    }
}

fn build_technical_debt_report(
    all_debt_items: Vec<core::DebtItem>,
    duplications: Vec<core::DuplicationBlock>,
) -> TechnicalDebtReport {
    let debt_by_type = debt::categorize_debt(all_debt_items.clone());
    let priorities = debt::prioritize_debt(all_debt_items.clone())
        .into_iter()
        .map(|item| item.priority)
        .collect();

    TechnicalDebtReport {
        items: all_debt_items,
        by_type: debt_by_type,
        priorities,
        duplications,
    }
}

fn output_results_with_risk(
    results: AnalysisResults,
    risk_insights: Option<risk::RiskInsight>,
    format: io::output::OutputFormat,
    output_file: Option<PathBuf>,
) -> Result<()> {
    let mut writer = io::output::create_writer(format);
    writer.write_results(&results)?;

    // Add risk insights if available
    if let Some(insights) = risk_insights {
        writer.write_risk_insights(&insights)?;
    }

    if let Some(path) = output_file {
        io::write_file(&path, &format!("{results:?}"))?;
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

    let mut aggregator = risk::context::ContextAggregator::new();

    // Determine which providers to enable
    let enabled_providers = if let Some(providers) = context_providers {
        providers
    } else {
        // Default providers
        vec![
            "critical_path".to_string(),
            "dependency".to_string(),
            "git_history".to_string(),
        ]
    };

    let disabled = disable_context.unwrap_or_default();

    for provider_name in enabled_providers {
        if disabled.contains(&provider_name) {
            continue;
        }

        match provider_name.as_str() {
            "critical_path" => {
                // For now, create a simple critical path analyzer
                // In a real implementation, we'd build this from the AST
                let analyzer = risk::context::critical_path::CriticalPathAnalyzer::new();
                let provider = risk::context::critical_path::CriticalPathProvider::new(analyzer);
                aggregator = aggregator.with_provider(Box::new(provider));
            }
            "dependency" => {
                // Create dependency graph from analysis results
                let graph = risk::context::dependency::DependencyGraph::new();
                let provider = risk::context::dependency::DependencyRiskProvider::new(graph);
                aggregator = aggregator.with_provider(Box::new(provider));
            }
            "git_history" => {
                // Try to create git history provider
                if let Ok(provider) =
                    risk::context::git_history::GitHistoryProvider::new(project_path.to_path_buf())
                {
                    aggregator = aggregator.with_provider(Box::new(provider));
                }
            }
            _ => {
                eprintln!("Warning: Unknown context provider: {provider_name}");
            }
        }
    }

    Some(aggregator)
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

        // Try to get coverage for this function
        let coverage = lcov_data.get_function_coverage(&func.file, &func.name);

        let risk = analyzer.analyze_function(
            func.file.clone(),
            func.name.clone(),
            (func.line, func.line + func.length),
            &complexity_metrics,
            coverage,
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

    // Handle risk analysis if coverage file provided
    let risk_insights = if let Some(lcov_path) = config.coverage_file.clone() {
        analyze_risk_with_coverage(
            &results,
            &lcov_path,
            &config.path,
            config.enable_context,
            config.context_providers.clone(),
            config.disable_context.clone(),
        )?
    } else if config.enable_context {
        analyze_risk_without_coverage(
            &results,
            config.enable_context,
            config.context_providers.clone(),
            config.disable_context.clone(),
            &config.path,
        )?
    } else {
        None
    };

    // If format and/or output are specified, generate report
    if config.format.is_some() || config.output.is_some() {
        let output_format = config.format.unwrap_or(cli::OutputFormat::Terminal);
        output_results_with_risk(
            results.clone(),
            risk_insights.clone(),
            output_format.into(),
            config.output.clone(),
        )?;
    }

    // Apply validation with coverage-aware risk if available
    let pass = if let Some(ref insights) = risk_insights {
        // With coverage: use risk-adjusted validation
        validate_with_risk(&results, insights)
    } else {
        // Without coverage: use basic validation
        results.complexity.summary.average_complexity < 10.0
            && results.complexity.summary.high_complexity_count < 10
            && results.technical_debt.items.len() < 100
    };

    if pass {
        println!("✅ Validation PASSED - All metrics within thresholds");
        if config.coverage_file.is_some() {
            println!("  Coverage analysis was applied to risk calculations");
        }
        Ok(())
    } else {
        println!("❌ Validation FAILED - Some metrics exceed thresholds");
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

        if let Some(insights) = risk_insights {
            println!("\n  Overall codebase risk score: {:.1}", insights.codebase_risk_score);
            
            if !insights.top_risks.is_empty() {
                println!("\n  Critical risk functions (high complexity + low/no coverage):");
                for func in insights.top_risks.iter().take(5) {
                    let coverage_str = func.coverage_percentage
                        .map(|c| format!("{:.0}%", c * 100.0))
                        .unwrap_or_else(|| "0%".to_string());
                    println!("    - {} (risk: {:.1}, coverage: {})", 
                        func.function_name, func.risk_score, coverage_str);
                }
            }
        }

        anyhow::bail!("Validation failed");
    }
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

fn analyze_single_file(file_path: &Path) -> Option<FileMetrics> {
    let content = io::read_file(file_path).ok()?;
    let ext = file_path.extension()?.to_str()?;
    let language = Language::from_extension(ext);

    (language != Language::Unknown)
        .then(|| {
            let analyzer = analyzers::get_analyzer(language);
            analyzers::analyze_file(content, file_path.to_path_buf(), analyzer.as_ref())
        })?
        .ok()
}

fn is_analysis_passing(results: &AnalysisResults, _complexity_threshold: u32) -> bool {
    let debt_score = debt::total_debt_score(&results.technical_debt.items);
    let debt_threshold = 100;

    results.complexity.summary.average_complexity <= 10.0
        && results.complexity.summary.high_complexity_count <= 5
        && debt_score <= debt_threshold
}

fn create_dependency_report(file_metrics: &[FileMetrics]) -> DependencyReport {
    let file_deps: Vec<(std::path::PathBuf, Vec<core::Dependency>)> = file_metrics
        .iter()
        .map(|m| (m.path.clone(), m.dependencies.clone()))
        .collect();

    let dep_graph = analyze_module_dependencies(&file_deps);

    DependencyReport {
        modules: dep_graph.calculate_coupling_metrics(),
        circular: dep_graph.detect_circular_dependencies(),
    }
}

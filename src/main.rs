use debtmap::analyzers;
use debtmap::cli;
use debtmap::core;
use debtmap::debt;
use debtmap::io;

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

fn main() -> Result<()> {
    let cli = cli::parse_args();

    match cli.command {
        Commands::Analyze {
            path,
            format,
            output,
            threshold_complexity,
            threshold_duplication,
            languages,
        } => handle_analyze(
            path,
            format,
            output,
            threshold_complexity,
            threshold_duplication,
            languages,
        ),
        Commands::Complexity {
            path,
            format,
            threshold,
        } => handle_complexity(path, format, threshold),
        Commands::Debt {
            path,
            format,
            min_priority,
        } => handle_debt(path, format, min_priority),
        Commands::Deps { path, format } => handle_deps(path, format),
        Commands::Init { force } => init_config(force),
        Commands::Validate { path, config } => validate_project(path, config),
    }
}

fn handle_analyze(
    path: PathBuf,
    format: cli::OutputFormat,
    output: Option<PathBuf>,
    threshold_complexity: u32,
    threshold_duplication: usize,
    languages: Option<Vec<String>>,
) -> Result<()> {
    let languages = parse_languages(languages);
    let results = analyze_project(path, languages, threshold_complexity, threshold_duplication)?;
    output_results(results, format.into(), output)
}

fn handle_complexity(path: PathBuf, format: cli::OutputFormat, threshold: u32) -> Result<()> {
    let results = analyze_complexity_only(path, threshold)?;
    output_results(results, format.into(), None)
}

fn handle_debt(
    path: PathBuf,
    format: cli::OutputFormat,
    min_priority: Option<cli::Priority>,
) -> Result<()> {
    let results = analyze_debt_only(path, min_priority.map(Into::into))?;
    output_results(results, format.into(), None)
}

fn handle_deps(path: PathBuf, format: cli::OutputFormat) -> Result<()> {
    let results = analyze_dependencies_only(path)?;
    output_results(results, format.into(), None)
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

fn analyze_complexity_only(path: PathBuf, threshold: u32) -> Result<AnalysisResults> {
    analyze_project(path, default_languages(), threshold, 50)
}

fn analyze_debt_only(
    path: PathBuf,
    min_priority: Option<core::Priority>,
) -> Result<AnalysisResults> {
    let mut results = analyze_project(path, default_languages(), 10, 50)?;

    if let Some(priority) = min_priority {
        results.technical_debt.items =
            debt::filter_by_priority(results.technical_debt.items, priority);
    }

    Ok(results)
}

fn analyze_dependencies_only(path: PathBuf) -> Result<AnalysisResults> {
    analyze_project(path, default_languages(), 10, 50)
}

fn output_results(
    results: AnalysisResults,
    format: io::output::OutputFormat,
    output_file: Option<PathBuf>,
) -> Result<()> {
    let mut writer = io::output::create_writer(format);
    writer.write_results(&results)?;

    if let Some(path) = output_file {
        io::write_file(&path, &format!("{results:?}"))?;
    }

    Ok(())
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

fn validate_project(path: PathBuf, _config: Option<PathBuf>) -> Result<()> {
    let results = analyze_project(path, vec![Language::Rust, Language::Python], 10, 50)?;

    let pass = results.complexity.summary.average_complexity < 10.0
        && results.complexity.summary.high_complexity_count < 10
        && results.technical_debt.items.len() < 100;

    if pass {
        println!("✅ Validation PASSED - All metrics within thresholds");
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

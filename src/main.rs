mod analyzers;
mod cli;
mod complexity;
mod core;
mod debt;
mod io;
mod transformers;

use anyhow::{Context, Result};
use chrono::Utc;
use cli::Commands;
use core::{
    AnalysisResults, ComplexityReport, ComplexitySummary, DependencyReport, FileMetrics, Language,
    TechnicalDebtReport,
};
use rayon::prelude::*;
use std::path::PathBuf;

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
        } => {
            let languages = parse_languages(languages);
            let results =
                analyze_project(path, languages, threshold_complexity, threshold_duplication)?;

            output_results(results, format.into(), output)?;
        }

        Commands::Complexity {
            path,
            format,
            threshold,
        } => {
            let results = analyze_complexity_only(path, threshold)?;
            output_results(results, format.into(), None)?;
        }

        Commands::Debt {
            path,
            format,
            min_priority,
        } => {
            let results = analyze_debt_only(path, min_priority.map(Into::into))?;
            output_results(results, format.into(), None)?;
        }

        Commands::Deps { path, format } => {
            let results = analyze_dependencies_only(path)?;
            output_results(results, format.into(), None)?;
        }

        Commands::Init { force } => {
            init_config(force)?;
        }

        Commands::Validate { path, config } => {
            validate_project(path, config)?;
        }
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

    let file_metrics: Vec<FileMetrics> = files
        .par_iter()
        .filter_map(|file_path| {
            let content = io::read_file(file_path).ok()?;
            let ext = file_path.extension()?.to_str()?;
            let language = Language::from_extension(ext);

            if language == Language::Unknown {
                return None;
            }

            let analyzer = analyzers::get_analyzer(language);
            analyzers::analyze_file(content, file_path.clone(), analyzer.as_ref()).ok()
        })
        .collect();

    let all_functions: Vec<_> = file_metrics
        .iter()
        .flat_map(|m| m.complexity.functions.clone())
        .collect();

    let all_debt_items: Vec<_> = file_metrics
        .iter()
        .flat_map(|m| m.debt_items.clone())
        .collect();

    let files_for_duplication: Vec<(PathBuf, String)> = files
        .iter()
        .filter_map(|path| {
            let content = io::read_file(path).ok()?;
            Some((path.clone(), content))
        })
        .collect();

    let _duplications =
        debt::duplication::detect_duplication(files_for_duplication, duplication_threshold, 0.8);

    let complexity_report = ComplexityReport {
        metrics: all_functions.clone(),
        summary: ComplexitySummary {
            total_functions: all_functions.len(),
            average_complexity: core::metrics::calculate_average_complexity(&all_functions),
            max_complexity: core::metrics::find_max_complexity(&all_functions),
            high_complexity_count: core::metrics::count_high_complexity(
                &all_functions,
                complexity_threshold,
            ),
        },
    };

    let debt_by_type = debt::categorize_debt(all_debt_items.clone());
    let priorities = debt::prioritize_debt(all_debt_items.clone())
        .into_iter()
        .map(|item| item.priority)
        .collect();

    let technical_debt = TechnicalDebtReport {
        items: all_debt_items,
        by_type: debt_by_type,
        priorities,
    };

    let dependencies = DependencyReport {
        modules: Vec::new(),
        circular: Vec::new(),
    };

    Ok(AnalysisResults {
        project_path: path,
        timestamp: Utc::now(),
        complexity: complexity_report,
        technical_debt,
        dependencies,
    })
}

fn analyze_complexity_only(path: PathBuf, threshold: u32) -> Result<AnalysisResults> {
    analyze_project(path, vec![Language::Rust, Language::Python], threshold, 50)
}

fn analyze_debt_only(
    path: PathBuf,
    min_priority: Option<core::Priority>,
) -> Result<AnalysisResults> {
    let mut results = analyze_project(path, vec![Language::Rust, Language::Python], 10, 50)?;

    if let Some(priority) = min_priority {
        let filtered_items =
            debt::filter_by_priority(results.technical_debt.items.into_iter().collect(), priority);
        results.technical_debt.items = filtered_items;
    }

    Ok(results)
}

fn analyze_dependencies_only(path: PathBuf) -> Result<AnalysisResults> {
    analyze_project(path, vec![Language::Rust, Language::Python], 10, 50)
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
                .map(|l| match l.as_str() {
                    "rust" => Language::Rust,
                    "python" => Language::Python,
                    _ => Language::Unknown,
                })
                .filter(|l| *l != Language::Unknown)
                .collect()
        })
        .unwrap_or_else(|| vec![Language::Rust, Language::Python])
}

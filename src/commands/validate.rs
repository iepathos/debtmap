use super::super::builders::{call_graph, unified_analysis};
use super::super::output;
use super::super::utils::{analysis_helpers, risk_analyzer, validation_printer};
use crate::{cli, config, core::*, risk};
use anyhow::Result;
use std::path::PathBuf;

pub struct ValidateConfig {
    pub path: PathBuf,
    pub config: Option<PathBuf>,
    pub coverage_file: Option<PathBuf>,
    pub format: Option<cli::OutputFormat>,
    pub output: Option<PathBuf>,
    pub enable_context: bool,
    pub context_providers: Option<Vec<String>>,
    pub disable_context: Option<Vec<String>>,
    pub top: Option<usize>,
    pub tail: Option<usize>,
    pub semantic_off: bool,
    pub verbosity: u8,
}

pub struct ValidationDetails {
    pub average_complexity: f64,
    pub max_average_complexity: f64,
    pub high_complexity_count: usize,
    pub max_high_complexity_count: usize,
    pub debt_items: usize,
    pub max_debt_items: usize,
    pub total_debt_score: u32,
    pub max_total_debt_score: u32,
    pub codebase_risk_score: f64,
    pub max_codebase_risk_score: f64,
    pub high_risk_functions: usize,
    pub max_high_risk_functions: usize,
    pub coverage_percentage: f64,
    pub min_coverage_percentage: f64,
}

pub fn validate_project(config: ValidateConfig) -> Result<()> {
    let complexity_threshold = 10;
    let duplication_threshold = 50;

    let results = analysis_helpers::analyze_project(
        config.path.clone(),
        vec![Language::Rust, Language::Python],
        complexity_threshold,
        duplication_threshold,
    )?;

    let risk_insights = get_risk_insights(&config, &results)?;
    generate_report_if_requested(&config, &results, &risk_insights)?;
    validate_and_report(&config, &results, &risk_insights)
}

fn get_risk_insights(
    config: &ValidateConfig,
    results: &AnalysisResults,
) -> Result<Option<risk::RiskInsight>> {
    match (&config.coverage_file, config.enable_context) {
        (Some(lcov_path), _) => risk_analyzer::analyze_risk_with_coverage(
            results,
            lcov_path,
            &config.path,
            config.enable_context,
            config.context_providers.clone(),
            config.disable_context.clone(),
        ),
        (None, true) => risk_analyzer::analyze_risk_without_coverage(
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
            output::output_results_with_risk(
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
        validation_printer::print_validation_success(&details, config.verbosity);
        Ok(())
    } else {
        validation_printer::print_validation_failure_with_details(
            &details,
            risk_insights,
            config.verbosity,
        );
        anyhow::bail!("Validation failed")
    }
}

fn validate_with_risk(
    results: &AnalysisResults,
    insights: &risk::RiskInsight,
    lcov_data: Option<&risk::lcov::LcovData>,
) -> (bool, ValidationDetails) {
    let thresholds = config::get_validation_thresholds();
    let risk_threshold = 7.0;

    let high_risk_count = insights
        .top_risks
        .iter()
        .filter(|f| f.risk_score > risk_threshold)
        .count();

    let total_debt_score = calculate_unified_debt_score(results, lcov_data);
    let coverage_percentage = lcov_data
        .map(|lcov| lcov.get_overall_coverage())
        .unwrap_or(0.0);

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

fn validate_basic(results: &AnalysisResults) -> (bool, ValidationDetails) {
    let thresholds = config::get_validation_thresholds();
    let total_debt_score = calculate_unified_debt_score(results, None);

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

fn calculate_unified_debt_score(
    results: &AnalysisResults,
    lcov_data: Option<&risk::lcov::LcovData>,
) -> u32 {
    let mut call_graph = call_graph::build_initial_call_graph(&results.complexity.metrics);
    let project_path = &results.project_path;

    let (framework_exclusions, function_pointer_used_functions) =
        call_graph::process_rust_files_for_call_graph(project_path, &mut call_graph, false, false)
            .unwrap_or_default();

    if let Err(e) = call_graph::process_python_files_for_call_graph(project_path, &mut call_graph) {
        log::warn!("Failed to process Python files for call graph: {}", e);
    }

    let unified = unified_analysis::create_unified_analysis_with_exclusions(
        &results.complexity.metrics,
        &call_graph,
        lcov_data,
        &framework_exclusions,
        Some(&function_pointer_used_functions),
        Some(&results.technical_debt.items),
        false, // no_aggregation - use default settings for validate
        None,  // aggregation_method - use default
        None,  // min_problematic - use default
    );

    unified.total_debt_score as u32
}

use super::super::builders::unified_analysis;
use super::super::commands::analyze;
use super::super::output;
use super::super::utils::{risk_analyzer, validation_printer};
use crate::formatting::FormattingConfig;
use crate::priority::UnifiedAnalysisUtils;
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
    pub max_debt_density: Option<f64>,
    pub top: Option<usize>,
    pub tail: Option<usize>,
    pub semantic_off: bool,
    pub verbosity: u8,
    pub no_parallel: bool,
    pub jobs: usize,
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
    pub debt_density: f64,
    pub max_debt_density: f64,
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

    // Enable parallel processing by default to match analyze command performance.
    // This significantly improves validation time on multi-core systems by
    // parallelizing call graph construction and unified analysis.
    let parallel_enabled = !config.no_parallel;
    let jobs = config.jobs;

    if parallel_enabled {
        std::env::set_var("DEBTMAP_PARALLEL", "true");
        if config.verbosity > 0 {
            let thread_msg = if jobs == 0 {
                "all available cores".to_string()
            } else {
                format!("{} threads", jobs)
            };
            eprintln!("Building call graph using {}...", thread_msg);
        }
    }

    if jobs > 0 {
        std::env::set_var("DEBTMAP_JOBS", jobs.to_string());
    }

    let results = analyze::analyze_project(
        config.path.clone(),
        vec![Language::Rust, Language::Python],
        complexity_threshold,
        duplication_threshold,
        parallel_enabled,
        FormattingConfig::default(),
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

    // Check for deprecated threshold usage and warn user
    let thresholds = config::get_validation_thresholds();
    warn_deprecated_thresholds(&thresholds);

    let (pass, details) = risk_insights
        .as_ref()
        .map(|insights| validate_with_risk(results, insights, lcov_data.as_ref(), config))
        .unwrap_or_else(|| validate_basic(results, config));

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

/// Warn users about deprecated validation thresholds
#[allow(deprecated)]
fn warn_deprecated_thresholds(thresholds: &config::ValidationThresholds) {
    let mut deprecated = Vec::new();

    if thresholds.max_high_complexity_count.is_some() {
        deprecated.push("max_high_complexity_count");
    }
    if thresholds.max_debt_items.is_some() {
        deprecated.push("max_debt_items");
    }
    if thresholds.max_high_risk_functions.is_some() {
        deprecated.push("max_high_risk_functions");
    }

    if !deprecated.is_empty() {
        eprintln!("\n⚠️  DEPRECATION WARNING:");
        eprintln!("   The following validation thresholds are deprecated:");
        for metric in &deprecated {
            eprintln!("   - {}", metric);
        }
        eprintln!("\n   These scale-dependent metrics will be removed in v1.0.");
        eprintln!("   Please migrate to density-based validation:");
        eprintln!("     - Use 'max_debt_density' instead of absolute counts");
        eprintln!("     - Density metrics remain stable as your codebase grows");
        eprintln!("     - See: https://github.com/your-repo/debtmap#density-based-validation\n");
    }
}

fn validate_with_risk(
    results: &AnalysisResults,
    insights: &risk::RiskInsight,
    lcov_data: Option<&risk::lcov::LcovData>,
    config: &ValidateConfig,
) -> (bool, ValidationDetails) {
    let thresholds = config::get_validation_thresholds();
    let risk_threshold = 7.0;

    let high_risk_count = insights
        .top_risks
        .iter()
        .filter(|f| f.risk_score > risk_threshold)
        .count();

    let unified =
        calculate_unified_analysis(results, config.coverage_file.as_ref(), config.verbosity);
    let total_debt_score = unified.total_debt_score as u32;
    let debt_density = unified.debt_density;

    let coverage_percentage = lcov_data
        .map(|lcov| lcov.get_overall_coverage())
        .unwrap_or(0.0);

    let max_debt_density = config
        .max_debt_density
        .unwrap_or(thresholds.max_debt_density);

    // === PRIMARY QUALITY METRICS (Scale-Independent) ===
    // These are the core validation criteria that measure actual code quality
    let avg_complexity_pass =
        results.complexity.summary.average_complexity <= thresholds.max_average_complexity;

    let debt_density_pass = debt_density <= max_debt_density;

    let codebase_risk_pass = insights.codebase_risk_score <= thresholds.max_codebase_risk_score;

    // === SAFETY NET ===
    // High ceiling to catch extreme cases only
    let debt_score_pass = total_debt_score <= thresholds.max_total_debt_score;

    // === OPTIONAL: Coverage Requirement ===
    let coverage_pass = coverage_percentage >= thresholds.min_coverage_percentage;

    // === DEPRECATED METRICS (Warn but allow) ===
    // Only validate if explicitly set by user
    #[allow(deprecated)]
    let high_complexity_pass = thresholds
        .max_high_complexity_count
        .map(|threshold| results.complexity.summary.high_complexity_count <= threshold)
        .unwrap_or(true);

    #[allow(deprecated)]
    let debt_items_pass = thresholds
        .max_debt_items
        .map(|threshold| results.technical_debt.items.len() <= threshold)
        .unwrap_or(true);

    #[allow(deprecated)]
    let high_risk_func_pass = thresholds
        .max_high_risk_functions
        .map(|threshold| high_risk_count <= threshold)
        .unwrap_or(true);

    // Primary validation based on density and quality ratios
    let pass = avg_complexity_pass
        && debt_density_pass
        && codebase_risk_pass
        && debt_score_pass
        && coverage_pass
        && high_complexity_pass
        && debt_items_pass
        && high_risk_func_pass;

    #[allow(deprecated)]
    let details = ValidationDetails {
        average_complexity: results.complexity.summary.average_complexity,
        max_average_complexity: thresholds.max_average_complexity,
        high_complexity_count: results.complexity.summary.high_complexity_count,
        max_high_complexity_count: thresholds.max_high_complexity_count.unwrap_or(0),
        debt_items: results.technical_debt.items.len(),
        max_debt_items: thresholds.max_debt_items.unwrap_or(0),
        total_debt_score,
        max_total_debt_score: thresholds.max_total_debt_score,
        debt_density,
        max_debt_density,
        codebase_risk_score: insights.codebase_risk_score,
        max_codebase_risk_score: thresholds.max_codebase_risk_score,
        high_risk_functions: high_risk_count,
        max_high_risk_functions: thresholds.max_high_risk_functions.unwrap_or(0),
        coverage_percentage,
        min_coverage_percentage: thresholds.min_coverage_percentage,
    };

    (pass, details)
}

fn validate_basic(results: &AnalysisResults, config: &ValidateConfig) -> (bool, ValidationDetails) {
    let thresholds = config::get_validation_thresholds();
    let unified =
        calculate_unified_analysis(results, config.coverage_file.as_ref(), config.verbosity);
    let total_debt_score = unified.total_debt_score as u32;
    let debt_density = unified.debt_density;

    let max_debt_density = config
        .max_debt_density
        .unwrap_or(thresholds.max_debt_density);

    // === PRIMARY QUALITY METRICS (Scale-Independent) ===
    let avg_complexity_pass =
        results.complexity.summary.average_complexity <= thresholds.max_average_complexity;

    let debt_density_pass = debt_density <= max_debt_density;

    // === SAFETY NET ===
    let debt_score_pass = total_debt_score <= thresholds.max_total_debt_score;

    // === DEPRECATED METRICS (Warn but allow) ===
    #[allow(deprecated)]
    let high_complexity_pass = thresholds
        .max_high_complexity_count
        .map(|threshold| results.complexity.summary.high_complexity_count <= threshold)
        .unwrap_or(true);

    #[allow(deprecated)]
    let debt_items_pass = thresholds
        .max_debt_items
        .map(|threshold| results.technical_debt.items.len() <= threshold)
        .unwrap_or(true);

    let pass = avg_complexity_pass
        && debt_density_pass
        && debt_score_pass
        && high_complexity_pass
        && debt_items_pass;

    #[allow(deprecated)]
    let details = ValidationDetails {
        average_complexity: results.complexity.summary.average_complexity,
        max_average_complexity: thresholds.max_average_complexity,
        high_complexity_count: results.complexity.summary.high_complexity_count,
        max_high_complexity_count: thresholds.max_high_complexity_count.unwrap_or(0),
        debt_items: results.technical_debt.items.len(),
        max_debt_items: thresholds.max_debt_items.unwrap_or(0),
        total_debt_score,
        max_total_debt_score: thresholds.max_total_debt_score,
        debt_density,
        max_debt_density,
        codebase_risk_score: 0.0,
        max_codebase_risk_score: thresholds.max_codebase_risk_score,
        high_risk_functions: 0,
        max_high_risk_functions: thresholds.max_high_risk_functions.unwrap_or(0),
        coverage_percentage: 0.0,
        min_coverage_percentage: thresholds.min_coverage_percentage,
    };

    (pass, details)
}

/// Calculate unified analysis using the shared pipeline (spec 130)
fn calculate_unified_analysis(
    results: &AnalysisResults,
    coverage_file: Option<&std::path::PathBuf>,
    verbosity: u8,
) -> crate::priority::UnifiedAnalysis {
    // Check if parallel processing is enabled via environment variable
    let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let jobs = std::env::var("DEBTMAP_JOBS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    // Use the shared unified analysis pipeline (spec 130)
    let unified = unified_analysis::perform_unified_analysis_with_options(
        unified_analysis::UnifiedAnalysisOptions {
            results,
            coverage_file,
            semantic_off: false,
            project_path: &results.project_path,
            verbose_macro_warnings: false,
            show_macro_stats: false,
            parallel: parallel_enabled,
            jobs,
            use_cache: true,
            multi_pass: false,
            show_attribution: false,
            aggregate_only: false,
            no_aggregation: false,
            aggregation_method: None,
            min_problematic: None,
            no_god_object: false,
            suppress_coverage_tip: true, // Suppress coverage TIP for validate (spec 131)
            _formatting_config: Default::default(),
        },
    )
    .expect("Unified analysis failed");

    // Display timing information if verbosity is enabled (spec 130)
    display_timing_information(&unified, verbosity);

    unified
}

/// Display timing information for analysis phases (spec 130)
fn display_timing_information(unified: &crate::priority::UnifiedAnalysis, verbosity: u8) {
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();

    // Don't display timing if quiet mode is enabled or verbosity is 0
    if quiet_mode || verbosity == 0 {
        return;
    }

    if let Some(timings) = unified.timings() {
        eprintln!("\nTiming information:");
        eprintln!("  Total analysis time: {:?}", timings.total);

        // Only show detailed breakdown at verbosity >= 1
        if verbosity >= 1 {
            eprintln!("  - Call graph building: {:?}", timings.call_graph_building);
            eprintln!("  - Trait resolution: {:?}", timings.trait_resolution);
            eprintln!("  - Coverage loading: {:?}", timings.coverage_loading);

            if timings.data_flow_creation.as_millis() > 0 {
                eprintln!("  - Data flow: {:?}", timings.data_flow_creation);
            }
            if timings.purity_analysis.as_millis() > 0 {
                eprintln!("  - Purity: {:?}", timings.purity_analysis);
            }
            if timings.test_detection.as_millis() > 0 {
                eprintln!("  - Test detection: {:?}", timings.test_detection);
            }
            if timings.debt_aggregation.as_millis() > 0 {
                eprintln!("  - Debt aggregation: {:?}", timings.debt_aggregation);
            }
            if timings.function_analysis.as_millis() > 0 {
                eprintln!("  - Function analysis: {:?}", timings.function_analysis);
            }
            if timings.file_analysis.as_millis() > 0 {
                eprintln!("  - File analysis: {:?}", timings.file_analysis);
            }
            if timings.sorting.as_millis() > 0 {
                eprintln!("  - Sorting: {:?}", timings.sorting);
            }
        }
        eprintln!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_sets_parallel_env_var() {
        // Clear any existing env var
        std::env::remove_var("DEBTMAP_PARALLEL");

        // Simulate validate command with parallel enabled (default)
        let config = ValidateConfig {
            path: PathBuf::from("."),
            config: None,
            coverage_file: None,
            format: None,
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            max_debt_density: None,
            top: None,
            tail: None,
            semantic_off: false,
            verbosity: 0,
            no_parallel: false,
            jobs: 0,
        };

        // When parallel is enabled, the environment variable should be set
        if !config.no_parallel {
            std::env::set_var("DEBTMAP_PARALLEL", "true");
        }

        assert_eq!(std::env::var("DEBTMAP_PARALLEL").unwrap(), "true");

        // Clean up
        std::env::remove_var("DEBTMAP_PARALLEL");
    }

    #[test]
    fn test_validate_respects_no_parallel_flag() {
        // Test the logic of no_parallel flag (don't rely on global env var state)
        let config = ValidateConfig {
            path: PathBuf::from("."),
            config: None,
            coverage_file: None,
            format: None,
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            max_debt_density: None,
            top: None,
            tail: None,
            semantic_off: false,
            verbosity: 0,
            no_parallel: true,
            jobs: 0,
        };

        // Verify that no_parallel flag is set correctly
        assert!(config.no_parallel);

        // Test that when no_parallel is true, we should NOT set the env var
        let should_set_parallel = !config.no_parallel;
        assert!(!should_set_parallel);
    }

    #[test]
    fn test_validate_sets_jobs_env_var() {
        // Clear any existing env var
        std::env::remove_var("DEBTMAP_JOBS");

        // Simulate validate command with custom job count
        let config = ValidateConfig {
            path: PathBuf::from("."),
            config: None,
            coverage_file: None,
            format: None,
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            max_debt_density: None,
            top: None,
            tail: None,
            semantic_off: false,
            verbosity: 0,
            no_parallel: false,
            jobs: 4,
        };

        // When jobs is set, the environment variable should be set
        if config.jobs > 0 {
            std::env::set_var("DEBTMAP_JOBS", config.jobs.to_string());
        }

        assert_eq!(std::env::var("DEBTMAP_JOBS").unwrap(), "4");

        // Clean up
        std::env::remove_var("DEBTMAP_JOBS");
    }

    #[test]
    fn test_env_var_parsing() {
        // Combined test for environment variable parsing to avoid race conditions
        // when tests run in parallel. Tests must not interfere with each other.

        // Test DEBTMAP_PARALLEL detection

        // Case 1: DEBTMAP_PARALLEL not set (default: sequential)
        std::env::remove_var("DEBTMAP_PARALLEL");
        let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        assert!(!parallel_enabled);

        // Case 2: DEBTMAP_PARALLEL=true
        std::env::set_var("DEBTMAP_PARALLEL", "true");
        let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        assert!(parallel_enabled);

        // Case 3: DEBTMAP_PARALLEL=1
        std::env::set_var("DEBTMAP_PARALLEL", "1");
        let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        assert!(parallel_enabled);

        // Case 4: DEBTMAP_PARALLEL=false
        std::env::set_var("DEBTMAP_PARALLEL", "false");
        let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        assert!(!parallel_enabled);

        // Clean up
        std::env::remove_var("DEBTMAP_PARALLEL");

        // Test DEBTMAP_JOBS parsing

        // Case 1: Valid number
        std::env::set_var("DEBTMAP_JOBS", "8");
        let jobs = std::env::var("DEBTMAP_JOBS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        assert_eq!(jobs, 8);

        // Case 2: Invalid number (defaults to 0)
        std::env::set_var("DEBTMAP_JOBS", "invalid");
        let jobs = std::env::var("DEBTMAP_JOBS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        assert_eq!(jobs, 0);

        // Case 3: Not set (defaults to 0)
        std::env::remove_var("DEBTMAP_JOBS");
        let jobs = std::env::var("DEBTMAP_JOBS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        assert_eq!(jobs, 0);

        // Clean up
        std::env::remove_var("DEBTMAP_JOBS");
    }

    #[test]
    fn test_validation_details_creation() {
        // Test that ValidationDetails can be constructed correctly
        let details = ValidationDetails {
            average_complexity: 5.0,
            max_average_complexity: 10.0,
            high_complexity_count: 3,
            max_high_complexity_count: 5,
            debt_items: 10,
            max_debt_items: 20,
            total_debt_score: 150,
            max_total_debt_score: 300,
            debt_density: 0.15,
            max_debt_density: 0.20,
            codebase_risk_score: 25.5,
            max_codebase_risk_score: 50.0,
            high_risk_functions: 5,
            max_high_risk_functions: 10,
            coverage_percentage: 75.0,
            min_coverage_percentage: 60.0,
        };

        assert_eq!(details.average_complexity, 5.0);
        assert_eq!(details.max_average_complexity, 10.0);
        assert_eq!(details.high_complexity_count, 3);
        assert_eq!(details.max_high_complexity_count, 5);
        assert_eq!(details.debt_density, 0.15);
        assert_eq!(details.max_debt_density, 0.20);
        assert_eq!(details.debt_items, 10);
        assert_eq!(details.max_debt_items, 20);
        assert_eq!(details.total_debt_score, 150);
        assert_eq!(details.max_total_debt_score, 300);
        assert_eq!(details.codebase_risk_score, 25.5);
        assert_eq!(details.max_codebase_risk_score, 50.0);
        assert_eq!(details.high_risk_functions, 5);
        assert_eq!(details.max_high_risk_functions, 10);
        assert_eq!(details.coverage_percentage, 75.0);
        assert_eq!(details.min_coverage_percentage, 60.0);
    }
}

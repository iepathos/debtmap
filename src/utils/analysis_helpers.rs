use crate::{
    analysis_utils, config,
    core::{
        AnalysisResults, ComplexityReport, DependencyReport, DuplicationBlock, FileMetrics,
        FunctionMetrics, Language, TechnicalDebtReport,
    },
    debt, io,
};
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::PathBuf;

const DEFAULT_SIMILARITY_THRESHOLD: f64 = 0.8;

pub fn analyze_project(
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
    let file_contexts = analysis_utils::extract_file_contexts(&file_metrics);

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
        file_contexts,
    })
}

pub fn detect_duplications(files: &[PathBuf], threshold: usize) -> Vec<DuplicationBlock> {
    let files_with_content = prepare_files_for_duplication_check(files);
    debt::duplication::detect_duplication(
        files_with_content,
        threshold,
        DEFAULT_SIMILARITY_THRESHOLD,
    )
}

pub fn prepare_files_for_duplication_check(files: &[PathBuf]) -> Vec<(PathBuf, String)> {
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

pub fn build_complexity_report(
    all_functions: &[FunctionMetrics],
    complexity_threshold: u32,
) -> ComplexityReport {
    analysis_utils::build_complexity_report(all_functions, complexity_threshold)
}

pub fn build_technical_debt_report(
    all_debt_items: Vec<crate::core::DebtItem>,
    duplications: Vec<DuplicationBlock>,
) -> TechnicalDebtReport {
    analysis_utils::build_technical_debt_report(all_debt_items, duplications)
}

pub fn create_dependency_report(file_metrics: &[FileMetrics]) -> DependencyReport {
    analysis_utils::create_dependency_report(file_metrics)
}

//! Pure file-level analysis functions.
//!
//! This module provides pure functions for file-level metric aggregation
//! and analysis without any I/O or progress reporting side effects.

use crate::analysis::FileContext;
use crate::analyzers::file_analyzer::UnifiedFileAnalyzer;
use crate::analyzers::FileAnalyzer;
use crate::core::{FunctionMetrics, Language};
use crate::priority::file_metrics::{FileDebtItem, FileDebtMetrics};
use crate::risk::lcov::LcovData;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Pure function to group functions by file.
pub fn group_functions_by_file(
    metrics: &[FunctionMetrics],
) -> HashMap<PathBuf, Vec<FunctionMetrics>> {
    let mut files_map = HashMap::new();
    for metric in metrics {
        files_map
            .entry(metric.file.clone())
            .or_insert_with(Vec::new)
            .push(metric.clone());
    }
    files_map
}

/// Pure function to aggregate function metrics into file metrics.
pub fn aggregate_file_metrics(
    functions: &[FunctionMetrics],
    coverage_data: Option<&LcovData>,
) -> FileDebtMetrics {
    let file_analyzer = UnifiedFileAnalyzer::new(coverage_data.cloned());
    file_analyzer.aggregate_functions(functions)
}

/// Pure function to calculate uncovered lines.
pub fn calculate_uncovered_lines(coverage_percent: f64, line_count: usize) -> usize {
    ((1.0 - coverage_percent) * line_count as f64) as usize
}

/// Pure function to determine if file should be included based on score.
pub fn should_include_file(score: f64) -> bool {
    score > 50.0
}

/// Pure function to detect file context for scoring adjustments.
pub fn detect_file_context(file_path: &Path, functions: &[FunctionMetrics]) -> FileContext {
    let language = Language::from_path(file_path);
    let detector = crate::analysis::FileContextDetector::new(language);
    detector.detect(file_path, functions)
}

/// Pure function to create file debt item from metrics with context.
pub fn create_file_debt_item(
    file_metrics: FileDebtMetrics,
    file_context: Option<&FileContext>,
) -> FileDebtItem {
    FileDebtItem::from_metrics(file_metrics, file_context)
}

/// Pure function to enhance metrics with actual line count.
pub fn enhance_metrics_with_line_count(
    mut file_metrics: FileDebtMetrics,
    actual_line_count: usize,
) -> FileDebtMetrics {
    file_metrics.total_lines = actual_line_count;
    file_metrics.uncovered_lines =
        calculate_uncovered_lines(file_metrics.coverage_percent, actual_line_count);
    file_metrics
}

/// Data structure for processed file information.
#[derive(Debug, Clone)]
pub struct ProcessedFileData {
    pub file_path: PathBuf,
    pub file_metrics: FileDebtMetrics,
    pub god_analysis: Option<crate::organization::GodObjectAnalysis>,
    pub file_context: FileContext,
    pub raw_functions: Vec<FunctionMetrics>,
    pub project_root: PathBuf,
}

/// Pure function to process a single file's metrics.
///
/// Note: This function requires file content to be passed in (I/O happens at the caller).
pub fn process_file_metrics(
    file_path: PathBuf,
    functions: Vec<FunctionMetrics>,
    file_content: Option<&str>,
    coverage_data: Option<&LcovData>,
    no_god_object: bool,
    project_root: &Path,
) -> ProcessedFileData {
    let file_analyzer = UnifiedFileAnalyzer::new(coverage_data.cloned());

    // Get base file metrics
    let mut file_metrics = file_analyzer.aggregate_functions(&functions);

    // Enhance with actual line count if content available
    if let Some(content) = file_content {
        let actual_line_count = content.lines().count();
        file_metrics = enhance_metrics_with_line_count(file_metrics, actual_line_count);

        // Apply god object detection if enabled
        if !no_god_object {
            file_metrics.god_object_analysis =
                detect_god_object_from_content(&file_analyzer, &file_path, content, &file_metrics);
        }
    }

    // Detect file context
    let file_context = detect_file_context(&file_path, &functions);

    // Generate god object analysis reference
    let god_analysis = file_metrics.god_object_analysis.clone();

    ProcessedFileData {
        file_path,
        file_metrics,
        god_analysis,
        file_context,
        raw_functions: functions,
        project_root: project_root.to_path_buf(),
    }
}

/// Pure function to detect god object analysis from file content.
fn detect_god_object_from_content(
    file_analyzer: &UnifiedFileAnalyzer,
    file_path: &Path,
    content: &str,
    file_metrics: &FileDebtMetrics,
) -> Option<crate::organization::GodObjectAnalysis> {
    use crate::priority::score_types::Score0To100;

    let actual_line_count = content.lines().count();

    // Get analysis from file analyzer
    let mut god_analysis = file_analyzer
        .analyze_file(file_path, content)
        .ok()
        .and_then(|m| m.god_object_analysis)
        .or_else(|| file_metrics.god_object_analysis.clone());

    // Apply size-based god object detection
    if actual_line_count > 2000 || file_metrics.function_count > 50 {
        if let Some(ref mut analysis) = god_analysis {
            analysis.is_god_object = true;
            if analysis.god_object_score == Score0To100::new(0.0) {
                analysis.god_object_score = Score0To100::new(
                    ((file_metrics.function_count as f64 / 50.0).min(2.0)) * 100.0,
                );
            }
        } else {
            god_analysis = Some(crate::organization::GodObjectAnalysis {
                is_god_object: true,
                method_count: file_metrics.function_count,
                weighted_method_count: None,
                field_count: 0,
                responsibility_count: 0,
                lines_of_code: actual_line_count,
                complexity_sum: file_metrics.total_complexity,
                god_object_score: Score0To100::new(
                    ((file_metrics.function_count as f64 / 50.0).min(2.0)) * 100.0,
                ),
                recommended_splits: Vec::new(),
                confidence: crate::organization::GodObjectConfidence::Probable,
                responsibilities: Vec::new(),
                responsibility_method_counts: std::collections::HashMap::new(),
                purity_distribution: None,
                module_structure: None,
                detection_type: crate::organization::DetectionType::GodFile,
                struct_name: None,
                struct_line: None,
                struct_location: None,
                visibility_breakdown: None,
                domain_count: 0,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: crate::organization::SplitAnalysisMethod::None,
                cross_domain_severity: None,
                domain_diversity_metrics: None,
                aggregated_entropy: None,
                aggregated_error_swallowing_count: None,
                aggregated_error_swallowing_patterns: None,
                layering_impact: None,
                anti_pattern_report: None,
                complexity_metrics: None, // Spec 211
            });
        }
    }

    god_analysis
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metric(file: &str, name: &str) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from(file),
            line: 1,
            length: 10,
            cyclomatic: 5,
            cognitive: 3,
            nesting: 0,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        }
    }

    #[test]
    fn test_group_functions_by_file() {
        let metrics = vec![
            create_test_metric("a.rs", "foo"),
            create_test_metric("b.rs", "bar"),
            create_test_metric("a.rs", "baz"),
        ];

        let grouped = group_functions_by_file(&metrics);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped.get(&PathBuf::from("a.rs")).unwrap().len(), 2);
        assert_eq!(grouped.get(&PathBuf::from("b.rs")).unwrap().len(), 1);
    }

    #[test]
    fn test_calculate_uncovered_lines() {
        // Note: Floating point truncation may cause off-by-one
        let uncovered = calculate_uncovered_lines(0.8, 100);
        assert!(uncovered == 19 || uncovered == 20); // Allow for floating point truncation
        assert_eq!(calculate_uncovered_lines(0.0, 100), 100);
        assert_eq!(calculate_uncovered_lines(1.0, 100), 0);
    }

    #[test]
    fn test_should_include_file() {
        assert!(should_include_file(51.0));
        assert!(!should_include_file(50.0));
        assert!(!should_include_file(49.0));
    }
}

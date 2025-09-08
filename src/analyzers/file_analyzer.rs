use crate::analyzers::FileAnalyzer;
use crate::core::FunctionMetrics;
use crate::priority::file_metrics::{FileDebtMetrics, GodObjectIndicators};
use crate::risk::lcov::LcovData;
use anyhow::Result;
use std::path::Path;

pub struct UnifiedFileAnalyzer {
    coverage_data: Option<LcovData>,
}

impl UnifiedFileAnalyzer {
    pub fn new(coverage_data: Option<LcovData>) -> Self {
        Self { coverage_data }
    }

    fn count_lines(content: &str) -> usize {
        content.lines().count()
    }

    fn analyze_god_object(&self, _path: &Path, content: &str) -> GodObjectIndicators {
        // For now, we'll use simple heuristics based on file analysis
        // This will be enhanced when spec 100 is implemented
        let lines = Self::count_lines(content);
        let function_count = content.matches("fn ").count() + content.matches("def ").count();
        let field_count = content.matches("pub ").count() + content.matches("self.").count() / 3;

        let is_god_object = function_count > 50 || lines > 2000 || field_count > 30;
        let god_object_score = if is_god_object {
            ((function_count as f64 / 50.0) + (lines as f64 / 2000.0) + (field_count as f64 / 30.0))
                / 3.0
        } else {
            0.0
        };

        GodObjectIndicators {
            methods_count: function_count,
            fields_count: field_count,
            responsibilities: if is_god_object { 5 } else { 1 },
            is_god_object,
            god_object_score: god_object_score.min(1.0),
        }
    }

    fn get_file_coverage(&self, path: &Path) -> f64 {
        if let Some(ref coverage) = self.coverage_data {
            coverage.get_file_coverage(path).unwrap_or(0.0) / 100.0
        } else {
            0.0
        }
    }
}

impl FileAnalyzer for UnifiedFileAnalyzer {
    fn analyze_file(&self, path: &Path, content: &str) -> Result<FileDebtMetrics> {
        let total_lines = Self::count_lines(content);
        let god_object_indicators = self.analyze_god_object(path, content);
        let coverage_percent = self.get_file_coverage(path);
        let uncovered_lines = ((1.0 - coverage_percent) * total_lines as f64) as usize;

        Ok(FileDebtMetrics {
            path: path.to_path_buf(),
            total_lines,
            function_count: 0, // Will be filled by aggregate_functions
            class_count: 0,    // Will be filled by aggregate_functions
            avg_complexity: 0.0,
            max_complexity: 0,
            total_complexity: 0,
            coverage_percent,
            uncovered_lines,
            god_object_indicators,
            function_scores: Vec::new(),
        })
    }

    fn aggregate_functions(&self, functions: &[FunctionMetrics]) -> FileDebtMetrics {
        if functions.is_empty() {
            return FileDebtMetrics::default();
        }

        let path = functions[0].file.clone();
        let function_count = functions.len();

        // Calculate complexity metrics
        let total_complexity: u32 = functions.iter().map(|f| f.cyclomatic).sum();
        let max_complexity = functions.iter().map(|f| f.cyclomatic).max().unwrap_or(0);
        let avg_complexity = if function_count > 0 {
            total_complexity as f64 / function_count as f64
        } else {
            0.0
        };

        // Extract function scores (would be calculated by unified scorer)
        let function_scores: Vec<f64> = functions.iter().map(|_| 0.0).collect();

        // Count classes (simple heuristic for now)
        let class_count = functions
            .iter()
            .filter(|f| f.name.contains("::new") || f.name.contains("__init__"))
            .count();

        // Get coverage
        let coverage_percent = if let Some(ref coverage) = self.coverage_data {
            let covered_functions = functions
                .iter()
                .filter(|f| {
                    coverage
                        .get_function_coverage(&f.file, &f.name)
                        .map(|c| c > 0.0)
                        .unwrap_or(false)
                })
                .count();
            covered_functions as f64 / function_count as f64
        } else {
            0.0
        };

        // Estimate total lines (sum of function lengths plus overhead)
        let total_lines: usize =
            functions.iter().map(|f| f.length).sum::<usize>() + function_count * 5;
        let uncovered_lines = ((1.0 - coverage_percent) * total_lines as f64) as usize;

        // Detect god object based on aggregated metrics
        let is_god_object = function_count > 50 || total_lines > 2000;
        let god_object_score = if is_god_object {
            (function_count as f64 / 50.0).min(2.0)
        } else {
            0.0
        };

        FileDebtMetrics {
            path,
            total_lines,
            function_count,
            class_count,
            avg_complexity,
            max_complexity,
            total_complexity,
            coverage_percent,
            uncovered_lines,
            god_object_indicators: GodObjectIndicators {
                methods_count: function_count,
                fields_count: class_count * 5, // Rough estimate
                responsibilities: if is_god_object { 5 } else { 2 },
                is_god_object,
                god_object_score,
            },
            function_scores,
        }
    }
}

pub fn analyze_file_with_metrics(
    path: &Path,
    content: &str,
    functions: &[FunctionMetrics],
    coverage: Option<&LcovData>,
) -> Result<FileDebtMetrics> {
    let analyzer = UnifiedFileAnalyzer::new(coverage.cloned());

    // Get base file metrics
    let mut file_metrics = analyzer.analyze_file(path, content)?;

    // Aggregate function data
    let aggregated = analyzer.aggregate_functions(functions);

    // Merge the results
    file_metrics.function_count = aggregated.function_count;
    file_metrics.class_count = aggregated.class_count;
    file_metrics.avg_complexity = aggregated.avg_complexity;
    file_metrics.max_complexity = aggregated.max_complexity;
    file_metrics.total_complexity = aggregated.total_complexity;
    file_metrics.function_scores = aggregated.function_scores;

    // Override god object detection with aggregated data if more accurate
    if aggregated.function_count > 0 {
        file_metrics.god_object_indicators = aggregated.god_object_indicators;
    }

    Ok(file_metrics)
}

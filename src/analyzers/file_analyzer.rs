use crate::analyzers::FileAnalyzer;
use crate::core::FunctionMetrics;
use crate::organization::GodObjectDetector;
use crate::priority::file_metrics::{FileDebtMetrics, GodObjectIndicators, ModuleSplit};
use crate::risk::lcov::LcovData;
use anyhow::Result;
use std::path::Path;

/// Helper struct for complexity calculation results
struct ComplexityMetrics {
    total_complexity: u32,
    max_complexity: u32,
    avg_complexity: f64,
}

/// Helper struct for coverage calculation results
struct CoverageMetrics {
    coverage_percent: f64,
}

/// Helper struct for line calculation results
struct LineMetrics {
    total_lines: usize,
    uncovered_lines: usize,
}

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

    fn analyze_god_object(&self, path: &Path, content: &str) -> GodObjectIndicators {
        // Use the comprehensive god object detector for Rust files
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(ast) = syn::parse_file(content) {
                let detector = GodObjectDetector::with_source_content(content);
                let analysis = detector.analyze_comprehensive(path, &ast);

                // Convert recommended splits to our format
                let recommended_splits: Vec<ModuleSplit> = analysis
                    .recommended_splits
                    .iter()
                    .map(|split| ModuleSplit {
                        suggested_name: split.suggested_name.clone(),
                        methods_to_move: split.methods_to_move.clone(),
                        structs_to_move: split.structs_to_move.clone(),
                        responsibility: split.responsibility.clone(),
                        estimated_lines: split.estimated_lines,
                        method_count: split.method_count,
                        warning: split.warning.clone(),
                        priority: split.priority.into(),
                    })
                    .collect();

                return GodObjectIndicators {
                    methods_count: analysis.method_count,
                    fields_count: analysis.field_count,
                    responsibilities: analysis.responsibility_count,
                    is_god_object: analysis.is_god_object,
                    god_object_score: analysis.god_object_score.min(100.0) / 100.0, // Normalize to 0-1
                    responsibility_names: analysis.responsibilities.clone(),
                    recommended_splits,
                    module_structure: analysis.module_structure.clone(),
                };
            }
        }

        // Fallback to simple heuristics for non-Rust files
        let lines = Self::count_lines(content);
        let function_count = content.matches("fn ").count()
            + content.matches("def ").count()
            + content.matches("function ").count();
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
            responsibility_names: Vec::new(),
            recommended_splits: Vec::new(),
            module_structure: None,
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

impl UnifiedFileAnalyzer {
    /// Calculate complexity-related metrics for functions
    fn calculate_complexity_metrics(functions: &[FunctionMetrics]) -> ComplexityMetrics {
        let total_complexity: u32 = functions.iter().map(|f| f.cyclomatic).sum();
        let max_complexity = functions.iter().map(|f| f.cyclomatic).max().unwrap_or(0);
        let function_count = functions.len();
        let avg_complexity = if function_count > 0 {
            total_complexity as f64 / function_count as f64
        } else {
            0.0
        };

        ComplexityMetrics {
            total_complexity,
            max_complexity,
            avg_complexity,
        }
    }

    /// Calculate individual function scores based on complexity
    /// This provides a basic score based solely on function metrics
    fn calculate_function_scores(functions: &[FunctionMetrics]) -> Vec<f64> {
        functions
            .iter()
            .map(|func| {
                // Calculate a basic score based on complexity (0-10 scale)
                let complexity_score = (func.cyclomatic + func.cognitive) as f64 / 2.0;
                let length_penalty = if func.length > 50 { 2.0 } else { 1.0 };
                let nesting_penalty = if func.nesting > 3 { 1.5 } else { 1.0 };

                // Basic scoring: complexity * length_penalty * nesting_penalty
                // Clamped to 0-10 range
                (complexity_score * length_penalty * nesting_penalty).min(10.0)
            })
            .collect()
    }

    /// Estimate class count using simple heuristics
    fn estimate_class_count(functions: &[FunctionMetrics]) -> usize {
        functions
            .iter()
            .filter(|f| f.name.contains("::new") || f.name.contains("__init__"))
            .count()
    }

    /// Calculate coverage-related metrics
    fn calculate_coverage_metrics(
        &self,
        functions: &[FunctionMetrics],
        function_count: usize,
    ) -> CoverageMetrics {
        let coverage_percent = match &self.coverage_data {
            Some(coverage) => {
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
            }
            None => 0.0,
        };

        CoverageMetrics { coverage_percent }
    }

    /// Calculate line-related metrics including uncovered lines
    fn calculate_line_metrics(
        functions: &[FunctionMetrics],
        function_count: usize,
        coverage_percent: f64,
    ) -> LineMetrics {
        const OVERHEAD_LINES_PER_FUNCTION: usize = 5;

        let total_lines: usize = functions.iter().map(|f| f.length).sum::<usize>()
            + function_count * OVERHEAD_LINES_PER_FUNCTION;
        let uncovered_lines = ((1.0 - coverage_percent) * total_lines as f64) as usize;

        LineMetrics {
            total_lines,
            uncovered_lines,
        }
    }

    /// Detect god object patterns and calculate indicators
    fn detect_god_object(function_count: usize, total_lines: usize) -> GodObjectIndicators {
        const MAX_FUNCTIONS_THRESHOLD: usize = 50;
        const MAX_LINES_THRESHOLD: usize = 2000;
        const ESTIMATED_FIELDS_PER_CLASS: usize = 5;

        let is_god_object =
            function_count > MAX_FUNCTIONS_THRESHOLD || total_lines > MAX_LINES_THRESHOLD;
        let god_object_score = if is_god_object {
            (function_count as f64 / MAX_FUNCTIONS_THRESHOLD as f64).min(2.0)
        } else {
            0.0
        };

        GodObjectIndicators {
            methods_count: function_count,
            fields_count: function_count * ESTIMATED_FIELDS_PER_CLASS
                / MAX_FUNCTIONS_THRESHOLD.max(1), // Rough estimate
            responsibilities: if is_god_object { 5 } else { 2 },
            is_god_object,
            god_object_score,
            responsibility_names: Vec::new(),
            recommended_splits: Vec::new(),
            module_structure: None,
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
        let complexity_metrics = Self::calculate_complexity_metrics(functions);
        let class_count = Self::estimate_class_count(functions);
        let coverage_metrics = self.calculate_coverage_metrics(functions, function_count);
        let line_metrics = Self::calculate_line_metrics(
            functions,
            function_count,
            coverage_metrics.coverage_percent,
        );
        let god_object_indicators =
            Self::detect_god_object(function_count, line_metrics.total_lines);

        // Calculate individual function scores based on complexity
        let function_scores = Self::calculate_function_scores(functions);

        FileDebtMetrics {
            path,
            total_lines: line_metrics.total_lines,
            function_count,
            class_count,
            avg_complexity: complexity_metrics.avg_complexity,
            max_complexity: complexity_metrics.max_complexity,
            total_complexity: complexity_metrics.total_complexity,
            coverage_percent: coverage_metrics.coverage_percent,
            uncovered_lines: line_metrics.uncovered_lines,
            god_object_indicators,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FunctionMetrics;
    use std::path::PathBuf;

    fn create_test_function_metrics(name: &str, cyclomatic: u32, length: usize) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic,
            cognitive: cyclomatic, // Approximate cognitive complexity
            nesting: 1,
            length,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
        }
    }

    #[test]
    fn test_calculate_complexity_metrics() {
        let functions = vec![
            create_test_function_metrics("func1", 5, 20),
            create_test_function_metrics("func2", 10, 30),
            create_test_function_metrics("func3", 3, 15),
        ];

        let metrics = UnifiedFileAnalyzer::calculate_complexity_metrics(&functions);

        assert_eq!(metrics.total_complexity, 18);
        assert_eq!(metrics.max_complexity, 10);
        assert_eq!(metrics.avg_complexity, 6.0);
    }

    #[test]
    fn test_estimate_class_count() {
        let functions = vec![
            create_test_function_metrics("MyClass::new", 1, 10),
            create_test_function_metrics("AnotherClass::new", 1, 10),
            create_test_function_metrics("__init__", 1, 10),
            create_test_function_metrics("regular_function", 1, 10),
            create_test_function_metrics("another_regular", 1, 10),
        ];

        let class_count = UnifiedFileAnalyzer::estimate_class_count(&functions);
        assert_eq!(class_count, 3); // Two ::new and one __init__
    }

    #[test]
    fn test_calculate_coverage_metrics_with_data() {
        use crate::risk::lcov::LcovData;
        use std::collections::HashMap;

        use crate::risk::lcov::FunctionCoverage;

        let mut functions = HashMap::new();
        let function_coverages = vec![
            FunctionCoverage {
                name: "func1".to_string(),
                start_line: 1,
                execution_count: 10,
                coverage_percentage: 80.0,
                uncovered_lines: vec![2, 3],
            },
            FunctionCoverage {
                name: "func2".to_string(),
                start_line: 10,
                execution_count: 0,
                coverage_percentage: 0.0,
                uncovered_lines: vec![10, 11, 12, 13, 14],
            },
            FunctionCoverage {
                name: "func3".to_string(),
                start_line: 20,
                execution_count: 5,
                coverage_percentage: 50.0,
                uncovered_lines: vec![21, 22],
            },
        ];
        functions.insert(PathBuf::from("test.rs"), function_coverages);

        let mut coverage_data = LcovData::default();
        coverage_data.functions = functions;
        coverage_data.total_lines = 100;
        coverage_data.lines_hit = 50;
        coverage_data.build_index(); // Rebuild index after modifying functions

        let analyzer = UnifiedFileAnalyzer::new(Some(coverage_data));
        let functions = vec![
            create_test_function_metrics("func1", 1, 10),
            create_test_function_metrics("func2", 1, 10),
            create_test_function_metrics("func3", 1, 10),
        ];

        let metrics = analyzer.calculate_coverage_metrics(&functions, 3);
        assert_eq!(metrics.coverage_percent, 2.0 / 3.0); // 2 out of 3 have coverage > 0
    }

    #[test]
    fn test_calculate_coverage_metrics_without_data() {
        let analyzer = UnifiedFileAnalyzer::new(None);
        let functions = vec![
            create_test_function_metrics("func1", 1, 10),
            create_test_function_metrics("func2", 1, 10),
        ];

        let metrics = analyzer.calculate_coverage_metrics(&functions, 2);
        assert_eq!(metrics.coverage_percent, 0.0);
    }

    #[test]
    fn test_calculate_line_metrics() {
        let functions = vec![
            create_test_function_metrics("func1", 1, 20),
            create_test_function_metrics("func2", 1, 30),
            create_test_function_metrics("func3", 1, 10),
        ];

        let metrics = UnifiedFileAnalyzer::calculate_line_metrics(&functions, 3, 0.6);

        // Total lines = 20 + 30 + 10 + 3*5 (overhead) = 75
        assert_eq!(metrics.total_lines, 75);
        // Uncovered lines = (1.0 - 0.6) * 75 = 30
        assert_eq!(metrics.uncovered_lines, 30);
    }

    #[test]
    fn test_detect_god_object() {
        // Test non-god object
        let normal_indicators = UnifiedFileAnalyzer::detect_god_object(20, 500);
        assert!(!normal_indicators.is_god_object);
        assert_eq!(normal_indicators.god_object_score, 0.0);
        assert_eq!(normal_indicators.methods_count, 20);

        // Test god object by function count
        let function_god = UnifiedFileAnalyzer::detect_god_object(60, 500);
        assert!(function_god.is_god_object);
        assert_eq!(function_god.god_object_score, 1.2); // 60/50 = 1.2

        // Test god object by line count
        let line_god = UnifiedFileAnalyzer::detect_god_object(30, 2500);
        assert!(line_god.is_god_object);
        assert_eq!(line_god.methods_count, 30);

        // Test capped god object score
        let extreme_god = UnifiedFileAnalyzer::detect_god_object(150, 5000);
        assert!(extreme_god.is_god_object);
        assert_eq!(extreme_god.god_object_score, 2.0); // Capped at 2.0
    }

    #[test]
    fn test_aggregate_functions_integration() {
        let analyzer = UnifiedFileAnalyzer::new(None);
        let functions = vec![
            create_test_function_metrics("MyClass::new", 3, 15),
            create_test_function_metrics("regular_func", 8, 25),
            create_test_function_metrics("another_func", 4, 20),
        ];

        let result = analyzer.aggregate_functions(&functions);

        assert_eq!(result.function_count, 3);
        assert_eq!(result.class_count, 1); // One ::new function
        assert_eq!(result.total_complexity, 15); // 3 + 8 + 4
        assert_eq!(result.max_complexity, 8);
        assert_eq!(result.avg_complexity, 5.0); // 15/3
        assert_eq!(result.total_lines, 75); // 15+25+20 + 3*5 overhead
        assert_eq!(result.coverage_percent, 0.0);
        assert_eq!(result.uncovered_lines, 75); // All uncovered
        assert!(!result.god_object_indicators.is_god_object);
        assert_eq!(result.function_scores.len(), 3);

        // Verify function scores are not all zeros
        assert!(
            result.function_scores.iter().any(|&score| score > 0.0),
            "Function scores should not all be zero"
        );
    }

    #[test]
    fn test_function_scores_calculation() {
        let functions = vec![
            create_test_function_metrics("simple_func", 2, 10), // Low complexity, short
            create_test_function_metrics("complex_func", 10, 100), // High complexity, long
            create_test_function_metrics("nested_func", 5, 30), // Medium complexity, medium length
        ];

        let scores = UnifiedFileAnalyzer::calculate_function_scores(&functions);

        assert_eq!(scores.len(), 3);

        // Simple function should have lower score
        assert!(scores[0] > 0.0);
        assert!(scores[0] < 5.0);

        // Complex function should have highest score (gets length penalty)
        assert!(scores[1] > scores[0]);
        assert!(scores[1] > scores[2]);

        // All scores should be in 0-10 range
        for score in scores {
            assert!(score >= 0.0);
            assert!(score <= 10.0);
        }
    }
}

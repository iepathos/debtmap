use crate::analyzers::FileAnalyzer;
use crate::analyzers::typescript::parser::{detect_variant, parse_source};
use crate::analyzers::typescript::visitor::class_analysis::extract_classes;
use crate::analyzers::typescript::visitor::function_analysis::extract_functions;
use crate::core::FunctionMetrics;
use crate::extraction::{ExtractedFileData, UnifiedFileExtractor};
use crate::organization::god_object::classifier::group_methods_by_responsibility;
use crate::organization::god_object::heuristics::{
    HEURISTIC_MAX_FUNCTIONS, HEURISTIC_MAX_LINES, detect_from_content,
    fallback_god_object_heuristics,
};
use crate::organization::god_object::scoring::calculate_god_object_score_weighted;
use crate::organization::god_object::{
    DetectionType, FunctionVisibilityBreakdown, GodObjectAnalysis, GodObjectConfidence,
    GodObjectThresholds, ModuleSplit, Priority, SplitAnalysisMethod,
};
use crate::priority::file_metrics::FileDebtMetrics;
use crate::risk::lcov::LcovData;
use anyhow::Result;
use std::collections::HashMap;
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

fn is_javascript_like(path: &Path) -> bool {
    crate::core::Language::from_path(path).is_js_ts()
}

fn uses_extracted_god_object_analysis(path: &Path) -> bool {
    matches!(
        crate::core::Language::from_path(path),
        crate::core::Language::Rust | crate::core::Language::Python
    )
}

fn semantic_responsibility_groups(function_names: &[String]) -> HashMap<String, Vec<String>> {
    group_methods_by_responsibility(function_names)
        .into_iter()
        .filter(|(responsibility, methods)| responsibility != "unclassified" && !methods.is_empty())
        .collect()
}

fn build_semantic_splits(groups: &HashMap<String, Vec<String>>) -> Vec<ModuleSplit> {
    let mut splits: Vec<ModuleSplit> = groups
        .iter()
        .filter(|(_, methods)| methods.len() >= 2)
        .map(|(responsibility, methods)| ModuleSplit {
            suggested_name: format!("{}_module", responsibility.to_lowercase().replace(' ', "_")),
            methods_to_move: methods.clone(),
            structs_to_move: Vec::new(),
            responsibility: responsibility.clone(),
            estimated_lines: 0,
            method_count: methods.len(),
            warning: None,
            priority: if methods.len() > 10 {
                Priority::High
            } else {
                Priority::Medium
            },
            cohesion_score: Some(0.85),
            dependencies_in: Vec::new(),
            dependencies_out: Vec::new(),
            domain: responsibility.clone(),
            rationale: Some("Parser-derived function names share this responsibility".to_string()),
            method: SplitAnalysisMethod::MethodBased,
            severity: None,
            interface_estimate: None,
            classification_evidence: None,
            representative_methods: methods.iter().take(8).cloned().collect(),
            fields_needed: Vec::new(),
            trait_suggestion: None,
            behavior_category: Some(responsibility.clone()),
            core_type: None,
            data_flow: Vec::new(),
            suggested_type_definition: None,
            data_flow_stage: None,
            pipeline_position: None,
            input_types: Vec::new(),
            output_types: Vec::new(),
            merge_history: Vec::new(),
            alternative_names: Vec::new(),
            naming_confidence: None,
            naming_strategy: None,
            cluster_quality: None,
        })
        .collect();

    splits.sort_by(|a, b| {
        b.method_count
            .cmp(&a.method_count)
            .then_with(|| a.responsibility.cmp(&b.responsibility))
    });
    splits
}

fn build_js_visibility_breakdown(
    functions: &[crate::analyzers::typescript::types::JsFunctionMetrics],
) -> FunctionVisibilityBreakdown {
    functions
        .iter()
        .fold(FunctionVisibilityBreakdown::new(), |mut breakdown, func| {
            if func.is_exported {
                breakdown.public += 1;
            } else {
                breakdown.private += 1;
            }
            breakdown
        })
}

fn analyze_js_ts_god_object(path: &Path, content: &str) -> Option<GodObjectAnalysis> {
    if !is_javascript_like(path) {
        return None;
    }

    let language = crate::core::Language::from_path(path);
    let ast = parse_source(content, path, detect_variant(path)).ok()?;
    let functions = extract_functions(&ast, true);
    let production_functions: Vec<_> = functions.iter().filter(|func| !func.is_test).collect();
    let function_count = production_functions.len();
    let total_lines = content.lines().count();

    if function_count <= HEURISTIC_MAX_FUNCTIONS && total_lines <= HEURISTIC_MAX_LINES {
        return None;
    }

    let function_names: Vec<String> = production_functions
        .iter()
        .filter(|func| !func.name.is_empty())
        .map(|func| func.name.clone())
        .collect();
    let responsibility_groups = semantic_responsibility_groups(&function_names);
    let responsibility_method_counts: HashMap<String, usize> = responsibility_groups
        .iter()
        .map(|(name, methods)| (name.clone(), methods.len()))
        .collect();
    let mut responsibilities: Vec<String> = responsibility_groups.keys().cloned().collect();
    responsibilities.sort();

    let complexity_sum: u32 = production_functions
        .iter()
        .map(|func| func.cyclomatic)
        .sum();
    let avg_complexity = if function_count == 0 {
        0.0
    } else {
        complexity_sum as f64 / function_count as f64
    };
    let class_count = extract_classes(&ast).len();
    let thresholds = GodObjectThresholds::default();
    let responsibility_count = responsibilities.len();
    let god_object_score = calculate_god_object_score_weighted(
        function_count as f64,
        0,
        responsibility_count,
        total_lines,
        avg_complexity,
        &thresholds,
    );
    let confidence = crate::organization::god_object::classifier::determine_confidence(
        function_count,
        0,
        responsibility_count,
        total_lines,
        complexity_sum,
        &thresholds,
    );

    Some(GodObjectAnalysis {
        is_god_object: true,
        method_count: function_count,
        weighted_method_count: None,
        field_count: 0,
        responsibility_count,
        lines_of_code: total_lines,
        complexity_sum,
        god_object_score: god_object_score.max(0.0),
        recommended_splits: build_semantic_splits(&responsibility_groups),
        confidence: if confidence == GodObjectConfidence::NotGodObject {
            GodObjectConfidence::Possible
        } else {
            confidence
        },
        responsibilities,
        responsibility_method_counts,
        purity_distribution: None,
        module_structure: Some(match language {
            crate::core::Language::TypeScript => {
                crate::analysis::ModuleStructureAnalyzer::new_typescript()
                    .analyze_typescript_file(content, path)
            }
            _ => crate::analysis::ModuleStructureAnalyzer::new_javascript()
                .analyze_javascript_file(content, path),
        }),
        detection_type: if class_count == 0 {
            DetectionType::GodFile
        } else {
            DetectionType::GodModule
        },
        struct_name: None,
        struct_line: None,
        struct_location: None,
        visibility_breakdown: Some(build_js_visibility_breakdown(&functions)),
        domain_count: responsibility_count,
        domain_diversity: if function_count == 0 {
            0.0
        } else {
            responsibility_count as f64 / function_count as f64
        },
        struct_ratio: if function_count == 0 {
            0.0
        } else {
            class_count as f64 / function_count as f64
        },
        analysis_method: if responsibility_count == 0 {
            SplitAnalysisMethod::None
        } else {
            SplitAnalysisMethod::MethodBased
        },
        cross_domain_severity: None,
        domain_diversity_metrics: None,
        aggregated_entropy: None,
        aggregated_error_swallowing_count: None,
        aggregated_error_swallowing_patterns: None,
        layering_impact: None,
        anti_pattern_report: None,
        complexity_metrics: None,
        trait_method_summary: None,
    })
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

    /// Analyze god object using pre-extracted data (spec 202).
    ///
    /// Uses `ExtractedFileData` to avoid redundant parsing.
    fn analyze_god_object_from_extracted(
        &self,
        path: &Path,
        content: &str,
        extracted: &ExtractedFileData,
    ) -> (
        Option<crate::organization::GodObjectAnalysis>,
        Option<crate::organization::GodObjectType>,
    ) {
        let language = crate::core::Language::from_path(path);

        if language == crate::core::Language::Python {
            let analysis = self.analyze_god_object_from_data(path, content, extracted);
            if analysis.0.is_some() {
                return analysis;
            }
        }

        // Use the comprehensive god object detector for Rust files
        // The detector needs the AST, but we can check if extracted data indicates
        // a potential god object first to avoid unnecessary re-parsing for simple files
        if language == crate::core::Language::Rust {
            // Quick heuristic check using extracted data
            let method_count: usize = extracted.impls.iter().map(|i| i.methods.len()).sum();
            let field_count: usize = extracted.structs.iter().map(|s| s.fields.len()).sum();
            let total_lines = extracted.total_lines;

            // Only do full analysis if there's potential for god object
            // This avoids re-parsing simple files
            if method_count > 10 || field_count > 8 || total_lines > 500 {
                // For comprehensive god object analysis, we need the AST
                // Use UnifiedFileExtractor to get a fresh parse with SourceMap reset
                if let Ok(data) = UnifiedFileExtractor::extract(path, content) {
                    // Create analysis from extracted data
                    return self.analyze_god_object_from_data(path, content, &data);
                }
            }
        }

        // Fallback to simple heuristics for non-Rust or simple files
        self.analyze_god_object_simple(content)
    }

    /// Simple heuristics-based god object detection for non-Rust files or fallback.
    ///
    /// Uses shared heuristics from `organization::god_object::heuristics` (Spec 212).
    fn analyze_god_object_simple(
        &self,
        content: &str,
    ) -> (
        Option<crate::organization::GodObjectAnalysis>,
        Option<crate::organization::GodObjectType>,
    ) {
        // Delegate to shared heuristics module (Spec 212 consolidation)
        (detect_from_content(content), None)
    }

    /// Analyze god object from extracted data.
    ///
    /// Spec 212: Uses the extraction adapter as the single source of truth
    /// for god object detection.
    fn analyze_god_object_from_data(
        &self,
        path: &Path,
        _content: &str,
        extracted: &ExtractedFileData,
    ) -> (
        Option<crate::organization::GodObjectAnalysis>,
        Option<crate::organization::GodObjectType>,
    ) {
        // Use extraction adapter as single source of truth (Spec 212)
        let analysis = crate::extraction::adapters::god_object::analyze_god_object(path, extracted);

        // Note: GodObjectType classification is not available from the adapter
        // since it was tightly coupled to the AST-based analysis.
        // For most use cases, the GodObjectAnalysis.detection_type field provides
        // sufficient classification information.
        (analysis, None)
    }

    fn analyze_god_object(
        &self,
        path: &Path,
        content: &str,
    ) -> (
        Option<crate::organization::GodObjectAnalysis>,
        Option<crate::organization::GodObjectType>,
    ) {
        // Use extracted data for languages with a unified extractor.
        if uses_extracted_god_object_analysis(path) {
            // Try to use extracted data first (spec 202)
            if let Ok(extracted) = UnifiedFileExtractor::extract(path, content) {
                return self.analyze_god_object_from_extracted(path, content, &extracted);
            }
        }

        if let Some(analysis) = analyze_js_ts_god_object(path, content) {
            return (Some(analysis), None);
        }

        // Fallback to simple heuristics for non-Rust files
        self.analyze_god_object_simple(content)
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

    /// Detect god object patterns and calculate indicators.
    ///
    /// Uses shared heuristics from `organization::god_object::heuristics` (Spec 212).
    fn detect_god_object(
        function_count: usize,
        total_lines: usize,
    ) -> Option<crate::organization::GodObjectAnalysis> {
        // Delegate to shared heuristics module (Spec 212 consolidation)
        // Field count estimated as 0 when not available from AST
        fallback_god_object_heuristics(function_count, total_lines, 0, 0)
    }
}

impl FileAnalyzer for UnifiedFileAnalyzer {
    fn analyze_file(&self, path: &Path, content: &str) -> Result<FileDebtMetrics> {
        let total_lines = Self::count_lines(content);
        let (god_object_analysis, god_object_type) = self.analyze_god_object(path, content);
        let coverage_percent = self.get_file_coverage(path);
        let uncovered_lines = ((1.0 - coverage_percent) * total_lines as f64) as usize;

        // Classify file type for context-aware thresholds (spec 135)
        let file_type = Some(crate::organization::classify_file(content, path));

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
            god_object_analysis,
            function_scores: Vec::new(),
            god_object_type,
            file_type,
            // Spec 201: File-level dependency metrics (populated during analysis aggregation)
            afferent_coupling: 0,
            efferent_coupling: 0,
            instability: 0.0,
            dependents: Vec::new(),
            dependencies_list: Vec::new(),
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

        // BUG FIX: Read file content to properly detect boilerplate patterns
        // This was missing - we need to analyze the actual file content to detect
        // boilerplate (like ripgrep's flags/defs.rs trait implementations)
        let (god_object_analysis, god_object_type) =
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.analyze_god_object(&path, &content)
            } else {
                // Fallback to simple heuristics if we can't read the file
                let fallback_analysis =
                    Self::detect_god_object(function_count, line_metrics.total_lines);
                (fallback_analysis, None)
            };

        // Calculate individual function scores based on complexity
        let function_scores = Self::calculate_function_scores(functions);

        // Classify file type for context-aware thresholds (spec 135)
        let file_type = if let Ok(content) = std::fs::read_to_string(&path) {
            Some(crate::organization::classify_file(&content, &path))
        } else {
            None
        };

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
            god_object_analysis,
            function_scores,
            god_object_type,
            file_type,
            // Spec 201: File-level dependency metrics (populated during analysis aggregation)
            afferent_coupling: 0,
            efferent_coupling: 0,
            instability: 0.0,
            dependents: Vec::new(),
            dependencies_list: Vec::new(),
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
        file_metrics.god_object_analysis = aggregated.god_object_analysis;
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
            entropy_analysis: None,
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
                normalized: crate::risk::lcov::NormalizedFunctionName::simple("func1"),
            },
            FunctionCoverage {
                name: "func2".to_string(),
                start_line: 10,
                execution_count: 0,
                coverage_percentage: 0.0,
                uncovered_lines: vec![10, 11, 12, 13, 14],
                normalized: crate::risk::lcov::NormalizedFunctionName::simple("func2"),
            },
            FunctionCoverage {
                name: "func3".to_string(),
                start_line: 20,
                execution_count: 5,
                coverage_percentage: 50.0,
                uncovered_lines: vec![21, 22],
                normalized: crate::risk::lcov::NormalizedFunctionName::simple("func3"),
            },
        ];
        functions.insert(PathBuf::from("test.rs"), function_coverages);

        let mut coverage_data = LcovData {
            functions,
            total_lines: 100,
            lines_hit: 50,
            ..Default::default()
        };
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
        let normal_analysis = UnifiedFileAnalyzer::detect_god_object(20, 500);
        assert!(normal_analysis.is_none());

        // Test god object by function count
        let function_god = UnifiedFileAnalyzer::detect_god_object(60, 500);
        assert!(function_god.is_some());
        let function_god = function_god.unwrap();
        assert!(function_god.is_god_object);
        // Spec 212: Uses weighted scoring algorithm now
        assert!(function_god.god_object_score > 0.0);
        assert_eq!(function_god.method_count, 60);

        // Test god object by line count
        let line_god = UnifiedFileAnalyzer::detect_god_object(30, 2500);
        assert!(line_god.is_some());
        let line_god = line_god.unwrap();
        assert!(line_god.is_god_object);
        assert_eq!(line_god.method_count, 30);

        // Test extreme case - should produce a high score
        let extreme_god = UnifiedFileAnalyzer::detect_god_object(150, 5000);
        assert!(extreme_god.is_some());
        let extreme_god = extreme_god.unwrap();
        assert!(extreme_god.is_god_object);
        // Spec 212: Weighted scoring produces higher scores for severe violations
        assert!(extreme_god.god_object_score > function_god.god_object_score);
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
        assert!(result.god_object_analysis.is_none()); // Not a god object
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

    #[test]
    fn test_js_god_object_uses_semantic_responsibilities() {
        let mut content = String::new();
        for i in 0..20 {
            content.push_str(&format!("function parseThing{}() {{ return {}; }}\n", i, i));
        }
        for i in 0..20 {
            content.push_str(&format!(
                "function validateThing{}() {{ return true; }}\n",
                i
            ));
        }
        for i in 0..12 {
            content.push_str(&format!(
                "function renderThing{}() {{ return String({}); }}\n",
                i, i
            ));
        }

        let analyzer = UnifiedFileAnalyzer::new(None);
        let metrics = analyzer
            .analyze_file(Path::new("generator.js"), &content)
            .unwrap();
        let analysis = metrics.god_object_analysis.unwrap();

        assert!(analysis.is_god_object);
        assert_eq!(analysis.method_count, 52);
        assert!(analysis.responsibilities.contains(&"Parsing".to_string()));
        assert!(
            analysis
                .responsibilities
                .contains(&"Validation".to_string())
        );
        assert!(analysis.responsibilities.contains(&"Rendering".to_string()));
        assert!(
            !analysis
                .responsibilities
                .iter()
                .any(|name| name.starts_with("responsibility_"))
        );
    }

    #[test]
    fn test_python_god_object_uses_semantic_responsibilities() {
        let mut content = String::from("class ReportGenerator:\n");
        for i in 0..20 {
            content.push_str(&format!("    def parse_thing_{}(self):\n", i));
            content.push_str(&format!("        return {}\n", i));
        }
        for i in 0..20 {
            content.push_str(&format!("    def validate_thing_{}(self):\n", i));
            content.push_str("        return True\n");
        }
        for i in 0..12 {
            content.push_str(&format!("    def render_thing_{}(self):\n", i));
            content.push_str(&format!("        return str({})\n", i));
        }

        let analyzer = UnifiedFileAnalyzer::new(None);
        let metrics = analyzer
            .analyze_file(Path::new("generator.py"), &content)
            .unwrap();
        let analysis = metrics.god_object_analysis.unwrap();

        assert!(analysis.is_god_object);
        assert_eq!(analysis.method_count, 52);
        assert!(analysis.responsibilities.contains(&"Parsing".to_string()));
        assert!(
            analysis
                .responsibilities
                .contains(&"Validation".to_string())
        );
        assert!(analysis.responsibilities.contains(&"Rendering".to_string()));
        assert!(
            !analysis
                .responsibilities
                .iter()
                .any(|name| name.starts_with("responsibility_"))
        );
    }

    #[test]
    fn test_heuristic_fallback_does_not_invent_responsibilities() {
        let analysis = UnifiedFileAnalyzer::detect_god_object(60, 1000).unwrap();

        assert!(analysis.is_god_object);
        assert_eq!(analysis.responsibility_count, 0);
        assert!(analysis.responsibilities.is_empty());
        assert!(analysis.responsibility_method_counts.is_empty());
    }
}

use crate::analysis::{FileContext, FileContextDetector};
use crate::core::{FunctionMetrics, Language};
use crate::organization::{GodObjectAnalysis, GodObjectDetector};
use crate::priority::score_types::Score0To100;
use anyhow::Result;
use std::path::Path;

/// Represents the type of file for analysis
#[derive(Debug, Clone, Copy, PartialEq)]
enum FileType {
    Rust,
    Other,
}

/// Pure function to determine file type from path
fn determine_file_type(path: &Path) -> FileType {
    match path.extension().and_then(|s| s.to_str()) {
        Some("rs") => FileType::Rust,
        _ => FileType::Other,
    }
}

/// Pure function to extract metrics from functions
fn extract_function_metrics(functions: &[FunctionMetrics]) -> (usize, u32) {
    let function_count = functions.len();
    let total_complexity = functions.iter().map(|f| f.cyclomatic).sum();
    (function_count, total_complexity)
}

/// Pure function to calculate god object score
fn calculate_god_object_score(function_count: usize, lines: usize) -> f64 {
    ((function_count as f64 / 50.0) + (lines as f64 / 2000.0)) * 50.0
}

/// Pure predicate to check if metrics exceed god object thresholds
fn exceeds_god_object_thresholds(function_count: usize, lines: usize, complexity: u32) -> bool {
    function_count > 50 || lines > 2000 || complexity > 300
}

/// Pure function to create god object analysis from metrics
fn create_god_object_analysis(
    function_count: usize,
    lines: usize,
    total_complexity: u32,
) -> GodObjectAnalysis {
    GodObjectAnalysis {
        is_god_object: true,
        method_count: function_count,
        field_count: 0,
        responsibility_count: 5,
        lines_of_code: lines,
        complexity_sum: total_complexity,
        god_object_score: Score0To100::new(calculate_god_object_score(function_count, lines)),
        recommended_splits: Vec::new(),
        confidence: crate::organization::GodObjectConfidence::Probable,
        responsibilities: Vec::new(),
        responsibility_method_counts: Default::default(),
        purity_distribution: None,
        module_structure: None,
        detection_type: crate::organization::DetectionType::GodFile,
        struct_name: None,
        struct_line: None,
        struct_location: None,      // Spec 201: Added for per-struct analysis
        visibility_breakdown: None, // Spec 134: Added for compatibility
        domain_count: 0,
        domain_diversity: 0.0,
        struct_ratio: 0.0,
        analysis_method: crate::organization::SplitAnalysisMethod::None,
        cross_domain_severity: None,
        domain_diversity_metrics: None, // Spec 152: Added for struct-based analysis
        aggregated_entropy: None,
        aggregated_error_swallowing_count: None,
        aggregated_error_swallowing_patterns: None,
        layering_impact: None,
        anti_pattern_report: None,
    }
}

/// I/O function to detect god object in Rust files
///
/// Spec 201: Uses per-struct analysis. Returns the first god object found,
/// or None if no structs qualify as god objects.
fn detect_rust_god_object(path: &Path, content: &str) -> Result<Option<GodObjectAnalysis>> {
    syn::parse_file(content)
        .map(|ast| {
            let detector = GodObjectDetector::with_source_content(content);
            let analyses = detector.analyze_comprehensive(path, &ast);
            // Return the first god object found, if any
            analyses.into_iter().find(|a| a.is_god_object)
        })
        .or(Ok(None))
}

/// Pure function to detect god object using generic metrics
fn detect_generic_god_object(
    content: &str,
    functions: &[FunctionMetrics],
) -> Result<Option<GodObjectAnalysis>> {
    let lines = content.lines().count();
    let (function_count, total_complexity) = extract_function_metrics(functions);

    if exceeds_god_object_thresholds(function_count, lines, total_complexity) {
        Ok(Some(create_god_object_analysis(
            function_count,
            lines,
            total_complexity,
        )))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub functions: Vec<FunctionMetrics>,
    pub god_object: Option<GodObjectAnalysis>,
    pub file_metrics: FileMetrics,
    pub file_context: FileContext,
}

#[derive(Debug, Clone, Default)]
pub struct FileMetrics {
    pub total_lines: usize,
    pub total_complexity: u32,
    pub function_count: usize,
}

pub trait EnhancedAnalyzer {
    fn analyze_with_patterns(&self, path: &Path, content: &str) -> Result<AnalysisResult> {
        let functions = self.analyze_functions(path, content)?;
        let god_object = self.detect_god_object(path, content, &functions)?;
        let file_metrics = self.calculate_file_metrics(path, content, &functions);
        let file_context = self.detect_file_context(path, &functions);

        Ok(AnalysisResult {
            functions,
            god_object,
            file_metrics,
            file_context,
        })
    }

    fn analyze_functions(&self, path: &Path, content: &str) -> Result<Vec<FunctionMetrics>>;

    fn detect_god_object(
        &self,
        path: &Path,
        content: &str,
        functions: &[FunctionMetrics],
    ) -> Result<Option<GodObjectAnalysis>> {
        match determine_file_type(path) {
            FileType::Rust => detect_rust_god_object(path, content),
            FileType::Other => detect_generic_god_object(content, functions),
        }
    }

    fn calculate_file_metrics(
        &self,
        _path: &Path,
        content: &str,
        functions: &[FunctionMetrics],
    ) -> FileMetrics {
        FileMetrics {
            total_lines: content.lines().count(),
            total_complexity: functions.iter().map(|f| f.cyclomatic).sum(),
            function_count: functions.len(),
        }
    }

    fn detect_file_context(&self, path: &Path, functions: &[FunctionMetrics]) -> FileContext {
        let language = Language::from_path(path);
        let detector = FileContextDetector::new(language);
        detector.detect(path, functions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FunctionMetrics;
    use std::path::PathBuf;

    struct TestAnalyzer;

    impl EnhancedAnalyzer for TestAnalyzer {
        fn analyze_functions(&self, _path: &Path, _content: &str) -> Result<Vec<FunctionMetrics>> {
            let mut metrics =
                FunctionMetrics::new("test_fn".to_string(), PathBuf::from("test.rs"), 1);
            metrics.cyclomatic = 5;
            Ok(vec![metrics])
        }
    }

    #[test]
    fn test_analyze_with_patterns() {
        let analyzer = TestAnalyzer;
        let path = Path::new("test.rs");
        let content = "fn test() {}";

        let result = analyzer.analyze_with_patterns(path, content).unwrap();
        assert_eq!(result.functions.len(), 1);
        assert!(result.god_object.is_none());
        assert_eq!(result.file_metrics.function_count, 1);
    }

    #[test]
    fn test_calculate_file_metrics() {
        let analyzer = TestAnalyzer;
        let path = Path::new("test.rs");
        let content = "line1\nline2\nline3";

        let mut fn1 = FunctionMetrics::new("f1".to_string(), PathBuf::from("test.rs"), 1);
        fn1.cyclomatic = 3;
        let mut fn2 = FunctionMetrics::new("f2".to_string(), PathBuf::from("test.rs"), 2);
        fn2.cyclomatic = 7;
        let functions = vec![fn1, fn2];

        let metrics = analyzer.calculate_file_metrics(path, content, &functions);
        assert_eq!(metrics.total_lines, 3);
        assert_eq!(metrics.total_complexity, 10);
        assert_eq!(metrics.function_count, 2);
    }

    #[test]
    fn test_detect_god_object_non_rust() {
        let analyzer = TestAnalyzer;
        let path = Path::new("test.js");
        let mut functions = Vec::new();
        for i in 0..51 {
            let mut metrics =
                FunctionMetrics::new(format!("fn_{}", i), PathBuf::from("test.js"), i);
            metrics.cyclomatic = 10;
            functions.push(metrics);
        }

        let result = analyzer
            .detect_god_object(path, "content", &functions)
            .unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().is_god_object);
    }

    #[test]
    fn test_detect_god_object_below_threshold() {
        let analyzer = TestAnalyzer;
        let path = Path::new("test.js");
        let functions = vec![FunctionMetrics::new(
            "fn1".to_string(),
            PathBuf::from("test.js"),
            1,
        )];

        let result = analyzer
            .detect_god_object(path, "content", &functions)
            .unwrap();
        assert!(result.is_none());
    }
}

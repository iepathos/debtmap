use crate::core::FunctionMetrics;
use crate::organization::{GodObjectAnalysis, GodObjectDetector};
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub functions: Vec<FunctionMetrics>,
    pub god_object: Option<GodObjectAnalysis>,
    pub file_metrics: FileMetrics,
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

        Ok(AnalysisResult {
            functions,
            god_object,
            file_metrics,
        })
    }

    fn analyze_functions(&self, path: &Path, content: &str) -> Result<Vec<FunctionMetrics>>;

    fn detect_god_object(
        &self,
        path: &Path,
        content: &str,
        functions: &[FunctionMetrics],
    ) -> Result<Option<GodObjectAnalysis>> {
        // Parse content as Rust AST for now (can be extended for other languages)
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(ast) = syn::parse_file(content) {
                let detector = GodObjectDetector::with_source_content(content);
                let analysis = detector.analyze_comprehensive(path, &ast);

                // Only return Some if it's actually a god object
                if analysis.is_god_object {
                    Ok(Some(analysis))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        } else {
            // For non-Rust files, use simpler heuristics
            let lines = content.lines().count();
            let function_count = functions.len();
            let total_complexity: u32 = functions.iter().map(|f| f.cyclomatic).sum();

            // Check if it meets god object thresholds
            if function_count > 50 || lines > 2000 || total_complexity > 300 {
                Ok(Some(GodObjectAnalysis {
                    is_god_object: true,
                    method_count: function_count,
                    field_count: 0,          // Would need language-specific parsing
                    responsibility_count: 5, // Estimated
                    lines_of_code: lines,
                    complexity_sum: total_complexity,
                    god_object_score: ((function_count as f64 / 50.0) + (lines as f64 / 2000.0))
                        * 50.0,
                    recommended_splits: Vec::new(),
                    confidence: crate::organization::GodObjectConfidence::Probable,
                    responsibilities: Vec::new(),
                }))
            } else {
                Ok(None)
            }
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

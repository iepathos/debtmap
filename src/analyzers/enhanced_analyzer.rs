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

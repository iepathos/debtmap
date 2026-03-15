//! Python analyzer implementation
//!
//! The PythonAnalyzer implements the Analyzer trait for Python code.

use crate::analyzers::Analyzer;
use crate::complexity::threshold_manager::{ComplexityThresholds, ThresholdPreset};
use crate::core::ast::Ast;
use crate::core::{ComplexityMetrics, DebtItem, DebtType, FileMetrics, Language, Priority};
use anyhow::Result;
use std::path::PathBuf;
use tracing::{debug, debug_span};

use super::parser::parse_source;

/// Analyzer for Python source code
pub struct PythonAnalyzer {
    /// Complexity threshold for debt detection
    pub(crate) complexity_threshold: u32,
    /// Enhanced thresholds for different complexity metrics
    pub(crate) enhanced_thresholds: ComplexityThresholds,
    /// Enable functional analysis (map/filter/reduce chains)
    pub(crate) enable_functional_analysis: bool,
}

impl PythonAnalyzer {
    /// Create a new Python analyzer
    pub fn new() -> Self {
        Self {
            complexity_threshold: 10,
            enhanced_thresholds: ComplexityThresholds::from_preset(ThresholdPreset::Balanced),
            enable_functional_analysis: false,
        }
    }

    /// Set threshold preset
    pub fn with_threshold_preset(mut self, preset: ThresholdPreset) -> Self {
        self.enhanced_thresholds = ComplexityThresholds::from_preset(preset);
        self
    }

    /// Enable functional analysis
    pub fn with_functional_analysis(mut self, enable: bool) -> Self {
        self.enable_functional_analysis = enable;
        self
    }

    /// Set complexity threshold
    pub fn with_threshold(mut self, threshold: u32) -> Self {
        self.complexity_threshold = threshold;
        self
    }
}

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for PythonAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let _span = debug_span!("parse_python_file", path = %path.display()).entered();

        let start = std::time::Instant::now();
        let python_ast = parse_source(content, &path)?;
        let parse_time = start.elapsed();

        debug!(
            path = %path.display(),
            time_ms = parse_time.as_millis(),
            bytes = content.len(),
            "Parsed Python file"
        );

        Ok(Ast::Python(python_ast))
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::Python(py_ast) => {
                let _span =
                    debug_span!("analyze_python_file", path = %py_ast.path.display()).entered();

                match crate::extraction::python::PythonExtractor::extract(py_ast) {
                    Ok(extracted) => {
                        let result = crate::extraction::adapters::to_file_metrics(&extracted);
                        debug!(
                            functions = result.complexity.functions.len(),
                            debt_items = result.debt_items.len(),
                            "Python file analysis complete"
                        );
                        result
                    }
                    Err(e) => {
                        debug!(error = %e, "Failed to extract Python file data");
                        FileMetrics {
                            path: py_ast.path.clone(),
                            language: Language::Python,
                            complexity: ComplexityMetrics::default(),
                            debt_items: vec![python_extraction_failure(
                                &py_ast.path,
                                &e.to_string(),
                            )],
                            dependencies: vec![],
                            duplications: vec![],
                            total_lines: py_ast.source.lines().count(),
                            module_scope: None,
                            classes: None,
                        }
                    }
                }
            }
            _ => FileMetrics {
                path: PathBuf::new(),
                language: Language::Python,
                complexity: ComplexityMetrics::default(),
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
                total_lines: 0,
                module_scope: None,
                classes: None,
            },
        }
    }

    fn language(&self) -> Language {
        Language::Python
    }
}

fn python_extraction_failure(path: &std::path::Path, error: &str) -> DebtItem {
    DebtItem {
        id: format!("python-extraction-failure-{}", path.display()),
        debt_type: DebtType::CodeSmell {
            smell_type: Some("PythonExtractionFailure".to_string()),
        },
        priority: Priority::High,
        file: path.to_path_buf(),
        line: 1,
        column: None,
        message: "Python analysis failed during extraction".to_string(),
        context: Some(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_analyzer_new() {
        let analyzer = PythonAnalyzer::new();
        assert_eq!(analyzer.language(), Language::Python);
    }

    #[test]
    fn test_parse_simple_python() {
        let analyzer = PythonAnalyzer::new();
        let content = "def hello():\n    return 'world'";
        let path = PathBuf::from("test.py");

        let result = analyzer.parse(content, path);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Ast::Python(_)));
    }

    #[test]
    fn test_python_extraction_failure_creates_debt_item() {
        let item = python_extraction_failure(std::path::Path::new("broken.py"), "boom");
        assert_eq!(item.priority, Priority::High);
        assert_eq!(item.line, 1);
        assert!(item.context.as_deref().unwrap().contains("boom"));
    }
}

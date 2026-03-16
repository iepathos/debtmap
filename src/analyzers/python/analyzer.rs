//! Python analyzer implementation
//!
//! The PythonAnalyzer implements the Analyzer trait for Python code.

use crate::analyzers::Analyzer;
use crate::complexity::threshold_manager::{
    ComplexityLevel, ComplexityThresholds, FunctionRole, ThresholdPreset,
};
use crate::core::ast::Ast;
use crate::core::{ComplexityMetrics, DebtItem, DebtType, FileMetrics, Language, Priority};
use crate::extraction::{ExtractedFileData, ExtractedFunctionData};
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
                        let result = self.enrich_analysis(
                            crate::extraction::adapters::to_file_metrics(&extracted),
                            &extracted,
                        );
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

impl PythonAnalyzer {
    fn enrich_analysis(
        &self,
        mut metrics: FileMetrics,
        extracted: &ExtractedFileData,
    ) -> FileMetrics {
        self.apply_function_annotations(&mut metrics, extracted);
        self.append_complexity_debt(&mut metrics, extracted);
        metrics
    }

    fn apply_function_annotations(&self, metrics: &mut FileMetrics, extracted: &ExtractedFileData) {
        for (function_metrics, extracted_function) in metrics
            .complexity
            .functions
            .iter_mut()
            .zip(extracted.functions.iter())
        {
            let detected_patterns = build_detected_patterns(
                function_metrics,
                extracted_function,
                &self.enhanced_thresholds,
                self.enable_functional_analysis,
            );
            function_metrics.detected_patterns =
                (!detected_patterns.is_empty()).then_some(detected_patterns);
            function_metrics.adjusted_complexity = Some(calculate_adjusted_complexity(
                function_metrics,
                &self.enhanced_thresholds,
            ));
        }
    }

    fn append_complexity_debt(&self, metrics: &mut FileMetrics, extracted: &ExtractedFileData) {
        let new_items = metrics
            .complexity
            .functions
            .iter()
            .zip(extracted.functions.iter())
            .filter(|(function, _)| !function.is_test)
            .filter(|(function, _)| {
                function.cyclomatic >= self.complexity_threshold
                    || function.cognitive >= self.complexity_threshold
            })
            .map(|(function, extracted_function)| {
                complexity_debt_item(
                    &metrics.path,
                    function,
                    extracted_function,
                    self.complexity_threshold,
                )
            });

        metrics.debt_items.extend(new_items);
    }
}

fn build_detected_patterns(
    function: &crate::core::FunctionMetrics,
    extracted_function: &ExtractedFunctionData,
    thresholds: &ComplexityThresholds,
    enable_functional_analysis: bool,
) -> Vec<String> {
    let mut patterns = Vec::new();
    let role = classify_function_role(extracted_function);

    if thresholds.should_flag_function(function, role) {
        let level = match thresholds.get_complexity_level(function) {
            ComplexityLevel::Trivial => "trivial",
            ComplexityLevel::Moderate => "moderate",
            ComplexityLevel::High => "high",
            ComplexityLevel::Excessive => "excessive",
        };
        patterns.push(format!("threshold-exceeded:{level}"));
    }

    if function.cyclomatic >= thresholds.minimum_cyclomatic_complexity {
        patterns.push("cyclomatic-threshold-exceeded".to_string());
    }

    if function.cognitive >= thresholds.minimum_cognitive_complexity {
        patterns.push("cognitive-threshold-exceeded".to_string());
    }

    if function.length >= thresholds.minimum_function_length {
        patterns.push("length-threshold-exceeded".to_string());
    }

    if enable_functional_analysis {
        patterns.extend(detect_functional_patterns(extracted_function));
    }

    patterns
}

fn detect_functional_patterns(extracted_function: &ExtractedFunctionData) -> Vec<String> {
    let mut patterns = Vec::new();

    if extracted_function
        .calls
        .iter()
        .any(|call| call.callee_name == "map")
    {
        patterns.push("functional-pattern:map".to_string());
    }

    if extracted_function
        .calls
        .iter()
        .any(|call| call.callee_name == "filter")
    {
        patterns.push("functional-pattern:filter".to_string());
    }

    if extracted_function
        .calls
        .iter()
        .any(|call| call.callee_name == "reduce")
    {
        patterns.push("functional-pattern:reduce".to_string());
    }

    patterns
}

fn calculate_adjusted_complexity(
    function: &crate::core::FunctionMetrics,
    thresholds: &ComplexityThresholds,
) -> f64 {
    let baseline = thresholds.minimum_total_complexity.max(1) as f64;
    (function.cyclomatic + function.cognitive) as f64 / baseline
}

fn classify_function_role(extracted_function: &ExtractedFunctionData) -> FunctionRole {
    if extracted_function.is_test || extracted_function.in_test_module {
        return FunctionRole::Test;
    }

    if extracted_function.name == "__init__"
        || extracted_function.name.starts_with("get_")
        || extracted_function.name.starts_with("set_")
        || extracted_function.name.starts_with("is_")
        || extracted_function.name.starts_with("has_")
    {
        return FunctionRole::Utility;
    }

    FunctionRole::from_name(&extracted_function.name)
}

fn complexity_debt_item(
    path: &std::path::Path,
    function: &crate::core::FunctionMetrics,
    extracted_function: &ExtractedFunctionData,
    threshold: u32,
) -> DebtItem {
    let max_complexity = function.cyclomatic.max(function.cognitive);
    let priority = if max_complexity >= threshold.saturating_mul(2) {
        Priority::Critical
    } else if max_complexity > threshold + 5 {
        Priority::High
    } else {
        Priority::Medium
    };

    DebtItem {
        id: format!("python-high-complexity-{}-{}", path.display(), function.line),
        debt_type: DebtType::Complexity {
            cyclomatic: function.cyclomatic,
            cognitive: function.cognitive,
        },
        priority,
        file: path.to_path_buf(),
        line: extracted_function.line,
        column: None,
        message: format!(
            "Function '{}' exceeds Python complexity threshold {} (cyclomatic: {}, cognitive: {})",
            function.name, threshold, function.cyclomatic, function.cognitive
        ),
        context: Some(
            "Consider splitting this Python function into smaller units or flattening nested branches."
                .to_string(),
        ),
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
    use crate::complexity::threshold_manager::ThresholdPreset;

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

    #[test]
    fn test_python_thresholds_and_complexity_threshold_affect_analysis() {
        let analyzer = PythonAnalyzer::new()
            .with_threshold(3)
            .with_threshold_preset(ThresholdPreset::Strict);
        let content = r#"
def handle_request(a, b, c, d):
    if a:
        return 1
    if b:
        return 2
    if c:
        return 3
    if d:
        return 4
    return 0
"#;
        let path = PathBuf::from("service.py");
        let ast = analyzer.parse(content, path).unwrap();

        let result = analyzer.analyze(&ast);
        let function = result
            .complexity
            .functions
            .iter()
            .find(|function| function.name == "handle_request")
            .unwrap();

        assert!(function
            .detected_patterns
            .as_ref()
            .is_some_and(|patterns| patterns
                .iter()
                .any(|pattern| pattern.starts_with("threshold-exceeded:"))));
        assert!(result
            .debt_items
            .iter()
            .any(|item| item.id.starts_with("python-high-complexity-")));
    }

    #[test]
    fn test_python_functional_analysis_toggle_detects_map_filter_usage() {
        let disabled = PythonAnalyzer::new();
        let enabled = PythonAnalyzer::new().with_functional_analysis(true);
        let content = r#"
def transform(items):
    return list(map(str, filter(None, items)))
"#;
        let path = PathBuf::from("transform.py");
        let ast_disabled = disabled.parse(content, path.clone()).unwrap();
        let ast_enabled = enabled.parse(content, path).unwrap();

        let disabled_result = disabled.analyze(&ast_disabled);
        let enabled_result = enabled.analyze(&ast_enabled);

        let disabled_patterns = disabled_result.complexity.functions[0]
            .detected_patterns
            .clone()
            .unwrap_or_default();
        let enabled_patterns = enabled_result.complexity.functions[0]
            .detected_patterns
            .clone()
            .unwrap_or_default();

        assert!(disabled_patterns
            .iter()
            .all(|pattern| !pattern.starts_with("functional-pattern:")));
        assert!(enabled_patterns
            .iter()
            .any(|pattern| pattern == "functional-pattern:map"));
        assert!(enabled_patterns
            .iter()
            .any(|pattern| pattern == "functional-pattern:filter"));
    }
}

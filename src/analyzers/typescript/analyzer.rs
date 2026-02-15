//! TypeScript/JavaScript analyzer implementation
//!
//! The TypeScriptAnalyzer implements the Analyzer trait for JavaScript and TypeScript code.

use crate::analyzers::Analyzer;
use crate::complexity::threshold_manager::{ComplexityThresholds, ThresholdPreset};
use crate::core::ast::{Ast, JsLanguageVariant};
use crate::core::{ComplexityMetrics, FileMetrics, Language};
use anyhow::Result;
use std::path::PathBuf;
use tracing::{debug, debug_span};

use super::orchestration::analyze_typescript_file;
use super::parser::{detect_variant, parse_source};

/// Analyzer for JavaScript and TypeScript source code
pub struct TypeScriptAnalyzer {
    /// Complexity threshold for debt detection
    pub(crate) complexity_threshold: u32,
    /// Enhanced thresholds for different complexity metrics
    pub(crate) enhanced_thresholds: ComplexityThresholds,
    /// Language variant to use (auto-detect from file extension if None)
    pub(crate) language_variant: Option<JsLanguageVariant>,
    /// Enable functional analysis (map/filter/reduce chains)
    pub(crate) enable_functional_analysis: bool,
    /// The language this analyzer reports (JavaScript or TypeScript)
    language: Language,
}

impl TypeScriptAnalyzer {
    /// Create a new TypeScript analyzer
    pub fn new() -> Self {
        Self {
            complexity_threshold: 10,
            enhanced_thresholds: ComplexityThresholds::from_preset(ThresholdPreset::Balanced),
            language_variant: None,
            enable_functional_analysis: false,
            language: Language::TypeScript,
        }
    }

    /// Create a JavaScript analyzer
    pub fn javascript() -> Self {
        Self {
            complexity_threshold: 10,
            enhanced_thresholds: ComplexityThresholds::from_preset(ThresholdPreset::Balanced),
            language_variant: Some(JsLanguageVariant::JavaScript),
            enable_functional_analysis: false,
            language: Language::JavaScript,
        }
    }

    /// Create a TypeScript analyzer with specific variant
    pub fn with_variant(variant: JsLanguageVariant) -> Self {
        let language = if variant.has_types() {
            Language::TypeScript
        } else {
            Language::JavaScript
        };

        Self {
            complexity_threshold: 10,
            enhanced_thresholds: ComplexityThresholds::from_preset(ThresholdPreset::Balanced),
            language_variant: Some(variant),
            enable_functional_analysis: false,
            language,
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

impl Default for TypeScriptAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TypeScriptAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let _span = debug_span!("parse_ts_file", path = %path.display()).entered();

        let start = std::time::Instant::now();

        // Determine variant from file extension or use specified variant
        let variant = self
            .language_variant
            .unwrap_or_else(|| detect_variant(&path));

        let ts_ast = parse_source(content, &path, variant)?;
        let parse_time = start.elapsed();

        debug!(
            path = %path.display(),
            time_ms = parse_time.as_millis(),
            bytes = content.len(),
            variant = ?variant,
            "Parsed TypeScript/JavaScript file"
        );

        Ok(Ast::TypeScript(ts_ast))
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::TypeScript(ts_ast) => {
                let _span = debug_span!("analyze_ts_file", path = %ts_ast.path.display()).entered();

                let result = analyze_typescript_file(
                    ts_ast,
                    self.complexity_threshold,
                    &self.enhanced_thresholds,
                    self.enable_functional_analysis,
                );

                debug!(
                    functions = result.complexity.functions.len(),
                    debt_items = result.debt_items.len(),
                    "TypeScript/JavaScript file analysis complete"
                );

                result
            }
            _ => FileMetrics {
                path: PathBuf::new(),
                language: self.language,
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
        self.language
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typescript_analyzer_new() {
        let analyzer = TypeScriptAnalyzer::new();
        assert_eq!(analyzer.language(), Language::TypeScript);
    }

    #[test]
    fn test_javascript_analyzer() {
        let analyzer = TypeScriptAnalyzer::javascript();
        assert_eq!(analyzer.language(), Language::JavaScript);
    }

    #[test]
    fn test_analyzer_with_variant() {
        let analyzer = TypeScriptAnalyzer::with_variant(JsLanguageVariant::Tsx);
        assert_eq!(analyzer.language(), Language::TypeScript);

        let analyzer = TypeScriptAnalyzer::with_variant(JsLanguageVariant::Jsx);
        assert_eq!(analyzer.language(), Language::JavaScript);
    }

    #[test]
    fn test_parse_simple_javascript() {
        let analyzer = TypeScriptAnalyzer::javascript();
        let content = "function hello() { return 'world'; }";
        let path = PathBuf::from("test.js");

        let result = analyzer.parse(content, path);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Ast::TypeScript(_)));
    }

    #[test]
    fn test_parse_simple_typescript() {
        let analyzer = TypeScriptAnalyzer::new();
        let content = "function hello(name: string): string { return `Hello ${name}`; }";
        let path = PathBuf::from("test.ts");

        let result = analyzer.parse(content, path);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Ast::TypeScript(_)));
    }

    #[test]
    fn test_analyzer_with_functional_analysis() {
        let analyzer = TypeScriptAnalyzer::new().with_functional_analysis(true);
        assert!(analyzer.enable_functional_analysis);
    }

    #[test]
    fn test_analyzer_with_threshold() {
        let analyzer = TypeScriptAnalyzer::new().with_threshold(15);
        assert_eq!(analyzer.complexity_threshold, 15);
    }
}

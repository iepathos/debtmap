//! AST visitor for TypeScript/JavaScript
//!
//! Traverses the tree-sitter AST to extract functions and calculate metrics.

pub mod class_analysis;
pub mod function_analysis;
pub mod helpers;

use crate::complexity::threshold_manager::ComplexityThresholds;
use crate::core::ast::TypeScriptAst;
use crate::core::FunctionMetrics;

use super::types::JsFunctionMetrics;
use function_analysis::extract_functions;
use helpers::convert_to_function_metrics;

/// Result of analyzing a TypeScript/JavaScript AST
pub struct AnalysisResult {
    /// Standard function metrics (for compatibility with existing pipeline)
    pub functions: Vec<FunctionMetrics>,
    /// JS-specific function metrics with additional detail
    pub js_functions: Vec<JsFunctionMetrics>,
}

/// Analyze a TypeScript/JavaScript AST
pub fn analyze_ast(
    ast: &TypeScriptAst,
    _thresholds: &ComplexityThresholds,
    enable_functional_analysis: bool,
) -> AnalysisResult {
    // Extract JS-specific function metrics
    let js_functions = extract_functions(ast, enable_functional_analysis);

    // Convert to standard function metrics for compatibility
    let functions: Vec<FunctionMetrics> = js_functions
        .iter()
        .map(convert_to_function_metrics)
        .collect();

    AnalysisResult {
        functions,
        js_functions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::complexity::threshold_manager::ThresholdPreset;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_analyze_ast_extracts_functions() {
        let source = r#"
function foo() { return 1; }
const bar = () => 2;
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();
        let thresholds = ComplexityThresholds::from_preset(ThresholdPreset::Balanced);

        let result = analyze_ast(&ast, &thresholds, false);

        assert_eq!(result.functions.len(), 2);
        assert_eq!(result.js_functions.len(), 2);
    }

    #[test]
    fn test_analyze_ast_typescript() {
        let source = r#"
function greet(name: string): string {
    return `Hello ${name}`;
}

class Greeter {
    greet(name: string): string {
        return `Hi ${name}`;
    }
}
"#;
        let path = PathBuf::from("test.ts");
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();
        let thresholds = ComplexityThresholds::from_preset(ThresholdPreset::Balanced);

        let result = analyze_ast(&ast, &thresholds, false);

        // Should find the function and method
        assert!(result.functions.len() >= 2);
    }
}

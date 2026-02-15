//! Analysis orchestration
//!
//! Top-level functions for orchestrating TypeScript/JavaScript file analysis.

use super::debt::create_debt_items;
use super::dependencies::extract_dependencies;
use super::metrics::{build_file_metrics, calculate_total_complexity};
use super::visitor::analyze_ast;
use crate::complexity::threshold_manager::ComplexityThresholds;
use crate::core::ast::TypeScriptAst;
use crate::core::FileMetrics;

/// Analyze a TypeScript/JavaScript file and return file metrics
pub fn analyze_typescript_file(
    ast: &TypeScriptAst,
    threshold: u32,
    enhanced_thresholds: &ComplexityThresholds,
    enable_functional_analysis: bool,
) -> FileMetrics {
    let start = std::time::Instant::now();

    // Capture line count from source during initial analysis
    let total_lines = ast.source.lines().count();

    // Phase 1: Extract functions and calculate metrics
    let analysis_start = std::time::Instant::now();
    let analysis_result = analyze_ast(ast, enhanced_thresholds, enable_functional_analysis);
    let analysis_time = analysis_start.elapsed();

    // Phase 2: Detect debt items
    let debt_start = std::time::Instant::now();
    let debt_items = create_debt_items(
        ast,
        threshold,
        &analysis_result.functions,
        &analysis_result.js_functions,
    );
    let debt_time = debt_start.elapsed();

    // Phase 3: Extract dependencies
    let deps_start = std::time::Instant::now();
    let dependencies = extract_dependencies(ast);
    let deps_time = deps_start.elapsed();

    // Phase 4: Calculate totals
    let complexity_metrics = calculate_total_complexity(&analysis_result.functions);

    let total_time = start.elapsed();

    if std::env::var("DEBTMAP_TIMING").is_ok() {
        eprintln!(
            "[TIMING] analyze_typescript_file {}: total={:.2}s (analysis={:.2}s, debt={:.2}s, deps={:.2}s)",
            ast.path.display(),
            total_time.as_secs_f64(),
            analysis_time.as_secs_f64(),
            debt_time.as_secs_f64(),
            deps_time.as_secs_f64()
        );
    }

    build_file_metrics(
        ast.path.clone(),
        ast.language_variant,
        analysis_result.functions,
        complexity_metrics,
        debt_items,
        dependencies,
        total_lines,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::complexity::threshold_manager::ThresholdPreset;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_analyze_simple_function() {
        let source = r#"
function hello(name) {
    return 'Hello ' + name;
}
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();
        let thresholds = ComplexityThresholds::from_preset(ThresholdPreset::Balanced);

        let result = analyze_typescript_file(&ast, 10, &thresholds, false);

        assert_eq!(result.path, path);
        assert!(result.total_lines > 0);
    }

    #[test]
    fn test_analyze_complex_function() {
        let source = r#"
function complex(a, b, c) {
    if (a) {
        if (b) {
            if (c) {
                return 1;
            }
            return 2;
        }
        return 3;
    }
    return 4;
}
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();
        let thresholds = ComplexityThresholds::from_preset(ThresholdPreset::Balanced);

        let result = analyze_typescript_file(&ast, 10, &thresholds, false);

        // Should detect the complex function
        assert!(!result.complexity.functions.is_empty());
    }

    #[test]
    fn test_analyze_typescript_with_types() {
        let source = r#"
function greet(name: string): string {
    return `Hello ${name}`;
}

interface User {
    name: string;
    age: number;
}

const getUser = (id: number): User => {
    return { name: "Test", age: 25 };
};
"#;
        let path = PathBuf::from("test.ts");
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();
        let thresholds = ComplexityThresholds::from_preset(ThresholdPreset::Balanced);

        let result = analyze_typescript_file(&ast, 10, &thresholds, false);

        assert_eq!(result.path, path);
    }
}

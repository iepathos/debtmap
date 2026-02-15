//! FileMetrics builder for TypeScript/JavaScript
//!
//! Constructs FileMetrics from analysis results.

use crate::core::ast::JsLanguageVariant;
use crate::core::{ComplexityMetrics, DebtItem, Dependency, FileMetrics, Language};
use std::path::PathBuf;

/// Build FileMetrics from analysis results
pub fn build_file_metrics(
    path: PathBuf,
    variant: JsLanguageVariant,
    functions: Vec<crate::core::FunctionMetrics>,
    complexity_totals: (u32, u32),
    debt_items: Vec<DebtItem>,
    dependencies: Vec<Dependency>,
    total_lines: usize,
) -> FileMetrics {
    let (cyclomatic_total, cognitive_total) = complexity_totals;

    let language = if variant.has_types() {
        Language::TypeScript
    } else {
        Language::JavaScript
    };

    FileMetrics {
        path,
        language,
        complexity: ComplexityMetrics {
            functions,
            cyclomatic_complexity: cyclomatic_total,
            cognitive_complexity: cognitive_total,
        },
        debt_items,
        dependencies,
        duplications: vec![],
        total_lines,
        module_scope: None,
        classes: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_file_metrics_javascript() {
        let path = PathBuf::from("test.js");
        let functions = vec![];

        let metrics = build_file_metrics(
            path.clone(),
            JsLanguageVariant::JavaScript,
            functions,
            (5, 10),
            vec![],
            vec![],
            100,
        );

        assert_eq!(metrics.path, path);
        assert_eq!(metrics.language, Language::JavaScript);
        assert_eq!(metrics.complexity.cyclomatic_complexity, 5);
        assert_eq!(metrics.complexity.cognitive_complexity, 10);
        assert_eq!(metrics.total_lines, 100);
    }

    #[test]
    fn test_build_file_metrics_typescript() {
        let path = PathBuf::from("test.ts");
        let functions = vec![];

        let metrics = build_file_metrics(
            path.clone(),
            JsLanguageVariant::TypeScript,
            functions,
            (3, 6),
            vec![],
            vec![],
            50,
        );

        assert_eq!(metrics.language, Language::TypeScript);
    }

    #[test]
    fn test_build_file_metrics_tsx() {
        let path = PathBuf::from("test.tsx");
        let functions = vec![];

        let metrics = build_file_metrics(
            path.clone(),
            JsLanguageVariant::Tsx,
            functions,
            (0, 0),
            vec![],
            vec![],
            25,
        );

        // TSX is TypeScript
        assert_eq!(metrics.language, Language::TypeScript);
    }
}

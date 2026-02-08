//! Metrics building functions
//!
//! Pure functions for building function and file metrics.

use crate::core::{
    ComplexityMetrics, Dependency, DebtItem, FileMetrics, FunctionMetrics, Language,
};
use std::path::PathBuf;

/// Pure function to calculate total complexity metrics
pub fn calculate_total_complexity(functions: &[FunctionMetrics]) -> (u32, u32) {
    functions.iter().fold((0, 0), |(cyc, cog), f| {
        (cyc + f.cyclomatic, cog + f.cognitive)
    })
}

/// Pure function to build file metrics
pub fn build_file_metrics(
    path: PathBuf,
    functions: Vec<FunctionMetrics>,
    (cyclomatic, cognitive): (u32, u32),
    debt_items: Vec<DebtItem>,
    dependencies: Vec<Dependency>,
    total_lines: usize,
) -> FileMetrics {
    FileMetrics {
        path,
        language: Language::Rust,
        complexity: ComplexityMetrics {
            functions,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
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
    use crate::core::DependencyKind;

    #[test]
    fn test_calculate_total_complexity() {
        let functions = vec![
            FunctionMetrics {
                name: "func1".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                length: 10,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 1,
                visibility: Some("pub".to_string()),
                is_test: false,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: Some(true),
                purity_confidence: Some(1.0),
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
            },
            FunctionMetrics {
                name: "func2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 15,
                length: 8,
                cyclomatic: 3,
                cognitive: 5,
                nesting: 0,
                visibility: None,
                is_test: false,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: Some(true),
                purity_confidence: Some(1.0),
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
            },
        ];

        let (total_cyc, total_cog) = calculate_total_complexity(&functions);
        assert_eq!(total_cyc, 8);
        assert_eq!(total_cog, 15);
    }

    #[test]
    fn test_calculate_total_complexity_empty() {
        let functions = vec![];
        let (total_cyc, total_cog) = calculate_total_complexity(&functions);
        assert_eq!(total_cyc, 0);
        assert_eq!(total_cog, 0);
    }

    #[test]
    fn test_build_file_metrics() {
        let path = PathBuf::from("test.rs");
        let functions = vec![FunctionMetrics {
            name: "test_fn".to_string(),
            file: path.clone(),
            line: 1,
            length: 5,
            cyclomatic: 2,
            cognitive: 3,
            nesting: 1,
            visibility: Some("pub".to_string()),
            is_test: false,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.9),
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
        }];
        let debt_items = vec![];
        let dependencies = vec![Dependency {
            name: "std".to_string(),
            kind: DependencyKind::Import,
        }];

        let metrics = build_file_metrics(
            path.clone(),
            functions.clone(),
            (2, 3),
            debt_items.clone(),
            dependencies.clone(),
            100, // total_lines
        );

        assert_eq!(metrics.path, path);
        assert_eq!(metrics.language, Language::Rust);
        assert_eq!(metrics.complexity.cyclomatic_complexity, 2);
        assert_eq!(metrics.complexity.cognitive_complexity, 3);
        assert_eq!(metrics.complexity.functions.len(), 1);
        assert_eq!(metrics.dependencies.len(), 1);
    }
}

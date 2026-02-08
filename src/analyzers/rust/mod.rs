//! Rust source code analysis
//!
//! This module provides comprehensive analysis of Rust source code, including:
//!
//! - Function complexity metrics (cyclomatic, cognitive)
//! - Technical debt detection
//! - Pattern recognition (mapping, parallel, functional)
//! - Dependency extraction
//!
//! # Example
//!
//! ```ignore
//! use debtmap::analyzers::rust::RustAnalyzer;
//! use debtmap::analyzers::Analyzer;
//!
//! let analyzer = RustAnalyzer::new();
//! let ast = analyzer.parse(source_code, path)?;
//! let metrics = analyzer.analyze(&ast);
//! ```

pub mod analyzer;
pub mod debt;
pub mod dependencies;
pub mod metadata;
pub mod metrics;
pub mod orchestration;
pub mod patterns;
pub mod types;
pub mod visitor;

// Re-export main types for backward compatibility
pub use analyzer::RustAnalyzer;
pub use dependencies::{extract_dependencies, extract_use_name};
pub use orchestration::analyze_rust_file;
pub use types::{
    AnalysisResult, ClosureComplexityMetrics, ComplexityMetricsData, EnhancedFunctionAnalysis,
    FunctionAnalysisData, FunctionContext, FunctionMetadata, PatternSignals,
};
pub use visitor::FunctionVisitor;

// Re-export call graph extraction
use crate::core::ast::RustAst;
use crate::priority::call_graph::CallGraph;

pub fn extract_rust_call_graph(ast: &RustAst) -> CallGraph {
    use super::rust_call_graph::extract_call_graph;
    extract_call_graph(&ast.file, &ast.path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::Analyzer;
    use crate::core::{DependencyKind, Language};
    use std::path::PathBuf;
    use syn::parse_quote;

    #[test]
    fn test_rust_analyzer_new() {
        let analyzer = RustAnalyzer::new();
        assert_eq!(analyzer.language(), Language::Rust);
    }

    #[test]
    fn test_calculate_total_complexity() {
        use crate::core::FunctionMetrics;

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

        let (total_cyc, total_cog) = metrics::calculate_total_complexity(&functions);
        assert_eq!(total_cyc, 8);
        assert_eq!(total_cog, 15);
    }

    #[test]
    fn test_extract_dependencies() {
        let file: syn::File = parse_quote! {
            use std::io;
            use serde::{Deserialize, Serialize};
            use crate::core::Config;

            fn main() {}
        };

        let deps = dependencies::extract_dependencies(&file);
        assert_eq!(deps.len(), 3);
        assert!(deps.iter().any(|d| d.name == "std"));
        assert!(deps.iter().any(|d| d.name == "serde"));
        assert!(deps.iter().any(|d| d.name == "crate"));
        assert!(deps.iter().all(|d| d.kind == DependencyKind::Import));
    }

    #[test]
    fn test_classify_test_file_with_test_directories() {
        assert!(metadata::classify_test_file("src/tests/mod.rs"));
        assert!(metadata::classify_test_file("src/test/utils.rs"));
        assert!(metadata::classify_test_file("src/testing/helpers.rs"));
    }

    #[test]
    fn test_classify_test_file_non_test_files() {
        assert!(!metadata::classify_test_file("src/main.rs"));
        assert!(!metadata::classify_test_file("src/lib.rs"));
        assert!(!metadata::classify_test_file("src/core/module.rs"));
    }

    #[test]
    fn test_has_test_name_pattern() {
        assert!(metadata::has_test_name_pattern("test_something"));
        assert!(metadata::has_test_name_pattern("it_should_work"));
        assert!(metadata::has_test_name_pattern("should_do_something"));
        assert!(metadata::has_test_name_pattern("mock_service"));
        assert!(!metadata::has_test_name_pattern("regular_function"));
    }

    #[test]
    fn test_extract_visibility() {
        let vis: syn::Visibility = parse_quote! { pub };
        assert_eq!(
            metadata::extract_visibility(&vis),
            Some("pub".to_string())
        );

        let vis: syn::Visibility = parse_quote! { pub(crate) };
        assert_eq!(
            metadata::extract_visibility(&vis),
            Some("pub(crate)".to_string())
        );

        let vis: syn::Visibility = parse_quote! {};
        assert_eq!(metadata::extract_visibility(&vis), None);
    }
}

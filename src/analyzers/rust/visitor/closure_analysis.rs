//! Closure analysis
//!
//! Functions for analyzing closures within functions.

use crate::analyzers::rust::types::ClosureComplexityMetrics;
use crate::complexity::cyclomatic::calculate_cyclomatic;
use crate::complexity::pure_mapping_patterns::{
    calculate_adjusted_complexity, MappingPatternConfig, MappingPatternDetector,
};
use crate::complexity::entropy_core::EntropyScore;
use crate::config::get_entropy_config;
use crate::core::FunctionMetrics;
use crate::analyzers::rust_complexity_calculation;
use std::path::PathBuf;

/// Convert closure body to a block for analysis
pub fn convert_closure_to_block(closure: &syn::ExprClosure) -> syn::Block {
    match &*closure.body {
        syn::Expr::Block(expr_block) => expr_block.block.clone(),
        _ => syn::Block {
            brace_token: Default::default(),
            stmts: vec![syn::Stmt::Expr(*closure.body.clone(), None)],
        },
    }
}

/// Calculate complexity metrics for closure
pub fn calculate_closure_complexity(block: &syn::Block) -> ClosureComplexityMetrics {
    ClosureComplexityMetrics {
        cyclomatic: calculate_cyclomatic(block),
        cognitive: rust_complexity_calculation::calculate_cognitive_syn(block),
        nesting: rust_complexity_calculation::calculate_nesting(block),
        length: rust_complexity_calculation::count_lines(block),
    }
}

/// Check if closure is substantial enough to track
pub fn is_substantial_closure(metrics: &ClosureComplexityMetrics) -> bool {
    metrics.cognitive > 1 || metrics.length > 1 || metrics.cyclomatic > 1
}

/// Build function metrics for closure
pub fn build_closure_metrics(
    _closure: &syn::ExprClosure,
    block: &syn::Block,
    complexity: &ClosureComplexityMetrics,
    name: String,
    line: usize,
    file: PathBuf,
    in_test_module: bool,
) -> FunctionMetrics {
    let entropy_score = calculate_closure_entropy(block);

    // Detect pure mapping patterns for closures (spec 118)
    let function_body = quote::quote!(#block).to_string();
    let mapping_detector = MappingPatternDetector::new(MappingPatternConfig::default());
    let mapping_result =
        mapping_detector.analyze_function(&function_body, complexity.cyclomatic);

    let adjusted_complexity = if mapping_result.is_pure_mapping {
        Some(calculate_adjusted_complexity(
            complexity.cyclomatic,
            complexity.cognitive,
            &mapping_result,
        ))
    } else {
        None
    };

    FunctionMetrics {
        name,
        file,
        line,
        cyclomatic: complexity.cyclomatic,
        cognitive: complexity.cognitive,
        nesting: complexity.nesting,
        length: complexity.length,
        is_test: in_test_module,
        visibility: None,
        is_trait_method: false,
        in_test_module,
        entropy_score,
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: if mapping_result.is_pure_mapping {
            Some(mapping_result)
        } else {
            None
        },
        adjusted_complexity,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        entropy_analysis: None,
    }
}

/// Generate name for closure
pub fn generate_closure_name(parent_function: Option<&str>, closure_index: usize) -> String {
    if let Some(parent) = parent_function {
        format!("{}::<closure@{}>", parent, closure_index)
    } else {
        format!("<closure@{}>", closure_index)
    }
}

/// Calculate entropy score for closure if enabled
pub fn calculate_closure_entropy(block: &syn::Block) -> Option<EntropyScore> {
    if get_entropy_config().enabled {
        let mut old_analyzer = crate::complexity::entropy::EntropyAnalyzer::new();
        let old_score = old_analyzer.calculate_entropy(block);

        Some(EntropyScore {
            token_entropy: old_score.token_entropy,
            pattern_repetition: old_score.pattern_repetition,
            branch_similarity: old_score.branch_similarity,
            effective_complexity: old_score.effective_complexity,
            unique_variables: old_score.unique_variables,
            max_nesting: old_score.max_nesting,
            dampening_applied: old_score.dampening_applied,
        })
    } else {
        None
    }
}

//! Function analysis
//!
//! Analysis functions for Rust functions.

use crate::analyzers::rust::metadata::{classify_function_role, extract_function_metadata};
use crate::analyzers::rust::metrics::{create_analysis_result, perform_enhanced_analysis};
use crate::analyzers::rust::patterns::{
    analyze_functional_composition, build_language_specific, detect_mapping_pattern,
    detect_parallel_patterns, detect_pattern_signals,
};
use crate::analyzers::rust::types::{
    ComplexityMetricsData, FunctionAnalysisData, FunctionContext, FunctionMetadata,
};
use crate::analyzers::rust_complexity_calculation;
use crate::complexity::threshold_manager::ComplexityThresholds;
use crate::core::FunctionMetrics;
use crate::debt::error_swallowing::detect_error_swallowing_in_function;

/// Create complete function analysis data
#[allow(clippy::too_many_arguments)]
pub fn create_function_analysis_data(
    name: &str,
    item_fn: &syn::ItemFn,
    _line: usize,
    _is_trait_method: bool,
    context: FunctionContext,
    file_ast: Option<&syn::File>,
    source_content: &str,
    enhanced_thresholds: &ComplexityThresholds,
    enable_functional_analysis: bool,
    enable_rust_patterns: bool,
) -> FunctionAnalysisData {
    let metadata = extract_function_metadata(name, item_fn);
    let complexity_metrics =
        calculate_complexity_metrics(&item_fn.block, item_fn, file_ast);
    let enhanced_analysis = perform_enhanced_analysis(&item_fn.block);
    let role = classify_function_role(name, metadata.is_test);

    let metrics = build_function_metrics(
        &context,
        metadata.clone(),
        complexity_metrics,
        &item_fn.block,
        item_fn,
        file_ast,
        source_content,
        enable_functional_analysis,
        enable_rust_patterns,
    );

    let analysis_result = create_analysis_result(
        name.to_string(),
        &metrics,
        role,
        enhanced_analysis,
        enhanced_thresholds,
    );

    FunctionAnalysisData {
        metrics,
        enhanced_analysis: analysis_result,
    }
}

/// Calculate complexity metrics for a function
pub fn calculate_complexity_metrics(
    block: &syn::Block,
    item_fn: &syn::ItemFn,
    file_ast: Option<&syn::File>,
) -> ComplexityMetricsData {
    ComplexityMetricsData {
        cyclomatic: rust_complexity_calculation::calculate_cyclomatic_with_visitor(
            block, item_fn, file_ast,
        ),
        cognitive: rust_complexity_calculation::calculate_cognitive_with_visitor(
            block, item_fn, file_ast,
        ),
    }
}

/// Build function metrics with all pattern detection
#[allow(clippy::too_many_arguments)]
pub fn build_function_metrics(
    context: &FunctionContext,
    metadata: FunctionMetadata,
    complexity: ComplexityMetricsData,
    block: &syn::Block,
    item_fn: &syn::ItemFn,
    file_ast: Option<&syn::File>,
    source_content: &str,
    enable_functional_analysis: bool,
    enable_rust_patterns: bool,
) -> FunctionMetrics {
    // Phase 1: Detect mapping patterns (spec 118)
    let (mapping_result, mapping_adjusted) =
        detect_mapping_pattern(block, complexity.cyclomatic, complexity.cognitive);

    // Phase 2: Detect parallel patterns (spec 127)
    let (detected_patterns, adjusted_complexity) = detect_parallel_patterns(
        file_ast,
        source_content,
        complexity.cyclomatic,
        mapping_adjusted,
    );

    // Phase 3: Functional composition analysis (spec 111)
    let composition_metrics =
        analyze_functional_composition(enable_functional_analysis, item_fn);

    // Phase 4: Pattern signal detection (specs 179, 180)
    let signals = detect_pattern_signals(block, &context.name);

    // Phase 5: Build language-specific data (spec 146)
    let language_specific = build_language_specific(context, item_fn, &signals, enable_rust_patterns);

    // Phase 6: Error swallowing detection
    let (error_count, error_patterns) = detect_error_swallowing_in_function(block);

    // Assemble final metrics
    FunctionMetrics {
        name: context.name.clone(),
        file: context.file.clone(),
        line: context.line,
        cyclomatic: complexity.cyclomatic,
        cognitive: complexity.cognitive,
        nesting: rust_complexity_calculation::calculate_nesting(block),
        length: rust_complexity_calculation::count_function_lines(item_fn),
        is_test: metadata.is_test,
        visibility: metadata.visibility,
        is_trait_method: context.is_trait_method,
        in_test_module: context.in_test_module,
        entropy_score: metadata.entropy_score,
        is_pure: metadata.purity_info.0,
        purity_confidence: metadata.purity_info.1,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: if detected_patterns.is_empty() {
            None
        } else {
            Some(detected_patterns)
        },
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: if mapping_result.is_pure_mapping {
            Some(mapping_result)
        } else {
            None
        },
        adjusted_complexity,
        composition_metrics,
        language_specific,
        purity_level: metadata.purity_info.2,
        error_swallowing_count: if error_count > 0 {
            Some(error_count)
        } else {
            None
        },
        error_swallowing_patterns: if error_patterns.is_empty() {
            None
        } else {
            Some(error_patterns)
        },
        entropy_analysis: None,
    }
}

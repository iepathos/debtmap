//! Debt item collection
//!
//! Functions for collecting technical debt items from Rust code.

use super::complexity_items::extract_debt_items_with_enhanced;
use super::organization::analyze_organization_patterns;
use super::resource::analyze_resource_patterns;
use crate::analyzers::rust::types::EnhancedFunctionAnalysis;
use crate::core::{DebtItem, FunctionMetrics, Language};
use crate::debt::async_errors::detect_async_errors;
use crate::debt::error_context::analyze_error_context;
use crate::debt::error_propagation::analyze_error_propagation;
use crate::debt::error_swallowing::detect_error_swallowing;
use crate::debt::panic_patterns::detect_panic_patterns;
use crate::debt::patterns::{
    find_code_smells_with_suppression, find_todos_and_fixmes_with_suppression,
};
use crate::debt::smells::{analyze_function_smells, analyze_module_smells};
use crate::debt::suppression::{parse_suppression_comments, SuppressionContext};
use crate::testing;
use std::path::Path;

/// Create debt items for a file
pub fn create_debt_items(
    file: &syn::File,
    path: &std::path::Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    source_content: &str,
    enhanced_analysis: &[EnhancedFunctionAnalysis],
) -> Vec<DebtItem> {
    let suppression_context = parse_suppression_comments(source_content, Language::Rust, path);

    report_rust_unclosed_blocks(&suppression_context);

    collect_all_rust_debt_items(
        file,
        path,
        threshold,
        functions,
        source_content,
        &suppression_context,
        enhanced_analysis,
    )
}

/// Collect all debt items from various sources
pub fn collect_all_rust_debt_items(
    file: &syn::File,
    path: &std::path::Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    source_content: &str,
    suppression_context: &SuppressionContext,
    enhanced_analysis: &[EnhancedFunctionAnalysis],
) -> Vec<DebtItem> {
    [
        extract_debt_items_with_enhanced(file, path, threshold, functions, enhanced_analysis),
        find_todos_and_fixmes_with_suppression(source_content, path, Some(suppression_context)),
        find_code_smells_with_suppression(source_content, path, Some(suppression_context)),
        extract_rust_module_smell_items(path, source_content, suppression_context),
        extract_rust_function_smell_items(functions, suppression_context),
        detect_error_swallowing(file, path, Some(suppression_context)),
        // New enhanced error handling detectors
        detect_panic_patterns(file, path, Some(suppression_context)),
        analyze_error_context(file, path, Some(suppression_context)),
        detect_async_errors(file, path, Some(suppression_context)),
        analyze_error_propagation(file, path, Some(suppression_context)),
        // Existing resource and organization analysis
        analyze_resource_patterns(file, path),
        analyze_organization_patterns(file, path),
        testing::analyze_testing_patterns(file, path),
        analyze_rust_test_quality(file, path),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Analyze Rust test quality
fn analyze_rust_test_quality(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    use crate::testing::rust::analyzer::RustTestQualityAnalyzer;
    use crate::testing::rust::convert_rust_test_issue_to_debt_item;

    let mut analyzer = RustTestQualityAnalyzer::new();
    let issues = analyzer.analyze_file(file, path);

    issues
        .into_iter()
        .map(|issue| convert_rust_test_issue_to_debt_item(issue, path))
        .collect()
}

fn extract_rust_module_smell_items(
    path: &std::path::Path,
    source_content: &str,
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    analyze_module_smells(path, source_content.lines().count())
        .into_iter()
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
        .collect()
}

fn extract_rust_function_smell_items(
    functions: &[FunctionMetrics],
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    functions
        .iter()
        .flat_map(|func| analyze_function_smells(func, 0))
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
        .collect()
}

fn report_rust_unclosed_blocks(suppression_context: &SuppressionContext) {
    for unclosed in &suppression_context.unclosed_blocks {
        eprintln!(
            "Warning: Unclosed suppression block in {} at line {}",
            unclosed.file.display(),
            unclosed.start_line
        );
    }
}

//! Complexity-based debt items
//!
//! Functions for creating debt items from complex functions.

use crate::analyzers::rust::types::EnhancedFunctionAnalysis;
use crate::complexity::message_generator::EnhancedComplexityMessage;
use crate::core::{DebtItem, DebtType, FunctionMetrics, Priority};
use std::path::Path;

/// Pure function to apply cognitive complexity scaling based on pattern type
pub fn extract_debt_items_with_enhanced(
    _file: &syn::File,
    _path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    enhanced_analysis: &[EnhancedFunctionAnalysis],
) -> Vec<DebtItem> {
    functions
        .iter()
        .filter(|func| func.is_complex(threshold))
        .map(|func| create_debt_item_for_function(func, threshold, enhanced_analysis))
        .collect()
}

/// Create debt item for a single function
fn create_debt_item_for_function(
    func: &FunctionMetrics,
    threshold: u32,
    enhanced_analysis: &[EnhancedFunctionAnalysis],
) -> DebtItem {
    // Find corresponding enhanced analysis if available
    let enhanced = find_enhanced_analysis_for_function(&func.name, enhanced_analysis);

    match enhanced.and_then(|a| a.enhanced_message.as_ref()) {
        Some(enhanced_msg) => create_enhanced_debt_item(func, threshold, enhanced_msg),
        None => create_complexity_debt_item(func, threshold),
    }
}

/// Find enhanced analysis for a function
fn find_enhanced_analysis_for_function<'a>(
    function_name: &str,
    enhanced_analysis: &'a [EnhancedFunctionAnalysis],
) -> Option<&'a EnhancedFunctionAnalysis> {
    enhanced_analysis
        .iter()
        .find(|e| e.function_name == function_name)
}

/// Create enhanced debt item with detailed message
fn create_enhanced_debt_item(
    func: &FunctionMetrics,
    threshold: u32,
    enhanced_msg: &EnhancedComplexityMessage,
) -> DebtItem {
    DebtItem {
        id: format!("complexity-{}-{}", func.file.display(), func.line),
        debt_type: DebtType::Complexity {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        },
        priority: classify_priority(func.cyclomatic, threshold),
        file: func.file.clone(),
        line: func.line,
        column: None,
        message: enhanced_msg.summary.clone(),
        context: Some(format_enhanced_context(enhanced_msg)),
    }
}

/// Classify priority based on complexity
fn classify_priority(cyclomatic: u32, threshold: u32) -> Priority {
    if cyclomatic > threshold * 2 {
        Priority::High
    } else {
        Priority::Medium
    }
}

fn format_enhanced_context(msg: &EnhancedComplexityMessage) -> String {
    let mut context = String::new();

    // Add details
    if !msg.details.is_empty() {
        context.push_str("\n\nComplexity Issues:");
        for detail in &msg.details {
            context.push_str(&format!("\n  • {}", detail.description));
        }
    }

    // Add recommendations
    if !msg.recommendations.is_empty() {
        context.push_str("\n\nRecommendations:");
        for rec in &msg.recommendations {
            context.push_str(&format!("\n  • {}: {}", rec.title, rec.description));
        }
    }

    context
}

/// Create basic complexity debt item
pub fn create_complexity_debt_item(func: &FunctionMetrics, threshold: u32) -> DebtItem {
    DebtItem {
        id: format!("complexity-{}-{}", func.file.display(), func.line),
        debt_type: DebtType::Complexity {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        },
        priority: classify_priority(func.cyclomatic, threshold),
        file: func.file.clone(),
        line: func.line,
        column: None,
        message: format!(
            "Function '{}' has high complexity (cyclomatic: {}, cognitive: {})",
            func.name, func.cyclomatic, func.cognitive
        ),
        context: None,
    }
}

#[allow(dead_code)]
pub fn extract_debt_items(
    _file: &syn::File,
    _path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
) -> Vec<DebtItem> {
    functions
        .iter()
        .filter(|func| func.is_complex(threshold))
        .map(|func| create_complexity_debt_item(func, threshold))
        .collect()
}

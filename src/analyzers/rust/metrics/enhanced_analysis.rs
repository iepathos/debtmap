//! Enhanced function analysis
//!
//! Performs enhanced analysis on functions including match patterns and if-else chains.

use crate::analyzers::rust::types::EnhancedFunctionAnalysis;
use crate::complexity::if_else_analyzer::{IfElseChain, IfElseChainAnalyzer};
use crate::complexity::message_generator::generate_enhanced_message;
use crate::complexity::recursive_detector::{MatchLocation, RecursiveMatchDetector};
use crate::complexity::threshold_manager::{ComplexityThresholds, FunctionRole};
use crate::core::FunctionMetrics;

/// Pure function for enhanced analysis
pub fn perform_enhanced_analysis(block: &syn::Block) -> (Vec<MatchLocation>, Vec<IfElseChain>) {
    let mut match_detector = RecursiveMatchDetector::new();
    let matches = match_detector.find_matches_in_block(block);

    let mut if_else_analyzer = IfElseChainAnalyzer::new();
    let if_else_chains = if_else_analyzer.analyze_block(block);

    (matches, if_else_chains)
}

/// Create analysis result with enhanced message if needed
pub fn create_analysis_result(
    name: String,
    metrics: &FunctionMetrics,
    role: FunctionRole,
    enhanced_analysis: (Vec<MatchLocation>, Vec<IfElseChain>),
    enhanced_thresholds: &ComplexityThresholds,
) -> EnhancedFunctionAnalysis {
    let (matches, if_else_chains) = enhanced_analysis;

    let enhanced_message = if enhanced_thresholds.should_flag_function(metrics, role) {
        Some(generate_enhanced_message(
            metrics,
            &matches,
            &if_else_chains,
            enhanced_thresholds,
        ))
    } else {
        None
    };

    EnhancedFunctionAnalysis {
        function_name: name,
        matches,
        if_else_chains,
        enhanced_message,
    }
}

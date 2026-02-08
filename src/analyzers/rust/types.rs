//! Core types for Rust analysis
//!
//! Contains struct definitions for function analysis, metrics, and patterns.

use crate::complexity::entropy_core::EntropyScore;
use crate::complexity::if_else_analyzer::IfElseChain;
use crate::complexity::message_generator::EnhancedComplexityMessage;
use crate::complexity::recursive_detector::MatchLocation;
use crate::core::{FunctionMetrics, PurityLevel};
use std::path::PathBuf;

/// Collected pattern signals from various detectors (used in build_function_metrics)
#[derive(Debug, Clone, Default)]
pub struct PatternSignals {
    pub validation: Option<crate::priority::complexity_patterns::ValidationSignals>,
    pub state_machine: Option<crate::priority::complexity_patterns::StateMachineSignals>,
    pub coordinator: Option<crate::priority::complexity_patterns::CoordinatorSignals>,
}

impl PatternSignals {
    pub fn has_any(&self) -> bool {
        self.validation.is_some() || self.state_machine.is_some() || self.coordinator.is_some()
    }
}

/// Structure to hold analysis results
pub struct AnalysisResult {
    pub functions: Vec<FunctionMetrics>,
    pub enhanced_analysis: Vec<EnhancedFunctionAnalysis>,
}

/// Enhanced function analysis with match patterns and complexity messages
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EnhancedFunctionAnalysis {
    pub function_name: String,
    pub matches: Vec<MatchLocation>,
    pub if_else_chains: Vec<IfElseChain>,
    pub enhanced_message: Option<EnhancedComplexityMessage>,
}

/// Metadata extracted from function signature and attributes
#[derive(Clone)]
pub struct FunctionMetadata {
    pub is_test: bool,
    pub visibility: Option<String>,
    pub entropy_score: Option<EntropyScore>,
    pub purity_info: (Option<bool>, Option<f32>, Option<PurityLevel>),
}

/// Basic complexity metrics
pub struct ComplexityMetricsData {
    pub cyclomatic: u32,
    pub cognitive: u32,
}

/// Complexity metrics for closures
pub struct ClosureComplexityMetrics {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub length: usize,
}

/// Context for function analysis
pub struct FunctionContext {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub is_trait_method: bool,
    pub in_test_module: bool,
    pub impl_type_name: Option<String>,
    pub trait_name: Option<String>,
}

/// Data structure to hold complete function analysis results
pub struct FunctionAnalysisData {
    pub metrics: FunctionMetrics,
    pub enhanced_analysis: EnhancedFunctionAnalysis,
}

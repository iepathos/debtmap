//! Advanced Analysis Module
//!
//! This module provides advanced analysis capabilities including:
//! - Rust-specific call graph analysis with trait dispatch and function pointers
//! - Python-specific call graph analysis with instance method tracking
//! - Python type tracking and inference for improved method resolution
//! - Python-aware dead code detection with magic methods and framework patterns
//! - Framework pattern detection
//! - Cross-module dependency tracking
//! - Multi-pass complexity analysis with attribution
//! - Diagnostic reporting and insights generation

pub mod attribution;
pub mod call_graph;
pub mod diagnostics;
pub mod framework_patterns;
pub mod function_visitor;
pub mod multi_pass;
pub mod python_call_graph;
pub mod python_dead_code;
pub mod python_dead_code_enhanced;
pub mod python_type_tracker;

pub use call_graph::{
    AnalysisConfig, CrossModuleTracker, DeadCodeAnalysis, FrameworkPatternDetector,
    FunctionPointerTracker, RustCallGraph, RustCallGraphBuilder, TraitRegistry,
};
pub use framework_patterns::{
    CustomPattern, FrameworkPattern as NewFrameworkPattern, FrameworkPatternRegistry, FrameworkType,
};
pub use python_dead_code::{FrameworkPattern, PythonDeadCodeDetector, RemovalConfidence};
pub use python_dead_code_enhanced::{
    DeadCodeConfidence, DeadCodeReason, DeadCodeResult, EnhancedDeadCodeAnalyzer, LiveCodeReason,
    RemovalSuggestion,
};
pub use python_type_tracker::{
    ClassInfo, FunctionSignature, PythonType, PythonTypeTracker, TwoPassExtractor,
};

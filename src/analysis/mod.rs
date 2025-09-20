//! Advanced Analysis Module
//!
//! This module provides advanced analysis capabilities including:
//! - Rust-specific call graph analysis with trait dispatch and function pointers
//! - Python-specific call graph analysis with instance method tracking
//! - Python type tracking and inference for improved method resolution
//! - Framework pattern detection
//! - Cross-module dependency tracking
//! - Multi-pass complexity analysis with attribution
//! - Diagnostic reporting and insights generation

pub mod attribution;
pub mod call_graph;
pub mod diagnostics;
pub mod function_visitor;
pub mod multi_pass;
pub mod python_call_graph;
pub mod python_type_tracker;

pub use call_graph::{
    AnalysisConfig, CrossModuleTracker, DeadCodeAnalysis, FrameworkPatternDetector,
    FunctionPointerTracker, RustCallGraph, RustCallGraphBuilder, TraitRegistry,
};
pub use python_type_tracker::{
    ClassInfo, FunctionSignature, PythonType, PythonTypeTracker, TwoPassExtractor,
};

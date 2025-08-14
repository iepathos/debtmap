//! Advanced Analysis Module
//!
//! This module provides advanced analysis capabilities including:
//! - Rust-specific call graph analysis with trait dispatch and function pointers
//! - Python-specific call graph analysis with instance method tracking
//! - Framework pattern detection
//! - Cross-module dependency tracking

pub mod call_graph;
pub mod python_call_graph;

pub use call_graph::{
    AnalysisConfig, CrossModuleTracker, DeadCodeAnalysis, FrameworkPatternDetector,
    FunctionPointerTracker, RustCallGraph, RustCallGraphBuilder, TraitRegistry,
};

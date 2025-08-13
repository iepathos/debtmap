//! Advanced Analysis Module
//!
//! This module provides enhanced analysis capabilities including:
//! - Enhanced call graph analysis with trait dispatch and function pointers
//! - Framework pattern detection
//! - Cross-module dependency tracking

pub mod call_graph;

pub use call_graph::{
    AnalysisConfig, CrossModuleTracker, DeadCodeAnalysis, EnhancedCallGraph,
    EnhancedCallGraphBuilder, FrameworkPatternDetector, FunctionPointerTracker, TraitRegistry,
};

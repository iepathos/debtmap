//! God Object Detection Modules
//!
//! This module contains specialized components for detecting and analyzing god objects:
//! - `types`: Core data structures (pure data)
//! - `thresholds`: Detection constants and configuration
//! - `ast_visitor`: AST traversal and data collection
//! - `metrics`: Scoring and metric calculations
//! - `classifier`: Pattern detection and classification
//! - `recommender`: Recommendation generation

pub mod ast_visitor;
pub mod metrics;
pub mod thresholds;
pub mod types;

// Re-export all types and thresholds for backward compatibility
pub use thresholds::*;
pub use types::*;

pub use ast_visitor::{
    FunctionParameter, FunctionWeight, ModuleFunctionInfo, Responsibility, TypeAnalysis,
    TypeVisitor,
};

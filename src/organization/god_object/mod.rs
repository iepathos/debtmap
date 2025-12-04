//! God Object Detection Modules
//!
//! This module contains specialized components for detecting and analyzing god objects:
//! - `core_types`: Fundamental types (GodObjectAnalysis, DetectionType, etc.)
//! - `classification_types`: GodObjectType, EnhancedGodObjectAnalysis, ClassificationResult
//! - `split_types`: ModuleSplit and recommendation types
//! - `metrics_types`: PurityDistribution and metrics
//! - `types`: Re-exports all types for backward compatibility
//! - `thresholds`: Detection constants and configuration
//! - `ast_visitor`: AST traversal and data collection
//! - `metrics`: Scoring and metric calculations
//! - `classifier`: Pattern detection and classification
//! - `recommender`: Recommendation generation

pub mod ast_visitor;
pub mod classification_types;
pub mod core_types;
pub mod metrics;
pub mod metrics_types;
pub mod scoring;
pub mod split_types;
pub mod thresholds;
pub mod types;

// Re-export all types and thresholds for backward compatibility
pub use classification_types::*;
pub use core_types::*;
pub use metrics_types::*;
pub use scoring::*;
pub use split_types::*;
pub use thresholds::*;

pub use ast_visitor::{
    FunctionParameter, FunctionWeight, ModuleFunctionInfo, Responsibility, TypeAnalysis,
    TypeVisitor,
};

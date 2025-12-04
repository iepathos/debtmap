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
//! - `classifier`: Pattern detection and classification (Phase 5)
//! - `predicates`: Pure boolean detection predicates (Phase 4)
//! - `scoring`: Pure scoring functions (Phase 3)
//! - `recommender`: Recommendation generation
//! - `detector`: Orchestration layer (Phase 7)

pub mod ast_visitor;
pub mod classification_types;
pub mod classifier;
pub mod core_types;
pub mod detector;
pub mod metrics;
pub mod metrics_types;
pub mod predicates;
pub mod recommender;
pub mod scoring;
pub mod split_types;
pub mod thresholds;
pub mod types;

// Re-export all types and thresholds for backward compatibility
pub use classification_types::*;
pub use classifier::*;
pub use core_types::*;
pub use detector::GodObjectDetector;
pub use metrics_types::*;
pub use predicates::*;
pub use recommender::*;
pub use scoring::*;
pub use split_types::*;
pub use thresholds::*;

pub use ast_visitor::{
    FunctionParameter, FunctionWeight, ModuleFunctionInfo, Responsibility, TypeAnalysis,
    TypeVisitor,
};

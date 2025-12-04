//! # God Object Types (Re-exports)
//!
//! This module re-exports all god object types for backward compatibility.
//! The types are organized into focused sub-modules:
//! - `core_types`: Fundamental types (GodObjectAnalysis, DetectionType, etc.)
//! - `classification_types`: GodObjectType, EnhancedGodObjectAnalysis, ClassificationResult
//! - `split_types`: ModuleSplit and recommendation types
//! - `metrics_types`: PurityDistribution and metrics
//!
//! ## Stillwater Architecture
//!
//! All types are part of the **Pure Core** - data structures with no behavior.
//! Following Stillwater principles:
//! - Types are pure data (no methods with side effects)
//! - Validation and computation are separate functions (in other modules)
//! - No I/O operations

// Re-export all types from focused sub-modules
pub use super::classification_types::*;
pub use super::core_types::*;
pub use super::metrics_types::*;
pub use super::split_types::*;

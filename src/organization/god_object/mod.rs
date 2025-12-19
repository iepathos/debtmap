//! God Object Detection Module
//!
//! Refactored following Stillwater principles (Pure Core, Imperative Shell).
//! Spec 262: Recommendation generation has been removed.
//!
//! ## Architecture
//!
//! **Pure Core** (business logic):
//! - `types` - Data structures (re-exports from sub-modules)
//! - `thresholds` - Configuration
//! - `predicates` - Detection predicates
//! - `scoring` - Scoring algorithms
//! - `classifier` - Classification logic
//!
//! Pattern detection is handled by `crate::organization::struct_patterns`
//!
//! **Orchestration**:
//! - `detector` - Composes pure functions into pipeline
//!
//! **I/O Shell**:
//! - `ast_visitor` - AST traversal

// Sub-modules for type organization (need to be public for types.rs to re-export)
pub mod classification_types;
pub mod core_types;
pub mod metrics_types;
pub mod split_types;

// Main modules
pub mod ast_visitor;
pub mod classifier;
pub mod detector;
pub mod heuristics; // Spec 212: Shared fallback heuristics
pub mod predicates;
pub mod scoring;
pub mod thresholds;
pub mod types;

pub mod metrics;
pub mod traits; // Spec 217: Trait-Mandated Method Detection

// Spec 262: The following recommendation modules have been removed:
// - context_recommendations
// - recommendation_generator
// - recommender

// Re-exports for public API
pub use ast_visitor::{
    FunctionParameter, FunctionWeight, ModuleFunctionInfo, Responsibility, TypeAnalysis,
    TypeVisitor,
};
pub use classifier::{
    analyze_function_responsibility, calculate_domain_cohesion, calculate_struct_ratio,
    classify_struct_domain, count_distinct_domains, determine_confidence, extract_domain_from_name,
    extract_domain_keywords, group_methods_by_responsibility, infer_responsibility_with_confidence,
    is_cohesive_struct,
};
pub use detector::GodObjectDetector;
pub use heuristics::{
    detect_from_content, fallback_god_object_heuristics, fallback_with_preserved_analysis,
};
pub use scoring::{calculate_god_object_score, calculate_god_object_score_weighted};
pub use thresholds::*;
pub use types::*;

// Spec 209: Accessor and Boilerplate Method Detection
pub use classification_types::{
    MethodAnalysis, MethodBodyAnalysis, MethodComplexityClass, ReturnExprType,
};
pub use classifier::{
    calculate_weighted_count_from_names, calculate_weighted_method_count, classify_method_by_name,
    classify_method_complexity,
};

// Spec 213: Pure Function Method Weighting
pub use classification_types::{MethodSelfUsage, MethodSelfUsageBreakdown};

// Spec 215: Functional Decomposition Recognition
pub use classification_types::{CompositionPattern, FunctionalDecompositionMetrics};
pub use classifier::{
    calculate_combined_method_weight, calculate_combined_weighted_count, classify_self_usage,
    classify_self_usage_standalone,
};

// Spec 211: Method Complexity Weighting
pub use metrics_types::{calculate_complexity_metrics, ComplexityMetrics, MethodComplexity};
pub use scoring::{calculate_complexity_factor, calculate_god_object_score_with_complexity};
pub use thresholds::ComplexityThresholds;

// Spec 213: Enhanced Scoring with Self-Usage
pub use scoring::{calculate_effective_method_count, calculate_god_object_score_with_self_usage};

// Spec 215: Functional Decomposition Recognition - Scoring
pub use scoring::{apply_functional_bonus, calculate_god_object_score_with_functional_bonus};

// Spec 217: Trait-Mandated Method Detection
pub use traits::{
    classify_all_methods, classify_method_origin, ClassifiedMethod, KnownTraitRegistry,
    MethodOrigin, MethodPattern, TraitCategory, TraitImplInfo, TraitMethodSummary,
};

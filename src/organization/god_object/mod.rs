//! God Object Detection Module
//!
//! Refactored following Stillwater principles (Pure Core, Imperative Shell).
//!
//! ## Architecture
//!
//! **Pure Core** (business logic):
//! - `types` - Data structures (re-exports from sub-modules)
//! - `thresholds` - Configuration
//! - `predicates` - Detection predicates
//! - `scoring` - Scoring algorithms
//! - `classifier` - Classification logic
//! - `recommendation_generator` - Responsibility-aware recommendations
//! - `recommender` - Module split recommendations
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
pub mod recommendation_generator; // Pure Core: Responsibility-aware recommendations
pub mod recommender;
pub mod scoring;
pub mod thresholds;
pub mod types;

pub mod metrics;

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
pub use recommendation_generator::generate_recommendation;
pub use recommender::{
    determine_cross_domain_severity, ensure_unique_name, recommend_module_splits,
    recommend_module_splits_enhanced, recommend_module_splits_enhanced_with_evidence,
    recommend_module_splits_with_evidence, sanitize_module_name, suggest_module_splits_by_domain,
};
pub use scoring::{calculate_god_object_score, calculate_god_object_score_weighted};
pub use thresholds::*;
pub use types::*;

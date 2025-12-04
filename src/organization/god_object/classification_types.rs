//! # Classification Types
//!
//! Types for god object classification and enhanced analysis.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - data structures with no behavior.

use serde::{Deserialize, Serialize};

use super::core_types::{GodObjectAnalysis, StructMetrics};
use super::split_types::ModuleSplit;

/// Classification of god object types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GodObjectType {
    /// Single struct with excessive methods and responsibilities
    GodClass {
        struct_name: String,
        method_count: usize,
        field_count: usize,
        responsibilities: usize,
    },
    /// Multiple structs in a file that collectively exceed thresholds
    GodModule {
        total_structs: usize,
        total_methods: usize,
        largest_struct: StructMetrics,
        suggested_splits: Vec<ModuleSplit>,
    },
    /// Registry/catalog pattern - intentional centralization of trait implementations
    Registry {
        pattern: crate::organization::registry_pattern::RegistryPattern,
        confidence: f64,
        original_score: f64,
        adjusted_score: f64,
    },
    /// Builder pattern - intentional fluent API with many setter methods
    Builder {
        pattern: crate::organization::builder_pattern::BuilderPattern,
        confidence: f64,
        original_score: f64,
        adjusted_score: f64,
    },
    /// Boilerplate pattern - repetitive low-complexity code that should be macro-ified
    BoilerplatePattern {
        pattern: crate::organization::boilerplate_detector::BoilerplatePattern,
        confidence: f64,
        recommendation: String,
    },
    /// No god object detected
    NotGodObject,
}

/// Enhanced god object analysis with struct-level detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedGodObjectAnalysis {
    pub file_metrics: GodObjectAnalysis,
    pub per_struct_metrics: Vec<StructMetrics>,
    pub classification: GodObjectType,
    pub recommendation: String,
}

/// Data structure for grouping structs with their methods
#[derive(Debug, Clone)]
pub struct StructWithMethods {
    pub name: String,
    pub methods: Vec<String>,
    pub line_span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassificationResult {
    /// The classified responsibility category, or `None` if confidence is too low
    pub category: Option<String>,
    /// Confidence score from 0.0 to 1.0
    pub confidence: f64,
    /// Signal types that contributed to this classification
    pub signals_used: Vec<SignalType>,
}

/// Types of signals used for responsibility classification.
///
/// These represent different sources of evidence used to determine
/// a method's responsibility category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalType {
    /// Method name pattern matching
    NameHeuristic,
    /// I/O operation detection in method body
    IoDetection,
    /// Call graph analysis
    CallGraph,
    /// Type signature analysis
    TypeSignature,
    /// Purity and side effect analysis
    PurityAnalysis,
    /// Framework-specific patterns
    FrameworkPattern,
}

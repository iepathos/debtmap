//! # Module Split Types
//!
//! Types for representing module split recommendations and related structures.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - data structures with no behavior.

use serde::{Deserialize, Serialize};

use super::core_types::{Priority, RecommendationSeverity, SplitAnalysisMethod};

/// Record of a split merge operation (Spec 190)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeRecord {
    /// Name of the split that was merged
    pub merged_from: String,
    /// Reason for the merge
    pub reason: String,
    /// Similarity score between the splits
    pub similarity_score: f64,
}

/// Stage type in data transformation pipeline (Spec 182)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum StageType {
    Source,
    Transform,
    Validate,
    Aggregate,
    Sink,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleSplit {
    /// Suggested module name WITHOUT file extension (e.g., "config/misc", not "config/misc.rs").
    /// The formatter will add the appropriate extension based on source file type.
    pub suggested_name: String,
    pub methods_to_move: Vec<String>,
    pub structs_to_move: Vec<String>,
    pub responsibility: String,
    pub estimated_lines: usize,
    pub method_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(default)]
    pub priority: Priority,
    /// Cohesion score (0.0-1.0) measuring how tightly related the methods are
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cohesion_score: Option<f64>,
    /// External modules/structs this module depends on
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies_in: Vec<String>,
    /// External modules/structs that depend on this module
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies_out: Vec<String>,
    /// Semantic domain this split represents
    #[serde(default)]
    pub domain: String,
    /// Explanation of why this split was suggested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// Analysis method that generated this split
    #[serde(default)]
    pub method: SplitAnalysisMethod,
    /// Severity of this recommendation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<RecommendationSeverity>,
    /// Estimated interface size between modules
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface_estimate: Option<InterfaceEstimate>,
    /// Multi-signal classification evidence for this split
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(skip)] // Skip in equality comparison
    pub classification_evidence:
        Option<crate::analysis::multi_signal_aggregation::AggregatedClassification>,
    /// Representative method names to show in recommendations (Spec 178)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub representative_methods: Vec<String>,
    /// Fields from original struct needed by this extracted module (Spec 178)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields_needed: Vec<String>,
    /// Suggested trait extraction for this behavioral group (Spec 178)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trait_suggestion: Option<String>,
    /// Behavioral category for this split (Spec 178)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior_category: Option<String>,
    /// Core type that owns the methods in this module (Spec 181)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_type: Option<String>,
    /// Data flow showing input and output types (Spec 181, 182)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_flow: Vec<String>,
    /// Example type definition with impl blocks showing idiomatic Rust patterns (Spec 181)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_type_definition: Option<String>,
    /// Pipeline stage type (Source, Transform, Validate, Aggregate, Sink) (Spec 182)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_flow_stage: Option<StageType>,
    /// Position in pipeline (0 = input, N = output) (Spec 182)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_position: Option<usize>,
    /// Input types consumed by this module (Spec 182)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_types: Vec<String>,
    /// Output types produced by this module (Spec 182)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_types: Vec<String>,
    /// History of splits that were merged into this one (Spec 190)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merge_history: Vec<MergeRecord>,
    /// Alternative module name suggestions (Spec 191)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternative_names: Vec<crate::organization::semantic_naming::NameCandidate>,
    /// Confidence in the suggested module name (Spec 191)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub naming_confidence: Option<f64>,
    /// Strategy used to generate the module name (Spec 191)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub naming_strategy: Option<crate::organization::semantic_naming::NamingStrategy>,
    /// Cluster quality metrics from improved clustering (Spec 192)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_quality: Option<crate::organization::clustering::ClusterQuality>,
}

impl PartialEq for ModuleSplit {
    fn eq(&self, other: &Self) -> bool {
        self.suggested_name == other.suggested_name
            && self.methods_to_move == other.methods_to_move
            && self.structs_to_move == other.structs_to_move
            && self.responsibility == other.responsibility
            && self.estimated_lines == other.estimated_lines
            && self.method_count == other.method_count
            && self.warning == other.warning
            && self.priority == other.priority
            && self.cohesion_score == other.cohesion_score
            && self.dependencies_in == other.dependencies_in
            && self.dependencies_out == other.dependencies_out
            && self.domain == other.domain
            && self.rationale == other.rationale
            && self.method == other.method
            && self.severity == other.severity
            && self.interface_estimate == other.interface_estimate
        // Skip classification_evidence in equality comparison
    }
}

/// Estimated interface size between proposed modules
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InterfaceEstimate {
    /// Number of public functions that cross module boundaries
    pub public_functions_needed: usize,
    /// Number of shared types between modules
    pub shared_types: usize,
    /// Estimated lines of code for interface definitions
    pub estimated_loc: usize,
}

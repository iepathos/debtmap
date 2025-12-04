//! # God Object Types (Pure Data)
//!
//! Core data structures for god object detection.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - data structures with no behavior.
//! Following Stillwater principles:
//! - Types are pure data (no methods with side effects)
//! - Validation and computation are separate functions (in other modules)
//! - No I/O operations
//!
//! ## Organization
//!
//! - Analysis types: `GodObjectAnalysis`, `EnhancedGodObjectAnalysis`
//! - Configuration types: `DetectionType`, `GodObjectConfidence`, `Priority`
//! - Metric types: `StructMetrics`, `PurityDistribution`, etc.
//! - Recommendation types: `ModuleSplit`, `ClassificationResult`

use serde::{Deserialize, Serialize};

/// Type of god object detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectionType {
    /// Single struct with excessive impl methods.
    ///
    /// Example: A `UserManager` struct with 40 impl methods across
    /// multiple responsibilities (validation, persistence, formatting).
    ///
    /// Detection: Struct with >15 methods, tests excluded from counts.
    GodClass,

    /// File with excessive standalone functions and no structs.
    ///
    /// Example: A functional module with 80 top-level functions
    /// for data processing, no struct definitions.
    ///
    /// Detection: File with no structs + >50 standalone functions, tests included.
    GodFile,

    /// Hybrid: File with both structs AND many standalone functions.
    ///
    /// Detected when standalone functions dominate (>50 functions and
    /// >3x the impl method count). Common in modules following "data
    /// > separate from behavior" patterns.
    ///
    /// Example: A formatter module with DTO structs (10 fields) and
    /// 106 formatting functions.
    ///
    /// # Detection Criteria
    ///
    /// A file is classified as `GodModule` when:
    /// - Contains at least one struct definition
    /// - Has >50 standalone functions
    /// - Standalone count > (impl method count * 3)
    GodModule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectAnalysis {
    pub is_god_object: bool,
    /// **Function count for god object scoring** (Spec 134 Phase 3)
    ///
    /// - **GodClass**: Production methods only (tests excluded)
    /// - **GodFile**: All functions including tests
    ///
    /// This count is used consistently for:
    /// - God object score calculation
    /// - Lines of code estimation
    /// - Visibility breakdown validation
    ///
    /// Note: `EnhancedGodObjectAnalysis.per_struct_metrics` may show different
    /// counts as they include all methods for per-struct breakdown.
    pub method_count: usize,
    pub field_count: usize,
    pub responsibility_count: usize,
    pub lines_of_code: usize,
    pub complexity_sum: u32,
    pub god_object_score: f64,
    pub recommended_splits: Vec<ModuleSplit>,
    pub confidence: GodObjectConfidence,
    pub responsibilities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purity_distribution: Option<PurityDistribution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_structure: Option<crate::analysis::ModuleStructure>,
    /// Type of god object detection (class vs file/module)
    pub detection_type: DetectionType,
    /// Function visibility breakdown (added for spec 134)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility_breakdown: Option<FunctionVisibilityBreakdown>,
    /// Number of distinct semantic domains detected (spec 140)
    #[serde(default)]
    pub domain_count: usize,
    /// Domain diversity score (0.0 to 1.0) (spec 140)
    #[serde(default)]
    pub domain_diversity: f64,
    /// Ratio of struct definitions to total functions (0.0 to 1.0) (spec 140)
    #[serde(default)]
    pub struct_ratio: f64,
    /// Analysis method used for recommendations (spec 140)
    #[serde(default)]
    pub analysis_method: SplitAnalysisMethod,
    /// Severity of cross-domain mixing (if applicable) (spec 140)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_domain_severity: Option<RecommendationSeverity>,
    /// Domain diversity metrics with detailed distribution (spec 152)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_diversity_metrics: Option<crate::organization::DomainDiversityMetrics>,
}

impl GodObjectAnalysis {
    /// Validate metric consistency (Spec 134)
    ///
    /// Checks for contradictions in metrics such as:
    /// - Visibility breakdown sums to total method count
    /// - Responsibility count matches named responsibilities
    /// - Method count is consistent with detection type
    pub fn validate(&self) -> Result<(), MetricInconsistency> {
        // Rule 1: If visibility breakdown exists, it must sum to method_count
        if let Some(ref visibility) = self.visibility_breakdown {
            let vis_total = visibility.total();
            if vis_total != self.method_count {
                return Err(MetricInconsistency::VisibilityMismatch {
                    visibility_total: vis_total,
                    method_count: self.method_count,
                });
            }
        }

        // Rule 2: Responsibility count must match number of named responsibilities
        if self.responsibility_count != self.responsibilities.len() {
            return Err(MetricInconsistency::ResponsibilityCountMismatch {
                declared_count: self.responsibility_count,
                actual_count: self.responsibilities.len(),
            });
        }

        // Rule 3: If functions exist, must have at least one responsibility
        if self.method_count > 0 && self.responsibilities.is_empty() {
            return Err(MetricInconsistency::MissingResponsibilities {
                method_count: self.method_count,
            });
        }

        Ok(())
    }
}

/// Errors indicating metric inconsistencies (Spec 134)
#[derive(Debug, Clone, PartialEq)]
pub enum MetricInconsistency {
    VisibilityMismatch {
        visibility_total: usize,
        method_count: usize,
    },
    ResponsibilityCountMismatch {
        declared_count: usize,
        actual_count: usize,
    },
    MissingResponsibilities {
        method_count: usize,
    },
}

impl std::fmt::Display for MetricInconsistency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricInconsistency::VisibilityMismatch {
                visibility_total,
                method_count,
            } => write!(
                f,
                "Visibility breakdown total ({}) != method_count ({})",
                visibility_total, method_count
            ),
            MetricInconsistency::ResponsibilityCountMismatch {
                declared_count,
                actual_count,
            } => write!(
                f,
                "Declared responsibility_count ({}) != actual responsibilities ({}) in Vec",
                declared_count, actual_count
            ),
            MetricInconsistency::MissingResponsibilities { method_count } => write!(
                f,
                "File has {} methods but responsibilities Vec is empty",
                method_count
            ),
        }
    }
}

impl std::error::Error for MetricInconsistency {}

/// Breakdown of functions by visibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionVisibilityBreakdown {
    pub public: usize,
    pub pub_crate: usize,
    pub pub_super: usize,
    pub private: usize,
}

impl FunctionVisibilityBreakdown {
    pub fn total(&self) -> usize {
        self.public + self.pub_crate + self.pub_super + self.private
    }

    pub fn new() -> Self {
        Self {
            public: 0,
            pub_crate: 0,
            pub_super: 0,
            private: 0,
        }
    }
}

impl Default for FunctionVisibilityBreakdown {
    fn default() -> Self {
        Self::new()
    }
}

/// Distribution of functions by purity level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityDistribution {
    pub pure_count: usize,
    pub probably_pure_count: usize,
    pub impure_count: usize,
    pub pure_weight_contribution: f64,
    pub probably_pure_weight_contribution: f64,
    pub impure_weight_contribution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GodObjectConfidence {
    Definite,     // Exceeds all thresholds
    Probable,     // Exceeds most thresholds
    Possible,     // Exceeds some thresholds
    NotGodObject, // Within acceptable limits
}

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

impl ModuleSplit {
    /// Validates that the suggested name does not include a file extension.
    /// Extensions should be added by the formatter based on the source file type.
    pub(crate) fn validate_name(name: &str) {
        debug_assert!(
            !name.ends_with(".rs")
                && !name.ends_with(".py")
                && !name.ends_with(".js")
                && !name.ends_with(".ts"),
            "ModuleSplit::suggested_name should not include file extension: {}",
            name
        );
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Priority {
    High,
    #[default]
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SplitAnalysisMethod {
    #[default]
    None,
    CrossDomain,
    MethodBased,
    Hybrid,
    TypeBased,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecommendationSeverity {
    Critical,
    High,
    Medium,
    Low,
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

/// Metrics for an individual struct within a file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructMetrics {
    pub name: String,
    pub method_count: usize,
    pub field_count: usize,
    pub responsibilities: Vec<String>,
    pub line_span: (usize, usize),
}

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

/// Main god object detector
pub struct GodObjectDetector {
    pub(crate) max_methods: usize,
    pub(crate) max_fields: usize,
    pub(crate) max_responsibilities: usize,
    pub(crate) location_extractor: Option<crate::common::UnifiedLocationExtractor>,
    pub(crate) source_content: Option<String>,
}

impl Default for GodObjectDetector {
    fn default() -> Self {
        Self {
            max_methods: 15,
            max_fields: 10,
            max_responsibilities: 3,
            location_extractor: None,
            source_content: None,
        }
    }
}

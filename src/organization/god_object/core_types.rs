//! # Core God Object Types
//!
//! Fundamental types for god object detection and classification.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - data structures with no behavior.

use crate::priority::score_types::Score0To100;
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

/// God object analysis result
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
    pub god_object_score: Score0To100,
    pub recommended_splits: Vec<crate::organization::god_object::ModuleSplit>,
    pub confidence: GodObjectConfidence,
    pub responsibilities: Vec<String>,
    #[serde(default)]
    pub responsibility_method_counts: std::collections::HashMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purity_distribution: Option<crate::organization::god_object::PurityDistribution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_structure: Option<crate::analysis::ModuleStructure>,
    /// Type of god object detection (class vs file/module)
    pub detection_type: DetectionType,
    /// Name of the primary struct (for GodClass detection type only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub struct_name: Option<String>,
    /// Line number where the primary struct is defined (for GodClass detection type only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub struct_line: Option<usize>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GodObjectConfidence {
    Definite,     // Exceeds all thresholds
    Probable,     // Exceeds most thresholds
    Possible,     // Exceeds some thresholds
    NotGodObject, // Within acceptable limits
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

/// Metrics for an individual struct within a file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructMetrics {
    pub name: String,
    pub method_count: usize,
    pub field_count: usize,
    pub responsibilities: Vec<String>,
    pub line_span: (usize, usize),
}

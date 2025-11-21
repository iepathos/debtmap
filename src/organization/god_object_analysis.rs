use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
                "Visibility breakdown ({}) does not match method count ({})",
                visibility_total, method_count
            ),
            MetricInconsistency::ResponsibilityCountMismatch {
                declared_count,
                actual_count,
            } => write!(
                f,
                "Declared responsibility count ({}) does not match actual named responsibilities ({})",
                declared_count, actual_count
            ),
            MetricInconsistency::MissingResponsibilities { method_count } => write!(
                f,
                "Functions exist ({}) but no responsibilities identified",
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRecord {
    /// Name of the split that was merged
    pub merged_from: String,
    /// Reason for the merge
    pub reason: String,
    /// Similarity score between the splits
    pub similarity_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Helper to create ModuleSplit with behavioral defaults
impl Default for ModuleSplit {
    fn default() -> Self {
        Self {
            suggested_name: String::new(),
            methods_to_move: vec![],
            structs_to_move: vec![],
            responsibility: String::new(),
            estimated_lines: 0,
            method_count: 0,
            warning: None,
            priority: Priority::Medium,
            cohesion_score: None,
            dependencies_in: vec![],
            dependencies_out: vec![],
            domain: String::new(),
            rationale: None,
            method: SplitAnalysisMethod::None,
            severity: None,
            interface_estimate: None,
            classification_evidence: None,
            representative_methods: vec![],
            fields_needed: vec![],
            trait_suggestion: None,
            behavior_category: None,
            core_type: None,
            data_flow: vec![],
            suggested_type_definition: None,
            data_flow_stage: None,
            pipeline_position: None,
            input_types: vec![],
            output_types: vec![],
            merge_history: vec![],
        }
    }
}

// Manual PartialEq implementation that skips classification_evidence field
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
            && self.representative_methods == other.representative_methods
            && self.fields_needed == other.fields_needed
            && self.trait_suggestion == other.trait_suggestion
            && self.behavior_category == other.behavior_category
            && self.core_type == other.core_type
            && self.data_flow == other.data_flow
            && self.suggested_type_definition == other.suggested_type_definition
            && self.data_flow_stage == other.data_flow_stage
            && self.pipeline_position == other.pipeline_position
            && self.input_types == other.input_types
            && self.output_types == other.output_types
        // Skip classification_evidence in equality comparison
    }
}

impl ModuleSplit {
    /// Validates that the suggested name does not include a file extension.
    /// Extensions should be added by the formatter based on the source file type.
    fn validate_name(name: &str) {
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

#[derive(Debug, Clone)]
pub struct GodObjectThresholds {
    pub max_methods: usize,
    pub max_fields: usize,
    pub max_traits: usize,
    pub max_lines: usize,
    pub max_complexity: u32,
}

impl Default for GodObjectThresholds {
    fn default() -> Self {
        Self {
            max_methods: 20,
            max_fields: 15,
            max_traits: 5,
            max_lines: 1000,
            max_complexity: 200,
        }
    }
}

impl GodObjectThresholds {
    pub fn for_rust() -> Self {
        Self {
            max_methods: 20,
            max_fields: 15,
            max_traits: 5,
            max_lines: 1000,
            max_complexity: 200,
        }
    }

    pub fn for_python() -> Self {
        Self {
            max_methods: 15,
            max_fields: 10,
            max_traits: 3,
            max_lines: 500,
            max_complexity: 150,
        }
    }

    pub fn for_javascript() -> Self {
        Self {
            max_methods: 15,
            max_fields: 20,
            max_traits: 3,
            max_lines: 500,
            max_complexity: 150,
        }
    }
}

pub fn calculate_god_object_score(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    thresholds: &GodObjectThresholds,
) -> f64 {
    let method_factor = (method_count as f64 / thresholds.max_methods as f64).min(3.0);
    let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
    let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
    let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

    // Calculate violation count for minimum score determination
    let mut violation_count = 0;
    if method_count > thresholds.max_methods {
        violation_count += 1;
    }
    if field_count > thresholds.max_fields {
        violation_count += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violation_count += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violation_count += 1;
    }

    // Exponential scaling for severe violations
    let base_score = method_factor * field_factor * responsibility_factor * size_factor;

    // Apply appropriate scoring based on violation severity
    // More nuanced approach to prevent over-flagging moderate files
    if violation_count > 0 {
        // Graduated minimum scores based on violation count
        let base_min_score = match violation_count {
            1 => 30.0, // Single violation: Moderate score
            2 => 50.0, // Two violations: Borderline CRITICAL
            _ => 70.0, // Three+ violations: Likely CRITICAL
        };

        // Reduced multiplier from 50.0 to 20.0 for more conservative scoring
        let score = base_score * 20.0 * (violation_count as f64);
        score.max(base_min_score)
    } else {
        base_score * 10.0
    }
}

/// Calculate complexity-weighted god object score.
///
/// Unlike raw method counting, this function weights each method by its
/// cyclomatic complexity, ensuring that 100 simple functions (complexity 1-3)
/// score better than 10 complex functions (complexity 17+).
///
/// # Arguments
///
/// * `weighted_method_count` - Sum of complexity weights for all functions
/// * `field_count` - Number of fields in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `lines_of_code` - Total lines of code
/// * `avg_complexity` - Average cyclomatic complexity across functions
/// * `thresholds` - God object thresholds for the language
///
/// # Returns
///
/// God object score (0-100+). Scores >70 indicate definite god objects.
pub fn calculate_god_object_score_weighted(
    weighted_method_count: f64,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    avg_complexity: f64,
    thresholds: &GodObjectThresholds,
) -> f64 {
    // Use weighted count instead of raw count
    let method_factor = (weighted_method_count / thresholds.max_methods as f64).min(3.0);
    let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
    let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
    let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

    // Add complexity bonus/penalty
    let complexity_factor = if avg_complexity < 3.0 {
        0.7 // Reward simple functions
    } else if avg_complexity > 10.0 {
        1.5 // Penalize complex functions
    } else {
        1.0
    };

    // Calculate violation count for minimum score determination
    let mut violation_count = 0;
    if weighted_method_count > thresholds.max_methods as f64 {
        violation_count += 1;
    }
    if field_count > thresholds.max_fields {
        violation_count += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violation_count += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violation_count += 1;
    }

    // Exponential scaling for severe violations
    let base_score = method_factor * field_factor * responsibility_factor * size_factor;

    // Apply complexity factor and ensure appropriate score for violations
    // Scale scores more conservatively to prevent small files from being CRITICAL
    if violation_count > 0 {
        // More nuanced minimum scores based on violation severity
        // 1 violation (e.g., just responsibilities): 30-50 range
        // 2 violations: 50-70 range
        // 3+ violations: 70+ range (CRITICAL territory)
        let base_min_score = match violation_count {
            1 => 30.0, // Moderate threshold - won't trigger CRITICAL (< 50)
            2 => 50.0, // High threshold - borderline CRITICAL
            _ => 70.0, // Multiple violations - likely CRITICAL
        };

        // Reduced multiplier from 50.0 to 20.0 for more conservative scoring
        let score = base_score * 20.0 * complexity_factor * (violation_count as f64);
        score.max(base_min_score)
    } else {
        base_score * 10.0 * complexity_factor
    }
}

pub fn determine_confidence(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_sum: u32,
    thresholds: &GodObjectThresholds,
) -> GodObjectConfidence {
    let mut violations = 0;

    if method_count > thresholds.max_methods {
        violations += 1;
    }
    if field_count > thresholds.max_fields {
        violations += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violations += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violations += 1;
    }
    if complexity_sum > thresholds.max_complexity {
        violations += 1;
    }

    match violations {
        5 => GodObjectConfidence::Definite,
        3..=4 => GodObjectConfidence::Probable,
        1..=2 => GodObjectConfidence::Possible,
        _ => GodObjectConfidence::NotGodObject,
    }
}

pub fn group_methods_by_responsibility(methods: &[String]) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for method in methods {
        let responsibility = infer_responsibility_from_method(method);
        groups
            .entry(responsibility)
            .or_default()
            .push(method.clone());
    }

    groups
}

/// Infer responsibility using both I/O detection and name-based heuristics.
///
/// This provides more accurate classification than name-based heuristics alone
/// by analyzing actual I/O operations in the function body.
///
/// # Arguments
///
/// * `method_name` - Name of the method/function
/// * `method_body` - Optional source code of the method body
/// * `language` - Programming language for I/O pattern detection
///
/// # Returns
///
/// Responsibility category string
///
/// # Strategy
///
/// 1. If method body is provided, use I/O detection (primary signal)
/// 2. Fall back to name-based heuristics if no I/O detected or body not available
/// 3. For conflicting signals, I/O detection takes precedence
pub fn infer_responsibility_with_io_detection(
    method_name: &str,
    method_body: Option<&str>,
    language: crate::analysis::io_detection::Language,
) -> String {
    use crate::analysis::io_detection::{IoDetector, Responsibility};

    // If we have the method body, use I/O detection as primary signal
    if let Some(body) = method_body {
        let detector = IoDetector::new();
        let profile = detector.detect_io(body, language);

        // If I/O operations detected, use I/O-based classification
        if !profile.is_pure {
            let io_responsibility = profile.primary_responsibility();
            return match io_responsibility {
                Responsibility::FileIO => "File I/O".to_string(),
                Responsibility::NetworkIO => "Network I/O".to_string(),
                Responsibility::ConsoleIO => "Console I/O".to_string(),
                Responsibility::DatabaseIO => "Database I/O".to_string(),
                Responsibility::MixedIO => "Mixed I/O".to_string(),
                Responsibility::SideEffects => "Side Effects".to_string(),
                Responsibility::PureComputation => {
                    // For pure functions, name heuristics might be more informative
                    infer_responsibility_from_method(method_name)
                }
            };
        }
    }

    // Fall back to name-based heuristics
    infer_responsibility_from_method(method_name)
}

/// Map I/O-based responsibility to traditional responsibility categories.
///
/// This helps maintain backward compatibility with existing classification while
/// leveraging the improved accuracy of I/O detection.
pub fn map_io_to_traditional_responsibility(io_resp: &str) -> String {
    match io_resp {
        "File I/O" | "Network I/O" | "Database I/O" => "persistence".to_string(),
        "Console I/O" => "output".to_string(),
        "Mixed I/O" => "processing".to_string(),
        _ => io_resp.to_string(),
    }
}

/// Infer responsibility from call patterns.
///
/// Analyzes what functions a method calls and who calls it to infer responsibility.
/// This complements name-based and I/O-based detection by looking at actual usage patterns.
///
/// # Arguments
///
/// * `function_name` - Name of the function
/// * `callees` - Functions that this function calls
/// * `callers` - Functions that call this function
///
/// # Returns
///
/// Optional responsibility string based on call patterns
///
/// # Examples
///
/// ```rust,ignore
/// // Function mostly called by formatting functions
/// let callees = vec![];
/// let callers = vec!["format_output", "render_table"];
/// let resp = infer_responsibility_from_call_patterns("escape_html", &callees, &callers);
/// assert_eq!(resp, Some("Formatting Support"));
/// ```
pub fn infer_responsibility_from_call_patterns(
    function_name: &str,
    callees: &[String],
    callers: &[String],
) -> Option<String> {
    // Special case: if function name suggests it's a helper and has many callers
    if (function_name.contains("helper") || function_name.contains("util")) && callers.len() >= 3 {
        return Some("utilities".to_string());
    }

    // Analyze caller patterns - who uses this function?
    let caller_categories = categorize_functions(callers);

    // Analyze callee patterns - what does this function use?
    let callee_categories = categorize_functions(callees);

    // If majority of callers are in one category, this is likely support for that category
    // Skip if the category is "utilities" (catch-all)
    if let Some((category, count)) = find_dominant_category(&caller_categories) {
        if count >= 2 && category != "utilities" {
            return Some(format!("{} Support", category));
        }
    }

    // If majority of callees are in one category, this is likely orchestration for that category
    if let Some((category, count)) = find_dominant_category(&callee_categories) {
        if count >= 3 {
            return Some(format!("{} Orchestration", category));
        }
    }

    None
}

/// Categorize functions by their name patterns
fn categorize_functions(functions: &[String]) -> std::collections::HashMap<String, usize> {
    let mut categories = std::collections::HashMap::new();

    for func in functions {
        let category = infer_responsibility_from_method(func);
        *categories.entry(category).or_insert(0) += 1;
    }

    categories
}

/// Find the dominant category in a set of categorized functions
fn find_dominant_category(
    categories: &std::collections::HashMap<String, usize>,
) -> Option<(String, usize)> {
    categories
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(category, count)| (category.clone(), *count))
}

/// Infer responsibility using multi-signal aggregation (Spec 145).
///
/// This provides the highest accuracy classification by combining:
/// - I/O Detection (40% weight)
/// - Call Graph Analysis (30% weight)
/// - Type Signatures (15% weight)
/// - Purity Analysis (10% weight)
/// - Framework Patterns (5% weight)
/// - Name Heuristics (5% weight)
///
/// Target accuracy: ~88% (vs ~50% with name-based alone)
///
/// # Arguments
///
/// * `method_name` - Name of the method/function
/// * `method_body` - Optional source code of the method body
/// * `language` - Programming language
///
/// # Returns
///
/// Tuple of (responsibility string, confidence score, classification evidence)
pub fn infer_responsibility_multi_signal(
    method_name: &str,
    method_body: Option<&str>,
    language: crate::analysis::io_detection::Language,
) -> (
    String,
    f64,
    crate::analysis::multi_signal_aggregation::AggregatedClassification,
) {
    use crate::analysis::multi_signal_aggregation::{ResponsibilityAggregator, SignalSet};

    let aggregator = ResponsibilityAggregator::new();

    // Collect all available signals
    let mut signals = SignalSet::default();

    // I/O signal (if body available)
    if let Some(body) = method_body {
        signals.io_signal = aggregator.collect_io_signal(body, language);
        signals.purity_signal = aggregator.collect_purity_signal(body, language);
    }

    // Name signal (always available)
    signals.name_signal = Some(aggregator.collect_name_signal(method_name));

    // Aggregate all signals
    let result = aggregator.aggregate(&signals);

    // Convert to traditional responsibility string
    let responsibility = result.primary.as_str().to_string();
    let confidence = result.confidence;

    (responsibility, confidence, result)
}

/// Group methods by responsibility using multi-signal aggregation.
///
/// This provides more accurate grouping than name-based heuristics alone.
pub fn group_methods_by_responsibility_multi_signal(
    methods: &[(String, Option<String>)],
    language: crate::analysis::io_detection::Language,
) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for (method_name, method_body) in methods {
        let (responsibility, _confidence, _evidence) =
            infer_responsibility_multi_signal(method_name, method_body.as_deref(), language);

        groups
            .entry(responsibility)
            .or_default()
            .push(method_name.clone());
    }

    groups
}

/// Group methods by responsibility with classification evidence.
///
/// Returns both grouped methods and their classification evidence.
pub fn group_methods_by_responsibility_with_evidence(
    methods: &[(String, Option<String>)],
    language: crate::analysis::io_detection::Language,
) -> (
    HashMap<String, Vec<String>>,
    HashMap<String, crate::analysis::multi_signal_aggregation::AggregatedClassification>,
) {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    let mut evidence_map: HashMap<
        String,
        crate::analysis::multi_signal_aggregation::AggregatedClassification,
    > = HashMap::new();

    for (method_name, method_body) in methods {
        let (responsibility, _confidence, evidence) =
            infer_responsibility_multi_signal(method_name, method_body.as_deref(), language);

        // Store evidence for this responsibility (use first occurrence)
        evidence_map
            .entry(responsibility.clone())
            .or_insert(evidence);

        groups
            .entry(responsibility)
            .or_default()
            .push(method_name.clone());
    }

    (groups, evidence_map)
}

/// Responsibility category definition for method name classification.
///
/// This struct defines a single category with its name and the method name
/// prefixes that indicate membership in that category.
///
/// # Examples
///
/// ```
/// # use debtmap::organization::god_object_analysis::ResponsibilityCategory;
/// let category = ResponsibilityCategory {
///     name: "data_access",
///     prefixes: &["get", "set"],
/// };
/// assert!(category.matches("get_value"));
/// assert!(category.matches("set_config"));
/// assert!(!category.matches("calculate_sum"));
/// ```
pub struct ResponsibilityCategory {
    pub name: &'static str,
    pub prefixes: &'static [&'static str],
}

impl ResponsibilityCategory {
    /// Check if a method name matches any of this category's prefixes.
    ///
    /// # Arguments
    ///
    /// * `method_name` - The lowercased method name to check
    ///
    /// # Returns
    ///
    /// `true` if the method name starts with any of this category's prefixes
    pub fn matches(&self, method_name: &str) -> bool {
        self.prefixes
            .iter()
            .any(|prefix| method_name.starts_with(prefix))
    }
}

/// Static responsibility categories ordered by specificity.
///
/// Categories are checked in order, so more specific categories should appear first.
/// The "utilities" category has no prefixes and serves as a fallback for unmatched methods.
///
/// # Adding New Categories
///
/// To add a new category:
/// 1. Insert a new `ResponsibilityCategory` entry in the appropriate position
/// 2. Provide a descriptive name and list of prefixes
/// 3. Add unit tests covering the new prefixes
/// 4. Update the function documentation below
///
/// # Example
///
/// ```
/// # use debtmap::organization::god_object_analysis::ResponsibilityCategory;
/// let category = ResponsibilityCategory {
///     name: "Authentication",
///     prefixes: &["auth", "login", "logout"],
/// };
/// # assert!(category.matches("auth_user"));
/// ```
const RESPONSIBILITY_CATEGORIES: &[ResponsibilityCategory] = &[
    ResponsibilityCategory {
        name: "output",
        prefixes: &[
            "format", "render", "write", "print", "display", "show", "draw", "output", "emit",
        ],
    },
    ResponsibilityCategory {
        name: "parsing",
        prefixes: &[
            "parse",
            "read",
            "extract",
            "decode",
            "deserialize",
            "unmarshal",
            "scan",
        ],
    },
    ResponsibilityCategory {
        name: "filtering",
        prefixes: &[
            "filter", "select", "find", "search", "query", "lookup", "match",
        ],
    },
    ResponsibilityCategory {
        name: "transformation",
        prefixes: &["transform", "convert", "map", "apply", "adapt"],
    },
    ResponsibilityCategory {
        name: "data_access",
        prefixes: &["get", "set", "fetch", "retrieve", "access"],
    },
    ResponsibilityCategory {
        name: "validation",
        prefixes: &["validate", "check", "verify", "is", "ensure", "assert"],
    },
    ResponsibilityCategory {
        name: "computation",
        prefixes: &["calculate", "compute", "evaluate", "measure"],
    },
    ResponsibilityCategory {
        name: "construction",
        prefixes: &["create", "build", "new", "make", "construct"],
    },
    ResponsibilityCategory {
        name: "persistence",
        prefixes: &["save", "load", "store", "persist", "cache"],
    },
    ResponsibilityCategory {
        name: "processing",
        prefixes: &["process", "handle", "execute", "run"],
    },
    ResponsibilityCategory {
        name: "communication",
        prefixes: &["send", "receive", "transmit", "broadcast", "notify"],
    },
    ResponsibilityCategory {
        name: "utilities",
        prefixes: &[],
    },
];

/// Infer responsibility category from function/method name using pattern matching.
///
/// This function uses a data-driven approach to categorize functions by matching
/// method name prefixes against predefined categories. It searches through
/// `RESPONSIBILITY_CATEGORIES` in order and returns the first matching category.
///
/// # Implementation
///
/// The function:
/// 1. Converts the method name to lowercase for case-insensitive matching
/// 2. Iterates through categories until finding one with a matching prefix
/// 3. Returns the category name, or "utilities" if no match is found
///
/// # Pattern Recognition
///
/// - `format_*`, `render_*`, `write_*`, `print_*`, `display_*`, `show_*`, `draw_*`, `output_*`, `emit_*` → "output"
/// - `parse_*`, `read_*`, `extract_*`, `decode_*`, `deserialize_*`, `unmarshal_*`, `scan_*` → "parsing"
/// - `filter_*`, `select_*`, `find_*`, `search_*`, `query_*`, `lookup_*`, `match_*` → "filtering"
/// - `transform_*`, `convert_*`, `map_*`, `apply_*`, `adapt_*` → "transformation"
/// - `get_*`, `set_*`, `fetch_*`, `retrieve_*`, `access_*` → "data_access"
/// - `validate_*`, `check_*`, `verify_*`, `is_*`, `ensure_*`, `assert_*` → "validation"
/// - `calculate_*`, `compute_*`, `evaluate_*`, `measure_*` → "computation"
/// - `create_*`, `build_*`, `new_*`, `make_*`, `construct_*` → "construction"
/// - `save_*`, `load_*`, `store_*`, `persist_*`, `cache_*` → "persistence"
/// - `process_*`, `handle_*`, `execute_*`, `run_*` → "processing"
/// - `send_*`, `receive_*`, `transmit_*`, `broadcast_*`, `notify_*` → "communication"
/// - Everything else → "utilities"
///
/// # Examples
///
/// ```
/// # use debtmap::organization::god_object_analysis::infer_responsibility_from_method;
/// assert_eq!(infer_responsibility_from_method("format_output"), "output");
/// assert_eq!(infer_responsibility_from_method("parse_json"), "parsing");
/// assert_eq!(infer_responsibility_from_method("calculate_average"), "computation");
/// assert_eq!(infer_responsibility_from_method("helper_function"), "Helper");
/// ```
///
/// # Performance
///
/// This is a pure function with O(n*m) complexity where n is the number of categories
/// (currently 12) and m is the average number of prefixes per category (~3).
/// In practice, most matches occur in the first few categories.
///
/// # Extending Patterns
///
/// To add new patterns, modify `RESPONSIBILITY_CATEGORIES` rather than this function.
/// See the documentation on `RESPONSIBILITY_CATEGORIES` for details.
///
/// # Alternative
///
/// For more accurate classification, consider `infer_responsibility_with_io_detection`
/// which analyzes actual I/O operations in the function body rather than just names.
pub fn infer_responsibility_from_method(method_name: &str) -> String {
    let lower = method_name.to_lowercase();

    // First try the responsibility categories
    if let Some(cat) = RESPONSIBILITY_CATEGORIES
        .iter()
        .find(|cat| cat.matches(&lower))
    {
        return cat.name.to_string();
    }

    // Fall back to behavioral categorization (Spec 178: avoid "misc" and "utilities")
    use crate::organization::BehavioralCategorizer;
    let category = BehavioralCategorizer::categorize_method(method_name);
    category.display_name()
}

/// Map old category names to new names for backward compatibility.
///
/// This function provides a migration path for code and configuration files that
/// still use the old verbose category names (e.g., "output").
///
/// # Arguments
///
/// * `old_name` - The category name to normalize
///
/// # Returns
///
/// The normalized category name in the new format (lowercase, underscores)
///
/// # Examples
///
/// ```
/// # use debtmap::organization::god_object_analysis::normalize_category_name;
/// assert_eq!(normalize_category_name("output"), "output");
/// assert_eq!(normalize_category_name("parsing"), "parsing");
/// assert_eq!(normalize_category_name("data_access"), "data_access");
/// assert_eq!(normalize_category_name("output"), "output"); // Already normalized
/// ```
pub fn normalize_category_name(old_name: &str) -> String {
    match old_name {
        "output" => "output".to_string(),
        "parsing" => "parsing".to_string(),
        "filtering" => "filtering".to_string(),
        "data_access" => "data_access".to_string(),
        // Already normalized names pass through
        name => name.to_lowercase().replace(' ', "_"),
    }
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

pub fn recommend_module_splits(
    type_name: &str,
    _methods: &[String],
    responsibility_groups: &HashMap<String, Vec<String>>,
) -> Vec<ModuleSplit> {
    recommend_module_splits_with_evidence(
        type_name,
        _methods,
        responsibility_groups,
        &HashMap::new(),
    )
}

/// Enhanced version that includes field access tracking and trait extraction
pub fn recommend_module_splits_enhanced(
    type_name: &str,
    responsibility_groups: &HashMap<String, Vec<String>>,
    field_tracker: Option<&crate::organization::FieldAccessTracker>,
) -> Vec<ModuleSplit> {
    recommend_module_splits_enhanced_with_evidence(
        type_name,
        responsibility_groups,
        &HashMap::new(),
        field_tracker,
    )
}

/// Full-featured recommendation with evidence, field tracking, and trait extraction
pub fn recommend_module_splits_enhanced_with_evidence(
    type_name: &str,
    responsibility_groups: &HashMap<String, Vec<String>>,
    evidence_map: &HashMap<
        String,
        crate::analysis::multi_signal_aggregation::AggregatedClassification,
    >,
    field_tracker: Option<&crate::organization::FieldAccessTracker>,
) -> Vec<ModuleSplit> {
    let mut recommendations = Vec::new();

    for (responsibility, methods) in responsibility_groups {
        if methods.len() > 5 {
            let classification_evidence = evidence_map.get(responsibility).cloned();

            // Sanitize the responsibility name for use in module name
            let sanitized_responsibility = sanitize_module_name(responsibility);

            // Get representative methods (first 5-8)
            let representative_methods: Vec<String> = methods.iter().take(8).cloned().collect();

            // Infer behavioral category from responsibility
            let behavior_category = Some(responsibility.clone());

            // Calculate minimal field set if field tracker available
            let fields_needed = field_tracker
                .map(|tracker| tracker.get_minimal_field_set(methods))
                .unwrap_or_default();

            // Generate trait suggestion using behavioral categorization
            use crate::organization::behavioral_decomposition::{
                suggest_trait_extraction, BehavioralCategorizer, MethodCluster,
            };

            let category = BehavioralCategorizer::categorize_method(
                methods.first().map(|s| s.as_str()).unwrap_or(""),
            );

            let cluster = MethodCluster {
                category,
                methods: methods.clone(),
                fields_accessed: fields_needed.clone(),
                internal_calls: 0, // Will be populated by call graph analysis
                external_calls: 0, // Will be populated by call graph analysis
                cohesion_score: 0.0,
            };

            let trait_suggestion = Some(suggest_trait_extraction(&cluster, type_name));

            recommendations.push(ModuleSplit {
                suggested_name: format!(
                    "{}_{}",
                    type_name.to_lowercase(),
                    sanitized_responsibility
                ),
                methods_to_move: methods.clone(),
                structs_to_move: vec![],
                responsibility: responsibility.clone(),
                estimated_lines: methods.len() * 20,
                method_count: methods.len(),
                warning: None,
                priority: Priority::Medium,
                cohesion_score: None,
                dependencies_in: vec![],
                dependencies_out: vec![],
                domain: String::new(),
                rationale: Some(format!(
                    "Methods grouped by '{}' responsibility pattern",
                    responsibility
                )),
                method: SplitAnalysisMethod::MethodBased,
                severity: None,
                interface_estimate: None,
                classification_evidence,
                representative_methods,
                fields_needed,
                trait_suggestion,
                behavior_category,
                core_type: None,
                data_flow: vec![],
                suggested_type_definition: None,
                data_flow_stage: None,
                pipeline_position: None,
                input_types: vec![],
                output_types: vec![],
                merge_history: vec![],
            });
        }
    }

    recommendations
}

pub fn recommend_module_splits_with_evidence(
    type_name: &str,
    _methods: &[String],
    responsibility_groups: &HashMap<String, Vec<String>>,
    evidence_map: &HashMap<
        String,
        crate::analysis::multi_signal_aggregation::AggregatedClassification,
    >,
) -> Vec<ModuleSplit> {
    let mut recommendations = Vec::new();

    for (responsibility, methods) in responsibility_groups {
        if methods.len() > 5 {
            let classification_evidence = evidence_map.get(responsibility).cloned();

            // Sanitize the responsibility name for use in module name
            let sanitized_responsibility = sanitize_module_name(responsibility);

            // Get representative methods (first 5-8)
            let representative_methods: Vec<String> = methods.iter().take(8).cloned().collect();

            // Infer behavioral category from responsibility
            let behavior_category = Some(responsibility.clone());

            // Generate trait suggestion using behavioral categorization
            use crate::organization::behavioral_decomposition::{
                suggest_trait_extraction, BehavioralCategorizer, MethodCluster,
            };

            let category = BehavioralCategorizer::categorize_method(
                methods.first().map(|s| s.as_str()).unwrap_or(""),
            );

            let cluster = MethodCluster {
                category,
                methods: methods.clone(),
                fields_accessed: vec![], // Will be populated when field tracker is available
                internal_calls: 0,       // Will be populated by call graph analysis
                external_calls: 0,       // Will be populated by call graph analysis
                cohesion_score: 0.0,
            };

            let trait_suggestion = Some(suggest_trait_extraction(&cluster, type_name));

            recommendations.push(ModuleSplit {
                suggested_name: format!(
                    "{}_{}",
                    type_name.to_lowercase(),
                    sanitized_responsibility
                ),
                methods_to_move: methods.clone(),
                structs_to_move: vec![],
                responsibility: responsibility.clone(),
                estimated_lines: methods.len() * 20, // Rough estimate
                method_count: methods.len(),
                warning: None,
                priority: Priority::Medium,
                cohesion_score: None,
                dependencies_in: vec![],
                dependencies_out: vec![],
                domain: String::new(),
                rationale: Some(format!(
                    "Methods grouped by '{}' responsibility pattern",
                    responsibility
                )),
                method: SplitAnalysisMethod::MethodBased,
                severity: None,
                interface_estimate: None,
                classification_evidence,
                representative_methods,
                fields_needed: vec![], // Will be populated by field access analysis when available
                trait_suggestion,
                behavior_category,
                core_type: None,
                data_flow: vec![],
                suggested_type_definition: None,
                data_flow_stage: None,
                pipeline_position: None,
                input_types: vec![],
                output_types: vec![],
                merge_history: vec![],
            });
        }
    }

    recommendations
}

/// Count distinct semantic domains in struct list
pub fn count_distinct_domains(structs: &[StructMetrics]) -> usize {
    use std::collections::HashSet;
    let domains: HashSet<String> = structs
        .iter()
        .map(|s| classify_struct_domain(&s.name))
        .collect();
    domains.len()
}

/// Calculate struct-to-function ratio
pub fn calculate_struct_ratio(struct_count: usize, total_functions: usize) -> f64 {
    if total_functions == 0 {
        return 0.0;
    }
    (struct_count as f64) / (total_functions as f64)
}

/// Determine severity of cross-domain mixing issue
pub fn determine_cross_domain_severity(
    struct_count: usize,
    domain_count: usize,
    lines: usize,
    is_god_object: bool,
) -> RecommendationSeverity {
    // CRITICAL: God object with cross-domain mixing
    if is_god_object && domain_count >= 3 {
        return RecommendationSeverity::Critical;
    }

    // CRITICAL: Massive cross-domain mixing
    if struct_count > 15 && domain_count >= 5 {
        return RecommendationSeverity::Critical;
    }

    // HIGH: Significant cross-domain issues
    if struct_count >= 10 && domain_count >= 4 {
        return RecommendationSeverity::High;
    }

    if lines > 800 && domain_count >= 3 {
        return RecommendationSeverity::High;
    }

    // MEDIUM: Proactive improvement opportunity
    if struct_count >= 8 || lines > 400 {
        return RecommendationSeverity::Medium;
    }

    // LOW: Informational only
    RecommendationSeverity::Low
}

/// Suggest module splits based on struct name patterns (domain-based grouping)
pub fn suggest_module_splits_by_domain(structs: &[StructMetrics]) -> Vec<ModuleSplit> {
    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    let mut line_estimates: HashMap<String, usize> = HashMap::new();
    let mut method_counts: HashMap<String, usize> = HashMap::new();

    for struct_metrics in structs {
        let domain = classify_struct_domain(&struct_metrics.name);
        grouped
            .entry(domain.clone())
            .or_default()
            .push(struct_metrics.name.clone());
        *line_estimates.entry(domain.clone()).or_insert(0) +=
            struct_metrics.line_span.1 - struct_metrics.line_span.0;
        *method_counts.entry(domain).or_insert(0) += struct_metrics.method_count;
    }

    grouped
        .into_iter()
        .filter(|(_, structs)| structs.len() > 1)
        .map(|(domain, structs)| {
            let estimated_lines = line_estimates.get(&domain).copied().unwrap_or(0);
            let method_count = method_counts.get(&domain).copied().unwrap_or(0);
            let suggested_name = format!("config/{}", domain);
            ModuleSplit::validate_name(&suggested_name);
            ModuleSplit {
                suggested_name,
                methods_to_move: vec![],
                structs_to_move: structs,
                responsibility: domain.clone(),
                estimated_lines,
                method_count,
                warning: None,
                priority: Priority::Medium,
                cohesion_score: None,
                dependencies_in: vec![],
                dependencies_out: vec![],
                domain: domain.clone(),
                rationale: Some(format!(
                    "Structs grouped by '{}' domain to improve organization",
                    domain
                )),
                method: SplitAnalysisMethod::CrossDomain,
                severity: None, // Will be set by caller based on overall analysis
                interface_estimate: None,
                classification_evidence: None,
                representative_methods: vec![],
                fields_needed: vec![],
                trait_suggestion: None,
                behavior_category: None,
                core_type: None,
                data_flow: vec![],
                suggested_type_definition: None,
                data_flow_stage: None,
                pipeline_position: None,
                input_types: vec![],
                output_types: vec![],
                merge_history: vec![],
            }
        })
        .collect()
}

/// Classify struct into a domain based on naming patterns
pub fn classify_struct_domain(struct_name: &str) -> String {
    let lower = struct_name.to_lowercase();

    if lower.contains("weight")
        || lower.contains("multiplier")
        || lower.contains("factor")
        || lower.contains("scoring")
    {
        "scoring".to_string()
    } else if lower.contains("threshold") || lower.contains("limit") || lower.contains("bound") {
        "thresholds".to_string()
    } else if lower.contains("detection") || lower.contains("detector") || lower.contains("checker")
    {
        "detection".to_string()
    } else if lower.contains("config") || lower.contains("settings") || lower.contains("options") {
        "core_config".to_string()
    } else if lower.contains("data") || lower.contains("info") || lower.contains("metrics") {
        "data".to_string()
    } else {
        // Extract first meaningful word from struct name as domain
        extract_domain_from_name(struct_name)
    }
}

/// Extract domain name from struct/type name by taking first meaningful word
fn extract_domain_from_name(name: &str) -> String {
    // Handle camelCase and PascalCase
    let mut domain = String::new();
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            break;
        }
        domain.push(c);
    }

    if !domain.is_empty() {
        domain
    } else {
        // Fallback to snake_case extraction
        name.split('_')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Core".to_string())
    }
}

/// Calculate domain diversity metrics from struct metrics (Spec 152).
///
/// Creates struct domain classifications and computes diversity metrics.
pub fn calculate_domain_diversity_from_structs(
    structs: &[StructMetrics],
    is_god_object: bool,
) -> Result<crate::organization::DomainDiversityMetrics, anyhow::Error> {
    use crate::organization::{DomainDiversityMetrics, StructDomainClassification};

    // Create classifications for each struct
    let classifications: Vec<StructDomainClassification> = structs
        .iter()
        .map(|s| {
            let domain = classify_struct_domain(&s.name);
            StructDomainClassification::simple(s.name.clone(), domain)
        })
        .collect();

    // Calculate metrics
    DomainDiversityMetrics::from_struct_classifications(&classifications, is_god_object)
}

/// Data structure for grouping structs with their methods
#[derive(Debug, Clone)]
pub struct StructWithMethods {
    pub name: String,
    pub methods: Vec<String>,
    pub line_span: (usize, usize),
}

/// Reserved keywords across Rust, Python, JavaScript, and TypeScript.
const RESERVED_KEYWORDS: &[&str] = &[
    // Rust
    "mod",
    "pub",
    "use",
    "type",
    "impl",
    "trait",
    "fn",
    "let",
    "mut",
    "const",
    "static",
    "self",
    "Self",
    "super",
    "crate",
    "as",
    "break",
    "continue",
    "else",
    "enum",
    "extern",
    "false",
    "for",
    "if",
    "in",
    "loop",
    "match",
    "move",
    "ref",
    "return",
    "struct",
    "true",
    "unsafe",
    "while",
    "where",
    "async",
    "await",
    "dyn",
    // Python
    "import",
    "from",
    "def",
    "class",
    "if",
    "elif",
    "else",
    "for",
    "while",
    "try",
    "except",
    "finally",
    "with",
    "lambda",
    "yield",
    "return",
    "pass",
    "break",
    "continue",
    "raise",
    "assert",
    "global",
    "nonlocal",
    "del",
    "and",
    "or",
    "not",
    "is",
    "in",
    "None",
    "True",
    "False",
    // JavaScript/TypeScript
    "export",
    "default",
    "function",
    "var",
    "case",
    "catch",
    "debugger",
    "delete",
    "do",
    "new",
    "switch",
    "this",
    "throw",
    "typeof",
    "void",
    "with",
    "arguments",
    "interface",
    "package",
    "private",
    "protected",
    "public",
    "implements",
    "extends",
];

/// Check if a name is a reserved keyword in any supported language.
fn is_reserved_keyword(name: &str) -> bool {
    RESERVED_KEYWORDS.contains(&name)
}

/// Ensure the name is not a reserved keyword by appending "_module" if needed.
fn ensure_not_reserved(mut name: String) -> String {
    if is_reserved_keyword(&name) {
        name.push_str("_module");
    }
    name
}

/// Sanitize module name to be valid across all languages.
///
/// Transforms human-readable responsibility names into valid module identifiers
/// by replacing invalid characters and normalizing whitespace.
///
/// # Character Transformations
///
/// - `&` → `and`
/// - `'` → removed
/// - `-` → `_`
/// - `/` → `_` (except when part of directory path)
/// - Multiple spaces → single `_`
/// - Multiple underscores → single `_`
/// - Leading/trailing underscores removed
/// - Convert to lowercase
/// - Preserve only alphanumeric characters and underscores
///
/// # Examples
///
/// ```
/// # use debtmap::organization::god_object_analysis::sanitize_module_name;
/// assert_eq!(sanitize_module_name("parsing"), "parsing");
/// assert_eq!(sanitize_module_name("Data  Access"), "data_access");
/// assert_eq!(sanitize_module_name("I/O Utilities"), "i_o_utilities");
/// assert_eq!(sanitize_module_name("User's Profile"), "users_profile");
/// assert_eq!(sanitize_module_name("Data-Access-Layer"), "data_access_layer");
/// ```
pub fn sanitize_module_name(name: &str) -> String {
    let sanitized = name
        .to_lowercase()
        .replace('&', "and")
        .replace(['/', '-'], "_")
        .replace('\'', "")
        .replace(' ', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    ensure_not_reserved(sanitized)
}

/// Ensure uniqueness by appending numeric suffix if needed.
///
/// If the name already exists in the set of existing names, appends a numeric
/// suffix starting from 1 until a unique name is found.
///
/// # Arguments
///
/// * `name` - The proposed module name
/// * `existing_names` - Set of already-used names
///
/// # Returns
///
/// A unique name, either the original or with a numeric suffix
///
/// # Examples
///
/// ```
/// # use std::collections::HashSet;
/// # use debtmap::organization::god_object_analysis::ensure_unique_name;
/// let mut existing = HashSet::new();
/// existing.insert("utilities".to_string());
///
/// assert_eq!(ensure_unique_name("utilities".to_string(), &existing), "utilities_1");
///
/// existing.insert("utilities_1".to_string());
/// assert_eq!(ensure_unique_name("utilities".to_string(), &existing), "utilities_2");
/// ```
pub fn ensure_unique_name(
    name: String,
    existing_names: &std::collections::HashSet<String>,
) -> String {
    if !existing_names.contains(&name) {
        return name;
    }

    let mut counter = 1;
    loop {
        let candidate = format!("{}_{}", name, counter);
        if !existing_names.contains(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}

/// Suggest module splits using enhanced struct ownership analysis.
///
/// This function uses the struct ownership analyzer to create more accurate
/// recommendations based on which methods belong to which structs.
///
/// # Arguments
///
/// * `structs` - Per-struct metrics
/// * `ownership` - Struct ownership analyzer (optional for backward compatibility)
/// * `file_path` - Path to the source file (optional, for call graph analysis)
/// * `ast` - Parsed AST (optional, for call graph analysis)
///
/// # Returns
///
/// Vector of validated module splits with cohesion scores and dependencies when available
pub fn suggest_splits_by_struct_grouping(
    structs: &[StructMetrics],
    ownership: Option<&crate::organization::struct_ownership::StructOwnershipAnalyzer>,
    file_path: Option<&std::path::Path>,
    ast: Option<&syn::File>,
) -> Vec<ModuleSplit> {
    use crate::organization::domain_classifier::classify_struct_domain_enhanced;
    use crate::organization::split_validator::validate_and_refine_splits;

    // If no ownership info, fall back to basic domain-based grouping
    let ownership = match ownership {
        Some(o) => o,
        None => return suggest_module_splits_by_domain(structs),
    };

    // Group structs by domain using enhanced classification
    let mut domain_groups: HashMap<String, Vec<StructWithMethods>> = HashMap::new();

    for struct_metrics in structs {
        let methods = ownership.get_struct_methods(&struct_metrics.name);
        let domain = classify_struct_domain_enhanced(&struct_metrics.name, methods);

        domain_groups
            .entry(domain)
            .or_default()
            .push(StructWithMethods {
                name: struct_metrics.name.clone(),
                methods: methods.to_vec(),
                line_span: struct_metrics.line_span,
            });
    }

    // Convert domain groups to module splits
    let splits: Vec<ModuleSplit> = domain_groups
        .into_iter()
        .map(|(domain, structs_with_methods)| {
            let struct_names: Vec<String> = structs_with_methods
                .iter()
                .map(|s| s.name.clone())
                .collect();

            let total_methods: usize = structs_with_methods.iter().map(|s| s.methods.len()).sum();

            let estimated_lines: usize = structs_with_methods
                .iter()
                .map(|s| s.line_span.1.saturating_sub(s.line_span.0))
                .sum();

            ModuleSplit {
                suggested_name: format!("{}_{}", "module", domain),
                methods_to_move: vec![],
                structs_to_move: struct_names,
                responsibility: domain.clone(),
                estimated_lines: estimated_lines.max(total_methods * 15), // Estimate if line_span not available
                method_count: total_methods,
                warning: None,
                priority: Priority::Medium,
                cohesion_score: None,
                dependencies_in: vec![],
                dependencies_out: vec![],
                domain: domain.clone(),
                rationale: Some(format!(
                    "Structs grouped by '{}' domain using struct ownership analysis",
                    domain
                )),
                method: SplitAnalysisMethod::CrossDomain,
                severity: None,
                interface_estimate: None,
                classification_evidence: None,
                representative_methods: vec![],
                fields_needed: vec![],
                trait_suggestion: None,
                behavior_category: None,
                core_type: None,
                data_flow: vec![],
                suggested_type_definition: None,
                data_flow_stage: None,
                pipeline_position: None,
                input_types: vec![],
                output_types: vec![],
                merge_history: vec![],
            }
        })
        .collect();

    // Validate and refine splits (filters too small, splits too large)
    let validated_splits = validate_and_refine_splits(splits);

    // Enhance with cohesion scores and dependencies if ast and file_path are available
    if let (Some(path), Some(ast_file)) = (file_path, ast) {
        use crate::organization::call_graph_cohesion::enhance_splits_with_cohesion;
        enhance_splits_with_cohesion(validated_splits, path, ast_file, ownership)
    } else {
        validated_splits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("format_output"), "output");
        assert_eq!(infer_responsibility_from_method("format_json"), "output");
        assert_eq!(infer_responsibility_from_method("FORMAT_DATA"), "output");
    }

    #[test]
    fn test_render_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("render_table"), "output");
    }

    #[test]
    fn test_write_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("write_to_file"), "output");
    }

    #[test]
    fn test_print_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("print_results"), "output");
    }

    #[test]
    fn test_parse_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("parse_input"), "parsing");
        assert_eq!(infer_responsibility_from_method("parse_json"), "parsing");
    }

    #[test]
    fn test_read_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("read_config"), "parsing");
    }

    #[test]
    fn test_extract_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("extract_data"), "parsing");
    }

    #[test]
    fn test_filter_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("filter_results"),
            "filtering"
        );
    }

    #[test]
    fn test_select_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("select_items"),
            "filtering"
        );
    }

    #[test]
    fn test_find_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("find_element"),
            "filtering"
        );
    }

    #[test]
    fn test_transform_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("transform_data"),
            "transformation"
        );
    }

    #[test]
    fn test_convert_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("convert_to_json"),
            "transformation"
        );
    }

    #[test]
    fn test_map_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("map_values"),
            "transformation"
        );
    }

    #[test]
    fn test_apply_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("apply_mapping"),
            "transformation"
        );
    }

    #[test]
    fn test_get_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("get_value"), "data_access");
    }

    #[test]
    fn test_set_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("set_value"), "data_access");
    }

    #[test]
    fn test_is_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("is_valid"), "validation");
        assert_eq!(infer_responsibility_from_method("is_empty"), "validation");
    }

    #[test]
    fn test_validate_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("validate_input"),
            "validation"
        );
    }

    #[test]
    fn test_check_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("check_constraints"),
            "validation"
        );
    }

    #[test]
    fn test_verify_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("verify_signature"),
            "validation"
        );
    }

    #[test]
    fn test_catch_all_uses_behavioral_categorization() {
        // Spec 178: Avoid "utilities", use behavioral categorization
        // "unknown_function" -> Domain("Unknown") based on first word
        assert_eq!(
            infer_responsibility_from_method("unknown_function"),
            "Unknown"
        );
        // "some_helper" -> Domain("Some") based on first word
        assert_eq!(infer_responsibility_from_method("some_helper"), "Some");
    }

    #[test]
    fn test_responsibility_grouping_not_empty() {
        let methods = vec!["format_a".to_string(), "format_b".to_string()];
        let groups = group_methods_by_responsibility(&methods);
        assert!(!groups.is_empty());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups.get("output").unwrap().len(), 2);
    }

    #[test]
    fn test_multiple_responsibility_groups() {
        let methods = vec![
            "format_output".to_string(),
            "format_json".to_string(),
            "parse_input".to_string(),
            "get_value".to_string(),
            "is_valid".to_string(),
        ];
        let groups = group_methods_by_responsibility(&methods);
        assert_eq!(groups.len(), 4); // Formatting & Output, Parsing & Input, Data Access, Validation
        assert!(groups.contains_key("output"));
        assert!(groups.contains_key("parsing"));
        assert!(groups.contains_key("data_access"));
        assert!(groups.contains_key("validation"));
    }

    #[test]
    fn test_case_insensitive_matching() {
        assert_eq!(infer_responsibility_from_method("FORMAT_OUTPUT"), "output");
        assert_eq!(infer_responsibility_from_method("Parse_Input"), "parsing");
        assert_eq!(infer_responsibility_from_method("IS_VALID"), "validation");
    }

    #[test]
    fn test_calculate_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("calculate_total"),
            "computation"
        );
        assert_eq!(
            infer_responsibility_from_method("calculate_sum"),
            "computation"
        );
    }

    #[test]
    fn test_compute_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("compute_result"),
            "computation"
        );
    }

    #[test]
    fn test_create_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("create_instance"),
            "construction"
        );
    }

    #[test]
    fn test_build_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("build_object"),
            "construction"
        );
    }

    #[test]
    fn test_new_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("new_connection"),
            "construction"
        );
    }

    #[test]
    fn test_save_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("save_to_disk"),
            "persistence"
        );
    }

    #[test]
    fn test_load_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("load_from_file"),
            "persistence"
        );
    }

    #[test]
    fn test_store_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("store_data"),
            "persistence"
        );
    }

    #[test]
    fn test_process_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("process_request"),
            "processing"
        );
    }

    #[test]
    fn test_handle_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("handle_event"),
            "processing"
        );
    }

    #[test]
    fn test_send_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("send_message"),
            "communication"
        );
    }

    #[test]
    fn test_receive_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("receive_data"),
            "communication"
        );
    }

    #[test]
    fn test_empty_string_returns_operations() {
        // Spec 178: Empty string defaults to "Operations" via Domain fallback
        assert_eq!(infer_responsibility_from_method(""), "Operations");
    }

    #[test]
    fn test_underscore_only_returns_operations() {
        // Spec 178: Underscores default to "Operations" via Domain fallback
        assert_eq!(infer_responsibility_from_method("_"), "Operations");
        assert_eq!(infer_responsibility_from_method("__"), "Operations");
    }

    #[test]
    fn test_special_chars_return_first_word_domain() {
        // Spec 178: Special chars extracted as domain name
        assert_eq!(infer_responsibility_from_method("@#$%"), "@#$%");
    }

    #[test]
    fn test_function_is_deterministic() {
        let input = "calculate_average";
        let result1 = infer_responsibility_from_method(input);
        let result2 = infer_responsibility_from_method(input);
        assert_eq!(result1, result2);
    }

    // Spec 134: Tests for metric validation
    #[test]
    fn test_validation_passes_consistent_metrics() {
        let analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 3,
            field_count: 5,
            responsibility_count: 2,
            lines_of_code: 100,
            complexity_sum: 20,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data_access".to_string(), "validation".to_string()],
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            visibility_breakdown: Some(FunctionVisibilityBreakdown {
                public: 1,
                pub_crate: 1,
                pub_super: 0,
                private: 1,
            }),
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
        };

        assert!(analysis.validate().is_ok());
    }

    #[test]
    fn test_validation_detects_visibility_mismatch() {
        let analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 10,
            field_count: 5,
            responsibility_count: 2,
            lines_of_code: 100,
            complexity_sum: 20,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data_access".to_string(), "validation".to_string()],
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            visibility_breakdown: Some(FunctionVisibilityBreakdown {
                public: 2,
                pub_crate: 1,
                pub_super: 0,
                private: 1,
            }),
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
        };

        let result = analysis.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MetricInconsistency::VisibilityMismatch { .. }
        ));
    }

    #[test]
    fn test_validation_detects_responsibility_count_mismatch() {
        let analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 3,
            field_count: 5,
            responsibility_count: 5, // Says 5 but only provides 2 names
            lines_of_code: 100,
            complexity_sum: 20,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data_access".to_string(), "validation".to_string()],
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            visibility_breakdown: Some(FunctionVisibilityBreakdown {
                public: 1,
                pub_crate: 1,
                pub_super: 0,
                private: 1,
            }),
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
        };

        let result = analysis.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MetricInconsistency::ResponsibilityCountMismatch { .. }
        ));
    }

    #[test]
    fn test_validation_detects_missing_responsibilities() {
        let analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 10, // Has methods
            field_count: 5,
            responsibility_count: 0,
            lines_of_code: 100,
            complexity_sum: 20,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec![], // But no responsibilities
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            visibility_breakdown: Some(FunctionVisibilityBreakdown {
                public: 5,
                pub_crate: 3,
                pub_super: 0,
                private: 2,
            }),
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
        };

        let result = analysis.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MetricInconsistency::MissingResponsibilities { .. }
        ));
    }

    #[test]
    fn test_function_visibility_breakdown_total() {
        let breakdown = FunctionVisibilityBreakdown {
            public: 5,
            pub_crate: 3,
            pub_super: 2,
            private: 10,
        };
        assert_eq!(breakdown.total(), 20);
    }

    #[test]
    fn test_count_distinct_domains() {
        let structs = vec![
            StructMetrics {
                name: "ThresholdConfig".to_string(),
                line_span: (0, 10),
                method_count: 2,
                field_count: 5,
                responsibilities: vec!["configuration".to_string()],
            },
            StructMetrics {
                name: "ThresholdValidator".to_string(),
                line_span: (11, 20),
                method_count: 3,
                field_count: 4,
                responsibilities: vec!["validation".to_string()],
            },
            StructMetrics {
                name: "ScoringWeight".to_string(),
                line_span: (21, 30),
                method_count: 4,
                field_count: 3,
                responsibilities: vec!["calculation".to_string()],
            },
            StructMetrics {
                name: "ScoringMultiplier".to_string(),
                line_span: (31, 40),
                method_count: 2,
                field_count: 6,
                responsibilities: vec!["configuration".to_string()],
            },
        ];

        // Should identify 2 domains: "thresholds" and "scoring"
        assert_eq!(count_distinct_domains(&structs), 2);
    }

    #[test]
    fn test_count_distinct_domains_single() {
        let structs = vec![
            StructMetrics {
                name: "ConfigA".to_string(),
                line_span: (0, 10),
                method_count: 2,
                field_count: 5,
                responsibilities: vec!["configuration".to_string()],
            },
            StructMetrics {
                name: "ConfigB".to_string(),
                line_span: (11, 20),
                method_count: 3,
                field_count: 4,
                responsibilities: vec!["configuration".to_string()],
            },
        ];

        // Should identify 1 domain: "config"
        assert_eq!(count_distinct_domains(&structs), 1);
    }

    #[test]
    fn test_calculate_struct_ratio() {
        // Normal case
        assert_eq!(calculate_struct_ratio(10, 20), 0.5);

        // More structs than functions
        assert_eq!(calculate_struct_ratio(15, 10), 1.5);

        // Single struct
        assert_eq!(calculate_struct_ratio(1, 10), 0.1);
    }

    #[test]
    fn test_calculate_struct_ratio_edge_cases() {
        // Zero functions should return 0.0 to avoid division by zero
        assert_eq!(calculate_struct_ratio(10, 0), 0.0);

        // Zero structs
        assert_eq!(calculate_struct_ratio(0, 10), 0.0);

        // Both zero
        assert_eq!(calculate_struct_ratio(0, 0), 0.0);
    }

    #[test]
    fn test_determine_cross_domain_severity_critical() {
        // Critical: God object with cross-domain mixing
        assert!(matches!(
            determine_cross_domain_severity(10, 3, 600, true),
            RecommendationSeverity::Critical
        ));

        // Critical: Massive cross-domain mixing
        assert!(matches!(
            determine_cross_domain_severity(16, 5, 400, false),
            RecommendationSeverity::Critical
        ));
    }

    #[test]
    fn test_determine_cross_domain_severity_high() {
        // High: Significant cross-domain issues
        assert!(matches!(
            determine_cross_domain_severity(10, 4, 500, false),
            RecommendationSeverity::High
        ));

        // High: Large file with multiple domains
        assert!(matches!(
            determine_cross_domain_severity(8, 3, 850, false),
            RecommendationSeverity::High
        ));
    }

    #[test]
    fn test_determine_cross_domain_severity_medium() {
        // Medium: Proactive improvement opportunity (8+ structs)
        assert!(matches!(
            determine_cross_domain_severity(8, 2, 300, false),
            RecommendationSeverity::Medium
        ));

        // Medium: Larger file
        assert!(matches!(
            determine_cross_domain_severity(6, 2, 450, false),
            RecommendationSeverity::Medium
        ));
    }

    #[test]
    fn test_determine_cross_domain_severity_low() {
        // Low: Small file with few structs
        assert!(matches!(
            determine_cross_domain_severity(5, 2, 200, false),
            RecommendationSeverity::Low
        ));

        // Low: Minimal mixing
        assert!(matches!(
            determine_cross_domain_severity(3, 2, 150, false),
            RecommendationSeverity::Low
        ));
    }

    #[test]
    fn test_struct_heavy_detection() {
        // Struct-heavy: 8 structs, 10 functions, ratio = 0.8
        let ratio = calculate_struct_ratio(8, 10);
        assert!(ratio > 0.3);

        // Not struct-heavy: 3 structs, 20 functions, ratio = 0.15
        let ratio = calculate_struct_ratio(3, 20);
        assert!(ratio < 0.3);

        // Edge case: Exactly at threshold
        let ratio = calculate_struct_ratio(5, 15);
        assert_eq!(ratio, 5.0 / 15.0);
    }

    // Tests for I/O-based responsibility detection (Spec 141)
    #[test]
    fn test_io_detection_file_io() {
        use crate::analysis::io_detection::Language;

        let method_name = "load_config";
        let method_body = r#"
            fn load_config() -> String {
                std::fs::read_to_string("config.toml").unwrap()
            }
        "#;

        let responsibility =
            infer_responsibility_with_io_detection(method_name, Some(method_body), Language::Rust);

        assert_eq!(responsibility, "File I/O");
    }

    #[test]
    fn test_io_detection_network_io() {
        use crate::analysis::io_detection::Language;

        let method_name = "fetch_data";
        let method_body = r#"
            fn fetch_data() {
                let client = reqwest::blocking::Client::new();
                let response = client.get("https://api.example.com").send();
            }
        "#;

        let responsibility =
            infer_responsibility_with_io_detection(method_name, Some(method_body), Language::Rust);

        assert_eq!(responsibility, "Network I/O");
    }

    #[test]
    fn test_io_detection_console_io() {
        use crate::analysis::io_detection::Language;

        let method_name = "display_results";
        let method_body = r#"
            fn display_results(data: &str) {
                println!("Results: {}", data);
            }
        "#;

        let responsibility =
            infer_responsibility_with_io_detection(method_name, Some(method_body), Language::Rust);

        assert_eq!(responsibility, "Console I/O");
    }

    #[test]
    fn test_io_detection_pure_computation() {
        use crate::analysis::io_detection::Language;

        let method_name = "calculate_sum";
        let method_body = r#"
            fn calculate_sum(a: i32, b: i32) -> i32 {
                a + b
            }
        "#;

        let responsibility =
            infer_responsibility_with_io_detection(method_name, Some(method_body), Language::Rust);

        // Pure functions fall back to name-based heuristics
        assert_eq!(responsibility, "computation");
    }

    #[test]
    fn test_io_detection_fallback_to_name() {
        use crate::analysis::io_detection::Language;

        let method_name = "format_output";
        let method_body = None; // No body provided

        let responsibility =
            infer_responsibility_with_io_detection(method_name, method_body, Language::Rust);

        // Without body, falls back to name-based detection
        assert_eq!(responsibility, "output");
    }

    #[test]
    fn test_io_detection_python_file_io() {
        use crate::analysis::io_detection::Language;

        let method_name = "read_data";
        let method_body = r#"
            def read_data():
                with open('data.json') as f:
                    return f.read()
        "#;

        let responsibility = infer_responsibility_with_io_detection(
            method_name,
            Some(method_body),
            Language::Python,
        );

        assert_eq!(responsibility, "File I/O");
    }

    #[test]
    fn test_io_detection_javascript_network() {
        use crate::analysis::io_detection::Language;

        let method_name = "getData";
        let method_body = r#"
            async function getData() {
                const response = await fetch('https://api.example.com');
                return await response.json();
            }
        "#;

        let responsibility = infer_responsibility_with_io_detection(
            method_name,
            Some(method_body),
            Language::JavaScript,
        );

        assert_eq!(responsibility, "Network I/O");
    }

    #[test]
    fn test_map_io_to_traditional_responsibility() {
        assert_eq!(
            map_io_to_traditional_responsibility("File I/O"),
            "persistence"
        );
        assert_eq!(
            map_io_to_traditional_responsibility("Network I/O"),
            "persistence"
        );
        assert_eq!(
            map_io_to_traditional_responsibility("Database I/O"),
            "persistence"
        );
        assert_eq!(
            map_io_to_traditional_responsibility("Console I/O"),
            "output"
        );
        assert_eq!(
            map_io_to_traditional_responsibility("Mixed I/O"),
            "processing"
        );
    }

    // Tests for call pattern-based responsibility detection (Spec 137)
    #[test]
    fn test_call_pattern_support_detection() {
        let function_name = "escape_html";
        let callees = vec![];
        let callers = vec!["format_output".to_string(), "render_table".to_string()];

        let resp = infer_responsibility_from_call_patterns(function_name, &callees, &callers);
        assert_eq!(resp, Some("output Support".to_string()));
    }

    #[test]
    fn test_call_pattern_orchestration_detection() {
        let function_name = "process_data";
        let callees = vec![
            "validate_input".to_string(),
            "check_bounds".to_string(),
            "verify_format".to_string(),
        ];
        let callers = vec![];

        let resp = infer_responsibility_from_call_patterns(function_name, &callees, &callers);
        assert_eq!(resp, Some("validation Orchestration".to_string()));
    }

    #[test]
    fn test_call_pattern_utility_detection() {
        let function_name = "helper_function";
        let callees = vec![];
        let callers = vec![
            "func1".to_string(),
            "func2".to_string(),
            "func3".to_string(),
            "func4".to_string(),
        ];

        let resp = infer_responsibility_from_call_patterns(function_name, &callees, &callers);
        assert_eq!(resp, Some("utilities".to_string()));
    }

    #[test]
    fn test_call_pattern_no_clear_pattern() {
        let function_name = "mixed_function";
        let callees = vec!["func1".to_string()];
        let callers = vec!["func2".to_string()];

        let resp = infer_responsibility_from_call_patterns(function_name, &callees, &callers);
        assert_eq!(resp, None);
    }

    #[test]
    fn test_categorize_functions() {
        let functions = vec![
            "format_output".to_string(),
            "format_json".to_string(),
            "parse_input".to_string(),
            "validate_data".to_string(),
        ];

        let categories = categorize_functions(&functions);
        assert_eq!(categories.get("output"), Some(&2));
        assert_eq!(categories.get("parsing"), Some(&1));
        assert_eq!(categories.get("validation"), Some(&1));
    }

    #[test]
    fn test_find_dominant_category() {
        let mut categories = std::collections::HashMap::new();
        categories.insert("output".to_string(), 5);
        categories.insert("parsing".to_string(), 2);
        categories.insert("validation".to_string(), 1);

        let dominant = find_dominant_category(&categories);
        assert_eq!(dominant, Some(("output".to_string(), 5)));
    }

    #[test]
    fn test_split_names_have_no_extensions() {
        // Valid names without extensions should pass
        ModuleSplit::validate_name("config/misc");
        ModuleSplit::validate_name("module_name");
        ModuleSplit::validate_name("some/path/to/module");
    }

    #[test]
    #[should_panic(expected = "should not include file extension")]
    fn test_split_name_validation_catches_rs_extension() {
        ModuleSplit::validate_name("config/misc.rs");
    }

    #[test]
    #[should_panic(expected = "should not include file extension")]
    fn test_split_name_validation_catches_py_extension() {
        ModuleSplit::validate_name("module.py");
    }

    #[test]
    #[should_panic(expected = "should not include file extension")]
    fn test_split_name_validation_catches_js_extension() {
        ModuleSplit::validate_name("handler.js");
    }

    #[test]
    #[should_panic(expected = "should not include file extension")]
    fn test_split_name_validation_catches_ts_extension() {
        ModuleSplit::validate_name("component.ts");
    }

    // Tests for module name sanitization (Spec 172)
    #[test]
    fn test_sanitize_ampersand_replacement() {
        assert_eq!(sanitize_module_name("parsing"), "parsing");
        assert_eq!(sanitize_module_name("Read & Write"), "read_and_write");
        assert_eq!(
            sanitize_module_name("data_access & validation"),
            "data_access_and_validation"
        );
    }

    #[test]
    fn test_sanitize_multiple_spaces() {
        assert_eq!(sanitize_module_name("data  access"), "data_access");
        // I/O → i_o (slash is converted to underscore, preserving letter boundaries)
        assert_eq!(sanitize_module_name("I/O   utilities"), "i_o_utilities");
        assert_eq!(
            sanitize_module_name("formatting    &    output"),
            "formatting_and_output"
        );
    }

    #[test]
    fn test_sanitize_special_characters() {
        assert_eq!(sanitize_module_name("User's Profile"), "users_profile");
        assert_eq!(
            sanitize_module_name("Data-Access-Layer"),
            "data_access_layer"
        );
        assert_eq!(sanitize_module_name("Config/Settings"), "config_settings");
        // I/O → i_o (slash is converted to underscore, preserving letter boundaries)
        assert_eq!(sanitize_module_name("I/O Utilities"), "i_o_utilities");
    }

    #[test]
    fn test_sanitize_leading_trailing_underscores() {
        assert_eq!(sanitize_module_name("_utilities_"), "utilities");
        assert_eq!(sanitize_module_name("__internal__"), "internal");
        assert_eq!(sanitize_module_name("_data_access_"), "data_access");
    }

    #[test]
    fn test_sanitize_empty_and_whitespace() {
        assert_eq!(sanitize_module_name(""), "");
        assert_eq!(sanitize_module_name("   "), "");
        assert_eq!(sanitize_module_name("___"), "");
    }

    #[test]
    fn test_sanitize_consecutive_underscores() {
        assert_eq!(sanitize_module_name("data__access"), "data_access");
        assert_eq!(
            sanitize_module_name("multiple___underscores"),
            "multiple_underscores"
        );
    }

    #[test]
    fn test_sanitize_mixed_case() {
        assert_eq!(sanitize_module_name("MixedCase"), "mixedcase");
        assert_eq!(sanitize_module_name("CamelCase"), "camelcase");
        assert_eq!(sanitize_module_name("UPPERCASE"), "uppercase");
    }

    #[test]
    fn test_sanitize_numbers() {
        assert_eq!(sanitize_module_name("version2"), "version2");
        assert_eq!(sanitize_module_name("data_v2"), "data_v2");
        assert_eq!(sanitize_module_name("test123"), "test123");
    }

    #[test]
    fn test_sanitize_all_special_chars() {
        assert_eq!(
            sanitize_module_name("@#$%^*()!~`"),
            "" // All special chars removed
        );
        assert_eq!(sanitize_module_name("data@access"), "dataaccess");
    }

    #[test]
    fn test_sanitize_real_world_examples() {
        // From spec - real-world example (updated for simplified names)
        assert_eq!(sanitize_module_name("parsing"), "parsing");
        assert_eq!(sanitize_module_name("data_access"), "data_access");
        assert_eq!(sanitize_module_name("utilities"), "utilities");
        assert_eq!(sanitize_module_name("output"), "output");
    }

    #[test]
    fn test_sanitize_already_valid_names() {
        // Names that are already valid should remain unchanged (except lowercase)
        assert_eq!(sanitize_module_name("utilities"), "utilities");
        assert_eq!(sanitize_module_name("data_access"), "data_access");
        assert_eq!(sanitize_module_name("io_handler"), "io_handler");
    }

    #[test]
    fn test_sanitize_complex_combinations() {
        assert_eq!(
            sanitize_module_name("User's Data & Config Settings"),
            "users_data_and_config_settings"
        );
        // I/O → i_o (slash is converted to underscore, preserving letter boundaries)
        assert_eq!(
            sanitize_module_name("I/O - Read & Write"),
            "i_o_read_and_write"
        );
    }

    #[test]
    fn test_sanitize_deterministic() {
        // Same input should always produce same output
        let input = "parsing";
        let result1 = sanitize_module_name(input);
        let result2 = sanitize_module_name(input);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_reserved_keyword_rust() {
        assert_eq!(ensure_not_reserved("mod".to_string()), "mod_module");
        assert_eq!(ensure_not_reserved("type".to_string()), "type_module");
        assert_eq!(ensure_not_reserved("impl".to_string()), "impl_module");
        assert_eq!(ensure_not_reserved("trait".to_string()), "trait_module");
    }

    #[test]
    fn test_reserved_keyword_python() {
        assert_eq!(ensure_not_reserved("import".to_string()), "import_module");
        assert_eq!(ensure_not_reserved("class".to_string()), "class_module");
        assert_eq!(ensure_not_reserved("def".to_string()), "def_module");
    }

    #[test]
    fn test_reserved_keyword_javascript() {
        assert_eq!(
            ensure_not_reserved("function".to_string()),
            "function_module"
        );
        assert_eq!(ensure_not_reserved("export".to_string()), "export_module");
        assert_eq!(ensure_not_reserved("const".to_string()), "const_module");
    }

    #[test]
    fn test_reserved_keyword_not_reserved() {
        assert_eq!(ensure_not_reserved("utilities".to_string()), "utilities");
        assert_eq!(ensure_not_reserved("data".to_string()), "data");
        assert_eq!(
            ensure_not_reserved("my_function".to_string()),
            "my_function"
        );
    }

    #[test]
    fn test_is_reserved_keyword() {
        assert!(is_reserved_keyword("mod"));
        assert!(is_reserved_keyword("import"));
        assert!(is_reserved_keyword("function"));
        assert!(!is_reserved_keyword("utilities"));
        assert!(!is_reserved_keyword("data_access"));
    }

    #[test]
    fn test_ensure_unique_name_no_collision() {
        use std::collections::HashSet;
        let existing = HashSet::new();
        assert_eq!(
            ensure_unique_name("utilities".to_string(), &existing),
            "utilities"
        );
    }

    #[test]
    fn test_ensure_unique_name_single_collision() {
        use std::collections::HashSet;
        let mut existing = HashSet::new();
        existing.insert("utilities".to_string());

        assert_eq!(
            ensure_unique_name("utilities".to_string(), &existing),
            "utilities_1"
        );
    }

    #[test]
    fn test_ensure_unique_name_multiple_collisions() {
        use std::collections::HashSet;
        let mut existing = HashSet::new();
        existing.insert("utilities".to_string());
        existing.insert("utilities_1".to_string());
        existing.insert("utilities_2".to_string());

        assert_eq!(
            ensure_unique_name("utilities".to_string(), &existing),
            "utilities_3"
        );
    }

    #[test]
    fn test_ensure_unique_name_deterministic() {
        use std::collections::HashSet;
        let mut existing = HashSet::new();
        existing.insert("data".to_string());

        let result1 = ensure_unique_name("data".to_string(), &existing);
        let result2 = ensure_unique_name("data".to_string(), &existing);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_sanitize_no_valid_characters() {
        // When all characters are removed, should result in empty string
        assert_eq!(sanitize_module_name("@#$%"), "");
        assert_eq!(sanitize_module_name("!!!"), "");
    }

    #[test]
    fn test_sanitize_unicode_characters() {
        // Unicode emojis should be filtered out
        assert_eq!(sanitize_module_name("data_🔥_access"), "data_access");
        // Unicode letters (like é) are preserved by is_alphanumeric()
        assert_eq!(sanitize_module_name("café"), "café");
    }

    #[test]
    fn test_sanitize_single_character() {
        assert_eq!(sanitize_module_name("a"), "a");
        // & → and → and_module (since "and" is a Python reserved keyword)
        assert_eq!(sanitize_module_name("&"), "and_module");
        assert_eq!(sanitize_module_name("1"), "1");
    }

    #[test]
    fn test_sanitize_very_long_name() {
        let long_name =
            "This Is A Very Long Module Name With Many Words And Special Characters & Symbols";
        let result = sanitize_module_name(long_name);
        assert!(!result.contains("  "));
        assert!(!result.contains("&"));
        assert!(!result.starts_with('_'));
        assert!(!result.ends_with('_'));
    }

    #[test]
    fn test_sanitize_preserves_alphanumeric() {
        assert_eq!(sanitize_module_name("abc123xyz"), "abc123xyz");
        assert_eq!(sanitize_module_name("test_123_data"), "test_123_data");
    }

    #[test]
    fn test_sanitize_no_consecutive_underscores_in_output() {
        let result = sanitize_module_name("data___access");
        assert!(!result.contains("__"));

        let result = sanitize_module_name("multiple   spaces");
        assert!(!result.contains("__"));
    }

    #[test]
    fn test_sanitize_integration_with_module_split() {
        // Test that sanitized names work in real module split creation
        let responsibility = "parsing";
        let sanitized = sanitize_module_name(responsibility);
        let module_name = format!("mytype_{}", sanitized);

        assert_eq!(module_name, "mytype_parsing");
        assert!(!module_name.contains('&'));
        assert!(!module_name.contains("  "));
    }

    #[test]
    fn test_recommend_module_splits_uses_sanitization() {
        let mut responsibility_groups = HashMap::new();
        responsibility_groups.insert(
            "parsing".to_string(),
            vec![
                "parse_a".to_string(),
                "parse_b".to_string(),
                "parse_c".to_string(),
                "parse_d".to_string(),
                "parse_e".to_string(),
                "parse_f".to_string(),
            ],
        );

        let splits = recommend_module_splits("MyType", &[], &responsibility_groups);

        assert_eq!(splits.len(), 1);
        assert_eq!(splits[0].suggested_name, "mytype_parsing");
        assert!(!splits[0].suggested_name.contains('&'));
    }
}

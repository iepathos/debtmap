use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of god object detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectionType {
    /// Single struct with excessive methods (tests excluded from counts)
    GodClass,
    /// File with excessive functions or lines (tests included in counts)
    GodFile,
    /// Alias for GodFile
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleSplit {
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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

    // Ensure minimum score of 100 for any god object
    if violation_count > 0 {
        // For any god object (at least 1 violation), ensure minimum 100 points
        // Scale up based on severity of violations
        let min_score = 100.0;
        let severity_multiplier = violation_count as f64;
        (base_score * 50.0 * severity_multiplier).max(min_score)
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

    // Apply complexity factor and ensure minimum score for violations
    if violation_count > 0 {
        let min_score = 100.0;
        let severity_multiplier = violation_count as f64;
        (base_score * 50.0 * complexity_factor * severity_multiplier).max(min_score)
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

/// Infer responsibility category from function/method name.
///
/// This function uses common naming patterns to categorize functions into
/// responsibility groups. It recognizes standard Rust function prefixes like
/// `format_*`, `parse_*`, `filter_*`, etc.
///
/// # Pattern Recognition
///
/// - `format_*`, `render_*`, `write_*`, `print_*` → "Formatting & Output"
/// - `parse_*`, `read_*`, `extract_*` → "Parsing & Input"
/// - `filter_*`, `select_*`, `find_*` → "Filtering & Selection"
/// - `transform_*`, `convert_*`, `map_*`, `apply_*` → "Transformation"
/// - `get_*`, `set_*` → "Data Access"
/// - `validate_*`, `check_*`, `verify_*`, `is_*` → "Validation"
/// - `calculate_*`, `compute_*` → "Computation"
/// - `create_*`, `build_*`, `new_*` → "Construction"
/// - `save_*`, `load_*`, `store_*` → "Persistence"
/// - `process_*`, `handle_*` → "Processing"
/// - `send_*`, `receive_*` → "Communication"
/// - Everything else → "Utilities"
///
/// # Extending Patterns
///
/// To add new patterns:
/// 1. Add a new `else if` clause with the prefix check
/// 2. Choose a descriptive category name
/// 3. Add a unit test for the new pattern
/// 4. Update this documentation
///
/// # Note on Catch-all
///
/// The catch-all category is "Utilities" rather than "Core Operations" because
/// it more accurately describes miscellaneous helper functions that don't fit
/// standard naming conventions.
fn infer_responsibility_from_method(method_name: &str) -> String {
    let lower = method_name.to_lowercase();

    // Formatting & Output
    if lower.starts_with("format")
        || lower.starts_with("render")
        || lower.starts_with("write")
        || lower.starts_with("print")
    {
        "Formatting & Output".to_string()
    }
    // Parsing & Input
    else if lower.starts_with("parse")
        || lower.starts_with("read")
        || lower.starts_with("extract")
    {
        "Parsing & Input".to_string()
    }
    // Filtering & Selection
    else if lower.starts_with("filter")
        || lower.starts_with("select")
        || lower.starts_with("find")
    {
        "Filtering & Selection".to_string()
    }
    // Transformation
    else if lower.starts_with("transform")
        || lower.starts_with("convert")
        || lower.starts_with("map")
        || lower.starts_with("apply")
    {
        "Transformation".to_string()
    }
    // Data Access (existing)
    else if lower.starts_with("get") || lower.starts_with("set") {
        "Data Access".to_string()
    }
    // Validation (existing, enhanced with 'is_' prefix)
    else if lower.starts_with("validate")
        || lower.starts_with("check")
        || lower.starts_with("verify")
        || lower.starts_with("is")
    {
        "Validation".to_string()
    }
    // Computation (existing)
    else if lower.starts_with("calculate") || lower.starts_with("compute") {
        "Computation".to_string()
    }
    // Construction (existing)
    else if lower.starts_with("create") || lower.starts_with("build") || lower.starts_with("new")
    {
        "Construction".to_string()
    }
    // Persistence (existing)
    else if lower.starts_with("save") || lower.starts_with("load") || lower.starts_with("store") {
        "Persistence".to_string()
    }
    // Processing (existing)
    else if lower.starts_with("process") || lower.starts_with("handle") {
        "Processing".to_string()
    }
    // Communication (existing)
    else if lower.starts_with("send") || lower.starts_with("receive") {
        "Communication".to_string()
    }
    // Utilities (renamed from "Core Operations")
    else {
        "Utilities".to_string()
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
    let mut recommendations = Vec::new();

    for (responsibility, methods) in responsibility_groups {
        if methods.len() > 5 {
            recommendations.push(ModuleSplit {
                suggested_name: format!(
                    "{}_{}",
                    type_name.to_lowercase(),
                    responsibility.to_lowercase().replace(' ', "_")
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
            ModuleSplit {
                suggested_name: format!("config/{}.rs", domain),
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
        "misc".to_string()
    }
}

/// Data structure for grouping structs with their methods
#[derive(Debug, Clone)]
pub struct StructWithMethods {
    pub name: String,
    pub methods: Vec<String>,
    pub line_span: (usize, usize),
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
        assert_eq!(
            infer_responsibility_from_method("format_output"),
            "Formatting & Output"
        );
        assert_eq!(
            infer_responsibility_from_method("format_json"),
            "Formatting & Output"
        );
        assert_eq!(
            infer_responsibility_from_method("FORMAT_DATA"),
            "Formatting & Output"
        );
    }

    #[test]
    fn test_render_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("render_table"),
            "Formatting & Output"
        );
    }

    #[test]
    fn test_write_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("write_to_file"),
            "Formatting & Output"
        );
    }

    #[test]
    fn test_print_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("print_results"),
            "Formatting & Output"
        );
    }

    #[test]
    fn test_parse_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("parse_input"),
            "Parsing & Input"
        );
        assert_eq!(
            infer_responsibility_from_method("parse_json"),
            "Parsing & Input"
        );
    }

    #[test]
    fn test_read_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("read_config"),
            "Parsing & Input"
        );
    }

    #[test]
    fn test_extract_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("extract_data"),
            "Parsing & Input"
        );
    }

    #[test]
    fn test_filter_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("filter_results"),
            "Filtering & Selection"
        );
    }

    #[test]
    fn test_select_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("select_items"),
            "Filtering & Selection"
        );
    }

    #[test]
    fn test_find_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("find_element"),
            "Filtering & Selection"
        );
    }

    #[test]
    fn test_transform_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("transform_data"),
            "Transformation"
        );
    }

    #[test]
    fn test_convert_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("convert_to_json"),
            "Transformation"
        );
    }

    #[test]
    fn test_map_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("map_values"),
            "Transformation"
        );
    }

    #[test]
    fn test_apply_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("apply_mapping"),
            "Transformation"
        );
    }

    #[test]
    fn test_get_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("get_value"), "Data Access");
    }

    #[test]
    fn test_set_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("set_value"), "Data Access");
    }

    #[test]
    fn test_is_prefix_recognized() {
        assert_eq!(infer_responsibility_from_method("is_valid"), "Validation");
        assert_eq!(infer_responsibility_from_method("is_empty"), "Validation");
    }

    #[test]
    fn test_validate_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("validate_input"),
            "Validation"
        );
    }

    #[test]
    fn test_check_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("check_constraints"),
            "Validation"
        );
    }

    #[test]
    fn test_verify_prefix_recognized() {
        assert_eq!(
            infer_responsibility_from_method("verify_signature"),
            "Validation"
        );
    }

    #[test]
    fn test_catch_all_renamed_to_utilities() {
        assert_eq!(
            infer_responsibility_from_method("unknown_function"),
            "Utilities"
        );
        assert_eq!(infer_responsibility_from_method("some_helper"), "Utilities");
    }

    #[test]
    fn test_responsibility_grouping_not_empty() {
        let methods = vec!["format_a".to_string(), "format_b".to_string()];
        let groups = group_methods_by_responsibility(&methods);
        assert!(!groups.is_empty());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups.get("Formatting & Output").unwrap().len(), 2);
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
        assert!(groups.contains_key("Formatting & Output"));
        assert!(groups.contains_key("Parsing & Input"));
        assert!(groups.contains_key("Data Access"));
        assert!(groups.contains_key("Validation"));
    }

    #[test]
    fn test_case_insensitive_matching() {
        assert_eq!(
            infer_responsibility_from_method("FORMAT_OUTPUT"),
            "Formatting & Output"
        );
        assert_eq!(
            infer_responsibility_from_method("Parse_Input"),
            "Parsing & Input"
        );
        assert_eq!(infer_responsibility_from_method("IS_VALID"), "Validation");
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
            responsibilities: vec!["Data Access".to_string(), "Validation".to_string()],
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
            responsibilities: vec!["Data Access".to_string(), "Validation".to_string()],
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
            responsibilities: vec!["Data Access".to_string(), "Validation".to_string()],
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
        assert!(8 >= 5);

        // Not struct-heavy: 3 structs, 20 functions, ratio = 0.15
        let ratio = calculate_struct_ratio(3, 20);
        assert!(ratio < 0.3);

        // Edge case: Exactly at threshold
        let ratio = calculate_struct_ratio(5, 15);
        assert_eq!(ratio, 5.0 / 15.0);
        assert!(5 >= 5);
    }
}

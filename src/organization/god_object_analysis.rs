use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectAnalysis {
    pub is_god_object: bool,
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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Priority {
    High,
    #[default]
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

fn infer_responsibility_from_method(method_name: &str) -> String {
    let lower = method_name.to_lowercase();

    if lower.starts_with("get") || lower.starts_with("set") {
        "Data Access".to_string()
    } else if lower.starts_with("calculate") || lower.starts_with("compute") {
        "Computation".to_string()
    } else if lower.starts_with("validate")
        || lower.starts_with("check")
        || lower.starts_with("verify")
    {
        "Validation".to_string()
    } else if lower.starts_with("save") || lower.starts_with("load") || lower.starts_with("store") {
        "Persistence".to_string()
    } else if lower.starts_with("create") || lower.starts_with("build") || lower.starts_with("new")
    {
        "Construction".to_string()
    } else if lower.starts_with("send")
        || lower.starts_with("receive")
        || lower.starts_with("handle")
    {
        "Communication".to_string()
    } else if lower.starts_with("process") || lower.starts_with("transform") {
        "Processing".to_string()
    } else {
        "Core Operations".to_string()
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
            });
        }
    }

    recommendations
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
                responsibility: domain,
                estimated_lines: estimated_lines.max(total_methods * 15), // Estimate if line_span not available
                method_count: total_methods,
                warning: None,
                priority: Priority::Medium,
                cohesion_score: None,
                dependencies_in: vec![],
                dependencies_out: vec![],
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

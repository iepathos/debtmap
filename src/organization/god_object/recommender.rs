//! # God Object Recommendation Generation (Pure Functions)
//!
//! Pure functions for generating refactoring recommendations for god objects.
//! This module contains the logic for:
//! - Suggesting module splits based on domain analysis
//! - Generating enhanced recommendations with confidence and rationale
//! - Determining severity of cross-domain issues
//! - Sanitizing and ensuring unique module names
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - functions that perform transformations
//! without side effects.

use std::collections::{HashMap, HashSet};

use super::classifier::classify_struct_domain;
use super::split_types::ModuleSplit;
use super::thresholds::ensure_not_reserved;
use super::types::*;

/// Suggest module splits based on struct name patterns (domain-based grouping).
///
/// Groups structs by domain and creates split recommendations for groups with
/// more than one struct.
///
/// # Arguments
///
/// * `structs` - Slice of struct metrics to analyze
///
/// # Returns
///
/// Vector of module split recommendations grouped by domain
///
/// # Examples
///
/// ```no_run
/// use debtmap::organization::god_object::recommender::suggest_module_splits_by_domain;
/// use debtmap::organization::god_object::types::StructMetrics;
///
/// let structs = vec![
///     StructMetrics {
///         name: "ConfigOption".to_string(),
///         line_span: (10, 20),
///         method_count: 5,
///         field_count: 3,
///         responsibilities: vec![],
///     },
///     StructMetrics {
///         name: "ConfigParser".to_string(),
///         line_span: (30, 50),
///         method_count: 8,
///         field_count: 2,
///         responsibilities: vec![],
///     },
/// ];
/// let splits = suggest_module_splits_by_domain(&structs);
/// ```
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
                alternative_names: vec![],
                naming_confidence: None,
                naming_strategy: None,
                cluster_quality: None,
            }
        })
        .collect()
}

/// Generate basic recommendations from responsibility groups.
///
/// This is a simplified version that delegates to the evidence-based version
/// with an empty evidence map for backward compatibility.
///
/// # Arguments
///
/// * `type_name` - Name of the type being analyzed
/// * `_methods` - All methods (kept for API compatibility)
/// * `responsibility_groups` - Methods grouped by responsibility
///
/// # Returns
///
/// Vector of module split recommendations
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

/// Generate recommendations with evidence-based confidence filtering.
///
/// Creates module split recommendations and filters them based on
/// classification confidence when evidence is provided.
///
/// # Arguments
///
/// * `type_name` - Name of the type being analyzed
/// * `_methods` - All methods (kept for API compatibility)
/// * `responsibility_groups` - Methods grouped by responsibility
/// * `evidence_map` - Classification evidence for confidence filtering
///
/// # Returns
///
/// Vector of module split recommendations that meet confidence thresholds
pub fn recommend_module_splits_with_evidence(
    type_name: &str,
    _methods: &[String],
    responsibility_groups: &HashMap<String, Vec<String>>,
    evidence_map: &HashMap<
        String,
        crate::analysis::multi_signal_aggregation::AggregatedClassification,
    >,
) -> Vec<ModuleSplit> {
    use crate::organization::confidence::{MIN_METHODS_FOR_SPLIT, MODULE_SPLIT_CONFIDENCE};

    let mut recommendations = Vec::new();

    for (responsibility, methods) in responsibility_groups {
        if methods.len() > MIN_METHODS_FOR_SPLIT {
            let classification_evidence = evidence_map.get(responsibility).cloned();

            // Calculate average confidence from evidence map
            // If evidence_map is provided (not empty), enforce confidence threshold
            // If evidence_map is empty, allow split for backward compatibility
            if !evidence_map.is_empty() {
                let avg_confidence = if let Some(evidence) = &classification_evidence {
                    evidence.confidence
                } else {
                    0.0
                };

                // Skip splits below confidence threshold
                if avg_confidence < MODULE_SPLIT_CONFIDENCE {
                    log::debug!(
                        "Skipping module split for '{}': confidence {:.2} below threshold {:.2}",
                        responsibility,
                        avg_confidence,
                        MODULE_SPLIT_CONFIDENCE
                    );
                    continue;
                }
            }

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
                alternative_names: vec![],
                naming_confidence: None,
                naming_strategy: None,
                cluster_quality: None,
            });
        }
    }

    recommendations
}

/// Enhanced version that includes field access tracking and trait extraction.
///
/// This is a simplified version that delegates to the full-featured version
/// with an empty evidence map.
///
/// # Arguments
///
/// * `type_name` - Name of the type being analyzed
/// * `responsibility_groups` - Methods grouped by responsibility
/// * `field_tracker` - Optional field access tracker for minimal field set calculation
///
/// # Returns
///
/// Vector of enhanced module split recommendations
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

/// Full-featured recommendation with evidence, field tracking, and trait extraction.
///
/// Generates comprehensive module split recommendations with:
/// - Field access tracking for minimal field sets
/// - Behavioral categorization and trait suggestions
/// - Classification evidence for confidence filtering
///
/// # Arguments
///
/// * `type_name` - Name of the type being analyzed
/// * `responsibility_groups` - Methods grouped by responsibility
/// * `evidence_map` - Classification evidence for confidence filtering
/// * `field_tracker` - Optional field access tracker for minimal field set calculation
///
/// # Returns
///
/// Vector of comprehensive module split recommendations
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
                alternative_names: vec![],
                naming_confidence: None,
                naming_strategy: None,
                cluster_quality: None,
            });
        }
    }

    recommendations
}

/// Determine severity of cross-domain mixing issue.
///
/// Calculates recommendation severity based on the number of structs,
/// domains, lines of code, and whether the file is a god object.
///
/// # Arguments
///
/// * `struct_count` - Number of structs in the file
/// * `domain_count` - Number of distinct domains identified
/// * `lines` - Total lines of code
/// * `is_god_object` - Whether the file is classified as a god object
///
/// # Returns
///
/// Severity level for the recommendation
///
/// # Severity Levels
///
/// - **Critical**: God object with 3+ domains, or 15+ structs with 5+ domains
/// - **High**: 10+ structs with 4+ domains, or 800+ lines with 3+ domains
/// - **Medium**: 8+ structs or 400+ lines
/// - **Low**: Informational only
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

/// Sanitize a responsibility name for use as a module name.
///
/// Converts a human-readable responsibility string into a valid Rust module name by:
/// - Converting to lowercase
/// - Replacing special characters with underscores
/// - Removing invalid characters
/// - Collapsing multiple underscores
/// - Ensuring the name is not a reserved keyword
///
/// # Arguments
///
/// * `name` - The responsibility name to sanitize
///
/// # Returns
///
/// A valid Rust module name
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::recommender::sanitize_module_name;
///
/// assert_eq!(sanitize_module_name("File I/O"), "file_i_o");
/// assert_eq!(sanitize_module_name("Network & HTTP"), "network_and_http");
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
/// use std::collections::HashSet;
/// use debtmap::organization::god_object::recommender::ensure_unique_name;
///
/// let mut existing = HashSet::new();
/// existing.insert("utilities".to_string());
///
/// assert_eq!(ensure_unique_name("utilities".to_string(), &existing), "utilities_1");
///
/// existing.insert("utilities_1".to_string());
/// assert_eq!(ensure_unique_name("utilities".to_string(), &existing), "utilities_2");
/// ```
pub fn ensure_unique_name(name: String, existing_names: &HashSet<String>) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_module_name() {
        assert_eq!(sanitize_module_name("File I/O"), "file_i_o");
        assert_eq!(sanitize_module_name("Network & HTTP"), "network_and_http");
        assert_eq!(sanitize_module_name("I/O Utilities"), "i_o_utilities");
        assert_eq!(sanitize_module_name("User's Profile"), "users_profile");
        assert_eq!(
            sanitize_module_name("Data-Access-Layer"),
            "data_access_layer"
        );
    }

    #[test]
    fn test_ensure_unique_name() {
        let mut existing = HashSet::new();
        existing.insert("utilities".to_string());

        assert_eq!(
            ensure_unique_name("utilities".to_string(), &existing),
            "utilities_1"
        );

        existing.insert("utilities_1".to_string());
        assert_eq!(
            ensure_unique_name("utilities".to_string(), &existing),
            "utilities_2"
        );
    }

    #[test]
    fn test_determine_cross_domain_severity() {
        // Critical: God object with cross-domain mixing
        assert_eq!(
            determine_cross_domain_severity(10, 3, 500, true),
            RecommendationSeverity::Critical
        );

        // Critical: Massive cross-domain mixing
        assert_eq!(
            determine_cross_domain_severity(16, 5, 500, false),
            RecommendationSeverity::Critical
        );

        // High: Significant cross-domain issues
        assert_eq!(
            determine_cross_domain_severity(10, 4, 500, false),
            RecommendationSeverity::High
        );

        // Medium: Proactive improvement
        assert_eq!(
            determine_cross_domain_severity(8, 2, 300, false),
            RecommendationSeverity::Medium
        );

        // Low: Informational
        assert_eq!(
            determine_cross_domain_severity(5, 2, 200, false),
            RecommendationSeverity::Low
        );
    }

    #[test]
    fn test_suggest_module_splits_by_domain() {
        let structs = vec![
            StructMetrics {
                name: "ConfigOption".to_string(),
                line_span: (10, 30),
                method_count: 5,
                field_count: 3,
                responsibilities: vec![],
            },
            StructMetrics {
                name: "ConfigParser".to_string(),
                line_span: (40, 80),
                method_count: 8,
                field_count: 2,
                responsibilities: vec![],
            },
        ];

        let splits = suggest_module_splits_by_domain(&structs);
        assert!(!splits.is_empty());
        assert_eq!(splits[0].structs_to_move.len(), 2);
    }

    #[test]
    fn test_recommend_module_splits_basic() {
        let mut responsibility_groups = HashMap::new();
        responsibility_groups.insert(
            "persistence".to_string(),
            vec![
                "save".to_string(),
                "load".to_string(),
                "delete".to_string(),
                "update".to_string(),
                "create".to_string(),
                "fetch".to_string(),
            ],
        );

        let splits = recommend_module_splits("DataManager", &[], &responsibility_groups);

        assert_eq!(splits.len(), 1);
        assert_eq!(splits[0].methods_to_move.len(), 6);
        assert!(splits[0].suggested_name.contains("persistence"));
    }
}

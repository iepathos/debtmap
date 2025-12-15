//! # God Object Recommendation Generation (Pure Core)
//!
//! Pure functions for generating actionable recommendations based on
//! god object detection results. Recommendations are responsibility-aware
//! and pattern-aware.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - all functions are deterministic
//! with no side effects. Recommendation generation is a pure transformation
//! of analysis data.
//!
//! ## Spec 210: Context-Aware Recommendations
//!
//! This module now integrates with `context_recommendations` for:
//! - Cohesive structs: internal refactoring recommendations
//! - Multi-domain structs: domain-specific split recommendations
//! - Rationale based on cohesion scores and domain analysis

use super::classification_types::GodObjectType;
use super::context_recommendations::{
    classify_scenario, format_recommendation, generate_context_aware_recommendation,
    LongMethodInfo, RecommendationContext,
};
use crate::organization::struct_patterns::{PatternAnalysis, StructPattern};
use std::collections::HashMap;

/// Generate human-readable recommendation for god object (pure function).
///
/// Creates context-aware recommendations based on:
/// - Number of responsibilities
/// - Detected pattern (DTO, Config, etc.)
/// - Metric thresholds violated
///
/// # Arguments
///
/// * `classification` - God object classification result
/// * `pattern_analysis` - Optional pattern detection result
///
/// # Returns
///
/// Human-readable recommendation string
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::recommendation_generator::generate_recommendation;
/// use debtmap::organization::god_object::classification_types::GodObjectType;
///
/// let classification = GodObjectType::GodClass {
///     struct_name: "UserManager".to_string(),
///     method_count: 25,
///     field_count: 20,
///     responsibilities: 5,
/// };
///
/// let rec = generate_recommendation(&classification, None);
/// assert!(rec.contains("5 distinct responsibilities"));
/// assert!(rec.contains("5 focused modules"));
/// ```
pub fn generate_recommendation(
    classification: &GodObjectType,
    pattern_analysis: Option<&PatternAnalysis>,
) -> String {
    match classification {
        GodObjectType::GodClass {
            struct_name,
            method_count,
            field_count,
            responsibilities,
        } => generate_god_class_recommendation(
            struct_name,
            *method_count,
            *field_count,
            *responsibilities,
            pattern_analysis,
        ),

        GodObjectType::GodModule {
            total_methods,
            largest_struct,
            suggested_splits,
            ..
        } => generate_god_module_recommendation(
            *total_methods,
            &largest_struct.name,
            suggested_splits.len(),
        ),

        GodObjectType::NotGodObject => {
            if let Some(analysis) = pattern_analysis {
                generate_pattern_observation(&analysis.pattern, &analysis.evidence)
            } else {
                "No god object detected. Metrics within acceptable thresholds.".to_string()
            }
        }

        _ => "Analysis complete.".to_string(),
    }
}

/// Generate recommendation for God Class (pure function).
///
/// Recommendation varies based on:
/// 1. Responsibility count (1 vs many)
/// 2. Detected pattern (DTO vs Config vs Standard)
/// 3. Which metrics are violated
fn generate_god_class_recommendation(
    struct_name: &str,
    method_count: usize,
    field_count: usize,
    responsibilities: usize,
    pattern_analysis: Option<&PatternAnalysis>,
) -> String {
    // Case 1: Pattern detected - provide pattern-specific guidance
    if let Some(analysis) = pattern_analysis {
        if analysis.skip_god_object_check {
            return generate_pattern_specific_advice(
                struct_name,
                method_count,
                field_count,
                &analysis.pattern,
            );
        }
    }

    // Case 2: Single responsibility but high metrics
    if responsibilities <= 1 {
        return generate_single_responsibility_advice(struct_name, method_count, field_count);
    }

    // Case 3: Multiple responsibilities - genuine god object
    generate_multiple_responsibility_advice(
        struct_name,
        method_count,
        field_count,
        responsibilities,
    )
}

/// Generate advice for pattern-detected structs (pure function).
fn generate_pattern_specific_advice(
    struct_name: &str,
    method_count: usize,
    field_count: usize,
    pattern: &StructPattern,
) -> String {
    match pattern {
        StructPattern::DataTransferObject => {
            format!(
                "Data Transfer Object '{}' has {} fields. \
                 Consider grouping related fields into nested structs for better organization. \
                 For example, cluster metrics, context, and analysis data into separate sub-structures.",
                struct_name, field_count
            )
        }

        StructPattern::Config => {
            format!(
                "Configuration struct '{}' is well-structured with factory methods. \
                 If {} methods feels high, consider if some are truly needed or could be standalone utilities.",
                struct_name, method_count
            )
        }

        StructPattern::AggregateRoot => {
            format!(
                "Domain entity '{}' has complex state ({} fields, {} methods). \
                 This may be acceptable for an aggregate root, but consider: \
                 (1) Can any fields be value objects? \
                 (2) Should some operations be domain services? \
                 (3) Are all fields truly cohesive?",
                struct_name, field_count, method_count
            )
        }

        StructPattern::Standard => {
            // Fallback - shouldn't normally reach here
            format!("Struct '{}' shows no clear pattern.", struct_name)
        }
    }
}

/// Generate advice for single-responsibility structs with high metrics (pure function).
fn generate_single_responsibility_advice(
    struct_name: &str,
    method_count: usize,
    field_count: usize,
) -> String {
    // Determine primary metric violation
    if field_count > 25 {
        format!(
            "Struct '{}' has {} fields but single responsibility. \
             Consider grouping related fields into nested structures. \
             This reduces cognitive load while preserving cohesion.",
            struct_name, field_count
        )
    } else if method_count > 20 {
        format!(
            "Struct '{}' has {} methods but single responsibility. \
             This may be acceptable depending on domain complexity. \
             Consider: (1) Are all methods essential? (2) Could some be free functions? \
             (3) Is there a finer-grained responsibility hiding?",
            struct_name, method_count
        )
    } else {
        format!(
            "Struct '{}' has elevated metrics ({} methods, {} fields) but single responsibility. \
             May be acceptable - evaluate domain complexity.",
            struct_name, method_count, field_count
        )
    }
}

/// Generate advice for multiple-responsibility god objects (pure function).
fn generate_multiple_responsibility_advice(
    struct_name: &str,
    method_count: usize,
    field_count: usize,
    responsibilities: usize,
) -> String {
    format!(
        "God Class '{}' detected: {} distinct responsibilities across {} methods and {} fields. \
         Recommend splitting into {} focused modules, one per responsibility. \
         This improves maintainability, testability, and allows independent evolution of each concern.",
        struct_name,
        responsibilities,
        method_count,
        field_count,
        responsibilities
    )
}

/// Generate recommendation for God Module (pure function).
fn generate_god_module_recommendation(
    total_methods: usize,
    largest_struct: &str,
    suggested_split_count: usize,
) -> String {
    if suggested_split_count > 1 {
        format!(
            "God Module detected: {} total methods with '{}' as largest struct. \
             Consider splitting into {} focused sub-modules by domain responsibility.",
            total_methods, largest_struct, suggested_split_count
        )
    } else {
        format!(
            "God Module detected: {} total methods. \
             Consider refactoring into smaller, focused sub-modules.",
            total_methods
        )
    }
}

/// Generate observation for detected patterns (pure function).
fn generate_pattern_observation(pattern: &StructPattern, evidence: &[String]) -> String {
    let pattern_name = match pattern {
        StructPattern::Config => "Configuration Pattern",
        StructPattern::DataTransferObject => "Data Transfer Object Pattern",
        StructPattern::AggregateRoot => "Aggregate Root Pattern",
        StructPattern::Standard => "Standard Struct",
    };

    if evidence.is_empty() {
        format!("{} detected. No god object concerns.", pattern_name)
    } else {
        format!(
            "{} detected. Evidence: {}. No immediate refactoring needed.",
            pattern_name,
            evidence.join("; ")
        )
    }
}

// ============================================================================
// Spec 210: Context-Aware Recommendation Generation
// ============================================================================

/// Generate context-aware recommendation using cohesion and domain analysis.
///
/// This function produces recommendations that:
/// 1. Consider struct cohesion before suggesting splits
/// 2. Identify specific methods/domains to extract
/// 3. Provide rationale based on actual metrics
///
/// # Arguments
///
/// * `struct_name` - Name of the struct being analyzed
/// * `method_names` - All method names in the struct
/// * `cohesion_score` - Pre-calculated cohesion score (0.0 to 1.0)
/// * `domain_groups` - Methods grouped by behavioral domain
/// * `method_line_counts` - Optional map of method names to line counts
/// * `method_complexities` - Optional map of method names to complexity values
///
/// # Returns
///
/// Human-readable recommendation string with rationale
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::recommendation_generator::generate_recommendation_with_context;
/// use std::collections::HashMap;
///
/// let methods = vec!["get_module".to_string(), "track_module".to_string()];
/// let mut domain_groups = HashMap::new();
/// domain_groups.insert("ModuleTracker".to_string(), methods.clone());
///
/// let rec = generate_recommendation_with_context(
///     "CrossModuleTracker",
///     &methods,
///     0.75, // High cohesion
///     &domain_groups,
///     None,
///     None,
/// );
/// assert!(rec.contains("cohesion") || rec.contains("Borderline"));
/// ```
pub fn generate_recommendation_with_context(
    struct_name: &str,
    method_names: &[String],
    cohesion_score: f64,
    domain_groups: &HashMap<String, Vec<String>>,
    method_line_counts: Option<&HashMap<String, usize>>,
    method_complexities: Option<&HashMap<String, u32>>,
) -> String {
    // Build long method info if line counts are available
    let empty_line_counts = HashMap::new();
    let empty_complexities = HashMap::new();
    let line_counts = method_line_counts.unwrap_or(&empty_line_counts);
    let complexities = method_complexities.unwrap_or(&empty_complexities);

    let long_methods: Vec<LongMethodInfo> = line_counts
        .iter()
        .filter(|(_, &count)| count >= super::context_recommendations::LONG_METHOD_THRESHOLD)
        .map(|(name, &count)| LongMethodInfo {
            name: name.clone(),
            line_count: count,
            complexity: complexities.get(name).copied().unwrap_or(0),
        })
        .collect();

    // Count substantive methods (non-accessors)
    let accessor_prefixes = ["get_", "set_", "is_", "has_"];
    let substantive_methods = method_names
        .iter()
        .filter(|m| !accessor_prefixes.iter().any(|p| m.starts_with(p)))
        .count();

    let largest_method_loc = line_counts.values().copied().max().unwrap_or(0);

    // Build context
    let context = RecommendationContext {
        cohesion_score,
        domain_groups: domain_groups.clone(),
        long_methods,
        total_methods: method_names.len(),
        substantive_methods,
        largest_method_loc,
        struct_name: struct_name.to_string(),
    };

    // Classify scenario and generate recommendation
    let scenario = classify_scenario(&context);
    let recommendation = generate_context_aware_recommendation(&context, &scenario);

    format_recommendation(&recommendation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiple_responsibilities_recommendation() {
        let classification = GodObjectType::GodClass {
            struct_name: "UserManager".to_string(),
            method_count: 25,
            field_count: 20,
            responsibilities: 5,
        };

        let rec = generate_recommendation(&classification, None);
        assert!(rec.contains("5 distinct responsibilities"));
        assert!(rec.contains("5 focused modules"));
        assert!(rec.contains("UserManager"));
    }

    #[test]
    fn test_single_responsibility_high_fields() {
        let classification = GodObjectType::GodClass {
            struct_name: "UnifiedDebtItem".to_string(),
            method_count: 1,
            field_count: 35,
            responsibilities: 1,
        };

        let rec = generate_recommendation(&classification, None);
        assert!(rec.contains("single responsibility"));
        assert!(rec.contains("nested structures"));
        assert!(!rec.contains("split into")); // Should NOT recommend splitting
    }

    #[test]
    fn test_dto_pattern_recommendation() {
        let pattern = PatternAnalysis {
            pattern: StructPattern::DataTransferObject,
            confidence: 0.9,
            evidence: vec!["High field count".to_string()],
            skip_god_object_check: true,
        };

        let classification = GodObjectType::GodClass {
            struct_name: "DebtItem".to_string(),
            method_count: 2,
            field_count: 30,
            responsibilities: 1,
        };

        let rec = generate_recommendation(&classification, Some(&pattern));
        assert!(rec.contains("Data Transfer Object"));
        assert!(rec.contains("grouping related fields"));
        assert!(!rec.contains("split into")); // DTOs should not split
    }

    #[test]
    fn test_config_pattern_recommendation() {
        let pattern = PatternAnalysis {
            pattern: StructPattern::Config,
            confidence: 0.8,
            evidence: vec!["Factory methods".to_string()],
            skip_god_object_check: true,
        };

        let classification = GodObjectType::GodClass {
            struct_name: "AppConfig".to_string(),
            method_count: 7,
            field_count: 5,
            responsibilities: 1,
        };

        let rec = generate_recommendation(&classification, Some(&pattern));
        assert!(rec.contains("Configuration struct"));
        assert!(rec.contains("well-structured"));
    }

    #[test]
    fn test_god_module_recommendation() {
        let classification = GodObjectType::GodModule {
            total_structs: 3,
            total_methods: 80,
            largest_struct: super::super::core_types::StructMetrics {
                name: "Analyzer".to_string(),
                method_count: 40,
                field_count: 15,
                responsibilities: vec![],
                line_span: (1, 500),
            },
            suggested_splits: vec![],
        };

        let rec = generate_recommendation(&classification, None);
        assert!(rec.contains("God Module"));
        assert!(rec.contains("80 total methods"));
        // Note: suggested_splits is empty, so no specific split count mentioned
    }

    // =========================================================================
    // Spec 210: Context-Aware Recommendation Tests
    // =========================================================================

    #[test]
    fn test_context_aware_high_cohesion_with_long_methods() {
        let methods = vec![
            "get_module".to_string(),
            "track_module".to_string(),
            "analyze_workspace".to_string(),
        ];
        let mut domain_groups = HashMap::new();
        domain_groups.insert("ModuleTracker".to_string(), methods.clone());

        let mut line_counts = HashMap::new();
        line_counts.insert("analyze_workspace".to_string(), 50);
        line_counts.insert("get_module".to_string(), 10);

        let rec = generate_recommendation_with_context(
            "CrossModuleTracker",
            &methods,
            0.75, // High cohesion
            &domain_groups,
            Some(&line_counts),
            None,
        );

        // Should recommend internal refactoring, not splitting
        assert!(
            rec.contains("refactor") || rec.contains("Refactor") || rec.contains("internal"),
            "Should recommend internal refactoring for cohesive struct, got: {}",
            rec
        );
        assert!(
            !rec.contains("sub-orchestrator"),
            "Should NOT suggest sub-orchestrators for cohesive struct"
        );
    }

    #[test]
    fn test_context_aware_multi_domain_god_object() {
        let methods = vec![
            "parse_json".to_string(),
            "render_html".to_string(),
            "validate_email".to_string(),
            "send_notification".to_string(),
        ];
        let mut domain_groups = HashMap::new();
        domain_groups.insert("Parsing".to_string(), vec!["parse_json".to_string()]);
        domain_groups.insert("Rendering".to_string(), vec!["render_html".to_string()]);
        domain_groups.insert("Validation".to_string(), vec!["validate_email".to_string()]);
        domain_groups.insert(
            "Communication".to_string(),
            vec!["send_notification".to_string()],
        );

        let rec = generate_recommendation_with_context(
            "AppManager",
            &methods,
            0.15, // Low cohesion
            &domain_groups,
            None,
            None,
        );

        // Should recommend domain-based splitting
        assert!(
            rec.contains("Split") || rec.contains("domain") || rec.contains("module"),
            "Should recommend domain splits for multi-domain struct, got: {}",
            rec
        );
    }

    #[test]
    fn test_context_aware_cohesive_no_long_methods() {
        let methods = vec!["get_module".to_string(), "track".to_string()];
        let mut domain_groups = HashMap::new();
        domain_groups.insert("ModuleTracker".to_string(), methods.clone());

        let rec = generate_recommendation_with_context(
            "ModuleTracker",
            &methods,
            0.80, // High cohesion
            &domain_groups,
            None, // No long methods
            None,
        );

        // Should suggest borderline/review approach
        assert!(
            rec.contains("cohesion") || rec.contains("Borderline") || rec.contains("justified"),
            "Should mention cohesion for borderline cohesive struct, got: {}",
            rec
        );
    }

    #[test]
    fn test_context_aware_recommendation_includes_rationale() {
        let methods = vec![
            "parse_json".to_string(),
            "render_html".to_string(),
            "validate".to_string(),
        ];
        let mut domain_groups = HashMap::new();
        domain_groups.insert("Parsing".to_string(), vec!["parse_json".to_string()]);
        domain_groups.insert("Rendering".to_string(), vec!["render_html".to_string()]);
        domain_groups.insert("Validation".to_string(), vec!["validate".to_string()]);

        let rec = generate_recommendation_with_context(
            "Manager",
            &methods,
            0.20, // Low cohesion
            &domain_groups,
            None,
            None,
        );

        // Should include rationale (mentions cohesion percentage or domain count)
        assert!(
            rec.contains('%') || rec.contains("domain") || rec.contains("purpose"),
            "Should include rationale, got: {}",
            rec
        );
    }
}

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

use super::classification_types::GodObjectType;
use crate::organization::struct_patterns::{PatternAnalysis, StructPattern};

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
/// assert!(rec.contains("5 responsibilities"));
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
}

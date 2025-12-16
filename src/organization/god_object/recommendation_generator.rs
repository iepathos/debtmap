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

use super::classification_types::{FunctionalDecompositionMetrics, GodObjectType};
use super::context_recommendations::{
    classify_scenario, format_recommendation, generate_context_aware_recommendation,
    LongMethodInfo, RecommendationContext,
};
use crate::organization::struct_patterns::{PatternAnalysis, StructPattern};
use std::collections::HashMap;

// ============================================================================
// Spec 215: Functional Decomposition Recommendation Override
// ============================================================================

/// Action to take for a god object recommendation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecommendationAction {
    /// No action needed - the code is well-designed
    NoActionNeeded,
    /// Consider refactoring but not critical
    ConsiderRefactoring,
    /// Split into multiple modules (traditional god object recommendation)
    SplitIntoModules,
}

/// Recommendation for a god object with functional awareness.
#[derive(Debug, Clone)]
pub struct FunctionalAwareRecommendation {
    /// What action to take
    pub action: RecommendationAction,
    /// Human-readable explanation
    pub rationale: String,
    /// Suggested module extractions (if any)
    pub suggested_extractions: Vec<String>,
    /// Was functional decomposition detected?
    pub functional_pattern_detected: bool,
    /// Functional score (0.0 to 1.0) if detected
    pub functional_score: Option<f64>,
}

/// Generate recommendation considering functional decomposition patterns (Spec 215).
///
/// **Pure function** - deterministic, no side effects.
///
/// When functional decomposition is detected (many pure helper functions composing
/// into a few orchestrators), this function overrides the default "extract sub-orchestrators"
/// recommendation with appropriate guidance.
///
/// # Arguments
///
/// * `method_count` - Total method count
/// * `responsibility_count` - Number of detected responsibilities
/// * `functional_metrics` - Functional decomposition metrics from Spec 215
///
/// # Returns
///
/// `FunctionalAwareRecommendation` with action, rationale, and suggestions
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{
///     FunctionalDecompositionMetrics, RecommendationAction,
///     generate_recommendation_with_functional_awareness,
/// };
///
/// // CallResolver example: 24 methods, 21 pure helpers, 3 orchestrators
/// let functional_metrics = FunctionalDecompositionMetrics {
///     pure_method_ratio: 0.875,
///     orchestrator_count: 3,
///     pure_helper_count: 21,
///     avg_pure_method_loc: 8.0,
///     composition_patterns: vec![],
///     functional_score: 0.80,
/// };
///
/// let rec = generate_recommendation_with_functional_awareness(
///     24, // methods
///     7,  // responsibilities
///     &functional_metrics,
/// );
///
/// assert_eq!(rec.action, RecommendationAction::NoActionNeeded);
/// assert!(rec.rationale.contains("functional design"));
/// ```
pub fn generate_recommendation_with_functional_awareness(
    method_count: usize,
    responsibility_count: usize,
    functional_metrics: &FunctionalDecompositionMetrics,
) -> FunctionalAwareRecommendation {
    // Strong functional pattern: override default recommendation
    if functional_metrics.is_strong_functional_design() {
        return FunctionalAwareRecommendation {
            action: RecommendationAction::NoActionNeeded,
            rationale: format!(
                "Well-structured functional design detected: {} pure helpers composing into {} orchestrator(s). \
                 This pattern is intentional decomposition, not a god object. \
                 The high method count ({}) reflects functional style where complex behavior is built from many small, composable functions.",
                functional_metrics.pure_helper_count,
                functional_metrics.orchestrator_count,
                method_count,
            ),
            suggested_extractions: vec![],
            functional_pattern_detected: true,
            functional_score: Some(functional_metrics.functional_score),
        };
    }

    // Moderate functional pattern with some issues: tailored advice
    if functional_metrics.is_moderate_functional_style() {
        if responsibility_count > 3 {
            // Still has too many responsibilities - some grouping could help
            return FunctionalAwareRecommendation {
                action: RecommendationAction::ConsiderRefactoring,
                rationale: format!(
                    "Partial functional decomposition detected ({}% pure methods, {} orchestrators), \
                     but {} distinct responsibilities suggest some grouping could improve organization. \
                     Consider grouping related pure helpers by responsibility domain.",
                    (functional_metrics.pure_method_ratio * 100.0) as usize,
                    functional_metrics.orchestrator_count,
                    responsibility_count,
                ),
                suggested_extractions: suggest_responsibility_groupings(responsibility_count),
                functional_pattern_detected: true,
                functional_score: Some(functional_metrics.functional_score),
            };
        }

        // Moderate functional with focused responsibilities - acceptable
        return FunctionalAwareRecommendation {
            action: RecommendationAction::NoActionNeeded,
            rationale: format!(
                "Functional decomposition with focused responsibilities detected. \
                 {} methods, {}% pure helpers, {} orchestrator(s). \
                 The code follows functional patterns and responsibilities are well-organized.",
                method_count,
                (functional_metrics.pure_method_ratio * 100.0) as usize,
                functional_metrics.orchestrator_count,
            ),
            suggested_extractions: vec![],
            functional_pattern_detected: true,
            functional_score: Some(functional_metrics.functional_score),
        };
    }

    // Weak functional elements: provide standard advice with functional context
    if functional_metrics.has_functional_elements() {
        return FunctionalAwareRecommendation {
            action: RecommendationAction::ConsiderRefactoring,
            rationale: format!(
                "Some functional patterns detected ({}% pure methods), but not enough \
                 for full functional decomposition benefits. Consider: \
                 (1) Extracting more pure helper functions from instance methods, \
                 (2) Reducing orchestrator count (currently {}), \
                 (3) Breaking down larger methods into smaller composable pieces.",
                (functional_metrics.pure_method_ratio * 100.0) as usize,
                functional_metrics.orchestrator_count,
            ),
            suggested_extractions: suggest_functional_improvements(functional_metrics),
            functional_pattern_detected: false,
            functional_score: Some(functional_metrics.functional_score),
        };
    }

    // No functional pattern: standard god object recommendation
    FunctionalAwareRecommendation {
        action: RecommendationAction::SplitIntoModules,
        rationale: format!(
            "Traditional god object pattern detected ({} methods, {} responsibilities). \
             The code does not follow functional decomposition patterns (only {}% pure methods). \
             Consider splitting into {} focused modules, one per responsibility.",
            method_count,
            responsibility_count,
            (functional_metrics.pure_method_ratio * 100.0) as usize,
            responsibility_count,
        ),
        suggested_extractions: (1..=responsibility_count)
            .map(|i| format!("responsibility_{}_module", i))
            .collect(),
        functional_pattern_detected: false,
        functional_score: Some(functional_metrics.functional_score),
    }
}

/// Suggest responsibility-based groupings for moderate functional code.
fn suggest_responsibility_groupings(responsibility_count: usize) -> Vec<String> {
    (1..=responsibility_count.min(3))
        .map(|i| format!("Group related helpers into '{}_helpers' sub-module", i))
        .collect()
}

/// Suggest improvements for code with weak functional elements.
fn suggest_functional_improvements(metrics: &FunctionalDecompositionMetrics) -> Vec<String> {
    let mut suggestions = Vec::new();

    if metrics.pure_method_ratio < 0.5 {
        suggestions
            .push("Extract pure helper functions from instance methods where possible".to_string());
    }

    if metrics.orchestrator_count > 5 {
        suggestions.push(format!(
            "Reduce orchestrator count from {} to 2-3 by composing orchestrators",
            metrics.orchestrator_count
        ));
    }

    if metrics.avg_pure_method_loc > 15.0 {
        suggestions.push(format!(
            "Break down pure methods (avg {:.0} LOC) into smaller composable functions",
            metrics.avg_pure_method_loc
        ));
    }

    if suggestions.is_empty() {
        suggestions.push(
            "Continue extracting pure functions to improve functional decomposition".to_string(),
        );
    }

    suggestions
}

/// Format a functional-aware recommendation as a human-readable string.
pub fn format_functional_recommendation(rec: &FunctionalAwareRecommendation) -> String {
    let mut output = String::new();

    // Action header
    let action_str = match rec.action {
        RecommendationAction::NoActionNeeded => "No action needed",
        RecommendationAction::ConsiderRefactoring => "Consider refactoring",
        RecommendationAction::SplitIntoModules => "Split into modules",
    };
    output.push_str(&format!("[{}]\n", action_str));

    // Functional pattern indicator
    if rec.functional_pattern_detected {
        if let Some(score) = rec.functional_score {
            output.push_str(&format!(
                "Functional decomposition detected (score: {:.2})\n",
                score
            ));
        } else {
            output.push_str("Functional decomposition detected\n");
        }
    }

    // Rationale
    output.push_str(&format!("\n{}\n", rec.rationale));

    // Suggestions
    if !rec.suggested_extractions.is_empty() {
        output.push_str("\nSuggestions:\n");
        for suggestion in &rec.suggested_extractions {
            output.push_str(&format!("  - {}\n", suggestion));
        }
    }

    output
}

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

    // =========================================================================
    // Spec 215: Functional Decomposition Recommendation Tests
    // =========================================================================

    #[test]
    fn test_functional_recommendation_strong_functional() {
        // CallResolver example: 24 methods, 21 pure helpers, 3 orchestrators
        let functional_metrics = FunctionalDecompositionMetrics {
            pure_method_ratio: 0.875,
            orchestrator_count: 3,
            pure_helper_count: 21,
            avg_pure_method_loc: 8.0,
            composition_patterns: vec![],
            functional_score: 0.80, // Strong functional
        };

        let rec = generate_recommendation_with_functional_awareness(
            24, // methods
            7,  // responsibilities (would normally trigger god object)
            &functional_metrics,
        );

        assert_eq!(
            rec.action,
            RecommendationAction::NoActionNeeded,
            "Strong functional design should not need action"
        );
        assert!(
            rec.rationale.contains("functional design"),
            "Should mention functional design"
        );
        assert!(
            rec.functional_pattern_detected,
            "Should detect functional pattern"
        );
        assert!(
            rec.suggested_extractions.is_empty(),
            "Should not suggest extractions"
        );
    }

    #[test]
    fn test_functional_recommendation_moderate_with_responsibilities() {
        let functional_metrics = FunctionalDecompositionMetrics {
            pure_method_ratio: 0.60,
            orchestrator_count: 4,
            pure_helper_count: 12,
            avg_pure_method_loc: 10.0,
            composition_patterns: vec![],
            functional_score: 0.55, // Moderate functional
        };

        let rec = generate_recommendation_with_functional_awareness(
            20, // methods
            5,  // Many responsibilities
            &functional_metrics,
        );

        assert_eq!(
            rec.action,
            RecommendationAction::ConsiderRefactoring,
            "Moderate functional with many responsibilities should consider refactoring"
        );
        assert!(
            rec.rationale.contains("responsibilities"),
            "Should mention responsibilities"
        );
        assert!(
            rec.functional_pattern_detected,
            "Should detect functional pattern"
        );
    }

    #[test]
    fn test_functional_recommendation_moderate_focused() {
        let functional_metrics = FunctionalDecompositionMetrics {
            pure_method_ratio: 0.60,
            orchestrator_count: 3,
            pure_helper_count: 12,
            avg_pure_method_loc: 8.0,
            composition_patterns: vec![],
            functional_score: 0.55, // Moderate functional
        };

        let rec = generate_recommendation_with_functional_awareness(
            20, // methods
            2,  // Few responsibilities - focused
            &functional_metrics,
        );

        assert_eq!(
            rec.action,
            RecommendationAction::NoActionNeeded,
            "Moderate functional with focused responsibilities should not need action"
        );
        assert!(
            rec.rationale.contains("focused responsibilities")
                || rec.rationale.contains("well-organized"),
            "Should mention focused responsibilities, got: {}",
            rec.rationale
        );
    }

    #[test]
    fn test_functional_recommendation_weak_functional() {
        let functional_metrics = FunctionalDecompositionMetrics {
            pure_method_ratio: 0.35,
            orchestrator_count: 6,
            pure_helper_count: 7,
            avg_pure_method_loc: 20.0,
            composition_patterns: vec![],
            functional_score: 0.35, // Weak functional
        };

        let rec = generate_recommendation_with_functional_awareness(
            20, // methods
            4,  // responsibilities
            &functional_metrics,
        );

        assert_eq!(
            rec.action,
            RecommendationAction::ConsiderRefactoring,
            "Weak functional should consider refactoring"
        );
        assert!(
            !rec.functional_pattern_detected,
            "Should not strongly detect functional pattern"
        );
        assert!(
            !rec.suggested_extractions.is_empty(),
            "Should provide improvement suggestions"
        );
    }

    #[test]
    fn test_functional_recommendation_no_functional_pattern() {
        let functional_metrics = FunctionalDecompositionMetrics {
            pure_method_ratio: 0.10,
            orchestrator_count: 10,
            pure_helper_count: 2,
            avg_pure_method_loc: 25.0,
            composition_patterns: vec![],
            functional_score: 0.15, // No functional pattern
        };

        let rec = generate_recommendation_with_functional_awareness(
            20, // methods
            5,  // responsibilities
            &functional_metrics,
        );

        assert_eq!(
            rec.action,
            RecommendationAction::SplitIntoModules,
            "Traditional god object should recommend splitting"
        );
        assert!(
            rec.rationale.contains("Traditional god object"),
            "Should identify as traditional god object"
        );
        assert!(
            !rec.functional_pattern_detected,
            "Should not detect functional pattern"
        );
        assert_eq!(
            rec.suggested_extractions.len(),
            5,
            "Should suggest one extraction per responsibility"
        );
    }

    #[test]
    fn test_format_functional_recommendation() {
        let rec = FunctionalAwareRecommendation {
            action: RecommendationAction::NoActionNeeded,
            rationale: "Well-structured functional design detected.".to_string(),
            suggested_extractions: vec![],
            functional_pattern_detected: true,
            functional_score: Some(0.82),
        };

        let formatted = format_functional_recommendation(&rec);

        assert!(
            formatted.contains("[No action needed]"),
            "Should show action"
        );
        assert!(
            formatted.contains("Functional decomposition detected"),
            "Should show functional detection"
        );
        assert!(formatted.contains("0.82"), "Should show functional score");
        assert!(
            formatted.contains("Well-structured"),
            "Should show rationale"
        );
    }

    #[test]
    fn test_format_functional_recommendation_with_suggestions() {
        let rec = FunctionalAwareRecommendation {
            action: RecommendationAction::ConsiderRefactoring,
            rationale: "Some improvements possible.".to_string(),
            suggested_extractions: vec![
                "Extract pure helpers".to_string(),
                "Reduce orchestrator count".to_string(),
            ],
            functional_pattern_detected: false,
            functional_score: Some(0.35),
        };

        let formatted = format_functional_recommendation(&rec);

        assert!(
            formatted.contains("[Consider refactoring]"),
            "Should show action"
        );
        assert!(
            formatted.contains("Suggestions:"),
            "Should show suggestions header"
        );
        assert!(
            formatted.contains("Extract pure helpers"),
            "Should list first suggestion"
        );
        assert!(
            formatted.contains("Reduce orchestrator count"),
            "Should list second suggestion"
        );
    }
}

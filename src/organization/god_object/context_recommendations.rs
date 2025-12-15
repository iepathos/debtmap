//! # Context-Aware God Object Recommendations (Spec 210)
//!
//! Generates actionable recommendations based on struct cohesion and domain alignment.
//!
//! ## Design Principles
//!
//! 1. **Cohesive structs get internal refactoring advice**, not "extract sub-orchestrators"
//! 2. **True god objects get domain-specific split recommendations** with method lists
//! 3. **Recommendations include rationale** based on cohesion score and domain analysis
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - all functions are deterministic
//! with no side effects. Recommendation generation is a pure transformation
//! of analysis data.

use super::classifier::{
    calculate_domain_cohesion, extract_domain_context, group_methods_by_domain,
};
use std::collections::HashMap;

/// Threshold above which a struct is considered highly cohesive
pub const HIGH_COHESION_THRESHOLD: f64 = 0.5;

/// Threshold for long method detection (lines)
pub const LONG_METHOD_THRESHOLD: usize = 30;

/// Maximum word count for recommendations
pub const MAX_RECOMMENDATION_WORDS: usize = 200;

// ============================================================================
// Core Types
// ============================================================================

/// Context for generating recommendations.
///
/// Aggregates cohesion metrics, domain groupings, and method information
/// needed to classify scenarios and generate appropriate recommendations.
#[derive(Debug, Clone)]
pub struct RecommendationContext {
    /// Cohesion score (0.0 to 1.0) - higher means more cohesive
    pub cohesion_score: f64,
    /// Methods grouped by domain
    pub domain_groups: HashMap<String, Vec<String>>,
    /// Methods exceeding the long method threshold
    pub long_methods: Vec<LongMethodInfo>,
    /// Total method count (including accessors)
    pub total_methods: usize,
    /// Non-accessor method count
    pub substantive_methods: usize,
    /// Lines of code of the largest method
    pub largest_method_loc: usize,
    /// Name of the struct being analyzed
    pub struct_name: String,
}

/// Information about a long method that may need refactoring.
#[derive(Debug, Clone, PartialEq)]
pub struct LongMethodInfo {
    /// Method name
    pub name: String,
    /// Number of lines in the method
    pub line_count: usize,
    /// Cyclomatic complexity of the method
    pub complexity: u32,
}

/// Recommendation output with structured actionable advice.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextAwareRecommendation {
    /// Primary action to take
    pub primary_action: String,
    /// Why this recommendation is made
    pub rationale: String,
    /// Specific extractions or refactorings suggested
    pub suggested_extractions: Vec<String>,
    /// Estimated effort level
    pub effort_estimate: String,
}

// ============================================================================
// Scenario Classification
// ============================================================================

/// Classification of god object scenarios for recommendation generation.
///
/// Different scenarios require different types of recommendations:
/// - Cohesive structs need internal refactoring (method extraction)
/// - Multi-domain structs need domain-based splits
/// - Single-domain large structs need layered extraction
/// - Borderline cases get graduated recommendations
#[derive(Debug, Clone, PartialEq)]
pub enum GodObjectScenario {
    /// High cohesion, large size - suggest internal refactoring.
    ///
    /// Example: `CrossModuleTracker` with many methods but all related to
    /// module tracking. Should extract long methods, not split the struct.
    CohesiveLarge {
        /// Names of methods exceeding the long method threshold
        long_methods: Vec<String>,
    },

    /// Low cohesion, multiple distinct domains - suggest domain splits.
    ///
    /// Example: `AppManager` with parsing, rendering, and validation
    /// methods that have no shared state.
    MultiDomain {
        /// Suggested domain-based splits
        domains: Vec<DomainSplit>,
    },

    /// Single domain, too many methods - suggest layered extraction.
    ///
    /// Example: A large data-access struct with 30 methods. Should
    /// extract into layers (queries, commands, validation).
    SingleDomainLarge {
        /// Suggested layer-based splits
        suggested_layers: Vec<LayerSplit>,
    },

    /// Borderline case - provide options rather than definitive advice.
    Borderline {
        /// Primary recommendation
        primary_recommendation: String,
        /// Alternative approaches
        alternatives: Vec<String>,
    },
}

/// Information about a suggested domain-based split.
#[derive(Debug, Clone, PartialEq)]
pub struct DomainSplit {
    /// Name of the domain (e.g., "parsing", "rendering")
    pub domain_name: String,
    /// Suggested module name (valid Rust identifier)
    pub suggested_module_name: String,
    /// Methods to extract to this module
    pub methods: Vec<String>,
    /// Estimated lines of code for the extracted module
    pub estimated_loc: usize,
}

/// Information about a suggested layer-based split.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerSplit {
    /// Name of the layer (e.g., "data_access", "business_logic")
    pub layer_name: String,
    /// Description of the layer's responsibility
    pub description: String,
    /// Methods to extract to this layer
    pub methods: Vec<String>,
}

// ============================================================================
// Pure Functions
// ============================================================================

/// Build recommendation context from god object analysis data.
///
/// # Arguments
///
/// * `struct_name` - Name of the struct being analyzed
/// * `method_names` - All method names in the struct
/// * `field_names` - Field names in the struct
/// * `field_types` - Field type names
/// * `method_line_counts` - Map of method name to line count
/// * `method_complexities` - Map of method name to cyclomatic complexity
///
/// # Returns
///
/// A `RecommendationContext` with computed cohesion and domain groupings.
pub fn build_recommendation_context(
    struct_name: &str,
    method_names: &[String],
    field_names: &[String],
    field_types: &[String],
    method_line_counts: &HashMap<String, usize>,
    method_complexities: &HashMap<String, u32>,
) -> RecommendationContext {
    // Calculate cohesion score
    let cohesion_score = calculate_domain_cohesion(struct_name, method_names);

    // Extract domain context and group methods
    let domain_context = extract_domain_context(struct_name, field_names, field_types);
    let domain_groups = group_methods_by_domain(method_names, &domain_context);

    // Identify long methods
    let long_methods: Vec<LongMethodInfo> = method_line_counts
        .iter()
        .filter(|(_, &line_count)| line_count >= LONG_METHOD_THRESHOLD)
        .map(|(name, &line_count)| LongMethodInfo {
            name: name.clone(),
            line_count,
            complexity: method_complexities.get(name).copied().unwrap_or(0),
        })
        .collect();

    // Count substantive (non-accessor) methods
    let accessor_prefixes = ["get_", "set_", "is_", "has_"];
    let substantive_methods = method_names
        .iter()
        .filter(|m| !accessor_prefixes.iter().any(|p| m.starts_with(p)))
        .count();

    // Find largest method LOC
    let largest_method_loc = method_line_counts.values().copied().max().unwrap_or(0);

    RecommendationContext {
        cohesion_score,
        domain_groups,
        long_methods,
        total_methods: method_names.len(),
        substantive_methods,
        largest_method_loc,
        struct_name: struct_name.to_string(),
    }
}

/// Classify the god object scenario based on context.
///
/// Pure function that determines which type of recommendation is appropriate.
///
/// # Arguments
///
/// * `context` - The recommendation context with computed metrics
///
/// # Returns
///
/// A `GodObjectScenario` classification
pub fn classify_scenario(context: &RecommendationContext) -> GodObjectScenario {
    let domain_count = context.domain_groups.len();
    let has_long_methods = !context.long_methods.is_empty();

    // High cohesion - suggest internal refactoring, not splitting
    if context.cohesion_score > HIGH_COHESION_THRESHOLD {
        if has_long_methods {
            return GodObjectScenario::CohesiveLarge {
                long_methods: context
                    .long_methods
                    .iter()
                    .map(|m| m.name.clone())
                    .collect(),
            };
        } else {
            return GodObjectScenario::Borderline {
                primary_recommendation: format!(
                    "Struct has good cohesion ({:.0}%). Consider if size is justified.",
                    context.cohesion_score * 100.0
                ),
                alternatives: vec![
                    "Extract utility methods to a helper module".to_string(),
                    "Consider if all methods need to be public".to_string(),
                ],
            };
        }
    }

    // Multiple distinct domains - suggest domain splits
    if domain_count >= 3 {
        let domains = context
            .domain_groups
            .iter()
            .filter(|(name, _)| name.as_str() != "unclassified")
            .map(|(name, methods)| DomainSplit {
                domain_name: name.clone(),
                suggested_module_name: generate_module_name(name),
                methods: methods.clone(),
                estimated_loc: methods.len() * 15, // Rough estimate: 15 lines per method
            })
            .collect();

        return GodObjectScenario::MultiDomain { domains };
    }

    // Single domain, many methods - suggest layers
    if domain_count <= 2 && context.substantive_methods > 10 {
        let layers = suggest_layer_splits(context);
        return GodObjectScenario::SingleDomainLarge {
            suggested_layers: layers,
        };
    }

    // Borderline case
    GodObjectScenario::Borderline {
        primary_recommendation: "Consider extracting groups of related methods".to_string(),
        alternatives: vec![
            "Review if responsibility boundaries are clear".to_string(),
            "Consider if complexity is intrinsic to the domain".to_string(),
        ],
    }
}

/// Generate context-aware recommendation from scenario.
///
/// Pure function that produces actionable advice based on the classified scenario.
///
/// # Arguments
///
/// * `context` - The recommendation context
/// * `scenario` - The classified scenario
///
/// # Returns
///
/// A `ContextAwareRecommendation` with structured advice
pub fn generate_context_aware_recommendation(
    context: &RecommendationContext,
    scenario: &GodObjectScenario,
) -> ContextAwareRecommendation {
    match scenario {
        GodObjectScenario::CohesiveLarge { long_methods } => {
            let methods_list = long_methods.join(", ");
            let method_details: Vec<String> = context
                .long_methods
                .iter()
                .take(3) // Limit to avoid overly long recommendations
                .map(|m| {
                    format!(
                        "{} ({} lines, complexity {})",
                        m.name, m.line_count, m.complexity
                    )
                })
                .collect();

            ContextAwareRecommendation {
                primary_action: format!(
                    "Refactor internally: break down long methods ({})",
                    if long_methods.len() > 3 {
                        format!("{} and {} more", methods_list.split(", ").take(3).collect::<Vec<_>>().join(", "), long_methods.len() - 3)
                    } else {
                        methods_list
                    }
                ),
                rationale: format!(
                    "Good cohesion ({:.0}%) suggests unified domain. Focus on method extraction, not struct splitting.",
                    context.cohesion_score * 100.0
                ),
                suggested_extractions: method_details
                    .iter()
                    .map(|m| format!("{} → extract helper functions", m))
                    .collect(),
                effort_estimate: "Low - internal refactoring only".to_string(),
            }
        }

        GodObjectScenario::MultiDomain { domains } => {
            let domain_summaries: Vec<String> = domains
                .iter()
                .take(4) // Limit to avoid overly long recommendations
                .map(|d| {
                    format!(
                        "{} ({} methods → {})",
                        d.domain_name,
                        d.methods.len(),
                        d.suggested_module_name
                    )
                })
                .collect();

            let total_domains = domains.len();
            let summary = if total_domains > 4 {
                format!(
                    "{} and {} more domains",
                    domain_summaries.join(", "),
                    total_domains - 4
                )
            } else {
                domain_summaries.join(", ")
            };

            ContextAwareRecommendation {
                primary_action: format!(
                    "Split into {} domain-specific modules: {}",
                    domains.len(),
                    summary
                ),
                rationale: format!(
                    "Low cohesion ({:.0}%) with {} distinct domains. Methods don't share common purpose.",
                    context.cohesion_score * 100.0,
                    domains.len()
                ),
                suggested_extractions: domains
                    .iter()
                    .take(5)
                    .map(|d| {
                        let method_preview: String = d
                            .methods
                            .iter()
                            .take(3)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ");
                        let suffix = if d.methods.len() > 3 {
                            format!(" +{} more", d.methods.len() - 3)
                        } else {
                            String::new()
                        };
                        format!("mod {}: [{}{}]", d.suggested_module_name, method_preview, suffix)
                    })
                    .collect(),
                effort_estimate: format!("Medium - {} new modules", domains.len()),
            }
        }

        GodObjectScenario::SingleDomainLarge { suggested_layers } => {
            ContextAwareRecommendation {
                primary_action: "Extract by responsibility layer".to_string(),
                rationale: format!(
                    "Single domain but {} methods. Consider separating data access, business logic, and coordination.",
                    context.total_methods
                ),
                suggested_extractions: suggested_layers
                    .iter()
                    .map(|l| format!("{}: {}", l.layer_name, l.description))
                    .collect(),
                effort_estimate: "Medium - architectural refactoring".to_string(),
            }
        }

        GodObjectScenario::Borderline {
            primary_recommendation,
            alternatives,
        } => ContextAwareRecommendation {
            primary_action: primary_recommendation.clone(),
            rationale: "Borderline case - multiple valid approaches".to_string(),
            suggested_extractions: alternatives.clone(),
            effort_estimate: "Variable".to_string(),
        },
    }
}

/// Generate a valid Rust module name from a domain name.
///
/// Pure function that converts domain names to valid identifiers.
///
/// # Arguments
///
/// * `domain_name` - The domain name to convert
///
/// # Returns
///
/// A valid Rust module name (snake_case)
pub fn generate_module_name(domain_name: &str) -> String {
    // Convert to lowercase and replace spaces/hyphens with underscores
    let sanitized = domain_name.to_lowercase().replace([' ', '-'], "_");

    // Ensure it starts with a letter
    let sanitized = if sanitized
        .chars()
        .next()
        .map(|c| c.is_numeric())
        .unwrap_or(false)
    {
        format!("mod_{}", sanitized)
    } else {
        sanitized
    };

    // Add appropriate suffix based on domain type
    if sanitized.ends_with("ing") || sanitized.ends_with("tion") || sanitized.ends_with("sion") {
        format!("{}_module", sanitized)
    } else {
        format!("{}_handler", sanitized)
    }
}

/// Suggest layer-based splits for single-domain large structs.
///
/// Pure function that identifies common architectural layers.
fn suggest_layer_splits(context: &RecommendationContext) -> Vec<LayerSplit> {
    let mut layers = Vec::new();

    // Collect all methods across groups for layer analysis
    let all_methods: Vec<&String> = context
        .domain_groups
        .values()
        .flat_map(|methods| methods.iter())
        .collect();

    // Identify data access methods
    let data_access_prefixes = ["get_", "set_", "fetch_", "retrieve_", "find_", "query_"];
    let data_access: Vec<String> = all_methods
        .iter()
        .filter(|m| data_access_prefixes.iter().any(|p| m.starts_with(p)))
        .map(|m| (*m).clone())
        .collect();

    if data_access.len() >= 3 {
        layers.push(LayerSplit {
            layer_name: "data_access".to_string(),
            description: format!("{} data access methods", data_access.len()),
            methods: data_access,
        });
    }

    // Identify mutation/command methods
    let command_prefixes = ["create_", "update_", "delete_", "add_", "remove_", "save_"];
    let commands: Vec<String> = all_methods
        .iter()
        .filter(|m| command_prefixes.iter().any(|p| m.starts_with(p)))
        .map(|m| (*m).clone())
        .collect();

    if commands.len() >= 3 {
        layers.push(LayerSplit {
            layer_name: "commands".to_string(),
            description: format!("{} mutation/command methods", commands.len()),
            methods: commands,
        });
    }

    // Identify validation methods
    let validation_prefixes = ["validate_", "check_", "verify_", "is_valid", "ensure_"];
    let validation: Vec<String> = all_methods
        .iter()
        .filter(|m| validation_prefixes.iter().any(|p| m.starts_with(p)))
        .map(|m| (*m).clone())
        .collect();

    if validation.len() >= 2 {
        layers.push(LayerSplit {
            layer_name: "validation".to_string(),
            description: format!("{} validation methods", validation.len()),
            methods: validation,
        });
    }

    // If no clear layers found, suggest generic business logic extraction
    if layers.is_empty() {
        layers.push(LayerSplit {
            layer_name: "core".to_string(),
            description: "Core business logic".to_string(),
            methods: vec![],
        });
        layers.push(LayerSplit {
            layer_name: "helpers".to_string(),
            description: "Helper/utility methods".to_string(),
            methods: vec![],
        });
    }

    layers
}

/// Format recommendation as human-readable string.
///
/// Converts structured recommendation to display format.
pub fn format_recommendation(rec: &ContextAwareRecommendation) -> String {
    let mut output = String::new();

    output.push_str(&rec.primary_action);
    output.push_str(". ");
    output.push_str(&rec.rationale);

    if !rec.suggested_extractions.is_empty() {
        output.push_str(" Suggested: ");
        output.push_str(&rec.suggested_extractions.join("; "));
        output.push('.');
    }

    output.push_str(" Effort: ");
    output.push_str(&rec.effort_estimate);
    output.push('.');

    // Truncate if too long (respecting word boundaries)
    let words: Vec<&str> = output.split_whitespace().collect();
    if words.len() > MAX_RECOMMENDATION_WORDS {
        output = words[..MAX_RECOMMENDATION_WORDS].join(" ");
        output.push_str("...");
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_cohesive_large() {
        let mut domain_groups = HashMap::new();
        domain_groups.insert(
            "ModuleTracker".to_string(),
            vec!["get_module".to_string(), "track_module".to_string()],
        );

        let context = RecommendationContext {
            cohesion_score: 0.75,
            domain_groups,
            long_methods: vec![LongMethodInfo {
                name: "analyze_workspace".to_string(),
                line_count: 50,
                complexity: 15,
            }],
            total_methods: 15,
            substantive_methods: 10,
            largest_method_loc: 50,
            struct_name: "ModuleTracker".to_string(),
        };

        let scenario = classify_scenario(&context);
        assert!(matches!(scenario, GodObjectScenario::CohesiveLarge { .. }));

        if let GodObjectScenario::CohesiveLarge { long_methods } = scenario {
            assert!(long_methods.contains(&"analyze_workspace".to_string()));
        }
    }

    #[test]
    fn test_classify_multi_domain() {
        let mut domain_groups = HashMap::new();
        domain_groups.insert("Parsing".to_string(), vec!["parse_json".to_string()]);
        domain_groups.insert("Rendering".to_string(), vec!["render_html".to_string()]);
        domain_groups.insert("Validation".to_string(), vec!["validate_email".to_string()]);

        let context = RecommendationContext {
            cohesion_score: 0.15,
            domain_groups,
            long_methods: vec![],
            total_methods: 20,
            substantive_methods: 20,
            largest_method_loc: 30,
            struct_name: "AppManager".to_string(),
        };

        let scenario = classify_scenario(&context);
        assert!(matches!(scenario, GodObjectScenario::MultiDomain { .. }));
    }

    #[test]
    fn test_generate_module_name() {
        assert_eq!(generate_module_name("Parsing"), "parsing_module");
        assert_eq!(generate_module_name("data access"), "data_access_handler");
        assert_eq!(generate_module_name("Validation"), "validation_module");
        assert_eq!(generate_module_name("rendering"), "rendering_module");
        assert_eq!(
            generate_module_name("Event Handling"),
            "event_handling_module"
        );
    }

    #[test]
    fn test_recommendation_for_cohesive_struct() {
        let mut domain_groups = HashMap::new();
        domain_groups.insert(
            "ModuleTracker".to_string(),
            vec!["analyze_workspace".to_string()],
        );

        let context = RecommendationContext {
            cohesion_score: 0.75,
            domain_groups,
            long_methods: vec![LongMethodInfo {
                name: "analyze_workspace".to_string(),
                line_count: 50,
                complexity: 15,
            }],
            total_methods: 15,
            substantive_methods: 10,
            largest_method_loc: 50,
            struct_name: "CrossModuleTracker".to_string(),
        };

        let scenario = GodObjectScenario::CohesiveLarge {
            long_methods: vec!["analyze_workspace".to_string()],
        };

        let rec = generate_context_aware_recommendation(&context, &scenario);

        assert!(
            rec.primary_action.contains("Refactor internally")
                || rec.primary_action.contains("break down"),
            "Should recommend internal refactoring, got: {}",
            rec.primary_action
        );
        assert!(
            rec.rationale.contains("cohesion"),
            "Should mention cohesion, got: {}",
            rec.rationale
        );
        assert!(
            !rec.primary_action.contains("sub-orchestrator"),
            "Should NOT suggest sub-orchestrators for cohesive struct"
        );
    }

    #[test]
    fn test_recommendation_for_multi_domain() {
        let mut domain_groups = HashMap::new();
        domain_groups.insert(
            "Parsing".to_string(),
            vec!["parse_json".to_string(), "parse_xml".to_string()],
        );
        domain_groups.insert(
            "Rendering".to_string(),
            vec!["render_html".to_string(), "render_pdf".to_string()],
        );
        domain_groups.insert("Validation".to_string(), vec!["validate_email".to_string()]);

        let context = RecommendationContext {
            cohesion_score: 0.15,
            domain_groups,
            long_methods: vec![],
            total_methods: 20,
            substantive_methods: 20,
            largest_method_loc: 30,
            struct_name: "AppManager".to_string(),
        };

        let scenario = GodObjectScenario::MultiDomain {
            domains: vec![
                DomainSplit {
                    domain_name: "Parsing".to_string(),
                    suggested_module_name: "parsing_module".to_string(),
                    methods: vec!["parse_json".to_string(), "parse_xml".to_string()],
                    estimated_loc: 30,
                },
                DomainSplit {
                    domain_name: "Rendering".to_string(),
                    suggested_module_name: "rendering_module".to_string(),
                    methods: vec!["render_html".to_string(), "render_pdf".to_string()],
                    estimated_loc: 30,
                },
                DomainSplit {
                    domain_name: "Validation".to_string(),
                    suggested_module_name: "validation_module".to_string(),
                    methods: vec!["validate_email".to_string()],
                    estimated_loc: 15,
                },
            ],
        };

        let rec = generate_context_aware_recommendation(&context, &scenario);

        assert!(
            rec.primary_action.contains("Split into")
                || rec.primary_action.contains("domain-specific"),
            "Should recommend domain splits, got: {}",
            rec.primary_action
        );
        assert!(
            rec.rationale.contains("cohesion") || rec.rationale.contains("domain"),
            "Should mention low cohesion or domains"
        );
    }

    #[test]
    fn test_borderline_high_cohesion_no_long_methods() {
        let mut domain_groups = HashMap::new();
        domain_groups.insert(
            "ModuleTracker".to_string(),
            vec!["get_module".to_string(), "track".to_string()],
        );

        let context = RecommendationContext {
            cohesion_score: 0.75,
            domain_groups,
            long_methods: vec![], // No long methods
            total_methods: 10,
            substantive_methods: 8,
            largest_method_loc: 20,
            struct_name: "ModuleTracker".to_string(),
        };

        let scenario = classify_scenario(&context);
        assert!(matches!(scenario, GodObjectScenario::Borderline { .. }));

        if let GodObjectScenario::Borderline {
            primary_recommendation,
            ..
        } = scenario
        {
            assert!(primary_recommendation.contains("cohesion"));
        }
    }

    #[test]
    fn test_format_recommendation_truncation() {
        let rec = ContextAwareRecommendation {
            primary_action: "This is a very long recommendation ".repeat(50),
            rationale: "With an equally long rationale ".repeat(50),
            suggested_extractions: vec!["Many suggestions".repeat(20)],
            effort_estimate: "High".to_string(),
        };

        let formatted = format_recommendation(&rec);
        let word_count = formatted.split_whitespace().count();

        // Should be truncated to approximately MAX_RECOMMENDATION_WORDS
        assert!(
            word_count <= MAX_RECOMMENDATION_WORDS + 10, // Allow small buffer for "..."
            "Should truncate long recommendations, got {} words",
            word_count
        );
    }

    #[test]
    fn test_suggest_layer_splits_with_data_access() {
        let mut domain_groups = HashMap::new();
        domain_groups.insert(
            "Core".to_string(),
            vec![
                "get_user".to_string(),
                "get_profile".to_string(),
                "fetch_data".to_string(),
                "create_record".to_string(),
                "update_entry".to_string(),
                "delete_item".to_string(),
                "validate_input".to_string(),
                "check_permissions".to_string(),
            ],
        );

        let context = RecommendationContext {
            cohesion_score: 0.3,
            domain_groups,
            long_methods: vec![],
            total_methods: 8,
            substantive_methods: 8,
            largest_method_loc: 20,
            struct_name: "UserService".to_string(),
        };

        let layers = suggest_layer_splits(&context);

        // Should identify data access and command layers
        let layer_names: Vec<_> = layers.iter().map(|l| l.layer_name.as_str()).collect();
        assert!(
            layer_names.contains(&"data_access") || layer_names.contains(&"commands"),
            "Should identify common layers, got: {:?}",
            layer_names
        );
    }

    #[test]
    fn test_build_recommendation_context() {
        let method_names = vec![
            "get_module".to_string(),
            "track_module".to_string(),
            "analyze".to_string(),
        ];
        let field_names = vec!["modules".to_string()];
        let field_types = vec!["HashMap".to_string()];
        let mut method_line_counts = HashMap::new();
        method_line_counts.insert("get_module".to_string(), 10);
        method_line_counts.insert("track_module".to_string(), 15);
        method_line_counts.insert("analyze".to_string(), 45); // Long method
        let mut method_complexities = HashMap::new();
        method_complexities.insert("analyze".to_string(), 12);

        let context = build_recommendation_context(
            "ModuleTracker",
            &method_names,
            &field_names,
            &field_types,
            &method_line_counts,
            &method_complexities,
        );

        assert!(context.cohesion_score >= 0.0);
        assert_eq!(context.total_methods, 3);
        assert_eq!(context.long_methods.len(), 1);
        assert_eq!(context.long_methods[0].name, "analyze");
        assert_eq!(context.struct_name, "ModuleTracker");
    }

    // Property-based test for determinism
    #[test]
    fn test_classify_scenario_deterministic() {
        let mut domain_groups = HashMap::new();
        domain_groups.insert("Domain".to_string(), vec!["method1".to_string()]);

        let context = RecommendationContext {
            cohesion_score: 0.75,
            domain_groups,
            long_methods: vec![],
            total_methods: 10,
            substantive_methods: 8,
            largest_method_loc: 20,
            struct_name: "TestStruct".to_string(),
        };

        // Same input should produce same output
        let scenario1 = classify_scenario(&context);
        let scenario2 = classify_scenario(&context);
        assert_eq!(scenario1, scenario2);
    }
}

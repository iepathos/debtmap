/// Intelligent module split recommendations for god object refactoring.
///
/// This module implements Spec 188: generating high-quality module split recommendations
/// that eliminate "misc" categories, enforce balanced sizing, and provide clear responsibilities.
use std::collections::HashMap;

use crate::organization::behavioral_decomposition::{BehaviorCategory, MethodCluster};

/// Recommendation for splitting a module with quality validation
#[derive(Debug, Clone)]
pub struct ModuleRecommendation {
    /// Module name (must not be generic like "misc" or "utils")
    pub name: String,
    /// Clear, specific responsibility statement
    pub responsibility: String,
    /// Methods to include in this module
    pub methods: Vec<String>,
    /// Estimated lines of code
    pub line_count_estimate: usize,
    /// Method count
    pub method_count: usize,
    /// Public interface methods
    pub public_interface: Vec<String>,
    /// Quality score (0.0 to 1.0)
    pub quality_score: f64,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Behavioral category
    pub category: BehaviorCategory,
    /// Fields needed by this module
    pub fields_needed: Vec<String>,
}

impl ModuleRecommendation {
    /// Validate module recommendation quality
    ///
    /// Checks for:
    /// - Generic anti-pattern names
    /// - Inappropriate size (too small <5 methods or too large >50 methods)
    /// - Name/responsibility mismatch
    pub fn validate(&mut self) {
        let mut score: f64 = 1.0;
        self.warnings.clear();

        // Check for anti-pattern names
        if is_generic_name(&self.name) {
            self.warnings.push(format!(
                "Generic module name '{}' - specify concrete responsibility",
                self.name
            ));
            score -= 0.5; // Strong penalty for anti-pattern names
        }

        // Check size
        if self.method_count < 5 {
            self.warnings.push(format!(
                "Too small ({} methods) - consider keeping in parent module",
                self.method_count
            ));
            score -= 0.5; // Strong penalty for too-small modules
        } else if self.method_count > 50 {
            self.warnings.push(format!(
                "Too large ({} methods) - consider further decomposition",
                self.method_count
            ));
            score -= 0.2;
        }

        // Check if name matches responsibility
        if !name_matches_responsibility(&self.name, &self.responsibility) {
            self.warnings
                .push("Module name doesn't reflect responsibility".to_string());
            score -= 0.1;
        }

        // Reward well-defined interface
        if (3..=10).contains(&self.public_interface.len()) {
            score += 0.1;
        }

        self.quality_score = score.clamp(0.0, 1.0);
    }

    /// Get size category for reporting
    pub fn size_category(&self) -> &'static str {
        match self.method_count {
            0..=10 => "Small",
            11..=30 => "Medium",
            31..=50 => "Large",
            _ => "Very Large",
        }
    }

    /// Check if this recommendation is acceptable (quality > 0.6)
    pub fn is_acceptable(&self) -> bool {
        self.quality_score > 0.6
    }
}

/// Check if a module name is a generic anti-pattern
///
/// Rejects names like:
/// - misc, miscellaneous
/// - util, utils, utilities
/// - helper, helpers
/// - common, shared
/// - stuff, things, other, extra
pub fn is_generic_name(name: &str) -> bool {
    const ANTI_PATTERN_NAMES: &[&str] = &[
        "misc",
        "miscellaneous",
        "util",
        "utils",
        "utilities",
        "utility",
        "helper",
        "helpers",
        "common",
        "shared",
        "stuff",
        "things",
        "other",
        "extra",
        "base",
        "core",
        "lib",
        "functions",
        "methods",
    ];

    let normalized = name.to_lowercase();
    ANTI_PATTERN_NAMES
        .iter()
        .any(|&pattern| normalized.contains(pattern))
}

/// Check if module name reflects its responsibility
///
/// Extracts key terms from responsibility and checks if they appear in the name.
fn name_matches_responsibility(name: &str, responsibility: &str) -> bool {
    // Extract key terms from responsibility (words > 3 chars)
    let resp_lower = responsibility.to_lowercase();
    let resp_terms: Vec<_> = resp_lower
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .collect();

    // Check if module name contains any key terms OR vice versa
    // (e.g., "event_handling" contains "handling" from "Handles user...")
    let name_lower = name.to_lowercase();
    resp_terms
        .iter()
        .any(|term| name_lower.contains(term) || term.contains(&name_lower.replace('_', " ")))
        || name_lower
            .split('_')
            .any(|name_part| resp_lower.contains(name_part) && name_part.len() > 3)
}

/// Multi-level decomposition plan for god objects
#[derive(Debug, Clone)]
pub struct DecompositionPlan {
    /// Decomposition levels (coarse to fine-grained)
    pub levels: Vec<DecompositionLevel>,
    /// Total methods in original god object
    pub total_methods: usize,
    /// Total lines in original god object
    pub total_lines: usize,
}

impl DecompositionPlan {
    /// Validate balance across module sizes
    ///
    /// Checks for:
    /// - Imbalanced splits (max/min ratio > 5:1)
    /// - Modules still too large (>50 methods)
    pub fn validate_balance(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        for level in &self.levels {
            let sizes: Vec<_> = level.modules.iter().map(|m| m.method_count).collect();

            if let (Some(&max), Some(&min)) = (sizes.iter().max(), sizes.iter().min()) {
                let ratio = max as f64 / min.max(1) as f64;

                if ratio > 5.0 {
                    warnings.push(format!(
                        "Level {}: Imbalanced module sizes ({:.1}:1 ratio) - largest: {} methods, smallest: {} methods",
                        level.level, ratio, max, min
                    ));
                }
            }
        }

        warnings
    }

    /// Suggest refinement for oversized modules
    pub fn suggest_refinement(&self) -> Option<String> {
        // Find modules that are still too large
        let oversized: Vec<_> = self
            .levels
            .last()
            .unwrap()
            .modules
            .iter()
            .filter(|m| m.method_count > 50)
            .collect();

        if !oversized.is_empty() {
            Some(format!(
                "Consider further splitting: {} (still has {} methods)",
                oversized[0].name, oversized[0].method_count
            ))
        } else {
            None
        }
    }

    /// Get recommended extraction order (placeholder for spec 179)
    ///
    /// Returns modules ordered by extraction priority.
    /// Currently uses basic heuristics; will be enhanced with coupling analysis from spec 179.
    pub fn get_extraction_order(&self) -> Vec<&ModuleRecommendation> {
        let final_level = self.levels.last().unwrap();
        let mut modules: Vec<&ModuleRecommendation> = final_level.modules.iter().collect();

        // Sort by method count (smaller modules first - easier to extract)
        modules.sort_by_key(|m| m.method_count);

        modules
    }
}

/// Single level in a decomposition plan
#[derive(Debug, Clone)]
pub struct DecompositionLevel {
    /// Level number (1 = coarse-grained, higher = more fine-grained)
    pub level: usize,
    /// Module recommendations at this level
    pub modules: Vec<ModuleRecommendation>,
}

/// Generate decomposition plan from behavioral clusters
///
/// Creates a multi-level decomposition plan based on the size of the god object:
/// - Small (<100 methods): Single-level decomposition
/// - Medium (100-300 methods): Two-level decomposition
/// - Large (>300 methods): Three-level decomposition
pub fn generate_decomposition_plan(
    total_methods: usize,
    total_lines: usize,
    clusters: &[MethodCluster],
) -> DecompositionPlan {
    if total_methods < 100 {
        single_level_decomposition(total_methods, total_lines, clusters)
    } else if total_methods < 300 {
        two_level_decomposition(total_methods, total_lines, clusters)
    } else {
        three_level_decomposition(total_methods, total_lines, clusters)
    }
}

/// Generate single-level decomposition (target: 5-8 modules)
fn single_level_decomposition(
    total_methods: usize,
    total_lines: usize,
    clusters: &[MethodCluster],
) -> DecompositionPlan {
    let modules = create_module_recommendations(clusters, total_lines);

    DecompositionPlan {
        levels: vec![DecompositionLevel { level: 1, modules }],
        total_methods,
        total_lines,
    }
}

/// Generate two-level decomposition (coarse + fine-grained)
fn two_level_decomposition(
    total_methods: usize,
    total_lines: usize,
    clusters: &[MethodCluster],
) -> DecompositionPlan {
    // Level 1: Coarse-grained modules
    let coarse_modules = create_module_recommendations(clusters, total_lines);

    // Level 2: Further split large modules
    let fine_modules = coarse_modules
        .iter()
        .flat_map(|module| {
            if module.method_count > 50 {
                // Split large module further
                split_large_module(module)
            } else {
                vec![module.clone()]
            }
        })
        .collect();

    DecompositionPlan {
        levels: vec![
            DecompositionLevel {
                level: 1,
                modules: coarse_modules,
            },
            DecompositionLevel {
                level: 2,
                modules: fine_modules,
            },
        ],
        total_methods,
        total_lines,
    }
}

/// Generate three-level decomposition for massive god objects
fn three_level_decomposition(
    total_methods: usize,
    total_lines: usize,
    clusters: &[MethodCluster],
) -> DecompositionPlan {
    // Level 1: Very coarse-grained (by major behavioral category)
    let level1 = create_module_recommendations(clusters, total_lines);

    // Level 2: Split each into medium-grained
    let level2: Vec<ModuleRecommendation> = level1
        .iter()
        .flat_map(|module| {
            if module.method_count > 80 {
                split_large_module(module)
            } else {
                vec![module.clone()]
            }
        })
        .collect();

    // Level 3: Final fine-grained split
    let level3: Vec<ModuleRecommendation> = level2
        .iter()
        .flat_map(|module| {
            if module.method_count > 50 {
                split_large_module(module)
            } else {
                vec![module.clone()]
            }
        })
        .collect();

    DecompositionPlan {
        levels: vec![
            DecompositionLevel {
                level: 1,
                modules: level1,
            },
            DecompositionLevel {
                level: 2,
                modules: level2,
            },
            DecompositionLevel {
                level: 3,
                modules: level3,
            },
        ],
        total_methods,
        total_lines,
    }
}

/// Create module recommendations from behavioral clusters
fn create_module_recommendations(
    clusters: &[MethodCluster],
    total_lines: usize,
) -> Vec<ModuleRecommendation> {
    let mut recommendations = Vec::new();

    for cluster in clusters {
        let module_name = cluster.category.module_name();
        let responsibility = generate_responsibility(&cluster.category, cluster.methods.len());
        let line_estimate = estimate_lines_for_cluster(cluster, total_lines);

        let mut recommendation = ModuleRecommendation {
            name: module_name,
            responsibility,
            methods: cluster.methods.clone(),
            line_count_estimate: line_estimate,
            method_count: cluster.methods.len(),
            public_interface: estimate_public_interface(&cluster.methods),
            quality_score: 0.0,
            warnings: Vec::new(),
            category: cluster.category.clone(),
            fields_needed: cluster.fields_accessed.clone(),
        };

        recommendation.validate();
        recommendations.push(recommendation);
    }

    recommendations
}

/// Generate responsibility statement for a behavioral category
fn generate_responsibility(category: &BehaviorCategory, method_count: usize) -> String {
    match category {
        BehaviorCategory::Rendering => {
            format!(
                "Responsible for rendering and visual display ({} methods)",
                method_count
            )
        }
        BehaviorCategory::EventHandling => {
            format!("Handles user input and events ({} methods)", method_count)
        }
        BehaviorCategory::Persistence => {
            format!(
                "Manages data persistence and serialization ({} methods)",
                method_count
            )
        }
        BehaviorCategory::Validation => {
            format!(
                "Validates data and business rules ({} methods)",
                method_count
            )
        }
        BehaviorCategory::Computation => {
            format!(
                "Performs calculations and transformations ({} methods)",
                method_count
            )
        }
        BehaviorCategory::StateManagement => {
            format!(
                "Manages internal state and data access ({} methods)",
                method_count
            )
        }
        BehaviorCategory::Lifecycle => {
            format!(
                "Handles object initialization and cleanup ({} methods)",
                method_count
            )
        }
        BehaviorCategory::Parsing => {
            format!("Parses and extracts data ({} methods)", method_count)
        }
        BehaviorCategory::Filtering => {
            format!("Filters and queries data ({} methods)", method_count)
        }
        BehaviorCategory::Transformation => {
            format!("Transforms and converts data ({} methods)", method_count)
        }
        BehaviorCategory::DataAccess => {
            format!("Accesses and retrieves data ({} methods)", method_count)
        }
        BehaviorCategory::Construction => {
            format!("Constructs and creates objects ({} methods)", method_count)
        }
        BehaviorCategory::Processing => {
            format!(
                "Processes and executes operations ({} methods)",
                method_count
            )
        }
        BehaviorCategory::Communication => {
            format!(
                "Communicates and exchanges messages ({} methods)",
                method_count
            )
        }
        BehaviorCategory::Domain(name) => {
            format!(
                "Handles {} domain operations ({} methods)",
                name, method_count
            )
        }
    }
}

/// Estimate lines of code for a cluster
fn estimate_lines_for_cluster(cluster: &MethodCluster, _total_lines: usize) -> usize {
    // Rough estimate: proportional to method count, with minimum of 10 lines per method
    let method_ratio = cluster.methods.len() as f64;
    let estimated = (method_ratio * 15.0) as usize; // Average 15 lines per method
    estimated.max(cluster.methods.len() * 10) // At least 10 lines per method
}

/// Estimate public interface methods from method names
fn estimate_public_interface(methods: &[String]) -> Vec<String> {
    methods
        .iter()
        .filter(|m| !m.starts_with('_')) // Assume non-underscore methods are public
        .take(10) // Limit to top 10 for interface estimate
        .cloned()
        .collect()
}

/// Split a large module into smaller submodules
fn split_large_module(module: &ModuleRecommendation) -> Vec<ModuleRecommendation> {
    // Split methods into groups by prefix/verb
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for method in &module.methods {
        let prefix = extract_method_prefix(method);
        groups.entry(prefix).or_default().push(method.clone());
    }

    // Create submodules from groups
    groups
        .into_iter()
        .filter(|(_, methods)| methods.len() >= 5) // Only keep groups with 5+ methods
        .map(|(prefix, methods)| {
            let submodule_name = format!("{}_{}", module.name, prefix);
            let responsibility = format!(
                "{} - {} operations ({} methods)",
                module.responsibility,
                prefix,
                methods.len()
            );

            let mut recommendation = ModuleRecommendation {
                name: submodule_name,
                responsibility,
                method_count: methods.len(),
                line_count_estimate: methods.len() * 15,
                methods: methods.clone(),
                public_interface: estimate_public_interface(&methods),
                quality_score: 0.0,
                warnings: Vec::new(),
                category: module.category.clone(),
                fields_needed: module.fields_needed.clone(),
            };

            recommendation.validate();
            recommendation
        })
        .collect()
}

/// Extract method prefix for grouping
fn extract_method_prefix(method_name: &str) -> String {
    // Common verb patterns
    let prefixes = [
        "get",
        "set",
        "update",
        "delete",
        "create",
        "build",
        "parse",
        "format",
        "validate",
        "check",
        "is",
        "has",
        "can",
        "should",
        "render",
        "draw",
        "paint",
        "handle",
        "on",
        "process",
        "calculate",
        "compute",
        "load",
        "save",
        "read",
        "write",
    ];

    let lower = method_name.to_lowercase();
    for prefix in &prefixes {
        if lower.starts_with(prefix) {
            return prefix.to_string();
        }
    }

    // Fallback: first word
    method_name.split('_').next().unwrap_or("misc").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_generic_name() {
        assert!(is_generic_name("misc"));
        assert!(is_generic_name("utilities"));
        assert!(is_generic_name("helpers"));
        assert!(is_generic_name("common"));

        assert!(!is_generic_name("rendering"));
        assert!(!is_generic_name("persistence"));
        assert!(!is_generic_name("validation"));
    }

    #[test]
    fn test_name_matches_responsibility() {
        assert!(name_matches_responsibility(
            "rendering",
            "Responsible for rendering and visual display"
        ));
        assert!(name_matches_responsibility(
            "event_handling",
            "Handles user input and events"
        ));
        assert!(!name_matches_responsibility(
            "misc",
            "Responsible for rendering and visual display"
        ));
    }

    #[test]
    fn test_module_recommendation_validation() {
        let mut recommendation = ModuleRecommendation {
            name: "rendering".to_string(),
            responsibility: "Handles rendering operations".to_string(),
            methods: (0..20).map(|i| format!("method_{}", i)).collect(),
            line_count_estimate: 300,
            method_count: 20,
            public_interface: vec!["render".to_string(), "draw".to_string()],
            quality_score: 0.0,
            warnings: Vec::new(),
            category: BehaviorCategory::Rendering,
            fields_needed: vec![],
        };

        recommendation.validate();
        assert!(recommendation.is_acceptable());
        assert!(recommendation.warnings.is_empty());
    }

    #[test]
    fn test_module_recommendation_too_small() {
        let mut recommendation = ModuleRecommendation {
            name: "tiny".to_string(),
            responsibility: "Does tiny things".to_string(),
            methods: vec!["method1".to_string(), "method2".to_string()],
            line_count_estimate: 20,
            method_count: 2,
            public_interface: vec![],
            quality_score: 0.0,
            warnings: Vec::new(),
            category: BehaviorCategory::Computation,
            fields_needed: vec![],
        };

        recommendation.validate();
        assert!(!recommendation.is_acceptable());
        assert!(recommendation
            .warnings
            .iter()
            .any(|w| w.contains("Too small")));
    }

    #[test]
    fn test_module_recommendation_generic_name() {
        let mut recommendation = ModuleRecommendation {
            name: "misc".to_string(),
            responsibility: "Miscellaneous operations".to_string(),
            methods: (0..15).map(|i| format!("method_{}", i)).collect(),
            line_count_estimate: 225,
            method_count: 15,
            public_interface: vec![],
            quality_score: 0.0,
            warnings: Vec::new(),
            category: BehaviorCategory::Domain("Misc".to_string()),
            fields_needed: vec![],
        };

        recommendation.validate();
        assert!(!recommendation.is_acceptable());
        assert!(recommendation
            .warnings
            .iter()
            .any(|w| w.contains("Generic module name")));
    }

    #[test]
    fn test_decomposition_plan_balance() {
        let modules = vec![
            ModuleRecommendation {
                name: "large".to_string(),
                responsibility: "Large module".to_string(),
                methods: (0..50).map(|i| format!("method_{}", i)).collect(),
                line_count_estimate: 750,
                method_count: 50,
                public_interface: vec![],
                quality_score: 0.8,
                warnings: vec![],
                category: BehaviorCategory::Computation,
                fields_needed: vec![],
            },
            ModuleRecommendation {
                name: "tiny".to_string(),
                responsibility: "Tiny module".to_string(),
                methods: (0..5).map(|i| format!("method_{}", i)).collect(),
                line_count_estimate: 75,
                method_count: 5,
                public_interface: vec![],
                quality_score: 0.8,
                warnings: vec![],
                category: BehaviorCategory::Validation,
                fields_needed: vec![],
            },
        ];

        let plan = DecompositionPlan {
            levels: vec![DecompositionLevel { level: 1, modules }],
            total_methods: 55,
            total_lines: 825,
        };

        let warnings = plan.validate_balance();
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("Imbalanced"));
    }
}

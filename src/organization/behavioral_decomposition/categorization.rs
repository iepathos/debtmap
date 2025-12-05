/// Method categorization by behavioral patterns.
///
/// This module provides functionality to categorize methods based on their
/// names and signatures, grouping them by behavioral patterns like lifecycle,
/// validation, parsing, etc.

use std::collections::HashMap;

use super::types::{capitalize_first, BehaviorCategory};

/// Behavioral categorizer for method names
pub struct BehavioralCategorizer;

impl BehavioralCategorizer {
    /// Categorize a method based on its name and signature
    ///
    /// Uses heuristics from Spec 208 (unified classification system):
    /// - Construction: create, build, new, make, construct (checked first before lifecycle)
    /// - Lifecycle: new, create, init, destroy, etc.
    /// - Parsing: parse, read, extract, decode, etc.
    /// - Rendering: render, draw, paint, format, etc.
    /// - Event handling: handle_*, on_*, etc.
    /// - Persistence: save, load, serialize, etc.
    /// - Validation: validate_*, check_*, verify_*, etc.
    /// - Computation: calculate, compute, evaluate, etc.
    /// - Filtering: filter, select, find, search, etc.
    /// - Transformation: transform, convert, map, apply, etc.
    /// - Data access: get, set, fetch, retrieve, access (checked before StateManagement per spec 208)
    /// - State management: update_*, mutate_*, *_state, etc.
    /// - Processing: process, handle, execute, run
    /// - Communication: send, receive, transmit, broadcast, notify
    pub fn categorize_method(method_name: &str) -> BehaviorCategory {
        let lower_name = method_name.to_lowercase();

        // Order matters: check more specific categories first

        // Construction (before lifecycle to catch "create_*")
        if Self::is_construction(&lower_name) {
            return BehaviorCategory::Construction;
        }

        // Lifecycle methods
        if Self::is_lifecycle(&lower_name) {
            return BehaviorCategory::Lifecycle;
        }

        // Validation methods (before rendering to prioritize verify_* over *_format)
        if Self::is_validation(&lower_name) {
            return BehaviorCategory::Validation;
        }

        // Parsing (check early as it's common)
        if Self::is_parsing(&lower_name) {
            return BehaviorCategory::Parsing;
        }

        // Rendering/Display methods
        if Self::is_rendering(&lower_name) {
            return BehaviorCategory::Rendering;
        }

        // Event handling methods
        if Self::is_event_handling(&lower_name) {
            return BehaviorCategory::EventHandling;
        }

        // Persistence methods
        if Self::is_persistence(&lower_name) {
            return BehaviorCategory::Persistence;
        }

        // Computation methods
        if Self::is_computation(&lower_name) {
            return BehaviorCategory::Computation;
        }

        // Filtering methods
        if Self::is_filtering(&lower_name) {
            return BehaviorCategory::Filtering;
        }

        // Transformation methods
        if Self::is_transformation(&lower_name) {
            return BehaviorCategory::Transformation;
        }

        // Data access methods (check before StateManagement per spec 208)
        if Self::is_data_access(&lower_name) {
            return BehaviorCategory::DataAccess;
        }

        // State management methods
        if Self::is_state_management(&lower_name) {
            return BehaviorCategory::StateManagement;
        }

        // Processing methods
        if Self::is_processing(&lower_name) {
            return BehaviorCategory::Processing;
        }

        // Communication methods
        if Self::is_communication(&lower_name) {
            return BehaviorCategory::Communication;
        }

        // Default: domain-specific based on first word (capitalized for better naming)
        let domain = method_name
            .split('_')
            .next()
            .filter(|s| !s.is_empty())
            .map(capitalize_first)
            .unwrap_or_else(|| "Operations".to_string());
        BehaviorCategory::Domain(domain)
    }

    pub(crate) fn is_lifecycle(name: &str) -> bool {
        const LIFECYCLE_KEYWORDS: &[&str] = &[
            "new",
            "create",
            "init",
            "initialize",
            "setup",
            "destroy",
            "cleanup",
            "dispose",
            "shutdown",
            "close",
        ];
        LIFECYCLE_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    pub(crate) fn is_rendering(name: &str) -> bool {
        const RENDERING_KEYWORDS: &[&str] = &[
            "render",
            "draw",
            "paint",
            "display",
            "show",
            "present",
            "format",
            "to_string",
            "print", // Per spec 208: print_* methods are rendering/output
        ];
        RENDERING_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    pub(crate) fn is_event_handling(name: &str) -> bool {
        name.starts_with("handle_")
            || name.starts_with("on_")
            || name.contains("_event")
            || name.contains("dispatch")
            || name.contains("trigger")
    }

    pub(crate) fn is_persistence(name: &str) -> bool {
        const PERSISTENCE_KEYWORDS: &[&str] = &[
            "save",
            "load",
            "persist",
            "restore",
            "serialize",
            "deserialize",
            "write",
            "read",
            "parse",
            "store", // Per spec 208: store_* methods are persistence operations
        ];
        PERSISTENCE_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    pub(crate) fn is_validation(name: &str) -> bool {
        // Per spec 208: "is_*" predicates are validation methods (e.g., is_valid, is_empty)
        const VALIDATION_KEYWORDS: &[&str] = &["validate", "check", "verify", "ensure", "is_"];
        VALIDATION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    pub(crate) fn is_state_management(name: &str) -> bool {
        name.starts_with("get_")
            || name.starts_with("set_")
            || name.starts_with("update_")
            || name.starts_with("mutate_")
            || name.contains("_state")
    }

    pub(crate) fn is_computation(name: &str) -> bool {
        const COMPUTATION_KEYWORDS: &[&str] = &["calculate", "compute", "evaluate", "measure"];
        COMPUTATION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    pub(crate) fn is_parsing(name: &str) -> bool {
        const PARSING_KEYWORDS: &[&str] = &[
            "parse",
            "read",
            "extract",
            "decode",
            "deserialize",
            "unmarshal",
            "scan",
        ];
        PARSING_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    pub(crate) fn is_filtering(name: &str) -> bool {
        const FILTERING_KEYWORDS: &[&str] = &[
            "filter", "select", "find", "search", "query", "lookup", "match",
        ];
        FILTERING_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    pub(crate) fn is_transformation(name: &str) -> bool {
        const TRANSFORMATION_KEYWORDS: &[&str] = &["transform", "convert", "map", "apply", "adapt"];
        TRANSFORMATION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    pub(crate) fn is_data_access(name: &str) -> bool {
        const DATA_ACCESS_KEYWORDS: &[&str] = &["get", "set", "fetch", "retrieve", "access"];
        DATA_ACCESS_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    pub(crate) fn is_construction(name: &str) -> bool {
        const CONSTRUCTION_KEYWORDS: &[&str] = &["create", "build", "new", "make", "construct"];
        CONSTRUCTION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    pub(crate) fn is_processing(name: &str) -> bool {
        const PROCESSING_KEYWORDS: &[&str] = &["process", "handle", "execute", "run"];
        PROCESSING_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    pub(crate) fn is_communication(name: &str) -> bool {
        const COMMUNICATION_KEYWORDS: &[&str] =
            &["send", "receive", "transmit", "broadcast", "notify"];
        COMMUNICATION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }
}

/// Cluster methods by behavioral category
pub fn cluster_methods_by_behavior(methods: &[String]) -> HashMap<BehaviorCategory, Vec<String>> {
    let mut clusters: HashMap<BehaviorCategory, Vec<String>> = HashMap::new();

    for method in methods {
        let category = BehavioralCategorizer::categorize_method(method);
        clusters.entry(category).or_default().push(method.clone());
    }

    // Filter out misc/domain clusters with too few methods
    clusters.retain(|category, methods| {
        matches!(
            category,
            BehaviorCategory::Lifecycle
                | BehaviorCategory::StateManagement
                | BehaviorCategory::Rendering
                | BehaviorCategory::EventHandling
                | BehaviorCategory::Persistence
                | BehaviorCategory::Validation
                | BehaviorCategory::Computation
                | BehaviorCategory::Parsing
                | BehaviorCategory::Filtering
                | BehaviorCategory::Transformation
                | BehaviorCategory::DataAccess
                | BehaviorCategory::Construction
                | BehaviorCategory::Processing
                | BehaviorCategory::Communication
        ) || methods.len() >= 3 // Keep domain clusters only if they have 3+ methods
    });

    clusters
}

/// Detect if a method is a test method that should stay in #[cfg(test)]
pub fn is_test_method(method_name: &str) -> bool {
    // Common test patterns in Rust
    method_name.starts_with("test_")
        || method_name.contains("_test_")
        || method_name.ends_with("_test")
        // Benchmark patterns
        || method_name.starts_with("bench_")
        || method_name.contains("_bench_")
        // Test helper patterns
        || method_name.starts_with("mock_")
        || method_name.starts_with("stub_")
        || method_name.starts_with("fixture_")
        || method_name == "setup"
        || method_name == "teardown"
}

/// Infer the category for a cluster of methods
///
/// Returns the most common non-Domain category among the methods,
/// or falls back to the first method's category if no clear winner.
pub(crate) fn infer_cluster_category(methods: &[String]) -> BehaviorCategory {
    let mut category_counts: HashMap<BehaviorCategory, usize> = HashMap::new();

    for method in methods {
        let category = BehavioralCategorizer::categorize_method(method);
        *category_counts.entry(category).or_insert(0) += 1;
    }

    // Return most common category (excluding Domain categories)
    category_counts
        .into_iter()
        .filter(|(cat, _)| !matches!(cat, BehaviorCategory::Domain(_)))
        .max_by_key(|(_, count)| *count)
        .map(|(cat, _)| cat)
        .unwrap_or_else(|| {
            // If no clear category, use domain based on first method
            BehavioralCategorizer::categorize_method(&methods[0])
        })
}

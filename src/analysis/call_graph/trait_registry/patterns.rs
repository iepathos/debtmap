//! Pure pattern detection for common Rust traits
//!
//! This module contains pure functions for detecting common trait patterns
//! (Default, Clone, From/Into, Display/Debug, constructors) and computing
//! which functions should be marked as entry points.

use super::types::TraitImplementation;
use crate::priority::call_graph::FunctionId;
use im::{HashMap, Vector};

/// Result of pattern detection - functions to mark as entry points
#[derive(Debug, Clone, Default)]
pub struct PatternDetectionResult {
    pub entry_point_functions: Vec<FunctionId>,
}

/// Detect all common trait patterns and return functions to mark as entry points
///
/// Pure function that computes results without modifying the call graph.
pub fn detect_common_trait_patterns<'a>(
    trait_implementations: &HashMap<String, Vector<TraitImplementation>>,
    all_functions: impl Iterator<Item = &'a FunctionId>,
) -> PatternDetectionResult {
    let mut result = PatternDetectionResult::default();

    // Detect trait implementations
    result
        .entry_point_functions
        .extend(detect_default_trait_impls(trait_implementations));
    result
        .entry_point_functions
        .extend(detect_clone_trait_impls(trait_implementations));
    result
        .entry_point_functions
        .extend(detect_from_into_impls(trait_implementations));
    result
        .entry_point_functions
        .extend(detect_display_debug_impls(trait_implementations));

    // Detect constructor patterns
    result
        .entry_point_functions
        .extend(detect_constructor_patterns(all_functions));

    result
}

/// Detect Default trait implementations
///
/// Returns function IDs for all `default()` method implementations.
pub fn detect_default_trait_impls(
    trait_implementations: &HashMap<String, Vector<TraitImplementation>>,
) -> Vec<FunctionId> {
    trait_implementations
        .get("Default")
        .map(|impls| {
            impls
                .iter()
                .flat_map(|impl_info| &impl_info.method_implementations)
                .filter(|method| method.method_name == "default")
                .map(|method| method.method_id.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Detect Clone trait implementations
///
/// Returns function IDs for `clone()`, `clone_box()`, and `clone_from()` implementations.
pub fn detect_clone_trait_impls(
    trait_implementations: &HashMap<String, Vector<TraitImplementation>>,
) -> Vec<FunctionId> {
    trait_implementations
        .get("Clone")
        .map(|impls| {
            impls
                .iter()
                .flat_map(|impl_info| &impl_info.method_implementations)
                .filter(|method| is_clone_method(&method.method_name))
                .map(|method| method.method_id.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Pure predicate for clone method names
fn is_clone_method(name: &str) -> bool {
    matches!(name, "clone" | "clone_box" | "clone_from")
}

/// Detect From/Into trait implementations
///
/// Returns function IDs for all From/Into method implementations.
pub fn detect_from_into_impls(
    trait_implementations: &HashMap<String, Vector<TraitImplementation>>,
) -> Vec<FunctionId> {
    ["From", "Into"]
        .iter()
        .flat_map(|trait_name| {
            trait_implementations
                .get(*trait_name)
                .map(|impls| {
                    impls
                        .iter()
                        .flat_map(|impl_info| &impl_info.method_implementations)
                        .map(|method| method.method_id.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect()
}

/// Detect Display/Debug trait implementations
///
/// Returns function IDs for `fmt()` implementations.
pub fn detect_display_debug_impls(
    trait_implementations: &HashMap<String, Vector<TraitImplementation>>,
) -> Vec<FunctionId> {
    ["Display", "Debug"]
        .iter()
        .flat_map(|trait_name| {
            trait_implementations
                .get(*trait_name)
                .map(|impls| {
                    impls
                        .iter()
                        .flat_map(|impl_info| &impl_info.method_implementations)
                        .filter(|method| method.method_name == "fmt")
                        .map(|method| method.method_id.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect()
}

/// Detect constructor patterns (Type::new, Type::builder, etc.)
///
/// Pure function that identifies constructor-like functions from a list of functions.
pub fn detect_constructor_patterns<'a>(
    functions: impl Iterator<Item = &'a FunctionId>,
) -> Vec<FunctionId> {
    functions
        .filter(|f| is_constructor_pattern(&f.name))
        .cloned()
        .collect()
}

/// Pure predicate for constructor-like function names
pub fn is_constructor_pattern(name: &str) -> bool {
    name.ends_with("::new")
        || name == "new"
        || name.ends_with("::builder")
        || name.contains("::with_")
        || name.ends_with("::create")
}

/// Apply pattern detection results to a call graph
///
/// This is the I/O boundary - it applies the computed results to the mutable call graph.
pub fn apply_patterns_to_call_graph(
    result: &PatternDetectionResult,
    call_graph: &mut crate::priority::call_graph::CallGraph,
) {
    for func_id in &result.entry_point_functions {
        call_graph.mark_as_trait_dispatch(func_id.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::call_graph::trait_registry::types::TraitMethodImplementation;
    use std::path::PathBuf;

    fn make_function_id(name: &str, line: usize) -> FunctionId {
        FunctionId::new(PathBuf::from("test.rs"), name.to_string(), line)
    }

    #[test]
    fn test_is_constructor_pattern() {
        assert!(is_constructor_pattern("MyType::new"));
        assert!(is_constructor_pattern("new"));
        assert!(is_constructor_pattern("Config::builder"));
        assert!(is_constructor_pattern("Settings::with_defaults"));
        assert!(is_constructor_pattern("Database::create"));

        assert!(!is_constructor_pattern("process"));
        assert!(!is_constructor_pattern("util::helper"));
        assert!(!is_constructor_pattern("new_thing")); // "new" must be suffix or standalone
    }

    #[test]
    fn test_is_clone_method() {
        assert!(is_clone_method("clone"));
        assert!(is_clone_method("clone_box"));
        assert!(is_clone_method("clone_from"));

        assert!(!is_clone_method("clone_into"));
        assert!(!is_clone_method("my_clone"));
    }

    #[test]
    fn test_detect_default_trait_impls() {
        let mut impls = HashMap::new();

        let trait_impl = TraitImplementation {
            trait_name: "Default".to_string(),
            implementing_type: "MyConfig".to_string(),
            method_implementations: vec![TraitMethodImplementation {
                method_name: "default".to_string(),
                method_id: make_function_id("MyConfig::default", 10),
                overrides_default: false,
            }]
            .into(),
            impl_id: None,
        };

        impls.insert("Default".to_string(), vec![trait_impl].into());

        let result = detect_default_trait_impls(&impls);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "MyConfig::default");
    }

    #[test]
    fn test_detect_constructor_patterns() {
        let functions = vec![
            make_function_id("MyType::new", 10),
            make_function_id("Config::builder", 20),
            make_function_id("Settings::with_defaults", 30),
            make_function_id("Database::create", 40),
            make_function_id("util::process", 50),
        ];

        let result = detect_constructor_patterns(functions.iter());

        assert_eq!(result.len(), 4);
        assert!(!result.iter().any(|f| f.name == "util::process"));
    }
}

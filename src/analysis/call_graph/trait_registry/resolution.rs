//! Pure trait resolution logic
//!
//! This module contains pure functions for resolving trait method calls
//! to their concrete implementations. No I/O or side effects.

use super::types::{TraitImplementation, TraitMethodCall};
use crate::analyzers::trait_implementation_tracker::TraitImplementationTracker;
use crate::priority::call_graph::FunctionId;
use im::{HashMap, HashSet, Vector};
use std::sync::Arc;

/// Extract method implementations matching a specific method name
///
/// Pure function that filters implementations by method name and optional type.
pub fn extract_matching_methods(
    method_name: &str,
    implementations: &Vector<TraitImplementation>,
    type_filter: Option<&str>,
) -> Vector<FunctionId> {
    implementations
        .iter()
        .filter(|impl_info| {
            type_filter.is_none_or(|type_name| impl_info.implementing_type == type_name)
        })
        .flat_map(|impl_info| &impl_info.method_implementations)
        .filter(|method| method.method_name == method_name)
        .map(|method| method.method_id.clone())
        .collect()
}

/// Collect implementations for a known receiver type
///
/// Pure function that finds all implementations for a type's traits.
pub fn collect_typed_implementations(
    receiver_type: &str,
    method_name: &str,
    traits: &HashSet<String>,
    trait_implementations: &HashMap<String, Vector<TraitImplementation>>,
) -> Vector<FunctionId> {
    traits
        .iter()
        .filter_map(|trait_name| trait_implementations.get(trait_name))
        .flat_map(|impls| extract_matching_methods(method_name, impls, Some(receiver_type)))
        .collect()
}

/// Resolve a trait method call to possible implementations
///
/// Uses enhanced tracker for better resolution, falling back to type mapping.
pub fn resolve_trait_call(
    call: &TraitMethodCall,
    enhanced_tracker: &Arc<TraitImplementationTracker>,
    type_to_traits: &HashMap<String, HashSet<String>>,
    trait_implementations: &HashMap<String, Vector<TraitImplementation>>,
) -> Vector<FunctionId> {
    match &call.receiver_type {
        Some(receiver_type) => resolve_typed_call(
            call,
            receiver_type,
            enhanced_tracker,
            type_to_traits,
            trait_implementations,
        ),
        None => resolve_untyped_call(call, enhanced_tracker),
    }
}

/// Resolve a trait call with a known receiver type
fn resolve_typed_call(
    call: &TraitMethodCall,
    receiver_type: &str,
    enhanced_tracker: &Arc<TraitImplementationTracker>,
    type_to_traits: &HashMap<String, HashSet<String>>,
    trait_implementations: &HashMap<String, Vector<TraitImplementation>>,
) -> Vector<FunctionId> {
    // Try enhanced tracker first
    if let Some(method_id) =
        enhanced_tracker.resolve_method(receiver_type, &call.trait_name, &call.method_name)
    {
        return vec![method_id].into();
    }

    // Fall back to type mapping
    type_to_traits
        .get(receiver_type)
        .map(|traits| {
            collect_typed_implementations(
                receiver_type,
                &call.method_name,
                traits,
                trait_implementations,
            )
        })
        .unwrap_or_default()
}

/// Resolve a trait call without a known receiver type (trait object)
fn resolve_untyped_call(
    call: &TraitMethodCall,
    enhanced_tracker: &Arc<TraitImplementationTracker>,
) -> Vector<FunctionId> {
    enhanced_tracker.resolve_trait_object_call(&call.trait_name, &call.method_name)
}

/// Result of resolving trait method calls
#[derive(Debug, Clone, Default)]
pub struct ResolutionResult {
    /// Number of calls successfully resolved
    pub resolved_count: usize,
    /// Edges to add: (caller, callee)
    pub edges: Vec<(FunctionId, FunctionId)>,
    /// Functions to mark as trait dispatch entry points
    pub trait_dispatch_functions: Vec<FunctionId>,
}

/// Resolve all trait method calls and compute edges to add
///
/// Pure function that computes resolution without modifying the call graph.
pub fn resolve_all_trait_calls(
    unresolved_calls: &Vector<TraitMethodCall>,
    enhanced_tracker: &Arc<TraitImplementationTracker>,
    type_to_traits: &HashMap<String, HashSet<String>>,
    trait_implementations: &HashMap<String, Vector<TraitImplementation>>,
) -> ResolutionResult {
    let mut result = ResolutionResult::default();

    for call in unresolved_calls.iter() {
        let implementations = resolve_trait_call(
            call,
            enhanced_tracker,
            type_to_traits,
            trait_implementations,
        );

        for impl_method_id in implementations.iter() {
            result
                .edges
                .push((call.caller.clone(), impl_method_id.clone()));
            result.trait_dispatch_functions.push(impl_method_id.clone());
            result.resolved_count += 1;
        }
    }

    result
}

/// Apply resolution results to a call graph
///
/// This is the I/O boundary - it applies the computed results to the mutable call graph.
pub fn apply_resolution_to_call_graph(
    result: &ResolutionResult,
    call_graph: &mut crate::priority::call_graph::CallGraph,
) {
    for (caller, callee) in &result.edges {
        call_graph.add_call_parts(
            caller.clone(),
            callee.clone(),
            crate::priority::call_graph::CallType::Direct,
        );
    }

    for func_id in &result.trait_dispatch_functions {
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
    fn test_extract_matching_methods_no_filter() {
        let impl1 = TraitImplementation {
            trait_name: "Default".to_string(),
            implementing_type: "MyType".to_string(),
            method_implementations: vec![TraitMethodImplementation {
                method_name: "default".to_string(),
                method_id: make_function_id("MyType::default", 10),
                overrides_default: false,
            }]
            .into(),
            impl_id: None,
        };

        let impl2 = TraitImplementation {
            trait_name: "Default".to_string(),
            implementing_type: "OtherType".to_string(),
            method_implementations: vec![TraitMethodImplementation {
                method_name: "default".to_string(),
                method_id: make_function_id("OtherType::default", 20),
                overrides_default: false,
            }]
            .into(),
            impl_id: None,
        };

        let implementations: Vector<_> = vec![impl1, impl2].into();
        let result = extract_matching_methods("default", &implementations, None);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_extract_matching_methods_with_filter() {
        let impl1 = TraitImplementation {
            trait_name: "Default".to_string(),
            implementing_type: "MyType".to_string(),
            method_implementations: vec![TraitMethodImplementation {
                method_name: "default".to_string(),
                method_id: make_function_id("MyType::default", 10),
                overrides_default: false,
            }]
            .into(),
            impl_id: None,
        };

        let implementations: Vector<_> = vec![impl1].into();
        let result = extract_matching_methods("default", &implementations, Some("MyType"));

        assert_eq!(result.len(), 1);

        let result = extract_matching_methods("default", &implementations, Some("OtherType"));
        assert_eq!(result.len(), 0);
    }
}

//! Trait Registry for Enhanced Call Graph Analysis
//!
//! This module tracks trait definitions, implementations, and method calls
//! to resolve trait method calls to their concrete implementations,
//! reducing false positives in dead code detection.
//!
//! ## Module Structure
//!
//! Following the Stillwater philosophy of "pure core, imperative shell":
//!
//! - `types` - Pure data types (the still core)
//! - `resolution` - Pure resolution logic (the still core)
//! - `patterns` - Pure pattern detection (the still core)
//! - `visitor` - AST visitor for extraction (I/O boundary)
//! - `mod` (this file) - Coordinator and state management (the shell)

pub mod patterns;
pub mod resolution;
pub mod types;
pub mod visitor;

// Re-export public types for convenience
pub use types::{
    TraitImplementation, TraitMethod, TraitMethodCall, TraitMethodImplementation, TraitStatistics,
};

use crate::analyzers::trait_implementation_tracker::TraitImplementationTracker;
use crate::analyzers::trait_resolver::TraitResolver;
use crate::priority::call_graph::FunctionId;
use anyhow::Result;
use im::{HashMap, HashSet, Vector};
use std::path::Path;
use std::sync::Arc;
use syn::File;
use visitor::TraitVisitor;

/// Registry for tracking trait definitions, implementations, and method calls
///
/// This struct coordinates the various submodules to provide a unified API
/// for trait analysis. It follows the coordinator pattern, delegating
/// pure computation to submodules while managing state.
#[derive(Debug, Clone)]
pub struct TraitRegistry {
    /// All trait definitions found
    trait_definitions: HashMap<String, Vector<TraitMethod>>,
    /// All trait implementations found
    trait_implementations: HashMap<String, Vector<TraitImplementation>>,
    /// Unresolved trait method calls
    unresolved_calls: Vector<TraitMethodCall>,
    /// Type to trait mapping (for quick lookup)
    type_to_traits: HashMap<String, HashSet<String>>,
    /// Visit trait implementations (special handling for visitor pattern)
    visit_implementations: HashMap<String, Vector<TraitMethodImplementation>>,
    /// Functions that are Visit trait methods
    visit_trait_methods: HashSet<FunctionId>,
    /// Enhanced trait implementation tracker
    enhanced_tracker: Arc<TraitImplementationTracker>,
    /// Trait resolver for method resolution
    trait_resolver: Option<Arc<TraitResolver>>,
}

impl TraitRegistry {
    /// Create a new trait registry
    pub fn new() -> Self {
        let enhanced_tracker = Arc::new(TraitImplementationTracker::new());
        Self {
            trait_definitions: HashMap::new(),
            trait_implementations: HashMap::new(),
            unresolved_calls: Vector::new(),
            type_to_traits: HashMap::new(),
            visit_implementations: HashMap::new(),
            visit_trait_methods: HashSet::new(),
            enhanced_tracker,
            trait_resolver: None,
        }
    }

    /// Analyze a file for trait definitions and implementations
    pub fn analyze_file(&mut self, file_path: &Path, ast: &File) -> Result<()> {
        let visitor = TraitVisitor::new(file_path.to_path_buf());
        let result = visitor.visit_and_extract(ast);

        self.merge_visitor_result(result);
        Ok(())
    }

    /// Merge visitor results into registry state
    fn merge_visitor_result(&mut self, result: visitor::TraitVisitorResult) {
        // Add discovered traits
        for (trait_name, methods) in result.trait_definitions {
            self.trait_definitions.insert(trait_name, methods);
        }

        // Add trait implementations
        for trait_impl in result.trait_implementations {
            self.add_trait_implementation(trait_impl);
        }

        // Add unresolved calls
        for call in result.trait_method_calls {
            self.unresolved_calls.push_back(call);
        }

        // Add Visit trait methods
        for method_id in result.visit_trait_methods {
            self.visit_trait_methods.insert(method_id);
        }

        // Add Visit implementations
        for (type_name, methods) in result.visit_implementations {
            self.visit_implementations.insert(type_name, methods);
        }
    }

    /// Add a trait implementation and update type mappings
    fn add_trait_implementation(&mut self, trait_impl: TraitImplementation) {
        let trait_name = trait_impl.trait_name.clone();
        let implementing_type = trait_impl.implementing_type.clone();

        // Update type to trait mapping
        self.type_to_traits
            .entry(implementing_type)
            .or_default()
            .insert(trait_name.clone());

        // Add to implementations
        self.trait_implementations
            .entry(trait_name)
            .or_default()
            .push_back(trait_impl);
    }

    // Query methods

    /// Get all unresolved trait method calls
    pub fn get_unresolved_trait_calls(&self) -> Vector<TraitMethodCall> {
        self.unresolved_calls.clone()
    }

    /// Find implementations for a specific trait
    pub fn find_implementations(
        &self,
        trait_name: &str,
    ) -> Option<Vector<TraitMethodImplementation>> {
        self.trait_implementations.get(trait_name).map(|impls| {
            impls
                .iter()
                .flat_map(|impl_info| impl_info.method_implementations.iter())
                .cloned()
                .collect()
        })
    }

    /// Check if a function has trait implementations
    pub fn has_trait_implementations(&self, func_id: &FunctionId) -> bool {
        self.trait_implementations.values().any(|impls| {
            impls.iter().any(|impl_info| {
                impl_info
                    .method_implementations
                    .iter()
                    .any(|method| &method.method_id == func_id)
            })
        })
    }

    /// Get trait methods for a specific trait
    pub fn get_trait_methods(&self, trait_name: &str) -> Option<&Vector<TraitMethod>> {
        self.trait_definitions.get(trait_name)
    }

    /// Find trait implementations for a specific type
    pub fn find_implementations_for_type(&self, type_name: &str) -> Option<&HashSet<String>> {
        self.type_to_traits.get(type_name)
    }

    /// Check if a function is a Visit trait method
    pub fn is_visit_trait_method(&self, func_id: &FunctionId) -> bool {
        self.visit_trait_methods.contains(func_id)
    }

    /// Get all Visit trait methods
    pub fn get_visit_trait_methods(&self) -> HashSet<FunctionId> {
        self.visit_trait_methods.clone()
    }

    /// Get the enhanced trait implementation tracker
    pub fn get_enhanced_tracker(&self) -> Arc<TraitImplementationTracker> {
        self.enhanced_tracker.clone()
    }

    /// Initialize the trait resolver
    pub fn init_resolver(&mut self) {
        self.trait_resolver = Some(Arc::new(TraitResolver::new(self.enhanced_tracker.clone())));
    }

    /// Get the trait resolver
    pub fn get_resolver(&self) -> Option<Arc<TraitResolver>> {
        self.trait_resolver.clone()
    }

    /// Check if a type implements a trait using enhanced tracking
    pub fn type_implements_trait(&self, type_name: &str, trait_name: &str) -> bool {
        self.enhanced_tracker
            .implements_trait(type_name, trait_name)
    }

    /// Get Visit implementations for a specific type
    pub fn get_visit_implementations(
        &self,
        type_name: &str,
    ) -> Option<&Vector<TraitMethodImplementation>> {
        self.visit_implementations.get(type_name)
    }

    /// Get statistics about trait usage
    pub fn get_statistics(&self) -> TraitStatistics {
        TraitStatistics {
            total_traits: self.trait_definitions.len(),
            total_implementations: self
                .trait_implementations
                .values()
                .map(|impls| impls.len())
                .sum(),
            total_unresolved_calls: self.unresolved_calls.len(),
        }
    }

    // Resolution methods - delegate to resolution module

    /// Resolve a trait method call to possible implementations
    pub fn resolve_trait_call(&self, call: &TraitMethodCall) -> Vector<FunctionId> {
        resolution::resolve_trait_call(
            call,
            &self.enhanced_tracker,
            &self.type_to_traits,
            &self.trait_implementations,
        )
    }

    /// Resolve trait method calls and add edges to call graph
    /// Returns the number of trait method calls resolved
    pub fn resolve_trait_method_calls(
        &self,
        call_graph: &mut crate::priority::call_graph::CallGraph,
    ) -> usize {
        self.resolve_trait_method_calls_with_progress(call_graph, &indicatif::ProgressBar::hidden())
    }

    /// Resolve trait method calls with progress reporting
    pub fn resolve_trait_method_calls_with_progress(
        &self,
        call_graph: &mut crate::priority::call_graph::CallGraph,
        progress: &indicatif::ProgressBar,
    ) -> usize {
        let total_calls = self.unresolved_calls.len() as u64;
        progress.set_length(total_calls);
        progress.set_message("Resolving trait method calls");

        let result = resolution::resolve_all_trait_calls(
            &self.unresolved_calls,
            &self.enhanced_tracker,
            &self.type_to_traits,
            &self.trait_implementations,
        );

        resolution::apply_resolution_to_call_graph(&result, call_graph);

        progress.set_position(total_calls);
        progress.finish_and_clear();

        result.resolved_count
    }

    /// Count total trait method calls in the call graph (spec 201)
    pub fn count_trait_method_calls(
        &self,
        _call_graph: &crate::priority::call_graph::CallGraph,
    ) -> usize {
        self.unresolved_calls.len()
    }

    // Pattern detection methods - delegate to patterns module

    /// Detect common trait patterns and mark them as entry points
    pub fn detect_common_trait_patterns(
        &self,
        call_graph: &mut crate::priority::call_graph::CallGraph,
    ) {
        let result = patterns::detect_common_trait_patterns(
            &self.trait_implementations,
            call_graph.get_all_functions(),
        );
        patterns::apply_patterns_to_call_graph(&result, call_graph);
    }
}

impl Default for TraitRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::call_graph::CallGraph;
    use std::path::PathBuf;

    fn make_function_id(name: &str, line: usize) -> FunctionId {
        FunctionId::new(PathBuf::from("test.rs"), name.to_string(), line)
    }

    #[test]
    fn test_detect_default_trait_impls() {
        let mut registry = TraitRegistry::new();
        let mut call_graph = CallGraph::new();

        let default_method = make_function_id("MyConfig::default", 10);

        let trait_impl = TraitImplementation {
            trait_name: "Default".to_string(),
            implementing_type: "MyConfig".to_string(),
            method_implementations: vec![TraitMethodImplementation {
                method_name: "default".to_string(),
                method_id: default_method.clone(),
                overrides_default: false,
            }]
            .into(),
            impl_id: None,
        };

        registry
            .trait_implementations
            .insert("Default".to_string(), vec![trait_impl].into());

        registry.detect_common_trait_patterns(&mut call_graph);

        assert!(call_graph.is_entry_point(&default_method));
    }

    #[test]
    fn test_detect_clone_trait_impls() {
        let mut registry = TraitRegistry::new();
        let mut call_graph = CallGraph::new();

        let clone_method = make_function_id("MyType::clone", 20);
        let clone_box_method = make_function_id("MyType::clone_box", 25);

        let trait_impl = TraitImplementation {
            trait_name: "Clone".to_string(),
            implementing_type: "MyType".to_string(),
            method_implementations: vec![
                TraitMethodImplementation {
                    method_name: "clone".to_string(),
                    method_id: clone_method.clone(),
                    overrides_default: false,
                },
                TraitMethodImplementation {
                    method_name: "clone_box".to_string(),
                    method_id: clone_box_method.clone(),
                    overrides_default: false,
                },
            ]
            .into(),
            impl_id: None,
        };

        registry
            .trait_implementations
            .insert("Clone".to_string(), vec![trait_impl].into());

        registry.detect_common_trait_patterns(&mut call_graph);

        assert!(call_graph.is_entry_point(&clone_method));
        assert!(call_graph.is_entry_point(&clone_box_method));
    }

    #[test]
    fn test_detect_constructor_patterns() {
        let registry = TraitRegistry::new();
        let mut call_graph = CallGraph::new();

        let new_method = make_function_id("MyType::new", 10);
        let builder_method = make_function_id("Config::builder", 20);
        let with_method = make_function_id("Settings::with_defaults", 30);
        let create_method = make_function_id("Database::create", 40);
        let normal_method = make_function_id("util::process", 50);

        call_graph.add_function(new_method.clone(), false, false, 1, 10);
        call_graph.add_function(builder_method.clone(), false, false, 1, 10);
        call_graph.add_function(with_method.clone(), false, false, 1, 10);
        call_graph.add_function(create_method.clone(), false, false, 1, 10);
        call_graph.add_function(normal_method.clone(), false, false, 1, 10);

        registry.detect_common_trait_patterns(&mut call_graph);

        assert!(call_graph.is_entry_point(&new_method));
        assert!(call_graph.is_entry_point(&builder_method));
        assert!(call_graph.is_entry_point(&with_method));
        assert!(call_graph.is_entry_point(&create_method));
        assert!(!call_graph.is_entry_point(&normal_method));
    }

    #[test]
    fn test_resolve_trait_method_calls() {
        let mut registry = TraitRegistry::new();
        let mut call_graph = CallGraph::new();

        let caller = make_function_id("create_config", 5);
        call_graph.add_function(caller.clone(), false, false, 1, 10);

        let default_impl = make_function_id("MyConfig::default", 10);
        call_graph.add_function(default_impl.clone(), false, false, 1, 10);

        let trait_impl = TraitImplementation {
            trait_name: "Default".to_string(),
            implementing_type: "MyConfig".to_string(),
            method_implementations: vec![TraitMethodImplementation {
                method_name: "default".to_string(),
                method_id: default_impl.clone(),
                overrides_default: false,
            }]
            .into(),
            impl_id: None,
        };

        registry
            .trait_implementations
            .insert("Default".to_string(), vec![trait_impl].into());

        registry.type_to_traits.insert(
            "MyConfig".to_string(),
            vec!["Default".to_string()].into_iter().collect(),
        );

        registry.unresolved_calls.push_back(TraitMethodCall {
            caller: caller.clone(),
            trait_name: "Default".to_string(),
            method_name: "default".to_string(),
            receiver_type: Some("MyConfig".to_string()),
            line: 6,
        });

        let resolved_count = registry.resolve_trait_method_calls(&mut call_graph);

        assert_eq!(resolved_count, 1);
        assert!(call_graph.get_callees(&caller).contains(&default_impl));
        assert!(call_graph.is_entry_point(&default_impl));
    }
}

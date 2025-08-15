/// Dynamic dispatch resolution for trait method calls
///
/// This module resolves trait object method calls and generic trait bounds
/// to their concrete implementations, enabling accurate call graph construction
/// for polymorphic Rust code.
use crate::analyzers::trait_implementation_tracker::{
    TraitBound, TraitImplementationTracker, TraitObject,
};
use crate::priority::call_graph::FunctionId;
use im::{HashMap, HashSet, Vector};
use std::sync::Arc;

/// Method resolution order following Rust's rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResolutionPriority {
    InherentMethod = 1,
    TraitMethodInScope = 2,
    BlanketImplementation = 3,
    DefaultTraitMethod = 4,
}

/// Result of method resolution
#[derive(Debug, Clone)]
pub struct ResolvedMethod {
    pub function_id: FunctionId,
    pub trait_name: Option<String>,
    pub priority: ResolutionPriority,
    pub confidence: f64,
}

/// Cache for resolved method calls
type ResolutionCache = HashMap<(String, String), Option<ResolvedMethod>>;

/// The main trait resolver
#[derive(Debug)]
pub struct TraitResolver {
    tracker: Arc<TraitImplementationTracker>,
    cache: ResolutionCache,
    inherent_methods: HashMap<String, HashMap<String, FunctionId>>,
}

impl TraitResolver {
    pub fn new(tracker: Arc<TraitImplementationTracker>) -> Self {
        Self {
            tracker,
            cache: HashMap::new(),
            inherent_methods: HashMap::new(),
        }
    }

    /// Register an inherent method (non-trait method)
    pub fn register_inherent_method(
        &mut self,
        type_name: String,
        method_name: String,
        function_id: FunctionId,
    ) {
        self.inherent_methods
            .entry(type_name)
            .or_default()
            .insert(method_name, function_id);
    }

    /// Resolve a method call following Rust's method resolution order
    pub fn resolve_method_call(
        &mut self,
        receiver_type: &str,
        method_name: &str,
        traits_in_scope: &HashSet<String>,
    ) -> Option<ResolvedMethod> {
        let cache_key = (receiver_type.to_string(), method_name.to_string());

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        // Follow Rust's method resolution order
        let result = self
            .resolve_inherent_method(receiver_type, method_name)
            .or_else(|| {
                self.resolve_trait_method_in_scope(receiver_type, method_name, traits_in_scope)
            })
            .or_else(|| self.resolve_blanket_implementation(receiver_type, method_name))
            .or_else(|| self.resolve_default_trait_method(receiver_type, method_name));

        // Cache the result
        self.cache.insert(cache_key, result.clone());
        result
    }

    /// Step 1: Check for inherent methods
    fn resolve_inherent_method(
        &self,
        receiver_type: &str,
        method_name: &str,
    ) -> Option<ResolvedMethod> {
        self.inherent_methods
            .get(receiver_type)
            .and_then(|methods| methods.get(method_name))
            .map(|function_id| ResolvedMethod {
                function_id: function_id.clone(),
                trait_name: None,
                priority: ResolutionPriority::InherentMethod,
                confidence: 1.0,
            })
    }

    /// Step 2: Check trait methods in scope
    fn resolve_trait_method_in_scope(
        &self,
        receiver_type: &str,
        method_name: &str,
        traits_in_scope: &HashSet<String>,
    ) -> Option<ResolvedMethod> {
        // Get all traits implemented by this type
        let type_traits = self.tracker.get_traits_for_type(receiver_type)?;

        // Find traits that are both implemented and in scope
        let available_traits: HashSet<String> = type_traits
            .iter()
            .filter(|trait_name| traits_in_scope.contains(*trait_name))
            .cloned()
            .collect();

        // Look for the method in these traits
        for trait_name in &available_traits {
            if let Some(function_id) =
                self.tracker
                    .resolve_method(receiver_type, trait_name, method_name)
            {
                return Some(ResolvedMethod {
                    function_id,
                    trait_name: Some(trait_name.clone()),
                    priority: ResolutionPriority::TraitMethodInScope,
                    confidence: 0.9,
                });
            }
        }

        None
    }

    /// Step 3: Check blanket implementations
    fn resolve_blanket_implementation(
        &self,
        receiver_type: &str,
        method_name: &str,
    ) -> Option<ResolvedMethod> {
        for blanket_impl in self.tracker.get_blanket_impls() {
            // Check if this blanket implementation applies to the receiver type
            if self.blanket_applies_to_type(blanket_impl, receiver_type) {
                if let Some(method) = blanket_impl.methods.get(method_name) {
                    return Some(ResolvedMethod {
                        function_id: method.function_id.clone(),
                        trait_name: Some(blanket_impl.trait_name.clone()),
                        priority: ResolutionPriority::BlanketImplementation,
                        confidence: 0.7,
                    });
                }
            }
        }

        None
    }

    /// Step 4: Check for default trait methods
    fn resolve_default_trait_method(
        &self,
        receiver_type: &str,
        method_name: &str,
    ) -> Option<ResolvedMethod> {
        // Get all traits implemented by this type
        let type_traits = self.tracker.get_traits_for_type(receiver_type)?;

        for trait_name in type_traits {
            if let Some(trait_def) = self.tracker.get_trait(trait_name) {
                for method in &trait_def.methods {
                    if method.name == method_name && method.has_default {
                        // Check if there's an override
                        if self
                            .tracker
                            .resolve_method(receiver_type, trait_name, method_name)
                            .is_none()
                        {
                            // Use default implementation
                            // Note: In a real implementation, we'd need to track default method IDs
                            return Some(ResolvedMethod {
                                function_id: FunctionId {
                                    file: std::path::PathBuf::from("trait_default"),
                                    name: format!("{}::{}", trait_name, method_name),
                                    line: 0,
                                },
                                trait_name: Some(trait_name.clone()),
                                priority: ResolutionPriority::DefaultTraitMethod,
                                confidence: 0.6,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    /// Check if a blanket implementation applies to a type
    fn blanket_applies_to_type(
        &self,
        blanket_impl: &crate::analyzers::trait_implementation_tracker::Implementation,
        type_name: &str,
    ) -> bool {
        // Simplified check - in reality, we'd need to evaluate generic constraints
        // For now, check if the type satisfies the where clauses
        for constraint in &blanket_impl.generic_constraints {
            // Check if type satisfies the constraint
            for bound in &constraint.bounds {
                if !self.type_satisfies_bound(type_name, bound) {
                    return false;
                }
            }
        }

        true
    }

    /// Check if a type satisfies a trait bound
    fn type_satisfies_bound(&self, type_name: &str, bound: &str) -> bool {
        // Extract trait name from bound (simplified)
        let trait_name = bound.split("::").last().unwrap_or(bound);
        self.tracker.implements_trait(type_name, trait_name)
    }

    /// Resolve a trait object method call to all possible implementations
    pub fn resolve_trait_object_call(
        &self,
        trait_object: &TraitObject,
        method_name: &str,
    ) -> Vector<ResolvedMethod> {
        let mut results = Vector::new();

        // Get all implementors of the trait
        if let Some(implementors) = self.tracker.get_implementors(&trait_object.trait_name) {
            for impl_type in implementors {
                if let Some(function_id) =
                    self.tracker
                        .resolve_method(&impl_type, &trait_object.trait_name, method_name)
                {
                    results.push_back(ResolvedMethod {
                        function_id,
                        trait_name: Some(trait_object.trait_name.clone()),
                        priority: ResolutionPriority::TraitMethodInScope,
                        confidence: 0.8,
                    });
                }
            }
        }

        results
    }

    /// Resolve generic constraint to possible implementations
    pub fn resolve_generic_bound(
        &self,
        bound: &TraitBound,
        method_name: &str,
    ) -> Vector<ResolvedMethod> {
        self.tracker
            .resolve_generic_bound(bound, method_name)
            .into_iter()
            .map(|function_id| ResolvedMethod {
                function_id,
                trait_name: Some(bound.trait_name.clone()),
                priority: ResolutionPriority::TraitMethodInScope,
                confidence: 0.75,
            })
            .collect()
    }

    /// Resolve associated type projection
    pub fn resolve_associated_type(&self, type_name: &str, assoc_type: &str) -> Option<String> {
        self.tracker.resolve_associated_type(type_name, assoc_type)
    }

    /// Clear the resolution cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let total = self.cache.len();
        let hits = self.cache.values().filter(|v| v.is_some()).count();
        (hits, total)
    }

    /// Disambiguate method call using explicit syntax
    pub fn disambiguate_method(
        &self,
        trait_name: &str,
        method_name: &str,
        receiver_type: &str,
    ) -> Option<ResolvedMethod> {
        self.tracker
            .resolve_method(receiver_type, trait_name, method_name)
            .map(|function_id| ResolvedMethod {
                function_id,
                trait_name: Some(trait_name.to_string()),
                priority: ResolutionPriority::TraitMethodInScope,
                confidence: 1.0, // Explicit disambiguation has high confidence
            })
    }

    /// Find all methods with a given name across all traits
    pub fn find_all_methods(&self, method_name: &str) -> Vector<(String, FunctionId)> {
        let mut methods = Vector::new();

        for (trait_name, impls) in self.tracker.implementations.iter() {
            for impl_info in impls {
                if let Some(method) = impl_info.methods.get(method_name) {
                    methods.push_back((trait_name.clone(), method.function_id.clone()));
                }
            }
        }

        methods
    }

    /// Check if a method could be a trait method
    pub fn is_potential_trait_method(&self, method_name: &str) -> bool {
        self.tracker.traits.values().any(|trait_def| {
            trait_def
                .methods
                .iter()
                .any(|method| method.name == method_name)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::trait_implementation_tracker::TraitImplementationTracker;
    use std::path::PathBuf;

    fn create_test_resolver() -> TraitResolver {
        let tracker = Arc::new(TraitImplementationTracker::new());
        TraitResolver::new(tracker)
    }

    #[test]
    fn test_resolver_creation() {
        let resolver = create_test_resolver();
        assert_eq!(resolver.cache_stats(), (0, 0));
    }

    #[test]
    fn test_register_inherent_method() {
        let mut resolver = create_test_resolver();
        let function_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "MyType::my_method".to_string(),
            line: 10,
        };

        resolver.register_inherent_method(
            "MyType".to_string(),
            "my_method".to_string(),
            function_id.clone(),
        );

        let resolved = resolver.resolve_inherent_method("MyType", "my_method");
        assert!(resolved.is_some());
        assert_eq!(
            resolved.unwrap().priority,
            ResolutionPriority::InherentMethod
        );
    }

    #[test]
    fn test_cache_functionality() {
        let mut resolver = create_test_resolver();
        let traits_in_scope = HashSet::new();

        // First call - cache miss
        let _ = resolver.resolve_method_call("MyType", "method", &traits_in_scope);
        assert_eq!(resolver.cache.len(), 1);

        // Second call - cache hit
        let _ = resolver.resolve_method_call("MyType", "method", &traits_in_scope);
        assert_eq!(resolver.cache.len(), 1);

        // Clear cache
        resolver.clear_cache();
        assert_eq!(resolver.cache.len(), 0);
    }

    #[test]
    fn test_resolution_priority_order() {
        assert!(ResolutionPriority::InherentMethod < ResolutionPriority::TraitMethodInScope);
        assert!(ResolutionPriority::TraitMethodInScope < ResolutionPriority::BlanketImplementation);
        assert!(ResolutionPriority::BlanketImplementation < ResolutionPriority::DefaultTraitMethod);
    }

    #[test]
    fn test_is_potential_trait_method() {
        let mut tracker = TraitImplementationTracker::new();
        let trait_def = crate::analyzers::trait_implementation_tracker::TraitDefinition {
            name: "TestTrait".to_string(),
            methods: vec![
                crate::analyzers::trait_implementation_tracker::TraitMethod {
                    name: "test_method".to_string(),
                    has_default: false,
                    is_async: false,
                    signature: "fn test_method(&self)".to_string(),
                },
            ]
            .into(),
            associated_types: Vector::new(),
            supertraits: Vector::new(),
            generic_params: Vector::new(),
            module_path: Vector::new(),
        };

        tracker.register_trait(trait_def);
        let resolver = TraitResolver::new(Arc::new(tracker));

        assert!(resolver.is_potential_trait_method("test_method"));
        assert!(!resolver.is_potential_trait_method("non_existent_method"));
    }
}

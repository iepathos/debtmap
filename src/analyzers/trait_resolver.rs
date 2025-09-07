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
            if let Some(resolved) =
                self.check_trait_default_method(receiver_type, trait_name, method_name)
            {
                return Some(resolved);
            }
        }

        None
    }

    /// Check if a trait has a default method that applies
    fn check_trait_default_method(
        &self,
        receiver_type: &str,
        trait_name: &str,
        method_name: &str,
    ) -> Option<ResolvedMethod> {
        let trait_def = self.tracker.get_trait(trait_name)?;

        let _method = trait_def
            .methods
            .iter()
            .find(|m| Self::is_matching_default_method(m, method_name))?;

        // Check if there's an override
        if self.has_method_override(receiver_type, trait_name, method_name) {
            return None;
        }

        Some(Self::create_default_method_resolution(
            trait_name,
            method_name,
        ))
    }

    /// Check if a method matches and has a default implementation
    fn is_matching_default_method(
        method: &crate::analyzers::trait_implementation_tracker::TraitMethod,
        method_name: &str,
    ) -> bool {
        method.name == method_name && method.has_default
    }

    /// Check if a method has been overridden in the implementation
    fn has_method_override(
        &self,
        receiver_type: &str,
        trait_name: &str,
        method_name: &str,
    ) -> bool {
        self.tracker
            .resolve_method(receiver_type, trait_name, method_name)
            .is_some()
    }

    /// Create a ResolvedMethod for a default trait method
    fn create_default_method_resolution(trait_name: &str, method_name: &str) -> ResolvedMethod {
        ResolvedMethod {
            function_id: FunctionId {
                file: std::path::PathBuf::from("trait_default"),
                name: format!("{}::{}", trait_name, method_name),
                line: 0,
            },
            trait_name: Some(trait_name.to_string()),
            priority: ResolutionPriority::DefaultTraitMethod,
            confidence: 0.6,
        }
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

    #[test]
    fn test_blanket_applies_to_type_no_constraints() {
        let tracker = Arc::new(TraitImplementationTracker::new());
        let resolver = TraitResolver::new(tracker);

        // Create a blanket implementation with no constraints
        let blanket_impl = crate::analyzers::trait_implementation_tracker::Implementation {
            trait_name: "Display".to_string(),
            implementing_type: "T".to_string(),
            methods: HashMap::new(),
            generic_constraints: Vector::new(),
            is_blanket: true,
            is_negative: false,
            module_path: Vector::new(),
        };

        // Should apply to any type when there are no constraints
        assert!(resolver.blanket_applies_to_type(&blanket_impl, "String"));
        assert!(resolver.blanket_applies_to_type(&blanket_impl, "MyType"));
        assert!(resolver.blanket_applies_to_type(&blanket_impl, "i32"));
    }

    #[test]
    fn test_blanket_applies_to_type_with_constraints() {
        let mut tracker = TraitImplementationTracker::new();

        // Register that String implements Clone
        tracker.register_implementation(
            crate::analyzers::trait_implementation_tracker::Implementation {
                trait_name: "Clone".to_string(),
                implementing_type: "String".to_string(),
                methods: HashMap::new(),
                generic_constraints: Vector::new(),
                is_blanket: false,
                is_negative: false,
                module_path: Vector::new(),
            },
        );

        let resolver = TraitResolver::new(Arc::new(tracker));

        // Create a blanket implementation with a Clone constraint
        let blanket_impl = crate::analyzers::trait_implementation_tracker::Implementation {
            trait_name: "Debug".to_string(),
            implementing_type: "T".to_string(),
            methods: HashMap::new(),
            generic_constraints: vec![
                crate::analyzers::trait_implementation_tracker::WhereClauseItem {
                    type_param: "T".to_string(),
                    bounds: vec!["Clone".to_string()].into(),
                },
            ]
            .into(),
            is_blanket: true,
            is_negative: false,
            module_path: Vector::new(),
        };

        // Should apply to String (implements Clone)
        assert!(resolver.blanket_applies_to_type(&blanket_impl, "String"));

        // Should not apply to a type that doesn't implement Clone
        assert!(!resolver.blanket_applies_to_type(&blanket_impl, "UnknownType"));
    }

    #[test]
    fn test_blanket_applies_to_type_multiple_constraints() {
        let mut tracker = TraitImplementationTracker::new();

        // Register that String implements both Clone and Send
        tracker.register_implementation(
            crate::analyzers::trait_implementation_tracker::Implementation {
                trait_name: "Clone".to_string(),
                implementing_type: "String".to_string(),
                methods: HashMap::new(),
                generic_constraints: Vector::new(),
                is_blanket: false,
                is_negative: false,
                module_path: Vector::new(),
            },
        );

        tracker.register_implementation(
            crate::analyzers::trait_implementation_tracker::Implementation {
                trait_name: "Send".to_string(),
                implementing_type: "String".to_string(),
                methods: HashMap::new(),
                generic_constraints: Vector::new(),
                is_blanket: false,
                is_negative: false,
                module_path: Vector::new(),
            },
        );

        // Register that MyType only implements Clone
        tracker.register_implementation(
            crate::analyzers::trait_implementation_tracker::Implementation {
                trait_name: "Clone".to_string(),
                implementing_type: "MyType".to_string(),
                methods: HashMap::new(),
                generic_constraints: Vector::new(),
                is_blanket: false,
                is_negative: false,
                module_path: Vector::new(),
            },
        );

        let resolver = TraitResolver::new(Arc::new(tracker));

        // Create a blanket implementation with multiple constraints
        let blanket_impl = crate::analyzers::trait_implementation_tracker::Implementation {
            trait_name: "Debug".to_string(),
            implementing_type: "T".to_string(),
            methods: HashMap::new(),
            generic_constraints: vec![
                crate::analyzers::trait_implementation_tracker::WhereClauseItem {
                    type_param: "T".to_string(),
                    bounds: vec!["Clone".to_string(), "Send".to_string()].into(),
                },
            ]
            .into(),
            is_blanket: true,
            is_negative: false,
            module_path: Vector::new(),
        };

        // Should apply to String (implements both Clone and Send)
        assert!(resolver.blanket_applies_to_type(&blanket_impl, "String"));

        // Should not apply to MyType (only implements Clone, not Send)
        assert!(!resolver.blanket_applies_to_type(&blanket_impl, "MyType"));

        // Should not apply to unknown types
        assert!(!resolver.blanket_applies_to_type(&blanket_impl, "UnknownType"));
    }

    #[test]
    fn test_type_satisfies_bound_simple() {
        let mut tracker = TraitImplementationTracker::new();

        // Register trait implementations
        tracker.register_implementation(
            crate::analyzers::trait_implementation_tracker::Implementation {
                trait_name: "Display".to_string(),
                implementing_type: "String".to_string(),
                methods: HashMap::new(),
                generic_constraints: Vector::new(),
                is_blanket: false,
                is_negative: false,
                module_path: Vector::new(),
            },
        );

        let resolver = TraitResolver::new(Arc::new(tracker));

        // Test simple trait bound
        assert!(resolver.type_satisfies_bound("String", "Display"));
        assert!(!resolver.type_satisfies_bound("String", "UnknownTrait"));
        assert!(!resolver.type_satisfies_bound("UnknownType", "Display"));
    }

    #[test]
    fn test_type_satisfies_bound_with_path() {
        let mut tracker = TraitImplementationTracker::new();

        // Register trait implementations
        tracker.register_implementation(
            crate::analyzers::trait_implementation_tracker::Implementation {
                trait_name: "IntoIterator".to_string(),
                implementing_type: "Vec<T>".to_string(),
                methods: HashMap::new(),
                generic_constraints: Vector::new(),
                is_blanket: false,
                is_negative: false,
                module_path: Vector::new(),
            },
        );

        let resolver = TraitResolver::new(Arc::new(tracker));

        // Test trait bound with module path (should extract last part)
        assert!(resolver.type_satisfies_bound("Vec<T>", "std::iter::IntoIterator"));
        assert!(resolver.type_satisfies_bound("Vec<T>", "core::iter::IntoIterator"));
        assert!(resolver.type_satisfies_bound("Vec<T>", "IntoIterator"));
    }

    #[test]
    fn test_type_satisfies_bound_edge_cases() {
        let tracker = Arc::new(TraitImplementationTracker::new());
        let resolver = TraitResolver::new(tracker);

        // Test edge cases
        assert!(!resolver.type_satisfies_bound("", "Display"));
        assert!(!resolver.type_satisfies_bound("String", ""));
        assert!(!resolver.type_satisfies_bound("", ""));
    }

    #[test]
    fn test_is_matching_default_method() {
        // Test method with default implementation
        let method_with_default = crate::analyzers::trait_implementation_tracker::TraitMethod {
            name: "test_method".to_string(),
            has_default: true,
            is_async: false,
            signature: "fn test_method(&self)".to_string(),
        };
        assert!(TraitResolver::is_matching_default_method(
            &method_with_default,
            "test_method"
        ));
        assert!(!TraitResolver::is_matching_default_method(
            &method_with_default,
            "other_method"
        ));

        // Test method without default implementation
        let method_without_default = crate::analyzers::trait_implementation_tracker::TraitMethod {
            name: "test_method".to_string(),
            has_default: false,
            is_async: false,
            signature: "fn test_method(&self)".to_string(),
        };
        assert!(!TraitResolver::is_matching_default_method(
            &method_without_default,
            "test_method"
        ));
    }

    #[test]
    fn test_create_default_method_resolution() {
        let resolved = TraitResolver::create_default_method_resolution("MyTrait", "my_method");

        assert_eq!(resolved.function_id.file, PathBuf::from("trait_default"));
        assert_eq!(resolved.function_id.name, "MyTrait::my_method");
        assert_eq!(resolved.function_id.line, 0);
        assert_eq!(resolved.trait_name, Some("MyTrait".to_string()));
        assert_eq!(resolved.priority, ResolutionPriority::DefaultTraitMethod);
        assert_eq!(resolved.confidence, 0.6);
    }

    #[test]
    fn test_has_method_override() {
        let mut tracker = TraitImplementationTracker::new();

        // Register a trait
        let trait_def = crate::analyzers::trait_implementation_tracker::TraitDefinition {
            name: "TestTrait".to_string(),
            methods: vec![
                crate::analyzers::trait_implementation_tracker::TraitMethod {
                    name: "test_method".to_string(),
                    has_default: true,
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

        // Register an implementation with an override
        let mut methods = HashMap::new();
        methods.insert(
            "test_method".to_string(),
            crate::analyzers::trait_implementation_tracker::MethodImpl {
                name: "test_method".to_string(),
                function_id: FunctionId {
                    file: PathBuf::from("test.rs"),
                    name: "MyType::test_method".to_string(),
                    line: 10,
                },
                overrides_default: true,
            },
        );

        tracker.register_implementation(
            crate::analyzers::trait_implementation_tracker::Implementation {
                trait_name: "TestTrait".to_string(),
                implementing_type: "MyType".to_string(),
                methods,
                generic_constraints: Vector::new(),
                is_blanket: false,
                is_negative: false,
                module_path: Vector::new(),
            },
        );

        let resolver = TraitResolver::new(Arc::new(tracker));

        // Should find override
        assert!(resolver.has_method_override("MyType", "TestTrait", "test_method"));

        // Should not find override for different type
        assert!(!resolver.has_method_override("OtherType", "TestTrait", "test_method"));

        // Should not find override for non-existent method
        assert!(!resolver.has_method_override("MyType", "TestTrait", "non_existent"));
    }

    #[test]
    fn test_check_trait_default_method_with_override() {
        let mut tracker = TraitImplementationTracker::new();

        // Register a trait with default method
        let trait_def = crate::analyzers::trait_implementation_tracker::TraitDefinition {
            name: "TestTrait".to_string(),
            methods: vec![
                crate::analyzers::trait_implementation_tracker::TraitMethod {
                    name: "default_method".to_string(),
                    has_default: true,
                    is_async: false,
                    signature: "fn default_method(&self)".to_string(),
                },
            ]
            .into(),
            associated_types: Vector::new(),
            supertraits: Vector::new(),
            generic_params: Vector::new(),
            module_path: Vector::new(),
        };
        tracker.register_trait(trait_def);

        // Register implementation with override
        let mut methods = HashMap::new();
        methods.insert(
            "default_method".to_string(),
            crate::analyzers::trait_implementation_tracker::MethodImpl {
                name: "default_method".to_string(),
                function_id: FunctionId {
                    file: PathBuf::from("test.rs"),
                    name: "MyType::default_method".to_string(),
                    line: 20,
                },
                overrides_default: true,
            },
        );

        tracker.register_implementation(
            crate::analyzers::trait_implementation_tracker::Implementation {
                trait_name: "TestTrait".to_string(),
                implementing_type: "MyType".to_string(),
                methods,
                generic_constraints: Vector::new(),
                is_blanket: false,
                is_negative: false,
                module_path: Vector::new(),
            },
        );

        let resolver = TraitResolver::new(Arc::new(tracker));

        // Should return None because method is overridden
        let result = resolver.check_trait_default_method("MyType", "TestTrait", "default_method");
        assert!(result.is_none());

        // Should return Some for type without override
        let result =
            resolver.check_trait_default_method("OtherType", "TestTrait", "default_method");
        assert!(result.is_some());

        if let Some(resolved) = result {
            assert_eq!(resolved.priority, ResolutionPriority::DefaultTraitMethod);
            assert_eq!(resolved.trait_name, Some("TestTrait".to_string()));
        }
    }

    #[test]
    fn test_resolve_default_trait_method_complete() {
        let mut tracker = TraitImplementationTracker::new();

        // Register trait with default method
        let trait_def = crate::analyzers::trait_implementation_tracker::TraitDefinition {
            name: "DefaultTrait".to_string(),
            methods: vec![
                crate::analyzers::trait_implementation_tracker::TraitMethod {
                    name: "default_fn".to_string(),
                    has_default: true,
                    is_async: false,
                    signature: "fn default_fn(&self)".to_string(),
                },
            ]
            .into(),
            associated_types: Vector::new(),
            supertraits: Vector::new(),
            generic_params: Vector::new(),
            module_path: Vector::new(),
        };
        tracker.register_trait(trait_def);

        // Register implementation without override
        tracker.register_implementation(
            crate::analyzers::trait_implementation_tracker::Implementation {
                trait_name: "DefaultTrait".to_string(),
                implementing_type: "MyStruct".to_string(),
                methods: HashMap::new(), // No override
                generic_constraints: Vector::new(),
                is_blanket: false,
                is_negative: false,
                module_path: Vector::new(),
            },
        );

        let resolver = TraitResolver::new(Arc::new(tracker));

        // Should resolve to default method
        let result = resolver.resolve_default_trait_method("MyStruct", "default_fn");
        assert!(result.is_some());

        if let Some(resolved) = result {
            assert_eq!(resolved.priority, ResolutionPriority::DefaultTraitMethod);
            assert_eq!(resolved.trait_name, Some("DefaultTrait".to_string()));
            assert_eq!(resolved.confidence, 0.6);
        }

        // Should return None for non-existent method
        let result = resolver.resolve_default_trait_method("MyStruct", "non_existent");
        assert!(result.is_none());
    }
}

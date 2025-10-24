//! Trait Registry for Enhanced Call Graph Analysis
//!
//! This module tracks trait implementations and resolves trait method calls
//! to their concrete implementations, reducing false positives in dead code detection.
//!
//! This module integrates with the new trait_implementation_tracker and trait_resolver
//! modules for comprehensive trait tracking and resolution.

use crate::analyzers::trait_implementation_tracker::{TraitExtractor, TraitImplementationTracker};
use crate::analyzers::trait_resolver::TraitResolver;
use crate::priority::call_graph::FunctionId;
use anyhow::Result;
use im::{HashMap, HashSet, Vector};
use std::path::Path;
use std::sync::Arc;
use syn::visit::Visit;
use syn::{File, ImplItem, ItemImpl, ItemTrait, Path as SynPath, TraitItem, Type, TypePath};

/// Information about a trait method
#[derive(Debug, Clone)]
pub struct TraitMethod {
    /// The trait this method belongs to
    pub trait_name: String,
    /// Method name
    pub method_name: String,
    /// Function ID for this method definition
    pub method_id: FunctionId,
    /// Whether this method has a default implementation
    pub has_default: bool,
}

/// Information about a trait implementation
#[derive(Debug, Clone)]
pub struct TraitImplementation {
    /// The trait being implemented
    pub trait_name: String,
    /// The type implementing the trait
    pub implementing_type: String,
    /// Method implementations
    pub method_implementations: Vector<TraitMethodImplementation>,
    /// Function ID for the impl block (if available)
    pub impl_id: Option<FunctionId>,
}

/// Information about a specific trait method implementation
#[derive(Debug, Clone)]
pub struct TraitMethodImplementation {
    /// Method name
    pub method_name: String,
    /// Function ID for this implementation
    pub method_id: FunctionId,
    /// Whether this overrides a default implementation
    pub overrides_default: bool,
}

/// Information about an unresolved trait method call
#[derive(Debug, Clone)]
pub struct TraitMethodCall {
    /// The caller of the trait method
    pub caller: FunctionId,
    /// The trait name (if determinable)
    pub trait_name: String,
    /// The method name being called
    pub method_name: String,
    /// The receiver type (if determinable)
    pub receiver_type: Option<String>,
    /// Line number of the call
    pub line: usize,
}

/// Registry for tracking trait definitions, implementations, and method calls
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
        // Use the enhanced trait extractor for comprehensive analysis
        let mut extractor = TraitExtractor::new(file_path.to_path_buf());
        let _extracted_tracker = extractor.extract(ast);

        // Merge extracted data into our enhanced tracker
        // Note: In a real implementation, we'd need a merge method
        // For now, we'll continue with the existing visitor pattern
        let mut visitor = TraitVisitor::new(file_path.to_path_buf());
        visitor.visit_file(ast);

        // Add discovered traits and implementations
        for trait_def in visitor.trait_definitions {
            let trait_name = trait_def.0;
            let methods = trait_def.1;
            self.trait_definitions.insert(trait_name, methods);
        }

        for trait_impl in visitor.trait_implementations {
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

        // Add unresolved calls
        for call in visitor.trait_method_calls {
            self.unresolved_calls.push_back(call);
        }

        // Add Visit trait methods
        for method_id in visitor.visit_trait_methods {
            self.visit_trait_methods.insert(method_id);
        }

        // Add Visit implementations
        for (type_name, methods) in visitor.visit_implementations {
            self.visit_implementations.insert(type_name, methods);
        }

        Ok(())
    }

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

    /// Extract method implementations matching a specific method name
    fn extract_matching_methods(
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

    /// Collect implementations for known receiver type
    fn collect_typed_implementations(
        &self,
        receiver_type: &str,
        method_name: &str,
        traits: &HashSet<String>,
    ) -> Vector<FunctionId> {
        traits
            .iter()
            .filter_map(|trait_name| self.trait_implementations.get(trait_name))
            .flat_map(|impls| {
                Self::extract_matching_methods(method_name, impls, Some(receiver_type))
            })
            .collect()
    }

    /// Resolve a trait method call to possible implementations
    pub fn resolve_trait_call(&self, call: &TraitMethodCall) -> Vector<FunctionId> {
        // Try using the enhanced tracker first for better resolution
        if let Some(receiver_type) = &call.receiver_type {
            // Check if we can resolve through the enhanced tracker
            if let Some(method_id) = self.enhanced_tracker.resolve_method(
                receiver_type,
                &call.trait_name,
                &call.method_name,
            ) {
                return vec![method_id].into();
            }

            // Fall back to existing resolution
            self.find_implementations_for_type(receiver_type)
                .map(|traits| {
                    self.collect_typed_implementations(receiver_type, &call.method_name, traits)
                })
                .unwrap_or_default()
        } else {
            // Use enhanced tracker for trait object resolution
            self.enhanced_tracker
                .resolve_trait_object_call(&call.trait_name, &call.method_name)
        }
    }

    /// Get statistics about trait usage
    pub fn get_statistics(&self) -> TraitStatistics {
        let total_traits = self.trait_definitions.len();
        let total_implementations = self
            .trait_implementations
            .values()
            .map(|impls| impls.len())
            .sum();
        let total_unresolved_calls = self.unresolved_calls.len();

        TraitStatistics {
            total_traits,
            total_implementations,
            total_unresolved_calls,
        }
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

    /// Resolve trait method calls and add edges to call graph
    /// Returns the number of trait method calls resolved
    pub fn resolve_trait_method_calls(
        &self,
        call_graph: &mut crate::priority::call_graph::CallGraph,
    ) -> usize {
        let mut resolved_count = 0;

        for call in self.unresolved_calls.iter() {
            let implementations = self.resolve_trait_call(call);

            for impl_method_id in implementations.iter() {
                // Add edge from caller to trait implementation
                call_graph.add_call_parts(
                    call.caller.clone(),
                    impl_method_id.clone(),
                    crate::priority::call_graph::CallType::Direct,
                );

                // Mark as reachable through trait dispatch
                call_graph.mark_as_trait_dispatch(impl_method_id.clone());

                resolved_count += 1;
            }
        }

        resolved_count
    }

    /// Detect common trait patterns and mark them as entry points
    pub fn detect_common_trait_patterns(
        &self,
        call_graph: &mut crate::priority::call_graph::CallGraph,
    ) {
        self.detect_default_trait_impls(call_graph);
        self.detect_clone_trait_impls(call_graph);
        self.detect_constructor_patterns(call_graph);
        self.detect_from_into_impls(call_graph);
        self.detect_display_debug_impls(call_graph);
    }

    /// Detect Default trait implementations
    fn detect_default_trait_impls(&self, call_graph: &mut crate::priority::call_graph::CallGraph) {
        if let Some(impls) = self.trait_implementations.get("Default") {
            for trait_impl in impls.iter() {
                for method_impl in trait_impl.method_implementations.iter() {
                    if method_impl.method_name == "default" {
                        call_graph.mark_as_trait_dispatch(method_impl.method_id.clone());
                    }
                }
            }
        }
    }

    /// Detect Clone trait implementations
    fn detect_clone_trait_impls(&self, call_graph: &mut crate::priority::call_graph::CallGraph) {
        if let Some(impls) = self.trait_implementations.get("Clone") {
            for trait_impl in impls.iter() {
                for method_impl in trait_impl.method_implementations.iter() {
                    if method_impl.method_name == "clone"
                        || method_impl.method_name == "clone_box"
                        || method_impl.method_name == "clone_from"
                    {
                        call_graph.mark_as_trait_dispatch(method_impl.method_id.clone());
                    }
                }
            }
        }
    }

    /// Detect constructor patterns (Type::new, Type::builder, etc.)
    fn detect_constructor_patterns(&self, call_graph: &mut crate::priority::call_graph::CallGraph) {
        // Collect only the matching function IDs (not full clones)
        let matching_functions: Vec<_> = call_graph
            .get_all_functions()
            .filter(|f| {
                f.name.ends_with("::new")
                    || f.name == "new"
                    || f.name.ends_with("::builder")
                    || f.name.contains("::with_")
                    || f.name.ends_with("::create")
            })
            .cloned()
            .collect();

        // Now mark them (much smaller set, ~200-500 instead of 4,991)
        for function in matching_functions {
            call_graph.mark_as_trait_dispatch(function);
        }
    }

    /// Detect From/Into trait implementations
    fn detect_from_into_impls(&self, call_graph: &mut crate::priority::call_graph::CallGraph) {
        for trait_name in &["From", "Into"] {
            if let Some(impls) = self.trait_implementations.get(*trait_name) {
                for trait_impl in impls.iter() {
                    for method_impl in trait_impl.method_implementations.iter() {
                        call_graph.mark_as_trait_dispatch(method_impl.method_id.clone());
                    }
                }
            }
        }
    }

    /// Detect Display/Debug trait implementations
    fn detect_display_debug_impls(&self, call_graph: &mut crate::priority::call_graph::CallGraph) {
        for trait_name in &["Display", "Debug"] {
            if let Some(impls) = self.trait_implementations.get(*trait_name) {
                for trait_impl in impls.iter() {
                    for method_impl in trait_impl.method_implementations.iter() {
                        if method_impl.method_name == "fmt" {
                            call_graph.mark_as_trait_dispatch(method_impl.method_id.clone());
                        }
                    }
                }
            }
        }
    }
}

/// Statistics about trait usage in the codebase
#[derive(Debug, Clone)]
pub struct TraitStatistics {
    pub total_traits: usize,
    pub total_implementations: usize,
    pub total_unresolved_calls: usize,
}

/// Visitor for extracting trait information from AST
struct TraitVisitor {
    file_path: std::path::PathBuf,
    trait_definitions: Vec<(String, Vector<TraitMethod>)>,
    trait_implementations: Vec<TraitImplementation>,
    trait_method_calls: Vec<TraitMethodCall>,
    current_function: Option<FunctionId>,
    visit_trait_methods: HashSet<FunctionId>,
    visit_implementations: HashMap<String, Vector<TraitMethodImplementation>>,
}

impl TraitVisitor {
    fn new(file_path: std::path::PathBuf) -> Self {
        Self {
            file_path,
            trait_definitions: Vec::new(),
            trait_implementations: Vec::new(),
            trait_method_calls: Vec::new(),
            current_function: None,
            visit_trait_methods: HashSet::new(),
            visit_implementations: HashMap::new(),
        }
    }

    fn extract_type_name(&self, ty: &Type) -> Option<String> {
        match ty {
            Type::Path(TypePath { path, .. }) => self.extract_path_name(path),
            _ => None,
        }
    }

    fn extract_path_name(&self, path: &SynPath) -> Option<String> {
        if path.segments.len() == 1 {
            Some(path.segments.first()?.ident.to_string())
        } else {
            // For multi-segment paths, join with ::
            let segments: Vec<String> = path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect();
            Some(segments.join("::"))
        }
    }

    /// Extract method implementations from impl items
    fn extract_method_implementations(
        &mut self,
        items: &[ImplItem],
        implementing_type: &str,
        is_visit_trait: bool,
    ) -> Vector<TraitMethodImplementation> {
        let mut method_implementations = Vector::new();

        for impl_item in items {
            if let ImplItem::Fn(method) = impl_item {
                let method_name = method.sig.ident.to_string();
                let line = self.get_line_number(method.sig.ident.span());

                let method_id = FunctionId::new(
                    self.file_path.clone(),
                    format!("{implementing_type}::{method_name}"),
                    line,
                );

                let implementation = TraitMethodImplementation {
                    method_name,
                    method_id: method_id.clone(),
                    overrides_default: false, // We'd need more analysis to determine this
                };

                method_implementations.push_back(implementation);

                // Special handling for Visit trait implementations
                if is_visit_trait {
                    self.visit_trait_methods.insert(method_id);
                }
            }
        }

        method_implementations
    }

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    fn is_visit_trait(&self, trait_name: &str) -> bool {
        Self::is_visitor_pattern_trait(trait_name)
    }

    /// Pure function to classify if a trait name represents a visitor pattern trait
    fn is_visitor_pattern_trait(trait_name: &str) -> bool {
        Self::is_generic_visitor_trait(trait_name) || Self::is_qualified_visitor_trait(trait_name)
    }

    /// Checks if the trait name is a generic visitor pattern (Visit, Visitor, or generic variants)
    fn is_generic_visitor_trait(trait_name: &str) -> bool {
        matches!(trait_name, "Visit" | "Visitor")
            || trait_name.starts_with("Visit<")
            || trait_name.starts_with("Visitor<")
    }

    /// Checks if the trait name is a fully qualified visitor trait from known libraries
    fn is_qualified_visitor_trait(trait_name: &str) -> bool {
        matches!(trait_name, "syn::visit::Visit" | "quote::visit::Visit")
    }
}

impl<'ast> Visit<'ast> for TraitVisitor {
    fn visit_item_trait(&mut self, item: &'ast ItemTrait) {
        let trait_name = item.ident.to_string();
        let mut methods = Vector::new();

        for trait_item in &item.items {
            if let TraitItem::Fn(method) = trait_item {
                let method_name = method.sig.ident.to_string();
                let line = self.get_line_number(method.sig.ident.span());

                let method_id = FunctionId::new(
                    self.file_path.clone(),
                    format!("{trait_name}::{method_name}"),
                    line,
                );

                let trait_method = TraitMethod {
                    trait_name: trait_name.clone(),
                    method_name,
                    method_id,
                    has_default: method.default.is_some(),
                };

                methods.push_back(trait_method);
            }
        }

        self.trait_definitions.push((trait_name, methods));

        // Continue visiting
        syn::visit::visit_item_trait(self, item);
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        // Check if this is a trait implementation
        if let Some((_, trait_path, _)) = &item.trait_ {
            if let Some(trait_name) = self.extract_path_name(trait_path) {
                if let Some(implementing_type) = self.extract_type_name(&item.self_ty) {
                    let is_visit_trait = self.is_visit_trait(&trait_name);

                    // Extract method implementations
                    let method_implementations = self.extract_method_implementations(
                        &item.items,
                        &implementing_type,
                        is_visit_trait,
                    );

                    // Store Visit implementations separately for special handling
                    if is_visit_trait {
                        self.visit_implementations
                            .entry(implementing_type.clone())
                            .or_default()
                            .extend(method_implementations.clone());
                    }

                    let trait_impl = TraitImplementation {
                        trait_name,
                        implementing_type,
                        method_implementations,
                        impl_id: None, // Could be enhanced to track impl blocks
                    };

                    self.trait_implementations.push(trait_impl);
                }
            }
        }

        // Continue visiting
        syn::visit::visit_item_impl(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let func_name = item.sig.ident.to_string();
        let line = self.get_line_number(item.sig.ident.span());

        self.current_function = Some(FunctionId::new(self.file_path.clone(), func_name, line));

        // Continue visiting the function body
        syn::visit::visit_item_fn(self, item);

        self.current_function = None;
    }

    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        if let Some(caller) = &self.current_function {
            let method_name = expr.method.to_string();
            let line = self.get_line_number(expr.method.span());

            // This is a simplified heuristic - in a real implementation,
            // we'd need more sophisticated type analysis to determine
            // if this is actually a trait method call
            let trait_call = TraitMethodCall {
                caller: caller.clone(),
                trait_name: "Unknown".to_string(), // Would need type inference
                method_name,
                receiver_type: None, // Would need type analysis
                line,
            };

            self.trait_method_calls.push(trait_call);
        }

        // Continue visiting
        syn::visit::visit_expr_method_call(self, expr);
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

    #[test]
    fn test_is_generic_visitor_trait_simple_names() {
        assert!(TraitVisitor::is_generic_visitor_trait("Visit"));
        assert!(TraitVisitor::is_generic_visitor_trait("Visitor"));
    }

    #[test]
    fn test_is_generic_visitor_trait_with_generics() {
        assert!(TraitVisitor::is_generic_visitor_trait("Visit<T>"));
        assert!(TraitVisitor::is_generic_visitor_trait("Visitor<'a>"));
        assert!(TraitVisitor::is_generic_visitor_trait("Visit<T, U>"));
        assert!(TraitVisitor::is_generic_visitor_trait("Visitor<'a, T>"));
    }

    #[test]
    fn test_is_generic_visitor_trait_negative_cases() {
        assert!(!TraitVisitor::is_generic_visitor_trait(""));
        assert!(!TraitVisitor::is_generic_visitor_trait("MyTrait"));
        assert!(!TraitVisitor::is_generic_visitor_trait("VisitData"));
        assert!(!TraitVisitor::is_generic_visitor_trait("VisitorPattern"));
        assert!(!TraitVisitor::is_generic_visitor_trait("syn::visit::Visit"));
    }

    #[test]
    fn test_is_qualified_visitor_trait_known_libraries() {
        assert!(TraitVisitor::is_qualified_visitor_trait(
            "syn::visit::Visit"
        ));
        assert!(TraitVisitor::is_qualified_visitor_trait(
            "quote::visit::Visit"
        ));
    }

    #[test]
    fn test_is_qualified_visitor_trait_negative_cases() {
        assert!(!TraitVisitor::is_qualified_visitor_trait("Visit"));
        assert!(!TraitVisitor::is_qualified_visitor_trait(
            "other::visit::Visit"
        ));
        assert!(!TraitVisitor::is_qualified_visitor_trait("custom::Visitor"));
        assert!(!TraitVisitor::is_qualified_visitor_trait(""));
    }

    #[test]
    fn test_is_visitor_pattern_trait_comprehensive() {
        // Generic visitor traits
        assert!(TraitVisitor::is_visitor_pattern_trait("Visit"));
        assert!(TraitVisitor::is_visitor_pattern_trait("Visitor"));
        assert!(TraitVisitor::is_visitor_pattern_trait("Visit<T>"));
        assert!(TraitVisitor::is_visitor_pattern_trait("Visitor<'a>"));

        // Qualified visitor traits
        assert!(TraitVisitor::is_visitor_pattern_trait("syn::visit::Visit"));
        assert!(TraitVisitor::is_visitor_pattern_trait(
            "quote::visit::Visit"
        ));

        // Non-visitor traits
        assert!(!TraitVisitor::is_visitor_pattern_trait("Debug"));
        assert!(!TraitVisitor::is_visitor_pattern_trait("Clone"));
        assert!(!TraitVisitor::is_visitor_pattern_trait("VisitData"));
        assert!(!TraitVisitor::is_visitor_pattern_trait("MyVisitor"));
        assert!(!TraitVisitor::is_visitor_pattern_trait(""));
    }

    #[test]
    fn test_detect_default_trait_impls() {
        use crate::priority::call_graph::{CallGraph, FunctionId};
        use std::path::PathBuf;

        let mut registry = TraitRegistry::new();
        let mut call_graph = CallGraph::new();

        // Add a Default trait implementation
        let file_path = PathBuf::from("test.rs");
        let default_method =
            FunctionId::new(file_path.clone(), "MyConfig::default".to_string(), 10);

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

        // Detect Default trait implementations
        registry.detect_default_trait_impls(&mut call_graph);

        // Verify the function was marked as entry point (trait dispatch)
        assert!(call_graph.is_entry_point(&default_method));
    }

    #[test]
    fn test_detect_clone_trait_impls() {
        use crate::priority::call_graph::{CallGraph, FunctionId};
        use std::path::PathBuf;

        let mut registry = TraitRegistry::new();
        let mut call_graph = CallGraph::new();

        let file_path = PathBuf::from("test.rs");
        let clone_method = FunctionId::new(file_path.clone(), "MyType::clone".to_string(), 20);
        let clone_box_method =
            FunctionId::new(file_path.clone(), "MyType::clone_box".to_string(), 25);

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

        registry.detect_clone_trait_impls(&mut call_graph);

        // Both methods should be marked as entry points
        assert!(call_graph.is_entry_point(&clone_method));
        assert!(call_graph.is_entry_point(&clone_box_method));
    }

    #[test]
    fn test_detect_constructor_patterns() {
        use crate::priority::call_graph::{CallGraph, FunctionId};
        use std::path::PathBuf;

        let registry = TraitRegistry::new();
        let mut call_graph = CallGraph::new();

        let file_path = PathBuf::from("test.rs");

        // Add various constructor patterns
        let new_method = FunctionId::new(file_path.clone(), "MyType::new".to_string(), 10);
        let builder_method = FunctionId::new(file_path.clone(), "Config::builder".to_string(), 20);
        let with_method =
            FunctionId::new(file_path.clone(), "Settings::with_defaults".to_string(), 30);
        let create_method = FunctionId::new(file_path.clone(), "Database::create".to_string(), 40);
        let normal_method = FunctionId::new(file_path.clone(), "util::process".to_string(), 50);

        call_graph.add_function(new_method.clone(), false, false, 1, 10);
        call_graph.add_function(builder_method.clone(), false, false, 1, 10);
        call_graph.add_function(with_method.clone(), false, false, 1, 10);
        call_graph.add_function(create_method.clone(), false, false, 1, 10);
        call_graph.add_function(normal_method.clone(), false, false, 1, 10);

        registry.detect_constructor_patterns(&mut call_graph);

        // Constructor patterns should be marked as entry points
        assert!(call_graph.is_entry_point(&new_method));
        assert!(call_graph.is_entry_point(&builder_method));
        assert!(call_graph.is_entry_point(&with_method));
        assert!(call_graph.is_entry_point(&create_method));

        // Normal methods should not be marked
        assert!(!call_graph.is_entry_point(&normal_method));
    }

    #[test]
    fn test_resolve_trait_method_calls() {
        use crate::priority::call_graph::{CallGraph, FunctionId};
        use std::path::PathBuf;

        let mut registry = TraitRegistry::new();
        let mut call_graph = CallGraph::new();

        let file_path = PathBuf::from("test.rs");

        // Create a caller function
        let caller = FunctionId::new(file_path.clone(), "create_config".to_string(), 5);
        call_graph.add_function(caller.clone(), false, false, 1, 10);

        // Create a Default trait implementation
        let default_impl = FunctionId::new(file_path.clone(), "MyConfig::default".to_string(), 10);
        // Add implementation to call graph so it can be found
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

        // Also need to add the type-to-trait mapping for resolution
        registry.type_to_traits.insert(
            "MyConfig".to_string(),
            vec!["Default".to_string()].into_iter().collect(),
        );

        // Add an unresolved trait call
        registry.unresolved_calls.push_back(TraitMethodCall {
            caller: caller.clone(),
            trait_name: "Default".to_string(),
            method_name: "default".to_string(),
            receiver_type: Some("MyConfig".to_string()),
            line: 6,
        });

        // Resolve trait method calls
        let resolved_count = registry.resolve_trait_method_calls(&mut call_graph);

        // Should have resolved one call
        assert_eq!(resolved_count, 1);

        // Should have added an edge from caller to implementation
        let callees = call_graph.get_callees(&caller);
        assert!(callees.contains(&default_impl));

        // Implementation should be marked as entry point
        assert!(call_graph.is_entry_point(&default_impl));
    }
}

//! Trait Registry for Enhanced Call Graph Analysis
//!
//! This module tracks trait implementations and resolves trait method calls
//! to their concrete implementations, reducing false positives in dead code detection.

use crate::priority::call_graph::FunctionId;
use anyhow::Result;
use im::{HashMap, HashSet, Vector};
use std::path::Path;
use syn::visit::Visit;
use syn::{
    File, ImplItem, ItemImpl, ItemTrait, Path as SynPath, TraitItem,
    Type, TypePath,
};

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
}

impl TraitRegistry {
    /// Create a new trait registry
    pub fn new() -> Self {
        Self {
            trait_definitions: HashMap::new(),
            trait_implementations: HashMap::new(),
            unresolved_calls: Vector::new(),
            type_to_traits: HashMap::new(),
        }
    }

    /// Analyze a file for trait definitions and implementations
    pub fn analyze_file(&mut self, file_path: &Path, ast: &File) -> Result<()> {
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

    /// Resolve a trait method call to possible implementations
    pub fn resolve_trait_call(&self, call: &TraitMethodCall) -> Vector<FunctionId> {
        let mut implementations = Vector::new();

        // If we know the receiver type, look for implementations on that type
        if let Some(receiver_type) = &call.receiver_type {
            if let Some(traits) = self.find_implementations_for_type(receiver_type) {
                for trait_name in traits {
                    if let Some(impls) = self.trait_implementations.get(trait_name) {
                        for impl_info in impls {
                            if impl_info.implementing_type == *receiver_type {
                                for method in &impl_info.method_implementations {
                                    if method.method_name == call.method_name {
                                        implementations.push_back(method.method_id.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // If we don't know the receiver type, find all implementations of this method
            if let Some(impls) = self.trait_implementations.get(&call.trait_name) {
                for impl_info in impls {
                    for method in &impl_info.method_implementations {
                        if method.method_name == call.method_name {
                            implementations.push_back(method.method_id.clone());
                        }
                    }
                }
            }
        }

        implementations
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
}

impl TraitVisitor {
    fn new(file_path: std::path::PathBuf) -> Self {
        Self {
            file_path,
            trait_definitions: Vec::new(),
            trait_implementations: Vec::new(),
            trait_method_calls: Vec::new(),
            current_function: None,
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

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
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

                let method_id = FunctionId {
                    file: self.file_path.clone(),
                    name: format!("{trait_name}::{method_name}"),
                    line,
                };

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
                    let mut method_implementations = Vector::new();

                    for impl_item in &item.items {
                        if let ImplItem::Fn(method) = impl_item {
                            let method_name = method.sig.ident.to_string();
                            let line = self.get_line_number(method.sig.ident.span());

                            let method_id = FunctionId {
                                file: self.file_path.clone(),
                                name: format!("{implementing_type}::{method_name}"),
                                line,
                            };

                            let implementation = TraitMethodImplementation {
                                method_name,
                                method_id,
                                overrides_default: false, // We'd need more analysis to determine this
                            };

                            method_implementations.push_back(implementation);
                        }
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

        self.current_function = Some(FunctionId {
            file: self.file_path.clone(),
            name: func_name,
            line,
        });

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

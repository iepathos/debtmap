//! AST Visitor for God Object Detection
//!
//! This module provides data collection from Rust ASTs for god object analysis.
//! It traverses the syntax tree to gather information about types, methods, and complexity.

use crate::common::{SourceLocation, UnifiedLocationExtractor};
use crate::complexity::cyclomatic::calculate_cyclomatic;
use crate::organization::{FunctionComplexityInfo, PurityLevel};
use std::collections::HashMap;
use syn::{self, visit::Visit};

/// Analysis data for a single type (struct/enum)
pub struct TypeAnalysis {
    pub name: String,
    pub method_count: usize,
    pub field_count: usize,
    pub methods: Vec<String>,
    pub fields: Vec<String>,
    /// Field type names for domain context extraction (Spec 208)
    pub field_types: Vec<String>,
    pub responsibilities: Vec<Responsibility>,
    pub trait_implementations: usize,
    pub location: SourceLocation,
    /// Locations of impl blocks associated with this type (Spec 207)
    pub impl_locations: Vec<SourceLocation>,
}

/// Represents a logical responsibility or concern within a type
pub struct Responsibility {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub methods: Vec<String>,
    #[allow(dead_code)]
    pub fields: Vec<String>,
    #[allow(dead_code)]
    pub cohesion_score: f64,
}

/// Represents weighted contribution of a function to god object score
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FunctionWeight {
    pub name: String,
    pub complexity: u32,
    pub purity_level: PurityLevel,
    pub complexity_weight: f64,
    pub purity_weight: f64,
    pub total_weight: f64,
}

/// Detailed information about a module-level function for multi-signal analysis
#[derive(Debug, Clone)]
pub struct ModuleFunctionInfo {
    pub name: String,
    pub body: String,
    pub return_type: Option<String>,
    pub parameters: Vec<FunctionParameter>,
    pub line_count: usize,
    pub is_public: bool,
    pub is_async: bool,
    pub is_test: bool,
}

/// Function parameter information
#[derive(Debug, Clone)]
pub struct FunctionParameter {
    pub name: String,
    pub type_name: String,
}

/// Visitor for collecting type information from Rust ASTs
pub struct TypeVisitor {
    pub types: HashMap<String, TypeAnalysis>,
    pub standalone_functions: Vec<String>,
    pub function_complexity: Vec<FunctionComplexityInfo>,
    pub function_items: Vec<syn::ItemFn>,
    location_extractor: Option<UnifiedLocationExtractor>,
    /// Tracks visibility of impl methods (Spec 134 Phase 2)
    pub method_visibility: HashMap<String, syn::Visibility>,
    /// Detailed module function information for multi-signal classification (Spec 149)
    pub module_functions: Vec<ModuleFunctionInfo>,
}

impl TypeVisitor {
    /// Create a new TypeVisitor with optional location extraction
    pub fn with_location_extractor(location_extractor: Option<UnifiedLocationExtractor>) -> Self {
        Self {
            types: HashMap::new(),
            standalone_functions: Vec::new(),
            function_complexity: Vec::new(),
            function_items: Vec::new(),
            location_extractor,
            method_visibility: HashMap::new(),
            module_functions: Vec::new(),
        }
    }

    /// Extract complexity from a function
    fn extract_function_complexity(&self, item_fn: &syn::ItemFn) -> FunctionComplexityInfo {
        let name = item_fn.sig.ident.to_string();

        // Check if this is a test function
        let is_test = item_fn.attrs.iter().any(|attr| {
            attr.path().is_ident("test")
                || attr.path().is_ident("cfg")
                    && attr
                        .meta
                        .require_list()
                        .ok()
                        .map(|list| {
                            list.tokens.to_string().contains("test")
                                || list.tokens.to_string().contains("cfg(test)")
                        })
                        .unwrap_or(false)
        });

        // Calculate cyclomatic complexity from the function body
        let cyclomatic_complexity = calculate_cyclomatic(&item_fn.block);

        FunctionComplexityInfo {
            name,
            cyclomatic_complexity,
            cognitive_complexity: cyclomatic_complexity, // Using cyclomatic as proxy for now
            is_test,
        }
    }

    /// Extract type name from a syn::Type
    pub fn extract_type_name(self_ty: &syn::Type) -> Option<String> {
        match self_ty {
            syn::Type::Path(type_path) => type_path.path.get_ident().map(|id| id.to_string()),
            _ => None,
        }
    }

    /// Extract the primary type name from a field type (Spec 208).
    ///
    /// For generic types like `HashMap<String, Module>`, extracts the outer type name.
    /// For simple types like `String`, returns the type name directly.
    fn extract_field_type_name(ty: &syn::Type) -> String {
        match ty {
            syn::Type::Path(type_path) => {
                // Get the last segment of the path (handles std::collections::HashMap, etc.)
                type_path
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            }
            syn::Type::Reference(type_ref) => {
                // For references, recurse on the inner type
                Self::extract_field_type_name(&type_ref.elem)
            }
            syn::Type::Ptr(type_ptr) => {
                // For pointers, recurse on the inner type
                Self::extract_field_type_name(&type_ptr.elem)
            }
            syn::Type::Array(type_array) => {
                // For arrays, recurse on the element type
                Self::extract_field_type_name(&type_array.elem)
            }
            syn::Type::Slice(type_slice) => {
                // For slices, recurse on the element type
                Self::extract_field_type_name(&type_slice.elem)
            }
            syn::Type::Tuple(_) => "Tuple".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    /// Count methods in an impl block and return method names
    pub fn count_impl_methods(items: &[syn::ImplItem]) -> (Vec<String>, usize) {
        let mut methods = Vec::new();
        let mut count = 0;

        for item in items {
            if let syn::ImplItem::Fn(method) = item {
                methods.push(method.sig.ident.to_string());
                count += 1;
            }
        }

        (methods, count)
    }

    /// Extract complexity information from impl methods
    fn extract_impl_complexity(&self, items: &[syn::ImplItem]) -> Vec<FunctionComplexityInfo> {
        items
            .iter()
            .filter_map(|item| {
                if let syn::ImplItem::Fn(method) = item {
                    let name = method.sig.ident.to_string();

                    // Check if this is a test function
                    let is_test = method.attrs.iter().any(|attr| {
                        attr.path().is_ident("test")
                            || attr.path().is_ident("cfg")
                                && attr
                                    .meta
                                    .require_list()
                                    .ok()
                                    .map(|list| {
                                        list.tokens.to_string().contains("test")
                                            || list.tokens.to_string().contains("cfg(test)")
                                    })
                                    .unwrap_or(false)
                    });

                    // Calculate cyclomatic complexity from the function body
                    let cyclomatic_complexity = calculate_cyclomatic(&method.block);

                    Some(FunctionComplexityInfo {
                        name,
                        cyclomatic_complexity,
                        cognitive_complexity: cyclomatic_complexity,
                        is_test,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Update type information with impl block data
    pub fn update_type_info(&mut self, type_name: &str, node: &syn::ItemImpl) {
        if let Some(type_info) = self.types.get_mut(type_name) {
            let (methods, count) = Self::count_impl_methods(&node.items);

            type_info.methods.extend(methods);
            type_info.method_count += count;

            if node.trait_.is_some() {
                type_info.trait_implementations += 1;
            }

            // Spec 207: Track impl block location for accurate LOC calculation
            if let Some(ref extractor) = self.location_extractor {
                let impl_location = extractor.extract_item_location(&syn::Item::Impl(node.clone()));
                type_info.impl_locations.push(impl_location);
            }

            // Spec 134 Phase 2: Track visibility of impl methods
            for item in &node.items {
                if let syn::ImplItem::Fn(method) = item {
                    let method_name = method.sig.ident.to_string();
                    self.method_visibility
                        .insert(method_name, method.vis.clone());
                }
            }
        }
    }

    /// Extract detailed function information for multi-signal analysis (Spec 149)
    fn extract_module_function_info(&self, item_fn: &syn::ItemFn) -> ModuleFunctionInfo {
        let name = item_fn.sig.ident.to_string();

        // Convert function body to string for analysis
        let body = quote::quote!(#item_fn).to_string();

        // Extract return type
        let return_type = Self::extract_return_type(&item_fn.sig.output);

        // Extract parameters
        let parameters = Self::extract_parameters(&item_fn.sig.inputs);

        // Estimate line count
        let line_count = Self::estimate_line_count(&item_fn.block);

        // Check visibility
        let is_public = matches!(item_fn.vis, syn::Visibility::Public(_));

        // Check if async
        let is_async = item_fn.sig.asyncness.is_some();

        // Check if test
        let is_test = item_fn.attrs.iter().any(|attr| {
            attr.path().is_ident("test")
                || attr.path().is_ident("cfg")
                    && attr
                        .meta
                        .require_list()
                        .ok()
                        .map(|list| {
                            list.tokens.to_string().contains("test")
                                || list.tokens.to_string().contains("cfg(test)")
                        })
                        .unwrap_or(false)
        });

        ModuleFunctionInfo {
            name,
            body,
            return_type,
            parameters,
            line_count,
            is_public,
            is_async,
            is_test,
        }
    }

    /// Extract return type from function signature
    fn extract_return_type(output: &syn::ReturnType) -> Option<String> {
        match output {
            syn::ReturnType::Type(_, ty) => Some(quote::quote!(#ty).to_string()),
            syn::ReturnType::Default => None,
        }
    }

    /// Extract parameters from function signature
    fn extract_parameters(
        inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    ) -> Vec<FunctionParameter> {
        inputs
            .iter()
            .filter_map(|arg| {
                if let syn::FnArg::Typed(pat_type) = arg {
                    Some(FunctionParameter {
                        name: Self::extract_param_name(&pat_type.pat),
                        type_name: quote::quote!(#pat_type.ty).to_string(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Extract parameter name from pattern
    fn extract_param_name(pat: &syn::Pat) -> String {
        match pat {
            syn::Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Estimate line count from block
    fn estimate_line_count(block: &syn::Block) -> usize {
        // Simple estimation: count statements + 2 for braces
        block.stmts.len() + 2
    }
}

impl<'ast> Visit<'ast> for TypeVisitor {
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        let type_name = node.ident.to_string();
        let field_count = match &node.fields {
            syn::Fields::Named(fields) => fields.named.len(),
            syn::Fields::Unnamed(fields) => fields.unnamed.len(),
            syn::Fields::Unit => 0,
        };

        let fields = match &node.fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref().map(|id| id.to_string()))
                .collect(),
            _ => Vec::new(),
        };

        // Spec 208: Extract field type names for domain context
        let field_types = match &node.fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .map(|f| Self::extract_field_type_name(&f.ty))
                .collect(),
            syn::Fields::Unnamed(fields) => fields
                .unnamed
                .iter()
                .map(|f| Self::extract_field_type_name(&f.ty))
                .collect(),
            syn::Fields::Unit => Vec::new(),
        };

        let location = if let Some(ref extractor) = self.location_extractor {
            extractor.extract_item_location(&syn::Item::Struct(node.clone()))
        } else {
            SourceLocation::default()
        };

        self.types.insert(
            type_name.clone(),
            TypeAnalysis {
                name: type_name,
                method_count: 0,
                field_count,
                methods: Vec::new(),
                fields,
                field_types,
                responsibilities: Vec::new(),
                trait_implementations: 0,
                location,
                impl_locations: Vec::new(),
            },
        );
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if let Some(type_name) = Self::extract_type_name(&node.self_ty) {
            self.update_type_info(&type_name, node);

            // Extract complexity information for impl methods
            let complexity_info = self.extract_impl_complexity(&node.items);
            self.function_complexity.extend(complexity_info);
        }
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        // Track standalone functions
        self.standalone_functions.push(node.sig.ident.to_string());

        // Extract complexity information
        let complexity_info = self.extract_function_complexity(node);
        self.function_complexity.push(complexity_info);

        // Store the function item for purity analysis
        self.function_items.push(node.clone());

        // Extract detailed module function information for multi-signal analysis (Spec 149)
        let module_func_info = self.extract_module_function_info(node);
        self.module_functions.push(module_func_info);
    }
}

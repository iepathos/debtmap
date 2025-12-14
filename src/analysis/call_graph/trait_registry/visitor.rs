//! AST Visitor for extracting trait information
//!
//! This module handles the I/O boundary of parsing Rust AST to extract
//! trait definitions, implementations, and method calls.

use super::types::{
    TraitImplementation, TraitMethod, TraitMethodCall, TraitMethodImplementation,
};
use crate::priority::call_graph::FunctionId;
use im::{HashMap, HashSet, Vector};
use std::path::PathBuf;
use syn::visit::Visit;
use syn::{File, ImplItem, ItemImpl, ItemTrait, Path as SynPath, TraitItem, Type, TypePath};

/// Result of visiting a file for trait information
#[derive(Debug, Clone, Default)]
pub struct TraitVisitorResult {
    pub trait_definitions: Vec<(String, Vector<TraitMethod>)>,
    pub trait_implementations: Vec<TraitImplementation>,
    pub trait_method_calls: Vec<TraitMethodCall>,
    pub visit_trait_methods: HashSet<FunctionId>,
    pub visit_implementations: HashMap<String, Vector<TraitMethodImplementation>>,
}

/// Visitor for extracting trait information from AST
pub struct TraitVisitor {
    file_path: PathBuf,
    current_function: Option<FunctionId>,
    result: TraitVisitorResult,
}

impl TraitVisitor {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            current_function: None,
            result: TraitVisitorResult::default(),
        }
    }

    /// Visit a file and return the extracted trait information
    pub fn visit_and_extract(mut self, file: &File) -> TraitVisitorResult {
        self.visit_file(file);
        self.result
    }

    /// Extract the type name from a syn Type
    fn extract_type_name(&self, ty: &Type) -> Option<String> {
        match ty {
            Type::Path(TypePath { path, .. }) => self.extract_path_name(path),
            _ => None,
        }
    }

    /// Extract the name from a syn Path
    fn extract_path_name(&self, path: &SynPath) -> Option<String> {
        if path.segments.len() == 1 {
            Some(path.segments.first()?.ident.to_string())
        } else {
            let segments: Vec<String> = path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect();
            Some(segments.join("::"))
        }
    }

    /// Get line number from a span
    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    /// Check if a trait name represents a visitor pattern trait
    fn is_visit_trait(&self, trait_name: &str) -> bool {
        is_visitor_pattern_trait(trait_name)
    }

    /// Extract method implementations from impl items
    fn extract_method_implementations(
        &mut self,
        items: &[ImplItem],
        implementing_type: &str,
        is_visit_trait: bool,
    ) -> Vector<TraitMethodImplementation> {
        items
            .iter()
            .filter_map(|item| self.extract_single_method(item, implementing_type, is_visit_trait))
            .collect()
    }

    /// Extract a single method implementation
    fn extract_single_method(
        &mut self,
        item: &ImplItem,
        implementing_type: &str,
        is_visit_trait: bool,
    ) -> Option<TraitMethodImplementation> {
        let ImplItem::Fn(method) = item else {
            return None;
        };

        let method_name = method.sig.ident.to_string();
        let line = self.get_line_number(method.sig.ident.span());

        let method_id = FunctionId::new(
            self.file_path.clone(),
            format!("{implementing_type}::{method_name}"),
            line,
        );

        if is_visit_trait {
            self.result.visit_trait_methods.insert(method_id.clone());
        }

        Some(TraitMethodImplementation {
            method_name,
            method_id,
            overrides_default: false,
        })
    }

    /// Process a trait definition
    fn process_trait_definition(&mut self, item: &ItemTrait) {
        let trait_name = item.ident.to_string();
        let methods = self.extract_trait_methods(item, &trait_name);
        self.result.trait_definitions.push((trait_name, methods));
    }

    /// Extract methods from a trait definition
    fn extract_trait_methods(&self, item: &ItemTrait, trait_name: &str) -> Vector<TraitMethod> {
        item.items
            .iter()
            .filter_map(|trait_item| self.extract_trait_method(trait_item, trait_name))
            .collect()
    }

    /// Extract a single trait method
    fn extract_trait_method(
        &self,
        trait_item: &TraitItem,
        trait_name: &str,
    ) -> Option<TraitMethod> {
        let TraitItem::Fn(method) = trait_item else {
            return None;
        };

        let method_name = method.sig.ident.to_string();
        let line = self.get_line_number(method.sig.ident.span());

        let method_id = FunctionId::new(
            self.file_path.clone(),
            format!("{trait_name}::{method_name}"),
            line,
        );

        Some(TraitMethod {
            trait_name: trait_name.to_string(),
            method_name,
            method_id,
            has_default: method.default.is_some(),
        })
    }

    /// Process a trait implementation
    fn process_trait_impl(&mut self, item: &ItemImpl) {
        let Some((_, trait_path, _)) = &item.trait_ else {
            return;
        };

        let Some(trait_name) = self.extract_path_name(trait_path) else {
            return;
        };

        let Some(implementing_type) = self.extract_type_name(&item.self_ty) else {
            return;
        };

        let is_visit_trait = self.is_visit_trait(&trait_name);
        let method_implementations =
            self.extract_method_implementations(&item.items, &implementing_type, is_visit_trait);

        if is_visit_trait {
            self.result
                .visit_implementations
                .entry(implementing_type.clone())
                .or_default()
                .extend(method_implementations.clone());
        }

        let trait_impl = TraitImplementation {
            trait_name,
            implementing_type,
            method_implementations,
            impl_id: None,
        };

        self.result.trait_implementations.push(trait_impl);
    }

    /// Record a potential trait method call
    fn record_method_call(&mut self, method_name: String, line: usize) {
        let Some(caller) = &self.current_function else {
            return;
        };

        let trait_call = TraitMethodCall {
            caller: caller.clone(),
            trait_name: "Unknown".to_string(),
            method_name,
            receiver_type: None,
            line,
        };

        self.result.trait_method_calls.push(trait_call);
    }
}

impl<'ast> Visit<'ast> for TraitVisitor {
    fn visit_item_trait(&mut self, item: &'ast ItemTrait) {
        self.process_trait_definition(item);
        syn::visit::visit_item_trait(self, item);
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        self.process_trait_impl(item);
        syn::visit::visit_item_impl(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let func_name = item.sig.ident.to_string();
        let line = self.get_line_number(item.sig.ident.span());

        self.current_function = Some(FunctionId::new(self.file_path.clone(), func_name, line));
        syn::visit::visit_item_fn(self, item);
        self.current_function = None;
    }

    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        let method_name = expr.method.to_string();
        let line = self.get_line_number(expr.method.span());
        self.record_method_call(method_name, line);
        syn::visit::visit_expr_method_call(self, expr);
    }
}

// Pure functions for trait classification

/// Pure function to classify if a trait name represents a visitor pattern trait
pub fn is_visitor_pattern_trait(trait_name: &str) -> bool {
    is_generic_visitor_trait(trait_name) || is_qualified_visitor_trait(trait_name)
}

/// Checks if the trait name is a generic visitor pattern (Visit, Visitor, or generic variants)
pub fn is_generic_visitor_trait(trait_name: &str) -> bool {
    matches!(trait_name, "Visit" | "Visitor")
        || trait_name.starts_with("Visit<")
        || trait_name.starts_with("Visitor<")
}

/// Checks if the trait name is a fully qualified visitor trait from known libraries
pub fn is_qualified_visitor_trait(trait_name: &str) -> bool {
    matches!(trait_name, "syn::visit::Visit" | "quote::visit::Visit")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_generic_visitor_trait_simple_names() {
        assert!(is_generic_visitor_trait("Visit"));
        assert!(is_generic_visitor_trait("Visitor"));
    }

    #[test]
    fn test_is_generic_visitor_trait_with_generics() {
        assert!(is_generic_visitor_trait("Visit<T>"));
        assert!(is_generic_visitor_trait("Visitor<'a>"));
        assert!(is_generic_visitor_trait("Visit<T, U>"));
        assert!(is_generic_visitor_trait("Visitor<'a, T>"));
    }

    #[test]
    fn test_is_generic_visitor_trait_negative_cases() {
        assert!(!is_generic_visitor_trait(""));
        assert!(!is_generic_visitor_trait("MyTrait"));
        assert!(!is_generic_visitor_trait("VisitData"));
        assert!(!is_generic_visitor_trait("VisitorPattern"));
        assert!(!is_generic_visitor_trait("syn::visit::Visit"));
    }

    #[test]
    fn test_is_qualified_visitor_trait_known_libraries() {
        assert!(is_qualified_visitor_trait("syn::visit::Visit"));
        assert!(is_qualified_visitor_trait("quote::visit::Visit"));
    }

    #[test]
    fn test_is_qualified_visitor_trait_negative_cases() {
        assert!(!is_qualified_visitor_trait("Visit"));
        assert!(!is_qualified_visitor_trait("other::visit::Visit"));
        assert!(!is_qualified_visitor_trait("custom::Visitor"));
        assert!(!is_qualified_visitor_trait(""));
    }

    #[test]
    fn test_is_visitor_pattern_trait_comprehensive() {
        // Generic visitor traits
        assert!(is_visitor_pattern_trait("Visit"));
        assert!(is_visitor_pattern_trait("Visitor"));
        assert!(is_visitor_pattern_trait("Visit<T>"));
        assert!(is_visitor_pattern_trait("Visitor<'a>"));

        // Qualified visitor traits
        assert!(is_visitor_pattern_trait("syn::visit::Visit"));
        assert!(is_visitor_pattern_trait("quote::visit::Visit"));

        // Non-visitor traits
        assert!(!is_visitor_pattern_trait("Debug"));
        assert!(!is_visitor_pattern_trait("Clone"));
        assert!(!is_visitor_pattern_trait("VisitData"));
        assert!(!is_visitor_pattern_trait("MyVisitor"));
        assert!(!is_visitor_pattern_trait(""));
    }
}

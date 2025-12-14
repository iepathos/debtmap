//! Call graph construction for method relationship analysis.
//!
//! This module builds adjacency matrices representing method call
//! relationships within impl blocks and standalone functions.
//!
//! # Pure Function Properties
//!
//! All functions in this module are pure:
//! - Deterministic output for same input
//! - No side effects (no I/O, no logging)
//! - Thread-safe

use std::collections::{HashMap, HashSet};
use syn::visit::Visit;

/// Build method call adjacency matrix from impl blocks.
///
/// Analyzes method bodies to find `self.method()` calls and builds
/// a matrix of call relationships.
///
/// # Arguments
///
/// * `impl_blocks` - References to syn ItemImpl blocks to analyze
///
/// # Returns
///
/// HashMap where key is (caller, callee) and value is call count
pub fn build_method_call_adjacency_matrix(
    impl_blocks: &[&syn::ItemImpl],
) -> HashMap<(String, String), usize> {
    build_method_call_adjacency_matrix_with_functions(impl_blocks, &[])
}

/// Build method call adjacency matrix with support for standalone functions.
///
/// This enhanced version also tracks calls between standalone functions in the same file,
/// providing better clustering for modules with utility functions.
///
/// # Arguments
///
/// * `impl_blocks` - References to syn ItemImpl blocks to analyze
/// * `standalone_functions` - References to standalone function definitions
///
/// # Returns
///
/// HashMap where key is (caller, callee) and value is call count
pub fn build_method_call_adjacency_matrix_with_functions(
    impl_blocks: &[&syn::ItemImpl],
    standalone_functions: &[&syn::ItemFn],
) -> HashMap<(String, String), usize> {
    let all_function_names = collect_all_function_names(impl_blocks, standalone_functions);
    let mut matrix = HashMap::new();

    process_impl_methods(impl_blocks, &all_function_names, &mut matrix);
    process_standalone_functions(standalone_functions, &all_function_names, &mut matrix);

    matrix
}

/// Collect all function names from impl blocks and standalone functions.
fn collect_all_function_names(
    impl_blocks: &[&syn::ItemImpl],
    standalone_functions: &[&syn::ItemFn],
) -> HashSet<String> {
    let impl_names = impl_blocks
        .iter()
        .flat_map(|b| b.items.iter())
        .filter_map(extract_method_name);

    let standalone_names = standalone_functions.iter().map(|f| f.sig.ident.to_string());

    impl_names.chain(standalone_names).collect()
}

/// Extract method name from an impl item if it's a function.
fn extract_method_name(item: &syn::ImplItem) -> Option<String> {
    match item {
        syn::ImplItem::Fn(method) => Some(method.sig.ident.to_string()),
        _ => None,
    }
}

/// Process impl block methods to find call relationships.
fn process_impl_methods(
    impl_blocks: &[&syn::ItemImpl],
    all_function_names: &HashSet<String>,
    matrix: &mut HashMap<(String, String), usize>,
) {
    for impl_block in impl_blocks {
        for item in &impl_block.items {
            if let syn::ImplItem::Fn(method) = item {
                process_single_method(method, all_function_names, matrix);
            }
        }
    }
}

/// Process a single method to extract its call relationships.
fn process_single_method(
    method: &syn::ImplItemFn,
    all_function_names: &HashSet<String>,
    matrix: &mut HashMap<(String, String), usize>,
) {
    let method_name = method.sig.ident.to_string();

    let mut call_visitor = MethodCallVisitor {
        current_method: method_name.clone(),
        calls: Vec::new(),
        all_function_names,
    };
    call_visitor.visit_impl_item_fn(method);

    for called_method in call_visitor.calls {
        let key = (method_name.clone(), called_method);
        *matrix.entry(key).or_insert(0) += 1;
    }
}

/// Process standalone functions to find call relationships.
fn process_standalone_functions(
    standalone_functions: &[&syn::ItemFn],
    all_function_names: &HashSet<String>,
    matrix: &mut HashMap<(String, String), usize>,
) {
    for func in standalone_functions {
        let func_name = func.sig.ident.to_string();

        let mut call_visitor = MethodCallVisitor {
            current_method: func_name.clone(),
            calls: Vec::new(),
            all_function_names,
        };
        call_visitor.visit_item_fn(func);

        for called_function in call_visitor.calls {
            let key = (func_name.clone(), called_function);
            *matrix.entry(key).or_insert(0) += 1;
        }
    }
}

/// Visitor to extract method calls from a method body.
struct MethodCallVisitor<'a> {
    current_method: String,
    calls: Vec<String>,
    all_function_names: &'a HashSet<String>,
}

impl<'ast, 'a> Visit<'ast> for MethodCallVisitor<'a> {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if is_self_method_call(node) {
            let method_name = node.method.to_string();
            if method_name != self.current_method {
                self.calls.push(method_name);
            }
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Some(called_name) = extract_called_function_name(node, self.all_function_names) {
            if called_name != self.current_method {
                self.calls.push(called_name);
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

/// Check if an expression is a self.method() call.
fn is_self_method_call(node: &syn::ExprMethodCall) -> bool {
    if let syn::Expr::Path(ref path) = *node.receiver {
        return path
            .path
            .segments
            .first()
            .map(|seg| seg.ident == "self")
            .unwrap_or(false);
    }
    false
}

/// Extract the function name from a call expression if applicable.
fn extract_called_function_name(
    node: &syn::ExprCall,
    all_function_names: &HashSet<String>,
) -> Option<String> {
    let syn::Expr::Path(ref path) = *node.func else {
        return None;
    };

    // Check for self::method() or Self::method() calls
    if path.path.segments.len() >= 2 {
        let first = &path.path.segments[0].ident;
        if first == "self" || first == "Self" {
            return Some(path.path.segments[1].ident.to_string());
        }
    }

    // Check for standalone function calls
    if path.path.segments.len() == 1 {
        let func_name = path.path.segments[0].ident.to_string();
        if all_function_names.contains(&func_name) {
            return Some(func_name);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_function_names_empty() {
        let names = collect_all_function_names(&[], &[]);
        assert!(names.is_empty());
    }

    #[test]
    fn test_is_self_method_call_detection() {
        let code: syn::ExprMethodCall = syn::parse_quote!(self.other_method());
        assert!(is_self_method_call(&code));
    }

    #[test]
    fn test_is_not_self_method_call() {
        let code: syn::ExprMethodCall = syn::parse_quote!(other.method());
        assert!(!is_self_method_call(&code));
    }
}

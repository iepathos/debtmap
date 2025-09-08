/// Graph building and management functionality for call graph extraction
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::PathBuf;
use syn::{ImplItemFn, ItemFn};

/// Expression categorization for special handling
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExprCategory {
    Closure,
    Async,
    Await,
    Try,
    Unsafe,
    Regular,
}

/// Builds and manages the call graph
pub struct GraphBuilder {
    pub call_graph: CallGraph,
    current_file: PathBuf,
    module_path: Vec<String>,
}

impl GraphBuilder {
    pub fn new(file: PathBuf) -> Self {
        Self {
            call_graph: CallGraph::new(),
            current_file: file,
            module_path: Vec::new(),
        }
    }

    /// Set the current module path
    pub fn set_module_path(&mut self, path: Vec<String>) {
        self.module_path = path;
    }

    /// Get the current module path
    pub fn module_path(&self) -> &[String] {
        &self.module_path
    }

    /// Push a module to the path
    pub fn push_module(&mut self, module: String) {
        self.module_path.push(module);
    }

    /// Pop a module from the path
    pub fn pop_module(&mut self) {
        self.module_path.pop();
    }

    /// Add a function to the graph
    pub fn add_function(
        &mut self,
        name: String,
        line: usize,
        is_test: bool,
        is_async: bool,
    ) -> FunctionId {
        let function_id = FunctionId {
            name: name.clone(),
            file: self.current_file.clone(),
            line,
        };

        // Add function with appropriate parameters
        // Using defaults for entry_point and complexity for now
        self.call_graph.add_function(
            function_id.clone(),
            false, // is_entry_point
            is_test,
            0, // complexity (to be calculated)
            0, // lines (to be calculated)
        );
        function_id
    }

    /// Add a function from an ItemFn
    pub fn add_function_from_item(
        &mut self,
        name: String,
        line: usize,
        item_fn: &ItemFn,
    ) -> FunctionId {
        let is_test = item_fn.attrs.iter().any(|attr| {
            attr.path().is_ident("test")
                || attr.path().is_ident("tokio::test")
                || attr.path().is_ident("actix_rt::test")
        });

        let is_async = item_fn.sig.asyncness.is_some();

        self.add_function(name, line, is_test, is_async)
    }

    /// Add an impl method to the graph
    pub fn add_impl_method(
        &mut self,
        name: String,
        line: usize,
        impl_fn: &ImplItemFn,
    ) -> FunctionId {
        let is_test = impl_fn.attrs.iter().any(|attr| {
            attr.path().is_ident("test")
                || attr.path().is_ident("tokio::test")
                || attr.path().is_ident("actix_rt::test")
        });

        let is_async = impl_fn.sig.asyncness.is_some();

        self.add_function(name, line, is_test, is_async)
    }

    /// Add a call edge to the graph
    pub fn add_call(&mut self, caller: FunctionId, callee: FunctionId, call_type: CallType) {
        self.call_graph.add_call(FunctionCall {
            caller,
            callee,
            call_type,
        });
    }

    /// Get all functions in the graph
    pub fn all_functions(&self) -> impl Iterator<Item = &FunctionId> {
        self.call_graph.get_all_functions()
    }

    /// Get the number of functions in the graph
    pub fn function_count(&self) -> usize {
        self.call_graph.node_count()
    }

    /// Merge another call graph into this one
    pub fn merge(&mut self, other: CallGraph) {
        self.call_graph.merge(other);
    }

    /// Extract a function name from a syn path
    pub fn extract_function_name_from_path(path: &syn::Path) -> Option<String> {
        // Get the full path as a string
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect();

        if segments.is_empty() {
            return None;
        }

        // Join with :: to get the full qualified name
        Some(segments.join("::"))
    }

    /// Get line number from a span (placeholder for actual implementation)
    pub fn get_line_number(&self, _span: proc_macro2::Span) -> usize {
        // In a real implementation, this would use span information
        // to get the actual line number
        0
    }

    /// Classify an expression for special handling
    pub fn classify_expr_category(expr: &syn::Expr) -> ExprCategory {
        match expr {
            syn::Expr::Closure(_) => ExprCategory::Closure,
            syn::Expr::Async(_) => ExprCategory::Async,
            syn::Expr::Await(_) => ExprCategory::Await,
            syn::Expr::Try(_) => ExprCategory::Try,
            syn::Expr::Unsafe(_) => ExprCategory::Unsafe,
            _ => ExprCategory::Regular,
        }
    }

    /// Check if an expression category needs special handling
    pub fn needs_special_handling(category: ExprCategory) -> bool {
        !matches!(category, ExprCategory::Regular)
    }

    /// Build a qualified function name with module path
    pub fn build_qualified_name(&self, base_name: &str) -> String {
        if self.module_path.is_empty() {
            base_name.to_string()
        } else {
            format!("{}::{}", self.module_path.join("::"), base_name)
        }
    }

    /// Build a qualified name for an impl method
    pub fn build_impl_method_name(&self, impl_type: &str, method_name: &str) -> String {
        if self.module_path.is_empty() {
            format!("{}::{}", impl_type, method_name)
        } else {
            format!(
                "{}::{}::{}",
                self.module_path.join("::"),
                impl_type,
                method_name
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_path_operations() {
        let mut builder = GraphBuilder::new(PathBuf::from("test.rs"));

        assert!(builder.module_path().is_empty());

        builder.push_module("mod1".to_string());
        assert_eq!(builder.module_path(), &["mod1"]);

        builder.push_module("mod2".to_string());
        assert_eq!(builder.module_path(), &["mod1", "mod2"]);

        builder.pop_module();
        assert_eq!(builder.module_path(), &["mod1"]);
    }

    #[test]
    fn test_build_qualified_name() {
        let mut builder = GraphBuilder::new(PathBuf::from("test.rs"));

        assert_eq!(builder.build_qualified_name("func"), "func");

        builder.push_module("module".to_string());
        assert_eq!(builder.build_qualified_name("func"), "module::func");

        builder.push_module("submodule".to_string());
        assert_eq!(
            builder.build_qualified_name("func"),
            "module::submodule::func"
        );
    }

    #[test]
    fn test_build_impl_method_name() {
        let mut builder = GraphBuilder::new(PathBuf::from("test.rs"));

        assert_eq!(
            builder.build_impl_method_name("MyStruct", "method"),
            "MyStruct::method"
        );

        builder.push_module("module".to_string());
        assert_eq!(
            builder.build_impl_method_name("MyStruct", "method"),
            "module::MyStruct::method"
        );
    }

    #[test]
    fn test_classify_expr_category() {
        use syn::parse_quote;

        let closure: syn::Expr = parse_quote! { |x| x + 1 };
        assert_eq!(
            GraphBuilder::classify_expr_category(&closure),
            ExprCategory::Closure
        );

        let async_block: syn::Expr = parse_quote! { async { foo().await } };
        assert_eq!(
            GraphBuilder::classify_expr_category(&async_block),
            ExprCategory::Async
        );

        let regular: syn::Expr = parse_quote! { foo() };
        assert_eq!(
            GraphBuilder::classify_expr_category(&regular),
            ExprCategory::Regular
        );
    }

    #[test]
    fn test_needs_special_handling() {
        assert!(GraphBuilder::needs_special_handling(ExprCategory::Closure));
        assert!(GraphBuilder::needs_special_handling(ExprCategory::Async));
        assert!(GraphBuilder::needs_special_handling(ExprCategory::Await));
        assert!(!GraphBuilder::needs_special_handling(ExprCategory::Regular));
    }
}

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
        _is_async: bool,
        module_path: String,
    ) -> FunctionId {
        let function_id = FunctionId::with_module_path(
            self.current_file.clone(),
            name.clone(),
            line,
            module_path,
        );

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
        module_path: String,
    ) -> FunctionId {
        let is_test = Self::has_test_attribute(&item_fn.attrs);
        let is_async = item_fn.sig.asyncness.is_some();

        self.add_function(name, line, is_test, is_async, module_path)
    }

    /// Add an impl method to the graph
    pub fn add_impl_method(
        &mut self,
        name: String,
        line: usize,
        impl_fn: &ImplItemFn,
        module_path: String,
    ) -> FunctionId {
        let is_test = Self::has_test_attribute(&impl_fn.attrs);
        let is_async = impl_fn.sig.asyncness.is_some();

        self.add_function(name, line, is_test, is_async, module_path)
    }

    /// Check if a function has a test attribute.
    ///
    /// Detects:
    /// - `#[test]` - standard Rust test
    /// - `#[tokio::test]` - async tokio test
    /// - `#[actix_rt::test]` - actix runtime test
    /// - `#[rstest]` - rstest parameterized test
    /// - `#[test_case]` - test_case attribute
    fn has_test_attribute(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| {
            let path = attr.path();

            // Check single ident: #[test], #[rstest], #[test_case]
            if path.is_ident("test") || path.is_ident("rstest") || path.is_ident("test_case") {
                return true;
            }

            // Check path-based attributes: #[tokio::test], #[actix_rt::test]
            let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();

            if segments.len() == 2 {
                let first = segments[0].as_str();
                let second = segments[1].as_str();
                return (first == "tokio" && second == "test")
                    || (first == "actix_rt" && second == "test");
            }

            false
        })
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
    pub fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
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

    // Unit tests for test function detection (Spec 267)

    #[test]
    fn test_detect_basic_test_attribute() {
        use syn::parse_quote;

        let test_fn: ItemFn = parse_quote! {
            #[test]
            fn my_test() {
                assert!(true);
            }
        };

        assert!(
            GraphBuilder::has_test_attribute(&test_fn.attrs),
            "#[test] attribute should be detected"
        );
    }

    #[test]
    fn test_detect_production_function() {
        use syn::parse_quote;

        let prod_fn: ItemFn = parse_quote! {
            fn production_function() {
                println!("hello");
            }
        };

        assert!(
            !GraphBuilder::has_test_attribute(&prod_fn.attrs),
            "Function without #[test] should NOT be detected as test"
        );
    }

    #[test]
    fn test_detect_tokio_test_attribute() {
        use syn::parse_quote;

        let tokio_test_fn: ItemFn = parse_quote! {
            #[tokio::test]
            async fn async_test() {
                assert!(true);
            }
        };

        assert!(
            GraphBuilder::has_test_attribute(&tokio_test_fn.attrs),
            "#[tokio::test] attribute should be detected"
        );
    }

    #[test]
    fn test_detect_actix_test_attribute() {
        use syn::parse_quote;

        let actix_test_fn: ItemFn = parse_quote! {
            #[actix_rt::test]
            async fn actix_test() {
                assert!(true);
            }
        };

        assert!(
            GraphBuilder::has_test_attribute(&actix_test_fn.attrs),
            "#[actix_rt::test] attribute should be detected"
        );
    }

    #[test]
    fn test_detect_rstest_attribute() {
        use syn::parse_quote;

        let rstest_fn: ItemFn = parse_quote! {
            #[rstest]
            fn parameterized_test() {
                assert!(true);
            }
        };

        assert!(
            GraphBuilder::has_test_attribute(&rstest_fn.attrs),
            "#[rstest] attribute should be detected"
        );
    }

    #[test]
    fn test_detect_test_case_attribute() {
        use syn::parse_quote;

        let test_case_fn: ItemFn = parse_quote! {
            #[test_case]
            fn test_case_test() {
                assert!(true);
            }
        };

        assert!(
            GraphBuilder::has_test_attribute(&test_case_fn.attrs),
            "#[test_case] attribute should be detected"
        );
    }

    #[test]
    fn test_helper_function_without_test_attr_not_detected() {
        use syn::parse_quote;

        // Helper functions in test modules don't have #[test]
        let helper_fn: ItemFn = parse_quote! {
            fn create_test_fixture() -> i32 {
                42
            }
        };

        assert!(
            !GraphBuilder::has_test_attribute(&helper_fn.attrs),
            "Helper functions without #[test] should NOT be detected as tests"
        );
    }

    #[test]
    fn test_function_with_other_attributes_not_detected() {
        use syn::parse_quote;

        let other_fn: ItemFn = parse_quote! {
            #[inline]
            #[must_use]
            fn some_function() -> i32 {
                42
            }
        };

        assert!(
            !GraphBuilder::has_test_attribute(&other_fn.attrs),
            "Function with non-test attributes should NOT be detected as test"
        );
    }
}

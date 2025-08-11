use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, ImplItemFn, ItemFn};

pub struct CallGraphExtractor {
    pub call_graph: CallGraph,
    current_function: Option<FunctionId>,
    current_file: PathBuf,
}

impl CallGraphExtractor {
    pub fn new(file: PathBuf) -> Self {
        Self {
            call_graph: CallGraph::new(),
            current_function: None,
            current_file: file,
        }
    }

    /// Classify a function/method name into its call type
    /// This is a pure function that determines the type based on naming patterns
    fn classify_call_type(name: &str) -> CallType {
        match () {
            _ if name == "await" => CallType::Async,
            _ if name.contains("async") || name.contains("await") => CallType::Async,
            _ if name.starts_with("handle_") || name.starts_with("process_") => CallType::Delegate,
            _ if name.starts_with("map") || name.starts_with("and_then") => CallType::Pipeline,
            _ => CallType::Direct,
        }
    }

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    fn add_function_to_graph(&mut self, name: String, line: usize, item_fn: &ItemFn) {
        let func_id = FunctionId {
            file: self.current_file.clone(),
            name: name.clone(),
            line,
        };

        // Check if this is a test function
        let is_test = item_fn.attrs.iter().any(|attr| {
            attr.path()
                .segments
                .iter()
                .any(|s| s.ident == "test" || s.ident == "tokio_test")
        });

        // Check if this is likely an entry point
        let is_entry_point = name == "main"
            || name.starts_with("handle_")
            || name.starts_with("process_")
            || name.starts_with("run_")
            || name.starts_with("execute_");

        // Calculate basic complexity for the call graph
        let complexity = calculate_basic_complexity(&item_fn.block);
        let lines = count_lines(&item_fn.block);

        self.call_graph
            .add_function(func_id.clone(), is_entry_point, is_test, complexity, lines);

        // Set as current function for call extraction
        self.current_function = Some(func_id);
    }

    fn extract_function_name_from_path(path: &syn::Path) -> Option<String> {
        // Handle simple function calls like `foo()` or `module::foo()`
        if let Some(last_segment) = path.segments.last() {
            return Some(last_segment.ident.to_string());
        }
        None
    }

    fn add_call(&mut self, callee_name: String, call_type: CallType) {
        if let Some(ref caller) = self.current_function {
            // Create a function ID for the callee
            // Note: We don't know the exact file/line of the callee yet,
            // this will be resolved later when we match with actual function definitions
            let callee = FunctionId {
                file: self.current_file.clone(),
                name: callee_name,
                line: 0, // Will be resolved later
            };

            self.call_graph.add_call(FunctionCall {
                caller: caller.clone(),
                callee,
                call_type,
            });
        }
    }
}

impl<'ast> Visit<'ast> for CallGraphExtractor {
    fn visit_item_fn(&mut self, item_fn: &'ast ItemFn) {
        let name = item_fn.sig.ident.to_string();
        let line = self.get_line_number(item_fn.sig.ident.span());

        // Add function to graph
        self.add_function_to_graph(name, line, item_fn);

        // Visit the function body to extract calls
        syn::visit::visit_item_fn(self, item_fn);

        // Clear current function after visiting
        self.current_function = None;
    }

    fn visit_impl_item_fn(&mut self, impl_fn: &'ast ImplItemFn) {
        let name = impl_fn.sig.ident.to_string();
        let line = self.get_line_number(impl_fn.sig.ident.span());

        // Convert to ItemFn for consistency
        let item_fn = ItemFn {
            attrs: impl_fn.attrs.clone(),
            vis: syn::Visibility::Inherited,
            sig: impl_fn.sig.clone(),
            block: Box::new(impl_fn.block.clone()),
        };

        // Add function to graph
        self.add_function_to_graph(name, line, &item_fn);

        // Visit the function body to extract calls
        syn::visit::visit_impl_item_fn(self, impl_fn);

        // Clear current function after visiting
        self.current_function = None;
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Handle regular function calls: foo(), module::foo()
            Expr::Call(ExprCall { func, .. }) => {
                if let Expr::Path(expr_path) = &**func {
                    if let Some(name) = Self::extract_function_name_from_path(&expr_path.path) {
                        self.add_call(name.clone(), Self::classify_call_type(&name));
                    }
                }
            }
            // Handle method calls: obj.method()
            Expr::MethodCall(ExprMethodCall { method, .. }) => {
                let name = method.to_string();
                self.add_call(name.clone(), Self::classify_call_type(&name));
            }
            // Handle closures that might be callbacks
            Expr::Closure(closure) if self.current_function.is_some() => {
                self.add_call(
                    format!("<closure@{}>", closure.body.span().start().line),
                    CallType::Callback,
                );
            }
            _ => {}
        }

        // Continue visiting nested expressions
        syn::visit::visit_expr(self, expr);
    }
}

fn calculate_basic_complexity(block: &syn::Block) -> u32 {
    struct ComplexityVisitor {
        complexity: u32,
    }

    impl<'ast> Visit<'ast> for ComplexityVisitor {
        fn visit_expr(&mut self, expr: &'ast Expr) {
            match expr {
                Expr::If(_) | Expr::Match(_) | Expr::While(_) | Expr::ForLoop(_) => {
                    self.complexity += 1;
                }
                _ => {}
            }
            syn::visit::visit_expr(self, expr);
        }
    }

    let mut visitor = ComplexityVisitor { complexity: 1 };
    visitor.visit_block(block);
    visitor.complexity
}

fn count_lines(block: &syn::Block) -> usize {
    // Simple approximation based on statement count
    block.stmts.len().max(1)
}

/// Extract call graph from a parsed Rust file
pub fn extract_call_graph(file: &syn::File, path: &Path) -> CallGraph {
    let mut extractor = CallGraphExtractor::new(path.to_path_buf());
    extractor.visit_file(file);
    extractor.call_graph
}

/// Merge a file's call graph into the main call graph
pub fn merge_call_graphs(_main: &mut CallGraph, _file_graph: CallGraph) {
    // This would be implemented in the CallGraph struct
    // For now, we'll need to add a merge method to CallGraph
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_classify_call_type_await() {
        // Special case: "await" is always Async
        assert_eq!(
            CallGraphExtractor::classify_call_type("await"),
            CallType::Async
        );
    }

    #[test]
    fn test_classify_call_type_async_patterns() {
        assert_eq!(
            CallGraphExtractor::classify_call_type("async_fetch"),
            CallType::Async
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("fetch_async"),
            CallType::Async
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("await_result"),
            CallType::Async
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("do_await"),
            CallType::Async
        );
    }

    #[test]
    fn test_classify_call_type_delegate_patterns() {
        assert_eq!(
            CallGraphExtractor::classify_call_type("handle_request"),
            CallType::Delegate
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("handle_error"),
            CallType::Delegate
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("process_data"),
            CallType::Delegate
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("process_"),
            CallType::Delegate
        );
    }

    #[test]
    fn test_classify_call_type_pipeline_patterns() {
        assert_eq!(
            CallGraphExtractor::classify_call_type("map"),
            CallType::Pipeline
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("map_err"),
            CallType::Pipeline
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("and_then"),
            CallType::Pipeline
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("and_then_some"),
            CallType::Pipeline
        );
    }

    #[test]
    fn test_classify_call_type_direct_default() {
        // Everything else defaults to Direct
        assert_eq!(
            CallGraphExtractor::classify_call_type("foo"),
            CallType::Direct
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("calculate"),
            CallType::Direct
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("get_value"),
            CallType::Direct
        );
        assert_eq!(CallGraphExtractor::classify_call_type(""), CallType::Direct);
    }

    #[test]
    fn test_classify_call_type_priority_order() {
        // Test that "await" takes precedence even if it matches other patterns
        assert_eq!(
            CallGraphExtractor::classify_call_type("await"),
            CallType::Async
        );

        // Test that async patterns take precedence over others
        assert_eq!(
            CallGraphExtractor::classify_call_type("handle_async"),
            CallType::Async
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("process_await"),
            CallType::Async
        );
    }

    #[test]
    fn test_visit_expr_function_call() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_fn".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });

        // Test async function call
        let expr: Expr = parse_quote! { async_operation() };
        extractor.visit_expr(&expr);

        // Test delegate function call
        let expr: Expr = parse_quote! { handle_request() };
        extractor.visit_expr(&expr);

        // Test direct function call
        let expr: Expr = parse_quote! { calculate_sum() };
        extractor.visit_expr(&expr);

        // Verify calls were added with correct types
        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);

        assert_eq!(calls.len(), 3);

        let async_call = calls
            .iter()
            .find(|c| c.callee.name == "async_operation")
            .unwrap();
        assert_eq!(async_call.call_type, CallType::Async);

        let delegate_call = calls
            .iter()
            .find(|c| c.callee.name == "handle_request")
            .unwrap();
        assert_eq!(delegate_call.call_type, CallType::Delegate);

        let direct_call = calls
            .iter()
            .find(|c| c.callee.name == "calculate_sum")
            .unwrap();
        assert_eq!(direct_call.call_type, CallType::Direct);
    }

    #[test]
    fn test_visit_expr_method_call() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_fn".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });

        // Test pipeline method call
        let expr: Expr = parse_quote! { result.map(|x| x * 2) };
        extractor.visit_expr(&expr);

        // Test another pipeline method
        let expr: Expr = parse_quote! { option.and_then(|v| Some(v + 1)) };
        extractor.visit_expr(&expr);

        // Test direct method call
        let expr: Expr = parse_quote! { vec.len() };
        extractor.visit_expr(&expr);

        // Verify calls
        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);

        // The visitor recursively finds:
        // - map, and_then, len (method calls)
        // - 2 closures (one in map, one in and_then)
        // - Some (constructor call inside and_then closure)
        assert!(calls.len() >= 3);

        // Verify the main method calls we care about
        let map_call = calls.iter().find(|c| c.callee.name == "map").unwrap();
        assert_eq!(map_call.call_type, CallType::Pipeline);

        let and_then_call = calls.iter().find(|c| c.callee.name == "and_then").unwrap();
        assert_eq!(and_then_call.call_type, CallType::Pipeline);

        let len_call = calls.iter().find(|c| c.callee.name == "len").unwrap();
        assert_eq!(len_call.call_type, CallType::Direct);
    }

    #[test]
    fn test_visit_expr_closure() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_fn".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });

        // Test closure detection
        let expr: Expr = parse_quote! { vec.iter().map(|x| x + 1) };
        extractor.visit_expr(&expr);

        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);

        // Should have the map call
        assert!(calls.iter().any(|c| c.callee.name == "map"));

        // Note: The closure inside map is visited but recorded as a closure call
        // This behavior depends on how syn parses nested expressions
    }

    #[test]
    fn test_visit_expr_no_current_function() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        // No current function set
        extractor.current_function = None;

        // These should not crash but also not add any calls
        let expr: Expr = parse_quote! { foo() };
        extractor.visit_expr(&expr);

        let expr: Expr = parse_quote! { bar.method() };
        extractor.visit_expr(&expr);

        // The call graph should remain empty
        assert!(extractor.call_graph.is_empty());
    }

    #[test]
    fn test_extract_function_name_simple() {
        use syn::parse_str;

        let path: syn::Path = parse_str("foo").unwrap();
        assert_eq!(
            CallGraphExtractor::extract_function_name_from_path(&path),
            Some("foo".to_string())
        );
    }

    #[test]
    fn test_extract_function_name_qualified() {
        use syn::parse_str;

        let path: syn::Path = parse_str("std::vec::Vec").unwrap();
        assert_eq!(
            CallGraphExtractor::extract_function_name_from_path(&path),
            Some("Vec".to_string())
        );
    }
}

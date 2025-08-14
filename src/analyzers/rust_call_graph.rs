use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, ImplItemFn, ItemFn};

pub struct CallGraphExtractor {
    pub call_graph: CallGraph,
    current_function: Option<FunctionId>,
    current_impl_type: Option<String>,
    current_file: PathBuf,
}

impl CallGraphExtractor {
    pub fn new(file: PathBuf) -> Self {
        Self {
            call_graph: CallGraph::new(),
            current_function: None,
            current_impl_type: None,
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

    /// Constructs a method name, qualifying it with the impl type if it's a self method
    fn construct_method_name(
        method: &syn::Ident,
        receiver: &Expr,
        current_impl_type: &Option<String>,
    ) -> String {
        let method_name = method.to_string();
        if matches!(receiver, Expr::Path(p) if p.path.is_ident("self")) {
            // This is a self method call, use the impl type if available
            if let Some(ref impl_type) = current_impl_type {
                format!("{impl_type}::{method_name}")
            } else {
                method_name
            }
        } else {
            // Regular method call on another object
            method_name
        }
    }

    /// Process arguments to check for function references and visit nested expressions
    fn process_arguments(&mut self, args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>) {
        // First pass: check for function references
        for arg in args {
            self.check_for_function_reference(arg);
        }
        // Second pass: visit nested expressions
        for arg in args {
            self.visit_expr(arg);
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

    fn check_for_function_reference(&mut self, expr: &Expr) {
        if let Expr::Path(expr_path) = expr {
            if let Some(func_name) = Self::extract_function_name_from_path(&expr_path.path) {
                // Function passed as argument
                if !is_likely_variable_name(&func_name) {
                    self.add_call(func_name, CallType::Callback);
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for CallGraphExtractor {
    fn visit_item_impl(&mut self, item_impl: &'ast syn::ItemImpl) {
        // Extract the type name from the impl block
        let impl_type = if let syn::Type::Path(type_path) = &*item_impl.self_ty {
            type_path
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
        } else {
            None
        };

        // Store the current impl type
        let prev_impl_type = self.current_impl_type.clone();
        self.current_impl_type = impl_type;

        // Continue visiting the impl block
        syn::visit::visit_item_impl(self, item_impl);

        // Restore previous impl type
        self.current_impl_type = prev_impl_type;
    }

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
        // Construct the full function name including the impl type
        let method_name = impl_fn.sig.ident.to_string();
        let name = if let Some(ref impl_type) = self.current_impl_type {
            format!("{impl_type}::{method_name}")
        } else {
            method_name.clone()
        };

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
            Expr::Call(ExprCall { func, args, .. }) => {
                if let Expr::Path(expr_path) = &**func {
                    if let Some(name) = Self::extract_function_name_from_path(&expr_path.path) {
                        self.add_call(name.clone(), Self::classify_call_type(&name));
                    }
                }
                // Process arguments for references and nested calls
                self.process_arguments(args);
                return; // Early return to avoid visiting children
            }
            // Handle method calls: obj.method()
            Expr::MethodCall(ExprMethodCall {
                method,
                args,
                receiver,
                ..
            }) => {
                let name = Self::construct_method_name(method, receiver, &self.current_impl_type);
                self.add_call(name.clone(), Self::classify_call_type(&name));

                // Process arguments and visit receiver
                self.process_arguments(args);
                self.visit_expr(receiver);
                return; // Early return to avoid visiting children
            }
            // Handle closures that might be callbacks
            Expr::Closure(closure) if self.current_function.is_some() => {
                self.add_call(
                    format!("<closure@{}>", closure.body.span().start().line),
                    CallType::Callback,
                );
                // Explicitly visit the closure body to ensure calls inside are detected
                // The body is an expression that needs to be visited
                self.visit_expr(&closure.body);
                return; // Return to avoid double-visiting
            }
            _ => {}
        }

        // Continue visiting nested expressions
        syn::visit::visit_expr(self, expr);
    }
}

fn is_likely_variable_name(name: &str) -> bool {
    // Common variable names and language keywords that shouldn't be treated as function references
    matches!(
        name,
        "self" | "super" | "crate" | "true" | "false" | "None" | "Some" | "Ok" | "Err"
    ) || name.starts_with("r#") // Raw identifiers
      || name.chars().next().is_some_and(|c| c.is_uppercase()) // Likely a constant or enum variant
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
    fn test_construct_method_name_self_method() {
        let method: syn::Ident = parse_quote!(foo);
        let receiver: Expr = parse_quote!(self);
        let impl_type = Some("MyStruct".to_string());

        assert_eq!(
            CallGraphExtractor::construct_method_name(&method, &receiver, &impl_type),
            "MyStruct::foo"
        );
    }

    #[test]
    fn test_construct_method_name_self_method_no_impl() {
        let method: syn::Ident = parse_quote!(foo);
        let receiver: Expr = parse_quote!(self);
        let impl_type = None;

        assert_eq!(
            CallGraphExtractor::construct_method_name(&method, &receiver, &impl_type),
            "foo"
        );
    }

    #[test]
    fn test_construct_method_name_regular_method() {
        let method: syn::Ident = parse_quote!(bar);
        let receiver: Expr = parse_quote!(obj);
        let impl_type = Some("MyStruct".to_string());

        assert_eq!(
            CallGraphExtractor::construct_method_name(&method, &receiver, &impl_type),
            "bar"
        );
    }

    #[test]
    fn test_visit_expr_simple_function_call() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });

        let expr: Expr = parse_quote!(foo());
        extractor.visit_expr(&expr);

        // Check that the call was recorded
        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].callee.name, "foo");
        assert_eq!(calls[0].call_type, CallType::Direct);
    }

    #[test]
    fn test_visit_expr_simple_method_call() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });
        extractor.current_impl_type = Some("MyStruct".to_string());

        let expr: Expr = parse_quote!(self.process());
        extractor.visit_expr(&expr);

        // Check that the call was recorded with impl type
        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].callee.name, "MyStruct::process");
        assert_eq!(calls[0].call_type, CallType::Direct);
    }

    #[test]
    fn test_visit_expr_async_method_call() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });

        let expr: Expr = parse_quote!(obj.async_fetch());
        extractor.visit_expr(&expr);

        // Check that the call was recorded as async
        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].callee.name, "async_fetch");
        assert_eq!(calls[0].call_type, CallType::Async);
    }

    #[test]
    fn test_visit_expr_delegate_call() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });

        let expr: Expr = parse_quote!(handle_request());
        extractor.visit_expr(&expr);

        // Check that the call was recorded as delegate
        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].callee.name, "handle_request");
        assert_eq!(calls[0].call_type, CallType::Delegate);
    }

    #[test]
    fn test_visit_expr_pipeline_call() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });

        let expr: Expr = parse_quote!(result.map(|x| x + 1));
        extractor.visit_expr(&expr);

        // Check that map was recorded as pipeline
        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);
        assert_eq!(calls.len(), 2); // map and closure
        assert_eq!(calls[0].callee.name, "map");
        assert_eq!(calls[0].call_type, CallType::Pipeline);
    }

    #[test]
    fn test_visit_expr_closure() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });

        let expr: Expr = parse_quote!(|x| x + 1);
        extractor.visit_expr(&expr);

        // Check that closure was recorded
        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);
        assert_eq!(calls.len(), 1);
        assert!(calls[0].callee.name.starts_with("<closure@"));
        assert_eq!(calls[0].call_type, CallType::Callback);
    }

    #[test]
    fn test_visit_expr_nested_calls() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });

        let expr: Expr = parse_quote!(foo(bar(), baz()));
        extractor.visit_expr(&expr);

        // Check that all calls were recorded
        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);
        assert_eq!(calls.len(), 3); // foo, bar, baz
        let names: Vec<String> = calls.iter().map(|c| c.callee.name.clone()).collect();
        assert!(names.contains(&"foo".to_string()));
        assert!(names.contains(&"bar".to_string()));
        assert!(names.contains(&"baz".to_string()));
    }

    #[test]
    fn test_visit_expr_module_path_call() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });

        let expr: Expr = parse_quote!(std::fs::read_to_string("file.txt"));
        extractor.visit_expr(&expr);

        // Check that the function name was extracted properly
        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].callee.name, "read_to_string");
    }

    #[test]
    fn test_visit_expr_callback_as_argument() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.current_function = Some(FunctionId {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        });

        let expr: Expr = parse_quote!(vec.iter().for_each(print_item));
        extractor.visit_expr(&expr);

        // Check that print_item was recorded as callback
        let func_id = extractor.current_function.as_ref().unwrap();
        let calls = extractor.call_graph.get_function_calls(func_id);
        let callback_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.callee.name == "print_item" && c.call_type == CallType::Callback)
            .collect();
        assert_eq!(callback_calls.len(), 1);
    }

    #[test]
    fn test_extract_function_name_from_path() {
        let path: syn::Path = parse_quote!(foo);
        assert_eq!(
            CallGraphExtractor::extract_function_name_from_path(&path),
            Some("foo".to_string())
        );

        let path: syn::Path = parse_quote!(module::submodule::bar);
        assert_eq!(
            CallGraphExtractor::extract_function_name_from_path(&path),
            Some("bar".to_string())
        );
    }

    #[test]
    fn test_visit_expr_function_call_with_call_graph() {
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
    fn test_visit_expr_method_call_with_call_graph() {
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
    fn test_visit_expr_closure_with_map() {
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

    #[test]
    fn test_add_function_to_graph_regular_function() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));

        // Create a regular function without test attributes or entry point pattern
        let item_fn: ItemFn = parse_quote! {
            fn calculate_sum(a: i32, b: i32) -> i32 {
                a + b
            }
        };

        extractor.add_function_to_graph("calculate_sum".to_string(), 10, &item_fn);

        // Verify function was added to graph
        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "calculate_sum".to_string(),
            line: 10,
        };

        // Verify function properties using available methods
        assert!(
            !extractor.call_graph.is_entry_point(&func_id),
            "Regular function should not be entry point"
        );
        assert!(
            !extractor.call_graph.is_test_function(&func_id),
            "Regular function should not be test"
        );

        // Verify current function was set
        assert_eq!(extractor.current_function, Some(func_id));
    }

    #[test]
    fn test_add_function_to_graph_test_function() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));

        // Create a test function with #[test] attribute
        let item_fn: ItemFn = parse_quote! {
            #[test]
            fn test_something() {
                assert_eq!(1, 1);
            }
        };

        extractor.add_function_to_graph("test_something".to_string(), 20, &item_fn);

        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "test_something".to_string(),
            line: 20,
        };

        // Verify function properties
        assert!(
            extractor.call_graph.is_test_function(&func_id),
            "Function with #[test] attribute should be marked as test"
        );
        assert!(
            !extractor.call_graph.is_entry_point(&func_id),
            "Test function should not be entry point"
        );
    }

    #[test]
    fn test_add_function_to_graph_tokio_test_function() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));

        // Create a tokio test function
        let item_fn: ItemFn = parse_quote! {
            #[tokio_test]
            async fn test_async_something() {
                assert_eq!(1, 1);
            }
        };

        extractor.add_function_to_graph("test_async_something".to_string(), 30, &item_fn);

        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "test_async_something".to_string(),
            line: 30,
        };

        // Verify function properties
        assert!(
            extractor.call_graph.is_test_function(&func_id),
            "Function with #[tokio_test] attribute should be marked as test"
        );
    }

    #[test]
    fn test_add_function_to_graph_main_entry_point() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("main.rs"));

        // Create main function
        let item_fn: ItemFn = parse_quote! {
            fn main() {
                println!("Hello, world!");
            }
        };

        extractor.add_function_to_graph("main".to_string(), 1, &item_fn);

        let func_id = FunctionId {
            file: PathBuf::from("main.rs"),
            name: "main".to_string(),
            line: 1,
        };

        // Verify function properties
        assert!(
            extractor.call_graph.is_entry_point(&func_id),
            "main function should be marked as entry point"
        );
        assert!(
            !extractor.call_graph.is_test_function(&func_id),
            "main function should not be test"
        );
    }

    #[test]
    fn test_add_function_to_graph_handle_entry_point() {
        // Test all entry point patterns
        let test_cases = vec![
            ("handle_request", true),
            ("process_data", true),
            ("run_pipeline", true),
            ("execute_command", true),
            ("handle_", true),           // Edge case: just the prefix
            ("handler_function", false), // Should NOT be entry point
            ("some_handle", false),      // Should NOT be entry point
        ];

        for (name, should_be_entry) in test_cases {
            let item_fn: ItemFn = parse_quote! {
                fn some_function() {}
            };

            let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
            extractor.add_function_to_graph(name.to_string(), 1, &item_fn);

            let func_id = FunctionId {
                file: PathBuf::from("test.rs"),
                name: name.to_string(),
                line: 1,
            };

            // Verify entry point status
            assert_eq!(
                extractor.call_graph.is_entry_point(&func_id),
                should_be_entry,
                "Function '{name}' entry point status mismatch"
            );
        }
    }

    #[test]
    fn test_add_function_to_graph_with_complexity() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("complex.rs"));

        // Create a function with some complexity
        let item_fn: ItemFn = parse_quote! {
            fn complex_function(x: i32) -> i32 {
                if x > 0 {
                    if x > 10 {
                        x * 2
                    } else {
                        x + 1
                    }
                } else {
                    match x {
                        -1 => 0,
                        -2 => 1,
                        _ => -x
                    }
                }
            }
        };

        extractor.add_function_to_graph("complex_function".to_string(), 50, &item_fn);

        let func_id = FunctionId {
            file: PathBuf::from("complex.rs"),
            name: "complex_function".to_string(),
            line: 50,
        };

        // Verify function was added to the graph
        // We can verify it exists by checking if it has no callers initially
        let callers = extractor.call_graph.get_callers(&func_id);
        assert_eq!(callers.len(), 0, "New function should have no callers");

        // Verify it's not marked as test or entry point (since name doesn't match patterns)
        assert!(!extractor.call_graph.is_test_function(&func_id));
        assert!(!extractor.call_graph.is_entry_point(&func_id));
    }
}

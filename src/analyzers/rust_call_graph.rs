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
                    if let Some(func_name) = Self::extract_function_name_from_path(&expr_path.path)
                    {
                        // Determine call type based on patterns
                        let call_type =
                            if func_name.contains("async") || func_name.contains("await") {
                                CallType::Async
                            } else if func_name.starts_with("handle_")
                                || func_name.starts_with("process_")
                            {
                                CallType::Delegate
                            } else {
                                CallType::Direct
                            };

                        self.add_call(func_name, call_type);
                    }
                }
            }
            // Handle method calls: obj.method()
            Expr::MethodCall(ExprMethodCall { method, .. }) => {
                let method_name = method.to_string();

                // Determine call type
                let call_type = if method_name == "await" {
                    CallType::Async
                } else if method_name.starts_with("map") || method_name.starts_with("and_then") {
                    CallType::Pipeline
                } else {
                    CallType::Direct
                };

                self.add_call(method_name, call_type);
            }
            // Handle closures that might be callbacks
            Expr::Closure(closure) => {
                // If closure is passed as argument, it might be a callback
                if self.current_function.is_some() {
                    let closure_name = format!("<closure@{}>", closure.body.span().start().line);
                    self.add_call(closure_name, CallType::Callback);
                }
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

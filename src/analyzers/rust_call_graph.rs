/// Two-pass call graph extraction for accurate call resolution
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::{Path, PathBuf};
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, ImplItemFn, ItemFn};

/// Represents an unresolved function call that needs to be resolved in phase 2
#[derive(Debug, Clone)]
struct UnresolvedCall {
    caller: FunctionId,
    callee_name: String,
    call_type: CallType,
    same_file_hint: bool, // Hint that this is likely a same-file call
}

/// Call graph extractor that uses two-pass resolution for accurate call tracking
pub struct CallGraphExtractor {
    pub call_graph: CallGraph,
    unresolved_calls: Vec<UnresolvedCall>,
    current_function: Option<FunctionId>,
    current_impl_type: Option<String>,
    current_file: PathBuf,
}

impl CallGraphExtractor {
    pub fn new(file: PathBuf) -> Self {
        Self {
            call_graph: CallGraph::new(),
            unresolved_calls: Vec::new(),
            current_function: None,
            current_impl_type: None,
            current_file: file,
        }
    }

    /// Phase 1: Extract all functions and collect unresolved calls
    fn extract_phase1(&mut self, file: &syn::File) {
        self.visit_file(file);
    }

    /// Phase 2: Resolve all calls now that we know all functions
    fn resolve_phase2(&mut self) {
        let unresolved = std::mem::take(&mut self.unresolved_calls);

        for call in unresolved {
            // Try to resolve the callee
            if let Some(resolved_callee) =
                self.resolve_function(&call.callee_name, &call.caller, call.same_file_hint)
            {
                self.call_graph.add_call(FunctionCall {
                    caller: call.caller,
                    callee: resolved_callee,
                    call_type: call.call_type,
                });
            }
            // If resolution fails, the call is simply not added (could log this)
        }
    }

    /// Resolve a function name to a FunctionId
    fn resolve_function(
        &self,
        name: &str,
        caller: &FunctionId,
        same_file_hint: bool,
    ) -> Option<FunctionId> {
        let all_functions = self.call_graph.find_all_functions();

        // If same_file_hint is true, prioritize same-file matches
        if same_file_hint {
            // First try exact match in same file
            if let Some(func) = all_functions
                .iter()
                .find(|f| f.name == name && f.file == caller.file)
            {
                return Some(func.clone());
            }

            // For method calls, try with type prefix
            if let Some(impl_type) = self.extract_impl_type_from_caller(&caller.name) {
                let qualified_name = format!("{}::{}", impl_type, name);
                if let Some(func) = all_functions
                    .iter()
                    .find(|f| f.name == qualified_name && f.file == caller.file)
                {
                    return Some(func.clone());
                }
            }
        }

        // Try cross-file resolution
        let matches: Vec<_> = all_functions.iter().filter(|f| f.name == name).collect();

        match matches.len() {
            1 => Some(matches[0].clone()), // Unique match across all files
            0 => None,                     // No match found
            _ => {
                // Multiple matches - try to pick the best one
                // Prefer same file if available
                matches
                    .iter()
                    .find(|f| f.file == caller.file)
                    .or_else(|| matches.first())
                    .map(|f| (*f).clone())
            }
        }
    }

    /// Extract impl type from a function name like "TypeName::method"
    fn extract_impl_type_from_caller(&self, caller_name: &str) -> Option<String> {
        caller_name.split("::").next().map(|s| s.to_string())
    }

    /// Add an unresolved call to be resolved later
    fn add_unresolved_call(
        &mut self,
        callee_name: String,
        call_type: CallType,
        same_file_hint: bool,
    ) {
        if let Some(ref caller) = self.current_function {
            self.unresolved_calls.push(UnresolvedCall {
                caller: caller.clone(),
                callee_name,
                call_type,
                same_file_hint,
            });
        }
    }

    /// Classify a function/method name into its call type
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

    /// Extract function name from a path expression
    fn extract_function_name_from_path(path: &syn::Path) -> Option<String> {
        if path.segments.is_empty() {
            return None;
        }

        // For simple paths like `foo` or complex like `module::foo`
        // We want the full path for cross-module resolution
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect();

        Some(segments.join("::"))
    }

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        // Use proc-macro2's span-locations feature to get actual line numbers
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

    fn add_impl_method_to_graph(&mut self, name: String, line: usize, impl_fn: &ImplItemFn) {
        let func_id = FunctionId {
            file: self.current_file.clone(),
            name: name.clone(),
            line,
        };

        // Check for test attribute
        let is_test = impl_fn.attrs.iter().any(|attr| {
            attr.path()
                .segments
                .iter()
                .any(|s| s.ident == "test" || s.ident == "tokio_test")
        });

        let complexity = calculate_basic_complexity(&impl_fn.block);
        let lines = count_lines(&impl_fn.block);

        self.call_graph
            .add_function(func_id.clone(), false, is_test, complexity, lines);

        self.current_function = Some(func_id);
    }

    /// Process arguments to check for function references and visit nested expressions
    fn process_arguments(&mut self, args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>) {
        for arg in args {
            self.check_for_function_reference(arg);
            // Visit the argument to detect nested calls
            self.visit_expr(arg);
        }
    }

    fn check_for_function_reference(&mut self, expr: &Expr) {
        if let Expr::Path(expr_path) = expr {
            if let Some(name) = Self::extract_function_name_from_path(&expr_path.path) {
                // This is a function being passed as an argument (treat as callback)
                self.add_unresolved_call(
                    format!("<fn_ref:{}>", name),
                    CallType::Callback,
                    true, // Likely same file
                );
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
        let method_name = impl_fn.sig.ident.to_string();
        let line = self.get_line_number(impl_fn.sig.ident.span());

        // Create the qualified name if we're in an impl block
        let name = if let Some(ref impl_type) = self.current_impl_type {
            format!("{impl_type}::{method_name}")
        } else {
            method_name
        };

        // Add function to graph
        self.add_impl_method_to_graph(name, line, impl_fn);

        // Visit the function body to extract calls
        syn::visit::visit_impl_item_fn(self, impl_fn);

        // Clear current function after visiting
        self.current_function = None;
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Handle regular function calls: foo(), module::foo(), Self::method()
            Expr::Call(ExprCall { func, args, .. }) => {
                if let Expr::Path(expr_path) = &**func {
                    if let Some(mut name) = Self::extract_function_name_from_path(&expr_path.path) {
                        // Handle Self:: calls by replacing with the current impl type
                        if name.starts_with("Self::") {
                            if let Some(ref impl_type) = self.current_impl_type {
                                name = name.replace("Self::", &format!("{}::", impl_type));
                            }
                        }
                        let same_file_hint = !name.contains("::")
                            || self
                                .current_impl_type
                                .as_ref()
                                .is_some_and(|t| name.starts_with(t));
                        self.add_unresolved_call(
                            name.clone(),
                            Self::classify_call_type(&name),
                            same_file_hint,
                        );
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
                let same_file_hint =
                    matches!(&**receiver, Expr::Path(p) if p.path.is_ident("self"));
                self.add_unresolved_call(
                    name.clone(),
                    Self::classify_call_type(&name),
                    same_file_hint,
                );

                // Process arguments and visit receiver
                self.process_arguments(args);
                self.visit_expr(receiver);
                return; // Early return to avoid visiting children
            }
            // Handle closures that might contain calls
            Expr::Closure(closure) => {
                // Visit the closure body to detect calls inside
                self.visit_expr(&closure.body);
                return;
            }
            // Handle async blocks
            Expr::Async(async_block) => {
                for stmt in &async_block.block.stmts {
                    self.visit_stmt(stmt);
                }
                return;
            }
            // Handle await expressions
            Expr::Await(await_expr) => {
                self.visit_expr(&await_expr.base);
                return;
            }
            _ => {}
        }

        // Continue visiting for other expression types
        syn::visit::visit_expr(self, expr);
    }
}

/// Helper function to calculate basic cyclomatic complexity
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

/// Extract call graph from a parsed Rust file using two-pass resolution
pub fn extract_call_graph(file: &syn::File, path: &Path) -> CallGraph {
    let mut extractor = CallGraphExtractor::new(path.to_path_buf());

    // Phase 1: Extract functions and collect unresolved calls
    extractor.extract_phase1(file);

    // Phase 2: Resolve all calls
    extractor.resolve_phase2();

    extractor.call_graph
}

/// Merge a file's call graph into the main call graph (placeholder for compatibility)
pub fn merge_call_graphs(_main: &mut CallGraph, _file_graph: CallGraph) {
    // This is handled by CallGraph::merge method now
}

/// Two-pass call graph extraction for accurate call resolution
use crate::analyzers::function_registry::FunctionSignatureRegistry;
use crate::analyzers::signature_extractor::SignatureExtractor;
use crate::analyzers::type_registry::GlobalTypeRegistry;
use crate::analyzers::type_tracker::{extract_type_from_pattern, ScopeKind, TypeTracker};
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, ImplItemFn, Item, ItemFn, Local, Pat};

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
    module_path: Vec<String>,
    /// Type tracker for accurate method resolution
    type_tracker: TypeTracker,
    /// Global type registry (optional)
    #[allow(dead_code)]
    type_registry: Option<Arc<GlobalTypeRegistry>>,
    /// Function signature registry for return type resolution
    #[allow(dead_code)]
    function_registry: Option<Arc<FunctionSignatureRegistry>>,
}

impl CallGraphExtractor {
    pub fn new(file: PathBuf) -> Self {
        Self {
            call_graph: CallGraph::new(),
            unresolved_calls: Vec::new(),
            current_function: None,
            current_impl_type: None,
            current_file: file.clone(),
            module_path: Vec::new(),
            type_tracker: TypeTracker::new(),
            type_registry: None,
            function_registry: None,
        }
    }

    /// Create a new extractor with a shared type registry
    pub fn with_registry(file: PathBuf, registry: Arc<GlobalTypeRegistry>) -> Self {
        let mut tracker = TypeTracker::with_registry(registry.clone());
        tracker.set_current_file(file.clone());

        Self {
            call_graph: CallGraph::new(),
            unresolved_calls: Vec::new(),
            current_function: None,
            current_impl_type: None,
            current_file: file,
            module_path: Vec::new(),
            type_tracker: tracker,
            type_registry: Some(registry),
            function_registry: None,
        }
    }

    /// Set the function signature registry
    pub fn set_function_registry(&mut self, registry: Arc<FunctionSignatureRegistry>) {
        self.type_tracker.set_function_registry(registry.clone());
        self.function_registry = Some(registry);
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
            // If resolution fails, the call is simply not added
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

        // Handle special path prefixes
        let resolved_name = if name.starts_with("crate::") {
            // crate:: refers to the root of the current crate
            name.strip_prefix("crate::").unwrap().to_string()
        } else if name.starts_with("super::") {
            // super:: refers to the parent module
            // For simplicity, we'll just strip it and try to match
            name.strip_prefix("super::").unwrap().to_string()
        } else {
            name.to_string()
        };

        // First try exact match in same file
        if same_file_hint {
            if let Some(func) = all_functions
                .iter()
                .find(|f| (f.name == resolved_name || f.name == name) && f.file == caller.file)
            {
                return Some(func.clone());
            }

            // For method calls, try with type prefix
            if let Some(impl_type) = self.extract_impl_type_from_caller(&caller.name) {
                let qualified_name = format!("{}::{}", impl_type, resolved_name);
                if let Some(func) = all_functions
                    .iter()
                    .find(|f| f.name == qualified_name && f.file == caller.file)
                {
                    return Some(func.clone());
                }
            }
        }

        // Enhanced cross-module resolution with prioritization
        let mut matches: Vec<_> = all_functions
            .iter()
            .filter(|f| {
                // Exact match (highest priority)
                if f.name == resolved_name || f.name == name {
                    return true;
                }

                // Function name ends with the target (e.g., "SomeType::new" matches "new")
                if f.name.ends_with(&format!("::{}", resolved_name)) {
                    return true;
                }

                // Cross-module associated function call resolution:
                // For calls like "ContextualRisk::new", match against functions with exactly that name
                // even if they're in different files
                if name.contains("::") && f.name == name {
                    return true;
                }

                // Module-qualified match: strip common module prefixes and try to match
                // BUT only if the type names also match to avoid false matches
                if let Some(base_name) = name.split("::").last() {
                    if let Some(func_base_name) = f.name.split("::").last() {
                        if base_name == func_base_name {
                            // Additional check: if both have type prefixes, they should match
                            let name_parts: Vec<&str> = name.split("::").collect();
                            let func_parts: Vec<&str> = f.name.split("::").collect();

                            if name_parts.len() >= 2 && func_parts.len() >= 2 {
                                // Both have type::method format, check if types match
                                return name_parts[0] == func_parts[0];
                            } else {
                                // At least one is unqualified, allow the match
                                return true;
                            }
                        }
                    }
                }

                false
            })
            .collect();

        // Sort matches to prioritize qualified names over unqualified ones
        // This helps resolve ambiguity between "method_name" and "Type::method_name"
        matches.sort_by(|a, b| {
            let a_qualified = a.name.contains("::");
            let b_qualified = b.name.contains("::");
            match (a_qualified, b_qualified) {
                (true, false) => std::cmp::Ordering::Less, // Prefer qualified names
                (false, true) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });

        match matches.len() {
            1 => Some(matches[0].clone()), // Unique match across all files
            0 => None,                     // No match found
            _ => {
                // Multiple matches - use sophisticated disambiguation

                // For unqualified method names (like "calculate_coupling_metrics"),
                // prefer qualified matches (like "Type::calculate_coupling_metrics")
                // over standalone functions with the same name.
                // This helps resolve method calls correctly.

                // 1. If the name doesn't contain "::" (unqualified), prefer qualified matches
                if !name.contains("::") {
                    // Look for qualified matches first (Type::method)
                    if let Some(qualified_match) = matches
                        .iter()
                        .find(|f| f.name.contains("::") && f.name.ends_with(&format!("::{}", name)))
                    {
                        return Some((*qualified_match).clone());
                    }
                }

                // 2. Prefer exact name match
                if let Some(exact_match) = matches
                    .iter()
                    .find(|f| f.name == name || f.name == resolved_name)
                {
                    return Some((*exact_match).clone());
                }

                // 3. Prefer same file if available
                if let Some(same_file_match) = matches.iter().find(|f| f.file == caller.file) {
                    return Some((*same_file_match).clone());
                }

                // 4. For associated function calls (Type::method), prefer the match that looks most like an impl
                if name.contains("::") {
                    if let Some(impl_match) = matches.iter().find(|f| f.name == name) {
                        return Some((*impl_match).clone());
                    }
                }

                // 5. Fall back to the first match (after sorting)
                matches.first().map(|f| (*f).clone())
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

    /// Constructs a method name, qualifying it with the type information when available
    fn construct_method_name(
        &self,
        method: &syn::Ident,
        receiver: &Expr,
        current_impl_type: &Option<String>,
    ) -> String {
        let method_name = method.to_string();

        // First check if it's a self method call
        if matches!(receiver, Expr::Path(p) if p.path.is_ident("self")) {
            // This is a self method call, use the impl type if available
            if let Some(ref impl_type) = current_impl_type {
                return format!("{impl_type}::{method_name}");
            }
        } else {
            // Try to resolve the receiver's type using the type tracker
            if let Some(resolved_type) = self.type_tracker.resolve_expr_type(receiver) {
                return format!("{}::{}", resolved_type.type_name, method_name);
            }
        }

        // Fallback to unqualified name
        method_name
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
    fn visit_local(&mut self, local: &'ast Local) {
        // Track type when visiting variable declarations
        if let Pat::Ident(pat_ident) = &local.pat {
            let var_name = pat_ident.ident.to_string();
            // Convert LocalInit to Option<Box<Expr>>
            let init_expr = local.init.as_ref().map(|init| init.expr.clone());
            if let Some(ty) = extract_type_from_pattern(&local.pat, &init_expr) {
                self.type_tracker.record_variable(var_name, ty);
            }
        }

        // Continue visiting
        syn::visit::visit_local(self, local);
    }

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
        self.current_impl_type = impl_type.clone();

        // Enter impl scope in type tracker
        self.type_tracker.enter_scope(ScopeKind::Impl, impl_type);

        // Continue visiting the impl block
        syn::visit::visit_item_impl(self, item_impl);

        // Exit impl scope
        self.type_tracker.exit_scope();

        // Restore previous impl type
        self.current_impl_type = prev_impl_type;
    }

    fn visit_item_mod(&mut self, item_mod: &'ast syn::ItemMod) {
        // Push module name to path
        self.module_path.push(item_mod.ident.to_string());

        // Visit module contents
        syn::visit::visit_item_mod(self, item_mod);

        // Pop module name from path
        self.module_path.pop();
    }

    fn visit_item_fn(&mut self, item_fn: &'ast ItemFn) {
        let base_name = item_fn.sig.ident.to_string();
        let line = self.get_line_number(item_fn.sig.ident.span());

        // Build qualified name with module path
        let name = if self.module_path.is_empty() {
            base_name
        } else {
            format!("{}::{}", self.module_path.join("::"), base_name)
        };

        // Add function to graph
        self.add_function_to_graph(name, line, item_fn);

        // Enter function scope
        self.type_tracker.enter_scope(ScopeKind::Function, None);

        // Track self parameter if present
        self.type_tracker.track_self_param(Some(item_fn), None);

        // Visit the function body to extract calls
        syn::visit::visit_item_fn(self, item_fn);

        // Exit function scope
        self.type_tracker.exit_scope();

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

        // Enter function scope
        self.type_tracker.enter_scope(ScopeKind::Function, None);

        // Track self parameter if present
        self.type_tracker.track_self_param(None, Some(impl_fn));

        // Visit the function body to extract calls
        syn::visit::visit_impl_item_fn(self, impl_fn);

        // Exit function scope
        self.type_tracker.exit_scope();

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
                let name = self.construct_method_name(method, receiver, &self.current_impl_type);
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
            // Handle struct literals to find function calls in field values
            Expr::Struct(expr_struct) => {
                // Visit each field's value expression to detect function calls
                for field in &expr_struct.fields {
                    self.visit_expr(&field.expr);
                }
                // If there's a base struct (e.g., Foo { field: value, ..base })
                if let Some(ref base) = expr_struct.rest {
                    self.visit_expr(base);
                }
                return;
            }
            // Handle macros like vec![] that might contain function calls
            Expr::Macro(expr_macro) => {
                // Try to parse the macro tokens as expressions
                // This handles common cases like vec![...] where the content is valid expressions
                if let Ok(parsed_expr) = syn::parse2::<Expr>(expr_macro.mac.tokens.clone()) {
                    self.visit_expr(&parsed_expr);
                }
                // Also continue with default visiting in case there's more to process
                syn::visit::visit_expr(self, expr);
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

/// Extract call graph with enhanced type tracking using a global type registry
pub fn extract_call_graph_with_types(
    file: &syn::File,
    path: &Path,
    registry: Arc<GlobalTypeRegistry>,
) -> CallGraph {
    let mut extractor = CallGraphExtractor::with_registry(path.to_path_buf(), registry);

    // Phase 1: Extract functions and collect unresolved calls
    extractor.extract_phase1(file);

    // Phase 2: Resolve all calls
    extractor.resolve_phase2();

    extractor.call_graph
}

/// Extract call graph with function signatures for enhanced return type resolution
pub fn extract_call_graph_with_signatures(
    file: &syn::File,
    path: &Path,
    registry: Arc<GlobalTypeRegistry>,
) -> (CallGraph, Arc<FunctionSignatureRegistry>) {
    // First extract function signatures
    let mut sig_extractor = SignatureExtractor::new();
    sig_extractor.extract_from_file(file);
    let function_registry = Arc::new(sig_extractor.registry);

    // Create call graph extractor with both registries
    let mut extractor = CallGraphExtractor::with_registry(path.to_path_buf(), registry);
    extractor.set_function_registry(function_registry.clone());

    // Phase 1: Extract functions and collect unresolved calls
    extractor.extract_phase1(file);

    // Phase 2: Resolve all calls
    extractor.resolve_phase2();

    (extractor.call_graph, function_registry)
}

/// Merge a file's call graph into the main call graph (placeholder for compatibility)
pub fn merge_call_graphs(_main: &mut CallGraph, _file_graph: CallGraph) {
    // This is handled by CallGraph::merge method now
}

/// Collect type definitions from a file into the global type registry
fn collect_types_from_file(registry: &mut GlobalTypeRegistry, file: &syn::File, _path: &Path) {
    // Track the module path as we traverse the file
    let module_path = Vec::new();

    for item in &file.items {
        match item {
            Item::Struct(item_struct) => {
                // Register the struct with its fields
                registry.register_struct(module_path.clone(), item_struct);
            }
            Item::Mod(item_mod) => {
                // Handle nested modules
                if let Some((_, items)) = &item_mod.content {
                    let mut nested_path = module_path.clone();
                    nested_path.push(item_mod.ident.to_string());

                    // Recursively process items in nested module
                    for nested_item in items {
                        if let Item::Struct(nested_struct) = nested_item {
                            registry.register_struct(nested_path.clone(), nested_struct);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Extract call graph from multiple files with cross-file resolution
pub fn extract_call_graph_multi_file(files: &[(syn::File, PathBuf)]) -> CallGraph {
    // Create a global type registry for cross-module type resolution
    let mut type_registry = GlobalTypeRegistry::new();

    // Phase 1a: First pass - collect all type definitions from all files
    // This ensures we have complete type information before resolving method calls
    for (file, path) in files {
        collect_types_from_file(&mut type_registry, file, path);
    }

    // Now wrap in Arc for sharing
    let type_registry = Arc::new(type_registry);

    // Create the combined extractor with the populated type registry
    let mut combined_extractor =
        CallGraphExtractor::with_registry(PathBuf::from("multi_file"), type_registry.clone());

    // Phase 1b: Extract all functions from all files and collect all unresolved calls
    // Now each file extractor has access to the complete type registry
    for (file, path) in files {
        let mut file_extractor =
            CallGraphExtractor::with_registry(path.clone(), type_registry.clone());
        file_extractor.extract_phase1(file);

        // Merge the functions and unresolved calls into the combined extractor
        combined_extractor
            .call_graph
            .merge(file_extractor.call_graph);
        combined_extractor
            .unresolved_calls
            .extend(file_extractor.unresolved_calls);
    }

    // Phase 2: Resolve all calls now that we know ALL functions from ALL files
    // and have complete type information
    combined_extractor.resolve_phase2();

    combined_extractor.call_graph
}

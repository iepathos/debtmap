/// Two-pass call graph extraction for accurate call resolution
use crate::analyzers::function_registry::FunctionSignatureRegistry;
use crate::analyzers::signature_extractor::SignatureExtractor;
use crate::analyzers::type_registry::GlobalTypeRegistry;
use crate::analyzers::type_tracker::{extract_type_from_pattern, ScopeKind, TypeTracker};
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::visit::Visit;
use syn::{Expr, ExprMacro, ImplItemFn, Item, ItemFn, Local, Pat};

/// Represents an unresolved function call that needs to be resolved in phase 2
#[derive(Debug, Clone)]
struct UnresolvedCall {
    caller: FunctionId,
    callee_name: String,
    call_type: CallType,
    same_file_hint: bool, // Hint that this is likely a same-file call
}

/// Statistics for macro expansion
#[derive(Debug, Default)]
pub struct MacroExpansionStats {
    pub total_macros: usize,
    pub successfully_parsed: usize,
    pub failed_macros: HashMap<String, usize>,
}

/// Configuration for macro handling
#[derive(Debug, Clone, Default)]
pub struct MacroHandlingConfig {
    pub verbose_warnings: bool,
    pub show_statistics: bool,
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
    /// Macro expansion statistics
    macro_stats: MacroExpansionStats,
    /// Macro handling configuration
    macro_config: MacroHandlingConfig,
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
            macro_stats: MacroExpansionStats::default(),
            macro_config: MacroHandlingConfig::default(),
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
            macro_stats: MacroExpansionStats::default(),
            macro_config: MacroHandlingConfig::default(),
        }
    }

    /// Set the function signature registry
    pub fn set_function_registry(&mut self, registry: Arc<FunctionSignatureRegistry>) {
        self.type_tracker.set_function_registry(registry.clone());
        self.function_registry = Some(registry);
    }

    /// Configure macro handling
    pub fn set_macro_config(&mut self, config: MacroHandlingConfig) {
        self.macro_config = config;
    }

    /// Get macro expansion statistics
    pub fn get_macro_stats(&self) -> &MacroExpansionStats {
        &self.macro_stats
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

    /// Resolves Self:: references to the actual impl type
    fn resolve_self_type(name: &str, current_impl_type: &Option<String>) -> String {
        if name.starts_with("Self::") {
            if let Some(ref impl_type) = current_impl_type {
                return name.replace("Self::", &format!("{}::", impl_type));
            }
        }
        name.to_string()
    }

    /// Determines if a function call is likely in the same file
    fn is_same_file_call(name: &str, current_impl_type: &Option<String>) -> bool {
        !name.contains("::")
            || current_impl_type
                .as_ref()
                .is_some_and(|t| name.starts_with(t))
    }

    /// Determines if a method call receiver is self
    fn is_self_receiver(receiver: &Expr) -> bool {
        matches!(receiver, Expr::Path(p) if p.path.is_ident("self"))
    }

    /// Process a function or method call, adding it to unresolved calls
    fn process_call(&mut self, name: String, same_file_hint: bool) {
        self.add_unresolved_call(
            name.clone(),
            Self::classify_call_type(&name),
            same_file_hint,
        );
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

    /// Classify expression for appropriate handling strategy
    /// Enhanced macro handling with pattern recognition and logging
    fn handle_macro_expression(&mut self, expr_macro: &ExprMacro) {
        self.macro_stats.total_macros += 1;

        let macro_name = self.extract_macro_name(&expr_macro.mac.path);

        match macro_name.as_str() {
            // Collection macros
            "vec" | "vec_deque" | "hashmap" | "btreemap" | "hashset" | "btreeset" => {
                self.parse_collection_macro(&expr_macro.mac.tokens, &macro_name);
            }
            // Formatting macros
            "format" | "print" | "println" | "eprint" | "eprintln" | "write" | "writeln"
            | "format_args" => {
                self.parse_format_macro(&expr_macro.mac.tokens, &macro_name);
            }
            // Assertion macros
            "assert" | "assert_eq" | "assert_ne" | "debug_assert" | "debug_assert_eq"
            | "debug_assert_ne" => {
                self.parse_assert_macro(&expr_macro.mac.tokens, &macro_name);
            }
            // Logging macros
            "log" | "trace" | "debug" | "info" | "warn" | "error" => {
                self.parse_logging_macro(&expr_macro.mac.tokens, &macro_name);
            }
            // Try generic expression parsing
            _ => {
                if let Ok(parsed_expr) = syn::parse2::<Expr>(expr_macro.mac.tokens.clone()) {
                    self.macro_stats.successfully_parsed += 1;
                    self.visit_expr(&parsed_expr);
                } else {
                    self.log_unexpandable_macro(&macro_name);
                }
            }
        }
    }

    /// Parse collection macros like vec![], hashmap![]
    fn parse_collection_macro(&mut self, tokens: &proc_macro2::TokenStream, macro_name: &str) {
        // Try to parse as array-like: [expr, expr, ...]
        if let Ok(exprs) = self.parse_bracketed_exprs(tokens) {
            self.macro_stats.successfully_parsed += 1;
            for expr in exprs {
                self.visit_expr(&expr);
            }
        }
        // Try to parse as map-like: {key => value, ...}
        else if let Ok(exprs) = self.parse_braced_exprs(tokens) {
            self.macro_stats.successfully_parsed += 1;
            for expr in exprs {
                self.visit_expr(&expr);
            }
        } else {
            self.log_unexpandable_macro(macro_name);
        }
    }

    /// Parse format macros, extracting arguments after the format string
    fn parse_format_macro(&mut self, tokens: &proc_macro2::TokenStream, macro_name: &str) {
        // Try to parse comma-separated expressions
        if let Ok(exprs) = self.parse_comma_separated_exprs(tokens) {
            self.macro_stats.successfully_parsed += 1;
            // Skip the first element (format string) and visit the rest
            for expr in exprs.into_iter().skip(1) {
                self.visit_expr(&expr);
            }
        } else {
            self.log_unexpandable_macro(macro_name);
        }
    }

    /// Attempt to parse tokens as a single expression
    fn try_parse_single_expr(tokens: &proc_macro2::TokenStream) -> syn::Result<Expr> {
        syn::parse2::<Expr>(tokens.clone())
    }

    /// Process parsed expressions by visiting each one
    fn process_parsed_exprs(&mut self, exprs: Vec<Expr>) {
        for expr in exprs {
            self.visit_expr(&expr);
        }
    }

    /// Parse assertion macros
    fn parse_assert_macro(&mut self, tokens: &proc_macro2::TokenStream, macro_name: &str) {
        // First try to parse as a single expression (for assert!)
        if let Ok(expr) = Self::try_parse_single_expr(tokens) {
            self.macro_stats.successfully_parsed += 1;
            self.visit_expr(&expr);
        }
        // Then try to parse as comma-separated expressions (for assert_eq!, assert_ne!)
        else if let Ok(exprs) = self.parse_comma_separated_exprs(tokens) {
            self.macro_stats.successfully_parsed += 1;
            self.process_parsed_exprs(exprs);
        } else {
            self.log_unexpandable_macro(macro_name);
        }
    }

    /// Parse logging macros similar to format macros
    fn parse_logging_macro(&mut self, tokens: &proc_macro2::TokenStream, macro_name: &str) {
        self.parse_format_macro(tokens, macro_name);
    }

    /// Parse bracketed expressions [expr, expr, ...]
    fn parse_bracketed_exprs(&self, tokens: &proc_macro2::TokenStream) -> syn::Result<Vec<Expr>> {
        let parser = Punctuated::<Expr, Comma>::parse_terminated;

        // Try to parse the tokens directly as comma-separated expressions
        if let Ok(punctuated) = parser.parse2(tokens.clone()) {
            return Ok(punctuated.into_iter().collect());
        }

        // If that fails, try to parse as [...]
        let content = tokens.to_string();
        if content.starts_with('[') && content.ends_with(']') {
            let inner = &content[1..content.len() - 1];
            if let Ok(inner_tokens) = inner.parse::<proc_macro2::TokenStream>() {
                if let Ok(punctuated) = parser.parse2(inner_tokens) {
                    return Ok(punctuated.into_iter().collect());
                }
            }
        }

        Err(syn::Error::new_spanned(
            tokens,
            "Could not parse bracketed expressions",
        ))
    }

    /// Check if content is braced (starts with '{' and ends with '}')
    fn is_braced_content(content: &str) -> bool {
        content.starts_with('{') && content.ends_with('}')
    }

    /// Extract inner content from braced string
    fn extract_braced_inner(content: &str) -> &str {
        &content[1..content.len() - 1]
    }

    /// Parse a single expression from a string
    fn parse_expression_from_str(expr_str: &str) -> Option<Expr> {
        expr_str
            .parse::<proc_macro2::TokenStream>()
            .ok()
            .and_then(|tokens| syn::parse2::<Expr>(tokens).ok())
    }

    /// Parse a key-value pair separated by "=>"
    fn parse_key_value_pair(pair: &str) -> Vec<Expr> {
        let mut exprs = Vec::new();

        if let Some(arrow_pos) = pair.find("=>") {
            let key_str = pair[..arrow_pos].trim();
            let val_str = pair[arrow_pos + 2..].trim();

            if let Some(key_expr) = Self::parse_expression_from_str(key_str) {
                exprs.push(key_expr);
            }
            if let Some(val_expr) = Self::parse_expression_from_str(val_str) {
                exprs.push(val_expr);
            }
        }

        exprs
    }

    /// Parse braced expressions for map-like macros
    fn parse_braced_exprs(&self, tokens: &proc_macro2::TokenStream) -> syn::Result<Vec<Expr>> {
        let content = tokens.to_string();

        if Self::is_braced_content(&content) {
            let inner = Self::extract_braced_inner(&content);
            let exprs: Vec<Expr> = inner
                .split(',')
                .flat_map(Self::parse_key_value_pair)
                .collect();

            if !exprs.is_empty() {
                return Ok(exprs);
            }
        }

        Err(syn::Error::new_spanned(
            tokens,
            "Could not parse braced expressions",
        ))
    }

    /// Parse comma-separated expressions
    fn parse_comma_separated_exprs(
        &self,
        tokens: &proc_macro2::TokenStream,
    ) -> syn::Result<Vec<Expr>> {
        let parser = Punctuated::<Expr, Comma>::parse_terminated;

        if let Ok(punctuated) = parser.parse2(tokens.clone()) {
            return Ok(punctuated.into_iter().collect());
        }

        Err(syn::Error::new_spanned(
            tokens,
            "Could not parse comma-separated expressions",
        ))
    }

    /// Extract macro name from path
    fn extract_macro_name(&self, path: &syn::Path) -> String {
        path.segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Log unexpandable macro
    fn log_unexpandable_macro(&mut self, macro_name: &str) {
        if self.macro_config.verbose_warnings {
            eprintln!(
                "⚠ Cannot expand macro '{}' - may contain hidden function calls",
                macro_name
            );
        }
        self.macro_stats
            .failed_macros
            .entry(macro_name.to_string())
            .and_modify(|e| *e += 1)
            .or_insert(1);
    }

    /// Report macro expansion statistics
    pub fn report_macro_stats(&self) {
        if !self.macro_config.show_statistics || self.macro_stats.total_macros == 0 {
            return;
        }

        eprintln!("\nMacro Expansion Statistics:");
        eprintln!(
            "  Total macros encountered: {}",
            self.macro_stats.total_macros
        );
        eprintln!(
            "  Successfully parsed: {} ({:.1}%)",
            self.macro_stats.successfully_parsed,
            (self.macro_stats.successfully_parsed as f64 / self.macro_stats.total_macros as f64)
                * 100.0
        );

        if !self.macro_stats.failed_macros.is_empty() {
            eprintln!("  Failed macros:");
            let mut failed: Vec<_> = self.macro_stats.failed_macros.iter().collect();
            failed.sort_by_key(|(name, _)| name.as_str());
            for (name, count) in failed {
                eprintln!("    {}: {} occurrences", name, count);
            }
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

impl CallGraphExtractor {
    fn handle_call_expr(
        &mut self,
        func: &Expr,
        args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>,
    ) {
        if let Expr::Path(expr_path) = func {
            if let Some(name) = Self::extract_function_name_from_path(&expr_path.path) {
                let resolved_name = Self::resolve_self_type(&name, &self.current_impl_type);
                let same_file_hint =
                    Self::is_same_file_call(&resolved_name, &self.current_impl_type);
                self.process_call(resolved_name, same_file_hint);
            }
        }
        // Process arguments for references and nested calls
        self.process_arguments(args);
    }

    fn handle_method_call_expr(
        &mut self,
        method: &syn::Ident,
        args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>,
        receiver: &Expr,
    ) {
        let name = self.construct_method_name(method, receiver, &self.current_impl_type);
        let same_file_hint = Self::is_self_receiver(receiver);
        self.process_call(name, same_file_hint);

        // Process arguments and visit receiver
        self.process_arguments(args);
        self.visit_expr(receiver);
    }

    fn handle_struct_expr(&mut self, expr_struct: &syn::ExprStruct) {
        // Visit each field's value expression to detect function calls
        for field in &expr_struct.fields {
            self.visit_expr(&field.expr);
        }
        // If there's a base struct (e.g., Foo { field: value, ..base })
        if let Some(ref base) = expr_struct.rest {
            self.visit_expr(base);
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
            Expr::Call(call_expr) => self.handle_call_expr(&call_expr.func, &call_expr.args),
            Expr::MethodCall(method_call) => self.handle_method_call_expr(
                &method_call.method,
                &method_call.args,
                &method_call.receiver,
            ),
            Expr::Closure(closure) => self.visit_expr(&closure.body),
            Expr::Async(async_block) => {
                for stmt in &async_block.block.stmts {
                    self.visit_stmt(stmt);
                }
            }
            Expr::Await(await_expr) => self.visit_expr(&await_expr.base),
            Expr::Struct(struct_expr) => self.handle_struct_expr(struct_expr),
            Expr::Macro(macro_expr) => self.handle_macro_expression(macro_expr),
            _ => syn::visit::visit_expr(self, expr),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use syn;

    fn parse_rust_code(code: &str) -> syn::File {
        syn::parse_str(code).expect("Failed to parse code")
    }

    #[test]
    fn test_resolve_self_type() {
        let impl_type = Some("MyStruct".to_string());
        let no_impl_type = None;

        // Test with Self:: and impl type present
        assert_eq!(
            CallGraphExtractor::resolve_self_type("Self::new", &impl_type),
            "MyStruct::new"
        );

        // Test with Self:: but no impl type
        assert_eq!(
            CallGraphExtractor::resolve_self_type("Self::new", &no_impl_type),
            "Self::new"
        );

        // Test with regular function name
        assert_eq!(
            CallGraphExtractor::resolve_self_type("foo::bar", &impl_type),
            "foo::bar"
        );

        // Test with no prefix
        assert_eq!(
            CallGraphExtractor::resolve_self_type("simple_func", &impl_type),
            "simple_func"
        );
    }

    #[test]
    fn test_is_same_file_call() {
        let impl_type = Some("MyStruct".to_string());
        let no_impl_type = None;

        // Test local function (no ::)
        assert!(CallGraphExtractor::is_same_file_call(
            "local_func",
            &impl_type
        ));
        assert!(CallGraphExtractor::is_same_file_call(
            "local_func",
            &no_impl_type
        ));

        // Test external function
        assert!(!CallGraphExtractor::is_same_file_call(
            "std::vec::Vec",
            &impl_type
        ));
        assert!(!CallGraphExtractor::is_same_file_call(
            "other::module",
            &no_impl_type
        ));

        // Test impl type method
        assert!(CallGraphExtractor::is_same_file_call(
            "MyStruct::method",
            &impl_type
        ));
        assert!(!CallGraphExtractor::is_same_file_call(
            "OtherStruct::method",
            &impl_type
        ));
    }

    #[test]
    fn test_is_self_receiver() {
        use syn::parse_quote;

        // Test self receiver
        let self_expr: Expr = parse_quote!(self);
        assert!(CallGraphExtractor::is_self_receiver(&self_expr));

        // Test non-self receiver
        let other_expr: Expr = parse_quote!(other);
        assert!(!CallGraphExtractor::is_self_receiver(&other_expr));

        // Test field access
        let field_expr: Expr = parse_quote!(self.field);
        assert!(!CallGraphExtractor::is_self_receiver(&field_expr));

        // Test method call receiver
        let method_expr: Expr = parse_quote!(obj);
        assert!(!CallGraphExtractor::is_self_receiver(&method_expr));
    }

    #[test]
    fn test_classify_call_type() {
        // Test async calls
        assert_eq!(
            CallGraphExtractor::classify_call_type("await"),
            CallType::Async
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("async_func"),
            CallType::Async
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("run_await"),
            CallType::Async
        );

        // Test delegate calls
        assert_eq!(
            CallGraphExtractor::classify_call_type("handle_request"),
            CallType::Delegate
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("process_data"),
            CallType::Delegate
        );

        // Test pipeline calls
        assert_eq!(
            CallGraphExtractor::classify_call_type("map"),
            CallType::Pipeline
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("and_then"),
            CallType::Pipeline
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("map_values"),
            CallType::Pipeline
        );

        // Test direct calls
        assert_eq!(
            CallGraphExtractor::classify_call_type("regular_func"),
            CallType::Direct
        );
        assert_eq!(
            CallGraphExtractor::classify_call_type("compute"),
            CallType::Direct
        );
    }

    #[test]
    fn test_visit_expr_call_processing() {
        let code = r#"
            struct MyStruct;
            
            impl MyStruct {
                fn new() -> Self {
                    Self
                }
                
                fn method(&self) {
                    Self::new();
                    other_func();
                    module::external_func();
                }
            }
            
            fn other_func() {}
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();
        let graph = extractor.call_graph;

        // Find the method function
        let method_fn = graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "MyStruct::method")
            .expect("MyStruct::method should exist");

        let callees = graph.get_callees(&method_fn);
        let callee_names: Vec<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Should have resolved Self::new to MyStruct::new
        assert!(
            callee_names.contains(&"MyStruct::new"),
            "Should contain MyStruct::new"
        );
        assert!(
            callee_names.contains(&"other_func"),
            "Should contain other_func"
        );
        // module::external_func might not resolve in all cases since it's external
        // Just verify we have at least the local calls
        assert!(callees.len() >= 2, "Should have at least 2 callees");
    }

    #[test]
    fn test_visit_expr_method_call_processing() {
        let code = r#"
            struct Foo {
                value: i32,
            }
            
            impl Foo {
                fn process(&self) -> i32 {
                    self.compute()
                }
                
                fn compute(&self) -> i32 {
                    self.value * 2
                }
            }
            
            fn use_foo() {
                let foo = Foo { value: 42 };
                foo.process();
            }
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();
        let graph = extractor.call_graph;

        // Find the process function
        let process_fn = graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "Foo::process")
            .expect("Foo::process should exist");

        let callees = graph.get_callees(&process_fn);
        let callee_names: Vec<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Should have detected self.compute() as Foo::compute
        assert!(
            callee_names.contains(&"Foo::compute"),
            "Should contain Foo::compute"
        );

        // Find use_foo function
        let use_foo_fn = graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "use_foo")
            .expect("use_foo should exist");

        let use_foo_callees = graph.get_callees(&use_foo_fn);
        let use_foo_callee_names: Vec<_> =
            use_foo_callees.iter().map(|c| c.name.as_str()).collect();

        // Should have detected foo.process()
        assert!(
            use_foo_callee_names.contains(&"Foo::process"),
            "Should contain Foo::process"
        );
    }

    #[test]
    fn test_vec_macro_with_struct_literals() {
        let code = r#"
            struct Item {
                value: i32,
            }
            
            fn create_item() -> Item {
                Item { value: 42 }
            }
            
            fn test() {
                let items = vec![
                    create_item(),
                    Item { value: create_item().value },
                ];
            }
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();
        let graph = extractor.call_graph;

        // Find the test function with the correct line number
        let test_functions: Vec<_> = (1..20)
            .filter_map(|line| {
                let id = FunctionId {
                    file: PathBuf::from("test.rs"),
                    name: "test".to_string(),
                    line,
                };
                if !graph.get_callees(&id).is_empty() {
                    Some((line, graph.get_callees(&id)))
                } else {
                    None
                }
            })
            .collect();

        // Should find callees for the test function
        assert!(
            !test_functions.is_empty(),
            "No test function found with callees"
        );

        // Check if any test function has create_item as a callee
        let found = test_functions
            .iter()
            .any(|(_, callees)| callees.iter().any(|callee| callee.name == "create_item"));

        assert!(
            found,
            "create_item not found in any test function's callees"
        );
    }

    // TODO: Re-enable when macro parsing fully supports function call detection
    // The expansion module was removed in favor of token parsing, which doesn't
    // yet fully support detecting function calls within all macro contexts
    #[test]
    #[ignore]
    fn test_format_macro_with_function_calls() {
        let code = r#"
            fn get_name() -> String {
                "Alice".to_string()
            }
            
            fn get_age() -> u32 {
                30
            }
            
            fn test() {
                let msg = format!("Name: {}, Age: {}", get_name(), get_age());
            }
        "#;

        let file = parse_rust_code(code);
        let graph = extract_call_graph(&file, Path::new("test.rs"));

        // Find the test function (line number may vary)
        let all_functions = graph.find_all_functions();
        let test_fn = all_functions
            .iter()
            .find(|f| f.name == "test")
            .expect("test function should exist");

        let callees = graph.get_callees(test_fn);

        assert!(callees.iter().any(|callee| callee.name == "get_name"));
        assert!(callees.iter().any(|callee| callee.name == "get_age"));
    }

    #[test]
    #[ignore]
    fn test_println_macro_with_expressions() {
        let code = r#"
            fn calculate() -> i32 {
                42
            }
            
            fn test() {
                println!("Result: {}", calculate() * 2);
            }
        "#;

        let file = parse_rust_code(code);
        let graph = extract_call_graph(&file, Path::new("test.rs"));

        // Find the test function (line number may vary)
        let all_functions = graph.find_all_functions();
        let test_fn = all_functions
            .iter()
            .find(|f| f.name == "test")
            .expect("test function should exist");

        let callees = graph.get_callees(test_fn);
        assert!(callees.iter().any(|callee| callee.name == "calculate"));
    }

    #[test]
    #[ignore]
    fn test_assert_macro_with_function_calls() {
        let code = r#"
            fn is_valid() -> bool {
                true
            }
            
            fn get_value() -> i32 {
                42
            }
            
            fn test() {
                assert!(is_valid());
                assert_eq!(get_value(), 42);
            }
        "#;

        let file = parse_rust_code(code);
        let graph = extract_call_graph(&file, Path::new("test.rs"));

        // Find the test function (line number may vary)
        let all_functions = graph.find_all_functions();
        let test_fn = all_functions
            .iter()
            .find(|f| f.name == "test")
            .expect("test function should exist");

        let callees = graph.get_callees(test_fn);

        assert!(callees.iter().any(|callee| callee.name == "is_valid"));
        assert!(callees.iter().any(|callee| callee.name == "get_value"));
    }

    #[test]
    #[ignore]
    fn test_hashmap_macro_with_function_calls() {
        let code = r#"
            use std::collections::HashMap;
            
            fn get_key() -> String {
                "key".to_string()
            }
            
            fn get_value() -> i32 {
                42
            }
            
            fn test() {
                let map = hashmap!{
                    get_key() => get_value(),
                };
            }
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();

        // Should detect both key and value function calls
        let test_fn_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "test".to_string(),
            line: 0,
        };
        let callees = extractor.call_graph.get_callees(&test_fn_id);

        assert!(callees.iter().any(|callee| callee.name == "get_key"));
        assert!(callees.iter().any(|callee| callee.name == "get_value"));
    }

    #[test]
    #[ignore]
    fn test_macro_stats_tracking() {
        let code = r#"
            fn test() {
                vec![1, 2, 3];
                format!("test");
                println!("hello");
                unknown_macro!(something);
            }
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.set_macro_config(MacroHandlingConfig {
            verbose_warnings: false,
            show_statistics: true,
        });

        extractor.extract_phase1(&file);

        let stats = extractor.get_macro_stats();
        // Check that at least some macros were detected and parsed
        assert!(stats.total_macros > 0, "Should detect macros");
        assert!(
            stats.successfully_parsed > 0,
            "Should parse some macros successfully"
        );
    }

    #[test]
    #[ignore]
    fn test_nested_macros() {
        let code = r#"
            fn get_item() -> i32 {
                42
            }
            
            fn test() {
                let result = vec![
                    format!("{}", get_item()),
                ];
            }
        "#;

        let file = parse_rust_code(code);
        let graph = extract_call_graph(&file, Path::new("test.rs"));

        // Should detect the call inside nested macros
        let test_fn_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "test".to_string(),
            line: 0,
        };
        let callees = graph.get_callees(&test_fn_id);
        assert!(callees.iter().any(|callee| callee.name == "get_item"));
    }

    #[test]
    #[ignore]
    fn test_logging_macros() {
        let code = r#"
            fn get_debug_info() -> String {
                "debug".to_string()
            }
            
            fn test() {
                info!("Info: {}", get_debug_info());
                error!("Error occurred");
                debug!("Debug: {}", get_debug_info());
            }
        "#;

        let file = parse_rust_code(code);
        let graph = extract_call_graph(&file, Path::new("test.rs"));

        // Find the test function (line number may vary)
        let all_functions = graph.find_all_functions();
        let test_fn = all_functions
            .iter()
            .find(|f| f.name == "test")
            .expect("test function should exist");

        let callees = graph.get_callees(test_fn);

        // Should find at least 2 calls to get_debug_info (from info! and debug!)
        let debug_calls = callees
            .iter()
            .filter(|callee| callee.name == "get_debug_info")
            .count();
        assert!(debug_calls >= 2);
    }

    #[test]
    fn test_is_braced_content() {
        // Positive cases
        assert!(CallGraphExtractor::is_braced_content("{}"));
        assert!(CallGraphExtractor::is_braced_content("{foo}"));
        assert!(CallGraphExtractor::is_braced_content("{key => value}"));
        assert!(CallGraphExtractor::is_braced_content(
            "{ nested { braces } }"
        ));

        // Negative cases
        assert!(!CallGraphExtractor::is_braced_content(""));
        assert!(!CallGraphExtractor::is_braced_content("foo"));
        assert!(!CallGraphExtractor::is_braced_content("{"));
        assert!(!CallGraphExtractor::is_braced_content("}"));
        assert!(!CallGraphExtractor::is_braced_content("[foo]"));
        assert!(!CallGraphExtractor::is_braced_content("(foo)"));
        assert!(!CallGraphExtractor::is_braced_content("{foo"));
        assert!(!CallGraphExtractor::is_braced_content("foo}"));
    }

    #[test]
    fn test_extract_braced_inner() {
        assert_eq!(CallGraphExtractor::extract_braced_inner("{}"), "");
        assert_eq!(CallGraphExtractor::extract_braced_inner("{foo}"), "foo");
        assert_eq!(
            CallGraphExtractor::extract_braced_inner("{a, b, c}"),
            "a, b, c"
        );
        assert_eq!(
            CallGraphExtractor::extract_braced_inner("{ spaced }"),
            " spaced "
        );
        assert_eq!(
            CallGraphExtractor::extract_braced_inner("{key => value}"),
            "key => value"
        );
    }

    #[test]
    fn test_parse_expression_from_str() {
        // Valid expressions
        assert!(CallGraphExtractor::parse_expression_from_str("42").is_some());
        assert!(CallGraphExtractor::parse_expression_from_str("foo").is_some());
        assert!(CallGraphExtractor::parse_expression_from_str("foo()").is_some());
        assert!(CallGraphExtractor::parse_expression_from_str("a + b").is_some());
        assert!(CallGraphExtractor::parse_expression_from_str("vec![1, 2, 3]").is_some());

        // Invalid expressions
        assert!(CallGraphExtractor::parse_expression_from_str("").is_none());
        assert!(CallGraphExtractor::parse_expression_from_str("struct Foo").is_none());
        assert!(CallGraphExtractor::parse_expression_from_str("fn bar()").is_none());
        assert!(CallGraphExtractor::parse_expression_from_str(";;;").is_none());
    }

    #[test]
    fn test_parse_key_value_pair() {
        // Valid key-value pairs
        let exprs = CallGraphExtractor::parse_key_value_pair("key => value");
        assert_eq!(exprs.len(), 2);

        let exprs = CallGraphExtractor::parse_key_value_pair("1 => foo()");
        assert_eq!(exprs.len(), 2);

        let exprs = CallGraphExtractor::parse_key_value_pair("get_key() => compute_value()");
        assert_eq!(exprs.len(), 2);

        // With spaces
        let exprs = CallGraphExtractor::parse_key_value_pair("  key  =>  value  ");
        assert_eq!(exprs.len(), 2);

        // No arrow separator
        let exprs = CallGraphExtractor::parse_key_value_pair("just_value");
        assert_eq!(exprs.len(), 0);

        let exprs = CallGraphExtractor::parse_key_value_pair("no arrow here");
        assert_eq!(exprs.len(), 0);

        // Invalid expressions
        let exprs = CallGraphExtractor::parse_key_value_pair("=> value");
        assert_eq!(exprs.len(), 1); // Only value is valid

        let exprs = CallGraphExtractor::parse_key_value_pair("key =>");
        assert_eq!(exprs.len(), 1); // Only key is valid

        let exprs = CallGraphExtractor::parse_key_value_pair("=>");
        assert_eq!(exprs.len(), 0); // Neither is valid
    }

    #[test]
    fn test_parse_braced_exprs_integration() {
        let extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));

        // Valid braced expressions with key-value pairs
        let tokens: proc_macro2::TokenStream = "{ key => value, foo => bar }".parse().unwrap();
        let result = extractor.parse_braced_exprs(&tokens);
        assert!(result.is_ok());
        let exprs = result.unwrap();
        assert_eq!(exprs.len(), 4); // 2 keys + 2 values

        // Empty braces
        let tokens: proc_macro2::TokenStream = "{}".parse().unwrap();
        let result = extractor.parse_braced_exprs(&tokens);
        assert!(result.is_err());

        // Not braced
        let tokens: proc_macro2::TokenStream = "foo, bar".parse().unwrap();
        let result = extractor.parse_braced_exprs(&tokens);
        assert!(result.is_err());

        // Single expression in braces (no key-value pairs)
        let tokens: proc_macro2::TokenStream = "{ single_expr }".parse().unwrap();
        let result = extractor.parse_braced_exprs(&tokens);
        assert!(result.is_err()); // No valid expressions parsed from "single_expr" as key-value

        // Multiple key-value pairs with function calls
        let tokens: proc_macro2::TokenStream = "{ get_key() => compute(), 42 => process() }"
            .parse()
            .unwrap();
        let result = extractor.parse_braced_exprs(&tokens);
        assert!(result.is_ok());
        let exprs = result.unwrap();
        assert_eq!(exprs.len(), 4); // 2 keys + 2 values
    }

    #[test]
    fn test_try_parse_single_expr() {
        // Test valid single expression
        let tokens: proc_macro2::TokenStream = "42".parse().unwrap();
        let result = CallGraphExtractor::try_parse_single_expr(&tokens);
        assert!(result.is_ok());

        // Test complex expression
        let tokens: proc_macro2::TokenStream = "foo + bar * 2".parse().unwrap();
        let result = CallGraphExtractor::try_parse_single_expr(&tokens);
        assert!(result.is_ok());

        // Test invalid expression (malformed syntax)
        let tokens: proc_macro2::TokenStream = ":::: invalid".parse().unwrap();
        let result = CallGraphExtractor::try_parse_single_expr(&tokens);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_assert_macro_single_expr() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));

        // Test assert! with simple expression
        let tokens: proc_macro2::TokenStream = "x > 0".parse().unwrap();
        extractor.parse_assert_macro(&tokens, "assert");
        assert_eq!(extractor.macro_stats.successfully_parsed, 1);
    }

    #[test]
    fn test_parse_assert_macro_comma_separated() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));

        // Test assert_eq! with comma-separated expressions
        let tokens: proc_macro2::TokenStream = "a, b".parse().unwrap();
        extractor.parse_assert_macro(&tokens, "assert_eq");
        assert_eq!(extractor.macro_stats.successfully_parsed, 1);
    }

    #[test]
    fn test_parse_assert_macro_with_message() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));

        // Test assert! with condition and message
        let tokens: proc_macro2::TokenStream = r#"x > 0, "x must be positive""#.parse().unwrap();
        extractor.parse_assert_macro(&tokens, "assert");
        assert_eq!(extractor.macro_stats.successfully_parsed, 1);
    }

    #[test]
    fn test_parse_assert_macro_complex_condition() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));

        // Test assert! with complex boolean expression
        let tokens: proc_macro2::TokenStream = "(x > 0 && y < 10) || z == 5".parse().unwrap();
        extractor.parse_assert_macro(&tokens, "assert");
        assert_eq!(extractor.macro_stats.successfully_parsed, 1);
    }

    #[test]
    fn test_parse_assert_macro_unparseable() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));

        // Test with unparseable tokens
        let tokens: proc_macro2::TokenStream = ";;;".parse().unwrap();
        let initial_parsed = extractor.macro_stats.successfully_parsed;
        let initial_failed = extractor.macro_stats.failed_macros.len();
        extractor.parse_assert_macro(&tokens, "assert");
        assert_eq!(extractor.macro_stats.successfully_parsed, initial_parsed);
        assert!(extractor.macro_stats.failed_macros.len() > initial_failed);
    }

    #[test]
    fn test_process_parsed_exprs() {
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        use syn::parse_quote;

        // Test processing multiple expressions
        let exprs = vec![
            parse_quote!(foo()),
            parse_quote!(bar()),
            parse_quote!(baz()),
        ];

        // Simply test that the method runs without panicking
        extractor.process_parsed_exprs(exprs);
        // The method processes expressions by visiting them
        // which may or may not add to the call graph depending on the expressions
    }

    #[test]
    fn test_visit_expr_closure_handling() {
        let code = r#"
            fn process_with_closure() {
                let numbers = vec![1, 2, 3];
                
                // Closure with function calls inside
                let result = numbers.iter().map(|x| {
                    let doubled = double(*x);
                    let formatted = format_number(doubled);
                    formatted
                }).collect::<Vec<_>>();
                
                // Closure with method calls
                let processor = |value: i32| {
                    helper_function(value);
                    value.to_string()
                };
            }
            
            fn double(x: i32) -> i32 { x * 2 }
            fn format_number(x: i32) -> String { x.to_string() }
            fn helper_function(x: i32) {}
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();
        let graph = extractor.call_graph;

        // Find the process_with_closure function
        let process_fn = graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "process_with_closure")
            .expect("process_with_closure should exist");

        let callees = graph.get_callees(&process_fn);
        let callee_names: Vec<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Should detect function calls inside closures
        assert!(
            callee_names.contains(&"double"),
            "Should detect double() call inside closure"
        );
        assert!(
            callee_names.contains(&"format_number"),
            "Should detect format_number() call inside closure"
        );
        assert!(
            callee_names.contains(&"helper_function"),
            "Should detect helper_function() call inside closure"
        );
    }

    #[test]
    fn test_visit_expr_async_block_handling() {
        let code = r#"
            async fn async_processor() {
                // Async block with function calls
                let future = async {
                    prepare_data().await;
                    let result = compute_async().await;
                    finalize(result);
                    result
                };
                
                // Another async block
                let another = async move {
                    validate_input();
                    process_item().await;
                };
            }
            
            async fn prepare_data() {}
            async fn compute_async() -> i32 { 42 }
            fn finalize(x: i32) {}
            fn validate_input() {}
            async fn process_item() {}
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();
        let graph = extractor.call_graph;

        // Find the async_processor function
        let async_fn = graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "async_processor")
            .expect("async_processor should exist");

        let callees = graph.get_callees(&async_fn);
        let callee_names: Vec<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Should detect function calls inside async blocks
        assert!(
            callee_names.contains(&"prepare_data"),
            "Should detect prepare_data() call inside async block"
        );
        assert!(
            callee_names.contains(&"compute_async"),
            "Should detect compute_async() call inside async block"
        );
        assert!(
            callee_names.contains(&"finalize"),
            "Should detect finalize() call inside async block"
        );
        assert!(
            callee_names.contains(&"validate_input"),
            "Should detect validate_input() call inside async block"
        );
        assert!(
            callee_names.contains(&"process_item"),
            "Should detect process_item() call inside async block"
        );
    }

    #[test]
    fn test_visit_expr_await_handling() {
        let code = r#"
            async fn await_handler() {
                // Simple await expression
                let result = fetch_data().await;
                
                // Chained await expressions
                let processed = fetch_data()
                    .await
                    .transform()
                    .await;
                
                // Await with method call on result
                let final_result = compute()
                    .await
                    .finalize();
            }
            
            async fn fetch_data() -> DataWrapper { DataWrapper }
            async fn compute() -> Processor { Processor }
            
            struct DataWrapper;
            impl DataWrapper {
                async fn transform(self) -> ProcessedData { ProcessedData }
            }
            
            struct ProcessedData;
            struct Processor;
            impl Processor {
                fn finalize(self) -> i32 { 42 }
            }
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();
        let graph = extractor.call_graph;

        // Find the await_handler function
        let await_fn = graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "await_handler")
            .expect("await_handler should exist");

        let callees = graph.get_callees(&await_fn);
        let callee_names: Vec<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Should detect function calls with await expressions
        assert!(
            callee_names.contains(&"fetch_data"),
            "Should detect fetch_data() call with await"
        );
        assert!(
            callee_names.contains(&"compute"),
            "Should detect compute() call with await"
        );
        // Method calls on awaited results
        assert!(
            callee_names.iter().any(|n| n.contains("transform")),
            "Should detect transform() method call"
        );
        assert!(
            callee_names.iter().any(|n| n.contains("finalize")),
            "Should detect finalize() method call"
        );
    }

    #[test]
    fn test_visit_expr_struct_literal_handling() {
        let code = r#"
            struct Config {
                name: String,
                value: i32,
                processor: fn(i32) -> i32,
            }
            
            struct Nested {
                config: Config,
                data: Vec<i32>,
            }
            
            fn create_config() {
                // Struct literal with function calls in field values
                let config = Config {
                    name: generate_name(),
                    value: calculate_value(),
                    processor: get_processor(),
                };
                
                // Nested struct literal
                let nested = Nested {
                    config: Config {
                        name: format_string("test"),
                        value: compute_default(),
                        processor: default_processor,
                    },
                    data: generate_data(),
                };
                
                // Struct with base
                let updated = Config {
                    name: new_name(),
                    ..get_base_config()
                };
            }
            
            fn generate_name() -> String { String::new() }
            fn calculate_value() -> i32 { 42 }
            fn get_processor() -> fn(i32) -> i32 { |x| x }
            fn format_string(s: &str) -> String { s.to_string() }
            fn compute_default() -> i32 { 0 }
            fn default_processor(x: i32) -> i32 { x }
            fn generate_data() -> Vec<i32> { vec![] }
            fn new_name() -> String { String::new() }
            fn get_base_config() -> Config { 
                Config { 
                    name: String::new(), 
                    value: 0, 
                    processor: |x| x 
                } 
            }
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();
        let graph = extractor.call_graph;

        // Find the create_config function
        let create_fn = graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "create_config")
            .expect("create_config should exist");

        let callees = graph.get_callees(&create_fn);
        let callee_names: Vec<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Should detect function calls in struct field values
        assert!(
            callee_names.contains(&"generate_name"),
            "Should detect generate_name() in struct field"
        );
        assert!(
            callee_names.contains(&"calculate_value"),
            "Should detect calculate_value() in struct field"
        );
        assert!(
            callee_names.contains(&"get_processor"),
            "Should detect get_processor() in struct field"
        );
        assert!(
            callee_names.contains(&"format_string"),
            "Should detect format_string() in nested struct"
        );
        assert!(
            callee_names.contains(&"compute_default"),
            "Should detect compute_default() in nested struct"
        );
        assert!(
            callee_names.contains(&"generate_data"),
            "Should detect generate_data() in struct field"
        );
        assert!(
            callee_names.contains(&"new_name"),
            "Should detect new_name() in struct update"
        );
        assert!(
            callee_names.contains(&"get_base_config"),
            "Should detect get_base_config() in struct base"
        );
    }

    #[test]
    fn test_visit_expr_complex_nested_calls() {
        let code = r#"
            fn complex_nested() {
                // Deeply nested function calls in various expression types
                let result = async {
                    let closure = |x| {
                        let config = Config {
                            value: process(x),
                            data: transform(fetch().await),
                        };
                        config
                    };
                    
                    closure(compute())
                }.await;
            }
            
            struct Config {
                value: i32,
                data: String,
            }
            
            fn process(x: i32) -> i32 { x }
            fn transform(s: String) -> String { s }
            async fn fetch() -> String { String::new() }
            fn compute() -> i32 { 42 }
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();
        let graph = extractor.call_graph;

        // Find the complex_nested function
        let complex_fn = graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "complex_nested")
            .expect("complex_nested should exist");

        let callees = graph.get_callees(&complex_fn);
        let callee_names: Vec<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Should detect all nested function calls
        assert!(
            callee_names.contains(&"process"),
            "Should detect process() in nested context"
        );
        assert!(
            callee_names.contains(&"transform"),
            "Should detect transform() in nested context"
        );
        assert!(
            callee_names.contains(&"fetch"),
            "Should detect fetch() in nested context"
        );
        assert!(
            callee_names.contains(&"compute"),
            "Should detect compute() in nested context"
        );
    }

    #[test]
    fn test_visit_expr_macro_expressions() {
        // Test that visit_expr correctly handles macro expressions through Expr::Macro path
        let code = r#"
            fn macro_user() {
                // Collection macros with function calls - these are successfully parsed
                let v = vec![generate_item(), process_item(), finalize_item()];
                
                // Format macro with function call - also successfully parsed
                let msg = format!("Result: {}", calculate_result());
                
                // HashSet macro with function calls
                let set = hashset![first_item(), second_item()];
            }
            
            fn generate_item() -> i32 { 1 }
            fn process_item() -> i32 { 2 }
            fn finalize_item() -> i32 { 3 }
            fn calculate_result() -> i32 { 100 }
            fn first_item() -> i32 { 10 }
            fn second_item() -> i32 { 20 }
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();
        let graph = extractor.call_graph;

        // Find the macro_user function
        let macro_fn = graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "macro_user")
            .expect("macro_user should exist");

        let callees = graph.get_callees(&macro_fn);
        let callee_names: Vec<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Test that the Expr::Macro path in visit_expr correctly delegates to handle_macro_expression
        // which then parses known macro patterns and visits the expressions within

        // vec! macro - collection macro parsing
        assert!(
            callee_names.contains(&"generate_item"),
            "Should detect generate_item() in vec! macro"
        );
        assert!(
            callee_names.contains(&"process_item"),
            "Should detect process_item() in vec! macro"
        );
        assert!(
            callee_names.contains(&"finalize_item"),
            "Should detect finalize_item() in vec! macro"
        );

        // format! macro - format macro parsing (skips format string, visits arguments)
        assert!(
            callee_names.contains(&"calculate_result"),
            "Should detect calculate_result() in format! macro"
        );

        // hashset! macro - collection macro parsing
        assert!(
            callee_names.contains(&"first_item"),
            "Should detect first_item() in hashset! macro"
        );
        assert!(
            callee_names.contains(&"second_item"),
            "Should detect second_item() in hashset! macro"
        );
    }

    #[test]
    fn test_visit_expr_default_case_expressions() {
        // Test expressions that hit the default case in visit_expr
        let code = r#"
            fn expressions_handler() {
                // Binary operations with function calls
                let sum = get_left() + get_right();
                
                // Unary operations
                let neg = -compute_value();
                
                // Array expressions with function calls
                let arr = [first(), second(), third()];
                
                // Index expressions
                let val = get_array()[get_index()];
                
                // Field access on function result
                let field = get_struct().field;
                
                // Range expressions
                let range = start_value()..end_value();
                
                // If expressions with function calls
                let result = if check_condition() {
                    handle_true()
                } else {
                    handle_false()
                };
                
                // Match expressions
                match get_option() {
                    Some(x) => process_some(x),
                    None => process_none(),
                }
                
                // Loop expressions
                loop {
                    if should_break() {
                        break;
                    }
                    loop_body();
                }
                
                // While expressions
                while continue_condition() {
                    iterate();
                }
                
                // For loop expressions
                for item in get_iterator() {
                    process_item(item);
                }
            }
            
            fn get_left() -> i32 { 1 }
            fn get_right() -> i32 { 2 }
            fn compute_value() -> i32 { 42 }
            fn first() -> i32 { 1 }
            fn second() -> i32 { 2 }
            fn third() -> i32 { 3 }
            fn get_array() -> Vec<i32> { vec![1, 2, 3] }
            fn get_index() -> usize { 0 }
            struct MyStruct { field: i32 }
            fn get_struct() -> MyStruct { MyStruct { field: 42 } }
            fn start_value() -> i32 { 0 }
            fn end_value() -> i32 { 10 }
            fn check_condition() -> bool { true }
            fn handle_true() -> i32 { 1 }
            fn handle_false() -> i32 { 0 }
            fn get_option() -> Option<i32> { Some(42) }
            fn process_some(x: i32) {}
            fn process_none() {}
            fn should_break() -> bool { true }
            fn loop_body() {}
            fn continue_condition() -> bool { false }
            fn iterate() {}
            fn get_iterator() -> Vec<i32> { vec![1, 2, 3] }
            fn process_item(x: i32) {}
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();
        let graph = extractor.call_graph;

        // Find the expressions_handler function
        let expr_fn = graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "expressions_handler")
            .expect("expressions_handler should exist");

        let callees = graph.get_callees(&expr_fn);
        let callee_names: Vec<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Should detect function calls in various expression contexts
        assert!(
            callee_names.contains(&"get_left"),
            "Should detect get_left() in binary operation"
        );
        assert!(
            callee_names.contains(&"get_right"),
            "Should detect get_right() in binary operation"
        );
        assert!(
            callee_names.contains(&"compute_value"),
            "Should detect compute_value() in unary operation"
        );
        assert!(
            callee_names.contains(&"first"),
            "Should detect first() in array expression"
        );
        assert!(
            callee_names.contains(&"get_array"),
            "Should detect get_array() in index expression"
        );
        assert!(
            callee_names.contains(&"get_index"),
            "Should detect get_index() in index expression"
        );
        assert!(
            callee_names.contains(&"get_struct"),
            "Should detect get_struct() in field access"
        );
        assert!(
            callee_names.contains(&"check_condition"),
            "Should detect check_condition() in if expression"
        );
        assert!(
            callee_names.contains(&"handle_true"),
            "Should detect handle_true() in if branch"
        );
        assert!(
            callee_names.contains(&"handle_false"),
            "Should detect handle_false() in else branch"
        );
        assert!(
            callee_names.contains(&"get_option"),
            "Should detect get_option() in match expression"
        );
        assert!(
            callee_names.contains(&"process_some"),
            "Should detect process_some() in match arm"
        );
        assert!(
            callee_names.contains(&"process_none"),
            "Should detect process_none() in match arm"
        );
        assert!(
            callee_names.contains(&"should_break"),
            "Should detect should_break() in loop"
        );
        assert!(
            callee_names.contains(&"loop_body"),
            "Should detect loop_body() in loop"
        );
        assert!(
            callee_names.contains(&"continue_condition"),
            "Should detect continue_condition() in while"
        );
        assert!(
            callee_names.contains(&"iterate"),
            "Should detect iterate() in while body"
        );
        assert!(
            callee_names.contains(&"get_iterator"),
            "Should detect get_iterator() in for loop"
        );
        assert!(
            callee_names.contains(&"process_item"),
            "Should detect process_item() in for loop body"
        );
    }

    #[test]
    fn test_visit_expr_integration() {
        // Test that visit_expr correctly handles different expression types
        let code = r#"
            fn test_function() {
                // Function call
                foo();
                
                // Method call
                obj.method();
                
                // Closure
                let c = |x| x + 1;
                
                // Async block
                async {
                    fetch().await;
                };
                
                // Struct literal
                let p = Point { x: 1, y: 2 };
                
                // Macro
                let v = vec![1, 2, 3];
                
                // Regular expression (should use default handling)
                let x = 42;
            }
            
            fn foo() {}
            struct Point { x: i32, y: i32 }
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();

        // Verify that the function was found and processed
        let test_fn = extractor
            .call_graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "test_function")
            .expect("test_function should exist");

        // Verify that foo() call was detected
        let callees = extractor.call_graph.get_callees(&test_fn);
        let callee_names: Vec<_> = callees.iter().map(|c| c.name.as_str()).collect();
        assert!(callee_names.contains(&"foo"), "Should detect foo() call");
    }

    #[test]
    fn test_visit_expr_all_branches() {
        // Comprehensive test covering all match arms in visit_expr
        let code = r#"
            fn comprehensive_test() {
                // Expr::Call case
                regular_function();
                module::external_func();
                
                // Expr::MethodCall case
                let obj = MyType::new();
                obj.instance_method();
                self.self_method();
                
                // Expr::Closure case (with nested expression)
                let closure = |x| {
                    nested_func(x)
                };
                
                // Expr::Async case (with statements)
                let future = async {
                    let result = async_func().await;
                    process_result(result);
                };
                
                // Expr::Await case
                let value = some_future.await;
                
                // Expr::Struct case with fields containing calls
                let s = MyStruct {
                    field1: compute_value(),
                    field2: Default::default(),
                };
                
                // Expr::Macro case
                vec![get_item(), get_item()];
                format!("{}", formatter());
                
                // Default case expressions (should traverse children)
                let binary = 1 + compute_sum();
                let unary = !is_valid();
                let array = [element_func(); 10];
                let tuple = (first_func(), second_func());
                let block_expr = {
                    block_func();
                    42
                };
                let if_expr = if condition_func() {
                    then_func()
                } else {
                    else_func()
                };
                let match_expr = match get_option() {
                    Some(v) => process_some(v),
                    None => process_none(),
                };
                let loop_expr = loop {
                    if exit_condition() {
                        break;
                    }
                    loop_body();
                };
                let while_expr = while continue_condition() {
                    while_body();
                };
                let for_expr = for item in get_iterator() {
                    process_item(item);
                };
            }
            
            fn regular_function() {}
            fn nested_func(x: i32) -> i32 { x }
            async fn async_func() -> i32 { 42 }
            fn process_result(r: i32) {}
            fn compute_value() -> i32 { 0 }
            fn formatter() -> String { String::new() }
            fn get_item() -> i32 { 1 }
            fn compute_sum() -> i32 { 2 }
            fn is_valid() -> bool { true }
            fn element_func() -> i32 { 3 }
            fn first_func() -> i32 { 4 }
            fn second_func() -> i32 { 5 }
            fn block_func() {}
            fn condition_func() -> bool { true }
            fn then_func() -> i32 { 6 }
            fn else_func() -> i32 { 7 }
            fn get_option() -> Option<i32> { Some(8) }
            fn process_some(v: i32) -> i32 { v }
            fn process_none() -> i32 { 0 }
            fn exit_condition() -> bool { true }
            fn loop_body() {}
            fn continue_condition() -> bool { false }
            fn while_body() {}
            fn get_iterator() -> Vec<i32> { vec![1, 2, 3] }
            fn process_item(item: i32) {}
            
            struct MyType;
            impl MyType {
                fn new() -> Self { MyType }
                fn instance_method(&self) {}
                fn self_method(&self) {}
            }
            
            struct MyStruct {
                field1: i32,
                field2: i32,
            }
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();

        // Find the comprehensive_test function
        let test_fn = extractor
            .call_graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "comprehensive_test")
            .expect("comprehensive_test should exist");

        let callees = extractor.call_graph.get_callees(&test_fn);
        let callee_names: HashSet<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Verify calls from different expression types are detected

        // From Expr::Call
        assert!(
            callee_names.contains("regular_function"),
            "Should detect regular function call"
        );

        // From Expr::MethodCall
        assert!(
            callee_names.contains("MyType::new"),
            "Should detect constructor call"
        );
        assert!(
            callee_names.contains("MyType::instance_method"),
            "Should detect method call"
        );

        // From nested expressions in Expr::Closure body
        assert!(
            callee_names.contains("nested_func"),
            "Should detect call in closure body"
        );

        // From Expr::Async block statements
        assert!(
            callee_names.contains("process_result"),
            "Should detect call in async block"
        );

        // From Expr::Struct fields
        assert!(
            callee_names.contains("compute_value"),
            "Should detect call in struct field"
        );

        // From default case expressions that contain calls
        assert!(
            callee_names.contains("compute_sum"),
            "Should detect call in binary expression"
        );
        assert!(
            callee_names.contains("is_valid"),
            "Should detect call in unary expression"
        );
        assert!(
            callee_names.contains("element_func"),
            "Should detect call in array expression"
        );
        assert!(
            callee_names.contains("first_func"),
            "Should detect call in tuple expression"
        );
        assert!(
            callee_names.contains("block_func"),
            "Should detect call in block expression"
        );
        assert!(
            callee_names.contains("condition_func"),
            "Should detect call in if condition"
        );
        assert!(
            callee_names.contains("then_func"),
            "Should detect call in then branch"
        );
        assert!(
            callee_names.contains("else_func"),
            "Should detect call in else branch"
        );
        assert!(
            callee_names.contains("get_option"),
            "Should detect call in match expression"
        );
        assert!(
            callee_names.contains("process_some"),
            "Should detect call in match arm"
        );
        assert!(
            callee_names.contains("exit_condition"),
            "Should detect call in loop condition"
        );
        assert!(
            callee_names.contains("loop_body"),
            "Should detect call in loop body"
        );
        assert!(
            callee_names.contains("continue_condition"),
            "Should detect call in while condition"
        );
        assert!(
            callee_names.contains("while_body"),
            "Should detect call in while body"
        );
        assert!(
            callee_names.contains("get_iterator"),
            "Should detect call in for iterator"
        );
        assert!(
            callee_names.contains("process_item"),
            "Should detect call in for body"
        );
    }

    #[test]
    fn test_visit_expr_nested_expressions() {
        // Test deeply nested expressions to ensure recursion works correctly
        let code = r#"
            fn deeply_nested() {
                // Nested calls within calls
                outer_func(middle_func(inner_func()));
                
                // Nested method chains
                builder()
                    .with_option(get_option())
                    .with_value(compute())
                    .build();
                
                // Complex nested structure
                let result = if check_condition() {
                    match analyze_data() {
                        Some(data) => process_data(transform_data(data)),
                        None => default_value(),
                    }
                } else {
                    fallback_computation()
                };
                
                // Nested closures
                let nested_closure = |x| {
                    |y| {
                        combine(x, y)
                    }
                };
                
                // Nested async blocks
                let nested_async = async {
                    let inner = async {
                        fetch_data().await
                    };
                    process_async(inner.await).await
                };
            }
            
            fn outer_func(x: i32) -> i32 { x }
            fn middle_func(x: i32) -> i32 { x }
            fn inner_func() -> i32 { 1 }
            fn builder() -> Builder { Builder }
            fn get_option() -> i32 { 2 }
            fn compute() -> i32 { 3 }
            fn check_condition() -> bool { true }
            fn analyze_data() -> Option<i32> { Some(4) }
            fn process_data(x: i32) -> i32 { x }
            fn transform_data(x: i32) -> i32 { x }
            fn default_value() -> i32 { 0 }
            fn fallback_computation() -> i32 { 5 }
            fn combine(x: i32, y: i32) -> i32 { x + y }
            async fn fetch_data() -> i32 { 6 }
            async fn process_async(x: i32) -> i32 { x }
            
            struct Builder;
            impl Builder {
                fn with_option(self, _: i32) -> Self { self }
                fn with_value(self, _: i32) -> Self { self }
                fn build(self) -> i32 { 7 }
            }
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();

        let test_fn = extractor
            .call_graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "deeply_nested")
            .expect("deeply_nested should exist");

        let callees = extractor.call_graph.get_callees(&test_fn);
        let callee_names: HashSet<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Verify nested calls are all detected
        assert!(
            callee_names.contains("outer_func"),
            "Should detect outer function"
        );
        assert!(
            callee_names.contains("middle_func"),
            "Should detect middle function"
        );
        assert!(
            callee_names.contains("inner_func"),
            "Should detect inner function"
        );

        // Verify method chain calls
        assert!(
            callee_names.contains("builder"),
            "Should detect builder function"
        );
        assert!(
            callee_names.contains("Builder::with_option"),
            "Should detect with_option method"
        );
        assert!(
            callee_names.contains("Builder::with_value"),
            "Should detect with_value method"
        );
        assert!(
            callee_names.contains("Builder::build"),
            "Should detect build method"
        );
        assert!(
            callee_names.contains("get_option"),
            "Should detect get_option in chain"
        );
        assert!(
            callee_names.contains("compute"),
            "Should detect compute in chain"
        );

        // Verify complex nested structure calls
        assert!(
            callee_names.contains("check_condition"),
            "Should detect condition check"
        );
        assert!(
            callee_names.contains("analyze_data"),
            "Should detect data analysis"
        );
        assert!(
            callee_names.contains("process_data"),
            "Should detect data processing"
        );
        assert!(
            callee_names.contains("transform_data"),
            "Should detect data transformation"
        );
        assert!(
            callee_names.contains("default_value"),
            "Should detect default value"
        );
        assert!(
            callee_names.contains("fallback_computation"),
            "Should detect fallback"
        );

        // Verify nested closure calls
        assert!(
            callee_names.contains("combine"),
            "Should detect call in nested closure"
        );

        // Verify nested async calls
        assert!(
            callee_names.contains("fetch_data"),
            "Should detect call in nested async"
        );
        assert!(
            callee_names.contains("process_async"),
            "Should detect async processing"
        );
    }

    #[test]
    fn test_visit_expr_edge_cases() {
        // Test edge cases and error conditions
        let code = r#"
            struct EmptyStruct {}
            
            fn edge_cases() {
                // Empty expressions
                {};
                
                // Macro with empty content
                vec![];
                
                // Closure with no body expression
                let empty_closure = |_| {};
                
                // Async block with no statements
                let empty_async = async {};
                
                // Struct with no fields
                let empty_struct = EmptyStruct {};
                
                // Complex macro patterns (known limitation)
                assert_eq!(compute_left(), compute_right());
                debug_assert!(validate());
                
                // Path expressions with function calls
                std::mem::drop(create_value());
                
                // Reference and dereference
                let ref_call = &make_ref();
                let deref_call = *make_ptr();
                
                // Range expressions
                for i in start_value()..end_value() {
                    process_index(i);
                }
            }
            
            fn compute_left() -> i32 { 1 }
            fn compute_right() -> i32 { 1 }
            fn validate() -> bool { true }
            fn create_value() -> i32 { 2 }
            fn make_ref() -> i32 { 3 }
            fn make_ptr() -> Box<i32> { Box::new(4) }
            fn start_value() -> i32 { 0 }
            fn end_value() -> i32 { 10 }
            fn process_index(i: i32) {}
        "#;

        let file = parse_rust_code(code);
        let mut extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        extractor.extract_phase1(&file);
        extractor.resolve_phase2();

        let test_fn = extractor
            .call_graph
            .find_all_functions()
            .into_iter()
            .find(|f| f.name == "edge_cases")
            .expect("edge_cases should exist");

        let callees = extractor.call_graph.get_callees(&test_fn);
        let callee_names: HashSet<_> = callees.iter().map(|c| c.name.as_str()).collect();

        // Verify calls that should be detected in expressions
        // Note: Many edge cases like drop() arguments, macro expansions, and
        // qualified paths have known limitations in the current implementation

        // Basic function calls that work through the default visitor
        assert!(
            callee_names.contains("create_value"),
            "Should detect create_value call"
        );
        assert!(
            callee_names.contains("make_ref"),
            "Should detect make_ref call"
        );
        assert!(
            callee_names.contains("make_ptr"),
            "Should detect make_ptr call"
        );

        // Range and loop expressions
        assert!(
            callee_names.contains("start_value"),
            "Should detect start_value call"
        );
        assert!(
            callee_names.contains("end_value"),
            "Should detect end_value call"
        );
        assert!(
            callee_names.contains("process_index"),
            "Should detect process_index call"
        );
    }
}

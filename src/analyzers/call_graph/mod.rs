pub mod call_resolution;
pub mod debug;
pub mod graph_builder;
pub mod import_map;
/// Call graph extraction module providing two-pass resolution for accurate call tracking
///
/// This module is organized into focused submodules:
/// - `macro_expansion`: Handles macro parsing and expansion
/// - `call_resolution`: Resolves function calls to their definitions
/// - `graph_builder`: Builds and manages the call graph structure
/// - `trait_handling`: Handles trait resolution and method dispatch
/// - `import_map`: Tracks imports and re-exports for resolution
/// - `module_tree`: Maintains module hierarchy for path resolution
/// - `path_resolver`: Combines imports and hierarchy for full resolution
/// - `debug`: Debug and diagnostic tools for call resolution
/// - `validation`: Call graph validation and health checks
pub mod macro_expansion;
pub mod module_tree;
pub mod path_resolver;
pub mod trait_handling;
pub mod validation;

// Re-export main types for backward compatibility
pub use call_resolution::{CallResolver, UnresolvedCall};
pub use debug::{
    CallGraphDebugger, DebugConfig, DebugFormat, ResolutionAttempt, ResolutionStatistics,
    StrategyAttempt,
};
pub use graph_builder::{ExprCategory, GraphBuilder};
pub use import_map::ImportMap;
pub use macro_expansion::{MacroExpander, MacroExpansionStats, MacroHandlingConfig};
pub use module_tree::ModuleTree;
pub use path_resolver::{PathResolver, PathResolverBuilder};
pub use trait_handling::TraitHandler;
pub use validation::{CallGraphValidator, ValidationReport};

use crate::analyzers::function_registry::FunctionSignatureRegistry;
use crate::analyzers::type_registry::GlobalTypeRegistry;
use crate::analyzers::type_tracker::{ScopeKind, TypeTracker};
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::PathBuf;
use std::sync::Arc;
use syn::visit::Visit;
use syn::{Expr, ExprMacro, ImplItemFn, ItemFn, Local, Pat};

/// Main call graph extractor that coordinates all submodules
pub struct CallGraphExtractor {
    pub call_graph: CallGraph,
    pub unresolved_calls: Vec<UnresolvedCall>,
    current_function: Option<FunctionId>,
    current_impl_type: Option<String>,
    current_file: PathBuf,
    module_path: Vec<String>,
    /// Module path string for this file (e.g., "builders::call_graph")
    file_module_path: String,
    /// Type tracker for accurate method resolution
    type_tracker: TypeTracker,
    /// Global type registry (optional)
    #[allow(dead_code)]
    type_registry: Option<Arc<GlobalTypeRegistry>>,
    /// Function signature registry for return type resolution
    function_registry: Option<Arc<FunctionSignatureRegistry>>,
    /// Macro expander
    macro_expander: MacroExpander,
    /// Graph builder
    pub graph_builder: GraphBuilder,
}

impl CallGraphExtractor {
    pub fn new(file: PathBuf) -> Self {
        let graph_builder = GraphBuilder::new(file.clone());
        let file_module_path = ModuleTree::infer_module_from_file(&file);

        Self {
            call_graph: graph_builder.call_graph.clone(),
            unresolved_calls: Vec::new(),
            current_function: None,
            current_impl_type: None,
            current_file: file.clone(),
            module_path: Vec::new(),
            file_module_path,
            type_tracker: TypeTracker::new(),
            type_registry: None,
            function_registry: None,
            macro_expander: MacroExpander::new(),
            graph_builder,
        }
    }

    /// Create a new extractor with a shared type registry
    pub fn with_registry(file: PathBuf, registry: Arc<GlobalTypeRegistry>) -> Self {
        let mut tracker = TypeTracker::with_registry(registry.clone());
        tracker.set_current_file(file.clone());
        let graph_builder = GraphBuilder::new(file.clone());
        let file_module_path = ModuleTree::infer_module_from_file(&file);

        Self {
            call_graph: graph_builder.call_graph.clone(),
            unresolved_calls: Vec::new(),
            current_function: None,
            current_impl_type: None,
            current_file: file,
            module_path: Vec::new(),
            file_module_path,
            type_tracker: tracker,
            type_registry: Some(registry),
            function_registry: None,
            macro_expander: MacroExpander::new(),
            graph_builder,
        }
    }

    /// Set the function signature registry
    pub fn set_function_registry(&mut self, registry: Arc<FunctionSignatureRegistry>) {
        self.function_registry = Some(registry);
    }

    /// Set macro handling configuration
    pub fn set_macro_config(&mut self, config: MacroHandlingConfig) {
        self.macro_expander.config = config;
    }

    /// Get macro expansion statistics
    pub fn get_macro_stats(&self) -> &MacroExpansionStats {
        &self.macro_expander.stats
    }

    /// Report macro expansion statistics
    pub fn report_macro_stats(&self) {
        self.macro_expander.report_macro_stats();
    }

    /// Main extraction method - performs two-phase extraction
    pub fn extract(mut self, file: &syn::File) -> CallGraph {
        // Phase 1: Extract functions and unresolved calls
        self.extract_phase1(file);

        // Merge graphs before phase 2 so functions are available for resolution
        self.call_graph.merge(self.graph_builder.call_graph.clone());

        // Phase 2: Resolve calls
        self.resolve_phase2();

        self.call_graph
    }

    /// Phase 1: Extract functions and collect unresolved calls
    pub fn extract_phase1(&mut self, file: &syn::File) {
        self.visit_file(file);
    }

    /// Phase 2: Resolve all collected calls
    fn resolve_phase2(&mut self) {
        let mut resolved_calls = Vec::new();

        {
            let resolver = CallResolver::new(&self.call_graph, &self.current_file);

            for unresolved in &self.unresolved_calls {
                if let Some(callee) = resolver.resolve_call(unresolved) {
                    resolved_calls.push(FunctionCall {
                        caller: unresolved.caller.clone(),
                        callee,
                        call_type: unresolved.call_type.clone(),
                    });
                }
            }
        }

        // Add resolved calls to the graph
        for call in resolved_calls {
            self.call_graph.add_call(call);
        }
    }

    /// Add an unresolved call for later resolution
    fn add_unresolved_call(
        &mut self,
        caller: FunctionId,
        callee_name: String,
        call_type: CallType,
        call_site_type: call_resolution::CallSiteType,
        same_file_hint: bool,
    ) {
        self.unresolved_calls.push(UnresolvedCall {
            caller,
            callee_name,
            call_type,
            call_site_type,
            same_file_hint,
        });
    }

    /// Process a function call
    fn process_call(
        &mut self,
        name: String,
        call_site_type: call_resolution::CallSiteType,
        same_file_hint: bool,
    ) {
        if let Some(current_fn) = &self.current_function {
            let call_type = CallResolver::classify_call_type(&name);
            let resolved_name = CallResolver::resolve_self_type(&name, &self.current_impl_type);

            self.add_unresolved_call(
                current_fn.clone(),
                resolved_name,
                call_type,
                call_site_type,
                same_file_hint,
            );
        }
    }

    /// Construct a method name from components and classify call site
    fn construct_method_name(
        &mut self,
        receiver: &Expr,
        method: &syn::Ident,
    ) -> (String, call_resolution::CallSiteType, bool) {
        let method_name = method.to_string();

        // Get receiver type
        let receiver_type = if CallResolver::is_self_receiver(receiver) {
            self.current_impl_type.clone()
        } else {
            self.type_tracker
                .resolve_expr_type(receiver)
                .map(|t| t.type_name)
        };

        // Determine call site type
        let call_site_type = if CallResolver::is_std_trait_method(&method_name) {
            call_resolution::CallSiteType::TraitMethod {
                trait_name: CallResolver::infer_trait_name(&method_name),
                receiver_type: receiver_type.clone(),
            }
        } else {
            call_resolution::CallSiteType::Instance {
                receiver_type: receiver_type.clone(),
            }
        };

        let full_name = CallResolver::construct_method_name(
            receiver_type,
            &method_name,
            &self.current_impl_type,
        );

        let same_file_hint =
            CallResolver::is_self_receiver(receiver) && self.current_impl_type.is_some();

        (full_name, call_site_type, same_file_hint)
    }

    /// Handle macro expression
    fn handle_macro_expression(&mut self, expr_macro: &ExprMacro) {
        let exprs = self.macro_expander.handle_macro_expression(expr_macro);
        for expr in exprs {
            self.visit_expr(&expr);
        }
    }

    /// Process function arguments
    fn process_arguments(&mut self, args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>) {
        for arg in args {
            self.check_for_function_reference(arg);
            self.visit_expr(arg);
        }
    }

    /// Check for function references in expressions
    fn check_for_function_reference(&mut self, expr: &Expr) {
        if let Expr::Path(path_expr) = expr {
            if let Some(func_name) = GraphBuilder::extract_function_name_from_path(&path_expr.path)
            {
                self.process_call(func_name, call_resolution::CallSiteType::Static, true);
            }
        }
    }

    /// Get line number from span
    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        self.graph_builder.get_line_number(span)
    }

    /// Add a function to the graph
    fn add_function_to_graph(&mut self, name: String, line: usize, item_fn: &ItemFn) {
        let function_id = self.graph_builder.add_function_from_item(
            name,
            line,
            item_fn,
            self.file_module_path.clone(),
        );
        self.current_function = Some(function_id);
    }

    /// Add an impl method to the graph
    fn add_impl_method_to_graph(&mut self, name: String, line: usize, impl_fn: &ImplItemFn) {
        let function_id =
            self.graph_builder
                .add_impl_method(name, line, impl_fn, self.file_module_path.clone());
        self.current_function = Some(function_id);
    }

    /// Handle call expression
    fn handle_call_expr(
        &mut self,
        func: &Expr,
        args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>,
    ) {
        if let Expr::Path(path_expr) = func {
            if let Some(func_name) = GraphBuilder::extract_function_name_from_path(&path_expr.path)
            {
                let same_file_hint =
                    CallResolver::is_same_file_call(&func_name, &self.current_impl_type);
                // Static calls use Expr::Call syntax
                let call_site_type = call_resolution::CallSiteType::Static;
                self.process_call(func_name, call_site_type, same_file_hint);
            }
        }

        self.process_arguments(args);
        self.visit_expr(func);
    }

    /// Handle method call expression
    fn handle_method_call_expr(
        &mut self,
        receiver: &Expr,
        method: &syn::Ident,
        args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>,
    ) {
        let (method_name, call_site_type, same_file_hint) =
            self.construct_method_name(receiver, method);
        self.process_call(method_name, call_site_type, same_file_hint);

        self.visit_expr(receiver);
        self.process_arguments(args);
    }

    /// Handle struct expression
    fn handle_struct_expr(&mut self, expr_struct: &syn::ExprStruct) {
        // Visit fields - each field may contain a function call
        for field in &expr_struct.fields {
            // Visit the field expression, which will handle any function calls
            self.visit_expr(&field.expr);
        }

        // Visit rest if present
        if let Some(rest) = &expr_struct.rest {
            self.visit_expr(rest);
        }
    }

    /// Process special expression categories
    fn process_special_expr(&mut self, expr: &Expr, category: ExprCategory) {
        match category {
            ExprCategory::Closure => self.process_closure_expr(expr),
            ExprCategory::Async => self.process_async_expr(expr),
            ExprCategory::Await => self.process_await_expr(expr),
            _ => syn::visit::visit_expr(self, expr),
        }
    }

    /// Process closure expression
    fn process_closure_expr(&mut self, expr: &Expr) {
        if let Expr::Closure(closure) = expr {
            self.type_tracker.enter_scope(ScopeKind::Block, None);
            syn::visit::visit_expr(self, &closure.body);
            self.type_tracker.exit_scope();
        }
    }

    /// Process async expression
    fn process_async_expr(&mut self, expr: &Expr) {
        if let Expr::Async(async_block) = expr {
            self.type_tracker.enter_scope(ScopeKind::Block, None);
            for stmt in &async_block.block.stmts {
                self.visit_stmt(stmt);
            }
            self.type_tracker.exit_scope();
        }
    }

    /// Process await expression
    fn process_await_expr(&mut self, expr: &Expr) {
        if let Expr::Await(await_expr) = expr {
            self.visit_expr(&await_expr.base);
        }
    }
}

/// Visitor implementation for syntax tree traversal
impl<'ast> Visit<'ast> for CallGraphExtractor {
    fn visit_stmt(&mut self, stmt: &'ast syn::Stmt) {
        match stmt {
            syn::Stmt::Local(local) => {
                self.visit_local(local);
            }
            _ => {
                syn::visit::visit_stmt(self, stmt);
            }
        }
    }

    fn visit_local(&mut self, local: &'ast Local) {
        // Track variable type if possible
        if let Pat::Ident(pat_ident) = &local.pat {
            let var_name = pat_ident.ident.to_string();

            if let Some(init) = &local.init {
                if let Some(type_info) = self.type_tracker.resolve_expr_type(&init.expr) {
                    self.type_tracker.record_variable(var_name, type_info);
                }
            }
        }

        syn::visit::visit_local(self, local);
    }

    fn visit_item_impl(&mut self, item_impl: &'ast syn::ItemImpl) {
        let impl_type = TraitHandler::extract_impl_type(item_impl);
        let old_impl_type = self.current_impl_type.clone();
        self.current_impl_type = impl_type.clone();

        self.type_tracker
            .enter_scope(ScopeKind::Impl, impl_type.clone());

        for item in &item_impl.items {
            if let syn::ImplItem::Fn(impl_fn) = item {
                self.visit_impl_item_fn(impl_fn);
            }
        }

        self.type_tracker.exit_scope();
        self.current_impl_type = old_impl_type;
    }

    fn visit_item_mod(&mut self, item_mod: &'ast syn::ItemMod) {
        let module_name = item_mod.ident.to_string();
        self.module_path.push(module_name.clone());
        self.graph_builder.push_module(module_name);

        syn::visit::visit_item_mod(self, item_mod);

        self.module_path.pop();
        self.graph_builder.pop_module();
    }

    fn visit_item_fn(&mut self, item_fn: &'ast ItemFn) {
        let func_name = item_fn.sig.ident.to_string();
        let qualified_name = if self.module_path.is_empty() {
            func_name.clone()
        } else {
            format!("{}::{}", self.module_path.join("::"), func_name)
        };

        let line = self.get_line_number(item_fn.sig.ident.span());
        self.add_function_to_graph(qualified_name, line, item_fn);

        self.type_tracker.enter_scope(ScopeKind::Function, None);

        // Track function parameters
        for input in &item_fn.sig.inputs {
            if let syn::FnArg::Typed(pat_type) = input {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let _param_name = pat_ident.ident.to_string();
                    // Track parameter - type inference would happen here
                    // For now, we skip tracking to avoid type issues
                }
            }
        }

        self.visit_block(&item_fn.block);

        self.type_tracker.exit_scope();
        self.current_function = None;
    }

    fn visit_impl_item_fn(&mut self, impl_fn: &'ast ImplItemFn) {
        let method_name = impl_fn.sig.ident.to_string();
        let qualified_name = if let Some(impl_type) = &self.current_impl_type {
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
        } else {
            method_name.clone()
        };

        let line = self.get_line_number(impl_fn.sig.ident.span());
        self.add_impl_method_to_graph(qualified_name, line, impl_fn);

        self.type_tracker.enter_scope(ScopeKind::Function, None);

        // Track self parameter if present
        for input in &impl_fn.sig.inputs {
            match input {
                syn::FnArg::Receiver(_) => {
                    // Self parameter handled by type tracker
                }
                syn::FnArg::Typed(pat_type) => {
                    if let Pat::Ident(pat_ident) = &*pat_type.pat {
                        let _param_name = pat_ident.ident.to_string();
                        // Track parameter - type inference would happen here
                        // For now, we skip tracking to avoid type issues
                    }
                }
            }
        }

        self.visit_block(&impl_fn.block);

        self.type_tracker.exit_scope();
        self.current_function = None;
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        // Check for special handling needs
        let category = GraphBuilder::classify_expr_category(expr);
        if GraphBuilder::needs_special_handling(category) {
            self.process_special_expr(expr, category);
            return;
        }

        // Handle regular expressions
        match expr {
            Expr::Call(call_expr) => {
                self.handle_call_expr(&call_expr.func, &call_expr.args);
            }
            Expr::MethodCall(method_call) => {
                self.handle_method_call_expr(
                    &method_call.receiver,
                    &method_call.method,
                    &method_call.args,
                );
            }
            Expr::Macro(expr_macro) => {
                self.handle_macro_expression(expr_macro);
            }
            Expr::Struct(expr_struct) => {
                self.handle_struct_expr(expr_struct);
            }
            Expr::Let(expr_let) => {
                if let Pat::Ident(pat_ident) = &*expr_let.pat {
                    let var_name = pat_ident.ident.to_string();
                    if let Some(type_info) = self.type_tracker.resolve_expr_type(&expr_let.expr) {
                        self.type_tracker.record_variable(var_name, type_info);
                    }
                }
                syn::visit::visit_expr(self, expr);
            }
            Expr::Field(field_expr) => {
                // Visit base expression
                self.visit_expr(&field_expr.base);
            }
            Expr::Path(_path_expr) => {
                self.check_for_function_reference(expr);
                syn::visit::visit_expr(self, expr);
            }
            Expr::Block(block_expr) => {
                self.type_tracker.enter_scope(ScopeKind::Block, None);
                for stmt in &block_expr.block.stmts {
                    self.visit_stmt(stmt);
                }
                self.type_tracker.exit_scope();
            }
            Expr::If(if_expr) => {
                self.visit_expr(&if_expr.cond);

                self.type_tracker.enter_scope(ScopeKind::Block, None);
                for stmt in &if_expr.then_branch.stmts {
                    self.visit_stmt(stmt);
                }
                self.type_tracker.exit_scope();

                if let Some((_, else_branch)) = &if_expr.else_branch {
                    self.type_tracker.enter_scope(ScopeKind::Block, None);
                    self.visit_expr(else_branch);
                    self.type_tracker.exit_scope();
                }
            }
            Expr::Loop(loop_expr) => {
                self.type_tracker.enter_scope(ScopeKind::Block, None);
                for stmt in &loop_expr.body.stmts {
                    self.visit_stmt(stmt);
                }
                self.type_tracker.exit_scope();
            }
            Expr::While(while_expr) => {
                self.visit_expr(&while_expr.cond);

                self.type_tracker.enter_scope(ScopeKind::Block, None);
                for stmt in &while_expr.body.stmts {
                    self.visit_stmt(stmt);
                }
                self.type_tracker.exit_scope();
            }
            Expr::ForLoop(for_loop) => {
                self.visit_expr(&for_loop.expr);

                self.type_tracker.enter_scope(ScopeKind::Block, None);

                // Track the loop variable
                if let Pat::Ident(pat_ident) = &*for_loop.pat {
                    let _var_name = pat_ident.ident.to_string();
                    // In a real implementation, we'd infer the iterator item type
                }

                for stmt in &for_loop.body.stmts {
                    self.visit_stmt(stmt);
                }
                self.type_tracker.exit_scope();
            }
            Expr::Match(match_expr) => {
                self.visit_expr(&match_expr.expr);

                for arm in &match_expr.arms {
                    self.type_tracker.enter_scope(ScopeKind::Block, None);

                    // Track pattern bindings
                    // In a real implementation, we'd extract type info from patterns

                    if let Some(guard) = &arm.guard {
                        self.visit_expr(&guard.1);
                    }
                    self.visit_expr(&arm.body);

                    self.type_tracker.exit_scope();
                }
            }
            _ => {
                syn::visit::visit_expr(self, expr);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_rust_code(code: &str) -> syn::File {
        syn::parse_str(code).expect("Failed to parse code")
    }

    #[test]
    fn test_basic_extraction() {
        let code = r#"
            fn main() {
                helper();
            }
            
            fn helper() {
                println!("Hello");
            }
        "#;

        let file = parse_rust_code(code);
        let extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        let graph = extractor.extract(&file);

        assert_eq!(graph.node_count(), 2);
        assert!(graph.get_all_functions().any(|f| f.name == "main"));
        assert!(graph.get_all_functions().any(|f| f.name == "helper"));
    }

    #[test]
    fn test_method_extraction() {
        let code = r#"
            struct MyStruct;
            
            impl MyStruct {
                fn new() -> Self {
                    MyStruct
                }
                
                fn method(&self) {
                    self.other_method();
                }
                
                fn other_method(&self) {
                    println!("Called");
                }
            }
        "#;

        let file = parse_rust_code(code);
        let extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        let graph = extractor.extract(&file);

        assert_eq!(graph.node_count(), 3);
        assert!(graph.get_all_functions().any(|f| f.name == "MyStruct::new"));
        assert!(graph
            .get_all_functions()
            .any(|f| f.name == "MyStruct::method"));
        assert!(graph
            .get_all_functions()
            .any(|f| f.name == "MyStruct::other_method"));
    }

    #[test]
    fn test_module_qualified_names() {
        let code = r#"
            mod submodule {
                pub fn func() {
                    inner_func();
                }

                fn inner_func() {
                    println!("Inner");
                }
            }

            fn main() {
                submodule::func();
            }
        "#;

        let file = parse_rust_code(code);
        let extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        let graph = extractor.extract(&file);

        assert!(graph.get_all_functions().any(|f| f.name == "main"));
        assert!(graph
            .get_all_functions()
            .any(|f| f.name == "submodule::func"));
        assert!(graph
            .get_all_functions()
            .any(|f| f.name == "submodule::inner_func"));
    }

    #[test]
    fn test_intra_struct_method_call_tracking() {
        let code = r#"
            struct Formatter {
                plain: bool,
            }

            impl Formatter {
                pub fn format_output(&self, data: &str) -> String {
                    // Calls helper method on self
                    let formatted = self.format_helper(data);
                    formatted
                }

                fn format_helper(&self, data: &str) -> String {
                    data.to_uppercase()
                }
            }
        "#;

        let file = parse_rust_code(code);
        let extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        let graph = extractor.extract(&file);

        // Find the functions
        let format_output = graph
            .get_all_functions()
            .find(|f| f.name.contains("format_output"))
            .expect("format_output should exist in call graph");

        let format_helper = graph
            .get_all_functions()
            .find(|f| f.name.contains("format_helper"))
            .expect("format_helper should exist in call graph");

        // Verify call is tracked - format_helper should have callers
        let callers = graph.get_callers(format_helper);
        assert!(
            !callers.is_empty(),
            "format_helper should have callers (format_output calls it via self.format_helper())"
        );

        let has_format_output = callers.iter().any(|c| c.name.contains("format_output"));
        assert!(
            has_format_output,
            "format_output should be in the list of callers for format_helper. Found callers: {:?}",
            callers.iter().map(|c| &c.name).collect::<Vec<_>>()
        );

        // Verify the reverse - format_output should have callees
        let callees = graph.get_callees(format_output);
        let has_format_helper = callees.iter().any(|c| c.name.contains("format_helper"));
        assert!(
            has_format_helper,
            "format_helper should be in the list of callees for format_output. Found callees: {:?}",
            callees.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_multiple_intra_struct_calls() {
        let code = r#"
            struct PatternOutputFormatter;

            impl PatternOutputFormatter {
                pub fn format_pattern_usage(&self, pattern: &Pattern) -> String {
                    self.format_pattern_type(&pattern.pattern_type)
                }

                pub fn format_detailed(&self, pattern: &Pattern) -> String {
                    self.format_pattern_type(&pattern.pattern_type)
                }

                pub fn format_compact(&self, pattern: &Pattern) -> String {
                    self.format_pattern_type(&pattern.pattern_type)
                }

                fn format_pattern_type(&self, pattern_type: &str) -> String {
                    pattern_type.to_string()
                }
            }

            struct Pattern {
                pattern_type: String,
            }
        "#;

        let file = parse_rust_code(code);
        let extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
        let graph = extractor.extract(&file);

        // Find format_pattern_type method
        let format_pattern_type = graph
            .get_all_functions()
            .find(|f| f.name.contains("format_pattern_type"))
            .expect("format_pattern_type should exist");

        // Verify it has callers
        let callers = graph.get_callers(format_pattern_type);
        assert!(
            callers.len() >= 3,
            "format_pattern_type should have at least 3 callers (format_pattern_usage, format_detailed, format_compact), found: {}. Callers: {:?}",
            callers.len(),
            callers.iter().map(|c| &c.name).collect::<Vec<_>>()
        );

        // Verify specific callers
        assert!(
            callers
                .iter()
                .any(|c| c.name.contains("format_pattern_usage")),
            "format_pattern_usage should be a caller"
        );
        assert!(
            callers.iter().any(|c| c.name.contains("format_detailed")),
            "format_detailed should be a caller"
        );
        assert!(
            callers.iter().any(|c| c.name.contains("format_compact")),
            "format_compact should be a caller"
        );
    }

    /// Test cross-module function calls via `use` imports.
    ///
    /// This reproduces the issue where `diagnose_coverage_file` calls `parse_lcov_file`
    /// from another module but the call graph shows 0 dependencies.
    ///
    /// Structure:
    /// - File 1 (commands/diagnose.rs): has `use crate::risk::lcov::parse_lcov_file;`
    ///   and calls `parse_lcov_file(path)?` and `generate_suggestions(...)`
    /// - File 2 (risk/lcov.rs): defines `parse_lcov_file`
    #[test]
    fn test_cross_module_call_via_use_import() {
        use crate::analyzers::rust_call_graph::extract_call_graph_multi_file;

        // File 1: commands/diagnose_coverage.rs
        let file1_code = r#"
            use crate::risk::lcov::parse_lcov_file;
            use anyhow::Result;
            use std::path::Path;

            pub fn diagnose_coverage_file(lcov_path: &Path, format: &str) -> Result<()> {
                let lcov_data = parse_lcov_file(lcov_path)?;
                let total_files = lcov_data.functions.len();
                let suggestions = generate_suggestions(total_files);
                Ok(())
            }

            fn generate_suggestions(total_files: usize) -> Vec<String> {
                vec![]
            }
        "#;

        // File 2: risk/lcov.rs
        let file2_code = r#"
            use anyhow::Result;
            use std::path::Path;

            pub struct LcovData {
                pub functions: std::collections::HashMap<String, Vec<()>>,
            }

            pub fn parse_lcov_file(path: &Path) -> Result<LcovData> {
                Ok(LcovData {
                    functions: std::collections::HashMap::new(),
                })
            }
        "#;

        let file1 = parse_rust_code(file1_code);
        let file2 = parse_rust_code(file2_code);

        let files = vec![
            (file1, PathBuf::from("src/commands/diagnose_coverage.rs")),
            (file2, PathBuf::from("src/risk/lcov.rs")),
        ];

        let graph = extract_call_graph_multi_file(&files);

        // Debug: print all functions found
        let all_funcs: Vec<_> = graph
            .get_all_functions()
            .map(|f| format!("{}:{}", f.file.display(), f.name))
            .collect();
        eprintln!("All functions in graph: {:?}", all_funcs);

        // Find diagnose_coverage_file
        let diagnose_fn = graph
            .get_all_functions()
            .find(|f| f.name.contains("diagnose_coverage_file"))
            .expect("diagnose_coverage_file should exist in call graph");

        // Find parse_lcov_file
        let parse_fn = graph
            .get_all_functions()
            .find(|f| f.name.contains("parse_lcov_file"))
            .expect("parse_lcov_file should exist in call graph");

        // Find generate_suggestions (same file call)
        let suggestions_fn = graph
            .get_all_functions()
            .find(|f| f.name.contains("generate_suggestions"))
            .expect("generate_suggestions should exist in call graph");

        // TEST 1: diagnose_coverage_file should have downstream callees
        let callees = graph.get_callees(diagnose_fn);
        eprintln!(
            "Callees of diagnose_coverage_file: {:?}",
            callees.iter().map(|c| &c.name).collect::<Vec<_>>()
        );

        // Should call generate_suggestions (same file)
        assert!(
            callees
                .iter()
                .any(|c| c.name.contains("generate_suggestions")),
            "diagnose_coverage_file should call generate_suggestions. Found callees: {:?}",
            callees.iter().map(|c| &c.name).collect::<Vec<_>>()
        );

        // Should call parse_lcov_file (cross-module via use import)
        assert!(
            callees.iter().any(|c| c.name.contains("parse_lcov_file")),
            "diagnose_coverage_file should call parse_lcov_file (cross-module). Found callees: {:?}",
            callees.iter().map(|c| &c.name).collect::<Vec<_>>()
        );

        // TEST 2: parse_lcov_file should have upstream callers
        let callers = graph.get_callers(parse_fn);
        eprintln!(
            "Callers of parse_lcov_file: {:?}",
            callers.iter().map(|c| &c.name).collect::<Vec<_>>()
        );

        assert!(
            callers
                .iter()
                .any(|c| c.name.contains("diagnose_coverage_file")),
            "parse_lcov_file should be called by diagnose_coverage_file. Found callers: {:?}",
            callers.iter().map(|c| &c.name).collect::<Vec<_>>()
        );

        // TEST 3: generate_suggestions should have upstream callers
        let callers = graph.get_callers(suggestions_fn);
        assert!(
            callers
                .iter()
                .any(|c| c.name.contains("diagnose_coverage_file")),
            "generate_suggestions should be called by diagnose_coverage_file. Found callers: {:?}",
            callers.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
    }

    /// Test that method calls on returned values are tracked.
    ///
    /// `lcov_data.functions.len()` should NOT create a call to user code,
    /// but `lcov_data.get_overall_coverage()` SHOULD if it's defined in user code.
    #[test]
    fn test_method_call_on_returned_struct() {
        use crate::analyzers::rust_call_graph::extract_call_graph_multi_file;

        let file1_code = r#"
            use crate::data::MyData;

            pub fn process_data() -> usize {
                let data = get_data();
                data.calculate_total()
            }

            fn get_data() -> MyData {
                MyData::new()
            }
        "#;

        let file2_code = r#"
            pub struct MyData {
                value: usize,
            }

            impl MyData {
                pub fn new() -> Self {
                    MyData { value: 42 }
                }

                pub fn calculate_total(&self) -> usize {
                    self.value
                }
            }
        "#;

        let file1 = parse_rust_code(file1_code);
        let file2 = parse_rust_code(file2_code);

        let files = vec![
            (file1, PathBuf::from("src/commands/process.rs")),
            (file2, PathBuf::from("src/data/mod.rs")),
        ];

        let graph = extract_call_graph_multi_file(&files);

        // Find process_data
        let process_fn = graph
            .get_all_functions()
            .find(|f| f.name.contains("process_data"))
            .expect("process_data should exist");

        let callees = graph.get_callees(process_fn);
        eprintln!(
            "Callees of process_data: {:?}",
            callees.iter().map(|c| &c.name).collect::<Vec<_>>()
        );

        // Should call get_data (same file)
        assert!(
            callees.iter().any(|c| c.name.contains("get_data")),
            "process_data should call get_data. Found: {:?}",
            callees.iter().map(|c| &c.name).collect::<Vec<_>>()
        );

        // Should call calculate_total (cross-module method call)
        // This is the tricky one - method call on a returned struct
        assert!(
            callees.iter().any(|c| c.name.contains("calculate_total")),
            "process_data should call calculate_total (method on returned struct). Found: {:?}",
            callees.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
    }
}

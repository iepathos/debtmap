pub mod call_resolution;
pub mod graph_builder;
/// Call graph extraction module providing two-pass resolution for accurate call tracking
///
/// This module is organized into focused submodules:
/// - `macro_expansion`: Handles macro parsing and expansion
/// - `call_resolution`: Resolves function calls to their definitions
/// - `graph_builder`: Builds and manages the call graph structure
/// - `trait_handling`: Handles trait resolution and method dispatch
pub mod macro_expansion;
pub mod trait_handling;

// Re-export main types for backward compatibility
pub use call_resolution::{CallResolver, UnresolvedCall};
pub use graph_builder::{ExprCategory, GraphBuilder};
pub use macro_expansion::{MacroExpander, MacroExpansionStats, MacroHandlingConfig};
pub use trait_handling::TraitHandler;

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

        Self {
            call_graph: graph_builder.call_graph.clone(),
            unresolved_calls: Vec::new(),
            current_function: None,
            current_impl_type: None,
            current_file: file.clone(),
            module_path: Vec::new(),
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

        Self {
            call_graph: graph_builder.call_graph.clone(),
            unresolved_calls: Vec::new(),
            current_function: None,
            current_impl_type: None,
            current_file: file,
            module_path: Vec::new(),
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
        let function_id = self
            .graph_builder
            .add_function_from_item(name, line, item_fn);
        self.current_function = Some(function_id);
    }

    /// Add an impl method to the graph
    fn add_impl_method_to_graph(&mut self, name: String, line: usize, impl_fn: &ImplItemFn) {
        let function_id = self.graph_builder.add_impl_method(name, line, impl_fn);
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
}

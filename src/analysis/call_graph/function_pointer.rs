//! Function Pointer and Closure Tracking
//!
//! This module tracks function pointers, closures, and higher-order functions
//! to resolve indirect function calls and reduce false positives in dead code detection.

use crate::priority::call_graph::FunctionId;
use anyhow::Result;
use im::{HashMap, HashSet, Vector};
use std::path::Path;
use syn::visit::Visit;
use syn::{
    Expr, ExprCall, ExprClosure, ExprPath, File, Ident, ItemFn, Local, Pat, PatIdent, Stmt, Type,
    TypeBareFn,
};

/// Information about a closure
#[derive(Debug, Clone)]
pub struct ClosureInfo {
    /// Unique identifier for this closure
    pub closure_id: String,
    /// Function that contains this closure
    pub containing_function: FunctionId,
    /// Line number where closure is defined
    pub line: usize,
    /// Functions that this closure calls
    pub calls: Vector<FunctionId>,
    /// Whether this closure captures variables
    pub captures_variables: bool,
    /// Parameters captured by reference
    pub captured_by_ref: HashSet<String>,
    /// Parameters captured by value
    pub captured_by_value: HashSet<String>,
}

/// Information about a function pointer
#[derive(Debug, Clone)]
pub struct FunctionPointerInfo {
    /// Variable name that holds the function pointer
    pub variable_name: String,
    /// Function where this pointer is defined
    pub defining_function: FunctionId,
    /// Possible target functions
    pub possible_targets: HashSet<FunctionId>,
    /// Line number where pointer is defined
    pub line: usize,
    /// Whether this is a function parameter
    pub is_parameter: bool,
}

/// Information about a function pointer call
#[derive(Debug, Clone)]
pub struct FunctionPointerCall {
    /// Function making the call
    pub caller: FunctionId,
    /// Function pointer being called
    pub pointer_id: String,
    /// Line number of the call
    pub line: usize,
}

/// Information about higher-order function usage
#[derive(Debug, Clone)]
pub struct HigherOrderFunctionCall {
    /// Function making the call
    pub caller: FunctionId,
    /// The higher-order function being called (map, filter, etc.)
    pub hof_function: String,
    /// Functions passed as arguments
    pub function_arguments: Vector<FunctionId>,
    /// Line number of the call
    pub line: usize,
}

/// Tracker for function pointers, closures, and higher-order functions
#[derive(Debug, Clone)]
pub struct FunctionPointerTracker {
    /// All closures found
    closures: HashMap<String, ClosureInfo>,
    /// All function pointers found
    function_pointers: HashMap<String, FunctionPointerInfo>,
    /// Function pointer calls that need resolution
    pointer_calls: Vector<FunctionPointerCall>,
    /// Higher-order function calls
    hof_calls: Vector<HigherOrderFunctionCall>,
    /// Mapping from variable names to function pointers
    variable_to_pointer: HashMap<String, String>,
    /// Functions that might be called through pointers
    potential_pointer_targets: HashSet<FunctionId>,
}

impl FunctionPointerTracker {
    /// Create a new function pointer tracker
    pub fn new() -> Self {
        Self {
            closures: HashMap::new(),
            function_pointers: HashMap::new(),
            pointer_calls: Vector::new(),
            hof_calls: Vector::new(),
            variable_to_pointer: HashMap::new(),
            potential_pointer_targets: HashSet::new(),
        }
    }

    /// Analyze a file for function pointers and closures
    pub fn analyze_file(&mut self, file_path: &Path, ast: &File) -> Result<()> {
        let mut visitor = FunctionPointerVisitor::new(file_path.to_path_buf());
        visitor.visit_file(ast);

        // Add discovered closures
        for closure in visitor.closures {
            let closure_id = closure.closure_id.clone();
            self.closures.insert(closure_id, closure);
        }

        // Add discovered function pointers
        for pointer in visitor.function_pointers {
            let pointer_id = format!(
                "{}_{}",
                pointer.defining_function.name, pointer.variable_name
            );

            // Update variable mapping
            self.variable_to_pointer
                .insert(pointer.variable_name.clone(), pointer_id.clone());

            // Add to potential targets
            for target in &pointer.possible_targets {
                self.potential_pointer_targets.insert(target.clone());
            }

            self.function_pointers.insert(pointer_id, pointer);
        }

        // Add function pointer calls
        for call in visitor.pointer_calls {
            self.pointer_calls.push_back(call);
        }

        // Add higher-order function calls
        for hof_call in visitor.hof_calls {
            // Add targets to potential pointer targets
            for func_arg in &hof_call.function_arguments {
                self.potential_pointer_targets.insert(func_arg.clone());
            }
            self.hof_calls.push_back(hof_call);
        }

        Ok(())
    }

    /// Get all function pointer calls that need resolution
    pub fn get_function_pointer_calls(&self) -> Vector<FunctionPointerCall> {
        self.pointer_calls.clone()
    }

    /// Resolve a function pointer to its possible targets
    pub fn resolve_pointer_targets(&self, pointer_id: &str) -> Option<Vector<FunctionId>> {
        self.function_pointers
            .get(pointer_id)
            .map(|pointer| pointer.possible_targets.iter().cloned().collect())
    }

    /// Check if a function might be called through a function pointer
    pub fn might_be_called_through_pointer(&self, func_id: &FunctionId) -> bool {
        self.potential_pointer_targets.contains(func_id)
            || self
                .closures
                .values()
                .any(|closure| closure.calls.contains(func_id))
    }

    /// Get all higher-order function calls
    pub fn get_higher_order_calls(&self) -> Vector<HigherOrderFunctionCall> {
        self.hof_calls.clone()
    }

    /// Get statistics about function pointer usage
    pub fn get_statistics(&self) -> FunctionPointerStatistics {
        let total_closures = self.closures.len();
        let total_function_pointers = self.function_pointers.len();
        let total_pointer_calls = self.pointer_calls.len();
        let total_hof_calls = self.hof_calls.len();
        let potential_targets = self.potential_pointer_targets.len();

        FunctionPointerStatistics {
            total_closures,
            total_function_pointers,
            total_pointer_calls,
            total_hof_calls,
            potential_targets,
        }
    }

    /// Get functions that are definitely used through function pointers
    pub fn get_definitely_used_functions(&self) -> HashSet<FunctionId> {
        let mut used_functions = HashSet::new();

        // Functions called from closures
        for closure in self.closures.values() {
            for called_func in &closure.calls {
                used_functions.insert(called_func.clone());
            }
        }

        // Functions passed to higher-order functions
        for hof_call in &self.hof_calls {
            for func_arg in &hof_call.function_arguments {
                used_functions.insert(func_arg.clone());
            }
        }

        used_functions
    }
}

/// Statistics about function pointer usage
#[derive(Debug, Clone)]
pub struct FunctionPointerStatistics {
    pub total_closures: usize,
    pub total_function_pointers: usize,
    pub total_pointer_calls: usize,
    pub total_hof_calls: usize,
    pub potential_targets: usize,
}

/// Visitor for extracting function pointer and closure information
struct FunctionPointerVisitor {
    file_path: std::path::PathBuf,
    closures: Vec<ClosureInfo>,
    function_pointers: Vec<FunctionPointerInfo>,
    pointer_calls: Vec<FunctionPointerCall>,
    hof_calls: Vec<HigherOrderFunctionCall>,
    current_function: Option<FunctionId>,
    closure_counter: usize,
}

impl FunctionPointerVisitor {
    fn new(file_path: std::path::PathBuf) -> Self {
        Self {
            file_path,
            closures: Vec::new(),
            function_pointers: Vec::new(),
            pointer_calls: Vec::new(),
            hof_calls: Vec::new(),
            current_function: None,
            closure_counter: 0,
        }
    }

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    fn is_higher_order_function(&self, name: &str) -> bool {
        matches!(
            name,
            "map"
                | "filter"
                | "fold"
                | "reduce"
                | "for_each"
                | "find"
                | "any"
                | "all"
                | "collect"
                | "and_then"
                | "or_else"
                | "iter"
                | "enumerate"
                | "zip"
                | "chain"
                | "take"
                | "skip"
        )
    }

    fn extract_function_name_from_path(&self, path: &ExprPath) -> Option<String> {
        if path.path.segments.len() == 1 {
            Some(path.path.segments.first()?.ident.to_string())
        } else {
            // For multi-segment paths, join with ::
            let segments: Vec<String> = path
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect();
            Some(segments.join("::"))
        }
    }

    fn analyze_closure(&mut self, closure: &ExprClosure) {
        if let Some(containing_function) = &self.current_function {
            self.closure_counter += 1;
            let closure_id = format!(
                "{}_closure_{}",
                containing_function.name, self.closure_counter
            );
            let line = self.get_line_number(closure.or1_token.span);

            let mut closure_visitor = ClosureCallVisitor::new();
            closure_visitor.visit_expr(&closure.body);

            let closure_info = ClosureInfo {
                closure_id,
                containing_function: containing_function.clone(),
                line,
                calls: closure_visitor.function_calls.into_iter().collect(),
                captures_variables: !closure.capture.is_none(),
                captured_by_ref: HashSet::new(), // Would need more analysis
                captured_by_value: HashSet::new(), // Would need more analysis
            };

            self.closures.push(closure_info);
        }
    }

    fn analyze_function_pointer_assignment(&mut self, local: &Local) {
        if let Some(current_func) = &self.current_function {
            // Check if this is assigning a function to a variable
            if let Pat::Ident(PatIdent { ident, .. }) = &local.pat {
                let var_name = ident.to_string();

                if let Some(init) = &local.init {
                    let line = self.get_line_number(ident.span());

                    // Check if the initializer is a function path
                    let mut possible_targets = HashSet::new();
                    if let Expr::Path(path) = &*init.expr {
                        if let Some(func_name) = self.extract_function_name_from_path(path) {
                            let target_func = FunctionId {
                                file: self.file_path.clone(),
                                name: func_name,
                                line: 0, // Unknown line for external function
                            };
                            possible_targets.insert(target_func);
                        }
                    }

                    let pointer_info = FunctionPointerInfo {
                        variable_name: var_name,
                        defining_function: current_func.clone(),
                        possible_targets,
                        line,
                        is_parameter: false,
                    };

                    self.function_pointers.push(pointer_info);
                }
            }
        }
    }

    fn analyze_call_expression(&mut self, call: &ExprCall) {
        if let Some(caller) = &self.current_function {
            let line = self.get_line_number(call.paren_token.span.open());

            match &*call.func {
                // Direct function pointer call: func_ptr(args)
                Expr::Path(path) => {
                    if let Some(func_name) = self.extract_function_name_from_path(path) {
                        // Check if this might be a function pointer call
                        let pointer_call = FunctionPointerCall {
                            caller: caller.clone(),
                            pointer_id: func_name,
                            line,
                        };
                        self.pointer_calls.push(pointer_call);
                    }
                }
                // Other types of calls could be added here
                _ => {}
            }

            // Check for higher-order function calls
            if let Expr::Path(path) = &*call.func {
                if let Some(func_name) = self.extract_function_name_from_path(path) {
                    if self.is_higher_order_function(&func_name) {
                        let mut function_arguments = Vector::new();

                        // Analyze arguments for function references
                        for arg in &call.args {
                            if let Expr::Path(arg_path) = arg {
                                if let Some(arg_func_name) =
                                    self.extract_function_name_from_path(arg_path)
                                {
                                    let func_arg = FunctionId {
                                        file: self.file_path.clone(),
                                        name: arg_func_name,
                                        line: 0,
                                    };
                                    function_arguments.push_back(func_arg);
                                }
                            }
                        }

                        if !function_arguments.is_empty() {
                            let hof_call = HigherOrderFunctionCall {
                                caller: caller.clone(),
                                hof_function: func_name,
                                function_arguments,
                                line,
                            };
                            self.hof_calls.push(hof_call);
                        }
                    }
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for FunctionPointerVisitor {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let func_name = item.sig.ident.to_string();
        let line = self.get_line_number(item.sig.ident.span());

        self.current_function = Some(FunctionId {
            file: self.file_path.clone(),
            name: func_name,
            line,
        });

        // Analyze function parameters for function pointers
        for param in &item.sig.inputs {
            if let syn::FnArg::Typed(typed_param) = param {
                if let Type::BareFn(_) = &*typed_param.ty {
                    // This is a function pointer parameter
                    if let Pat::Ident(PatIdent { ident, .. }) = &*typed_param.pat {
                        let param_name = ident.to_string();
                        let line = self.get_line_number(ident.span());

                        if let Some(current_func) = &self.current_function {
                            let pointer_info = FunctionPointerInfo {
                                variable_name: param_name,
                                defining_function: current_func.clone(),
                                possible_targets: HashSet::new(), // Unknown targets for parameters
                                line,
                                is_parameter: true,
                            };

                            self.function_pointers.push(pointer_info);
                        }
                    }
                }
            }
        }

        // Continue visiting the function body
        syn::visit::visit_item_fn(self, item);

        self.current_function = None;
    }

    fn visit_expr_closure(&mut self, expr: &'ast ExprClosure) {
        self.analyze_closure(expr);

        // Continue visiting
        syn::visit::visit_expr_closure(self, expr);
    }

    fn visit_local(&mut self, local: &'ast Local) {
        self.analyze_function_pointer_assignment(local);

        // Continue visiting
        syn::visit::visit_local(self, local);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        self.analyze_call_expression(call);

        // Continue visiting
        syn::visit::visit_expr_call(self, call);
    }
}

/// Visitor specifically for analyzing function calls within closures
struct ClosureCallVisitor {
    function_calls: Vec<FunctionId>,
}

impl ClosureCallVisitor {
    fn new() -> Self {
        Self {
            function_calls: Vec::new(),
        }
    }

    fn extract_function_name_from_path(&self, path: &ExprPath) -> Option<String> {
        if path.path.segments.len() == 1 {
            Some(path.path.segments.first()?.ident.to_string())
        } else {
            let segments: Vec<String> = path
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect();
            Some(segments.join("::"))
        }
    }
}

impl<'ast> Visit<'ast> for ClosureCallVisitor {
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = &*call.func {
            if let Some(func_name) = self.extract_function_name_from_path(path) {
                let func_id = FunctionId {
                    file: std::path::PathBuf::new(), // Will be filled in by parent
                    name: func_name,
                    line: 0,
                };
                self.function_calls.push(func_id);
            }
        }

        // Continue visiting
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let method_name = call.method.to_string();
        let func_id = FunctionId {
            file: std::path::PathBuf::new(),
            name: method_name,
            line: 0,
        };
        self.function_calls.push(func_id);

        // Continue visiting
        syn::visit::visit_expr_method_call(self, call);
    }
}

impl Default for FunctionPointerTracker {
    fn default() -> Self {
        Self::new()
    }
}

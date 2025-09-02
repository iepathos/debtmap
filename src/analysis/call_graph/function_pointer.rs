//! Function Pointer and Closure Tracking
//!
//! This module tracks function pointers, closures, and higher-order functions
//! to resolve indirect function calls and reduce false positives in dead code detection.

use crate::priority::call_graph::FunctionId;
use anyhow::Result;
use im::{HashMap, HashSet, Vector};
use std::path::Path;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprClosure, ExprPath, File, Ident, ItemFn, Local, Pat, PatIdent, Type};

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
                captures_variables: closure.capture.is_some(),
                captured_by_ref: HashSet::new(), // Would need more analysis
                captured_by_value: HashSet::new(), // Would need more analysis
            };

            self.closures.push(closure_info);
        }
    }

    /// Extract variable name and init expression from a local statement
    fn extract_pointer_assignment_data(local: &Local) -> Option<(&Ident, &Expr)> {
        match &local.pat {
            Pat::Ident(PatIdent { ident, .. }) => {
                local.init.as_ref().map(|init| (ident, &*init.expr))
            }
            _ => None,
        }
    }

    fn analyze_function_pointer_assignment(&mut self, local: &Local) {
        let Some(current_func) = &self.current_function else {
            return;
        };

        let Some((ident, init_expr)) = Self::extract_pointer_assignment_data(local) else {
            return;
        };

        let var_name = ident.to_string();
        let line = self.get_line_number(ident.span());
        let possible_targets = self.extract_possible_targets(init_expr);

        let pointer_info = FunctionPointerInfo {
            variable_name: var_name,
            defining_function: current_func.clone(),
            possible_targets,
            line,
            is_parameter: false,
        };

        self.function_pointers.push(pointer_info);
    }

    /// Extract possible function targets from an expression
    fn extract_possible_targets(&self, expr: &Expr) -> HashSet<FunctionId> {
        let mut possible_targets = HashSet::new();

        if let Expr::Path(path) = expr {
            if let Some(func_name) = self.extract_function_name_from_path(path) {
                let target_func = FunctionId {
                    file: self.file_path.clone(),
                    name: func_name,
                    line: 0, // Unknown line for external function
                };
                possible_targets.insert(target_func);
            }
        }

        possible_targets
    }

    /// Extract direct function pointer call from expression
    fn extract_direct_pointer_call(
        &self,
        call: &ExprCall,
        caller: &FunctionId,
        line: usize,
    ) -> Option<FunctionPointerCall> {
        if let Expr::Path(path) = &*call.func {
            self.extract_function_name_from_path(path)
                .map(|func_name| FunctionPointerCall {
                    caller: caller.clone(),
                    pointer_id: func_name,
                    line,
                })
        } else {
            None
        }
    }

    /// Extract higher-order function call from expression
    fn extract_hof_call(
        &self,
        call: &ExprCall,
        caller: &FunctionId,
        line: usize,
    ) -> Option<HigherOrderFunctionCall> {
        // Early return if not a path expression
        let path = match &*call.func {
            Expr::Path(p) => p,
            _ => return None,
        };

        // Extract and validate function name
        let func_name = self.extract_function_name_from_path(path)?;

        // Check if it's a higher-order function
        if !self.is_higher_order_function(&func_name) {
            return None;
        }

        // Extract function arguments
        let function_arguments = self.extract_function_arguments(call);

        // Return HOF call if arguments exist
        (!function_arguments.is_empty()).then(|| HigherOrderFunctionCall {
            caller: caller.clone(),
            hof_function: func_name,
            function_arguments,
            line,
        })
    }

    /// Extract function arguments from call expression
    fn extract_function_arguments(&self, call: &ExprCall) -> Vector<FunctionId> {
        let mut function_arguments = Vector::new();

        for arg in &call.args {
            if let Expr::Path(arg_path) = arg {
                if let Some(arg_func_name) = self.extract_function_name_from_path(arg_path) {
                    let func_arg = FunctionId {
                        file: self.file_path.clone(),
                        name: arg_func_name,
                        line: 0,
                    };
                    function_arguments.push_back(func_arg);
                }
            }
        }

        function_arguments
    }

    fn analyze_call_expression(&mut self, call: &ExprCall) {
        if let Some(caller) = &self.current_function {
            let line = self.get_line_number(call.paren_token.span.open());

            // Extract and store direct function pointer call
            if let Some(pointer_call) = self.extract_direct_pointer_call(call, caller, line) {
                self.pointer_calls.push(pointer_call);
            }

            // Extract and store higher-order function call
            if let Some(hof_call) = self.extract_hof_call(call, caller, line) {
                self.hof_calls.push(hof_call);
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

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_extract_direct_pointer_call() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        let caller = FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 1,
        };

        // Test direct function pointer call
        let call: ExprCall = parse_quote! { func_ptr(42) };
        let result = visitor.extract_direct_pointer_call(&call, &caller, 10);

        assert!(result.is_some());
        let pointer_call = result.unwrap();
        assert_eq!(pointer_call.pointer_id, "func_ptr");
        assert_eq!(pointer_call.line, 10);
        assert_eq!(pointer_call.caller.name, "test_func");
    }

    #[test]
    fn test_extract_direct_pointer_call_with_method_call() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        let caller = FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 1,
        };

        // Test method call (not a direct pointer call)
        // Using a different expression type that's not a Path
        let call: ExprCall = parse_quote! { compute(42) };
        // Manually change the func to something that's not a Path
        let mut call = call;
        call.func = Box::new(parse_quote! { 42 }); // Replace with a literal, not a path
        let result = visitor.extract_direct_pointer_call(&call, &caller, 10);

        assert!(result.is_none());
    }

    #[test]
    fn test_extract_hof_call_map() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        let caller = FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 1,
        };

        // Test higher-order function call with map
        let call: ExprCall = parse_quote! { map(process_item) };
        let result = visitor.extract_hof_call(&call, &caller, 15);

        assert!(result.is_some());
        let hof_call = result.unwrap();
        assert_eq!(hof_call.hof_function, "map");
        assert_eq!(hof_call.function_arguments.len(), 1);
        assert_eq!(hof_call.function_arguments[0].name, "process_item");
        assert_eq!(hof_call.line, 15);
    }

    #[test]
    fn test_extract_hof_call_filter() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        let caller = FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 1,
        };

        // Test higher-order function call with filter
        let call: ExprCall = parse_quote! { filter(is_valid) };
        let result = visitor.extract_hof_call(&call, &caller, 20);

        assert!(result.is_some());
        let hof_call = result.unwrap();
        assert_eq!(hof_call.hof_function, "filter");
        assert_eq!(hof_call.function_arguments.len(), 1);
        assert_eq!(hof_call.function_arguments[0].name, "is_valid");
    }

    #[test]
    fn test_extract_hof_call_non_hof() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        let caller = FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 1,
        };

        // Test non-higher-order function call
        let call: ExprCall = parse_quote! { regular_func(arg) };
        let result = visitor.extract_hof_call(&call, &caller, 25);

        assert!(result.is_none());
    }

    #[test]
    fn test_extract_hof_call_empty_arguments() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        let caller = FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 1,
        };

        // Test higher-order function call with no function arguments
        let call: ExprCall = parse_quote! { map() };
        let result = visitor.extract_hof_call(&call, &caller, 30);

        // Should return None when no function arguments
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_hof_call_closure_argument() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        let caller = FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 1,
        };

        // Test call with closure argument (should not extract closure)
        let call: ExprCall = parse_quote! { map(|x| x + 1) };
        let result = visitor.extract_hof_call(&call, &caller, 35);

        // Should return None when only closure arguments (not function paths)
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_hof_call_nested_path() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        let caller = FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 1,
        };

        // Test higher-order function with nested path argument
        let call: ExprCall = parse_quote! { filter(module::is_valid) };
        let result = visitor.extract_hof_call(&call, &caller, 40);

        assert!(result.is_some());
        let hof_call = result.unwrap();
        assert_eq!(hof_call.hof_function, "filter");
        // Should extract the full path as function name
        assert_eq!(hof_call.function_arguments.len(), 1);
        assert_eq!(hof_call.function_arguments[0].name, "module::is_valid");
    }

    #[test]
    fn test_extract_function_arguments() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));

        // Test extracting multiple function arguments
        let call: ExprCall = parse_quote! { fold(initial, combine_func) };
        let args = visitor.extract_function_arguments(&call);

        assert_eq!(args.len(), 2);
        assert_eq!(args[0].name, "initial");
        assert_eq!(args[1].name, "combine_func");
    }

    #[test]
    fn test_extract_function_arguments_mixed() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));

        // Test mixed arguments (functions and non-functions)
        let call: ExprCall = parse_quote! { process(42, handler, "string") };
        let args = visitor.extract_function_arguments(&call);

        // Should only extract function references
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].name, "handler");
    }

    #[test]
    fn test_extract_function_arguments_empty() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));

        // Test call with no function arguments
        let call: ExprCall = parse_quote! { compute(42, "string", true) };
        let args = visitor.extract_function_arguments(&call);

        assert!(args.is_empty());
    }

    #[test]
    fn test_is_higher_order_function() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));

        // Test HOF detection
        assert!(visitor.is_higher_order_function("map"));
        assert!(visitor.is_higher_order_function("filter"));
        assert!(visitor.is_higher_order_function("fold"));
        assert!(visitor.is_higher_order_function("for_each"));
        assert!(visitor.is_higher_order_function("find"));
        assert!(visitor.is_higher_order_function("any"));
        assert!(visitor.is_higher_order_function("all"));

        // Test non-HOF
        assert!(!visitor.is_higher_order_function("process"));
        assert!(!visitor.is_higher_order_function("compute"));
        assert!(!visitor.is_higher_order_function("regular_func"));
    }

    #[test]
    fn test_analyze_call_expression_integration() {
        let mut visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        visitor.current_function = Some(FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 1,
        });

        // Test direct pointer call
        let call: ExprCall = parse_quote! { callback() };
        visitor.analyze_call_expression(&call);
        assert_eq!(visitor.pointer_calls.len(), 1);
        assert_eq!(visitor.pointer_calls[0].pointer_id, "callback");

        // Test HOF call
        let hof_call: ExprCall = parse_quote! { map(transform) };
        visitor.analyze_call_expression(&hof_call);
        assert_eq!(visitor.hof_calls.len(), 1);
        assert_eq!(visitor.hof_calls[0].hof_function, "map");
    }

    #[test]
    fn test_analyze_call_expression_no_current_function() {
        let mut visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        // No current function set

        let call: ExprCall = parse_quote! { func() };
        visitor.analyze_call_expression(&call);

        // Should not record any calls without current function context
        assert!(visitor.pointer_calls.is_empty());
        assert!(visitor.hof_calls.is_empty());
    }

    #[test]
    fn test_extract_possible_targets_with_path() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));

        // Test extracting function target from path expression
        let expr: Expr = parse_quote! { my_function };
        let targets = visitor.extract_possible_targets(&expr);

        assert_eq!(targets.len(), 1);
        let target = targets.iter().next().unwrap();
        assert_eq!(target.name, "my_function");
        assert_eq!(target.file, std::path::PathBuf::from("test.rs"));
    }

    #[test]
    fn test_extract_possible_targets_with_qualified_path() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));

        // Test extracting function target from qualified path
        let expr: Expr = parse_quote! { module::submodule::function };
        let targets = visitor.extract_possible_targets(&expr);

        assert_eq!(targets.len(), 1);
        let target = targets.iter().next().unwrap();
        assert_eq!(target.name, "module::submodule::function");
    }

    #[test]
    fn test_extract_possible_targets_non_path() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));

        // Test with non-path expression (should return empty set)
        let expr: Expr = parse_quote! { 42 };
        let targets = visitor.extract_possible_targets(&expr);

        assert!(targets.is_empty());
    }

    #[test]
    fn test_extract_possible_targets_closure() {
        let visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));

        // Test with closure expression (should return empty set)
        let expr: Expr = parse_quote! { |x| x + 1 };
        let targets = visitor.extract_possible_targets(&expr);

        assert!(targets.is_empty());
    }

    #[test]
    fn test_analyze_function_pointer_assignment_complete() {
        let mut visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        visitor.current_function = Some(FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "outer_func".to_string(),
            line: 1,
        });

        // Test complete function pointer assignment
        // Creating a synthetic Local with function assignment
        let pat = Pat::Ident(PatIdent {
            attrs: vec![],
            by_ref: None,
            mutability: None,
            ident: parse_quote! { func_ptr },
            subpat: None,
        });

        let init_expr: Expr = parse_quote! { my_function };
        let local = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat,
            init: Some(syn::LocalInit {
                eq_token: Default::default(),
                expr: Box::new(init_expr),
                diverge: None,
            }),
            semi_token: Default::default(),
        };

        visitor.analyze_function_pointer_assignment(&local);

        assert_eq!(visitor.function_pointers.len(), 1);
        let pointer = &visitor.function_pointers[0];
        assert_eq!(pointer.variable_name, "func_ptr");
        assert!(!pointer.is_parameter);
        assert_eq!(pointer.possible_targets.len(), 1);
    }

    #[test]
    fn test_analyze_function_pointer_assignment_no_init() {
        let mut visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        visitor.current_function = Some(FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "outer_func".to_string(),
            line: 1,
        });

        // Test local without initialization (should not add pointer)
        let pat = Pat::Ident(PatIdent {
            attrs: vec![],
            by_ref: None,
            mutability: None,
            ident: parse_quote! { func_ptr },
            subpat: None,
        });

        let local = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat,
            init: None, // No initialization
            semi_token: Default::default(),
        };

        visitor.analyze_function_pointer_assignment(&local);

        assert!(visitor.function_pointers.is_empty());
    }

    #[test]
    fn test_analyze_function_pointer_assignment_non_ident_pattern() {
        let mut visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        visitor.current_function = Some(FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "outer_func".to_string(),
            line: 1,
        });

        // Test with tuple pattern (should not add pointer)
        let pat: Pat = parse_quote! { (a, b) };
        let init_expr: Expr = parse_quote! { get_tuple() };

        let local = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat,
            init: Some(syn::LocalInit {
                eq_token: Default::default(),
                expr: Box::new(init_expr),
                diverge: None,
            }),
            semi_token: Default::default(),
        };

        visitor.analyze_function_pointer_assignment(&local);

        assert!(visitor.function_pointers.is_empty());
    }

    #[test]
    fn test_analyze_function_pointer_assignment_no_current_function() {
        let mut visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        // No current function set

        let pat = Pat::Ident(PatIdent {
            attrs: vec![],
            by_ref: None,
            mutability: None,
            ident: parse_quote! { func_ptr },
            subpat: None,
        });

        let init_expr: Expr = parse_quote! { my_function };
        let local = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat,
            init: Some(syn::LocalInit {
                eq_token: Default::default(),
                expr: Box::new(init_expr),
                diverge: None,
            }),
            semi_token: Default::default(),
        };

        visitor.analyze_function_pointer_assignment(&local);

        // Should not record without current function context
        assert!(visitor.function_pointers.is_empty());
    }

    #[test]
    fn test_extract_pointer_assignment_data_with_ident() {
        // Test extracting from a valid identifier pattern with init
        let pat: Pat = parse_quote! { func_ptr };
        let init_expr: Expr = parse_quote! { my_function };
        let local = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat,
            init: Some(syn::LocalInit {
                eq_token: Default::default(),
                expr: Box::new(init_expr),
                diverge: None,
            }),
            semi_token: Default::default(),
        };

        let result = FunctionPointerVisitor::extract_pointer_assignment_data(&local);
        assert!(result.is_some());
        let (ident, _expr) = result.unwrap();
        assert_eq!(ident.to_string(), "func_ptr");
    }

    #[test]
    fn test_extract_pointer_assignment_data_without_init() {
        // Test with identifier pattern but no initialization
        let pat: Pat = parse_quote! { func_ptr };
        let local = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat,
            init: None,
            semi_token: Default::default(),
        };

        let result = FunctionPointerVisitor::extract_pointer_assignment_data(&local);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_pointer_assignment_data_with_non_ident_pattern() {
        // Test with a tuple pattern instead of identifier
        let pat: Pat = parse_quote! { (a, b) };
        let init_expr: Expr = parse_quote! { (func1, func2) };
        let local = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat,
            init: Some(syn::LocalInit {
                eq_token: Default::default(),
                expr: Box::new(init_expr),
                diverge: None,
            }),
            semi_token: Default::default(),
        };

        let result = FunctionPointerVisitor::extract_pointer_assignment_data(&local);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_pointer_assignment_data_with_mut_ident() {
        // Test with mutable identifier pattern
        let pat: Pat = parse_quote! { mut func_ptr };
        let init_expr: Expr = parse_quote! { my_function };
        let local = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat,
            init: Some(syn::LocalInit {
                eq_token: Default::default(),
                expr: Box::new(init_expr),
                diverge: None,
            }),
            semi_token: Default::default(),
        };

        let result = FunctionPointerVisitor::extract_pointer_assignment_data(&local);
        assert!(result.is_some());
        let (ident, _expr) = result.unwrap();
        assert_eq!(ident.to_string(), "func_ptr");
    }

    #[test]
    fn test_analyze_function_pointer_assignment_complete_flow() {
        // Test the complete flow with all components working together
        let mut visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        visitor.current_function = Some(FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "parent_func".to_string(),
            line: 10,
        });

        let pat: Pat = parse_quote! { callback };
        let init_expr: Expr = parse_quote! { process_item };
        let local = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat,
            init: Some(syn::LocalInit {
                eq_token: Default::default(),
                expr: Box::new(init_expr),
                diverge: None,
            }),
            semi_token: Default::default(),
        };

        visitor.analyze_function_pointer_assignment(&local);

        assert_eq!(visitor.function_pointers.len(), 1);
        let pointer_info = &visitor.function_pointers[0];
        assert_eq!(pointer_info.variable_name, "callback");
        assert_eq!(pointer_info.defining_function.name, "parent_func");
        assert!(!pointer_info.is_parameter);
        assert_eq!(pointer_info.possible_targets.len(), 1);
    }

    #[test]
    fn test_analyze_function_pointer_assignment_with_complex_init() {
        // Test with a more complex initialization expression
        let mut visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        visitor.current_function = Some(FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "setup_handlers".to_string(),
            line: 5,
        });

        let pat: Pat = parse_quote! { handler };
        // Using a closure expression
        let init_expr: Expr = parse_quote! { |x| x + 1 };
        let local = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat,
            init: Some(syn::LocalInit {
                eq_token: Default::default(),
                expr: Box::new(init_expr),
                diverge: None,
            }),
            semi_token: Default::default(),
        };

        visitor.analyze_function_pointer_assignment(&local);

        // Should still record the assignment even with non-path expression
        assert_eq!(visitor.function_pointers.len(), 1);
        let pointer_info = &visitor.function_pointers[0];
        assert_eq!(pointer_info.variable_name, "handler");
        // Possible targets will be empty for non-path expressions
        assert!(pointer_info.possible_targets.is_empty());
    }

    #[test]
    fn test_analyze_function_pointer_assignment_edge_cases() {
        // Test various edge cases
        let mut visitor = FunctionPointerVisitor::new(std::path::PathBuf::from("test.rs"));
        visitor.current_function = Some(FunctionId {
            file: std::path::PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 1,
        });

        // Edge case 1: Pattern with type annotation
        let pat: Pat = parse_quote! { func_ptr };
        let init_expr: Expr = parse_quote! { some_func };
        let local = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat,
            init: Some(syn::LocalInit {
                eq_token: Default::default(),
                expr: Box::new(init_expr),
                diverge: None,
            }),
            semi_token: Default::default(),
        };

        visitor.analyze_function_pointer_assignment(&local);
        assert_eq!(visitor.function_pointers.len(), 1);

        // Edge case 2: Destructuring pattern (should be ignored)
        let pat2: Pat = parse_quote! { Point { x, y } };
        let init_expr2: Expr = parse_quote! { get_point() };
        let local2 = Local {
            attrs: vec![],
            let_token: Default::default(),
            pat: pat2,
            init: Some(syn::LocalInit {
                eq_token: Default::default(),
                expr: Box::new(init_expr2),
                diverge: None,
            }),
            semi_token: Default::default(),
        };

        visitor.analyze_function_pointer_assignment(&local2);
        // Should still be 1, not 2
        assert_eq!(visitor.function_pointers.len(), 1);
    }
}

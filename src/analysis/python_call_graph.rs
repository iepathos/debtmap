//! Python-Specific Call Graph Analysis Module
//!
//! This module provides Python-specific call graph analysis that addresses false positives
//! in dead code detection by tracking:
//! - Instance method calls (self.method())  
//! - Class method calls (cls.method())
//! - Static method calls
//! - Context manager usage (with statements)
//! - Property access (@property decorators)
//! - Nested function definitions and callback patterns
//! - Functions passed as arguments to callback-accepting functions

use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use anyhow::Result;
use rustpython_parser::ast;
use std::collections::HashMap;
use std::path::Path;

/// Callback pattern configuration
#[derive(Debug, Clone)]
struct CallbackPattern {
    function_name: String,
    module_name: Option<String>,
    argument_position: usize, // Which argument is the callback (0-indexed)
}

/// Python-specific call graph analyzer
#[derive(Default)]
pub struct PythonCallGraphAnalyzer {
    current_class: Option<String>,
    current_function: Option<String>,
    function_lines: HashMap<String, usize>,
    nested_functions: HashMap<String, Vec<String>>, // parent -> nested functions
}

impl PythonCallGraphAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get callback patterns for a given function name
    fn get_callback_patterns() -> Vec<CallbackPattern> {
        vec![
            // wxPython
            CallbackPattern {
                function_name: "CallAfter".to_string(),
                module_name: Some("wx".to_string()),
                argument_position: 0,
            },
            CallbackPattern {
                function_name: "CallLater".to_string(),
                module_name: Some("wx".to_string()),
                argument_position: 1,
            },
            // asyncio
            CallbackPattern {
                function_name: "create_task".to_string(),
                module_name: Some("asyncio".to_string()),
                argument_position: 0,
            },
            CallbackPattern {
                function_name: "ensure_future".to_string(),
                module_name: Some("asyncio".to_string()),
                argument_position: 0,
            },
            CallbackPattern {
                function_name: "run_in_executor".to_string(),
                module_name: None,
                argument_position: 1,
            },
            // threading
            CallbackPattern {
                function_name: "Timer".to_string(),
                module_name: Some("threading".to_string()),
                argument_position: 1,
            },
            CallbackPattern {
                function_name: "Thread".to_string(),
                module_name: Some("threading".to_string()),
                argument_position: 0,
            },
            // multiprocessing
            CallbackPattern {
                function_name: "Process".to_string(),
                module_name: Some("multiprocessing".to_string()),
                argument_position: 0,
            },
            CallbackPattern {
                function_name: "apply_async".to_string(),
                module_name: None,
                argument_position: 0,
            },
            // Generic patterns
            CallbackPattern {
                function_name: "schedule".to_string(),
                module_name: None,
                argument_position: 0,
            },
            CallbackPattern {
                function_name: "submit".to_string(),
                module_name: None,
                argument_position: 0,
            },
            CallbackPattern {
                function_name: "defer".to_string(),
                module_name: None,
                argument_position: 0,
            },
            CallbackPattern {
                function_name: "setTimeout".to_string(),
                module_name: None,
                argument_position: 0,
            },
        ]
    }

    /// Check if a function accepts callbacks
    fn is_callback_accepting_function(
        &self,
        func_name: &str,
        module_name: Option<&str>,
    ) -> Option<usize> {
        let patterns = Self::get_callback_patterns();
        for pattern in patterns {
            if pattern.function_name == func_name {
                // If module_name is specified in pattern, check it matches
                if let Some(pattern_module) = &pattern.module_name {
                    if let Some(actual_module) = module_name {
                        if pattern_module == actual_module {
                            return Some(pattern.argument_position);
                        }
                    }
                } else {
                    // Pattern doesn't specify module, so match on function name alone
                    return Some(pattern.argument_position);
                }
            }
        }
        None
    }

    /// Build a nested function name
    #[allow(dead_code)]
    fn build_nested_function_name(&self, nested_name: &str) -> String {
        if let Some(parent) = &self.current_function {
            format!("{}.{}", parent, nested_name)
        } else if let Some(class_name) = &self.current_class {
            format!("{}.{}", class_name, nested_name)
        } else {
            nested_name.to_string()
        }
    }

    /// Analyze a Python module and extract method calls with source text for line numbers
    pub fn analyze_module_with_source(
        &mut self,
        module: &ast::Mod,
        file_path: &Path,
        source: &str,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        if let ast::Mod::Module(module) = module {
            // First pass: collect function line numbers
            self.collect_function_lines(&module.body, source, None);

            // Second pass: analyze function calls
            for stmt in &module.body {
                self.analyze_stmt(stmt, file_path, call_graph)?;
            }
        }
        Ok(())
    }

    /// Analyze without source (backward compatibility - uses line 0)
    pub fn analyze_module(
        &mut self,
        module: &ast::Mod,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                self.analyze_stmt(stmt, file_path, call_graph)?;
            }
        }
        Ok(())
    }

    /// Build a fully qualified function name given optional class context
    fn build_function_name(func_name: &str, class_context: Option<&str>) -> String {
        match class_context {
            Some(class_name) => format!("{}.{}", class_name, func_name),
            None => func_name.to_string(),
        }
    }

    /// Find the line number of a function definition in source code
    fn find_function_line(func_name: &str, prefix: &str, lines: &[&str]) -> usize {
        let def_pattern = format!("{} {}", prefix, func_name);
        lines
            .iter()
            .enumerate()
            .find(|(_, line)| line.trim_start().starts_with(&def_pattern))
            .map(|(idx, _)| idx + 1) // Line numbers are 1-based
            .unwrap_or(1)
    }

    /// Collect line numbers for all functions in the module using text search
    fn collect_function_lines(
        &mut self,
        stmts: &[ast::Stmt],
        source: &str,
        class_context: Option<&str>,
    ) {
        let lines: Vec<&str> = source.lines().collect();

        for stmt in stmts {
            match stmt {
                ast::Stmt::FunctionDef(func_def) => {
                    let func_name = Self::build_function_name(&func_def.name, class_context);
                    let line = Self::find_function_line(&func_def.name, "def", &lines);
                    self.function_lines.insert(func_name.clone(), line);

                    // Track this function as we descend to find nested functions
                    let prev_function = self.current_function.clone();
                    self.current_function = Some(func_name.clone());

                    // Collect nested functions - this already handles nested functions,
                    // so we don't need to call collect_function_lines recursively
                    self.collect_nested_functions_in_body(&func_def.body, source, &func_name);

                    self.current_function = prev_function;
                }
                ast::Stmt::AsyncFunctionDef(func_def) => {
                    let func_name = Self::build_function_name(&func_def.name, class_context);
                    let line = Self::find_function_line(&func_def.name, "async def", &lines);
                    self.function_lines.insert(func_name.clone(), line);

                    // Track this function as we descend to find nested functions
                    let prev_function = self.current_function.clone();
                    self.current_function = Some(func_name.clone());

                    // Collect nested functions - this already handles nested functions,
                    // so we don't need to call collect_function_lines recursively
                    self.collect_nested_functions_in_body(&func_def.body, source, &func_name);

                    self.current_function = prev_function;
                }
                ast::Stmt::ClassDef(class_def) => {
                    self.collect_function_lines(
                        &class_def.body,
                        source,
                        Some(class_def.name.as_ref()),
                    );
                }
                _ => {}
            }
        }
    }

    /// Collect nested functions within a function body
    fn collect_nested_functions_in_body(
        &mut self,
        stmts: &[ast::Stmt],
        source: &str,
        parent_func: &str,
    ) {
        let lines: Vec<&str> = source.lines().collect();
        let mut nested_funcs = Vec::new();

        for stmt in stmts {
            match stmt {
                ast::Stmt::FunctionDef(func_def) => {
                    // Only track as nested if we haven't already tracked it as a regular function
                    let nested_name = format!("{}.{}", parent_func, func_def.name);
                    if !self.function_lines.contains_key(&nested_name) {
                        let line = Self::find_function_line(&func_def.name, "def", &lines);
                        self.function_lines.insert(nested_name.clone(), line);
                    }
                    nested_funcs.push(func_def.name.to_string());
                }
                ast::Stmt::AsyncFunctionDef(func_def) => {
                    // Only track as nested if we haven't already tracked it as a regular function
                    let nested_name = format!("{}.{}", parent_func, func_def.name);
                    if !self.function_lines.contains_key(&nested_name) {
                        let line = Self::find_function_line(&func_def.name, "async def", &lines);
                        self.function_lines.insert(nested_name.clone(), line);
                    }
                    nested_funcs.push(func_def.name.to_string());
                }
                _ => {}
            }
        }

        if !nested_funcs.is_empty() {
            self.nested_functions
                .insert(parent_func.to_string(), nested_funcs);
        }
    }

    fn analyze_stmt(
        &mut self,
        stmt: &ast::Stmt,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        match stmt {
            ast::Stmt::ClassDef(class_def) => {
                self.analyze_class(class_def, file_path, call_graph)?;
            }
            ast::Stmt::FunctionDef(func_def) => {
                self.analyze_function(func_def, file_path, call_graph)?;
            }
            ast::Stmt::AsyncFunctionDef(func_def) => {
                self.analyze_async_function(func_def, file_path, call_graph)?;
            }
            ast::Stmt::With(with_stmt) => {
                self.analyze_with_stmt(with_stmt, file_path, call_graph)?;
            }
            _ => {
                // Recursively analyze nested statements
                self.analyze_nested_stmts(stmt, file_path, call_graph)?;
            }
        }
        Ok(())
    }

    fn analyze_class(
        &mut self,
        class_def: &ast::StmtClassDef,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        let prev_class = self.current_class.clone();
        self.current_class = Some(class_def.name.to_string());

        // Analyze all methods in the class
        for stmt in &class_def.body {
            self.analyze_stmt(stmt, file_path, call_graph)?;
        }

        self.current_class = prev_class;
        Ok(())
    }

    fn analyze_function(
        &mut self,
        func_def: &ast::StmtFunctionDef,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        let prev_function = self.current_function.clone();

        // Create function ID based on class context or parent function
        let func_name = if let Some(parent_func) = &self.current_function {
            // This is a nested function
            format!("{}.{}", parent_func, func_def.name)
        } else if let Some(class_name) = &self.current_class {
            format!("{}.{}", class_name, func_def.name)
        } else {
            func_def.name.to_string()
        };

        self.current_function = Some(func_name.clone());

        // Analyze function body for nested functions and method calls
        for stmt in &func_def.body {
            match stmt {
                ast::Stmt::FunctionDef(nested_func) => {
                    // Recursively analyze nested function
                    self.analyze_function(nested_func, file_path, call_graph)?;
                }
                ast::Stmt::AsyncFunctionDef(nested_func) => {
                    // Handle async nested functions too
                    self.analyze_async_function(nested_func, file_path, call_graph)?;
                }
                _ => {
                    self.analyze_stmt_for_calls(stmt, file_path, call_graph)?;
                }
            }
        }

        self.current_function = prev_function;
        Ok(())
    }

    fn analyze_async_function(
        &mut self,
        func_def: &ast::StmtAsyncFunctionDef,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        let prev_function = self.current_function.clone();

        // Create function ID based on class context or parent function
        let func_name = if let Some(parent_func) = &self.current_function {
            // This is a nested function
            format!("{}.{}", parent_func, func_def.name)
        } else if let Some(class_name) = &self.current_class {
            format!("{}.{}", class_name, func_def.name)
        } else {
            func_def.name.to_string()
        };

        self.current_function = Some(func_name.clone());

        // Analyze function body for nested functions and method calls
        for stmt in &func_def.body {
            match stmt {
                ast::Stmt::FunctionDef(nested_func) => {
                    // Recursively analyze nested function
                    self.analyze_function(nested_func, file_path, call_graph)?;
                }
                ast::Stmt::AsyncFunctionDef(nested_func) => {
                    // Handle async nested functions too
                    self.analyze_async_function(nested_func, file_path, call_graph)?;
                }
                _ => {
                    self.analyze_stmt_for_calls(stmt, file_path, call_graph)?;
                }
            }
        }

        self.current_function = prev_function;
        Ok(())
    }

    fn analyze_stmt_for_calls(
        &mut self,
        stmt: &ast::Stmt,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        match stmt {
            ast::Stmt::Expr(expr_stmt) => {
                self.analyze_expr_for_calls(&expr_stmt.value, file_path, call_graph)?;
            }
            ast::Stmt::Assign(assign_stmt) => {
                // Also analyze the value being assigned for calls
                self.analyze_expr_for_calls(&assign_stmt.value, file_path, call_graph)?;
            }
            ast::Stmt::With(with_stmt) => {
                self.analyze_with_stmt(with_stmt, file_path, call_graph)?;
            }
            ast::Stmt::If(if_stmt) => {
                for s in &if_stmt.body {
                    self.analyze_stmt_for_calls(s, file_path, call_graph)?;
                }
                for s in &if_stmt.orelse {
                    self.analyze_stmt_for_calls(s, file_path, call_graph)?;
                }
            }
            ast::Stmt::For(for_stmt) => {
                for s in &for_stmt.body {
                    self.analyze_stmt_for_calls(s, file_path, call_graph)?;
                }
            }
            ast::Stmt::While(while_stmt) => {
                for s in &while_stmt.body {
                    self.analyze_stmt_for_calls(s, file_path, call_graph)?;
                }
            }
            ast::Stmt::Try(try_stmt) => {
                for s in &try_stmt.body {
                    self.analyze_stmt_for_calls(s, file_path, call_graph)?;
                }
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    for s in &h.body {
                        self.analyze_stmt_for_calls(s, file_path, call_graph)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn analyze_expr_for_calls(
        &mut self,
        expr: &ast::Expr,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        match expr {
            ast::Expr::Call(call_expr) => {
                self.analyze_call_expr(call_expr, file_path, call_graph)?;
            }
            ast::Expr::Attribute(attr_expr) => {
                // Check if this is a method call on self/cls
                if let ast::Expr::Name(name) = &*attr_expr.value {
                    if name.id.as_str() == "self" || name.id.as_str() == "cls" {
                        // This is a method reference, track it if it's called
                        self.track_method_reference(attr_expr, file_path, call_graph)?;
                    }
                }
            }
            _ => {
                // Recursively analyze nested expressions
                self.analyze_nested_exprs(expr, file_path, call_graph)?;
            }
        }
        Ok(())
    }

    fn analyze_call_expr(
        &mut self,
        call_expr: &ast::ExprCall,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Check if this is a method call on self or cls
        if let ast::Expr::Attribute(attr_expr) = &*call_expr.func {
            if let ast::Expr::Name(name) = &*attr_expr.value {
                if name.id.as_str() == "self" || name.id.as_str() == "cls" {
                    // This is an instance/class method call
                    self.add_instance_method_call(&attr_expr.attr, file_path, call_graph)?;
                }
            }
        }

        // Check for event handler binding patterns like obj.Bind(event, self.method)
        self.check_for_event_binding(call_expr, file_path, call_graph)?;

        // Check for callback patterns like wx.CallAfter(nested_func, ...)
        self.check_for_callback_patterns(call_expr, file_path, call_graph)?;

        // Recursively analyze arguments
        for arg in &call_expr.args {
            self.analyze_expr_for_calls(arg, file_path, call_graph)?;
        }

        Ok(())
    }

    /// Check for callback patterns where functions are passed as arguments
    fn check_for_callback_patterns(
        &mut self,
        call_expr: &ast::ExprCall,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Extract the function name and module being called
        let (func_name, module_name) = match &*call_expr.func {
            ast::Expr::Name(name) => (name.id.to_string(), None),
            ast::Expr::Attribute(attr_expr) => {
                // Handle module.function pattern (e.g., wx.CallAfter)
                if let ast::Expr::Name(module) = &*attr_expr.value {
                    (attr_expr.attr.to_string(), Some(module.id.to_string()))
                } else {
                    // Could be a more complex expression like obj.method.CallAfter
                    (attr_expr.attr.to_string(), None)
                }
            }
            _ => return Ok(()),
        };

        // Check if this is a callback-accepting function
        if let Some(callback_position) =
            self.is_callback_accepting_function(&func_name, module_name.as_deref())
        {
            // Check if we have enough arguments
            if call_expr.args.len() > callback_position {
                // Get the argument at the callback position
                let callback_arg = &call_expr.args[callback_position];

                // Check if it's a function reference
                self.track_function_argument(callback_arg, file_path, call_graph)?;
            }
        }

        Ok(())
    }

    /// Track a function passed as an argument
    fn track_function_argument(
        &mut self,
        arg: &ast::Expr,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        match arg {
            ast::Expr::Name(name) => {
                // This is a simple function name - could be a nested function
                self.add_function_reference(&name.id, file_path, call_graph)?;
            }
            ast::Expr::Attribute(attr_expr) => {
                // This could be self.method or cls.method
                if let ast::Expr::Name(obj_name) = &*attr_expr.value {
                    if obj_name.id.as_str() == "self" || obj_name.id.as_str() == "cls" {
                        // This is a method reference passed as callback
                        self.add_event_handler_reference(&attr_expr.attr, file_path, call_graph)?;
                    }
                }
            }
            ast::Expr::Call(call_expr) => {
                // The callback might be the result of a function call
                // For now, just analyze the call itself
                self.analyze_call_expr(call_expr, file_path, call_graph)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Add a reference to a function (potentially nested) passed as an argument
    fn add_function_reference(
        &mut self,
        func_name: &str,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        if let Some(caller_name) = &self.current_function {
            // Check if this is a nested function
            let callee_name = if let Some(nested_funcs) = self.nested_functions.get(caller_name) {
                if nested_funcs.contains(&func_name.to_string()) {
                    // It's a nested function - build the full name
                    format!("{}.{}", caller_name, func_name)
                } else {
                    // Not a nested function, use as is
                    func_name.to_string()
                }
            } else {
                func_name.to_string()
            };

            // Get line numbers
            let caller_line = self.function_lines.get(caller_name).copied().unwrap_or(0);
            let callee_line = self.function_lines.get(&callee_name).copied().unwrap_or(0);

            let caller_id = FunctionId {
                name: caller_name.clone(),
                file: file_path.to_path_buf(),
                line: caller_line,
            };

            let callee_id = FunctionId {
                name: callee_name.clone(),
                file: file_path.to_path_buf(),
                line: callee_line,
            };

            // Add the call edge to indicate the function is referenced
            let call = FunctionCall {
                caller: caller_id,
                callee: callee_id,
                call_type: CallType::Direct,
            };

            call_graph.add_call(call);
        }

        Ok(())
    }

    fn check_for_event_binding(
        &mut self,
        call_expr: &ast::ExprCall,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Check if this is a method call like obj.Bind(...)
        if let ast::Expr::Attribute(attr_expr) = &*call_expr.func {
            let method_name = &attr_expr.attr;

            // Look for common event binding methods
            if self.is_event_binding_method(method_name) {
                // Check all arguments for method references like self.on_paint
                for arg in &call_expr.args {
                    if let ast::Expr::Attribute(handler_attr) = arg {
                        if let ast::Expr::Name(obj_name) = &*handler_attr.value {
                            if obj_name.id.as_str() == "self" || obj_name.id.as_str() == "cls" {
                                // This is a method reference passed as event handler
                                self.add_event_handler_reference(
                                    &handler_attr.attr,
                                    file_path,
                                    call_graph,
                                )?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn is_event_binding_method(&self, method_name: &str) -> bool {
        matches!(
            method_name,
            "Bind" |           // wxPython: obj.Bind(wx.EVT_PAINT, self.on_paint)
            "bind" |           // Tkinter: widget.bind("<Button-1>", self.on_click)
            "connect" |        // PyQt/PySide: signal.connect(self.slot)
            "on" |             // Some frameworks
            "addEventListener" | // Web frameworks
            "addListener" |    // Event systems
            "subscribe" |      // Observer patterns
            "observe" |        // Observer patterns
            "listen" // Event systems
        )
    }

    fn add_event_handler_reference(
        &mut self,
        method_name: &str,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        if let (Some(class_name), Some(caller_name)) = (&self.current_class, &self.current_function)
        {
            let handler_name = format!("{}.{}", class_name, method_name);

            // Use the actual line numbers we collected, or fall back to 0
            let caller_line = self.function_lines.get(caller_name).copied().unwrap_or(0);
            let handler_line = self.function_lines.get(&handler_name).copied().unwrap_or(0);

            let caller_id = FunctionId {
                name: caller_name.clone(),
                file: file_path.to_path_buf(),
                line: caller_line,
            };

            let handler_id = FunctionId {
                name: handler_name,
                file: file_path.to_path_buf(),
                line: handler_line,
            };

            // Add the call edge to indicate the handler is referenced by the caller
            let call = FunctionCall {
                caller: caller_id,
                callee: handler_id,
                call_type: CallType::Direct, // Event binding is a direct reference
            };

            call_graph.add_call(call);
        }

        Ok(())
    }

    fn analyze_with_stmt(
        &mut self,
        with_stmt: &ast::StmtWith,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Analyze context manager expressions (important for self._get_ssh_connection())
        for item in &with_stmt.items {
            if let ast::Expr::Call(call_expr) = &item.context_expr {
                self.analyze_call_expr(call_expr, file_path, call_graph)?;
            }
        }

        // Analyze body
        for stmt in &with_stmt.body {
            self.analyze_stmt_for_calls(stmt, file_path, call_graph)?;
        }

        Ok(())
    }

    fn add_instance_method_call(
        &mut self,
        method_name: &str,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        if let (Some(class_name), Some(caller_name)) = (&self.current_class, &self.current_function)
        {
            let callee_name = format!("{}.{}", class_name, method_name);

            // Use the actual line numbers we collected, or fall back to 0
            let caller_line = self.function_lines.get(caller_name).copied().unwrap_or(0);
            let callee_line = self.function_lines.get(&callee_name).copied().unwrap_or(0);

            let caller_id = FunctionId {
                name: caller_name.clone(),
                file: file_path.to_path_buf(),
                line: caller_line,
            };

            let callee_id = FunctionId {
                name: callee_name,
                file: file_path.to_path_buf(),
                line: callee_line,
            };

            // Add the call edge
            let call = FunctionCall {
                caller: caller_id,
                callee: callee_id,
                call_type: CallType::Direct,
            };

            call_graph.add_call(call);
        }

        Ok(())
    }

    fn track_method_reference(
        &mut self,
        _attr_expr: &ast::ExprAttribute,
        _file_path: &Path,
        _call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Track method references for potential indirect calls
        // This would handle cases where methods are passed as arguments
        Ok(())
    }

    fn analyze_nested_stmts(
        &mut self,
        stmt: &ast::Stmt,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Helper to recursively analyze nested statements
        match stmt {
            ast::Stmt::If(if_stmt) => {
                for s in &if_stmt.body {
                    self.analyze_stmt(s, file_path, call_graph)?;
                }
                for s in &if_stmt.orelse {
                    self.analyze_stmt(s, file_path, call_graph)?;
                }
            }
            ast::Stmt::For(for_stmt) => {
                for s in &for_stmt.body {
                    self.analyze_stmt(s, file_path, call_graph)?;
                }
            }
            ast::Stmt::While(while_stmt) => {
                for s in &while_stmt.body {
                    self.analyze_stmt(s, file_path, call_graph)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn analyze_nested_exprs(
        &mut self,
        expr: &ast::Expr,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Helper to recursively analyze nested expressions
        match expr {
            ast::Expr::BinOp(binop) => {
                self.analyze_expr_for_calls(&binop.left, file_path, call_graph)?;
                self.analyze_expr_for_calls(&binop.right, file_path, call_graph)?;
            }
            ast::Expr::UnaryOp(unaryop) => {
                self.analyze_expr_for_calls(&unaryop.operand, file_path, call_graph)?;
            }
            ast::Expr::IfExp(ifexp) => {
                self.analyze_expr_for_calls(&ifexp.test, file_path, call_graph)?;
                self.analyze_expr_for_calls(&ifexp.body, file_path, call_graph)?;
                self.analyze_expr_for_calls(&ifexp.orelse, file_path, call_graph)?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::parse;

    #[test]
    fn test_instance_method_call_detection() {
        let python_code = r#"
class MyClass:
    def _private_method(self):
        pass
    
    def public_method(self):
        self._private_method()
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module(&module, Path::new("test.py"), &mut call_graph)
            .unwrap();

        // Check that the call was tracked
        let private_method_id = FunctionId {
            name: "MyClass._private_method".to_string(),
            file: Path::new("test.py").to_path_buf(),
            line: 0,
        };

        assert!(
            !call_graph.get_callers(&private_method_id).is_empty(),
            "Private method should have callers"
        );
    }

    #[test]
    fn test_with_statement_method_call() {
        let python_code = r#"
class Manager:
    def _get_connection(self):
        pass
    
    def use_connection(self):
        with self._get_connection() as conn:
            pass
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module(&module, Path::new("test.py"), &mut call_graph)
            .unwrap();

        let connection_method_id = FunctionId {
            name: "Manager._get_connection".to_string(),
            file: Path::new("test.py").to_path_buf(),
            line: 0,
        };

        assert!(
            !call_graph.get_callers(&connection_method_id).is_empty(),
            "Connection method used in with statement should have callers"
        );
    }

    #[test]
    fn test_event_handler_binding_detection() {
        let python_code = r#"
class ConversationPanel:
    def on_paint(self, event):
        """Handle paint events to draw the drag-and-drop indicator line."""
        dc = wx.PaintDC(self.message_container)
        # ... drawing logic ...
    
    def setup_event_handlers(self):
        """Setup event handlers for the panel."""
        self.message_container.Bind(wx.EVT_PAINT, self.on_paint)
        self.widget.bind("<Button-1>", self.on_click)
        self.signal.connect(self.on_signal)
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module(&module, Path::new("conversation_panel.py"), &mut call_graph)
            .unwrap();

        let on_paint_id = FunctionId {
            name: "ConversationPanel.on_paint".to_string(),
            file: Path::new("conversation_panel.py").to_path_buf(),
            line: 0,
        };

        // on_paint should have callers because it's bound as an event handler
        assert!(
            !call_graph.get_callers(&on_paint_id).is_empty(),
            "on_paint should have callers because it's bound as an event handler"
        );
    }

    #[test]
    fn test_multiple_event_binding_frameworks() {
        let python_code = r#"
class EventPanel:
    def on_click(self, event):
        """Handle click events."""
        pass
    
    def on_signal(self):
        """Handle signal events."""
        pass
        
    def on_custom(self):
        """Handle custom events."""
        pass
    
    def setup_events(self):
        """Setup various event bindings."""
        # wxPython style
        self.widget.Bind(wx.EVT_CLICK, self.on_click)
        # PyQt/PySide style  
        self.signal.connect(self.on_signal)
        # Tkinter style
        self.frame.bind("<Button>", self.on_click)
        # Custom event system
        self.emitter.subscribe("custom", self.on_custom)
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module(&module, Path::new("event_panel.py"), &mut call_graph)
            .unwrap();

        // Check that all event handlers have callers
        let handlers = [
            "EventPanel.on_click",
            "EventPanel.on_signal",
            "EventPanel.on_custom",
        ];

        for handler_name in &handlers {
            let handler_id = FunctionId {
                name: handler_name.to_string(),
                file: Path::new("event_panel.py").to_path_buf(),
                line: 0,
            };

            assert!(
                !call_graph.get_callers(&handler_id).is_empty(),
                "{} should have callers because it's bound as an event handler",
                handler_name
            );
        }
    }

    #[test]
    fn test_nested_function_callback_detection() {
        let python_code = r#"
class DeliveryBoy:
    def deliver_message_added(self, observers, message, index):
        def deliver(observers, message, index):
            for observer in observers:
                observer.on_message_added(message, index)
        
        wx.CallAfter(deliver, observers, message, index)
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module_with_source(
                &module,
                Path::new("delivery_boy.py"),
                python_code,
                &mut call_graph,
            )
            .unwrap();

        // Check that the nested deliver function has callers
        // We need to check with the actual line number from the analyzer
        let deliver_line = analyzer
            .function_lines
            .get("DeliveryBoy.deliver_message_added.deliver")
            .copied()
            .unwrap_or(0);
        let deliver_id = FunctionId {
            name: "DeliveryBoy.deliver_message_added.deliver".to_string(),
            file: Path::new("delivery_boy.py").to_path_buf(),
            line: deliver_line,
        };

        // The nested deliver function should have callers because it's passed to wx.CallAfter
        assert!(
            !call_graph.get_callers(&deliver_id).is_empty(),
            "Nested deliver function should have callers because it's passed to wx.CallAfter"
        );
    }

    #[test]
    fn test_asyncio_callback_patterns() {
        let python_code = r#"
import asyncio

async def main():
    async def worker():
        await asyncio.sleep(1)
        return "done"
    
    task = asyncio.create_task(worker)
    result = await task
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module_with_source(
                &module,
                Path::new("async_test.py"),
                python_code,
                &mut call_graph,
            )
            .unwrap();

        let worker_line = analyzer
            .function_lines
            .get("main.worker")
            .copied()
            .unwrap_or(0);
        let worker_id = FunctionId {
            name: "main.worker".to_string(),
            file: Path::new("async_test.py").to_path_buf(),
            line: worker_line,
        };

        // Worker should have callers because it's passed to asyncio.create_task
        assert!(
            !call_graph.get_callers(&worker_id).is_empty(),
            "Worker function should have callers because it's passed to asyncio.create_task (line={})", worker_line
        );
    }

    #[test]
    fn test_threading_timer_callback() {
        let python_code = r#"
import threading

def schedule_task():
    def background_task():
        print("Running in background")
    
    timer = threading.Timer(5.0, background_task)
    timer.start()
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module_with_source(
                &module,
                Path::new("threading_test.py"),
                python_code,
                &mut call_graph,
            )
            .unwrap();

        let task_line = analyzer
            .function_lines
            .get("schedule_task.background_task")
            .copied()
            .unwrap_or(0);
        let task_id = FunctionId {
            name: "schedule_task.background_task".to_string(),
            file: Path::new("threading_test.py").to_path_buf(),
            line: task_line,
        };

        // background_task should have callers because it's passed to Timer
        assert!(
            !call_graph.get_callers(&task_id).is_empty(),
            "background_task should have callers because it's passed to threading.Timer"
        );
    }

    #[test]
    fn test_generic_callback_patterns() {
        let python_code = r#"
class Scheduler:
    def run_scheduled(self):
        def task():
            return "completed"
        
        # Various generic callback patterns
        self.scheduler.submit(task)
        self.queue.defer(task)
        setTimeout(task, 1000)
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module_with_source(
                &module,
                Path::new("scheduler_test.py"),
                python_code,
                &mut call_graph,
            )
            .unwrap();

        let task_line = analyzer
            .function_lines
            .get("Scheduler.run_scheduled.task")
            .copied()
            .unwrap_or(0);
        let task_id = FunctionId {
            name: "Scheduler.run_scheduled.task".to_string(),
            file: Path::new("scheduler_test.py").to_path_buf(),
            line: task_line,
        };

        // task should have callers because it's passed to various callback-accepting functions
        assert!(
            !call_graph.get_callers(&task_id).is_empty(),
            "task function should have callers because it's passed as a callback"
        );
    }
}

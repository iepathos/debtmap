//! Call Analysis and AST Traversal Module
//!
//! This module handles the core call graph analysis logic including
//! AST traversal, expression analysis, and call detection.

use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use anyhow::Result;
use rustpython_parser::ast;
use std::collections::HashMap;
use std::path::Path;

use super::callback_patterns::{
    extract_call_target, find_callback_position, get_callback_argument, get_callback_patterns,
};
use super::event_tracking::EventTracker;

/// Core call analysis functionality
pub struct CallAnalyzer<'a> {
    pub current_class: Option<&'a str>,
    pub current_function: Option<&'a str>,
    pub function_lines: &'a HashMap<String, usize>,
    pub nested_functions: &'a HashMap<String, Vec<String>>,
}

impl<'a> CallAnalyzer<'a> {
    pub fn new(
        current_class: Option<&'a str>,
        current_function: Option<&'a str>,
        function_lines: &'a HashMap<String, usize>,
        nested_functions: &'a HashMap<String, Vec<String>>,
    ) -> Self {
        Self {
            current_class,
            current_function,
            function_lines,
            nested_functions,
        }
    }

    /// Check if a function accepts callbacks
    pub fn is_callback_accepting_function(
        &self,
        func_name: &str,
        module_name: Option<&str>,
    ) -> Option<usize> {
        let patterns = get_callback_patterns();
        find_callback_position(&patterns, func_name, module_name)
    }

    /// Analyze expression for function calls
    pub fn analyze_expr_for_calls(
        &self,
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
                        let event_tracker = EventTracker::new(
                            self.current_class,
                            self.current_function,
                            self.function_lines,
                        );
                        event_tracker.track_method_reference(attr_expr, file_path, call_graph)?;
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

    /// Analyze call expression for various call patterns
    pub fn analyze_call_expr(
        &self,
        call_expr: &ast::ExprCall,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Check if this is a method call on self or cls
        if let ast::Expr::Attribute(attr_expr) = &*call_expr.func {
            if let ast::Expr::Name(name) = &*attr_expr.value {
                if name.id.as_str() == "self" || name.id.as_str() == "cls" {
                    // This is an instance/class method call
                    let event_tracker = EventTracker::new(
                        self.current_class,
                        self.current_function,
                        self.function_lines,
                    );
                    event_tracker.add_instance_method_call(
                        &attr_expr.attr,
                        file_path,
                        call_graph,
                    )?;
                }
            }
        }

        // Check for event handler binding patterns like obj.Bind(event, self.method)
        let event_tracker = EventTracker::new(
            self.current_class,
            self.current_function,
            self.function_lines,
        );
        event_tracker.check_for_event_binding(call_expr, file_path, call_graph)?;

        // Check for callback patterns like wx.CallAfter(nested_func, ...)
        self.check_for_callback_patterns(call_expr, file_path, call_graph)?;

        // Recursively analyze arguments
        for arg in &call_expr.args {
            self.analyze_expr_for_calls(arg, file_path, call_graph)?;
        }

        Ok(())
    }

    /// Check for callback patterns where functions are passed as arguments
    pub fn check_for_callback_patterns(
        &self,
        call_expr: &ast::ExprCall,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Extract the function name and module being called
        let Some((func_name, module_name)) = extract_call_target(call_expr) else {
            return Ok(());
        };

        // Check if this is a callback-accepting function
        if let Some(callback_position) =
            self.is_callback_accepting_function(&func_name, module_name.as_deref())
        {
            if let Some(callback_arg) = get_callback_argument(call_expr, callback_position) {
                self.track_function_argument(callback_arg, file_path, call_graph)?;
            }
        }

        Ok(())
    }

    /// Track a function passed as an argument
    pub fn track_function_argument(
        &self,
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
                        let event_tracker = EventTracker::new(
                            self.current_class,
                            self.current_function,
                            self.function_lines,
                        );
                        event_tracker.add_event_handler_reference(
                            &attr_expr.attr,
                            file_path,
                            call_graph,
                        )?;
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
    pub fn add_function_reference(
        &self,
        func_name: &str,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        if let Some(caller_name) = self.current_function {
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
                name: caller_name.to_string(),
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

    /// Analyze nested expressions recursively
    pub fn analyze_nested_exprs(
        &self,
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
            ast::Expr::Dict(dict_expr) => {
                // Analyze dictionary values that might be callbacks
                for value in &dict_expr.values {
                    self.analyze_expr_for_calls(value, file_path, call_graph)?;
                }
            }
            ast::Expr::List(list_expr) => {
                // Analyze list elements that might be callbacks
                for elem in &list_expr.elts {
                    self.analyze_expr_for_calls(elem, file_path, call_graph)?;
                }
            }
            ast::Expr::Tuple(tuple_expr) => {
                // Analyze tuple elements that might be callbacks
                for elem in &tuple_expr.elts {
                    self.analyze_expr_for_calls(elem, file_path, call_graph)?;
                }
            }
            ast::Expr::Set(set_expr) => {
                // Analyze set elements that might be callbacks
                for elem in &set_expr.elts {
                    self.analyze_expr_for_calls(elem, file_path, call_graph)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Pure function to extract statement bodies for analysis
pub fn extract_if_stmt_bodies(if_stmt: &ast::StmtIf) -> (&[ast::Stmt], &[ast::Stmt]) {
    (&if_stmt.body, &if_stmt.orelse)
}

/// Pure function to extract for loop body
pub fn extract_for_stmt_body(for_stmt: &ast::StmtFor) -> &[ast::Stmt] {
    &for_stmt.body
}

/// Pure function to extract while loop body
pub fn extract_while_stmt_body(while_stmt: &ast::StmtWhile) -> &[ast::Stmt] {
    &while_stmt.body
}

/// Pure function to extract all statement bodies from try block
pub fn extract_try_stmt_bodies(
    try_stmt: &ast::StmtTry,
) -> (Vec<&[ast::Stmt]>, &[ast::Stmt], &[ast::Stmt]) {
    let mut handler_bodies = Vec::new();
    for handler in &try_stmt.handlers {
        let ast::ExceptHandler::ExceptHandler(h) = handler;
        handler_bodies.push(h.body.as_slice());
    }
    (handler_bodies, &try_stmt.orelse, &try_stmt.finalbody)
}

/// Statement analyzer for control flow
pub struct StatementAnalyzer<'a> {
    call_analyzer: &'a CallAnalyzer<'a>,
}

impl<'a> StatementAnalyzer<'a> {
    pub fn new(call_analyzer: &'a CallAnalyzer<'a>) -> Self {
        Self { call_analyzer }
    }

    /// Analyze control flow statement by processing its bodies
    pub fn analyze_control_flow_stmt(
        &self,
        stmt: &ast::Stmt,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        match stmt {
            ast::Stmt::If(if_stmt) => {
                let (body, orelse) = extract_if_stmt_bodies(if_stmt);
                self.analyze_stmt_list(body, file_path, call_graph)?;
                self.analyze_stmt_list(orelse, file_path, call_graph)?;
            }
            ast::Stmt::For(for_stmt) => {
                let body = extract_for_stmt_body(for_stmt);
                self.analyze_stmt_list(body, file_path, call_graph)?;
            }
            ast::Stmt::While(while_stmt) => {
                let body = extract_while_stmt_body(while_stmt);
                self.analyze_stmt_list(body, file_path, call_graph)?;
            }
            ast::Stmt::Try(try_stmt) => {
                let (handler_bodies, orelse, finalbody) = extract_try_stmt_bodies(try_stmt);
                self.analyze_stmt_list(&try_stmt.body, file_path, call_graph)?;
                for handler_body in handler_bodies {
                    self.analyze_stmt_list(handler_body, file_path, call_graph)?;
                }
                self.analyze_stmt_list(orelse, file_path, call_graph)?;
                self.analyze_stmt_list(finalbody, file_path, call_graph)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Analyze statement for calls
    pub fn analyze_stmt_for_calls(
        &self,
        stmt: &ast::Stmt,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        match stmt {
            ast::Stmt::Expr(expr_stmt) => {
                self.call_analyzer.analyze_expr_for_calls(
                    &expr_stmt.value,
                    file_path,
                    call_graph,
                )?;
            }
            ast::Stmt::Assign(assign_stmt) => {
                self.call_analyzer.analyze_expr_for_calls(
                    &assign_stmt.value,
                    file_path,
                    call_graph,
                )?;
            }
            ast::Stmt::With(with_stmt) => {
                self.analyze_with_stmt(with_stmt, file_path, call_graph)?;
            }
            _ => {
                self.analyze_control_flow_stmt(stmt, file_path, call_graph)?;
            }
        }
        Ok(())
    }

    /// Analyze with statement
    pub fn analyze_with_stmt(
        &self,
        with_stmt: &ast::StmtWith,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Analyze context manager expressions
        self.analyze_context_managers(&with_stmt.items, file_path, call_graph)?;

        // Analyze body
        self.analyze_stmt_list(&with_stmt.body, file_path, call_graph)?;

        Ok(())
    }

    /// Extract context manager analysis logic as a pure-like method
    pub fn analyze_context_managers(
        &self,
        items: &[ast::WithItem],
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        for item in items {
            if let ast::Expr::Call(call_expr) = &item.context_expr {
                self.call_analyzer
                    .analyze_call_expr(call_expr, file_path, call_graph)?;
            }
        }
        Ok(())
    }

    /// Extract statement list analysis as a reusable method
    pub fn analyze_stmt_list(
        &self,
        stmts: &[ast::Stmt],
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        for stmt in stmts {
            self.analyze_stmt_for_calls(stmt, file_path, call_graph)?;
        }
        Ok(())
    }
}

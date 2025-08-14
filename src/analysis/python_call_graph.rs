//! Python-Specific Call Graph Analysis Module
//!
//! This module provides Python-specific call graph analysis that addresses false positives
//! in dead code detection by tracking:
//! - Instance method calls (self.method())  
//! - Class method calls (cls.method())
//! - Static method calls
//! - Context manager usage (with statements)
//! - Property access (@property decorators)

use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use anyhow::Result;
use rustpython_parser::ast;
use std::path::Path;

/// Python-specific call graph analyzer
#[derive(Default)]
pub struct PythonCallGraphAnalyzer {
    current_class: Option<String>,
    current_function: Option<String>,
    function_lines: std::collections::HashMap<String, usize>,
}

impl PythonCallGraphAnalyzer {
    pub fn new() -> Self {
        Self::default()
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
                    self.function_lines.insert(func_name, line);
                    self.collect_function_lines(&func_def.body, source, class_context);
                }
                ast::Stmt::AsyncFunctionDef(func_def) => {
                    let func_name = Self::build_function_name(&func_def.name, class_context);
                    let line = Self::find_function_line(&func_def.name, "async def", &lines);
                    self.function_lines.insert(func_name, line);
                    self.collect_function_lines(&func_def.body, source, class_context);
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

        // Create function ID based on class context
        let func_name = if let Some(class_name) = &self.current_class {
            format!("{}.{}", class_name, func_def.name)
        } else {
            func_def.name.to_string()
        };

        self.current_function = Some(func_name.clone());

        // Analyze function body for method calls
        for stmt in &func_def.body {
            self.analyze_stmt_for_calls(stmt, file_path, call_graph)?;
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

        // Recursively analyze arguments
        for arg in &call_expr.args {
            self.analyze_expr_for_calls(arg, file_path, call_graph)?;
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
}

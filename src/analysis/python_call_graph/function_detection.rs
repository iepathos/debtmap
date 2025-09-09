//! Function Detection and Line Number Collection Module
//!
//! This module provides pure functions for detecting function definitions
//! and collecting their line numbers from Python source code.

use rustpython_parser::ast;
use std::collections::HashMap;

/// Build a fully qualified function name given optional class context
pub fn build_function_name(func_name: &str, class_context: Option<&str>) -> String {
    match class_context {
        Some(class_name) => format!("{}.{}", class_name, func_name),
        None => func_name.to_string(),
    }
}

/// Find the line number of a function definition in source code
pub fn find_function_line(func_name: &str, prefix: &str, lines: &[&str]) -> usize {
    let def_pattern = format!("{} {}", prefix, func_name);
    lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.trim_start().starts_with(&def_pattern))
        .map(|(idx, _)| idx + 1) // Line numbers are 1-based
        .unwrap_or(1)
}

/// Pure function to extract function info from function definition
pub fn extract_function_info(
    func_name: &str,
    definition_prefix: &str,
    class_context: Option<&str>,
    lines: &[&str],
) -> (String, usize) {
    let qualified_name = build_function_name(func_name, class_context);
    let line = find_function_line(func_name, definition_prefix, lines);
    (qualified_name, line)
}

/// Pure function to process a regular function definition statement
pub fn process_function_def(
    func_def: &ast::StmtFunctionDef,
    class_context: Option<&str>,
    lines: &[&str],
) -> (String, usize) {
    extract_function_info(&func_def.name, "def", class_context, lines)
}

/// Pure function to process an async function definition statement
pub fn process_async_function_def(
    func_def: &ast::StmtAsyncFunctionDef,
    class_context: Option<&str>,
    lines: &[&str],
) -> (String, usize) {
    extract_function_info(&func_def.name, "async def", class_context, lines)
}

/// Function line collection state
pub struct FunctionLineCollector {
    pub function_lines: HashMap<String, usize>,
    pub nested_functions: HashMap<String, Vec<String>>, // parent -> nested functions
    current_function: Option<String>,
}

impl FunctionLineCollector {
    pub fn new() -> Self {
        Self {
            function_lines: HashMap::new(),
            nested_functions: HashMap::new(),
            current_function: None,
        }
    }

    /// Collect line numbers for all functions in the module using text search
    pub fn collect_function_lines(
        &mut self,
        stmts: &[ast::Stmt],
        source: &str,
        class_context: Option<&str>,
    ) {
        let lines: Vec<&str> = source.lines().collect();

        for stmt in stmts {
            self.process_single_statement(stmt, &lines, source, class_context);
        }
    }

    /// Process a single statement for function line collection
    fn process_single_statement(
        &mut self,
        stmt: &ast::Stmt,
        lines: &[&str],
        source: &str,
        class_context: Option<&str>,
    ) {
        match stmt {
            ast::Stmt::FunctionDef(func_def) => {
                let (func_name, line) = process_function_def(func_def, class_context, lines);
                self.function_lines.insert(func_name.clone(), line);
                self.process_function_with_nested(&func_def.body, source, func_name);
            }
            ast::Stmt::AsyncFunctionDef(func_def) => {
                let (func_name, line) = process_async_function_def(func_def, class_context, lines);
                self.function_lines.insert(func_name.clone(), line);
                self.process_function_with_nested(&func_def.body, source, func_name);
            }
            ast::Stmt::ClassDef(class_def) => {
                self.collect_function_lines(&class_def.body, source, Some(class_def.name.as_ref()));
            }
            _ => {}
        }
    }

    /// Process a function body while tracking nested functions
    fn process_function_with_nested(
        &mut self,
        body: &[ast::Stmt],
        source: &str,
        func_name: String,
    ) {
        let prev_function = self.current_function.clone();
        self.current_function = Some(func_name.clone());
        self.collect_nested_functions_in_body(body, source, &func_name);
        self.current_function = prev_function;
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
                        let line = find_function_line(&func_def.name, "def", &lines);
                        self.function_lines.insert(nested_name.clone(), line);
                    }
                    nested_funcs.push(func_def.name.to_string());
                }
                ast::Stmt::AsyncFunctionDef(func_def) => {
                    // Only track as nested if we haven't already tracked it as a regular function
                    let nested_name = format!("{}.{}", parent_func, func_def.name);
                    if !self.function_lines.contains_key(&nested_name) {
                        let line = find_function_line(&func_def.name, "async def", &lines);
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

    /// Get collected function lines
    pub fn get_function_lines(&self) -> &HashMap<String, usize> {
        &self.function_lines
    }

    /// Get nested functions mapping
    pub fn get_nested_functions(&self) -> &HashMap<String, Vec<String>> {
        &self.nested_functions
    }

    /// Build a nested function name
    #[allow(dead_code)]
    pub fn build_nested_function_name(&self, nested_name: &str) -> String {
        if let Some(parent) = &self.current_function {
            format!("{}.{}", parent, nested_name)
        } else {
            nested_name.to_string()
        }
    }
}

impl Default for FunctionLineCollector {
    fn default() -> Self {
        Self::new()
    }
}

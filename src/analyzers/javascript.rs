use crate::analyzers::Analyzer;
use crate::core::{
    ast::{Ast, JavaScriptAst, TypeScriptAst},
    ComplexityMetrics, DebtItem, DebtType, Dependency, DependencyKind, FileMetrics,
    FunctionMetrics, Language, Priority,
};
use crate::debt::patterns::{
    find_code_smells_with_suppression, find_todos_and_fixmes_with_suppression,
};
use crate::debt::smells::{analyze_function_smells, analyze_module_smells};
use crate::debt::suppression::{parse_suppression_comments, SuppressionContext};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

pub struct JavaScriptAnalyzer {
    parser: Parser,
    language: Language,
    complexity_threshold: u32,
}

impl JavaScriptAnalyzer {
    pub fn new_javascript() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .context("Failed to set JavaScript language")?;
        Ok(Self {
            parser,
            language: Language::JavaScript,
            complexity_threshold: 10,
        })
    }

    pub fn new_typescript() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .context("Failed to set TypeScript language")?;
        Ok(Self {
            parser,
            language: Language::TypeScript,
            complexity_threshold: 10,
        })
    }

    fn parse_tree(&mut self, content: &str) -> Result<tree_sitter::Tree> {
        self.parser
            .parse(content, None)
            .context("Failed to parse JavaScript/TypeScript code")
    }

    fn extract_functions(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        path: &Path,
    ) -> Vec<FunctionMetrics> {
        let mut functions = Vec::new();
        let root_node = tree.root_node();
        self.visit_node_for_functions(root_node, source, path, &mut functions);
        functions
    }

    fn visit_node_for_functions(
        &self,
        node: Node,
        source: &str,
        path: &Path,
        functions: &mut Vec<FunctionMetrics>,
    ) {
        match node.kind() {
            "function_declaration"
            | "function_expression"
            | "arrow_function"
            | "method_definition"
            | "generator_function_declaration" => {
                if let Some(metrics) = self.analyze_function(node, source, path) {
                    functions.push(metrics);
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.visit_node_for_functions(child, source, path, functions);
        }
    }

    fn analyze_function(&self, node: Node, source: &str, path: &Path) -> Option<FunctionMetrics> {
        let name = self.get_function_name(node, source);
        let line = node.start_position().row + 1;
        let mut metrics = FunctionMetrics::new(name, path.to_path_buf(), line);

        // Calculate complexity
        metrics.cyclomatic = self.calculate_cyclomatic_complexity(node, source);
        metrics.cognitive = self.calculate_cognitive_complexity(node, source, 0);
        metrics.nesting = self.calculate_max_nesting(node, 0);
        metrics.length = node.end_position().row - node.start_position().row + 1;

        Some(metrics)
    }

    fn get_function_name(&self, node: Node, source: &str) -> String {
        // Try to find the identifier node for the function name
        for child in node.children(&mut node.walk()) {
            if child.kind() == "identifier" || child.kind() == "property_identifier" {
                if let Ok(name) = child.utf8_text(source.as_bytes()) {
                    return name.to_string();
                }
            }
        }

        // For arrow functions without explicit names, try to find assignment
        if node.kind() == "arrow_function" {
            if let Some(parent) = node.parent() {
                if parent.kind() == "variable_declarator" {
                    for child in parent.children(&mut parent.walk()) {
                        if child.kind() == "identifier" {
                            if let Ok(name) = child.utf8_text(source.as_bytes()) {
                                return name.to_string();
                            }
                        }
                    }
                }
            }
        }

        "<anonymous>".to_string()
    }

    fn calculate_cyclomatic_complexity(&self, node: Node, source: &str) -> u32 {
        let mut complexity = 1; // Base complexity

        self.visit_node_for_complexity(node, source, &mut complexity);
        complexity
    }

    fn visit_node_for_complexity(&self, node: Node, source: &str, complexity: &mut u32) {
        match node.kind() {
            // Control flow statements
            "if_statement" | "ternary_expression" => *complexity += 1,
            "switch_case" | "case_statement" => *complexity += 1,
            "while_statement" | "do_statement" | "for_statement" | "for_in_statement"
            | "for_of_statement" => *complexity += 1,
            "catch_clause" => *complexity += 1,
            // Logical operators create branches
            "binary_expression" => {
                if let Ok(text) = node.utf8_text(source.as_bytes()) {
                    if text.contains("&&") || text.contains("||") {
                        *complexity += 1;
                    }
                }
            }
            // Optional chaining and nullish coalescing
            "optional_chain" => *complexity += 1,
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.visit_node_for_complexity(child, source, complexity);
        }
    }

    fn calculate_cognitive_complexity(&self, node: Node, source: &str, nesting_level: u32) -> u32 {
        let mut complexity = 0;

        match node.kind() {
            // Structural complexity
            "if_statement" => {
                complexity += 1 + nesting_level;
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "else_clause" {
                        complexity += 1; // Additional complexity for else
                    }
                    complexity +=
                        self.calculate_cognitive_complexity(child, source, nesting_level + 1);
                }
                return complexity;
            }
            "switch_statement" => {
                complexity += nesting_level;
                for child in node.children(&mut node.walk()) {
                    complexity +=
                        self.calculate_cognitive_complexity(child, source, nesting_level + 1);
                }
                return complexity;
            }
            "while_statement" | "do_statement" | "for_statement" | "for_in_statement"
            | "for_of_statement" => {
                complexity += 1 + nesting_level;
                for child in node.children(&mut node.walk()) {
                    complexity +=
                        self.calculate_cognitive_complexity(child, source, nesting_level + 1);
                }
                return complexity;
            }
            "catch_clause" => {
                complexity += nesting_level;
                for child in node.children(&mut node.walk()) {
                    complexity +=
                        self.calculate_cognitive_complexity(child, source, nesting_level + 1);
                }
                return complexity;
            }
            "ternary_expression" => {
                complexity += nesting_level;
            }
            // Nested functions and callbacks increase cognitive complexity
            "function_expression" | "arrow_function" if nesting_level > 0 => {
                complexity += 1 + nesting_level;
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            complexity += self.calculate_cognitive_complexity(child, source, nesting_level);
        }

        complexity
    }

    fn calculate_max_nesting(&self, node: Node, current_depth: u32) -> u32 {
        let mut max_depth = current_depth;
        let new_depth = match node.kind() {
            "if_statement" | "while_statement" | "do_statement" | "for_statement"
            | "for_in_statement" | "for_of_statement" | "switch_statement" | "try_statement" => {
                current_depth + 1
            }
            _ => current_depth,
        };

        for child in node.children(&mut node.walk()) {
            let child_depth = self.calculate_max_nesting(child, new_depth);
            max_depth = max_depth.max(child_depth);
        }

        max_depth
    }

    fn extract_dependencies(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        let root_node = tree.root_node();
        self.visit_node_for_dependencies(root_node, source, &mut dependencies);
        dependencies
    }

    fn visit_node_for_dependencies(
        &self,
        node: Node,
        source: &str,
        dependencies: &mut Vec<Dependency>,
    ) {
        match node.kind() {
            // ES6 imports
            "import_statement" => {
                if let Some(source_node) = node.child_by_field_name("source") {
                    if let Ok(module_name) = source_node.utf8_text(source.as_bytes()) {
                        dependencies.push(Dependency {
                            name: module_name
                                .trim_matches(|c| c == '"' || c == '\'' || c == '`')
                                .to_string(),
                            kind: DependencyKind::Import,
                        });
                    }
                }
            }
            // CommonJS require
            "call_expression" => {
                if let Some(function_node) = node.child_by_field_name("function") {
                    if let Ok(func_name) = function_node.utf8_text(source.as_bytes()) {
                        if func_name == "require" {
                            if let Some(args_node) = node.child_by_field_name("arguments") {
                                for child in args_node.children(&mut args_node.walk()) {
                                    if child.kind() == "string" {
                                        if let Ok(module_name) = child.utf8_text(source.as_bytes())
                                        {
                                            dependencies.push(Dependency {
                                                name: module_name
                                                    .trim_matches(|c| {
                                                        c == '"' || c == '\'' || c == '`'
                                                    })
                                                    .to_string(),
                                                kind: DependencyKind::Import,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Dynamic imports
            "import" => {
                // Handle dynamic import() expressions
                if let Some(parent) = node.parent() {
                    if parent.kind() == "call_expression" {
                        if let Some(args_node) = parent.child_by_field_name("arguments") {
                            for child in args_node.children(&mut args_node.walk()) {
                                if child.kind() == "string" {
                                    if let Ok(module_name) = child.utf8_text(source.as_bytes()) {
                                        dependencies.push(Dependency {
                                            name: module_name
                                                .trim_matches(|c| c == '"' || c == '\'' || c == '`')
                                                .to_string(),
                                            kind: DependencyKind::Import,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.visit_node_for_dependencies(child, source, dependencies);
        }
    }

    fn create_debt_items(
        &self,
        _tree: &tree_sitter::Tree,
        source: &str,
        path: &Path,
        functions: &[FunctionMetrics],
    ) -> Vec<DebtItem> {
        let suppression_context = parse_suppression_comments(source, self.language, path);

        // Report unclosed blocks
        report_unclosed_blocks(&suppression_context);

        let mut debt_items = Vec::new();

        // Find TODOs and FIXMEs
        debt_items.extend(find_todos_and_fixmes_with_suppression(
            source,
            path,
            Some(&suppression_context),
        ));

        // Find code smells
        debt_items.extend(find_code_smells_with_suppression(
            source,
            path,
            Some(&suppression_context),
        ));

        // Analyze function-level smells
        for func in functions {
            let smells = analyze_function_smells(func, 0); // 0 for param count (not available from tree-sitter easily)
            for smell in smells {
                let debt_item = smell.to_debt_item();
                if !suppression_context.is_suppressed(debt_item.line, &debt_item.debt_type) {
                    debt_items.push(debt_item);
                }
            }
        }

        // Analyze module-level smells
        let lines = source.lines().count();
        let smells = analyze_module_smells(path, lines);
        for smell in smells {
            let debt_item = smell.to_debt_item();
            if !suppression_context.is_suppressed(debt_item.line, &debt_item.debt_type) {
                debt_items.push(debt_item);
            }
        }

        // Check for high complexity
        for func in functions {
            if func.is_complex(self.complexity_threshold) {
                let debt_item = DebtItem {
                    id: format!("complexity-{}-{}", path.display(), func.line),
                    debt_type: DebtType::Complexity,
                    priority: if func.cyclomatic > 20 || func.cognitive > 20 {
                        Priority::High
                    } else {
                        Priority::Medium
                    },
                    file: path.to_path_buf(),
                    line: func.line,
                    message: format!(
                        "Function '{}' has high complexity (cyclomatic: {}, cognitive: {})",
                        func.name, func.cyclomatic, func.cognitive
                    ),
                    context: None,
                };

                // Check if this debt item is suppressed
                if !suppression_context.is_suppressed(func.line, &debt_item.debt_type) {
                    debt_items.push(debt_item);
                }
            }
        }

        debt_items
    }
}

impl Analyzer for JavaScriptAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let mut analyzer = Self {
            parser: Parser::new(),
            language: self.language,
            complexity_threshold: self.complexity_threshold,
        };

        // Set the appropriate language
        match self.language {
            Language::JavaScript => {
                analyzer
                    .parser
                    .set_language(&tree_sitter_javascript::LANGUAGE.into())
                    .context("Failed to set JavaScript language")?;
            }
            Language::TypeScript => {
                analyzer
                    .parser
                    .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                    .context("Failed to set TypeScript language")?;
            }
            _ => unreachable!(),
        }

        let tree = analyzer.parse_tree(content)?;

        match self.language {
            Language::JavaScript => Ok(Ast::JavaScript(JavaScriptAst {
                tree,
                source: content.to_string(),
                path,
            })),
            Language::TypeScript => Ok(Ast::TypeScript(TypeScriptAst {
                tree,
                source: content.to_string(),
                path,
            })),
            _ => unreachable!(),
        }
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::JavaScript(js_ast) => {
                let functions = self.extract_functions(&js_ast.tree, &js_ast.source, &js_ast.path);
                let dependencies = self.extract_dependencies(&js_ast.tree, &js_ast.source);
                let debt_items =
                    self.create_debt_items(&js_ast.tree, &js_ast.source, &js_ast.path, &functions);

                FileMetrics {
                    path: js_ast.path.clone(),
                    language: Language::JavaScript,
                    complexity: ComplexityMetrics { functions },
                    debt_items,
                    dependencies,
                    duplications: vec![],
                }
            }
            Ast::TypeScript(ts_ast) => {
                let functions = self.extract_functions(&ts_ast.tree, &ts_ast.source, &ts_ast.path);
                let dependencies = self.extract_dependencies(&ts_ast.tree, &ts_ast.source);
                let debt_items =
                    self.create_debt_items(&ts_ast.tree, &ts_ast.source, &ts_ast.path, &functions);

                FileMetrics {
                    path: ts_ast.path.clone(),
                    language: Language::TypeScript,
                    complexity: ComplexityMetrics { functions },
                    debt_items,
                    dependencies,
                    duplications: vec![],
                }
            }
            _ => FileMetrics {
                path: PathBuf::new(),
                language: self.language,
                complexity: ComplexityMetrics { functions: vec![] },
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
            },
        }
    }

    fn language(&self) -> Language {
        self.language
    }
}

fn report_unclosed_blocks(suppression_context: &SuppressionContext) {
    for unclosed in &suppression_context.unclosed_blocks {
        eprintln!(
            "Warning: Unclosed suppression block starting at line {} in {}",
            unclosed.start_line,
            unclosed.file.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_javascript_analyzer_creation() {
        let analyzer = JavaScriptAnalyzer::new_javascript().unwrap();
        assert_eq!(analyzer.language(), Language::JavaScript);
    }

    #[test]
    fn test_typescript_analyzer_creation() {
        let analyzer = JavaScriptAnalyzer::new_typescript().unwrap();
        assert_eq!(analyzer.language(), Language::TypeScript);
    }

    #[test]
    fn test_parse_simple_javascript() {
        let analyzer = JavaScriptAnalyzer::new_javascript().unwrap();
        let content = r#"
function hello() {
    console.log("Hello, World!");
}
"#;
        let ast = analyzer.parse(content, PathBuf::from("test.js")).unwrap();
        assert!(matches!(ast, Ast::JavaScript(_)));
    }

    #[test]
    fn test_parse_simple_typescript() {
        let analyzer = JavaScriptAnalyzer::new_typescript().unwrap();
        let content = r#"
function hello(name: string): void {
    console.log(`Hello, ${name}!`);
}
"#;
        let ast = analyzer.parse(content, PathBuf::from("test.ts")).unwrap();
        assert!(matches!(ast, Ast::TypeScript(_)));
    }

    #[test]
    fn test_cyclomatic_complexity_calculation() {
        let analyzer = JavaScriptAnalyzer::new_javascript().unwrap();
        let content = r#"
function complexFunction(x) {
    if (x > 0) {
        if (x < 10) {
            return x * 2;
        } else if (x < 20) {
            return x * 3;
        }
    } else {
        while (x < 0) {
            x++;
        }
    }
    return x;
}
"#;
        let ast = analyzer.parse(content, PathBuf::from("test.js")).unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.complexity.functions.len(), 1);
        let func = &metrics.complexity.functions[0];
        assert!(func.cyclomatic > 1);
    }

    #[test]
    fn test_arrow_function_detection() {
        let analyzer = JavaScriptAnalyzer::new_javascript().unwrap();
        let content = r#"
const add = (a, b) => a + b;
const multiply = (a, b) => {
    return a * b;
};
"#;
        let ast = analyzer.parse(content, PathBuf::from("test.js")).unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.complexity.functions.len(), 2);
    }

    #[test]
    fn test_import_dependency_extraction() {
        let analyzer = JavaScriptAnalyzer::new_javascript().unwrap();
        let content = r#"
import React from 'react';
import { useState } from 'react';
const fs = require('fs');
"#;
        let ast = analyzer.parse(content, PathBuf::from("test.js")).unwrap();
        let metrics = analyzer.analyze(&ast);

        assert!(metrics.dependencies.len() >= 2);
        assert!(metrics.dependencies.iter().any(|d| d.name == "react"));
        assert!(metrics.dependencies.iter().any(|d| d.name == "fs"));
    }

    #[test]
    fn test_todo_detection() {
        let analyzer = JavaScriptAnalyzer::new_javascript().unwrap();
        let content = r#"
function test() {
    // TODO: Implement this function
    // FIXME: This is broken
    return null;
}
"#;
        let ast = analyzer.parse(content, PathBuf::from("test.js")).unwrap();
        let metrics = analyzer.analyze(&ast);

        assert!(metrics
            .debt_items
            .iter()
            .any(|item| item.debt_type == DebtType::Todo));
        assert!(metrics
            .debt_items
            .iter()
            .any(|item| item.debt_type == DebtType::Fixme));
    }

    #[test]
    fn test_jsx_parsing() {
        let analyzer = JavaScriptAnalyzer::new_javascript().unwrap();
        let content = r#"
function Component() {
    return <div>Hello</div>;
}
"#;
        // JSX parsing might require additional configuration
        // This test ensures the parser doesn't crash on JSX
        let result = analyzer.parse(content, PathBuf::from("test.jsx"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_typescript_interface_parsing() {
        let analyzer = JavaScriptAnalyzer::new_typescript().unwrap();
        let content = r#"
interface User {
    name: string;
    age: number;
}

function greet(user: User): string {
    return `Hello, ${user.name}`;
}
"#;
        let ast = analyzer.parse(content, PathBuf::from("test.ts")).unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.complexity.functions.len(), 1);
        assert_eq!(metrics.complexity.functions[0].name, "greet");
    }

    #[test]
    fn test_async_await_complexity() {
        let analyzer = JavaScriptAnalyzer::new_javascript().unwrap();
        let content = r#"
async function fetchData(url) {
    try {
        const response = await fetch(url);
        if (response.ok) {
            return await response.json();
        } else {
            throw new Error('Failed to fetch');
        }
    } catch (error) {
        console.error(error);
        return null;
    }
}
"#;
        let ast = analyzer.parse(content, PathBuf::from("test.js")).unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.complexity.functions.len(), 1);
        let func = &metrics.complexity.functions[0];
        assert!(func.cyclomatic > 1); // Should account for if and catch
    }
}

//! Dependency extraction for TypeScript/JavaScript
//!
//! Extracts import/export/require dependencies from JS/TS files.

use crate::analyzers::typescript::parser::node_text;
use crate::core::ast::TypeScriptAst;
use crate::core::{Dependency, DependencyKind};
use tree_sitter::Node;

/// Extract dependencies from a TypeScript/JavaScript AST
pub fn extract_dependencies(ast: &TypeScriptAst) -> Vec<Dependency> {
    let mut deps = Vec::new();
    let root = ast.tree.root_node();

    extract_dependencies_recursive(&root, ast, &mut deps);

    deps
}

fn extract_dependencies_recursive(node: &Node, ast: &TypeScriptAst, deps: &mut Vec<Dependency>) {
    match node.kind() {
        "import_statement" => {
            if let Some(dep) = extract_import(node, ast) {
                deps.push(dep);
            }
        }
        "export_statement" => {
            // Check for re-exports: export { x } from 'module'
            if let Some(source) = node.child_by_field_name("source") {
                let specifier = extract_string_value(&source, ast);
                deps.push(Dependency {
                    name: specifier,
                    kind: DependencyKind::Import, // Re-exports are still imports
                });
            }
        }
        "call_expression" => {
            // Check for require() calls
            if let Some(dep) = extract_require(node, ast) {
                deps.push(dep);
            }
            // Check for dynamic import()
            if let Some(dep) = extract_dynamic_import(node, ast) {
                deps.push(dep);
            }
        }
        _ => {}
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_dependencies_recursive(&child, ast, deps);
    }
}

fn extract_import(node: &Node, ast: &TypeScriptAst) -> Option<Dependency> {
    // import x from 'module'
    // import { x } from 'module'
    // import 'module'
    if let Some(source) = node.child_by_field_name("source") {
        let specifier = extract_string_value(&source, ast);
        return Some(Dependency {
            name: specifier,
            kind: DependencyKind::Import,
        });
    }
    None
}

fn extract_require(node: &Node, ast: &TypeScriptAst) -> Option<Dependency> {
    // const x = require('module')
    if let Some(func) = node.child_by_field_name("function") {
        let func_text = node_text(&func, &ast.source);
        if func_text == "require" {
            if let Some(args) = node.child_by_field_name("arguments") {
                let mut cursor = args.walk();
                for arg in args.children(&mut cursor) {
                    if arg.kind() == "string" || arg.kind() == "template_string" {
                        let specifier = extract_string_value(&arg, ast);
                        return Some(Dependency {
                            name: specifier,
                            kind: DependencyKind::Import,
                        });
                    }
                }
            }
        }
    }
    None
}

fn extract_dynamic_import(node: &Node, ast: &TypeScriptAst) -> Option<Dependency> {
    // import('module')
    // The function is literally "import"
    if let Some(func) = node.child_by_field_name("function") {
        if func.kind() == "import" {
            if let Some(args) = node.child_by_field_name("arguments") {
                let mut cursor = args.walk();
                for arg in args.children(&mut cursor) {
                    if arg.kind() == "string" || arg.kind() == "template_string" {
                        let specifier = extract_string_value(&arg, ast);
                        return Some(Dependency {
                            name: specifier,
                            kind: DependencyKind::Import,
                        });
                    }
                }
            }
        }
    }
    None
}

fn extract_string_value(node: &Node, ast: &TypeScriptAst) -> String {
    let text = node_text(node, &ast.source);
    // Remove quotes
    text.trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_extract_es_import() {
        let source = r#"
import React from 'react';
import { useState, useEffect } from 'react';
import './styles.css';
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let deps = extract_dependencies(&ast);

        assert!(!deps.is_empty());
        assert!(deps.iter().any(|d| d.name == "react"));
        assert!(deps.iter().any(|d| d.name == "./styles.css"));
    }

    #[test]
    fn test_extract_require() {
        let source = r#"
const fs = require('fs');
const path = require('path');
"#;
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let deps = extract_dependencies(&ast);

        assert!(!deps.is_empty());
        assert!(deps.iter().any(|d| d.name == "fs"));
        assert!(deps.iter().any(|d| d.name == "path"));
    }

    #[test]
    fn test_extract_typescript_imports() {
        let source = r#"
import type { User } from './types';
import { api } from '@company/sdk';
"#;
        let path = PathBuf::from("test.ts");
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();

        let deps = extract_dependencies(&ast);

        assert!(!deps.is_empty());
        assert!(deps.iter().any(|d| d.name == "./types"));
        assert!(deps.iter().any(|d| d.name == "@company/sdk"));
    }

    #[test]
    fn test_extract_re_export() {
        let source = "export { foo, bar } from './utils';";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let deps = extract_dependencies(&ast);

        assert!(!deps.is_empty());
        assert!(deps.iter().any(|d| d.name == "./utils"));
    }
}

// Organization pattern detection for JavaScript/TypeScript

use super::{get_node_text, SourceLocation};
use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

#[derive(Debug, Clone)]
pub enum OrganizationAntiPattern {
    CallbackHell {
        location: SourceLocation,
        depth: usize,
    },
    CircularDependency {
        location: SourceLocation,
        modules: Vec<String>,
    },
    TypeSafetyViolation {
        location: SourceLocation,
        violation_type: String,
        count: usize,
    },
    GlobalNamespacePollution {
        location: SourceLocation,
        globals: Vec<String>,
    },
    MixedModuleSystems {
        location: SourceLocation,
        commonjs_count: usize,
        es6_count: usize,
    },
    ComplexTypeGymnastics {
        location: SourceLocation,
        complexity: usize,
    },
}

impl OrganizationAntiPattern {
    pub fn to_debt_item(&self, path: &Path) -> DebtItem {
        let (message, priority) = match self {
            Self::CallbackHell { depth, .. } => (
                format!("Callback hell detected (depth: {})", depth),
                if *depth > 3 {
                    Priority::High
                } else {
                    Priority::Medium
                },
            ),
            Self::CircularDependency { modules, .. } => (
                format!("Circular dependency detected: {}", modules.join(" -> ")),
                Priority::High,
            ),
            Self::TypeSafetyViolation {
                violation_type,
                count,
                ..
            } => (
                format!("{} (found {} occurrences)", violation_type, count),
                Priority::Medium,
            ),
            Self::GlobalNamespacePollution { globals, .. } => (
                format!(
                    "Global namespace pollution: {} global variables",
                    globals.len()
                ),
                if globals.len() > 5 {
                    Priority::High
                } else {
                    Priority::Medium
                },
            ),
            Self::MixedModuleSystems {
                commonjs_count,
                es6_count,
                ..
            } => (
                format!(
                    "Mixed module systems: {} CommonJS, {} ES6",
                    commonjs_count, es6_count
                ),
                Priority::Low,
            ),
            Self::ComplexTypeGymnastics { complexity, .. } => (
                format!(
                    "Overly complex TypeScript type (complexity: {})",
                    complexity
                ),
                Priority::Low,
            ),
        };

        let location = match self {
            Self::CallbackHell { location, .. }
            | Self::CircularDependency { location, .. }
            | Self::TypeSafetyViolation { location, .. }
            | Self::GlobalNamespacePollution { location, .. }
            | Self::MixedModuleSystems { location, .. }
            | Self::ComplexTypeGymnastics { location, .. } => location,
        };

        DebtItem {
            id: format!("org-{}-{}", path.display(), location.line),
            debt_type: DebtType::CodeOrganization,
            priority,
            file: path.to_path_buf(),
            line: location.line,
            column: location.column,
            message,
            context: None,
        }
    }
}

pub fn detect_organization_patterns(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<OrganizationAntiPattern>,
) {
    detect_callback_hell(root, source, language, patterns);
    detect_any_type_overuse(root, source, language, patterns);
    detect_global_pollution(root, source, language, patterns);
    detect_mixed_modules(root, source, language, patterns);
    detect_complex_types(root, source, language, patterns);
}

fn detect_callback_hell(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<OrganizationAntiPattern>,
) {
    // Detect deeply nested callbacks
    let query_str = r#"
    (arrow_function
      body: (block_statement
        (expression_statement
          (call_expression
            arguments: (arguments
              (arrow_function
                body: (block_statement
                  (expression_statement
                    (call_expression
                      arguments: (arguments
                        (arrow_function) @deep_callback
                      )
                    )
                  )
                )
              ) @mid_callback
            )
          )
        )
      )
    ) @top_callback
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(top) = match_.captures.iter().find(|c| c.index == 2) {
                let location = SourceLocation::from_node(top.node);
                patterns.push(OrganizationAntiPattern::CallbackHell { location, depth: 3 });
            }
        }
    }
}

fn detect_any_type_overuse(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<OrganizationAntiPattern>,
) {
    // TypeScript-specific: detect excessive use of 'any' type
    let query_str = r#"
    (type_annotation
      (any_type) @any
    )
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());
        let mut any_count = 0;
        let mut first_location = None;

        while let Some(match_) = matches.next() {
            any_count += 1;
            if first_location.is_none() {
                if let Some(node) = match_.captures.first() {
                    first_location = Some(SourceLocation::from_node(node.node));
                }
            }
        }

        if any_count > 5 {
            let location = first_location.unwrap_or_else(|| SourceLocation::from_node(root));

            patterns.push(OrganizationAntiPattern::TypeSafetyViolation {
                location,
                violation_type:
                    "Excessive use of 'any' type - consider using specific types or 'unknown'"
                        .to_string(),
                count: any_count,
            });
        }
    }
}

fn detect_global_pollution(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<OrganizationAntiPattern>,
) {
    // Detect global variable declarations at the top level
    let query_str = r#"
    (program
      (variable_declaration
        (variable_declarator
          name: (identifier) @global
        )
      ) @declaration
    )
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        let mut globals = Vec::new();
        while let Some(match_) = matches.next() {
            if let Some(global) = match_.captures.iter().find(|c| c.index == 0) {
                let name = get_node_text(global.node, source);
                globals.push(name.to_string());
            }
        }

        if globals.len() > 3 {
            let location = SourceLocation::from_node(root);
            patterns.push(OrganizationAntiPattern::GlobalNamespacePollution { location, globals });
        }
    }
}

fn detect_mixed_modules(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<OrganizationAntiPattern>,
) {
    // Detect CommonJS require() calls
    let require_query = r#"
    (call_expression
      function: (identifier) @func (#eq? @func "require")
    ) @require_call
    "#;

    // Detect ES6 imports
    let import_query = r#"
    (import_statement) @import
    "#;

    let mut commonjs_count = 0;
    let mut es6_count = 0;

    if let Ok(query) = Query::new(language, require_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());
        while matches.next().is_some() {
            commonjs_count += 1;
        }
    }

    if let Ok(query) = Query::new(language, import_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());
        while matches.next().is_some() {
            es6_count += 1;
        }
    }

    if commonjs_count > 0 && es6_count > 0 {
        let location = SourceLocation::from_node(root);
        patterns.push(OrganizationAntiPattern::MixedModuleSystems {
            location,
            commonjs_count,
            es6_count,
        });
    }
}

fn detect_complex_types(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<OrganizationAntiPattern>,
) {
    // TypeScript-specific: detect overly complex type definitions
    let query_str = r#"
    (type_alias_declaration
      value: (_) @type_def
    ) @type_alias
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(type_def) = match_.captures.iter().find(|c| c.index == 0) {
                let complexity = calculate_type_complexity(type_def.node);

                if complexity > 10 {
                    let location = SourceLocation::from_node(type_def.node);
                    patterns.push(OrganizationAntiPattern::ComplexTypeGymnastics {
                        location,
                        complexity,
                    });
                }
            }
        }
    }
}

fn calculate_type_complexity(node: Node) -> usize {
    // Simple heuristic: count the number of child nodes
    let mut complexity = 1;
    let mut cursor = node.walk();

    loop {
        if cursor.goto_first_child() || cursor.goto_next_sibling() {
            complexity += 1;
        } else {
            // Go back up and try next sibling
            loop {
                if !cursor.goto_parent() {
                    return complexity;
                }
                if cursor.goto_next_sibling() {
                    complexity += 1;
                    break;
                }
            }
        }
    }
}

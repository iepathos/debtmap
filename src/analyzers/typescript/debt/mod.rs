//! Technical debt detection for TypeScript/JavaScript
//!
//! Detects various forms of technical debt specific to JS/TS codebases.

pub mod async_patterns;
pub mod complexity_items;
pub mod type_safety;

use crate::analyzers::typescript::types::JsFunctionMetrics;
use crate::core::ast::TypeScriptAst;
use crate::core::{DebtItem, FunctionMetrics};

use async_patterns::detect_async_debt;
use complexity_items::detect_complexity_debt;
use type_safety::detect_type_safety_debt;

/// Create debt items from TypeScript/JavaScript analysis
pub fn create_debt_items(
    ast: &TypeScriptAst,
    threshold: u32,
    functions: &[FunctionMetrics],
    js_functions: &[JsFunctionMetrics],
) -> Vec<DebtItem> {
    let mut items = Vec::new();

    // Phase 1: Complexity-based debt
    items.extend(detect_complexity_debt(&ast.path, threshold, functions));

    // Phase 2: Async pattern debt
    items.extend(detect_async_debt(ast, js_functions));

    // Phase 3: TypeScript-specific debt (type safety)
    if ast.language_variant.has_types() {
        items.extend(detect_type_safety_debt(ast));
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    #[test]
    fn test_create_debt_items_empty() {
        let source = "function simple() { return 1; }";
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();

        let items = create_debt_items(&ast, 10, &[], &[]);

        // Simple function should have no debt
        assert!(items.is_empty());
    }
}

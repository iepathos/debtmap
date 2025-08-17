use serde::{Deserialize, Serialize};
use syn::spanned::Spanned;
use syn::{Expr, Item, Stmt, Type};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: usize,
    pub column: Option<usize>,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
    pub confidence: LocationConfidence,
}

impl Default for SourceLocation {
    fn default() -> Self {
        Self {
            line: 1,
            column: None,
            end_line: None,
            end_column: None,
            confidence: LocationConfidence::Unavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LocationConfidence {
    Exact,       // Precise syn::Span information
    Approximate, // Estimated from surrounding context
    Unavailable, // No location information available
}

/// Common utilities for all detectors to extract source locations
#[derive(Clone)]
pub struct UnifiedLocationExtractor {
    source_lines: Vec<String>,
}

impl UnifiedLocationExtractor {
    pub fn new(source_content: &str) -> Self {
        Self {
            source_lines: source_content.lines().map(String::from).collect(),
        }
    }

    /// Extract location from any syn AST node that implements Spanned
    pub fn extract_location<T: Spanned>(&self, node: &T) -> SourceLocation {
        let span = node.span();
        self.span_to_location(span)
            .unwrap_or_else(SourceLocation::default)
    }

    /// Extract location from item definitions (structs, enums, functions)
    pub fn extract_item_location(&self, item: &Item) -> SourceLocation {
        self.extract_location(item)
    }

    /// Extract location from expressions
    pub fn extract_expr_location(&self, expr: &Expr) -> SourceLocation {
        self.extract_location(expr)
    }

    /// Extract location from type definitions
    pub fn extract_type_location(&self, ty: &Type) -> SourceLocation {
        self.extract_location(ty)
    }

    /// Extract location from statements
    pub fn extract_stmt_location(&self, stmt: &Stmt) -> SourceLocation {
        self.extract_location(stmt)
    }

    fn span_to_location(&self, span: proc_macro2::Span) -> Option<SourceLocation> {
        // syn::Span provides line and column information
        let start = span.start();
        let end = span.end();

        // syn line numbers are 1-based
        let line = start.line;
        let column = Some(start.column);
        let end_line = if end.line != start.line {
            Some(end.line)
        } else {
            None
        };
        let end_column = if end.line != start.line || end.column != start.column {
            Some(end.column)
        } else {
            None
        };

        Some(SourceLocation {
            line,
            column,
            end_line,
            end_column,
            confidence: LocationConfidence::Exact,
        })
    }
}

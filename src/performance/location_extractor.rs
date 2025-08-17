use crate::common::{LocationConfidence, SourceLocation};
use syn::spanned::Spanned;
use syn::{Expr, Stmt};

pub struct LocationExtractor {
    source_lines: Vec<String>,
}

impl LocationExtractor {
    pub fn new(source_content: &str) -> Self {
        Self {
            source_lines: source_content.lines().map(String::from).collect(),
        }
    }

    /// Extract location from any syn AST node that implements Spanned
    pub fn extract_location<T: Spanned>(&self, node: &T) -> SourceLocation {
        let span = node.span();

        match self.span_to_location(span) {
            Some(location) => location,
            None => SourceLocation {
                line: 1,
                column: None,
                end_line: None,
                end_column: None,
                confidence: LocationConfidence::Unavailable,
            },
        }
    }

    /// Extract location from expression with high precision
    pub fn extract_expr_location(&self, expr: &Expr) -> SourceLocation {
        let span = expr.span();
        self.span_to_location(span)
            .unwrap_or_else(|| self.fallback_location_from_context(expr))
    }

    /// Extract location from statement
    pub fn extract_stmt_location(&self, stmt: &Stmt) -> SourceLocation {
        let span = stmt.span();
        self.span_to_location(span).unwrap_or(SourceLocation {
            line: 1,
            column: None,
            end_line: None,
            end_column: None,
            confidence: LocationConfidence::Unavailable,
        })
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

    fn fallback_location_from_context(&self, _expr: &Expr) -> SourceLocation {
        // If span information is unavailable, try to estimate from expression type
        // This is a fallback for edge cases where syn spans are not available
        SourceLocation {
            line: 1, // Conservative fallback
            column: None,
            end_line: None,
            end_column: None,
            confidence: LocationConfidence::Unavailable,
        }
    }
}

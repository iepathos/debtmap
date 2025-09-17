/// Macro expansion and parsing functionality for call graph extraction
use std::collections::HashMap;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Expr, ExprMacro};

/// Statistics for macro expansion
#[derive(Debug, Default)]
pub struct MacroExpansionStats {
    pub total_macros: usize,
    pub successfully_parsed: usize,
    pub failed_macros: HashMap<String, usize>,
}

/// Configuration for macro handling
#[derive(Debug, Clone, Default)]
pub struct MacroHandlingConfig {
    pub verbose_warnings: bool,
    pub show_statistics: bool,
}

/// Type classification for macros to determine parsing strategy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MacroType {
    Collection,
    Formatting,
    Assertion,
    Logging,
    Generic,
}

/// Result of collection macro parsing
#[derive(Debug)]
pub enum CollectionParseResult {
    Success(Vec<Expr>),
    MapSuccess(Vec<Expr>),
    Failed,
}

/// Handles macro expansion and parsing for call graph extraction
pub struct MacroExpander {
    pub stats: MacroExpansionStats,
    pub config: MacroHandlingConfig,
}

impl Default for MacroExpander {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroExpander {
    pub fn new() -> Self {
        Self {
            stats: MacroExpansionStats::default(),
            config: MacroHandlingConfig::default(),
        }
    }

    pub fn with_config(config: MacroHandlingConfig) -> Self {
        Self {
            stats: MacroExpansionStats::default(),
            config,
        }
    }

    /// Classify a macro based on its name to determine parsing strategy
    pub fn classify_macro_type(macro_name: &str) -> MacroType {
        match macro_name {
            "vec" | "hashmap" | "btreemap" | "hashset" | "btreeset" | "maplit" => {
                MacroType::Collection
            }
            "format" | "format_args" | "write" | "writeln" | "print" | "println" | "eprint"
            | "eprintln" => MacroType::Formatting,
            "assert" | "assert_eq" | "assert_ne" | "debug_assert" | "debug_assert_eq"
            | "debug_assert_ne" => MacroType::Assertion,
            "log" | "trace" | "debug" | "info" | "warn" | "error" | "tracing" => MacroType::Logging,
            _ => MacroType::Generic,
        }
    }

    /// Extract macro name from a syn path
    pub fn extract_macro_name(path: &syn::Path) -> String {
        path.segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }

    /// Handle macro expression parsing
    pub fn handle_macro_expression(&mut self, expr_macro: &ExprMacro) -> Vec<Expr> {
        let macro_name = Self::extract_macro_name(&expr_macro.mac.path);
        self.stats.total_macros += 1;

        let result = self.dispatch_macro_parsing(&expr_macro.mac.tokens, &macro_name);

        if result.is_empty() {
            self.log_unexpandable_macro(&macro_name);
        } else {
            self.stats.successfully_parsed += 1;
        }

        result
    }

    /// Dispatch to appropriate parsing method based on macro type
    fn dispatch_macro_parsing(
        &mut self,
        tokens: &proc_macro2::TokenStream,
        macro_name: &str,
    ) -> Vec<Expr> {
        let macro_type = Self::classify_macro_type(macro_name);

        match macro_type {
            MacroType::Collection => self.parse_collection_macro(tokens, macro_name),
            MacroType::Formatting => self.parse_format_macro(tokens, macro_name),
            MacroType::Assertion => self.parse_assert_macro(tokens, macro_name),
            MacroType::Logging => self.parse_logging_macro(tokens, macro_name),
            MacroType::Generic => self.parse_generic_macro(tokens, macro_name),
        }
    }

    /// Parse a generic macro that doesn't fit specific categories
    fn parse_generic_macro(
        &mut self,
        tokens: &proc_macro2::TokenStream,
        macro_name: &str,
    ) -> Vec<Expr> {
        if self.config.verbose_warnings {
            eprintln!("Attempting generic parse for macro: {}", macro_name);
        }
        self.parse_comma_separated_exprs(tokens)
            .unwrap_or_else(|_| Vec::new())
    }

    /// Parse collection-type macros (vec!, hashmap!, etc.)
    fn parse_collection_macro(
        &mut self,
        tokens: &proc_macro2::TokenStream,
        macro_name: &str,
    ) -> Vec<Expr> {
        // For vec! macro, try to parse directly as comma-separated expressions
        if macro_name == "vec" {
            // Try parsing the token stream directly as expressions
            if let Ok(exprs) = self.parse_comma_separated_exprs(tokens) {
                return exprs;
            }
        }

        match self.try_parse_collection(tokens, macro_name) {
            CollectionParseResult::Success(exprs) => exprs,
            CollectionParseResult::MapSuccess(exprs) => exprs,
            CollectionParseResult::Failed => {
                if self.config.verbose_warnings {
                    eprintln!("Failed to parse collection macro: {}", macro_name);
                }
                Vec::new()
            }
        }
    }

    /// Try to parse a collection macro with different strategies
    fn try_parse_collection(
        &mut self,
        tokens: &proc_macro2::TokenStream,
        macro_name: &str,
    ) -> CollectionParseResult {
        // Try bracketed expressions first
        if let Ok(exprs) = self.parse_bracketed_exprs(tokens) {
            return CollectionParseResult::Success(exprs);
        }

        // Try map-specific parsing
        if Self::is_map_macro(macro_name) {
            if let Some(exprs) = Self::parse_map_tokens(tokens) {
                return CollectionParseResult::MapSuccess(exprs);
            }
        }

        // Try braced expressions
        if let Ok(exprs) = self.parse_braced_exprs(tokens) {
            return CollectionParseResult::Success(exprs);
        }

        CollectionParseResult::Failed
    }

    /// Check if macro is a map-type collection
    fn is_map_macro(macro_name: &str) -> bool {
        matches!(macro_name, "hashmap" | "btreemap" | "maplit")
    }

    /// Parse map-specific tokens
    fn parse_map_tokens(tokens: &proc_macro2::TokenStream) -> Option<Vec<Expr>> {
        let content = tokens.to_string();
        let mut exprs = Vec::new();

        // Parse key-value pairs
        for pair in content.split(',') {
            let pair = pair.trim();
            if !pair.is_empty() {
                exprs.extend(Self::parse_key_value_pair(pair));
            }
        }

        if exprs.is_empty() {
            None
        } else {
            Some(exprs)
        }
    }

    /// Parse format-type macros
    fn parse_format_macro(
        &mut self,
        tokens: &proc_macro2::TokenStream,
        macro_name: &str,
    ) -> Vec<Expr> {
        // Try to extract expressions from format strings
        if let Ok(expr) = Self::try_parse_single_expr(tokens) {
            vec![expr]
        } else {
            self.parse_comma_separated_exprs(tokens)
                .unwrap_or_else(|_| {
                    if self.config.verbose_warnings {
                        eprintln!("Failed to parse format macro: {}", macro_name);
                    }
                    Vec::new()
                })
        }
    }

    /// Try to parse a single expression
    fn try_parse_single_expr(tokens: &proc_macro2::TokenStream) -> syn::Result<Expr> {
        syn::parse2::<Expr>(tokens.clone())
    }

    /// Parse assertion macros
    fn parse_assert_macro(
        &mut self,
        tokens: &proc_macro2::TokenStream,
        macro_name: &str,
    ) -> Vec<Expr> {
        // Assertions typically have one or more expressions
        self.parse_comma_separated_exprs(tokens)
            .unwrap_or_else(|_| {
                if self.config.verbose_warnings {
                    eprintln!("Failed to parse assert macro: {}", macro_name);
                }
                Vec::new()
            })
    }

    /// Parse logging macros
    fn parse_logging_macro(
        &mut self,
        tokens: &proc_macro2::TokenStream,
        macro_name: &str,
    ) -> Vec<Expr> {
        self.parse_comma_separated_exprs(tokens)
            .unwrap_or_else(|_| {
                if self.config.verbose_warnings {
                    eprintln!("Failed to parse logging macro: {}", macro_name);
                }
                Vec::new()
            })
    }

    /// Parse bracketed expressions [expr1, expr2, ...]
    pub fn parse_bracketed_exprs(
        &self,
        tokens: &proc_macro2::TokenStream,
    ) -> syn::Result<Vec<Expr>> {
        let content = tokens.to_string();

        if content.starts_with('[') && content.ends_with(']') {
            let inner = &content[1..content.len() - 1];

            if inner.trim().is_empty() {
                return Ok(Vec::new());
            }

            let parser = Punctuated::<Expr, Comma>::parse_separated_nonempty;
            let token_stream: proc_macro2::TokenStream = inner.parse().unwrap_or_default();

            match parser.parse2(token_stream) {
                Ok(punctuated) => Ok(punctuated.into_iter().collect()),
                Err(_) => {
                    // Try parsing as single expression
                    if let Ok(expr) = syn::parse_str::<Expr>(inner) {
                        Ok(vec![expr])
                    } else {
                        Err(syn::Error::new(
                            proc_macro2::Span::call_site(),
                            "Failed to parse bracketed expressions",
                        ))
                    }
                }
            }
        } else {
            Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Not a bracketed expression",
            ))
        }
    }

    /// Parse braced expressions {expr1, expr2, ...}
    pub fn parse_braced_exprs(&self, tokens: &proc_macro2::TokenStream) -> syn::Result<Vec<Expr>> {
        let content = tokens.to_string();
        Self::validate_braced_format(&content)?;
        self.parse_validated_braced_content(&content)
    }

    /// Validate that content has proper braced format
    fn validate_braced_format(content: &str) -> syn::Result<()> {
        if content.starts_with('{') && content.ends_with('}') {
            Ok(())
        } else {
            Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Not a braced expression",
            ))
        }
    }

    /// Parse the content inside validated braces
    fn parse_validated_braced_content(&self, content: &str) -> syn::Result<Vec<Expr>> {
        let inner = Self::extract_inner_content(content);

        if inner.trim().is_empty() {
            return Ok(Vec::new());
        }

        self.try_parse_braced_expressions(content, inner)
    }

    /// Extract content between braces
    fn extract_inner_content(content: &str) -> &str {
        &content[1..content.len() - 1]
    }

    /// Try different parsing strategies for braced expressions
    fn try_parse_braced_expressions(&self, full_content: &str, inner: &str) -> syn::Result<Vec<Expr>> {
        // Strategy 1: Try special braced content extraction
        if let Ok(expr) = self.try_braced_content_extraction(full_content) {
            return Ok(vec![expr]);
        }

        // Strategy 2: Try comma-separated parsing
        if let Ok(exprs) = Self::try_comma_separated_parsing(inner) {
            return Ok(exprs);
        }

        // Strategy 3: Try single expression parsing
        Self::try_single_expression_parsing(inner)
    }

    /// Try parsing using braced content extraction
    fn try_braced_content_extraction(&self, content: &str) -> syn::Result<Expr> {
        if Self::is_braced_content(content) {
            let inner_content = Self::extract_braced_inner(content);
            if let Some(expr) = Self::parse_expression_from_str(inner_content) {
                return Ok(expr);
            }
        }
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Braced content extraction failed",
        ))
    }

    /// Try parsing as comma-separated expressions
    fn try_comma_separated_parsing(inner: &str) -> syn::Result<Vec<Expr>> {
        let parser = Punctuated::<Expr, Comma>::parse_separated_nonempty;
        let token_stream: proc_macro2::TokenStream = inner.parse().unwrap_or_default();
        parser.parse2(token_stream).map(|punctuated| punctuated.into_iter().collect())
    }

    /// Try parsing as a single expression
    fn try_single_expression_parsing(inner: &str) -> syn::Result<Vec<Expr>> {
        syn::parse_str::<Expr>(inner)
            .map(|expr| vec![expr])
            .map_err(|_| {
                syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "Failed to parse braced expressions",
                )
            })
    }

    /// Parse comma-separated expressions
    pub fn parse_comma_separated_exprs(
        &self,
        tokens: &proc_macro2::TokenStream,
    ) -> syn::Result<Vec<Expr>> {
        let parser = Punctuated::<Expr, Comma>::parse_separated_nonempty;
        parser
            .parse2(tokens.clone())
            .map(|punctuated| punctuated.into_iter().collect())
    }

    /// Check if content is braced
    fn is_braced_content(content: &str) -> bool {
        content.starts_with('{') && content.ends_with('}')
    }

    /// Extract inner content from braces
    fn extract_braced_inner(content: &str) -> &str {
        &content[1..content.len() - 1]
    }

    /// Parse expression from string
    fn parse_expression_from_str(expr_str: &str) -> Option<Expr> {
        syn::parse_str::<Expr>(expr_str).ok()
    }

    /// Parse key-value pair for maps
    fn parse_key_value_pair(pair: &str) -> Vec<Expr> {
        let mut exprs = Vec::new();

        // Split on => for map entries
        if let Some(arrow_pos) = pair.find("=>") {
            let key = pair[..arrow_pos].trim();
            let value = pair[arrow_pos + 2..].trim();

            if let Ok(key_expr) = syn::parse_str::<Expr>(key) {
                exprs.push(key_expr);
            }
            if let Ok(value_expr) = syn::parse_str::<Expr>(value) {
                exprs.push(value_expr);
            }
        } else if let Ok(expr) = syn::parse_str::<Expr>(pair) {
            exprs.push(expr);
        }

        exprs
    }

    /// Log unexpandable macro for statistics
    fn log_unexpandable_macro(&mut self, macro_name: &str) {
        *self
            .stats
            .failed_macros
            .entry(macro_name.to_string())
            .or_insert(0) += 1;

        if self.config.verbose_warnings {
            eprintln!(
                "Warning: Unable to expand macro '{}' - function calls within may be missed",
                macro_name
            );
        }
    }

    /// Report macro expansion statistics
    pub fn report_macro_stats(&self) {
        if !self.config.show_statistics {
            return;
        }

        println!("\n=== Macro Expansion Statistics ===");
        println!("Total macros encountered: {}", self.stats.total_macros);
        println!("Successfully parsed: {}", self.stats.successfully_parsed);
        println!(
            "Parse rate: {:.1}%",
            (self.stats.successfully_parsed as f64 / self.stats.total_macros.max(1) as f64) * 100.0
        );

        if !self.stats.failed_macros.is_empty() {
            println!("\nUnexpandable macros:");
            let mut failed: Vec<_> = self.stats.failed_macros.iter().collect();
            failed.sort_by(|a, b| b.1.cmp(a.1));

            for (name, count) in failed.iter().take(10) {
                println!("  {} ({}x)", name, count);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_macro_type() {
        assert_eq!(
            MacroExpander::classify_macro_type("vec"),
            MacroType::Collection
        );
        assert_eq!(
            MacroExpander::classify_macro_type("hashmap"),
            MacroType::Collection
        );
        assert_eq!(
            MacroExpander::classify_macro_type("format"),
            MacroType::Formatting
        );
        assert_eq!(
            MacroExpander::classify_macro_type("println"),
            MacroType::Formatting
        );
        assert_eq!(
            MacroExpander::classify_macro_type("assert"),
            MacroType::Assertion
        );
        assert_eq!(
            MacroExpander::classify_macro_type("assert_eq"),
            MacroType::Assertion
        );
        assert_eq!(
            MacroExpander::classify_macro_type("debug"),
            MacroType::Logging
        );
        assert_eq!(
            MacroExpander::classify_macro_type("info"),
            MacroType::Logging
        );
        assert_eq!(
            MacroExpander::classify_macro_type("custom_macro"),
            MacroType::Generic
        );
    }

    #[test]
    fn test_is_map_macro() {
        assert!(MacroExpander::is_map_macro("hashmap"));
        assert!(MacroExpander::is_map_macro("btreemap"));
        assert!(MacroExpander::is_map_macro("maplit"));
        assert!(!MacroExpander::is_map_macro("vec"));
        assert!(!MacroExpander::is_map_macro("hashset"));
    }

    #[test]
    fn test_parse_key_value_pair() {
        let exprs = MacroExpander::parse_key_value_pair("\"key\" => \"value\"");
        assert_eq!(exprs.len(), 2);

        let exprs = MacroExpander::parse_key_value_pair("42");
        assert_eq!(exprs.len(), 1);

        let exprs = MacroExpander::parse_key_value_pair("");
        assert_eq!(exprs.len(), 0);
    }
}

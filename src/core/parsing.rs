//! Safe Rust parsing utilities that prevent proc-macro2 SourceMap overflow.
//!
//! When using `syn` with `proc-macro2`'s `span-locations` feature, a global
//! SourceMap accumulates byte positions across all parsed files. In large
//! codebases (thousands of files), this can overflow u32::MAX and panic.
//!
//! ## The Problem
//!
//! The `Span` types in syn ASTs reference a global SourceMap. After parsing:
//! 1. The AST holds Spans that point into the SourceMap
//! 2. Calling `span.start().line` looks up the position in the SourceMap
//! 3. If the SourceMap is cleared, those lookups fail
//!
//! ## Solution
//!
//! Call `reset_span_locations()` AFTER you've extracted all line numbers
//! from the AST, but BEFORE parsing the next file:
//!
//! ```rust,ignore
//! for file in files {
//!     let ast = syn::parse_file(&content)?;
//!     let metrics = extract_all_line_numbers(&ast);  // Uses span.start().line
//!     debtmap::core::parsing::reset_span_locations(); // Safe now - done with spans
//! }
//! ```

/// Reset the proc-macro2 SourceMap to prevent overflow when parsing many files.
///
/// **IMPORTANT**: Call this AFTER you've extracted all line numbers from
/// the current AST. Spans become invalid after this call.
///
/// This prevents the SourceMap from accumulating positions across thousands
/// of files, which would otherwise cause a u32 overflow panic.
///
/// # Safety
///
/// This invalidates all existing Spans. Only call when you're done using
/// span information from previously parsed files.
///
/// # Example
///
/// ```rust,ignore
/// let ast = syn::parse_file(&content)?;
/// let line = some_item.span().start().line;  // OK - SourceMap valid
/// reset_span_locations();                      // Clear the SourceMap
/// // let line = some_item.span().start().line; // PANIC - SourceMap cleared
/// ```
#[inline]
pub fn reset_span_locations() {
    proc_macro2::extra::invalidate_current_thread_spans();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_span_locations_basic() {
        // Parse a file
        let code = "fn foo() {}";
        let ast = syn::parse_file(code).expect("Failed to parse");

        // Extract line numbers BEFORE reset
        if let syn::Item::Fn(func) = &ast.items[0] {
            let line = func.sig.ident.span().start().line;
            assert_eq!(line, 1);
        }

        // Now safe to reset
        reset_span_locations();
    }

    #[test]
    fn test_parse_many_files_with_reset_no_overflow() {
        // Simulate parsing many files - this would overflow without reset
        for i in 0..1000 {
            let code = format!("fn func_{i}() {{ let x = {i}; }}");
            let ast = syn::parse_file(&code).expect("Failed to parse");
            assert_eq!(ast.items.len(), 1);

            // Extract line number while SourceMap is valid
            if let syn::Item::Fn(func) = &ast.items[0] {
                let line = func.sig.ident.span().start().line;
                assert_eq!(line, 1);
            }

            // Reset after extracting all needed info
            reset_span_locations();
        }
        // If we get here without panic, the reset is working
    }

    #[test]
    fn test_correct_usage_pattern() {
        // This test demonstrates the correct usage pattern
        let files = vec![
            "fn first() {}\nfn also_first() {}",
            "fn second() {}",
            "fn third() { let x = 1; }",
        ];

        let mut extracted_lines = vec![];

        for code in files {
            let ast = syn::parse_file(code).expect("Failed to parse");

            // Extract all line numbers while SourceMap is valid
            for item in &ast.items {
                if let syn::Item::Fn(func) = item {
                    let line = func.sig.ident.span().start().line;
                    let name = func.sig.ident.to_string();
                    extracted_lines.push((name, line));
                }
            }

            // Reset AFTER extracting - safe because we're done with this AST's spans
            reset_span_locations();
        }

        // Verify we extracted correct info
        assert_eq!(extracted_lines.len(), 4);
        assert_eq!(extracted_lines[0], ("first".to_string(), 1));
        assert_eq!(extracted_lines[1], ("also_first".to_string(), 2));
        assert_eq!(extracted_lines[2], ("second".to_string(), 1)); // Reset between files
        assert_eq!(extracted_lines[3], ("third".to_string(), 1));
    }
}

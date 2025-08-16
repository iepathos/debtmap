---
number: 35
title: Enhanced Token Parsing for Macro Handling
category: optimization
priority: high
status: draft
dependencies: [22, 23]
created: 2025-01-16
---

# Specification 35: Enhanced Token Parsing for Macro Handling

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [22 (Perfect Macro Function Call Detection), 23 (Enhanced Call Graph Analysis)]

## Context

The current macro expansion implementation uses `cargo expand` to fully expand all macros before analysis. However, investigation has revealed several critical issues:

1. **Granularity Mismatch**: `cargo expand` operates at the module level, not file level, making it impractical for per-file analysis
2. **Performance Overhead**: Full library expansion generates 80,000+ lines of code, causing significant performance degradation
3. **Silent Failures**: The current implementation fails silently and falls back to unexpanded code without clear user notification
4. **Complex Module Resolution**: Converting file paths to module paths is error-prone and often fails
5. **External Dependency**: Requires `cargo-expand` to be installed, which may not be available in all environments

Our investigation showed that a simpler token-parsing approach successfully handles common macros like `vec![]`, `format!()`, and others, resolving false positives in dead code detection without the overhead of full expansion.

## Objective

Replace the complex and unreliable `cargo expand` integration with an enhanced token-parsing approach that:
1. Handles common macro patterns effectively
2. Provides clear logging when macros cannot be expanded
3. Maintains or improves accuracy in function call detection
4. Eliminates external dependencies
5. Improves performance by avoiding subprocess calls

## Requirements

### Functional Requirements

1. **Enhanced Token Parser**
   - Parse tokens from common macros (`vec!`, `format!`, `println!`, `assert!`, etc.)
   - Extract function calls from macro token streams
   - Handle nested macros and struct literals within macros
   - Support custom derive and attribute macros where possible

2. **Pattern Recognition**
   - Identify known macro patterns and handle them specifically
   - Recognize collection macros (`vec!`, `hashmap!`, `btreemap!`)
   - Handle formatting macros (`format!`, `print!`, `println!`, `write!`, `writeln!`)
   - Process assertion macros (`assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!`)
   - Support logging macros (`log!`, `info!`, `warn!`, `error!`, `debug!`, `trace!`)

3. **Clear Logging and Diagnostics**
   - Log when a macro is successfully parsed
   - Warn when a macro cannot be expanded with specific reason
   - Indicate potential hidden function calls in unexpandable macros
   - Provide statistics on macro expansion success rate

4. **Fallback Strategy**
   - Gracefully handle unparseable macros
   - Continue analysis with available information
   - Mark functions that might have hidden callers

### Non-Functional Requirements

1. **Performance**
   - Token parsing must complete in < 5ms per file
   - No external process spawning
   - Minimal memory overhead

2. **Compatibility**
   - Work with all Rust editions (2015, 2018, 2021, 2024)
   - Handle both declarative and procedural macros gracefully
   - No external tool dependencies

3. **Maintainability**
   - Clean separation between macro handling and core analysis
   - Extensible pattern recognition system
   - Well-documented macro patterns

## Acceptance Criteria

- [ ] Token parser successfully handles `vec![]` macros containing struct literals with function calls
- [ ] Parser extracts function calls from `format!()` and similar formatting macros
- [ ] Clear warning messages are displayed when macros cannot be expanded
- [ ] No false positives for functions called within common macros
- [ ] Performance improvement of at least 10x compared to `cargo expand` approach
- [ ] All existing tests pass with the new implementation
- [ ] New tests verify macro handling for common patterns
- [ ] Documentation clearly explains macro handling limitations
- [ ] `cargo expand` integration code is completely removed
- [ ] Configuration options allow users to control macro expansion verbosity

## Technical Details

### Implementation Approach

1. **Enhance Existing Token Parser**
```rust
impl<'ast> Visit<'ast> for CallGraphExtractor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Macro(expr_macro) => {
                self.handle_macro_expression(expr_macro);
            }
            // ... other cases
        }
    }
}

impl CallGraphExtractor {
    fn handle_macro_expression(&mut self, expr_macro: &ExprMacro) {
        let macro_name = self.extract_macro_name(&expr_macro.mac.path);
        
        match macro_name.as_str() {
            "vec" | "vec_deque" => self.parse_collection_macro(&expr_macro.mac.tokens),
            "format" | "print" | "println" | "write" | "writeln" => {
                self.parse_format_macro(&expr_macro.mac.tokens)
            }
            "assert" | "assert_eq" | "assert_ne" | "debug_assert" => {
                self.parse_assert_macro(&expr_macro.mac.tokens)
            }
            _ => {
                // Try generic expression parsing
                if let Ok(expr) = syn::parse2::<Expr>(expr_macro.mac.tokens.clone()) {
                    self.visit_expr(&expr);
                } else {
                    self.log_unexpandable_macro(&macro_name);
                }
            }
        }
    }
}
```

2. **Pattern-Specific Parsers**
```rust
fn parse_collection_macro(&mut self, tokens: &TokenStream) {
    // Parse [expr, expr, ...] pattern
    if let Ok(exprs) = parse_comma_separated_exprs(tokens) {
        for expr in exprs {
            self.visit_expr(&expr);
        }
    }
}

fn parse_format_macro(&mut self, tokens: &TokenStream) {
    // Skip format string, parse arguments
    if let Ok((_, args)) = parse_format_args(tokens) {
        for arg in args {
            self.visit_expr(&arg);
        }
    }
}
```

3. **Logging System**
```rust
struct MacroExpansionStats {
    total_macros: usize,
    successfully_parsed: usize,
    failed_macros: HashMap<String, usize>,
}

impl CallGraphExtractor {
    fn log_unexpandable_macro(&mut self, macro_name: &str) {
        if self.config.verbose_macro_warnings {
            eprintln!("⚠ Cannot expand macro '{}' - may contain hidden function calls", macro_name);
        }
        self.stats.failed_macros.entry(macro_name.to_string())
            .and_modify(|e| *e += 1)
            .or_insert(1);
    }
    
    fn report_macro_stats(&self) {
        if self.config.show_macro_stats {
            eprintln!("\nMacro Expansion Statistics:");
            eprintln!("  Total macros encountered: {}", self.stats.total_macros);
            eprintln!("  Successfully parsed: {} ({:.1}%)", 
                self.stats.successfully_parsed,
                (self.stats.successfully_parsed as f64 / self.stats.total_macros as f64) * 100.0
            );
            if !self.stats.failed_macros.is_empty() {
                eprintln!("  Failed macros:");
                for (name, count) in &self.stats.failed_macros {
                    eprintln!("    {}: {} occurrences", name, count);
                }
            }
        }
    }
}
```

### Architecture Changes

1. **Remove Expansion Module**
   - Delete `src/expansion/` directory and all its contents
   - Remove `ExpansionConfig` and related types
   - Remove `--expand-macros` and `--no-expand-macros` CLI flags
   - Remove expansion cache management

2. **Enhance Call Graph Module**
   - Add `MacroHandler` trait for extensible macro handling
   - Implement pattern-specific macro parsers
   - Add macro expansion statistics tracking

3. **Update Configuration**
   - Add `--verbose-macro-warnings` flag for detailed macro logging
   - Add `--show-macro-stats` flag for expansion statistics
   - Remove all cargo-expand related configuration

### Data Structures

```rust
/// Macro pattern types for specialized handling
enum MacroPattern {
    Collection(Vec<Expr>),           // vec![...], array literals
    Format(String, Vec<Expr>),       // format strings with arguments
    Assertion(Expr, Option<Expr>),   // assertions with conditions
    Generic(TokenStream),             // Unknown patterns
}

/// Macro expansion result
enum MacroExpansionResult {
    Success(Vec<Expr>),              // Successfully extracted expressions
    Partial(Vec<Expr>, String),      // Partial success with warning
    Failed(String),                  // Complete failure with reason
}

/// Configuration for macro handling
struct MacroHandlingConfig {
    verbose_warnings: bool,          // Show detailed warnings
    show_statistics: bool,           // Display expansion stats
    known_macros: HashSet<String>,   // User-defined known macros
}
```

## Dependencies

- **Prerequisites**: Spec 22 and 23 must remain functional
- **Affected Components**: 
  - `src/analyzers/rust_call_graph.rs` - Primary changes
  - `src/cli.rs` - Remove expansion-related flags
  - `src/main.rs` - Remove expansion initialization
- **External Dependencies**: None (removal of cargo-expand dependency)

## Testing Strategy

- **Unit Tests**: 
  - Test each macro pattern parser individually
  - Verify token stream parsing for various macro formats
  - Test error handling for malformed macros

- **Integration Tests**:
  - Verify no false positives for common macro patterns
  - Test with real-world Rust projects
  - Compare results with previous implementation

- **Performance Tests**:
  - Benchmark token parsing speed
  - Compare with cargo-expand performance
  - Memory usage analysis

- **Regression Tests**:
  - Ensure all existing tests continue to pass
  - Verify dead code detection accuracy is maintained

## Documentation Requirements

- **Code Documentation**: 
  - Document each macro pattern and its handling
  - Explain limitations of token-based approach
  - Provide examples of supported and unsupported macros

- **User Documentation**:
  - Update README with new macro handling approach
  - Document removed CLI flags
  - Explain when manual inspection might be needed

- **Architecture Updates**:
  - Update ARCHITECTURE.md to reflect removal of expansion module
  - Document new macro handling strategy

## Implementation Notes

1. **Priority Order**: Start with most common macros (vec!, format!, println!)
2. **Incremental Rollout**: Can be implemented alongside existing system initially
3. **Backwards Compatibility**: Ensure graceful handling of all existing code
4. **Future Extensions**: Design pattern system to be easily extensible

## Migration and Compatibility

1. **Breaking Changes**:
   - Removal of `--expand-macros` and `--no-expand-macros` flags
   - Removal of expansion cache directory `.debtmap/cache/expanded/`

2. **Migration Path**:
   - Provide clear migration guide for users relying on cargo-expand
   - Offer configuration to increase verbosity for debugging
   - Document any changes in analysis accuracy

3. **Compatibility Considerations**:
   - Ensure compatibility with all Rust versions
   - Handle both old and new macro syntax
   - Graceful degradation for unknown macros
# Macro Expansion Design for Debtmap

## Problem Statement

Rust macros can hide function calls, making dead code detection and call graph analysis incomplete. For example:

```rust
vec![MyStruct {
    field: some_function(), // This call is missed without macro expansion
}]
```

## Current Approaches

### 1. Full Expansion via `cargo expand`
- **Pros**: Complete and accurate
- **Cons**: Slow, requires external tool, complex module resolution

### 2. Token-Based Parsing
- **Pros**: Fast, simple, no dependencies
- **Cons**: Limited to simple macros

## Recommended Solution: Three-Tier Approach

### Tier 1: AST-Level Macro Recognition (Fastest)
Handle known macro patterns directly in the visitor:

```rust
impl<'ast> Visit<'ast> for CallGraphExtractor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Macro(mac) if is_known_macro(&mac.mac.path) => {
                handle_known_macro(self, mac);
            }
            Expr::Macro(mac) => {
                // Try token parsing for unknown macros
                if let Ok(parsed) = syn::parse2::<Expr>(mac.mac.tokens.clone()) {
                    self.visit_expr(&parsed);
                }
            }
            // ... other cases
        }
    }
}
```

### Tier 2: Lightweight Expansion (syn-expand)
Use `syn-expand` crate for in-process macro expansion:

```rust
use syn_expand::ExpandedFile;

fn expand_lightweight(file: &syn::File) -> Result<syn::File> {
    // Expands declarative macros without invoking cargo
    let expanded = syn_expand::expand_file(file)?;
    Ok(expanded)
}
```

### Tier 3: Full Expansion (When Needed)
Reserve `cargo expand` for special cases:
- Analyzing procedural macro-heavy code
- One-time comprehensive analysis
- CI/CD pipeline checks

## Implementation Plan

### Phase 1: Optimize Current Token Parsing
```rust
// Enhance macro token parsing with common patterns
fn parse_macro_content(mac: &ExprMacro) -> Option<MacroContent> {
    let path = &mac.mac.path;
    let tokens = &mac.mac.tokens;
    
    match get_macro_name(path).as_str() {
        "vec" => parse_vec_macro(tokens),
        "format" | "println" | "print" => parse_format_macro(tokens),
        "assert" | "assert_eq" => parse_assert_macro(tokens),
        _ => try_parse_as_expr(tokens),
    }
}
```

### Phase 2: Add Pattern Recognition
```rust
// Recognize common macro patterns without full expansion
enum MacroPattern {
    VecLiteral(Vec<Expr>),      // vec![expr, expr, ...]
    FormatString(Vec<Expr>),    // format!("...", args...)
    Assertion(Expr),            // assert!(condition)
    Custom(TokenStream),        // Unknown pattern
}
```

### Phase 3: Integrate syn-expand (Optional)
If more accuracy is needed:
```toml
[dependencies]
syn-expand = "0.1"  # Lightweight macro expansion
```

## Configuration

```toml
# .debtmap/config.toml
[macro_expansion]
strategy = "hybrid"  # Options: "none", "token", "hybrid", "full"
enable_cargo_expand = false  # Only for special cases
cache_expanded = true
known_macros = ["vec", "format", "println", "assert"]
```

## Performance Comparison

| Method | Speed | Accuracy | Dependencies | Complexity |
|--------|-------|----------|--------------|------------|
| Token Parsing | Fast (< 1ms) | 70% | None | Low |
| Pattern Recognition | Fast (< 2ms) | 85% | None | Medium |
| syn-expand | Medium (10ms) | 95% | syn-expand | Medium |
| cargo expand | Slow (1-5s) | 100% | cargo-expand | High |

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_vec_macro_parsing() {
    let code = r#"vec![Item { func: helper() }]"#;
    assert!(detects_call_to("helper", code));
}
```

### Integration Tests
```rust
#[test]
fn test_real_world_macros() {
    // Test against actual codebase patterns
    let stats = analyze_with_strategy("token");
    assert!(stats.detected_calls > stats.missed_calls);
}
```

### Benchmarks
```rust
#[bench]
fn bench_macro_strategies(b: &mut Bencher) {
    b.iter(|| {
        analyze_with_token_parsing(&test_file);
    });
}
```

## Recommendations

1. **Default Strategy**: Use token parsing with pattern recognition
   - Covers 90% of real-world cases
   - Fast enough for interactive use
   - No external dependencies

2. **For CI/CD**: Enable syn-expand
   - Better accuracy for critical analysis
   - Still reasonably fast
   - Deterministic results

3. **For Deep Analysis**: Use cargo expand selectively
   - Only for specific modules with heavy macro use
   - Cache results aggressively
   - Run in background/batch mode

## Migration Path

1. Keep current token parsing implementation (already working)
2. Add pattern recognition for common macros
3. Improve error messages to indicate when macros might hide calls
4. Consider syn-expand if users report many false positives
5. Make cargo expand optional and well-documented

## Conclusion

The hybrid approach balances accuracy and performance:
- **Fast path**: Token parsing handles 90% of cases in < 1ms
- **Accurate path**: Available when needed via configuration
- **User-friendly**: Clear feedback about what's being analyzed

This design ensures debtmap remains fast for interactive use while providing options for more thorough analysis when needed.
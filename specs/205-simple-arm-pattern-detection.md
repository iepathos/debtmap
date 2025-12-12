---
number: 205
title: Simple Arm Pattern Detection for Try and Macro Expressions
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-11
---

# Specification 205: Simple Arm Pattern Detection for Try and Macro Expressions

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap's match expression pattern recognition system (`src/complexity/match_patterns.rs`) determines whether a match expression qualifies for logarithmic complexity scaling. The key gate is the `is_simple_arm` function which identifies "simple" match arm bodies.

### The Problem

The current `is_simple_arm` function does not recognize two common Rust patterns:

1. **Try expressions (`?` operator)**: Arms like `write_header(writer)?` are not recognized as simple
2. **Macro invocations**: Arms like `writeln!(writer, "{}", text)?` are not recognized

### Evidence

The `write_section` function in `src/priority/formatter/writer.rs` is a textbook example of a clean match dispatcher:

```rust
fn write_section(writer: &mut dyn Write, section: &FormattedSection) -> io::Result<()> {
    match section {
        FormattedSection::Header { .. } => write_header_section(writer, ...)?
        FormattedSection::Location { .. } => {
            writeln!(writer, "...")?;
        }
        // ... 11 more simple arms
    }
    Ok(())
}
```

**Current metrics (inaccurate)**:
- Cyclomatic: 26 (should be ~4-5 with logarithmic scaling)
- Cognitive: 40 → 21 (dampened)

**Why it's flagged**: Pattern detection fails because arms end with `?` or use `writeln!` macro, so logarithmic scaling never applies.

### Current Implementation (`match_patterns.rs:20-50`)

```rust
pub fn is_simple_arm(&self, body: &Expr) -> bool {
    match body {
        Expr::Return(_) | Expr::Break(_) | Expr::Continue(_) => true,
        Expr::Lit(_) | Expr::Path(_) => true,
        Expr::MethodCall(_) | Expr::Field(_) => true,
        Expr::Call(call) => matches!(&*call.func, Expr::Path(_)),
        Expr::Block(block) => { /* limited block handling */ }
        _ => false,  // Expr::Try and Expr::Macro fall here!
    }
}
```

## Objective

Extend `is_simple_arm` to recognize:
1. Try expressions (`?` operator) wrapping simple expressions
2. Macro invocations (e.g., `writeln!`, `println!`, `format!`)
3. Blocks containing a single try expression or macro

This enables logarithmic complexity scaling for match dispatchers using idiomatic Rust error handling patterns.

## Requirements

### Functional Requirements

1. **FR-1: Recognize Try Expressions**
   - `Expr::Try(try_expr)` should be considered simple if `try_expr.expr` is simple
   - Recursive check: `foo()?` is simple if `foo()` is simple
   - Handles chained: `foo()?.bar()?` should evaluate the chain

2. **FR-2: Recognize Macro Invocations**
   - `Expr::Macro(_)` should be considered simple by default
   - Common I/O macros: `writeln!`, `println!`, `eprintln!`, `format!`, `write!`
   - Logging macros: `log!`, `debug!`, `info!`, `warn!`, `error!`
   - Consider macro name if restrictive mode needed later

3. **FR-3: Handle Blocks with Try/Macro**
   - Block with single `Expr::Try` statement should be simple
   - Block with single macro invocation should be simple
   - Example: `{ writeln!(w, "{}", x)?; }` is simple

4. **FR-4: Maintain Existing Simple Detection**
   - All currently detected simple patterns must remain detected
   - No regression in existing tests

### Non-Functional Requirements

1. **NFR-1: Performance**
   - No additional AST traversal beyond what's currently done
   - Recursive depth limited (max ~5 levels for chained calls)

2. **NFR-2: Accuracy**
   - False positive rate < 5% (incorrectly marking complex as simple)
   - False negative rate < 10% (missing simple patterns)

3. **NFR-3: Testability**
   - All new patterns must have unit tests
   - Edge cases documented and tested

## Acceptance Criteria

- [ ] `foo()?` is detected as simple when `foo()` is a simple call
- [ ] `writeln!(w, "text")?` is detected as simple
- [ ] `{ helper_fn(x)?; }` block is detected as simple
- [ ] `write_section`-style match dispatchers get logarithmic scaling applied
- [ ] All existing `is_simple_arm` tests pass
- [ ] New test case for `write_section` pattern passes
- [ ] Match with 13 try-operator arms reports complexity ~4-5, not 26

## Technical Details

### Implementation Approach

#### Phase 1: Add Try Expression Handling

```rust
pub fn is_simple_arm(&self, body: &Expr) -> bool {
    match body {
        // ... existing cases ...

        // NEW: Try expressions (? operator)
        Expr::Try(try_expr) => self.is_simple_arm(&try_expr.expr),

        _ => false,
    }
}
```

#### Phase 2: Add Macro Invocation Handling

```rust
pub fn is_simple_arm(&self, body: &Expr) -> bool {
    match body {
        // ... existing cases ...

        // NEW: Macro invocations (writeln!, println!, etc.)
        Expr::Macro(_) => true,

        _ => false,
    }
}
```

**Note**: We treat all macros as simple. If future issues arise, we can add macro name filtering:

```rust
Expr::Macro(expr_macro) => {
    // Optional: filter by known safe macros
    let path = &expr_macro.mac.path;
    let name = path.segments.last().map(|s| s.ident.to_string());
    matches!(name.as_deref(), Some("writeln" | "println" | "format" | ...))
}
```

#### Phase 3: Enhance Block Handling

```rust
Expr::Block(block) => {
    let block = &block.block;
    match block.stmts.len() {
        0 => true,  // Empty block is simple
        1 => match &block.stmts[0] {
            Stmt::Expr(expr, _) => self.is_simple_arm(expr),
            Stmt::Semi(expr, _) => self.is_simple_arm(expr),  // Expression with semicolon
            _ => false,
        },
        2 => {
            // Allow one statement plus return (existing logic)
            matches!(&block.stmts[1], Stmt::Expr(Expr::Return(_), _))
        }
        _ => false,
    }
}
```

### Architecture Changes

None - this is a localized change to `MatchExpressionRecognizer::is_simple_arm`.

### Data Structures

No new data structures required.

### APIs and Interfaces

No public API changes. Internal method signature unchanged.

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/complexity/match_patterns.rs` - Core change location
  - Indirectly: Any code path using `MatchExpressionRecognizer`
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_simple_arm_try_expression() {
    let recognizer = MatchExpressionRecognizer::new();

    // Simple function call with ?
    let expr: Expr = parse_quote!(helper_fn()?);
    assert!(recognizer.is_simple_arm(&expr));

    // Method call with ?
    let expr: Expr = parse_quote!(self.write_header()?);
    assert!(recognizer.is_simple_arm(&expr));

    // Chained calls with ?
    let expr: Expr = parse_quote!(foo()?.bar()?);
    assert!(recognizer.is_simple_arm(&expr));
}

#[test]
fn test_simple_arm_macro_invocation() {
    let recognizer = MatchExpressionRecognizer::new();

    // writeln! macro
    let expr: Expr = parse_quote!(writeln!(w, "text"));
    assert!(recognizer.is_simple_arm(&expr));

    // writeln! with ?
    let expr: Expr = parse_quote!(writeln!(w, "text")?);
    assert!(recognizer.is_simple_arm(&expr));

    // println! macro
    let expr: Expr = parse_quote!(println!("debug"));
    assert!(recognizer.is_simple_arm(&expr));
}

#[test]
fn test_simple_arm_block_with_try() {
    let recognizer = MatchExpressionRecognizer::new();

    // Block with single try expression
    let expr: Expr = parse_quote!({
        helper_fn()?;
    });
    assert!(recognizer.is_simple_arm(&expr));

    // Block with macro and try
    let expr: Expr = parse_quote!({
        writeln!(w, "text")?;
    });
    assert!(recognizer.is_simple_arm(&expr));
}

#[test]
fn test_complex_arm_not_simple() {
    let recognizer = MatchExpressionRecognizer::new();

    // Block with multiple statements
    let expr: Expr = parse_quote!({
        let x = compute();
        process(x)?;
        cleanup();
    });
    assert!(!recognizer.is_simple_arm(&expr));

    // Conditional inside
    let expr: Expr = parse_quote!(if cond { foo() } else { bar() });
    assert!(!recognizer.is_simple_arm(&expr));
}
```

### Integration Tests

```rust
#[test]
fn test_write_section_style_match() {
    let block: syn::Block = parse_quote! {{
        match section {
            Section::Header { rank, score } => write_header(w, rank, score)?,
            Section::Location { file, line } => writeln!(w, "{}:{}", file, line)?,
            Section::Action { action } => writeln!(w, "{}", action)?,
            Section::Impact { reduction } => write_impact(w, reduction)?,
            Section::Evidence { text } => writeln!(w, "{}", text)?,
        }
    }};

    let recognizer = MatchExpressionRecognizer::new();
    let info = recognizer.detect(&block);

    assert!(info.is_some(), "Should detect as pattern match");
    let info = info.unwrap();
    assert_eq!(info.condition_count, 5);

    // Complexity should use logarithmic scaling
    let adjusted = recognizer.adjust_complexity(&info, 5);
    assert!(adjusted <= 4, "Adjusted complexity should be ~3-4, got {}", adjusted);
}
```

### Performance Tests

Not required - change is minimal and doesn't add loops.

## Documentation Requirements

- **Code Documentation**: Update doc comment on `is_simple_arm` to list all recognized patterns
- **User Documentation**: None required
- **Architecture Updates**: None required

## Implementation Notes

1. **Recursion Safety**: The recursive call for `Expr::Try` is safe because:
   - `try_expr.expr` is always a simpler expression
   - Rust's `?` syntax limits nesting depth naturally
   - Add recursion depth limit if paranoid: `is_simple_arm_impl(expr, depth + 1)`

2. **Macro Safety**: Treating all macros as simple is reasonable because:
   - Macros are typically single operations (print, format, assert)
   - Complex macros that generate loops/conditionals are rare in match arms
   - If issues arise, add allowlist filtering

3. **Semi vs Non-Semi Expressions**: In blocks, handle both:
   - `Stmt::Expr(expr, Some(Semi))` - expression with semicolon
   - `Stmt::Expr(expr, None)` - expression without semicolon (return value)

## Migration and Compatibility

- **Breaking Changes**: None
- **Migration**: Not required
- **Compatibility**: Existing complexity reports will improve (lower complexity for clean dispatchers)

## Success Metrics

1. **Pattern Detection**: `write_section` is recognized as pattern match
2. **Complexity Reduction**: Reported complexity drops from 26 to ~4-5
3. **No False Positives**: Complex match arms are still flagged appropriately
4. **Test Coverage**: All new code paths have unit tests

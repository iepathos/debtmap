# SPEC-215: Function-Level Debt Suppression with Justification

## Status
- **Status**: Draft
- **Author**: Claude Code Assistant
- **Created**: 2026-02-11
- **Priority**: High

## Problem Statement

### Current Situation

Debtmap's suppression system (`// debtmap:ignore[type]`) works well for line-level suppressions (TODOs, FIXMEs, simple code smells), but has gaps for function-level debt:

1. **Missing `testing` debt type**: Functions flagged for low coverage cannot be suppressed, even when the low coverage is intentional (e.g., I/O orchestration functions where pure logic is extracted and tested separately).

2. **Line-level vs function-level mismatch**: For complexity and testing debt, the debt item is associated with a function, not a specific line. Current suppressions (`ignore-next-line`, `ignore-start/end`) don't naturally apply to function-scoped debt.

3. **Justification visibility**: While reasons are supported (`-- reason`), they're not prominently surfaced in reports or validated for completeness.

### Real-World Example

```rust
/// Main agent loop coordinating iterations, state persistence, and shutdown.
///
/// This is an orchestration function with intentionally higher complexity.
/// Pure business logic is extracted into: `IterationOutcome`, `LoopAction`,
/// `apply_iteration_result`, and `apply_stop_status`.
async fn run_loop(...) -> Result<(), BoxError> {
    // 54 lines of async coordination
}
```

This function:
- Has 25% direct coverage (the async shell)
- Has 100% coverage on extracted pure functions
- Is correctly designed following "pure core, imperative shell"
- Gets flagged as High priority (score 67.48) due to Testing category
- Cannot be suppressed because `testing` isn't a valid debt type

## Proposed Solution

### 1. Add `testing` and `coverage` Debt Types

Extend the suppression parser to recognize:

```rust
// debtmap:allow[testing] -- Orchestration function; pure logic tested in callees
async fn run_loop(...) { ... }

// debtmap:allow[coverage] -- Same as testing, alias for clarity
```

### 2. Function-Level `allow` Annotation

Introduce `debtmap:allow` as a function-level annotation (vs `ignore` for line-level):

```rust
// debtmap:allow[complexity,testing] -- Orchestration function with extracted pure logic
async fn run_loop(...) { ... }
```

**Semantics:**
- `allow` applies to the entire function definition that follows
- Must appear in a doc comment or regular comment immediately before `fn`/`async fn`
- Suppresses the specified debt types for that function only
- Requires a justification (the `-- reason` part)

### 3. Supported Debt Types (Extended)

| Type | Aliases | Applies To |
|------|---------|------------|
| `todo` | - | TODO comments |
| `fixme` | - | FIXME comments |
| `complexity` | `cc`, `cognitive` | High cyclomatic/cognitive complexity |
| `testing` | `coverage`, `untested` | Low test coverage |
| `dependency` | `coupling` | High coupling/instability |
| `duplication` | `duplicate` | Duplicate code |
| `smell` | `codesmell` | General code smells |
| `dead` | `deadcode`, `unused` | Dead/unused code |
| `*` | `all` | Wildcard - all types |

### 4. Justification Requirements

For `allow` annotations, justification is **required** (not optional like `ignore`):

```rust
// Valid:
// debtmap:allow[testing] -- Orchestration function; callees are tested

// Invalid (will warn):
// debtmap:allow[testing]
```

Justifications should explain:
1. **Why** the debt is acceptable
2. **What** mitigates the risk (e.g., "callees are tested")

### 5. Annotation Syntax Grammar

```
annotation     := "debtmap:" directive "[" types "]" justification?
directive      := "allow" | "ignore" | "ignore-next-line" | "ignore-start" | "ignore-end"
types          := type ("," type)* | "*"
type           := "testing" | "coverage" | "complexity" | ...
justification  := "--" reason
reason         := <any text until end of comment>
```

### 6. Parsing Implementation

Location: `/src/debt/suppression.rs`

```rust
/// Extended suppression rule supporting function-level allow
pub struct SuppressionRule {
    pub debt_types: Vec<DebtType>,
    pub reason: Option<String>,
    pub scope: SuppressionScope,
}

pub enum SuppressionScope {
    /// Applies to the current line only (inline ignore)
    CurrentLine,
    /// Applies to the next line (ignore-next-line)
    NextLine,
    /// Applies to lines until ignore-end (block)
    Block { end_line: Option<usize> },
    /// Applies to the next function definition (allow)
    NextFunction,
}
```

### 7. Integration with Analysis Pipeline

In `/src/analyzers/rust/debt/collection.rs`:

```rust
fn is_function_suppressed(
    func: &FunctionMetrics,
    suppression_context: &SuppressionContext,
    debt_type: &DebtType,
) -> Option<&str> {
    // Check for function-level allow annotation before the function
    suppression_context
        .function_suppressions
        .get(&func.start_line)
        .filter(|rule| rule.matches_debt_type(debt_type))
        .map(|rule| rule.reason.as_deref().unwrap_or("No reason provided"))
}
```

### 8. Reporting Suppressed Items

Add `--show-suppressed` flag to include suppressed items in output:

```
## Suppressed Items (3)

| Function | Debt Type | Reason |
|----------|-----------|--------|
| run_loop | testing | Orchestration function; callees are tested |
| parse_config | complexity | Match exhaustiveness required for config variants |
```

### 9. Validation and Warnings

1. **Missing justification**: Warn when `allow` lacks a reason
2. **Orphaned suppression**: Warn when `allow` doesn't precede a function
3. **Unused suppression**: Warn when `allow` type doesn't match any actual debt
4. **Expired suppression**: Support optional expiry (`-- reason [expires: 2026-06-01]`)

## Implementation Plan

### Phase 1: Core Suppression Extension
1. Add `testing`/`coverage` to `DebtType` matching in suppression parser
2. Add `SuppressionScope::NextFunction` variant
3. Extend regex patterns to parse `debtmap:allow`
4. Update `is_suppressed()` to check function-level suppressions

### Phase 2: Integration
1. Wire function-level suppression into Rust analyzer
2. Add suppression context to `FunctionMetrics` or debt item generation
3. Filter debt items based on function-level suppressions

### Phase 3: Reporting
1. Add `--show-suppressed` flag
2. Include suppression reasons in JSON/markdown output
3. Add suppression statistics to summary

### Phase 4: Validation
1. Implement justification requirement warning
2. Add orphaned/unused suppression detection
3. Add expiry date support (optional)

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_allow_testing_on_function() {
    let source = r#"
// debtmap:allow[testing] -- Orchestration function
async fn run_loop() { }
"#;
    let ctx = parse_suppression_comments(source, Language::Rust, Path::new("test.rs"));
    assert!(ctx.is_function_suppressed(1, &DebtType::TestingGap { ... }));
}

#[test]
fn test_allow_requires_justification() {
    let source = "// debtmap:allow[testing]\nfn foo() {}";
    let ctx = parse_suppression_comments(source, Language::Rust, Path::new("test.rs"));
    assert!(!ctx.warnings.is_empty());
}
```

### Integration Tests
- Verify suppressed functions don't appear in default output
- Verify `--show-suppressed` includes them with reasons
- Verify JSON output includes suppression metadata

## Migration Path

Existing `ignore` annotations continue to work unchanged. The new `allow` syntax is additive.

## Open Questions

1. **Should `allow` work for file-level debt?** (e.g., god objects)
2. **Should justifications be structured?** (e.g., `-- reason: X; mitigated-by: Y`)
3. **Should there be a minimum justification length?**
4. **Should suppressions be auditable in a separate report?**

## References

- Current suppression implementation: `/src/debt/suppression.rs`
- Debt type definitions: `/src/priority/debt_types.rs`
- Integration point: `/src/analyzers/rust/debt/collection.rs`

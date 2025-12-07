---
number: 246
title: Variable Extraction from I/O Operations
category: foundation
priority: high
status: draft
dependencies: [245]
created: 2025-12-06
---

# Specification 246: Variable Extraction from I/O Operations

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 245 (AST-Based I/O Detection)

## Context

Currently, when I/O operations are detected, the `IoOperation` struct has an empty `variables` field (`variables: vec![]`). This is evident in the TUI data flow page showing entries like:

```
file_io at line 69 (variables: )
```

The blank variables list makes the data flow information essentially useless for understanding:
- What data is being written to files
- What variables are being logged or printed
- What data is being sent over the network
- What queries use which variables

Even with spec 245 improving detection coverage from 4.3% to 70%, the data remains incomplete without knowing which variables are involved in each I/O operation.

## Objective

Extract and track the actual variables involved in each I/O operation by analyzing expression arguments, enabling users to understand data flow dependencies and identify which variables contribute to I/O side effects.

## Requirements

### Functional Requirements

- **FR1**: Extract variables from function call arguments
  - Direct variable references: `file.write(data)` → `["data"]`
  - Field access: `file.write(self.buffer)` → `["buffer"]` or `["self.buffer"]`
  - Multiple arguments: `format!("{} {}", name, age)` → `["name", "age"]`

- **FR2**: Extract variables from method call receivers
  - `data.write_to_file()` → `["data"]`
  - `self.logger.log(msg)` → `["self.logger", "msg"]`

- **FR3**: Extract variables from macro arguments
  - `println!("Value: {}", x)` → `["x"]`
  - `write!(f, "{:?}", state)` → `["f", "state"]`
  - `eprintln!("{:#?}", complex_value)` → `["complex_value"]`

- **FR4**: Handle complex expressions
  - References: `&buffer` → `["buffer"]`
  - Dereferences: `*ptr` → `["ptr"]`
  - Function calls: `calculate(x, y)` → `["x", "y"]`
  - Method chains: `data.clone()` → `["data"]`

- **FR5**: Deduplicate and normalize variable names
  - Remove duplicates: `format!("{} {}", x, x)` → `["x"]` (not `["x", "x"]`)
  - Sort alphabetically for consistent output
  - Normalize paths: `self.field.nested` can be represented as full path

- **FR6**: Limit variable extraction depth
  - Don't recursively descend into complex expressions indefinitely
  - Focus on top-level variables that directly contribute
  - Configurable depth limit (default: 2 levels)

### Non-Functional Requirements

- **NFR1**: Performance - Variable extraction must add <5% overhead
- **NFR2**: Accuracy - Extract 90%+ of relevant variables without false positives
- **NFR3**: Readability - Variable names should be human-readable in TUI
- **NFR4**: Completeness - Prefer including too many variables over missing critical ones

## Acceptance Criteria

- [ ] Implement `extract_variables_from_expr` function in `src/analyzers/io_detector.rs`
- [ ] Handle all expression types from FR1-FR4 with test coverage
- [ ] Populate `IoOperation.variables` field in all detected operations
- [ ] Add comprehensive unit tests for each expression type (>95% coverage)
- [ ] TUI data flow page shows meaningful variable lists (not blank)
- [ ] Integration test verifies variables extracted from real functions
- [ ] Performance benchmark shows <5% overhead
- [ ] Document variable extraction strategy in code
- [ ] Handle edge cases: closures, complex nested expressions, method chains

## Technical Details

### Implementation Approach

Extend the `IoDetectorVisitor` from spec 245 to extract variables when detecting I/O operations:

```rust
impl IoDetectorVisitor {
    fn extract_variables_from_args(
        &self,
        args: &Punctuated<Expr, Token![,]>
    ) -> Vec<String> {
        let mut vars = Vec::new();
        for arg in args {
            self.collect_variables_from_expr(arg, &mut vars, 0);
        }
        vars.sort();
        vars.dedup();
        vars
    }

    fn collect_variables_from_expr(
        &self,
        expr: &Expr,
        vars: &mut Vec<String>,
        depth: usize,
    ) {
        const MAX_DEPTH: usize = 2;
        if depth > MAX_DEPTH {
            return;
        }

        match expr {
            Expr::Path(path) => {
                // Extract variable name from path
                if let Some(ident) = path.path.get_ident() {
                    vars.push(ident.to_string());
                } else {
                    // Multi-segment path: self.field.nested
                    let path_str = quote!(#path).to_string();
                    vars.push(path_str);
                }
            }
            Expr::Field(field) => {
                // field.member access
                self.collect_variables_from_expr(&field.base, vars, depth + 1);
                if let Member::Named(name) = &field.member {
                    // Option: collect full path or just leaf
                    vars.push(name.to_string());
                }
            }
            Expr::Reference(reference) => {
                // &expr or &mut expr
                self.collect_variables_from_expr(&reference.expr, vars, depth);
            }
            Expr::Unary(unary) => {
                // *expr, !expr, -expr
                self.collect_variables_from_expr(&unary.expr, vars, depth);
            }
            Expr::Call(call) => {
                // Function call: extract args
                for arg in &call.args {
                    self.collect_variables_from_expr(arg, vars, depth + 1);
                }
            }
            Expr::MethodCall(method_call) => {
                // Receiver and args
                self.collect_variables_from_expr(&method_call.receiver, vars, depth + 1);
                for arg in &method_call.args {
                    self.collect_variables_from_expr(arg, vars, depth + 1);
                }
            }
            Expr::Index(index) => {
                // array[index]
                self.collect_variables_from_expr(&index.expr, vars, depth + 1);
                self.collect_variables_from_expr(&index.index, vars, depth + 1);
            }
            // Stop at literals, blocks, closures
            _ => {}
        }
    }

    fn extract_variables_from_macro(
        &self,
        tokens: &TokenStream,
    ) -> Vec<String> {
        // Parse macro tokens to extract variable references
        // This is heuristic-based since macros aren't expanded
        let mut vars = Vec::new();

        // Convert tokens to string and parse for identifiers
        let token_str = tokens.to_string();

        // Split on common delimiters and extract identifiers
        for token in token_str.split(&[',', ' ', '{', '}', '(', ')'][..]) {
            let trimmed = token.trim();
            if is_valid_identifier(trimmed) && !is_literal(trimmed) {
                vars.push(trimmed.to_string());
            }
        }

        vars.sort();
        vars.dedup();
        vars
    }
}

fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().unwrap().is_alphabetic()
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn is_literal(s: &str) -> bool {
    s.chars().all(|c| c.is_numeric())
        || s.starts_with('"')
        || matches!(s, "true" | "false" | "None" | "Some")
}
```

### Architecture Changes

1. **Modified**: `src/analyzers/io_detector.rs`
   - Add variable extraction methods
   - Populate `variables` field in `IoOperation` during detection
   - Handle both expression arguments and macro arguments

2. **Modified**: `src/tui/results/detail_pages/data_flow.rs`
   - No changes needed - already displays variables when present
   - Variables will automatically appear in output

3. **No changes**: `src/data_flow/mod.rs`
   - `IoOperation.variables` field already exists

### Data Structures

No new data structures needed. Enhances existing:

```rust
// Already exists in src/data_flow/mod.rs
pub struct IoOperation {
    pub operation_type: String,
    pub variables: Vec<String>, // ← Will now be populated!
    pub line: usize,
}
```

### Variable Naming Strategy

**Simple variables**: Store as-is
- `x` → `"x"`
- `buffer` → `"buffer"`
- `data` → `"data"`

**Field access**: Store full path for context
- `self.field` → `"self.field"`
- `obj.nested.value` → `"obj.nested.value"`

**Complex expressions**: Extract contributing variables
- `calculate(x, y)` → `["x", "y"]`
- `&mut buffer[i]` → `["buffer", "i"]`

**Deduplication**: Remove redundant entries
- `format!("{} {}", x, x)` → `["x"]` (not `["x", "x"]`)

## Dependencies

- **Prerequisites**: Spec 245 (AST-Based I/O Detection) - must be implemented first
- **Affected Components**:
  - `src/analyzers/io_detector.rs` (extend with variable extraction)
  - `src/tui/results/detail_pages/data_flow.rs` (automatically benefits)
- **External Dependencies**:
  - `syn` crate for expression traversal
  - `quote` crate for path-to-string conversion

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_extract_simple_variable() {
    let expr: Expr = parse_quote!(x);
    let vars = extract_variables(&expr);
    assert_eq!(vars, vec!["x"]);
}

#[test]
fn test_extract_field_access() {
    let expr: Expr = parse_quote!(self.buffer);
    let vars = extract_variables(&expr);
    assert!(vars.contains(&"buffer".to_string())
         || vars.contains(&"self.buffer".to_string()));
}

#[test]
fn test_extract_multiple_args() {
    let args: Punctuated<Expr, Token![,]> = parse_quote!(name, age, status);
    let vars = extract_variables_from_args(&args);
    assert_eq!(vars, vec!["age", "name", "status"]); // Sorted
}

#[test]
fn test_extract_from_macro() {
    let mac: Macro = parse_quote!(println!("Value: {}", x));
    let vars = extract_variables_from_macro(&mac);
    assert!(vars.contains(&"x".to_string()));
}

#[test]
fn test_deduplication() {
    let args: Punctuated<Expr, Token![,]> = parse_quote!(x, x, y);
    let vars = extract_variables_from_args(&args);
    assert_eq!(vars, vec!["x", "y"]); // Deduplicated
}

#[test]
fn test_depth_limit() {
    // Very nested expression should not cause stack overflow
    let expr: Expr = parse_quote!(a.b.c.d.e.f.g.h);
    let vars = extract_variables(&expr);
    assert!(!vars.is_empty()); // Should extract something
}

#[test]
fn test_method_chain() {
    let expr: Expr = parse_quote!(data.clone());
    let vars = extract_variables(&expr);
    assert!(vars.contains(&"data".to_string()));
}

#[test]
fn test_reference_extraction() {
    let expr: Expr = parse_quote!(&buffer);
    let vars = extract_variables(&expr);
    assert_eq!(vars, vec!["buffer"]);
}
```

### Integration Tests

```rust
#[test]
fn test_io_operation_with_variables() {
    let code = parse_quote! {
        fn write_data(path: &str, content: String) {
            std::fs::write(path, content).unwrap();
        }
    };

    let ops = detect_io_operations(&code);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].operation_type, "file_io");
    assert!(ops[0].variables.contains(&"path".to_string()));
    assert!(ops[0].variables.contains(&"content".to_string()));
}

#[test]
fn test_println_with_variables() {
    let code = parse_quote! {
        fn log_status(name: &str, status: i32) {
            println!("User {} has status {}", name, status);
        }
    };

    let ops = detect_io_operations(&code);
    assert_eq!(ops.len(), 1);
    assert!(ops[0].variables.contains(&"name".to_string()));
    assert!(ops[0].variables.contains(&"status".to_string()));
}

#[test]
fn test_tui_displays_variables() {
    // Integration test using actual TUI rendering
    let item = create_test_debt_item_with_io();
    let data_flow = create_test_data_flow_graph();

    // Render to buffer and check output
    let output = render_data_flow_page(&item, &data_flow);

    // Should NOT contain "variables: )" (blank)
    assert!(!output.contains("variables: )"));
    // Should contain actual variable names
    assert!(output.contains("path") || output.contains("content"));
}
```

### Performance Tests

```rust
#[bench]
fn bench_variable_extraction(b: &mut Bencher) {
    let expr: Expr = parse_quote!(calculate(x, y, z));
    b.iter(|| {
        extract_variables(&expr);
    });
}

#[bench]
fn bench_complex_extraction(b: &mut Bencher) {
    let expr: Expr = parse_quote!(self.field.nested.value.method(a, b, c));
    b.iter(|| {
        extract_variables(&expr);
    });
}
```

Target: <5% overhead compared to I/O detection without variable extraction

## Documentation Requirements

### Code Documentation

```rust
/// Extract variables from an expression recursively up to MAX_DEPTH.
///
/// # Examples
///
/// ```
/// let expr: Expr = parse_quote!(x);
/// let vars = extract_variables(&expr);
/// assert_eq!(vars, vec!["x"]);
/// ```
///
/// ```
/// let expr: Expr = parse_quote!(self.buffer);
/// let vars = extract_variables(&expr);
/// // Returns either ["buffer"] or ["self.buffer"] depending on strategy
/// ```
///
/// # Depth Limiting
///
/// To prevent infinite recursion and excessive overhead, extraction stops
/// at MAX_DEPTH (default: 2). This means:
/// - `a.b.c` → `["a", "b", "c"]` (depth 2)
/// - `a.b.c.d.e` → extracts up to depth limit
///
/// # Deduplication
///
/// Variables are deduplicated and sorted:
/// - `format!("{} {}", x, x)` → `["x"]` (not `["x", "x"]`)
fn extract_variables_from_expr(expr: &Expr) -> Vec<String>
```

### User Documentation

Update book section on data flow analysis:

```markdown
## Data Flow Page (TUI)

The data flow page now shows variables involved in each I/O operation:

```
i/o operations
  file_io at line 42 (variables: path, content, buffer)
  console at line 87 (variables: name, status, timestamp)
  network at line 120 (variables: url, headers, body)
```

This helps you understand:
- What data is being written to files
- What variables are being logged
- What data flows over the network
```

### Architecture Updates

Update ARCHITECTURE.md:

```markdown
## Data Flow Analysis

### Variable Extraction (Spec 246)

When I/O operations are detected, the analyzer extracts variables involved
in each operation by traversing expression ASTs. This provides visibility into:

- Which variables are written to files
- What data is printed or logged
- What values are sent over the network

Variables are extracted up to a depth limit (2 levels) and deduplicated for
clean presentation in the TUI and reports.
```

## Implementation Notes

### Gotchas

1. **Macro argument parsing**: Macros aren't expanded by syn
   - Mitigation: Heuristic token parsing for common patterns
   - Accept some inaccuracy for macro-heavy code

2. **Type information**: Cannot determine if identifier is variable or type
   - Mitigation: Filter out known types (String, Vec, Option) if needed
   - Most false positives are harmless (listing types doesn't hurt)

3. **Complex expressions**: Deep nesting can cause performance issues
   - Mitigation: Depth limit (MAX_DEPTH = 2)
   - Focus on top-level contributing variables

4. **Path representation**: `self.field.nested` - store full path or just leaf?
   - Decision: Store full path for context (more useful in TUI)
   - Can be configured if needed

### Best Practices

- Prefer over-extraction to under-extraction (better to have extra vars than miss critical ones)
- Keep depth limit low (2) to avoid performance issues
- Always deduplicate and sort for consistent output
- Document edge cases and limitations clearly

### Performance Considerations

- AST traversal is inherently fast (linear in expression size)
- Depth limit prevents exponential blowup
- Deduplication is O(n log n) but n is small (typically <10 variables)
- Overall overhead should be <5% of total analysis time

## Migration and Compatibility

### Breaking Changes

None - this populates an existing field that was previously empty.

### Backward Compatibility

- Existing code reading `IoOperation.variables` will now get data instead of empty vec
- No API changes
- Fully backward compatible

### Migration Path

No migration needed - transparent improvement.

### Rollback Plan

Can easily disable variable extraction by returning empty vec if issues arise.

## Success Metrics

- **Coverage**: 90%+ of I/O operations have non-empty variable lists
- **Accuracy**: <5% false positives (non-variables included)
- **User feedback**: TUI data flow page is actionable and useful
- **Performance**: <5% overhead in total analysis time

## Future Enhancements (Out of Scope)

- **Type-aware extraction**: Use type information to filter out non-variables
- **Smart path abbreviation**: Show `buffer` instead of `self.inner.buffer` when unambiguous
- **Variable flow tracking**: Show where variables come from (assignment tracking)
- **Confidence scoring**: Indicate confidence in variable extraction (high for paths, low for macros)

---
number: 245
title: AST-Based I/O Operation Detection
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-12-06
---

# Specification 245: AST-Based I/O Operation Detection

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Currently, I/O operation detection in the data flow analysis is severely limited. The implementation in `src/data_flow/population.rs:101-132` uses a name-based heuristic that only checks if function names contain keywords like "read", "write", "print", "log", "fetch", or "request". This results in:

- **4.3% coverage**: Only 1 out of 23 debt items show data flow information
- **Empty variable lists**: All detected I/O operations have `variables: vec![]`
- **High false negative rate**: Misses ~95% of actual I/O operations with different naming conventions
- **No pattern detection**: Cannot detect I/O through `std::fs` APIs, network libraries, database operations, or system calls

The purity detector already has infrastructure (`has_io_operations: bool` flag in `src/analyzers/purity_detector.rs:18`) but lacks comprehensive I/O detection logic.

## Objective

Implement comprehensive AST-based I/O operation detection that analyzes Rust syntax trees to identify I/O operations based on actual code patterns rather than function names, achieving 70-80% coverage of real I/O operations.

## Requirements

### Functional Requirements

- **FR1**: Detect file I/O operations from `std::fs` module
  - `File::create`, `File::open`, `File::read`, `File::write_all`
  - `std::fs::read`, `std::fs::write`, `std::fs::read_to_string`
  - `BufReader`, `BufWriter` operations

- **FR2**: Detect console I/O operations
  - `println!`, `print!`, `eprintln!`, `eprint!` macros
  - `write!`, `writeln!` macros when used with stdout/stderr
  - `std::io::stdin()`, `std::io::stdout()`, `std::io::stderr()` calls

- **FR3**: Detect network I/O operations
  - `std::net::TcpStream`, `std::net::UdpSocket` operations
  - HTTP client library calls: `reqwest`, `hyper`, `ureq`
  - Common methods: `get`, `post`, `send`, `fetch`, `request`

- **FR4**: Detect async I/O operations
  - `tokio::fs` module operations
  - `async-std::fs` module operations
  - `.await` on I/O operations

- **FR5**: Detect database I/O operations
  - Database connection methods: `execute`, `query`, `prepare`
  - ORM operations from `diesel`, `sqlx`, `rusqlite`
  - Connection pool operations

- **FR6**: Classify I/O operation types
  - `file_io`, `console`, `network`, `database`, `system_call`
  - Support for custom classification based on receiver type and method name

### Non-Functional Requirements

- **NFR1**: Performance - Detection must add <10% overhead to analysis time
- **NFR2**: Accuracy - Achieve 70-80% true positive rate, <5% false positive rate
- **NFR3**: Extensibility - Easy to add new I/O pattern detection rules
- **NFR4**: Maintainability - Clear separation between detection logic and AST traversal

## Acceptance Criteria

- [ ] Create new `src/analyzers/io_detector.rs` module with AST visitor pattern
- [ ] Implement detection for all file I/O patterns (FR1) with test coverage
- [ ] Implement detection for all console I/O patterns (FR2) with test coverage
- [ ] Implement detection for network I/O patterns (FR3) with test coverage
- [ ] Implement detection for async I/O patterns (FR4) with test coverage
- [ ] Implement detection for database I/O patterns (FR5) with test coverage
- [ ] Replace `detect_io_from_metrics` in `src/data_flow/population.rs` with AST-based detector
- [ ] Achieve 70%+ coverage on real-world codebase (debtmap self-analysis)
- [ ] Add comprehensive unit tests with >90% code coverage
- [ ] Add integration test showing improvement from 4.3% to 70%+ coverage
- [ ] Document I/O detection patterns in module-level docs
- [ ] Performance benchmark shows <10% overhead increase

## Technical Details

### Implementation Approach

Create a new module `src/analyzers/io_detector.rs` that implements the `syn::visit::Visit` trait to traverse function ASTs and detect I/O patterns:

```rust
pub struct IoDetectorVisitor {
    operations: Vec<IoOperation>,
    current_line: usize,
    scope: ScopeTracker, // Track variable types for receiver analysis
}

impl<'ast> Visit<'ast> for IoDetectorVisitor {
    fn visit_expr_method_call(&mut self, expr: &ExprMethodCall) {
        // Detect I/O method calls
    }

    fn visit_macro(&mut self, mac: &Macro) {
        // Detect I/O macros
    }

    fn visit_expr_call(&mut self, call: &ExprCall) {
        // Detect direct function calls to I/O APIs
    }
}
```

### Architecture Changes

1. **New module**: `src/analyzers/io_detector.rs`
   - Public API: `pub fn detect_io_operations(item_fn: &ItemFn) -> Vec<IoOperation>`
   - Internal visitor implementation
   - Pattern matching logic

2. **Modified**: `src/data_flow/population.rs`
   - Replace `detect_io_from_metrics` with call to new detector
   - Requires reading file and parsing AST (already done for variable deps)

3. **Modified**: `src/analyzers/purity_detector.rs`
   - Integrate I/O detector during purity analysis
   - Set `has_io_operations` flag based on detector results

### Data Structures

```rust
// Existing in src/data_flow/mod.rs - no changes needed
pub struct IoOperation {
    pub operation_type: String,
    pub variables: Vec<String>, // Will be populated by spec 246
    pub line: usize,
}

// New internal structures in io_detector.rs
struct IoPattern {
    receiver_type: Option<&'static str>,
    method_name: &'static str,
    operation_type: &'static str,
}

const FILE_IO_PATTERNS: &[IoPattern] = &[
    IoPattern { receiver_type: Some("File"), method_name: "open", operation_type: "file_io" },
    IoPattern { receiver_type: Some("File"), method_name: "create", operation_type: "file_io" },
    // ... more patterns
];
```

### Pattern Matching Logic

**Type-based matching**:
- Infer receiver type from variable declarations in scope
- Match against known I/O types (File, TcpStream, Client, etc.)
- Use heuristics when type inference is unavailable

**Macro-based matching**:
- Pattern match on macro path segments
- Check for `std::io`, `std::fs`, `tokio::fs` prefixes
- Match macro names: `println`, `write`, etc.

**Method-based matching**:
- Combine receiver type + method name for classification
- Support wildcards for common patterns (e.g., any method on File)

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/data_flow/population.rs` (replace detection logic)
  - `src/analyzers/purity_detector.rs` (optional integration)
- **External Dependencies**:
  - `syn` crate (already used)
  - `proc-macro2` for span/line number extraction

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_detects_file_create() {
    let code = parse_quote! {
        fn test() {
            let f = File::create("test.txt")?;
        }
    };
    let ops = detect_io_operations(&code);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].operation_type, "file_io");
}

#[test]
fn test_detects_println_macro() {
    let code = parse_quote! {
        fn test() {
            println!("Hello, world!");
        }
    };
    let ops = detect_io_operations(&code);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].operation_type, "console");
}

#[test]
fn test_detects_network_request() {
    let code = parse_quote! {
        fn test() {
            let response = client.get("https://example.com").send()?;
        }
    };
    let ops = detect_io_operations(&code);
    assert!(ops.iter().any(|op| op.operation_type == "network"));
}
```

### Integration Tests

- **Coverage test**: Run analysis on debtmap itself, verify 70%+ items have I/O data
- **Comparison test**: Before/after coverage metrics
- **Real-world test**: Analyze sample Rust projects with known I/O patterns

### Performance Tests

```rust
#[bench]
fn bench_io_detection_overhead(b: &mut Bencher) {
    let metrics = load_test_metrics();
    b.iter(|| {
        populate_io_operations(&mut data_flow, &metrics);
    });
}
```

Target: <10% overhead compared to name-based detection

## Documentation Requirements

### Code Documentation

- Module-level documentation explaining detection strategy
- Document each pattern matching rule with examples
- Explain limitations and false positive/negative cases

```rust
//! AST-based I/O operation detection for Rust code.
//!
//! This module implements comprehensive I/O detection by analyzing Rust syntax trees
//! to identify I/O operations based on actual code patterns rather than function names.
//!
//! # Supported Patterns
//!
//! ## File I/O
//! ```rust,ignore
//! File::open("path") // Detected as file_io
//! std::fs::read_to_string("path") // Detected as file_io
//! ```
//!
//! ## Console I/O
//! ```rust,ignore
//! println!("message") // Detected as console
//! eprintln!("error") // Detected as console
//! ```
//!
//! # Limitations
//!
//! - Indirect I/O (functions calling I/O functions) not detected (see spec 248)
//! - Custom I/O wrappers may not be detected without explicit rules
//! - Type inference is limited to same-function scope
```

### User Documentation

- Update ARCHITECTURE.md section on data flow analysis
- Add examples to book showing improved I/O detection

### Architecture Updates

Update `ARCHITECTURE.md`:

```markdown
## Data Flow Analysis

### I/O Operation Detection (Spec 245)

I/O operations are detected through AST-based pattern matching rather than
name-based heuristics. The detector identifies:

- File I/O via std::fs and File methods
- Console I/O via println!/eprintln! macros
- Network I/O via std::net and HTTP libraries
- Database I/O via common ORM and driver patterns
- Async I/O via tokio::fs and async-std::fs

Coverage: ~70-80% of real I/O operations detected with <5% false positives.
```

## Implementation Notes

### Gotchas

1. **Type inference limitations**: Cannot infer types across function boundaries
   - Mitigation: Use heuristics based on variable names and patterns

2. **Macro expansion**: syn doesn't expand macros
   - Mitigation: Pattern match on macro invocation site

3. **Custom I/O wrappers**: Project-specific I/O abstractions not detected
   - Mitigation: Provide extensible pattern registration API (future work)

4. **Line number accuracy**: Macro spans may not correspond to actual source lines
   - Mitigation: Use `.start().line` from proc-macro2 spans

### Best Practices

- Start with high-confidence patterns (File, TcpStream, etc.)
- Add patterns incrementally based on real-world needs
- Document each pattern with example code
- Use conservative classification (prefer false negatives over false positives)

### Performance Considerations

- Visitor pattern is efficient (single AST traversal)
- Pattern matching is O(1) lookup in most cases
- Scope tracking adds minor overhead (acceptable for <10% target)

## Migration and Compatibility

### Breaking Changes

None - this is an additive change that improves existing functionality.

### Backward Compatibility

- Existing I/O operations detected by name will still be detected
- Coverage improvements are transparent to users
- No changes to `IoOperation` struct or public APIs

### Migration Path

1. Implement new detector alongside existing logic
2. Run both detectors in parallel during testing phase
3. Compare results and tune patterns
4. Replace old detector once confidence is high (70%+ coverage verified)

### Rollback Plan

If issues arise, can easily revert to name-based detection by keeping old function available.

## Success Metrics

- **Coverage**: Increase from 4.3% (1/23) to 70%+ of items showing I/O data
- **Accuracy**: <5% false positive rate on manual review
- **Performance**: <10% overhead in total analysis time
- **User feedback**: Positive feedback on data flow page usefulness in TUI

## Future Enhancements (Out of Scope)

- **Indirect I/O detection**: Track I/O through call graph (spec 248)
- **Custom pattern registration**: Allow users to define project-specific I/O patterns
- **Confidence scoring**: Assign confidence to each detection
- **Cross-file type inference**: Improve type inference across module boundaries

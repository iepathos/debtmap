---
number: 120
title: Fix Intra-Struct Method Call Tracking in Call Graph
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-25
---

# Specification 120: Fix Intra-Struct Method Call Tracking in Call Graph

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None (foundational call graph functionality)

## Context

During analysis of debtmap's output, we discovered that functions showing "0 callers" include methods that are **actually called** by other methods within the same struct. For example:

**PatternOutputFormatter::format_pattern_type()** (src/io/pattern_output.rs:67):
- Called by `format_pattern_usage()` at line 26: `self.format_pattern_type(&pattern.pattern_type)`
- Called by two other methods at lines 108 and 116
- **Yet shows "0 callers" in dependency scoring**

This suggests the call graph may not be tracking **intra-struct method calls** (methods calling other methods on `self`).

### Impact

Incorrect dependency scoring for methods leads to:
- Underestimating importance of internal helper methods
- Misleading "0 callers" messages for frequently-used methods
- Incorrect prioritization in technical debt recommendations
- User confusion and loss of trust in analysis accuracy

### Investigation Needed

We need to determine if this is:
1. A bug in call graph construction (not detecting `self.method()` calls)
2. A FunctionId matching issue (created vs lookup mismatch)
3. A limitation in AST analysis for method resolution
4. Working correctly but not displayed due to verbosity settings

## Objective

**Primary Goal**: Ensure the call graph accurately tracks method calls within the same struct, so that methods called via `self.method()` show correct caller counts.

**Secondary Goal**: Validate that FunctionId creation and lookup are consistent across the codebase.

## Requirements

### Functional Requirements

1. **Call Graph Must Track Intra-Struct Calls**
   - Detect when method A calls method B on `self` within same struct
   - Add edge from A → B in call graph
   - Update caller/callee lists correctly

2. **FunctionId Consistency**
   - FunctionIds created during call graph construction must match FunctionIds used during lookup
   - `module_path` field handling must be consistent
   - Hash/Eq implementations must work correctly

3. **Test Coverage**
   - Add test specifically for intra-struct method calls
   - Verify `self.method()` calls are tracked
   - Test with real-world example (e.g., PatternOutputFormatter)

### Non-Functional Requirements

- **Performance**: No degradation in call graph construction time
- **Accuracy**: 100% of intra-struct calls should be tracked
- **Compatibility**: No breaking changes to existing call graph API

## Acceptance Criteria

- [ ] Create test case for intra-struct method call tracking
- [ ] Test validates that `self.method()` calls appear in call graph
- [ ] Test uses real struct from codebase (e.g., PatternOutputFormatter)
- [ ] If test fails, identify root cause (AST parsing, FunctionId matching, etc.)
- [ ] Fix identified issue
- [ ] Verify PatternOutputFormatter::format_pattern_type() shows 3 callers
- [ ] Run full test suite to ensure no regressions
- [ ] Verify debtmap output shows correct caller counts for intra-struct calls

## Technical Details

### Implementation Approach

**Phase 1: Create Diagnostic Test** (1-2 hours)

```rust
#[test]
fn test_intra_struct_method_calls() {
    let code = r#"
        struct Formatter {
            plain: bool,
        }

        impl Formatter {
            pub fn format_output(&self, data: &str) -> String {
                // Calls helper method on self
                let formatted = self.format_helper(data);
                formatted
            }

            fn format_helper(&self, data: &str) -> String {
                data.to_uppercase()
            }
        }
    "#;

    let call_graph = build_call_graph_from_source(code);

    // Find the functions
    let format_output = find_function(&call_graph, "format_output");
    let format_helper = find_function(&call_graph, "format_helper");

    // Verify call is tracked
    let callers = call_graph.get_callers(&format_helper);
    assert!(
        !callers.is_empty(),
        "format_helper should have callers (format_output calls it via self.format_helper())"
    );

    let has_format_output = callers.iter().any(|c| c.name.contains("format_output"));
    assert!(
        has_format_output,
        "format_output should be in the list of callers for format_helper"
    );
}
```

**Phase 2: Debug if Test Fails**

Add logging to trace call graph construction:
1. Log when method calls are detected in AST
2. Log FunctionIds being created for caller/callee
3. Log when edges are added to graph
4. Compare FunctionIds at creation vs lookup time

**Phase 3: Identify Root Cause**

Possible issues to investigate:

1. **AST Parsing Issue**:
   - Check if `self.method()` calls are being detected
   - Verify method call expression handling in rust_call_graph.rs
   - Location: src/analyzers/rust_call_graph.rs or src/analyzers/call_graph/graph_builder.rs

2. **FunctionId Mismatch**:
   - FunctionId includes `module_path` field (src/priority/call_graph/types.rs:14)
   - Created with `FunctionId::new()` sets `module_path = ""`
   - If call graph uses non-empty module_path, Hash/Eq will fail
   - Location: src/priority/call_graph/types.rs:8-15

3. **Method Resolution**:
   - Trait method calls might be handled differently
   - Impl blocks might not be properly associated
   - Location: src/analyzers/trait_resolver.rs

**Phase 4: Implement Fix**

Depending on root cause:

**If AST Parsing Issue**:
```rust
// In rust_call_graph.rs or graph_builder.rs
fn visit_method_call_expr(&mut self, expr: &MethodCallExpr) {
    // Ensure we handle self.method() calls
    if let Some(receiver) = expr.receiver() {
        if is_self_reference(&receiver) {
            // This is an intra-struct call
            let callee_id = FunctionId::new(
                self.current_file.clone(),
                expr.name_ref().to_string(),
                expr.syntax().text_range().start().into()
            );
            self.add_call_edge(self.current_function_id.clone(), callee_id);
        }
    }
}
```

**If FunctionId Issue**:
```rust
// Option 1: Always use empty module_path for consistency
impl FunctionId {
    pub fn new(file: PathBuf, name: String, line: usize) -> Self {
        Self {
            file,
            name,
            line,
            module_path: String::new(), // ✓ Consistent
        }
    }
}

// Option 2: Exclude module_path from Hash/Eq
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
    #[serde(default)]
    #[hash_ignore] // Custom derive or manual impl
    pub module_path: String,
}
```

**If Method Resolution Issue**:
- Review trait resolution logic
- Ensure impl blocks are properly linked to structs
- Verify method name resolution in call graph construction

### Architecture Changes

No major architecture changes expected. This is a bug fix in existing call graph functionality.

**Modified Components**:
- `src/analyzers/rust_call_graph.rs` or `src/analyzers/call_graph/graph_builder.rs` (AST parsing)
- `src/priority/call_graph/types.rs` (FunctionId consistency, if needed)
- `tests/` (new test for intra-struct calls)

### Data Structures

No changes to core data structures. May need to adjust FunctionId Hash/Eq if that's the issue.

### APIs and Interfaces

No public API changes. Internal call graph construction only.

## Dependencies

- **Prerequisites**: None (this is foundational functionality)
- **Affected Components**:
  - Call graph construction (src/analyzers/rust_call_graph.rs)
  - FunctionId creation (src/priority/call_graph/types.rs)
  - Dependency scoring (src/priority/scoring/calculation.rs)
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_simple_intra_struct_call() {
    // Basic test: method A calls method B on self
}

#[test]
fn test_chained_intra_struct_calls() {
    // A calls B calls C, all on self
}

#[test]
fn test_mixed_internal_external_calls() {
    // Method calls both self.method() and external function
}

#[test]
fn test_function_id_consistency() {
    // Verify FunctionIds created at construction match lookup
    let func_id1 = FunctionId::new(path.clone(), "foo".to_string(), 100);
    let func_id2 = FunctionId::new(path.clone(), "foo".to_string(), 100);
    assert_eq!(func_id1, func_id2);
    assert_eq!(func_id1.fuzzy_key(), func_id2.fuzzy_key());
}
```

### Integration Tests

```rust
#[test]
fn test_real_world_pattern_output_formatter() {
    // Analyze actual PatternOutputFormatter from codebase
    let analysis = analyze_file("src/io/pattern_output.rs");

    // Find format_pattern_type method
    let format_pattern_type = find_function(&analysis.call_graph, "format_pattern_type");

    // Verify it has callers
    let callers = analysis.call_graph.get_callers(&format_pattern_type);
    assert!(
        callers.len() >= 3,
        "format_pattern_type should have at least 3 callers, found: {}",
        callers.len()
    );

    // Verify specific callers
    assert!(callers.iter().any(|c| c.name.contains("format_pattern_usage")));
}
```

### Regression Tests

- Run full existing test suite
- Verify no existing call graph tests break
- Check that cross-file calls still work
- Validate trait method resolution unaffected

### Manual Verification

```bash
# Analyze pattern_output.rs with high verbosity
cargo run --release --bin debtmap -- analyze src/io/pattern_output.rs -vv

# Look for format_pattern_type in output
# Should show 3 callers, not 0
```

## Documentation Requirements

### Code Documentation

1. Document intra-struct call handling in call graph builder
2. Add comments explaining FunctionId consistency requirements
3. Update ARCHITECTURE.md if call graph construction logic changes

### User Documentation

No user-facing documentation changes. This is an internal bug fix.

### Architecture Updates

If FunctionId handling changes, update ARCHITECTURE.md section on FunctionId:
- Explain module_path usage
- Document Hash/Eq implications
- Clarify when to use exact vs fuzzy matching

## Implementation Notes

### Known Challenges

1. **AST Complexity**: Rust method call resolution can be complex
   - Trait methods vs impl methods
   - Generic method instantiations
   - UFCS (universal function call syntax)

2. **Line Number Matching**: Method calls might not have exact line numbers
   - May need fuzzy matching by name + file
   - Could use FunctionId::fuzzy_key() for lookups

3. **Multiple Impl Blocks**: Struct might have multiple impl blocks
   - Need to handle all impl blocks for same struct
   - Verify calls work across impl block boundaries

### Testing Priorities

1. **CRITICAL**: Simple intra-struct call (`self.method()`)
2. **HIGH**: Real-world example (PatternOutputFormatter)
3. **MEDIUM**: Chained calls, trait method calls
4. **LOW**: Edge cases (generics, macros, UFCS)

### Debug Strategy

If test fails, add logging:
```rust
eprintln!("DEBUG: Detected method call: {} -> {}", caller, callee);
eprintln!("DEBUG: Caller FunctionId: {:?}", caller_id);
eprintln!("DEBUG: Callee FunctionId: {:?}", callee_id);
eprintln!("DEBUG: FunctionIds equal: {}", caller_id == callee_id);
eprintln!("DEBUG: Hash caller: {:?}, Hash callee: {:?}",
          calculate_hash(&caller_id), calculate_hash(&callee_id));
```

## Migration and Compatibility

### Breaking Changes

None. This is a bug fix that improves accuracy.

### Output Changes

Users will see **different output** after this fix:
- **Before**: Methods show "0 callers" incorrectly
- **After**: Methods show actual caller count

This is an **improvement** in accuracy, not a breaking change.

### Performance Impact

Expected: Negligible to none. We're fixing existing logic, not adding new expensive operations.

## Success Metrics

### Immediate Success

- [ ] Test for intra-struct calls passes
- [ ] PatternOutputFormatter::format_pattern_type shows 3 callers
- [ ] All existing tests still pass
- [ ] No performance regression in call graph construction

### Long-Term Success

- [ ] Dependency scores are more accurate for helper methods
- [ ] User feedback indicates improved recommendation quality
- [ ] Fewer "0 callers" anomalies in debtmap output

## Related Issues

This fix addresses the issue discovered during evaluation of debtmap output where:
- All top 10 items showed "0 callers"
- Investigation revealed some were entry points (expected)
- But others like `format_pattern_type()` had clear callers that weren't tracked

This spec focuses specifically on the intra-struct method call tracking issue.

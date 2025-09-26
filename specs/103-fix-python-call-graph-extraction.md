---
number: 103
title: Fix Python Call Graph Extraction and Testing
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-09-26
---

# Specification 103: Fix Python Call Graph Extraction and Testing

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The Python call graph extraction in debtmap is currently non-functional. Analysis reveals that while the infrastructure exists for building Python call graphs through the `TwoPassExtractor`, a critical bug prevents any edges from being added to the graph. This results in all Python functions showing 0 callers and 0 callees, making dead code detection and dependency analysis impossible for Python codebases.

The issue was discovered when analyzing the promptconstruct-frontend project, where debtmap incorrectly identifies many functions as having no callers (potentially dead code) when they are actually called. This false positive rate makes the tool unreliable for Python projects.

## Objective

Fix the Python call graph extraction to correctly identify function calls and method invocations, enabling accurate dead code detection, dependency analysis, and complexity scoring for Python codebases.

## Requirements

### Functional Requirements

1. **Correct Two-Pass Extraction**
   - Phase 1 must register all function definitions before attempting call resolution
   - Phase 2 must correctly resolve calls using the registered functions
   - Support for module-level functions, class methods, static methods, and nested functions

2. **Call Resolution**
   - Direct function calls (e.g., `func()`)
   - Method calls (e.g., `obj.method()`)
   - Static method calls (e.g., `Class.static_method()`)
   - Class method calls (e.g., `cls.class_method()`)
   - Nested function calls
   - Cross-module calls with proper import resolution

3. **Function Registration**
   - Track all function definitions with correct fully-qualified names
   - Maintain proper scope for nested functions
   - Handle class method namespacing correctly
   - Support async functions

4. **Edge Cases**
   - Dynamic calls through `getattr`
   - Callback patterns (functions passed as arguments)
   - Decorators that wrap functions
   - Lambda expressions
   - List comprehensions with function calls

### Non-Functional Requirements

1. **Performance**
   - Maintain current analysis speed (< 100ms for small projects)
   - Efficient lookup for function resolution
   - Minimal memory overhead for tracking

2. **Accuracy**
   - Zero false negatives for direct function calls
   - < 5% false positive rate for dead code detection
   - Correct handling of Python-specific patterns

3. **Testing**
   - Comprehensive test coverage for all call patterns
   - Integration tests with real Python projects
   - Regression tests for the current bug

## Acceptance Criteria

- [ ] Simple function calls are correctly tracked (main() calls helper())
- [ ] Method calls within classes are resolved (self.method())
- [ ] Cross-class method calls work (obj.method())
- [ ] Static and class methods are properly handled
- [ ] Nested functions are correctly scoped and tracked
- [ ] Test file demonstrates all call patterns working
- [ ] promptconstruct-frontend analysis shows proper caller/callee counts
- [ ] All existing Python tests continue to pass
- [ ] New comprehensive test suite achieves 90%+ coverage of call graph code
- [ ] Call graph information appears in default output (verbosity level 0)

## Technical Details

### Implementation Approach

1. **Fix TwoPassExtractor Logic**
   ```rust
   // Current broken logic at line 760:
   if self.call_graph.get_function_info(&func_id).is_some() {
       return Some(func_id);
   }

   // Should be:
   if self.known_functions.contains(&func_id) {
       return Some(func_id);
   }
   ```

2. **Track Functions Separately**
   - Add a `HashSet<FunctionId>` to track all discovered functions
   - Populate during phase one when analyzing function definitions
   - Use for resolution in phase two

3. **Improve Function Name Resolution**
   - Build fully-qualified names including module path
   - Handle relative imports correctly
   - Support namespace resolution for nested scopes

### Architecture Changes

1. **TwoPassExtractor Enhancement**
   ```rust
   pub struct TwoPassExtractor {
       phase_one_calls: Vec<UnresolvedCall>,
       type_tracker: PythonTypeTracker,
       call_graph: CallGraph,
       known_functions: HashSet<FunctionId>, // NEW
       current_function: Option<FunctionId>,
       current_class: Option<String>,
   }
   ```

2. **Function Discovery**
   - Register functions during phase one traversal
   - Store with proper scoping information
   - Include line number information when available

### Data Structures

```rust
// Enhanced function tracking
struct FunctionRegistry {
    functions: HashMap<String, FunctionId>,
    scopes: Vec<Scope>,
    current_module: PathBuf,
}

struct Scope {
    scope_type: ScopeType,
    name: String,
    functions: HashSet<String>,
}

enum ScopeType {
    Module,
    Class(String),
    Function(String),
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analysis/python_type_tracker.rs`
  - `src/analysis/python_call_graph/`
  - `src/builders/call_graph.rs`
  - `src/priority/formatter_verbosity.rs`
- **External Dependencies**: rustpython_parser (existing)

## Testing Strategy

### Unit Tests
- Test each call pattern individually
- Verify function registration logic
- Test name resolution with various scopes
- Validate edge cases like lambdas and comprehensions

### Integration Tests
- Create comprehensive Python test file with all patterns
- Test against real projects (pytest, django app, flask app)
- Verify JSON output contains correct caller/callee data
- Test with coverage data integration

### Performance Tests
- Benchmark against large Python codebases
- Ensure no regression in analysis speed
- Memory usage profiling for large projects

### Test Coverage Requirements
- Achieve 90%+ line coverage for python_call_graph module
- 100% coverage for the bug fix itself
- All test patterns from test_python_call_graph.py must pass

## Documentation Requirements

### Code Documentation
- Document the two-pass algorithm clearly
- Explain function resolution strategy
- Add examples of supported call patterns

### User Documentation
- Update README with Python support details
- Add troubleshooting guide for Python analysis
- Include examples of Python call graph output

### Architecture Updates
- Update ARCHITECTURE.md with Python call graph details
- Document the two-pass extraction process
- Add sequence diagram for call resolution

## Implementation Notes

1. **Debugging Support**
   - Add debug logging for function registration
   - Trace call resolution attempts
   - Provide verbose output option for call graph building

2. **Incremental Approach**
   - First fix basic function calls
   - Then add method resolution
   - Finally handle edge cases

3. **Compatibility**
   - Ensure Rust call graph continues to work
   - Maintain backward compatibility for JSON output
   - Keep existing CLI interface unchanged

## Migration and Compatibility

During the prototype phase, breaking changes are allowed. However, this fix should be backward compatible:
- Existing JSON schema remains unchanged (just populated correctly)
- CLI interface stays the same
- Performance characteristics maintained or improved

## Validation Approach

1. **Before Fix**
   ```bash
   ./target/debug/debtmap analyze test_python_debug.py -f json
   # Shows: "upstream_callers": [], "downstream_callees": []
   ```

2. **After Fix**
   ```bash
   ./target/debug/debtmap analyze test_python_debug.py -f json
   # Shows: "upstream_callers": ["main"], "downstream_callees": ["print"]
   ```

3. **Real Project Validation**
   - Run on promptconstruct-frontend
   - Verify init methods show callers
   - Check that UI event handlers aren't marked as dead code
   - Validate cross-file call tracking

## Risk Assessment

- **Low Risk**: Changes are isolated to Python analysis
- **No Breaking Changes**: Fix populates existing empty fields
- **Performance Impact**: Minimal, adds one HashSet lookup
- **Testing Risk**: Comprehensive test suite mitigates regression risk
# Implementation Plan: Refactor Python Exception Flow Analyzer

## Problem Summary

**Location**: ./src/analyzers/python_exception_flow.rs:file:0
**Priority Score**: 55.74
**Debt Type**: God Object (File-level)
**Current Metrics**:
- Lines of Code: 1521
- Functions: 71 (34 impl methods, 12 module-level functions)
- Cyclomatic Complexity: 199 (max: 21, avg: 2.8)
- Coverage: 0%
- Responsibilities: 4 (Construction, Validation, Parsing & Input, Utilities)
- God Object Score: 1.0 (highest severity)

**Issue**: URGENT: 1521 lines, 71 functions! This file has extreme god object characteristics with multiple distinct responsibilities packed into a single module. The `ExceptionFlowAnalyzer` struct has 24 methods handling everything from parsing to analysis to output formatting. The recommended approach is to split by data flow: 1) Input/parsing functions 2) Core logic/transformation 3) Output/formatting, creating 3 focused modules with <30 functions each.

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 39.8 points (20% reduction)
- Maintainability Improvement: 5.57 points (10% improvement)
- Test Effort: 152.1 (will need comprehensive testing after refactor)

**Success Criteria**:
- [ ] File split into 3 focused modules (<500 lines each)
- [ ] Each module has single responsibility
- [ ] `ExceptionFlowAnalyzer` reduced to orchestration only (<150 lines)
- [ ] All 71 functions redistributed by responsibility
- [ ] Pure functions separated from stateful analysis
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper module structure with clear boundaries
- [ ] Test coverage >80% for new pure functions

## Implementation Phases

### Phase 1: Extract Type Definitions Module

**Goal**: Extract all data types and enums into a focused types module

**Changes**:
- Create `src/analyzers/python_exception_flow/types.rs`
- Move all type definitions:
  - `ExceptionInfo`, `ExceptionType`, `BuiltinException`
  - `ExceptionFlow`, `CaughtException`, `HandlerType`, `HandlerAction`
  - `ExceptionTransformation`, `ExceptionClass`, `DocumentedException`
  - `ExceptionFlowPattern`, `ExceptionPatternType`, `Severity`
  - `ExceptionGraph`, `FunctionExceptions`
  - Constants: `BUILTIN_EXCEPTIONS`, `BUILTIN_EXCEPTION_HIERARCHY`
- Move pure helper: `find_parent_exception`
- Keep methods with types (ExceptionType methods, etc.)
- Update visibility (pub where needed)

**Testing**:
- `cargo check` - verify compilation
- `cargo test python_exception` - run existing tests
- Verify all type imports work

**Success Criteria**:
- [ ] ~400 lines moved to types.rs
- [ ] All types accessible via module path
- [ ] Zero compilation errors
- [ ] All existing tests pass

### Phase 2: Extract Parsing Functions Module

**Goal**: Extract all parsing and input processing into a focused module

**Changes**:
- Create `src/analyzers/python_exception_flow/parsing.rs`
- Move docstring parsing functions (6 functions, ~220 lines):
  - `extract_docstring`
  - `parse_exception_documentation`
  - `parse_google_raises`
  - `parse_numpy_raises`
  - `parse_sphinx_raises`
  - `process_sphinx_line`
- Move AST extraction methods from `ExceptionFlowAnalyzer` (6 methods):
  - `extract_class_name`
  - `extract_exception_from_expr`
  - `extract_exception_types`
  - `extract_exception_type`
  - `extract_exception_docs`
  - `extract_exception_docs_async`
- All functions should be pure (no mutation)
- Group related functions together

**Testing**:
- Test each parsing function independently
- Verify Google/NumPy/Sphinx docstring parsing
- Test AST extraction functions
- `cargo test python_exception` - all tests pass

**Success Criteria**:
- [ ] ~250 lines moved to parsing.rs
- [ ] All parsing functions are pure
- [ ] Clear separation: docstring vs AST parsing
- [ ] All existing tests pass
- [ ] No clippy warnings

### Phase 3: Extract Pattern Detection Module

**Goal**: Extract pattern detection functions into a focused module

**Changes**:
- Create `src/analyzers/python_exception_flow/patterns.rs`
- Move pure detection functions (5 functions, ~180 lines):
  - `detect_undocumented_exceptions`
  - `detect_documented_not_raised`
  - `detect_handler_patterns`
  - `detect_log_and_ignore`
  - `detect_transformation_lost`
- All functions are already pure (no state)
- Keep together for cohesion

**Testing**:
- Test each pattern detection function independently
- Create unit tests for edge cases
- `cargo test python_exception` - all tests pass

**Success Criteria**:
- [ ] ~180 lines moved to patterns.rs
- [ ] All functions remain pure
- [ ] Pattern detection logic isolated
- [ ] All existing tests pass
- [ ] New unit tests for each detector

### Phase 4: Extract Analysis Core Module

**Goal**: Extract core analysis logic into focused module

**Changes**:
- Create `src/analyzers/python_exception_flow/analysis.rs`
- Move analysis methods from `ExceptionFlowAnalyzer` (13 methods, ~300 lines):
  - `analyze_module`
  - `register_custom_exceptions`
  - `analyze_statement`
  - `analyze_function`
  - `analyze_async_function`
  - `analyze_function_body`
  - `track_raise`
  - `analyze_try_statement`
  - `analyze_handler`
  - `determine_handler_action`
  - `has_logging_call`
  - `is_logging_expr`
  - Helper validators: `is_exception_class`, `is_exception_type`
- Refactor `ExceptionFlowAnalyzer` to delegate to these functions
- Keep analyzer as thin orchestration layer

**Testing**:
- Test analysis functions with sample AST
- Verify exception tracking
- Test handler detection
- `cargo test python_exception` - all tests pass

**Success Criteria**:
- [ ] ~350 lines moved to analysis.rs
- [ ] Clear separation of concerns
- [ ] Analyzer becomes orchestration only
- [ ] All existing tests pass
- [ ] Improved testability

### Phase 5: Create Module Structure and Final Integration

**Goal**: Reorganize into proper module hierarchy and create clean public API

**Changes**:
- Create `src/analyzers/python_exception_flow/mod.rs`
- Define module structure:
  ```rust
  mod types;
  mod parsing;
  mod patterns;
  mod analysis;

  pub use types::{ExceptionGraph, FunctionExceptions, ExceptionFlowPattern};
  pub use analysis::ExceptionFlowAnalyzer;
  ```
- Refactor main analyzer to use submodules
- Reduce `ExceptionFlowAnalyzer` to <150 lines (orchestration only)
- Methods: `new`, `analyze_module`, `build_exception_graph`, `patterns_to_debt_items`, `detect_patterns`
- Update `src/analyzers/mod.rs` to reference new module structure
- Verify all imports work across codebase

**Testing**:
- Run full test suite: `cargo test`
- Test module visibility and API
- Integration tests for full workflow
- `cargo clippy` - no warnings

**Success Criteria**:
- [ ] Module hierarchy established
- [ ] Clean public API exported
- [ ] Main analyzer <150 lines
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Documentation builds: `cargo doc`

## Testing Strategy

**For each phase**:
1. Run `cargo check` to verify compilation
2. Run `cargo test python_exception` to verify existing tests
3. Run `cargo clippy` to check for warnings
4. Verify module boundaries are clean

**Final verification**:
1. `cargo test --all-features` - Full test suite
2. `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
3. `cargo fmt --all -- --check` - Code formatted
4. `cargo doc --no-deps` - Documentation builds
5. Visual inspection of module structure
6. Compare complexity metrics (should see reduction)

**New tests to add**:
- Unit tests for each parsing function
- Unit tests for each pattern detector
- Integration tests for full analysis workflow
- Property tests for invariants (e.g., all raised exceptions are tracked)

## Module Structure (Target)

```
src/analyzers/python_exception_flow/
├── mod.rs              (~50 lines) - Module declarations, public API
├── types.rs            (~400 lines) - All type definitions
├── parsing.rs          (~250 lines) - Docstring & AST parsing
├── patterns.rs         (~180 lines) - Pattern detection
└── analysis.rs         (~350 lines) - Core analysis logic
```

**Before**: 1 file, 1521 lines, 71 functions, 4 responsibilities
**After**: 5 files, ~1230 lines of implementation, clear separation

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the compilation/test errors
3. Adjust the plan based on discovered dependencies
4. Retry with refined approach

## Dependency Considerations

**Critical dependencies to watch**:
- `rustpython_parser::ast` - Used throughout, ensure imports in each module
- `CallGraph`, `FunctionId` - Only used in `build_exception_graph`
- `DebtItem`, `Priority` - Only used in `patterns_to_debt_items`
- HashMap/HashSet - Used in multiple modules

**Import strategy**:
- Each module imports what it needs
- Use `crate::` paths for internal types
- Re-export through `mod.rs` for public API

## Notes

**Why this structure works**:
- **Types module**: No dependencies on other modules, pure data
- **Parsing module**: Depends on types, all pure functions
- **Patterns module**: Depends on types, all pure functions
- **Analysis module**: Depends on all, orchestrates the workflow
- **Mod.rs**: Thin layer, just exports and declarations

**Functional programming alignment**:
- Parsing functions are already pure ✓
- Pattern detectors are already pure ✓
- Analysis functions work with immutable AST ✓
- Only `ExceptionFlowAnalyzer` has mutable state (HashMap fields)
- Clear data flow: Parse → Analyze → Detect Patterns → Convert to Debt

**Complexity reduction**:
- Breaking into focused modules reduces cognitive load
- Each file now has single responsibility
- Easier to test individual functions
- Better encapsulation and modularity
- Aligns with Rust best practices

**Testing approach**:
- Pure functions are trivial to test
- Can test parsing without analysis
- Can test patterns without full analysis
- Integration tests verify full workflow
- Aim for >80% coverage on pure functions

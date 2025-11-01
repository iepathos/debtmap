# Implementation Plan: Refactor Python Type Tracker God Object

## Target Confirmation

**Target**: `src/analysis/python_type_tracker/mod.rs:file:0`
**Priority Score**: 180.74
**Debt Type**: God Object (File-level)
**Action**: Split 3096-line file with 106 functions into focused modules

## Problem Summary

**Location**: `./src/analysis/python_type_tracker/mod.rs:file:0`
**Priority Score**: 180.74056620634124
**Debt Type**: God Object (File-level complexity)

**Current Metrics**:
- Lines of Code: 3096
- Total Functions: 106
- Classes/Structs: 5 (PythonTypeTracker, UnresolvedCall, TwoPassExtractor)
- Average Cyclomatic Complexity: 3.25
- Total Cyclomatic Complexity: 344
- Max Complexity: 13
- Test Coverage: 0.0%
- Responsibilities: 8 (Parsing & Input, Data Access, Filtering & Selection, Persistence, Utilities, Processing, Construction, Validation)

**God Object Indicators**:
- God Object Score: 1.0 (maximum)
- PythonTypeTracker: 10 fields, 28 methods, 5 responsibilities
- TwoPassExtractor: 12 fields, 41 methods
- Module spans lines 25-2368 with minimal separation
- Recommended splits: 2 modules (parsing & utilities)

**Issue**: URGENT: 3096 lines, 106 functions! This file violates the single responsibility principle by combining:
1. Type inference logic (PythonTypeTracker)
2. Call graph extraction (TwoPassExtractor)
3. Import resolution
4. Framework pattern detection
5. Observer pattern tracking
6. Type flow analysis
7. Callback tracking

The codebase has already started refactoring (types.rs, utils.rs exist), but the bulk of the logic remains in mod.rs.

## Target State

**Expected Impact**:
- Complexity Reduction: 68.8 points
- Maintainability Improvement: 18.07%
- Test Effort: 309.6 (indicates substantial testing will be needed)

**Success Criteria**:
- [ ] Main mod.rs reduced to <200 lines (orchestration only)
- [ ] Each new module has <400 lines and <25 functions
- [ ] PythonTypeTracker split into focused sub-components
- [ ] TwoPassExtractor split into phase-specific modules
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with cargo fmt
- [ ] Public API remains backward compatible

**Module Structure Target**:
```
src/analysis/python_type_tracker/
├── mod.rs                    # Main orchestration (<200 lines)
├── types.rs                  # Core types (DONE)
├── utils.rs                  # Utilities (DONE)
├── inference.rs              # Type inference logic (~400 lines)
├── import_resolver.rs        # Import tracking and resolution (~200 lines)
├── call_extraction.rs        # Call extraction phase one (~500 lines)
├── call_resolution.rs        # Call resolution phase two (~400 lines)
├── observer_patterns.rs      # Observer pattern detection (~400 lines)
├── framework_detection.rs    # Framework entry point detection (~200 lines)
└── tests/                    # Module-specific tests
```

## Implementation Phases

### Phase 1: Extract Import Resolution Module

**Goal**: Separate import tracking and resolution logic into dedicated module

**Changes**:
- Create `src/analysis/python_type_tracker/import_resolver.rs`
- Extract these methods from PythonTypeTracker (lines ~455-537):
  - `register_import`
  - `register_from_import`
  - `resolve_imported_name`
  - `track_import_stmt`
  - `track_import_from_stmt`
  - `is_imported_name`
  - `get_import_module`
  - `get_all_imports`
- Create `ImportResolver` struct to hold import state
- Update PythonTypeTracker to delegate to ImportResolver
- Maintain backward compatibility via delegation

**Testing**:
- Run `cargo test --lib` to ensure existing tests pass
- Verify import resolution still works correctly
- Check that cross-module context integration works

**Success Criteria**:
- [ ] import_resolver.rs created with ~150-200 lines
- [ ] ImportResolver struct owns imports and from_imports maps
- [ ] All import-related tests pass
- [ ] No clippy warnings
- [ ] Code compiles successfully

### Phase 2: Extract Type Inference Logic

**Goal**: Move pure type inference functions to dedicated module

**Changes**:
- Create `src/analysis/python_type_tracker/inference.rs`
- Extract these methods from PythonTypeTracker (lines ~137-284):
  - `infer_type` (pure function)
  - `infer_constant_type` (pure)
  - `resolve_attribute` (pure, uses class hierarchy)
  - `infer_call_return_type` (uses function signatures)
  - `infer_binop_type` (pure)
  - `infer_type_from_annotation` (pure)
- Create pure functions that take state as parameters
- Keep class hierarchy and function signatures in main struct
- Make functions testable in isolation

**Testing**:
- Add unit tests for each pure inference function
- Test edge cases (None values, complex types, generics)
- Verify type inference accuracy hasn't regressed
- Run `cargo test inference` to test the module

**Success Criteria**:
- [ ] inference.rs created with ~250-300 lines
- [ ] All type inference functions are pure (no &mut self)
- [ ] Comprehensive unit tests added (aim for 80%+ coverage)
- [ ] Type inference tests pass
- [ ] Documentation with examples for each function

### Phase 3: Extract Framework Detection

**Goal**: Separate framework pattern detection into its own module

**Changes**:
- Create `src/analysis/python_type_tracker/framework_detection.rs`
- Move framework-related logic:
  - `detect_frameworks_from_imports` from PythonTypeTracker
  - `is_framework_entry_point` from TwoPassExtractor
  - Framework pattern matching logic
- Create `FrameworkDetector` that wraps FrameworkPatternRegistry
- Simplify integration points

**Testing**:
- Test detection of various frameworks (FastAPI, Flask, Django, Click)
- Verify entry point detection accuracy
- Test decorator recognition
- Run `cargo test framework` for module tests

**Success Criteria**:
- [ ] framework_detection.rs created with ~150-200 lines
- [ ] FrameworkDetector encapsulates framework logic
- [ ] Tests for each supported framework
- [ ] No false positives in entry point detection
- [ ] Clear documentation of supported frameworks

### Phase 4: Split TwoPassExtractor - Phase One Extraction

**Goal**: Extract call collection logic into dedicated module

**Changes**:
- Create `src/analysis/python_type_tracker/call_extraction.rs`
- Extract phase one methods from TwoPassExtractor (lines ~709-1698):
  - `phase_one`
  - `analyze_stmt_phase_one`
  - `analyze_function_phase_one`
  - `analyze_async_function_phase_one`
  - `analyze_stmt_in_function`
  - `analyze_expr_for_calls`
  - `create_unresolved_call`
  - `track_collection_operation`
  - `extract_full_attribute_name`
- Create `CallExtractor` struct with minimal state
- Use functional pipeline: AST → Unresolved Calls
- Keep phase one and two clearly separated

**Testing**:
- Test extraction from various Python constructs
- Verify all call types are captured
- Test nested function calls
- Test method calls with complex receivers
- Run `cargo test call_extraction`

**Success Criteria**:
- [ ] call_extraction.rs created with ~400-500 lines
- [ ] CallExtractor is focused on collection only
- [ ] Clean separation from resolution logic
- [ ] All call extraction tests pass
- [ ] No missed call sites in test cases

### Phase 5: Extract Observer Pattern Detection

**Goal**: Move observer pattern tracking to separate module

**Changes**:
- Create `src/analysis/python_type_tracker/observer_patterns.rs`
- Extract observer-related methods from TwoPassExtractor (lines ~826-1417):
  - `infer_interface_from_field_name`
  - `register_observer_interfaces`
  - `find_observer_collections`
  - `collect_type_ids_for_observers`
  - `register_observer_interfaces_from_usage`
  - `discover_observer_interfaces_from_usage`
  - `analyze_dispatch_loops_for_interface_methods`
  - `find_and_register_interface_methods_in_function`
  - `process_stmt_for_dispatch_loops`
  - `extract_observer_dispatch_info`
  - `extract_method_calls_on_var`
  - `extract_method_calls_from_expr`
  - `infer_and_register_type_from_expr`
  - `populate_observer_registry`
  - `register_observer_implementations`
  - `store_pending_observer_dispatches`
  - `detect_observer_dispatch`
- Create `ObserverPatternDetector` struct
- Keep observer registry access clean

**Testing**:
- Test observer pattern recognition
- Test interface inference from field names
- Test dispatch loop detection
- Test false positive filtering
- Run `cargo test observer_patterns`

**Success Criteria**:
- [ ] observer_patterns.rs created with ~400-500 lines
- [ ] ObserverPatternDetector encapsulates observer logic
- [ ] Observer registry integration clean
- [ ] Comprehensive tests for observer detection
- [ ] Documentation of observer pattern heuristics

### Phase 6: Extract Call Resolution Logic

**Goal**: Separate call resolution (phase two) into its own module

**Changes**:
- Create `src/analysis/python_type_tracker/call_resolution.rs`
- Extract phase two methods from TwoPassExtractor (lines ~2148-2335):
  - `phase_two`
  - `resolve_call`
  - `check_for_callback_patterns`
  - `extract_callback_expr`
  - `check_for_event_bindings`
  - `is_main_guard`
  - `analyze_main_block`
  - `analyze_stmt_in_main_block`
- Create `CallResolver` struct
- Use type tracker, known functions, and import context for resolution
- Make resolution logic testable with mock data

**Testing**:
- Test resolution of various call types
- Test callback pattern recognition
- Test event binding detection
- Test main guard detection
- Test cross-module resolution
- Run `cargo test call_resolution`

**Success Criteria**:
- [ ] call_resolution.rs created with ~300-400 lines
- [ ] CallResolver cleanly separates resolution logic
- [ ] Callback tracking properly integrated
- [ ] All resolution tests pass
- [ ] Cross-module resolution works correctly

### Phase 7: Consolidate and Optimize Main Module

**Goal**: Reduce mod.rs to thin orchestration layer

**Changes**:
- Update `mod.rs` to coordinate sub-modules
- Keep only:
  - Module declarations
  - Public re-exports for backward compatibility
  - High-level orchestration functions
  - PythonTypeTracker struct (now much smaller)
  - TwoPassExtractor struct (now delegates to sub-modules)
- Ensure clean module boundaries
- Document module structure

**Refactoring**:
- PythonTypeTracker delegates to:
  - ImportResolver for import operations
  - Inference functions for type operations
  - FrameworkDetector for framework detection
- TwoPassExtractor delegates to:
  - CallExtractor for phase one
  - ObserverPatternDetector for observer patterns
  - CallResolver for phase two
- Remove duplicate code
- Optimize hot paths if needed

**Testing**:
- Run full test suite: `cargo test`
- Run integration tests
- Verify backward compatibility
- Test from external crates if any

**Success Criteria**:
- [ ] mod.rs reduced to <200 lines
- [ ] All sub-modules properly integrated
- [ ] Full test suite passes
- [ ] No clippy warnings
- [ ] No performance regression
- [ ] Documentation updated

### Phase 8: Add Module-Level Tests and Documentation

**Goal**: Ensure each module has comprehensive tests and documentation

**Changes**:
- Add module-level documentation for each file
- Add examples in doc comments
- Create integration tests in `tests/` directory
- Add property-based tests for pure functions (using proptest)
- Document module responsibilities and boundaries
- Add ARCHITECTURE.md section describing the refactoring

**Testing**:
- Achieve 80%+ test coverage for each module
- Run `cargo test --all-features`
- Run `cargo tarpaulin` to measure coverage
- Ensure documentation builds: `cargo doc --no-deps`

**Success Criteria**:
- [ ] Each module has comprehensive documentation
- [ ] Module-level examples in doc comments
- [ ] Test coverage >80% per module
- [ ] Integration tests cover cross-module interactions
- [ ] cargo doc builds without warnings
- [ ] README or ARCHITECTURE.md updated

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` to verify existing tests pass
2. Run `cargo clippy --all-targets --all-features -- -D warnings`
3. Run `cargo fmt --all -- --check`
4. Add new tests for extracted functionality
5. Verify no performance regression

**Between phases**:
1. Commit working code with clear message
2. Run full test suite
3. Check for any new clippy warnings

**Final verification**:
1. `cargo test --all-features` - All tests pass
2. `cargo clippy --all-targets --all-features -- -D warnings` - No warnings
3. `cargo fmt --all` - Code formatted
4. `cargo doc --no-deps` - Documentation builds
5. `cargo tarpaulin --out Xml` - Coverage measurement
6. `debtmap analyze` - Verify metrics improved

**Performance Testing**:
- Benchmark type inference on large Python files
- Benchmark call graph extraction
- Compare before/after performance
- Ensure no >10% regression

## Rollback Plan

**If a phase fails**:
1. Review the error carefully
2. Determine if it's a test issue or logic issue
3. If logic issue: revert with `git reset --hard HEAD~1`
4. If test issue: fix the test, don't disable it
5. Adjust the plan if needed
6. Document the issue and solution

**Phase-specific rollback**:
- Each phase creates one module file
- Can easily remove new file and restore mod.rs
- Git commits between phases allow easy rollback
- Keep original code until all tests pass

## Notes

**Key Principles**:
1. **Maintain backward compatibility** - Existing public API must work
2. **One phase at a time** - Don't try to do everything at once
3. **Test continuously** - Run tests after every change
4. **Functional over imperative** - Extract pure functions where possible
5. **Document as you go** - Don't defer documentation to the end

**Potential Challenges**:
1. **State management**: Many methods share mutable state
   - Solution: Pass state as parameters, use builder pattern
2. **Circular dependencies**: Modules may reference each other
   - Solution: Define clear dependency direction
3. **Test complexity**: Some logic is hard to test in isolation
   - Solution: Extract pure functions, use dependency injection
4. **Performance**: Multiple modules may add overhead
   - Solution: Benchmark and optimize hot paths

**Success Metrics**:
- Lines per file: <400 (target met when all <400)
- Functions per module: <25 (target met when all <25)
- Cyclomatic complexity: <10 per function (already mostly met)
- Test coverage: >80% (measure with cargo tarpaulin)
- God object score: <0.5 (verify with debtmap after refactoring)

**Debtmap Re-analysis**:
After completing all phases, run:
```bash
debtmap analyze --format json > after.json
```

Compare metrics:
- Total lines should be similar (~3100 distributed across modules)
- Functions per file should be <25
- God object score should drop from 1.0 to <0.5
- Complexity reduction should show ~68.8 point improvement
- Maintainability should improve ~18%

**Dependencies Between Phases**:
- Phase 1-3: Independent, can be done in any order
- Phase 4-5: Must be done after Phase 1-3 (use extracted modules)
- Phase 6: Must be done after Phase 4-5
- Phase 7: Must be done after all extraction phases
- Phase 8: Must be done last

**Estimated Time**:
- Each phase: 1-2 hours
- Total: 8-16 hours of focused work
- Allow extra time for debugging and testing

# Implementation Plan: Refactor God Object in src/analyzers/rust.rs

## Problem Summary

**Location**: ./src/analyzers/rust.rs:file:0
**Priority Score**: 58.25
**Debt Type**: God Object (GodClass)

**Current Metrics**:
- Lines of Code: 2034
- Functions: 124
- Cyclomatic Complexity: 186
- Coverage: 0%
- God Object: FunctionVisitor (11 fields, 33 methods, 7 responsibilities)

**Issue**: URGENT: File contains a God Class (FunctionVisitor) with 7 distinct responsibilities mixed together: Transformation, Data Access, Parsing & Input, Construction, Computation, Utilities, and Validation. The file has 124 functions handling everything from AST parsing to debt item creation to complexity calculation.

## Target State

**Expected Impact**:
- Complexity Reduction: 37.2
- Maintainability Improvement: 5.82
- Test Effort: 203.4

**Success Criteria**:
- [ ] FunctionVisitor split into focused, single-responsibility modules
- [ ] Each new module has <30 functions
- [ ] Clear separation between pure computation, I/O, and visitor logic
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with cargo fmt
- [ ] File size reduced to <500 lines per module

## Implementation Phases

### Phase 1: Extract Complexity Calculation Module

**Goal**: Separate pure complexity calculation functions into a dedicated module

**Changes**:
- Create `src/analyzers/rust/complexity_calculation.rs`
- Move 7 computation functions:
  - `calculate_cyclomatic_with_visitor`
  - `calculate_cognitive_with_visitor`
  - `calculate_cognitive_syn`
  - `apply_cognitive_pattern_scaling`
  - `calculate_nesting`
  - `count_lines`
  - `count_function_lines`
- These are pure functions with no side effects
- Add public interface and proper documentation

**Testing**:
- Run existing unit tests for these functions
- Verify complexity calculations are identical
- Check imports in main rust.rs file

**Success Criteria**:
- [ ] New module compiles successfully
- [ ] All tests pass: `cargo test --lib analyzers::rust`
- [ ] No clippy warnings: `cargo clippy`
- [ ] Functions are properly exported and used

### Phase 2: Extract Metadata and Construction Logic

**Goal**: Separate function metadata extraction and metrics construction into a builder module

**Changes**:
- Create `src/analyzers/rust/function_builder.rs`
- Move construction-related structs and functions:
  - `FunctionMetadata`
  - `ComplexityMetricsData`
  - `ClosureComplexityMetrics`
  - `FunctionContext`
  - `FunctionAnalysisData`
- Move builder functions:
  - `extract_function_metadata`
  - `create_function_context`
  - `create_function_analysis_data`
  - `build_function_metrics`
  - `build_closure_metrics`
  - `create_analysis_result`
- Implement builder pattern for FunctionMetrics construction

**Testing**:
- Verify FunctionMetrics objects are constructed correctly
- Test metadata extraction for various function types
- Ensure purity detection still works

**Success Criteria**:
- [ ] Builder module compiles and exports clean API
- [ ] All tests pass: `cargo test --lib analyzers::rust`
- [ ] FunctionVisitor uses builder instead of inline construction
- [ ] No behavioral changes

### Phase 3: Extract Utility and Classification Functions

**Goal**: Move pure utility and classification logic to dedicated module

**Changes**:
- Create `src/analyzers/rust/utilities.rs`
- Move utility functions:
  - `classify_test_file`
  - `is_test_function`
  - `has_test_attribute`
  - `has_test_name_pattern`
  - `extract_visibility`
  - `classify_function_role`
  - `classify_priority`
  - `detect_purity`
  - `calculate_entropy_if_enabled`
  - `calculate_closure_entropy`
  - `try_detect_visitor_pattern`
- All pure classification and detection logic

**Testing**:
- Run comprehensive test suite for classification functions
- Verify test detection still works correctly
- Check visibility extraction

**Success Criteria**:
- [ ] Utilities module with clear, single-purpose functions
- [ ] All existing tests pass
- [ ] Functions are well-documented with examples
- [ ] Module is independently testable

### Phase 4: Extract Debt Item Creation Module

**Goal**: Separate debt item creation and aggregation logic

**Changes**:
- Create `src/analyzers/rust/debt_creation.rs`
- Move debt-related functions:
  - `create_debt_items`
  - `collect_all_rust_debt_items`
  - `extract_debt_items_with_enhanced`
  - `create_debt_item_for_function`
  - `find_enhanced_analysis_for_function`
  - `create_enhanced_debt_item`
  - `create_complexity_debt_item`
  - `format_enhanced_context`
  - `extract_rust_module_smell_items`
  - `extract_rust_function_smell_items`
  - `report_rust_unclosed_blocks`
  - `analyze_rust_test_quality`
- Move pattern analysis functions:
  - `analyze_resource_patterns`
  - `analyze_organization_patterns`
  - `convert_organization_pattern_to_debt_item`
  - `pattern_to_message_context`
  - `impact_to_priority`

**Testing**:
- Verify debt items are created with correct priorities
- Test enhanced message formatting
- Ensure suppression context works

**Success Criteria**:
- [ ] Debt creation module is self-contained
- [ ] All debt detection tests pass
- [ ] Clear separation between detection and item creation
- [ ] No circular dependencies

### Phase 5: Refactor FunctionVisitor into Focused Visitor

**Goal**: Reduce FunctionVisitor to core visitor pattern responsibilities only

**Changes**:
- Keep FunctionVisitor focused on AST traversal
- Delegate to specialized modules:
  - Use `complexity_calculation` for metrics
  - Use `function_builder` for creating FunctionMetrics
  - Use `utilities` for classification
  - Use `debt_creation` only in coordinator functions
- Reduce FunctionVisitor to ~300 lines
- Move closure analysis to separate trait/module if needed
- Simplify visitor methods to delegate instead of implement

**Testing**:
- Run full test suite
- Verify visitor traversal is correct
- Check function detection works for all patterns

**Success Criteria**:
- [ ] FunctionVisitor has <300 lines
- [ ] Clear, focused responsibility (AST traversal only)
- [ ] All visitor methods are simple delegators
- [ ] All tests pass: `cargo test --lib`
- [ ] No clippy warnings

### Phase 6: Create Module Facade and Documentation

**Goal**: Create clean public API and comprehensive documentation

**Changes**:
- Create `src/analyzers/rust/mod.rs` as module root
- Re-export public APIs from submodules
- Keep only RustAnalyzer and high-level functions in main rust.rs
- Add module-level documentation
- Update import statements throughout codebase
- Final cleanup and organization

**Testing**:
- Run full test suite across entire codebase
- Verify public API is unchanged
- Test integration with other analyzers
- Run CI checks: `just ci`

**Success Criteria**:
- [ ] Clean module structure under src/analyzers/rust/
- [ ] All tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy --all-targets`
- [ ] Documentation builds: `cargo doc --no-deps`
- [ ] CI passes: `just ci`

## Testing Strategy

**For each phase**:
1. Run unit tests: `cargo test --lib analyzers::rust`
2. Check for clippy warnings: `cargo clippy --all-targets -- -D warnings`
3. Format code: `cargo fmt --all`
4. Verify compilation: `cargo check --all-features`

**Integration testing after Phase 6**:
1. Full test suite: `cargo test --all-features`
2. Coverage analysis: `cargo tarpaulin`
3. Run debtmap on itself: `debtmap analyze`
4. Verify improvements in metrics

**Final verification**:
1. CI checks: `just ci`
2. Documentation: `cargo doc --no-deps`
3. Security audit: `cargo deny check`

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the compilation errors or test failures
3. Identify the root cause (missing imports, circular deps, etc.)
4. Adjust the plan:
   - If it's a dependency issue, reorder the phases
   - If it's a module boundary issue, adjust what goes where
   - If tests fail, ensure test helpers are properly exported
5. Retry the phase with adjustments
6. Document lessons learned in commit message

## Notes

**Key Refactoring Principles**:
- Extract pure functions first (easiest, safest)
- Maintain existing behavior exactly
- Each module should be independently testable
- Use functional patterns (immutable data, pure functions)
- Clear data flow: parse → analyze → create metrics → create debt items

**Module Dependencies** (should flow downward):
```
rust.rs (coordinator)
  ├─> function_builder (constructs metrics)
  │     ├─> complexity_calculation (pure math)
  │     └─> utilities (classification)
  ├─> debt_creation (creates debt items)
  │     ├─> utilities (classification)
  │     └─> pattern analyzers (existing crate modules)
  └─> utilities (pure classification functions)
```

**Watch out for**:
- Circular dependencies between modules
- Test helpers that need to be shared
- Configuration access (entropy, thresholds)
- External dependencies (syn, quote, etc.)
- Trait implementations (Visit trait for FunctionVisitor)

**Performance Considerations**:
- These refactorings are pure code organization
- No algorithmic changes, so performance should be identical
- Module boundaries may enable future parallelization
- Smaller modules compile faster during development

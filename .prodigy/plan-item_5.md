# Implementation Plan: Reduce Complexity in CrossModuleContext::resolve_function

## Problem Summary

**Location**: ./src/analysis/python_call_graph/cross_module.rs:CrossModuleContext::resolve_function:197
**Priority Score**: 17.81
**Debt Type**: ComplexityHotspot (cyclomatic: 19, cognitive: 62)

**Current Metrics**:
- Function Length: 132 lines
- Cyclomatic Complexity: 19
- Cognitive Complexity: 62
- Nesting Depth: 6
- Function Role: PureLogic (marked as not pure due to side effects)

**Issue**: High complexity 19/62 makes function hard to test and maintain. The function attempts to resolve a function name through 5 different strategies:
1. Cache lookup
2. Enhanced resolver
3. Namespace-based resolution (with wildcard imports)
4. Tracker-based resolution (with multiple fallback strategies)
5. Direct symbol lookup

Each strategy has nested conditionals and side effects (cache writes), leading to high cognitive load.

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 9.5 (target cyclomatic: ~10)
- Coverage Improvement: 0.0
- Risk Reduction: 6.23

**Success Criteria**:
- [ ] Cyclomatic complexity reduced from 19 to ≤10
- [ ] Cognitive complexity reduced from 62 to <40
- [ ] Nesting depth reduced from 6 to ≤3
- [ ] Extract at least 4 pure helper functions
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with cargo fmt

## Implementation Phases

### Phase 1: Extract Cache Management Functions

**Goal**: Separate caching concerns from resolution logic to reduce nesting and clarify intent.

**Changes**:
- Extract `check_resolution_cache(&self, cache_key: &(PathBuf, String)) -> Option<FunctionId>` - Pure read from cache
- Extract `update_resolution_cache(&self, cache_key: (PathBuf, String), result: Option<FunctionId>)` - Write to cache
- Replace inline cache read/write blocks with these helper functions

**Testing**:
- Run `cargo test cross_module` to verify existing tests pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] Two new private helper functions created
- [ ] Cache read/write logic removed from main function
- [ ] Nesting depth reduced by 1 level
- [ ] All tests pass
- [ ] Ready to commit

**Expected Impact**: Reduces nesting from 6→5, improves readability

---

### Phase 2: Extract Enhanced Resolver Strategy

**Goal**: Extract the enhanced resolver resolution strategy into a dedicated function.

**Changes**:
- Extract `try_enhanced_resolver(&self, module_path: &Path, name: &str) -> Option<FunctionId>`
  - Move enhanced resolver logic (lines 208-232) into this function
  - Handle the resolver lock and symbol lookup internally
  - Return `Some(FunctionId)` on success, `None` on failure

**Testing**:
- Run `cargo test cross_module` to verify existing tests pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] New `try_enhanced_resolver` private method created
- [ ] Enhanced resolver block removed from main function
- [ ] Cyclomatic complexity reduced by ~2
- [ ] All tests pass
- [ ] Ready to commit

**Expected Impact**: Reduces cyclomatic complexity 19→17, improves separation of concerns

---

### Phase 3: Extract Namespace Resolution Strategy

**Goal**: Extract namespace-based resolution (including wildcard imports) into a focused function.

**Changes**:
- Extract `try_namespace_resolution(&self, module_path: &Path, name: &str) -> Option<FunctionId>`
  - Move namespace lookup logic (lines 235-259) into this function
  - Handle both direct namespace resolution and wildcard import iteration
  - Return `Some(FunctionId)` on success, `None` on failure

**Testing**:
- Run `cargo test cross_module` to verify existing tests pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] New `try_namespace_resolution` private method created
- [ ] Namespace resolution block removed from main function
- [ ] Cyclomatic complexity reduced by ~3
- [ ] All tests pass
- [ ] Ready to commit

**Expected Impact**: Reduces cyclomatic complexity 17→14, reduces nesting depth

---

### Phase 4: Extract Tracker Resolution Strategy

**Goal**: Extract the complex tracker-based resolution with multiple fallback strategies.

**Changes**:
- Extract `try_tracker_resolution(&self, module_path: &Path, name: &str) -> Option<FunctionId>`
  - Move tracker-based logic (lines 262-304) into this function
  - Includes: direct resolution, module.function split, export scanning
  - Return `Some(FunctionId)` on success, `None` on failure
- Consider extracting sub-helper: `try_export_scan(&self, resolved: &str) -> Option<FunctionId>`
  - Move the exports iteration loop (lines 286-302) into this sub-helper
  - Reduces complexity of the tracker resolution function

**Testing**:
- Run `cargo test cross_module` to verify existing tests pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] New `try_tracker_resolution` private method created
- [ ] Optional `try_export_scan` helper created if tracker function is still >15 lines
- [ ] Tracker resolution block removed from main function
- [ ] Cyclomatic complexity reduced by ~4
- [ ] All tests pass
- [ ] Ready to commit

**Expected Impact**: Reduces cyclomatic complexity 14→10, significantly improves readability

---

### Phase 5: Simplify Main Function with Strategy Chain

**Goal**: Refactor main `resolve_function` to be a clean strategy chain with caching.

**Changes**:
- Rewrite `resolve_function` as a pipeline:
  ```rust
  pub fn resolve_function(&self, module_path: &Path, name: &str) -> Option<FunctionId> {
      let cache_key = (module_path.to_path_buf(), name.to_string());

      // Check cache first
      if let Some(cached) = self.check_resolution_cache(&cache_key) {
          return cached;
      }

      // Try resolution strategies in order
      let result = self.try_enhanced_resolver(module_path, name)
          .or_else(|| self.try_namespace_resolution(module_path, name))
          .or_else(|| self.try_tracker_resolution(module_path, name))
          .or_else(|| self.try_direct_lookup(module_path, name));

      // Update cache
      self.update_resolution_cache(cache_key, result.clone());
      result
  }
  ```
- Extract `try_direct_lookup(&self, module_path: &Path, name: &str) -> Option<FunctionId>`
  - Move the final fallback logic (lines 307-321) into this function

**Testing**:
- Run `cargo test cross_module` to verify existing tests pass
- Run `cargo test --all` to ensure no regressions elsewhere
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] Main function reduced to ~15 lines
- [ ] Clear strategy chain using `.or_else()` pattern
- [ ] Cyclomatic complexity ≤10
- [ ] Cognitive complexity <40
- [ ] Nesting depth ≤2
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Ready to commit

**Expected Impact**: Achieves target complexity ≤10, dramatically improves readability and testability

---

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib cross_module` to verify cross-module tests pass
2. Run `cargo clippy -- -D warnings` to ensure no warnings
3. Run `cargo fmt --check` to verify formatting
4. Visual inspection: Confirm complexity reduction in changed function

**Final verification**:
1. `just ci` - Full CI checks (compile, test, clippy, fmt)
2. `cargo test --all` - All tests pass
3. `debtmap analyze` - Verify complexity metrics improved:
   - Cyclomatic complexity: 19 → ≤10
   - Cognitive complexity: 62 → <40
   - Function should rank lower in next debtmap run

**Coverage Verification**:
- Run `cargo tarpaulin --lib` to ensure coverage is maintained or improved
- Focus on `cross_module.rs` module coverage

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the failure:
   - Test failures: Check if new function has different behavior
   - Compilation errors: Check function signatures and lifetimes
   - Clippy warnings: Adjust to follow Rust idioms
3. Adjust the implementation approach
4. Retry the phase with corrections

## Notes

**Key Insights**:
- The function is marked as "PureLogic" by debtmap but has side effects (cache writes)
- Each resolution strategy is independent and can be extracted cleanly
- The `.or_else()` chain pattern is idiomatic Rust for fallback strategies
- Cache management should be separate from business logic

**Potential Challenges**:
- RwLock handling in extracted functions - may need to take locks in helpers
- Ensuring cache updates happen correctly after extraction
- Maintaining exact same resolution behavior (order matters)

**Functional Programming Principles Applied**:
- Extract pure functions where possible (cache reads, symbol lookups)
- Separate side effects (cache writes) from computation
- Use `.or_else()` for functional composition of strategies
- Each strategy function has single responsibility

**Performance Considerations**:
- Cache check remains first for performance
- Lock acquisition order preserved to avoid deadlocks
- Each strategy short-circuits on success (via `or_else`)

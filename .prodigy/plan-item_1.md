# Implementation Plan: Refactor GodObjectDetector God Object

## Problem Summary

**Location**: ./src/organization/god_object_detector.rs:file:0
**Priority Score**: 108.04
**Debt Type**: God Object (GodClass)

**Current Metrics**:
- Lines of Code: 2,882
- Functions: 113 total (34 methods in GodObjectDetector, 79 tests)
- Cyclomatic Complexity: 250 total (max: 24, avg: 2.21)
- Coverage: 0% (test file, not covered by integration tests)
- Struct Fields: 5
- Responsibilities: 7 distinct domains

**Identified Responsibilities**:
1. Parsing & Input - AST visiting, type extraction
2. Filtering & Selection - Finding god objects, classifying
3. Construction - Building metrics, TypeVisitor
4. Utilities - Helper functions (prefixes, naming)
5. Data Access - Getting thresholds, extracting data
6. Computation - Calculating scores, weights, complexity
7. Validation - Checking conditions, determining impact

**Issue**: The GodObjectDetector has grown into a god object itself, with 34 methods handling multiple distinct phases of analysis: data collection, pattern detection, scoring/metrics calculation, and recommendation generation. This violates the single responsibility principle and makes the code harder to test and maintain.

## Target State

**Expected Impact**:
- Complexity Reduction: 50.0 points
- Maintainability Improvement: 10.8 points
- Test Effort Reduction: 288.2 points

**Success Criteria**:
- [ ] Split into 4 focused modules aligned with analysis phases
- [ ] Each module has a single, clear responsibility
- [ ] Extract pure functions for scoring and calculations
- [ ] Reduce average function length to <15 lines
- [ ] Reduce maximum cyclomatic complexity to <10
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting with rustfmt

**Target Architecture**:
```
src/organization/
├── god_object_detector.rs          # Main orchestrator (~200 lines)
├── god_object/
│   ├── mod.rs                      # Module exports
│   ├── ast_visitor.rs              # Phase 1: Data collection
│   ├── classifier.rs               # Phase 2: Pattern detection
│   ├── metrics.rs                  # Phase 3: Scoring/metrics
│   └── recommender.rs              # Phase 4: Recommendations
```

## Implementation Phases

### Phase 1: Extract AST Visitor (Data Collection)

**Goal**: Separate AST traversal and data collection into a dedicated module

**Rationale**: The TypeVisitor and related data collection logic (lines 1418-1615, ~200 lines) represent a distinct responsibility. This is pure data collection that should be isolated from analysis logic.

**Changes**:
- Create `src/organization/god_object/mod.rs` and `ast_visitor.rs`
- Move `TypeVisitor` struct and its implementations
- Move helper types: `TypeAnalysis`, `Responsibility`, `FunctionWeight`
- Extract functions: `extract_type_name`, `count_impl_methods`, `extract_impl_complexity`, `update_type_info`
- Make `TypeVisitor` public with a clean API
- Keep all visitor trait implementations together

**Testing**:
- Run `cargo test god_object_detector` to verify visitor tests pass
- Run `cargo test --lib` to ensure no breakage
- Check that `analyze_enhanced` and `analyze_comprehensive` still work

**Success Criteria**:
- [ ] TypeVisitor moved to `ast_visitor.rs` with clean API
- [ ] All visitor-related tests pass (tests at lines 2061-2094)
- [ ] No compilation errors or warnings
- [ ] File reduced by ~200 lines
- [ ] Ready to commit

### Phase 2: Extract Metrics Calculation (Scoring)

**Goal**: Extract pure scoring and metrics calculation functions into a dedicated module

**Rationale**: Lines 555-696 contain complex scoring logic that should be pure functions. These calculations have no side effects and can be easily tested in isolation. This aligns with functional programming principles.

**Changes**:
- Create `src/organization/god_object/metrics.rs`
- Move pure calculation functions:
  - `calculate_weighted_metrics` (lines 555-633)
  - `calculate_final_god_object_score` (lines 635-695)
  - `calculate_purity_weights` (lines 1083-1142)
  - `calculate_visibility_breakdown` (lines 980-1059)
  - `integrate_visibility_into_counts` (lines 1061-1081)
- Move `build_per_struct_metrics` (lines 98-120)
- Keep these as pure functions with clear inputs/outputs
- Add comprehensive documentation

**Testing**:
- Run `cargo test metrics` to verify calculation tests
- Run unit tests for scoring functions (lines 2510-2526)
- Verify visibility tests pass (lines 2544-2884)
- Check integration with `analyze_comprehensive`

**Success Criteria**:
- [ ] All scoring functions extracted to `metrics.rs`
- [ ] Functions are pure (no side effects)
- [ ] All calculation tests pass
- [ ] File reduced by ~300 lines
- [ ] Ready to commit

### Phase 3: Extract Pattern Classification

**Goal**: Extract god object classification and pattern detection logic

**Rationale**: Lines 122-462 contain complex classification logic that determines god object types. This is a distinct analysis phase that should be separated from scoring and data collection.

**Changes**:
- Create `src/organization/god_object/classifier.rs`
- Move classification functions:
  - `classify_god_object` (lines 122-261) - **reduce complexity from 24 to <10**
  - `determine_god_object_type` (lines 491-553)
  - `analyze_domains_and_recommend_splits` (lines 697-796)
  - `analyze_module_structure_and_visibility` (lines 798-852)
- Extract helper functions:
  - `get_thresholds_for_path` (lines 463-489)
  - `is_god_object` (lines 1196-1201)
  - `classify_god_object_impact` (lines 1185-1194)
- Break down `classify_god_object` into smaller pure functions:
  - `check_boilerplate_pattern`
  - `check_registry_pattern`
  - `check_builder_pattern`
  - `classify_by_metrics`
  - Each should be <15 lines

**Testing**:
- Run `cargo test classify` to verify classification logic
- Run enhanced analysis tests (lines 2238-2477)
- Verify all pattern detection scenarios work
- Test domain classification (lines 2510-2542)

**Success Criteria**:
- [ ] Classification logic extracted to `classifier.rs`
- [ ] `classify_god_object` broken into <10 complexity functions
- [ ] All classification tests pass
- [ ] File reduced by ~340 lines
- [ ] Ready to commit

### Phase 4: Extract Recommendation Generation

**Goal**: Separate recommendation and reporting logic from core analysis

**Rationale**: Lines 263-461 contain recommendation generation logic. This is a distinct output formatting phase that should be isolated. It transforms analysis results into actionable recommendations.

**Changes**:
- Create `src/organization/god_object/recommender.rs`
- Move recommendation functions:
  - `generate_recommendation` (lines 263-461)
  - `suggest_responsibility_split` (lines 1203-1218)
  - `create_responsibility_group` (lines 1220-1233)
  - `create_default_responsibility_group` (lines 1235-1242)
- Move utility functions used for recommendations:
  - `group_methods_by_prefix` (lines 1244-1253)
  - `extract_method_prefix` (lines 1255-1259)
  - `find_matching_prefix` (lines 1261-1308)
  - `extract_first_word` (lines 1310-1316)
  - `infer_responsibility_name` (lines 1318-1321)
  - `classify_responsibility` (lines 1323-1338)
- Organize into logical groups with clear separation

**Testing**:
- Run `cargo test recommend` to verify recommendation logic
- Run responsibility split tests (lines 2142-2236)
- Run prefix matching tests (lines 1623-1658)
- Run responsibility classification tests (lines 1676-1903)

**Success Criteria**:
- [ ] Recommendation logic extracted to `recommender.rs`
- [ ] All utility functions organized logically
- [ ] All recommendation tests pass
- [ ] File reduced by ~200 lines
- [ ] Ready to commit

### Phase 5: Clean Up Main Detector

**Goal**: Reduce main detector to a clean orchestrator

**Rationale**: After extracting specialized modules, the main detector should be a thin orchestrator that coordinates the analysis phases. It should be <200 lines and have minimal logic.

**Changes**:
- Keep only orchestration methods in `god_object_detector.rs`:
  - `new`, `with_source_content`, `default`
  - `analyze_enhanced` (refactor to use extracted modules)
  - `analyze_comprehensive` (refactor to use extracted modules)
  - `detect_anti_patterns` (OrganizationDetector trait)
  - `detector_name`, `estimate_maintainability_impact`
- Update `analyze_enhanced` to use new modules:
  ```rust
  pub fn analyze_enhanced(&self, path: &Path, ast: &syn::File) -> EnhancedGodObjectAnalysis {
      let visitor = ast_visitor::TypeVisitor::collect(ast, self.location_extractor.clone());
      let per_struct_metrics = metrics::build_per_struct_metrics(&visitor);
      let file_metrics = self.analyze_comprehensive(path, ast);
      let thresholds = classifier::get_thresholds_for_path(path);
      let ownership = /* ... */;

      let classification = classifier::classify_god_object(
          &per_struct_metrics,
          file_metrics.method_count,
          &thresholds,
          ownership.as_ref(),
          path,
          ast,
          self.source_content.as_deref(),
      );

      let recommendation = recommender::generate_recommendation(
          &classification,
          path,
          &per_struct_metrics,
      );

      EnhancedGodObjectAnalysis { /* ... */ }
  }
  ```
- Add clear module documentation
- Ensure clean imports and exports

**Testing**:
- Run full test suite: `cargo test god_object_detector`
- Run integration tests to verify end-to-end functionality
- Verify all 79 tests still pass
- Run `cargo clippy` to check for warnings

**Success Criteria**:
- [ ] Main detector reduced to <200 lines
- [ ] Clear orchestration with minimal logic
- [ ] All 113 tests pass
- [ ] No clippy warnings
- [ ] Clean module structure
- [ ] Ready to commit

## Testing Strategy

**For each phase**:
1. Run targeted tests: `cargo test <module_name>`
2. Run full lib tests: `cargo test --lib`
3. Check for warnings: `cargo clippy --all-targets -- -D warnings`
4. Format code: `cargo fmt --all`
5. Verify no test count regression

**Final verification**:
1. Run complete CI: `just ci`
2. Verify all 113 tests pass
3. Check complexity with `debtmap analyze`
4. Verify metrics improved:
   - Total complexity reduced by ~50 points
   - Max complexity reduced from 24 to <10
   - Average file size reduced from 2,882 to ~400 lines max

**Coverage considerations**:
- This is test code (0% coverage is expected)
- Focus on ensuring the test suite itself passes
- Integration tests in other files cover this detector

## Rollback Plan

If a phase fails:
1. Check git status: `git status`
2. Review the error carefully
3. If needed, revert: `git reset --hard HEAD~1`
4. Re-read the relevant code sections
5. Identify what was missed
6. Update understanding and retry with adjustments

**Common failure points**:
- Import paths after moving modules
- Test module paths need updating
- Circular dependencies between new modules
- Missing pub visibility on moved items

**Mitigation**:
- Compile after each significant move
- Update imports incrementally
- Keep `mod.rs` exports clean
- Run tests frequently

## Notes

**Key Insights**:
- The file is actually well-tested (79 tests) but counts as 0% coverage because it's test code
- Most functions are already relatively simple (avg complexity 2.21)
- The main issue is organizational - too many responsibilities in one file
- The max complexity of 24 is in `classify_god_object` - this MUST be refactored

**Important Patterns to Preserve**:
- The `TypeVisitor` pattern for AST traversal
- The `OrganizationDetector` trait implementation
- Test organization and naming conventions
- Pure function style where already present

**Functional Programming Goals**:
- Extract pure calculation functions (already mostly pure)
- Separate I/O (AST reading) from computation (scoring)
- Use function composition in orchestration
- Minimize mutable state

**Dependencies to Watch**:
- `OrganizationDetector` trait must remain implemented
- `EnhancedGodObjectAnalysis` and `GodObjectAnalysis` return types
- Integration with `struct_ownership`, `boilerplate_detector`, `RegistryPatternDetector`, `BuilderPatternDetector`
- Location extractor integration

**Success Indicators**:
- Main file <200 lines
- Each extracted module <400 lines
- Max complexity <10
- All tests passing
- Clear module boundaries
- Easier to understand and modify

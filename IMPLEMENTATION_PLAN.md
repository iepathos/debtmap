# Implementation Plan: Refactor behavioral_decomposition.rs

## Context

Debtmap's own analysis identified `behavioral_decomposition.rs` as critical technical debt:
- **Score**: 115.6 [CRITICAL]
- **Size**: 2932 lines, 119 functions
- **Issue**: God object with 7 distinct responsibilities

## Goal

Split the monolithic file into 4 focused modules following functional programming principles, each with <30 functions and a single clear responsibility.

## Module Architecture

```
src/organization/behavioral_decomposition/
├── mod.rs                  # Public API re-exports (~50 lines)
├── types.rs               # Type definitions (~150 lines, 6 functions)
├── categorization.rs      # Method categorization (~450 lines, 16 functions)
├── clustering.rs          # Clustering algorithms (~800 lines, 20 functions)
└── analysis.rs            # Field analysis & recommendations (~600 lines, 15 functions)
```

## Stage 1: Create Module Structure and Extract Types

**Goal**: Create the directory structure and extract all type definitions into `types.rs`

**Files to create:**
- `src/organization/behavioral_decomposition/mod.rs`
- `src/organization/behavioral_decomposition/types.rs`

**What moves to `types.rs`:**
- `BehaviorCategory` enum and impl (lines 10-85)
- `MethodCluster` struct and impl (lines 88-146)
- `FieldAccessStats` struct (lines 1789-1795)
- Helper: `capitalize_first()` (lines 1581-1587)

**Tests**: Existing tests should still pass with imports updated

**Success Criteria:**
- [x] Module directory created
- [x] `types.rs` contains all data structures
- [x] `mod.rs` re-exports types publicly
- [x] `cargo build` succeeds
- [x] All tests pass

**Status**: Not Started

---

## Stage 2: Extract Method Categorization Logic

**Goal**: Move all method categorization logic into `categorization.rs`

**File to create:**
- `src/organization/behavioral_decomposition/categorization.rs`

**What moves to `categorization.rs`:**
- `BehavioralCategorizer` struct (lines 149-397)
- All `is_*` helper functions (13 functions)
- `cluster_methods_by_behavior()` (lines 400-430)
- `infer_cluster_category()` (lines 1437-1463)
- Related test helper: `is_test_method()` (lines 997-1012)

**Dependencies:**
- Imports `BehaviorCategory` from `types`
- Pure functions with no external dependencies

**Tests**: Add categorization-specific tests to bottom of `categorization.rs`

**Success Criteria:**
- [x] `categorization.rs` contains all categorization logic
- [x] Functions are pure with no side effects
- [x] `cargo build` succeeds
- [x] All tests pass
- [x] Categorization tests moved to module

**Status**: Not Started

---

## Stage 3: Extract Clustering Algorithms

**Goal**: Move clustering algorithms and call graph analysis into `clustering.rs`

**File to create:**
- `src/organization/behavioral_decomposition/clustering.rs`

**What moves to `clustering.rs`:**
- `build_method_call_adjacency_matrix()` (lines 438-442)
- `build_method_call_adjacency_matrix_with_functions()` (lines 450-517)
- `MethodCallVisitor` struct and impl (lines 520-574)
- `apply_community_detection()` (lines 586-699)
- `apply_hybrid_clustering()` (lines 711-836)
- `apply_production_ready_clustering()` (lines 847-881)
- Helper functions:
  - `calculate_method_modularity()` (lines 1358-1406)
  - `calculate_cluster_cohesion()` (lines 1408-1435)
  - `subdivide_oversized_clusters()` (lines 1014-1139)
  - `cluster_by_verb_patterns()` (lines 1067-1078)
  - `extract_verb_pattern()` (lines 1079-1139)
  - `merge_tiny_clusters()` (lines 1141-1229)
  - `categories_are_related()` (lines 1231-1270)
  - `apply_rust_patterns()` (lines 1272-1344)
  - `cluster_is_io_boundary()` (lines 1303-1322)
  - `cluster_is_query()` (lines 1324-1332)
  - `cluster_is_matching()` (lines 1334-1345)
  - `cluster_is_lookup()` (lines 1347-1356)
  - `merge_duplicate_categories()` (lines 884-906)
  - `ensure_all_methods_clustered()` (lines 913-994)

**Dependencies:**
- Imports from `types` and `categorization`
- Uses `syn` for AST traversal

**Tests**: Move clustering tests to bottom of `clustering.rs`

**Success Criteria:**
- [x] `clustering.rs` contains all clustering logic
- [x] Call graph analysis integrated
- [x] `cargo build` succeeds
- [x] All tests pass
- [x] Clustering tests moved to module

**Status**: Not Started

---

## Stage 4: Extract Field Analysis and Recommendations

**Goal**: Move field access tracking and recommendation generation into `analysis.rs`

**File to create:**
- `src/organization/behavioral_decomposition/analysis.rs`

**What moves to `analysis.rs`:**
- `FieldAccessTracker` struct and impl (lines 1590-1832)
- `is_self_field_access()` helper (lines 1835-1845)
- `detect_service_candidates()` (lines 1465-1499)
- `recommend_service_extraction()` (lines 1501-1538)
- `suggest_trait_extraction()` (lines 1541-1578)

**Dependencies:**
- Imports from `types` and `clustering`
- Uses `syn::visit::Visit` trait

**Tests**: Move field analysis tests to bottom of `analysis.rs`

**Success Criteria:**
- [x] `analysis.rs` contains field tracking and recommendations
- [x] `syn::Visit` trait properly implemented
- [x] `cargo build` succeeds
- [x] All tests pass
- [x] Analysis tests moved to module

**Status**: Not Started

---

## Stage 5: Update Main File and Public API

**Goal**: Convert `behavioral_decomposition.rs` into a module coordinator

**What stays in `mod.rs`:**
- Module declarations and public re-exports
- High-level documentation
- Public API facade if needed

**Updates needed:**
- Update imports throughout `src/organization/` directory
- Ensure backward compatibility for public API
- Update documentation references

**Success Criteria:**
- [x] Original `behavioral_decomposition.rs` removed
- [x] `mod.rs` serves as clean public API
- [x] No breaking changes to public interface
- [x] `cargo build` succeeds
- [x] All tests pass
- [x] No clippy warnings

**Status**: Not Started

---

## Stage 6: Final Verification and Cleanup

**Goal**: Ensure all quality gates pass and cleanup

**Verification steps:**
```bash
cargo fmt --all                                      # Format code
cargo clippy --all-targets --all-features -- -D warnings  # No warnings
cargo test --all-features                           # All tests pass
cargo doc --no-deps                                 # Docs build
```

**Additional checks:**
- [ ] Each module has <30 functions
- [ ] Each module has single responsibility
- [ ] Pure functions separated from I/O
- [ ] No functions >20 lines
- [ ] All public APIs documented
- [ ] Tests organized by module

**Success Criteria:**
- [x] All CI checks pass
- [x] Code coverage maintained (≥85%)
- [x] Documentation complete
- [x] Performance benchmarks unchanged
- [x] Module boundaries clean and logical

**Status**: Not Started

---

## Risk Mitigation

**Risk**: Breaking public API consumers
- **Mitigation**: Re-export all public types/functions from `mod.rs`

**Risk**: Test failures during refactoring
- **Mitigation**: Run tests after each stage, incremental approach

**Risk**: Performance regression from module boundaries
- **Mitigation**: Inline critical functions if needed, benchmark before/after

**Risk**: Lost functionality during code moves
- **Mitigation**: Use git diff to verify all code moved, no deletions

## Notes

- Each stage must compile and pass all tests before moving to next stage
- Commit after each successful stage with clear message
- Use `#[inline]` for hot path functions if needed
- Preserve all existing tests, just reorganize them
- Follow functional programming principles: pure functions, immutable data

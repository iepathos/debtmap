---
number: 195
title: Refactor unified.rs God Module
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-14
---

# Specification 195: Refactor unified.rs God Module

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

`src/output/unified.rs` has been identified by debtmap's self-analysis as a critical god module with:

- **74 functions** across **2189 lines**
- **10 mixed responsibilities** detected
- **Accumulated cyclomatic complexity**: 171
- **Accumulated cognitive complexity**: 221 (dampened to 110)
- **Max nesting depth**: 8
- **Test coverage**: 41.7%
- **Total score**: 100.0 (critical)

**Debtmap's Recommendation:**
> "Separate 10 I/O handlers from business logic - Mixed I/O concerns: 10 responsibilities detected - separating I/O from pure logic enables better testing and reduces coupling"

This module implements the unified output format (spec 108) for technical debt reporting. It has grown organically to include:

1. **Numeric formatting** - `round_score()`, `round_ratio()`
2. **Invariant assertions** - `assert_score_invariants()`, etc.
3. **Core output types** - `UnifiedOutput`, `OutputMetadata`, `DebtSummary`
4. **Priority classification** - `Priority` enum
5. **Location types** - `UnifiedLocation`
6. **File debt types + conversion** - `FileDebtItemOutput`, `From` impls
7. **Function debt types + conversion** - `FunctionDebtItemOutput`, `From` impls
8. **Cohesion types** - `CohesionOutput`, `CohesionClassification`, `CohesionSummary`
9. **Coupling/dependency types** - `FileDependencies`, `CouplingClassification`
10. **Anti-pattern output** - `AntiPatternOutput`, `AntiPatternItem`
11. **Pattern extraction** - `extract_complexity_pattern()`, `extract_pattern_data()`
12. **Deduplication** - `DebtItemKey`, `deduplicate_items()`
13. **Main conversion** - `convert_to_unified_format()`
14. **Tests** - 800+ lines of tests

Following the Stillwater philosophy of **"Pure Core, Imperative Shell"**, this module should be refactored to separate pure data types and functions from conversion logic.

## Objective

Refactor `src/output/unified.rs` into a focused submodule structure following the patterns used in `src/priority/` and `src/organization/`:

1. **Reduce file size** from 2189 lines to ~150 lines per file (max 250)
2. **Separate responsibilities** into focused files with single concerns
3. **Maintain public API** via `pub use` re-exports for backward compatibility
4. **Improve testability** by isolating pure functions
5. **Follow project conventions** for module organization

**Result**: Clean, maintainable module structure where each file has a single responsibility and can be understood in isolation.

## Requirements

### Functional Requirements

1. **Module Structure**
   - Create `src/output/unified/` directory as submodule
   - Split 2189 lines into 13 focused files (~100-200 lines each)
   - Re-export all public types from `unified/mod.rs`
   - Preserve exact public API (no breaking changes)

2. **File Organization by Responsibility**

   | File | Responsibility | Estimated Lines |
   |------|----------------|-----------------|
   | `mod.rs` | Orchestration + re-exports + `convert_to_unified_format()` | ~150 |
   | `types.rs` | Core: `UnifiedOutput`, `OutputMetadata`, `DebtSummary` | ~150 |
   | `file_item.rs` | `FileDebtItemOutput`, `FileMetricsOutput`, conversions | ~200 |
   | `func_item.rs` | `FunctionDebtItemOutput`, `FunctionMetricsOutput`, conversions | ~200 |
   | `coupling.rs` | `FileDependencies`, `CouplingClassification`, `classify_coupling()` | ~100 |
   | `cohesion.rs` | `CohesionOutput`, `CohesionClassification`, `CohesionSummary` | ~80 |
   | `anti_patterns.rs` | `AntiPatternOutput`, `AntiPatternItem`, `AntiPatternSummary` | ~80 |
   | `format.rs` | `round_score()`, `round_ratio()`, invariant assertions | ~60 |
   | `priority.rs` | `Priority` enum + `from_score()` | ~30 |
   | `location.rs` | `UnifiedLocation` struct | ~20 |
   | `dedup.rs` | `DebtItemKey`, `deduplicate_items()` | ~80 |
   | `patterns.rs` | `extract_complexity_pattern()`, `extract_pattern_data()` | ~80 |
   | `dependencies.rs` | `Dependencies`, `PurityAnalysis`, `RecommendationOutput` | ~60 |

3. **Pure Function Separation**
   - Keep pure formatting functions in `format.rs`
   - Keep pure classification functions in respective modules
   - Move I/O-adjacent conversion logic to item files
   - Main orchestration stays in `mod.rs`

4. **Test Organization**
   - Each file gets `#[cfg(test)] mod tests { }` section
   - Property tests stay with their implementations
   - Integration tests for `convert_to_unified_format` in `mod.rs`

### Non-Functional Requirements

1. **Backward Compatibility**
   - All external imports continue to work unchanged
   - `use crate::output::UnifiedOutput` still valid
   - `use crate::output::convert_to_unified_format` still valid

2. **Code Quality**
   - Each file under 250 lines
   - Single responsibility per file
   - Clear, descriptive file names
   - Consistent patterns with `src/priority/` organization

3. **Incremental Implementation**
   - Each extraction step verified with `cargo test`
   - No logic changes during extraction
   - Pure refactoring (behavior preserved)

## Acceptance Criteria

- [ ] All existing tests pass (`cargo test`)
- [ ] No changes to public API signatures
- [ ] Each file in `unified/` under 250 lines
- [ ] Clear single responsibility per file
- [ ] External callers unchanged:
  - [ ] `src/output/json.rs` - uses `convert_to_unified_format`, `UnifiedOutput`
  - [ ] `src/io/writers/html.rs` - uses `convert_to_unified_format`
  - [ ] `src/io/writers/dot.rs` - uses `CouplingClassification`, `FileDebtItemOutput`
  - [ ] `src/priority/formatter/mod.rs` - uses `classify_coupling`
  - [ ] `src/tui/results/detail_pages/overview.rs` - uses `CohesionClassification`
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` produces no changes
- [ ] Old `unified.rs` file deleted
- [ ] Debtmap self-analysis shows improved metrics for output module

## Technical Details

### Implementation Approach

**Phase 1: Create Module Structure**

```
src/output/
├── mod.rs                    # Update to re-export from unified/
├── unified/                  # NEW submodule
│   ├── mod.rs               # Re-exports + convert_to_unified_format()
│   ├── types.rs             # Core output structures
│   ├── priority.rs          # Priority enum
│   ├── location.rs          # UnifiedLocation
│   ├── file_item.rs         # File debt output
│   ├── func_item.rs         # Function debt output
│   ├── cohesion.rs          # Cohesion classification
│   ├── coupling.rs          # Coupling classification
│   ├── anti_patterns.rs     # Anti-pattern output
│   ├── format.rs            # Numeric utilities
│   ├── dedup.rs             # Deduplication logic
│   ├── patterns.rs          # Pattern extraction
│   └── dependencies.rs      # Shared dependency types
└── unified.rs               # DELETE after migration
```

**Phase 2: Extract in Dependency Order**

Extract files in order of dependencies (leaf modules first):

1. `format.rs` - No internal dependencies (pure utilities)
2. `priority.rs` - Depends only on format.rs
3. `location.rs` - No dependencies (pure type)
4. `coupling.rs` - Depends on format.rs
5. `cohesion.rs` - Depends on format.rs
6. `anti_patterns.rs` - No internal dependencies
7. `dependencies.rs` - Shared types (no deps)
8. `patterns.rs` - Pure extraction functions
9. `file_item.rs` - Depends on above modules
10. `func_item.rs` - Depends on above modules
11. `dedup.rs` - Depends on file_item, func_item
12. `types.rs` - Core types, depends on all above
13. `mod.rs` - Wire up re-exports and main function

**Phase 3: Update Parent Module**

```rust
// src/output/mod.rs - After refactoring

pub mod unified;  // Now a directory module

pub use unified::*;  // Preserves all existing exports
```

### Architecture Changes

**Before:**
```
src/output/
├── mod.rs           # Declares modules
├── unified.rs       # 2189 lines, 74 functions, 10 responsibilities
├── json.rs
├── markdown.rs
└── ...
```

**After:**
```
src/output/
├── mod.rs           # Declares modules, re-exports unified/*
├── unified/         # Focused submodule
│   ├── mod.rs       # ~150 lines (orchestration)
│   ├── types.rs     # ~150 lines (core types)
│   ├── file_item.rs # ~200 lines (file debt)
│   └── ...          # 10 more focused files
├── json.rs
├── markdown.rs
└── ...
```

### Data Structures

No changes to data structures. All types remain identical, just organized into separate files.

### APIs and Interfaces

**Public API (unchanged):**

```rust
// All of these continue to work after refactoring:
use crate::output::UnifiedOutput;
use crate::output::UnifiedDebtItemOutput;
use crate::output::FileDebtItemOutput;
use crate::output::FunctionDebtItemOutput;
use crate::output::Priority;
use crate::output::CohesionClassification;
use crate::output::CouplingClassification;
use crate::output::FileDependencies;
use crate::output::convert_to_unified_format;
use crate::output::classify_coupling;
use crate::output::calculate_instability;
// ... and all other existing exports
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/output/unified.rs` → `src/output/unified/` (refactored)
  - `src/output/mod.rs` (updated re-exports)
- **External Dependencies**: None
- **Consumers (unchanged)**:
  - `src/output/json.rs`
  - `src/io/writers/html.rs`
  - `src/io/writers/dot.rs`
  - `src/priority/formatter/mod.rs`
  - `src/tui/results/detail_pages/overview.rs`
  - `tests/output_validation_test.rs`

## Testing Strategy

### Unit Tests

Each extracted file includes its own `#[cfg(test)]` module with tests moved from the original file.

```rust
// Example: unified/priority.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_from_score() {
        assert!(matches!(Priority::from_score(150.0), Priority::Critical));
        assert!(matches!(Priority::from_score(75.0), Priority::High));
        assert!(matches!(Priority::from_score(35.0), Priority::Medium));
        assert!(matches!(Priority::from_score(10.0), Priority::Low));
    }
}
```

### Integration Tests

Main conversion function tested in `unified/mod.rs`:

```rust
// unified/mod.rs
#[cfg(test)]
mod tests {
    // Integration tests for convert_to_unified_format
    // Moved from original unified.rs
}
```

### Property Tests

Proptest-based tests stay with their implementations in `file_item.rs` and `func_item.rs`:

```rust
// unified/func_item.rs
#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_round_score_never_negative(score in 0.0f64..1000.0) {
            let rounded = round_score(score);
            prop_assert!(rounded >= 0.0);
        }
    }
}
```

### Verification Commands

```bash
# After each extraction step:
cargo test output::unified
cargo clippy -- -D warnings
cargo fmt --check

# Final verification:
cargo test
target/debug/debtmap analyze src/output/
```

## Documentation Requirements

### Code Documentation

Each new file includes module-level documentation:

```rust
//! Priority classification for debt items (spec 108)
//!
//! Provides the `Priority` enum that classifies debt items based on their
//! score thresholds:
//! - Critical: score >= 100
//! - High: score >= 50
//! - Medium: score >= 20
//! - Low: score < 20
```

### User Documentation

No user-facing documentation changes needed (internal refactoring).

### Architecture Updates

Update `ARCHITECTURE.md` to document the new output module organization if present.

## Implementation Notes

### Extraction Order Rationale

Files are extracted in dependency order to ensure compilation after each step:

1. **Leaf modules first**: `format.rs`, `priority.rs`, `location.rs` have no internal dependencies
2. **Classification modules**: `coupling.rs`, `cohesion.rs` depend on `format.rs`
3. **Type modules**: `anti_patterns.rs`, `dependencies.rs` are self-contained
4. **Item modules**: `file_item.rs`, `func_item.rs` depend on many above
5. **Core modules last**: `types.rs`, `dedup.rs`, `mod.rs` orchestrate everything

### Re-export Pattern

Following project conventions (see `src/priority/mod.rs`):

```rust
// unified/mod.rs
mod anti_patterns;
mod cohesion;
mod coupling;
mod dedup;
mod dependencies;
mod file_item;
mod format;
mod func_item;
mod location;
mod patterns;
mod priority;
mod types;

// Re-export all public items
pub use anti_patterns::*;
pub use cohesion::*;
pub use coupling::*;
pub use dedup::deduplicate_items;  // Only public fn
pub use dependencies::*;
pub use file_item::*;
pub use format::{round_ratio, round_score};  // Public utilities
pub use func_item::*;
pub use location::*;
pub use patterns::*;
pub use priority::*;
pub use types::*;

// Main conversion function stays here
pub fn convert_to_unified_format(...) -> UnifiedOutput {
    // Orchestration logic
}
```

### Test Migration

Tests are split alongside their implementations:

| Original Location | New Location |
|-------------------|--------------|
| `test_priority_from_score` | `unified/priority.rs` |
| `test_unified_location_*` | `unified/location.rs` |
| `test_calculate_instability_*` | `unified/coupling.rs` |
| `test_classify_coupling_*` | `unified/coupling.rs` |
| `test_round_*` | `unified/format.rs` |
| `test_deduplication_*` | `unified/dedup.rs` |
| `test_*_serialization_roundtrip` | Respective item files |
| `proptest_tests` | `unified/func_item.rs` (main location) |

## Migration and Compatibility

### Breaking Changes

**None** - This is a pure refactoring. All public APIs remain unchanged.

### Migration Steps

No migration needed. This is an internal reorganization that maintains the exact same public interface.

### Verification

After refactoring, verify all consumers still compile:

```bash
cargo check -p debtmap
cargo test -p debtmap output::
cargo test -p debtmap tui::
cargo test -p debtmap io::writers::
```

## Success Metrics

- **File count**: 1 file (2189 lines) → 13 files (~100-200 lines each)
- **Max file size**: 2189 lines → <250 lines per file
- **Responsibility count**: 10+ mixed → 1 per file
- **Test organization**: All tests co-located with implementations
- **External API**: 100% backward compatible
- **Debtmap score for module**: Should drop from 100 (critical) to <50

## Follow-up Work

After this refactoring:

1. **Improve test coverage** for individual files (currently 41.7%)
2. **Add property tests** for more pure functions
3. **Apply same patterns** to other large files identified by debtmap
4. **Document module organization patterns** for contributors

## References

- **Spec 108**: Unified output format definition
- **Spec 230**: Output invariants (scores, ratios, priority thresholds)
- **Spec 231**: Deduplication logic
- **Spec 232**: Dampened cyclomatic calculation
- **Stillwater Philosophy**: Pure Core, Imperative Shell pattern
- **CLAUDE.md**: Module organization patterns in `src/priority/` and `src/organization/`

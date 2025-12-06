---
number: 241
title: Type-Safe Score Scales with Newtype Pattern
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-01-06
---

# Specification 241: Type-Safe Score Scales with Newtype Pattern

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The god object TUI bug revealed that score scales are inconsistent across the codebase. The god object detector produces scores on a 0-100 scale, but there was a hidden normalization to 0-1 scale in `file_analyzer.rs` that caused filtering bugs. This normalization has been removed as a quick fix, but the underlying issue remains: **there's no type-level enforcement of score scale expectations**.

Current problems:
- Score scales are implicit conventions (0-100 vs 0-1)
- No compiler enforcement of correct scale usage
- Hidden transformations make bugs hard to detect
- Different parts of codebase assume different scales
- Testing can't verify scale consistency

This creates a category of bugs that are:
- **Silent**: Compile successfully but produce wrong values
- **Hard to detect**: Tests use hardcoded values that bypass transformations
- **Hard to debug**: Scale mismatch shows as wrong filtering, not obvious errors

## Objective

Introduce type-safe score scales using Rust's newtype pattern to make score ranges explicit in the type system. This prevents scale mismatch bugs at compile time and makes score transformations explicit and auditable.

The newtype pattern provides zero-cost abstraction—no runtime overhead while gaining compile-time safety.

## Requirements

### Functional Requirements

**FR1**: Define `Score0To100` newtype wrapper
- Wraps `f64` with guaranteed 0-100 bounds
- Provides `.new(value)` constructor that clamps to range
- Provides `.value()` accessor for raw value
- Implements common traits: `Debug`, `Clone`, `Copy`, `PartialEq`, `PartialOrd`

**FR2**: Define `Score0To1` newtype wrapper
- Wraps `f64` with guaranteed 0-1 bounds
- Provides `.new(value)` constructor that clamps to range
- Provides `.value()` accessor for raw value
- Implements common traits: `Debug`, `Clone`, `Copy`, `PartialEq`, `PartialOrd`

**FR3**: Provide explicit scale conversion methods
- `Score0To100::normalize(self) -> Score0To1` - Divide by 100
- `Score0To1::denormalize(self) -> Score0To100` - Multiply by 100
- Both conversions must be explicit method calls (no implicit conversion)
- Conversions must be mathematically reversible (roundtrip property)

**FR4**: Update `GodObjectAnalysis` struct
- Change `god_object_score: f64` to `god_object_score: Score0To100`
- Breaking change requiring coordinated migration
- Update all construction sites to use `Score0To100::new()`

**FR5**: Update `UnifiedScore` struct
- Change `final_score: f64` to `final_score: Score0To100`
- Ensures all debt scores use consistent scale
- Update all score calculations to produce `Score0To100`

### Non-Functional Requirements

**NFR1**: Zero runtime overhead
- Newtype pattern must compile to identical machine code as raw `f64`
- No heap allocations or indirection
- Verify with `cargo bench` that performance is unchanged

**NFR2**: Backward compatibility during migration
- Provide `.value()` method to extract raw `f64` for gradual migration
- Allow incremental adoption—can migrate one module at a time
- Compilation errors guide migration (type errors show where changes needed)

**NFR3**: Comprehensive documentation
- Document why each score type exists
- Provide examples of correct usage
- Document conversion methods with use cases
- Add module-level documentation explaining scale types

**NFR4**: Testability
- Pure functions for all score operations
- Property tests for scale invariants
- Integration tests for roundtrip conversions

## Acceptance Criteria

- [ ] **AC1**: `Score0To100` type defined in `src/priority/score_types.rs` with bounds enforcement
  - Constructor clamps values to [0.0, 100.0]
  - Out-of-bounds values don't panic, just clamp (e.g., 150.0 → 100.0, -10.0 → 0.0)
  - Implements `Debug`, `Clone`, `Copy`, `PartialEq`, `PartialOrd`

- [ ] **AC2**: `Score0To1` type defined in `src/priority/score_types.rs` with bounds enforcement
  - Constructor clamps values to [0.0, 1.0]
  - Out-of-bounds values don't panic, just clamp
  - Implements `Debug`, `Clone`, `Copy`, `PartialEq`, `PartialOrd`

- [ ] **AC3**: Explicit conversion methods implemented
  - `Score0To100::normalize()` correctly divides by 100
  - `Score0To1::denormalize()` correctly multiplies by 100
  - Roundtrip property verified: `score.normalize().denormalize() == score`

- [ ] **AC4**: `GodObjectAnalysis.god_object_score` uses `Score0To100`
  - Field type changed from `f64` to `Score0To100`
  - All construction sites updated
  - All access sites updated or use `.value()` for migration

- [ ] **AC5**: `UnifiedScore.final_score` uses `Score0To100`
  - Field type changed from `f64` to `Score0To100`
  - All score calculations updated to produce `Score0To100`
  - Comparison operations updated to work with typed scores

- [ ] **AC6**: Property tests verify scale invariants
  - Test: Scores always within bounds (0-100 or 0-1)
  - Test: Normalization preserves ordering
  - Test: Roundtrip conversion is identity
  - Test: Clamping behavior correct for out-of-bounds

- [ ] **AC7**: No performance regression
  - Benchmark shows identical performance to raw `f64`
  - `cargo bench` shows no measurable overhead
  - Generated assembly identical (verify with `cargo asm`)

- [ ] **AC8**: Integration test verifies god objects score correctly
  - Test: God object with score 85.0 creates `Score0To100(85.0)`
  - Test: Score comparison works (`score >= threshold`)
  - Test: Score displays correctly in output

- [ ] **AC9**: Documentation complete
  - Module-level docs explain score types and usage
  - Each type has comprehensive rustdoc
  - Examples show conversion patterns
  - Migration guide for updating existing code

- [ ] **AC10**: All tests pass
  - Unit tests for score types
  - Property tests for invariants
  - Integration tests for god object scoring
  - Existing tests updated for new types

## Technical Details

### Implementation Approach

**Phase 1: Define Score Types** (New file: `src/priority/score_types.rs`)

```rust
//! Type-safe score scales for debt scoring system.
//!
//! This module provides newtype wrappers for different score scales used
//! throughout the analysis. By encoding the scale in the type system, we
//! prevent bugs caused by mixing incompatible scales.
//!
//! # Score Scales
//!
//! - `Score0To100`: Standard 0-100 scale for most debt scores
//! - `Score0To1`: Normalized 0-1 scale for certain calculations
//!
//! # Examples
//!
//! ```rust
//! use debtmap::priority::score_types::{Score0To100, Score0To1};
//!
//! // Create scores with automatic bounds enforcement
//! let score = Score0To100::new(85.0);
//! assert_eq!(score.value(), 85.0);
//!
//! // Out-of-bounds values are clamped
//! let clamped = Score0To100::new(150.0);
//! assert_eq!(clamped.value(), 100.0);
//!
//! // Explicit conversion between scales
//! let normalized = score.normalize();
//! assert_eq!(normalized.value(), 0.85);
//!
//! // Roundtrip conversion is identity
//! assert_eq!(score, normalized.denormalize());
//! ```

use serde::{Deserialize, Serialize};

/// Score on 0-100 scale.
///
/// This is the standard scale for debt scores throughout the system.
/// God object scores, unified scores, and threshold configurations
/// all use this scale.
///
/// Values are automatically clamped to the [0.0, 100.0] range.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Score0To100(f64);

impl Score0To100 {
    /// Create a new score, clamping to [0.0, 100.0].
    ///
    /// # Examples
    ///
    /// ```rust
    /// let score = Score0To100::new(85.0);
    /// assert_eq!(score.value(), 85.0);
    ///
    /// let clamped = Score0To100::new(150.0);
    /// assert_eq!(clamped.value(), 100.0);
    /// ```
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 100.0))
    }

    /// Get the raw score value.
    pub fn value(self) -> f64 {
        self.0
    }

    /// Normalize to 0-1 scale by dividing by 100.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let score = Score0To100::new(85.0);
    /// let normalized = score.normalize();
    /// assert_eq!(normalized.value(), 0.85);
    /// ```
    pub fn normalize(self) -> Score0To1 {
        Score0To1(self.0 / 100.0)
    }
}

/// Score on 0-1 scale (normalized).
///
/// This scale is used for certain calculations where normalized
/// values are preferred. Most code should use `Score0To100`.
///
/// Values are automatically clamped to the [0.0, 1.0] range.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Score0To1(f64);

impl Score0To1 {
    /// Create a new normalized score, clamping to [0.0, 1.0].
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the raw score value.
    pub fn value(self) -> f64 {
        self.0
    }

    /// Denormalize to 0-100 scale by multiplying by 100.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let normalized = Score0To1::new(0.85);
    /// let score = normalized.denormalize();
    /// assert_eq!(score.value(), 85.0);
    /// ```
    pub fn denormalize(self) -> Score0To100 {
        Score0To100(self.0 * 100.0)
    }
}

// Implement Display for user-facing output
impl std::fmt::Display for Score0To100 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}", self.0)
    }
}

impl std::fmt::Display for Score0To1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_0_to_100_clamps_upper_bound() {
        let score = Score0To100::new(150.0);
        assert_eq!(score.value(), 100.0);
    }

    #[test]
    fn score_0_to_100_clamps_lower_bound() {
        let score = Score0To100::new(-10.0);
        assert_eq!(score.value(), 0.0);
    }

    #[test]
    fn score_0_to_1_clamps_upper_bound() {
        let score = Score0To1::new(1.5);
        assert_eq!(score.value(), 1.0);
    }

    #[test]
    fn score_0_to_1_clamps_lower_bound() {
        let score = Score0To1::new(-0.5);
        assert_eq!(score.value(), 0.0);
    }

    #[test]
    fn normalization_divides_by_100() {
        let score = Score0To100::new(85.0);
        let normalized = score.normalize();
        assert_eq!(normalized.value(), 0.85);
    }

    #[test]
    fn denormalization_multiplies_by_100() {
        let normalized = Score0To1::new(0.85);
        let score = normalized.denormalize();
        assert_eq!(score.value(), 85.0);
    }

    #[test]
    fn roundtrip_conversion_is_identity() {
        let original = Score0To100::new(75.5);
        let roundtrip = original.normalize().denormalize();
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn comparison_works_correctly() {
        let score1 = Score0To100::new(50.0);
        let score2 = Score0To100::new(75.0);

        assert!(score1 < score2);
        assert!(score2 > score1);
        assert_eq!(score1, Score0To100::new(50.0));
    }
}
```

**Phase 2: Add Property Tests**

```rust
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn score_0_to_100_always_in_bounds(value in -1000.0..1000.0f64) {
            let score = Score0To100::new(value);
            assert!(score.value() >= 0.0 && score.value() <= 100.0);
        }

        #[test]
        fn score_0_to_1_always_in_bounds(value in -10.0..10.0f64) {
            let score = Score0To1::new(value);
            assert!(score.value() >= 0.0 && score.value() <= 1.0);
        }

        #[test]
        fn normalization_preserves_ordering(a in 0.0..100.0f64, b in 0.0..100.0f64) {
            let score_a = Score0To100::new(a);
            let score_b = Score0To100::new(b);

            if a < b {
                assert!(score_a.normalize() < score_b.normalize());
            } else if a > b {
                assert!(score_a.normalize() > score_b.normalize());
            } else {
                assert_eq!(score_a.normalize(), score_b.normalize());
            }
        }

        #[test]
        fn roundtrip_conversion_exact(value in 0.0..100.0f64) {
            let original = Score0To100::new(value);
            let roundtrip = original.normalize().denormalize();
            // Use approximate equality for floating point
            assert!((original.value() - roundtrip.value()).abs() < 1e-10);
        }
    }
}
```

**Phase 3: Update Core Structs**

```rust
// src/organization/god_object/core_types.rs
pub struct GodObjectAnalysis {
    pub is_god_object: bool,
    pub method_count: usize,
    pub field_count: usize,
    pub responsibility_count: usize,
    pub lines_of_code: usize,
    pub complexity_sum: u32,
    pub god_object_score: Score0To100, // Changed from f64
    // ... other fields
}

// src/priority/mod.rs
pub struct UnifiedScore {
    pub final_score: Score0To100, // Changed from f64
    pub complexity_factor: f64,
    pub coverage_factor: f64,
    pub dependency_factor: f64,
    pub role_multiplier: f64,
    // ... other fields
}
```

**Phase 4: Update Score Calculations**

```rust
// Example: src/builders/unified_analysis.rs
pub fn create_god_object_debt_item(
    file_path: &Path,
    file_metrics: &FileDebtMetrics,
    god_analysis: &GodObjectAnalysis,
) -> UnifiedDebtItem {
    // god_analysis.god_object_score is now Score0To100
    let base_score = god_analysis.god_object_score; // No .value() needed

    let unified_score = UnifiedScore {
        final_score: base_score, // Type-safe assignment
        complexity_factor: file_metrics.total_complexity as f64 / 10.0,
        // ...
    };
    // ...
}
```

### Architecture Changes

**New Module**: `src/priority/score_types.rs`
- Defines `Score0To100` and `Score0To1` types
- Provides conversion methods
- Comprehensive documentation and examples

**Updated Modules**:
- `src/organization/god_object/core_types.rs` - Use `Score0To100` for god object scores
- `src/priority/mod.rs` - Use `Score0To100` for unified scores
- `src/builders/unified_analysis.rs` - Update score construction
- `src/priority/unified_analysis_utils.rs` - Update filtering comparisons

**Export Path**: `pub use crate::priority::score_types::{Score0To100, Score0To1};`

### Data Structures

No new data structures beyond the newtype wrappers. Existing structures updated to use typed scores instead of raw `f64`.

### APIs and Interfaces

**Public API**:
```rust
// Construction
let score = Score0To100::new(85.0);

// Access
let raw_value: f64 = score.value();

// Conversion
let normalized: Score0To1 = score.normalize();
let denormalized: Score0To100 = normalized.denormalize();

// Comparison (works automatically via PartialOrd)
if score >= Score0To100::new(50.0) { ... }
```

**Breaking Changes**:
- `GodObjectAnalysis.god_object_score` changes from `f64` to `Score0To100`
- `UnifiedScore.final_score` changes from `f64` to `Score0To100`
- Code that directly accesses these fields must update

**Migration Pattern**:
```rust
// Before
let score: f64 = analysis.god_object_score;
if score >= 50.0 { ... }

// After
let score: Score0To100 = analysis.god_object_score;
if score >= Score0To100::new(50.0) { ... }

// Or use .value() for gradual migration
let raw_score = analysis.god_object_score.value();
if raw_score >= 50.0 { ... }
```

## Dependencies

### Prerequisites
- None - foundational change

### Affected Components
- **God object detection**: `src/organization/god_object/` - Uses `Score0To100`
- **Unified scoring**: `src/priority/` - Uses `Score0To100` for all scores
- **Filtering logic**: `src/priority/unified_analysis_utils.rs` - Compares typed scores
- **Output formatting**: `src/output/` - Display typed scores
- **TUI**: `src/tui/` - Display typed scores in list/detail views

### External Dependencies
- Add `proptest` to dev-dependencies for property testing

```toml
[dev-dependencies]
proptest = "1.0"
```

## Testing Strategy

### Unit Tests
- **Bounds enforcement**: Verify clamping for out-of-bounds values
- **Conversion correctness**: Verify normalization/denormalization math
- **Comparison operations**: Verify `PartialOrd` implementation
- **Display formatting**: Verify string representation

### Property Tests
- **Always in bounds**: Scores never exceed range regardless of input
- **Ordering preservation**: Normalization preserves relative ordering
- **Roundtrip identity**: `score.normalize().denormalize() == score`
- **Clamping idempotent**: `Score0To100::new(value) == Score0To100::new(Score0To100::new(value).value())`

### Integration Tests
- **God object scoring**: End-to-end test from detection to filtering
- **Score comparison**: Verify filtering works with typed scores
- **TUI display**: Verify scores display correctly in TUI
- **JSON serialization**: Verify scores serialize/deserialize correctly

### Performance Tests
- **Benchmark**: Verify no overhead vs raw `f64`
- **Assembly inspection**: Use `cargo asm` to verify identical codegen
- **Memory layout**: Verify `std::mem::size_of::<Score0To100>() == std::mem::size_of::<f64>()`

## Documentation Requirements

### Code Documentation
- Module-level docs explaining score types and usage
- Comprehensive rustdoc for each type
- Examples for common operations
- Migration guide in module docs

### User Documentation
- No user-facing documentation needed (internal API change)

### Architecture Updates
Update `ARCHITECTURE.md`:
- Add section on "Type-Safe Score Scales"
- Document the newtype pattern and rationale
- Explain scale invariants and guarantees

## Implementation Notes

### Pure Functions
All score operations are pure:
- `new()`: Pure transformation (clamping)
- `value()`: Pure accessor
- `normalize()`: Pure conversion
- `denormalize()`: Pure conversion

### Zero-Cost Abstraction
The newtype pattern is zero-cost:
- No heap allocation
- No vtable indirection
- Compiles to identical machine code as `f64`
- `#[repr(transparent)]` could be added if needed

### Incremental Migration
Can migrate gradually:
1. Define types
2. Update `GodObjectAnalysis`
3. Fix compilation errors in god object code
4. Update `UnifiedScore`
5. Fix compilation errors in scoring code
6. Remove temporary `.value()` calls

### Common Pitfalls
- **Don't use raw `f64` for scores**: Always use typed scores
- **Don't skip `.new()` constructor**: Ensures bounds checking
- **Don't mix scales**: Type system prevents this
- **Don't assume scores are always valid**: Still need to handle edge cases

## Migration and Compatibility

### Breaking Changes
- `GodObjectAnalysis.god_object_score`: `f64` → `Score0To100`
- `UnifiedScore.final_score`: `f64` → `Score0To100`

### Migration Steps
1. Add `score_types.rs` module
2. Export types in `src/priority/mod.rs`
3. Update `GodObjectAnalysis` struct
4. Fix compilation errors in god object modules
5. Update `UnifiedScore` struct
6. Fix compilation errors in priority modules
7. Update tests to use typed scores
8. Remove temporary `.value()` migrations

### Backward Compatibility
- Can provide `.value()` accessor for gradual migration
- Old code can call `.value()` to get raw `f64`
- Once all code migrated, can deprecate or keep `.value()`

### Rollback Plan
If issues arise:
1. Revert struct field type changes
2. Keep score types module (useful for future)
3. Document learnings for next attempt

## Success Metrics

- [ ] All tests pass with typed scores
- [ ] No performance regression (benchmark within 1%)
- [ ] Compilation guides migration (type errors are helpful)
- [ ] God objects score correctly (85-100 range)
- [ ] Filtering works correctly with typed scores
- [ ] Zero instances of score scale confusion bugs

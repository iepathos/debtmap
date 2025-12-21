---
number: 001
title: Refined Types for Domain Invariants
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-12-20
---

# Specification 001: Refined Types for Domain Invariants

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently uses raw numeric types (`u32`, `f64`, `usize`) for domain values like complexity thresholds, scores, and line counts. These values have implicit invariants (e.g., thresholds must be positive, scores must be in range 0-100) that are validated inconsistently throughout the codebase. This leads to:

1. **Scattered validation** - Same checks repeated in multiple locations
2. **Runtime errors** - Invalid values can propagate before being caught
3. **Unclear contracts** - Function signatures don't communicate constraints
4. **Documentation burden** - Invariants exist only in comments or documentation

Stillwater 1.0 introduces refined types that implement the "parse, don't validate" pattern - validating data once at system boundaries and using types to guarantee validity throughout the codebase.

## Objective

Introduce refined types for core domain values in debtmap, ensuring type-level invariants for thresholds, scores, and metrics. This provides compile-time safety with zero runtime overhead while making the codebase more self-documenting.

## Requirements

### Functional Requirements

1. **Create refined type module** (`src/refined_types.rs` or `src/core/refined.rs`)
   - Define type aliases using stillwater's `Refined<T, P>` wrapper
   - Create custom predicates for domain-specific constraints
   - Export types for use throughout codebase

2. **Complexity thresholds** - Replace raw threshold values
   - `ComplexityThreshold`: `u32` in range [1, 1000]
   - `CognitiveThreshold`: `u32` in range [1, 500]
   - `NestingThreshold`: `u32` in range [1, 50]

3. **Score types** - Replace raw score values
   - `NormalizedScore`: `u32` in range [0, 100]
   - `RawComplexityScore`: `u32` in range [0, 1000]
   - `RiskScore`: `f64` in range [0.0, 1.0]

4. **Metric types** - Replace raw metric values
   - `LineCount`: `u32` that is positive (> 0)
   - `NestingDepth`: `u32` that is non-negative
   - `FunctionCount`: `usize` that is non-negative

5. **Configuration types** - Replace raw config values
   - `WeightFactor`: `f64` in range [0.0, 1.0]
   - `Percentage`: `u32` in range [0, 100]

### Non-Functional Requirements

- **Zero runtime overhead** - Same memory layout as inner types
- **Ergonomic API** - Use `Deref` for transparent access
- **Serde support** - Serialize/deserialize refined types
- **Clone and Debug** - Standard trait implementations
- **Backward compatibility** - Gradual migration with `.into_inner()` escape hatch

## Acceptance Criteria

- [ ] New module `src/core/refined.rs` or `src/refined_types.rs` created
- [ ] At least 5 refined type aliases defined for core domain values
- [ ] At least 2 custom predicates created for domain-specific constraints
- [ ] `ThresholdsConfig` updated to use refined threshold types
- [ ] `ScoringConfig` updated to use refined weight types
- [ ] Unit tests verify predicate behavior (success and failure cases)
- [ ] `Cargo.toml` updated to use stillwater 1.0 with serde feature
- [ ] No increase in binary size from refined type usage (verified)

## Technical Details

### Implementation Approach

1. **Phase 1: Core Types**
   - Create refined types module
   - Define predicates for numeric ranges
   - Add type aliases for common patterns

2. **Phase 2: Configuration Integration**
   - Update `ThresholdsConfig` to use refined types
   - Update `ScoringConfig` to use refined types
   - Modify config parsing to validate at boundary

3. **Phase 3: Metrics Integration**
   - Update `FunctionMetrics` to use refined types
   - Update `FileMetrics` to use refined types
   - Ensure analysis pipelines produce refined values

### Architecture Changes

```rust
// New module: src/core/refined.rs
use stillwater::refined::{Refined, Predicate, InRange, Positive, NonNegative};

// Threshold types
pub type ComplexityThreshold = Refined<u32, InRange<1, 1000>>;
pub type CognitiveThreshold = Refined<u32, InRange<1, 500>>;
pub type NestingThreshold = Refined<u32, InRange<1, 50>>;

// Score types
pub type NormalizedScore = Refined<u32, InRange<0, 100>>;
pub type RiskScore = Refined<f64, UnitInterval>;

// Custom predicate for [0.0, 1.0] range
pub struct UnitInterval;

impl Predicate<f64> for UnitInterval {
    type Error = &'static str;

    fn check(value: &f64) -> Result<(), Self::Error> {
        if *value >= 0.0 && *value <= 1.0 {
            Ok(())
        } else {
            Err("value must be in range [0.0, 1.0]")
        }
    }
}
```

### Data Structures

**Before:**
```rust
pub struct ThresholdsConfig {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
}
```

**After:**
```rust
pub struct ThresholdsConfig {
    pub cyclomatic: ComplexityThreshold,
    pub cognitive: CognitiveThreshold,
    pub nesting: NestingThreshold,
}
```

### APIs and Interfaces

- Refined types implement `Deref` for transparent access
- `get()` and `into_inner()` methods for explicit access
- `new()` constructor returns `Result` for validation
- `new_unchecked()` for performance-critical trusted contexts

## Dependencies

- **Prerequisites**: None (foundation specification)
- **Affected Components**:
  - `src/config/mod.rs` - Configuration parsing
  - `src/core/types.rs` - Core domain types
  - `src/complexity/mod.rs` - Complexity calculations
- **External Dependencies**: stillwater 1.0 with `serde` feature

## Testing Strategy

- **Unit Tests**:
  - Test each predicate with valid and invalid values
  - Test boundary conditions (0, max, min, overflow)
  - Test `try_map` transformations

- **Integration Tests**:
  - Test config parsing with refined types
  - Test analysis pipeline produces valid refined values

- **Property Tests**:
  - Verify predicate consistency with proptest
  - Test roundtrip serialization

## Documentation Requirements

- **Code Documentation**:
  - Rustdoc for all public types and predicates
  - Examples in module-level documentation

- **User Documentation**:
  - Update configuration documentation with valid ranges

- **Architecture Updates**:
  - Add refined types to ARCHITECTURE.md type system section

## Implementation Notes

1. Start with configuration types as they have clear boundaries
2. Use `validate_vec()` for error accumulation in multi-field validation
3. Consider `ValidationFieldExt` for field-specific error context
4. Prefer `InRange<MIN, MAX>` over custom predicates when possible
5. Use `And` combinator for multiple constraints on same type

## Migration and Compatibility

- **Breaking Changes**: Internal API changes, external config format unchanged
- **Migration Path**:
  1. Add refined types alongside existing types
  2. Update internal usage incrementally
  3. Remove old type aliases after full migration
- **Compatibility**: Config files remain valid; validation moved to parse time

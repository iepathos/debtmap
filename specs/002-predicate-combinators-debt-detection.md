---
number: 002
title: Predicate Combinators for Debt Detection Rules
category: foundation
priority: high
status: draft
dependencies: [001]
created: 2025-12-20
---

# Specification 002: Predicate Combinators for Debt Detection Rules

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 001 (Refined Types for Domain Invariants)

## Context

Debtmap currently implements debt detection through scattered conditional logic across multiple modules. Patterns like "high complexity", "low coverage", "deep nesting", and "god object" are detected using ad-hoc conditional checks that are:

1. **Difficult to compose** - Combining multiple conditions requires manual boolean logic
2. **Hard to test** - Predicates are embedded in larger functions
3. **Not reusable** - Same patterns reimplemented in different contexts
4. **Opaque to users** - Detection rules cannot be easily inspected or modified

Stillwater 1.0 provides a rich predicate combinator system that allows building complex validation rules from simple, composable pieces. This includes:

- Basic predicates: `Positive`, `NonNegative`, `InRange`, `NonEmpty`
- Logical combinators: `And`, `Or`, `Not`
- String predicates: `len_between`, `contains`, `starts_with`
- Collection predicates: `all`, `any`, `has_len`
- Effect integration: `.ensure()`, `.ensure_pred()`, `.filter_or()`

## Objective

Implement a composable predicate system for debt detection rules using stillwater's predicate combinators. This enables declarative rule definition, easy composition of complex detection patterns, and user-configurable detection thresholds.

## Requirements

### Functional Requirements

1. **Create debt predicate module** (`src/debt/predicates.rs`)
   - Define predicates for common debt patterns
   - Implement composition using `And`, `Or`, `Not`
   - Support parameterized predicates with thresholds

2. **Complexity predicates**
   - `HighCyclomatic`: Cyclomatic complexity > threshold
   - `HighCognitive`: Cognitive complexity > threshold
   - `CriticalComplexity`: Both cyclomatic AND cognitive exceed thresholds

3. **Structure predicates**
   - `DeepNesting`: Nesting depth > threshold
   - `LongMethod`: Line count > threshold
   - `TooManyParameters`: Parameter count > threshold

4. **Coverage predicates**
   - `LowCoverage`: Coverage < threshold (e.g., 50%)
   - `NoCoverage`: Coverage = 0%
   - `PartialCoverage`: Coverage between 0% and threshold

5. **Risk predicates** (composed)
   - `HighRisk`: High complexity AND low coverage
   - `CriticalRisk`: Critical complexity AND no coverage
   - `ModerateRisk`: High complexity OR low coverage

6. **God object predicates**
   - `TooManyMethods`: Method count > threshold
   - `TooManyDependencies`: Fan-out > threshold
   - `GodObject`: TooManyMethods AND TooManyDependencies

### Non-Functional Requirements

- **Composability** - Predicates combine with `and()`, `or()`, `not()`
- **Testability** - Each predicate independently testable
- **Configurability** - Thresholds parameterized via config
- **Zero allocation** - Use stillwater's zero-cost predicate system
- **Documentation** - Each predicate self-documenting via `description()`

## Acceptance Criteria

- [ ] New module `src/debt/predicates.rs` created with predicate definitions
- [ ] At least 5 complexity/structure predicates implemented
- [ ] At least 3 composed predicates using `And`/`Or`/`Not`
- [ ] Predicates integrated with existing debt detection in `src/debt/`
- [ ] Predicates accept thresholds from `DebtmapConfig`
- [ ] Unit tests for each predicate (success and failure cases)
- [ ] Integration test showing predicate composition for risk detection
- [ ] Example demonstrating user-defined detection rules

## Technical Details

### Implementation Approach

1. **Phase 1: Core Predicates**
   - Define struct for each predicate type
   - Implement `Predicate<T>` trait from stillwater
   - Add constructor accepting threshold parameters

2. **Phase 2: Composed Predicates**
   - Create type aliases for common combinations
   - Document composition patterns
   - Add factory functions for configured predicates

3. **Phase 3: Integration**
   - Update debt detection to use predicates
   - Replace inline conditionals with `.ensure_pred()`
   - Add predicate-based filtering in analysis

### Architecture Changes

```rust
// src/debt/predicates.rs
use stillwater::predicate::{Predicate, PredicateExt};
use stillwater::refined::And;

/// Predicate for high cyclomatic complexity
pub struct HighCyclomatic {
    threshold: u32,
}

impl HighCyclomatic {
    pub fn new(threshold: u32) -> Self {
        Self { threshold }
    }

    pub fn from_config(config: &ThresholdsConfig) -> Self {
        Self::new(config.cyclomatic.into_inner())
    }
}

impl Predicate<FunctionMetrics> for HighCyclomatic {
    fn test(&self, metrics: &FunctionMetrics) -> bool {
        metrics.cyclomatic > self.threshold
    }

    fn description(&self) -> String {
        format!("cyclomatic complexity > {}", self.threshold)
    }
}

/// Predicate for low test coverage
pub struct LowCoverage {
    threshold: f64,
}

impl Predicate<FunctionMetrics> for LowCoverage {
    fn test(&self, metrics: &FunctionMetrics) -> bool {
        metrics.coverage.map_or(true, |c| c < self.threshold)
    }
}

/// Composed predicate: High risk = high complexity AND low coverage
pub type HighRisk = And<HighCyclomatic, LowCoverage>;

/// Factory for creating configured predicates
pub struct DebtPredicates {
    pub high_cyclomatic: HighCyclomatic,
    pub high_cognitive: HighCognitive,
    pub deep_nesting: DeepNesting,
    pub long_method: LongMethod,
    pub low_coverage: LowCoverage,
}

impl DebtPredicates {
    pub fn from_config(config: &DebtmapConfig) -> Self {
        Self {
            high_cyclomatic: HighCyclomatic::from_config(&config.thresholds),
            high_cognitive: HighCognitive::from_config(&config.thresholds),
            deep_nesting: DeepNesting::from_config(&config.thresholds),
            long_method: LongMethod::from_config(&config.thresholds),
            low_coverage: LowCoverage::new(config.coverage_threshold),
        }
    }

    /// Create a high-risk predicate
    pub fn high_risk(&self) -> impl Predicate<FunctionMetrics> + '_ {
        self.high_cyclomatic.clone().and(self.low_coverage.clone())
    }
}
```

### Data Structures

```rust
/// Result of predicate evaluation with context
pub struct PredicateResult {
    pub matched: bool,
    pub predicate_name: String,
    pub description: String,
    pub details: Option<String>,
}

/// Collection of matched predicates for a function
pub struct DebtFindings {
    pub function: FunctionMetrics,
    pub matched_predicates: Vec<PredicateResult>,
    pub overall_risk: RiskLevel,
}
```

### APIs and Interfaces

```rust
// Usage in debt detection
fn detect_debt(metrics: &FunctionMetrics, predicates: &DebtPredicates) -> Vec<DebtItem> {
    let mut items = Vec::new();

    if predicates.high_cyclomatic.test(metrics) {
        items.push(DebtItem::high_complexity(metrics));
    }

    if predicates.high_risk().test(metrics) {
        items.push(DebtItem::high_risk(metrics));
    }

    items
}

// Usage with Effect ensure
fn analyze_function(metrics: FunctionMetrics) -> impl Effect<...> {
    pure(metrics)
        .ensure_pred(not_critical_complexity, DebtWarning::CriticalComplexity)
        .ensure_pred(not_too_deep, DebtWarning::DeepNesting)
}
```

## Dependencies

- **Prerequisites**: Spec 001 (Refined Types for Domain Invariants)
- **Affected Components**:
  - `src/debt/mod.rs` - Debt detection logic
  - `src/debt/detectors/` - Individual debt detectors
  - `src/organization/god_object/` - God object detection
  - `src/risk/mod.rs` - Risk assessment
- **External Dependencies**: stillwater 1.0 predicate module

## Testing Strategy

- **Unit Tests**:
  - Test each predicate with boundary values
  - Test composition with `and()`, `or()`, `not()`
  - Test factory functions with various configs

- **Integration Tests**:
  - Test predicate-based debt detection
  - Test multi-predicate evaluation
  - Test configurable thresholds

- **Property Tests**:
  - Verify predicate laws (idempotence, commutativity of `and`/`or`)
  - Test threshold boundary behavior

## Documentation Requirements

- **Code Documentation**:
  - Rustdoc for all predicates with examples
  - Document predicate composition patterns

- **User Documentation**:
  - Add section on configurable detection rules
  - Document available predicates and their meanings

- **Architecture Updates**:
  - Add predicates to debt detection architecture section

## Implementation Notes

1. Use stillwater's runtime `Predicate` trait for debt detection (not refined types)
2. Predicates should be `Clone` for composition flexibility
3. Consider caching predicate evaluation for repeated checks
4. `description()` method enables self-documenting error messages
5. Parameterized predicates enable per-project customization

## Migration and Compatibility

- **Breaking Changes**: Internal refactoring, no user-facing changes
- **Migration Path**:
  1. Add predicate module alongside existing detection
  2. Refactor detectors to use predicates internally
  3. Expose predicate configuration in config file
- **Compatibility**: Detection behavior unchanged with default config

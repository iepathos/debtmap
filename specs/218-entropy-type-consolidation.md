---
number: 218
title: Entropy Type Consolidation - Single Source of Truth
category: architecture
priority: high
status: draft
dependencies: []
created: 2025-12-17
---

# Specification 218: Entropy Type Consolidation - Single Source of Truth

**Category**: architecture
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current entropy implementation has grown organically across multiple modules, creating a tangled web of redundant types, multiple aggregation paths, and unclear data flow. This makes the codebase difficult to understand and maintain.

### Current Problems

1. **Multiple Representations of the Same Concept**:
   - `complexity::entropy_core::EntropyScore` (7 fields) - raw calculation output
   - `complexity::entropy::EntropyScore` (duplicate definition, 7 fields) - Rust-specific
   - `priority::unified_scorer::EntropyDetails` (6 fields) - scoring layer
   - `core::EntropyDetails` (8 fields) - yet another definition
   - `god_object_aggregation::GodObjectAggregatedMetrics.aggregated_entropy` - aggregated form
   - `UnifiedDebtItem.entropy_details` / `entropy_adjusted_cognitive` / `entropy_dampening_factor` - output fields

2. **Multiple Aggregation Functions**:
   - `aggregate_entropy_metrics()` - from UnifiedDebtItems
   - `aggregate_entropy_from_raw()` - from FunctionMetrics
   - `aggregate_from_raw_metrics()` - includes entropy
   - Manual inline aggregation in `god_object.rs`

3. **Unclear Data Flow**:
   - Entropy calculated in extractor → FunctionMetrics.entropy_score
   - May or may not propagate to UnifiedDebtItem.entropy_details
   - May or may not propagate to god_object_indicators.aggregated_entropy
   - Multiple conversion points where data can be lost

4. **No Single Source of Truth**:
   - Same calculation done multiple times
   - No clear ownership of entropy logic
   - Difficult to debug when values don't appear in output

### Example Data Loss Path (Current Bug)

```
extractor.rs: calculate entropy → entropy_score: Some(0.444)
     ↓
metrics_adapter.rs: pass through → FunctionMetrics.entropy_score: Some(...)
     ↓
god_object.rs: aggregate_from_raw_metrics() → aggregated_entropy: Some(...)
     ↓
god_object.rs:90-92: HARDCODED to None! ← Data loss!
     ↓
TUI: god_object_indicators.aggregated_entropy: null
```

## Objective

Consolidate entropy into a single module following Stillwater philosophy:
- **Pure Core**: All entropy calculation and transformation logic is pure
- **Single Source of Truth**: One canonical type definition, one aggregation function
- **Clear Data Flow**: Entropy flows through the pipeline without transformation

### Success Criteria

1. Single `EntropyAnalysis` type used throughout the codebase
2. One `aggregate_entropy()` function for all aggregation needs
3. Clear ownership: `complexity/entropy_core.rs` owns all entropy logic
4. Entropy data never lost in data transformations
5. All tests pass with no regression

## Design

### Module Structure

```
src/complexity/
├── mod.rs              # Re-exports
├── entropy_core.rs     # SINGLE SOURCE OF TRUTH
│   ├── EntropyScore    # Raw calculation (keep as-is)
│   ├── EntropyAnalysis # NEW: Unified analysis type
│   ├── calculate_entropy()      # Entry point
│   ├── analyze_entropy()        # EntropyScore → EntropyAnalysis
│   └── aggregate_entropy()      # Single aggregation function
├── entropy.rs          # Rust-specific AST analysis (uses entropy_core)
└── entropy_traits.rs   # Traits for language-agnostic analysis
```

### Type Definitions

#### Keep: EntropyScore (Raw Calculation)
```rust
// In entropy_core.rs - Already exists, no changes needed
pub struct EntropyScore {
    pub token_entropy: f64,
    pub pattern_repetition: f64,
    pub branch_similarity: f64,
    pub effective_complexity: f64,
    pub unique_variables: usize,
    pub max_nesting: u32,
    pub dampening_applied: f64,
}
```

#### New: EntropyAnalysis (Unified Analysis Type)
```rust
// In entropy_core.rs - Replaces all other EntropyDetails types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntropyAnalysis {
    // Core entropy value (from EntropyScore.token_entropy)
    pub entropy_score: f64,

    // Pattern metrics
    pub pattern_repetition: f64,
    pub branch_similarity: f64,

    // Dampening results
    pub dampening_factor: f64,          // 0.5-1.0
    pub dampening_was_applied: bool,    // True if entropy < threshold

    // Complexity adjustments
    pub original_complexity: u32,
    pub adjusted_complexity: u32,

    // Explanation for humans
    pub reasoning: Vec<String>,
}
```

### Pure Functions

#### analyze_entropy() - Convert Raw to Analysis
```rust
/// Pure function: Convert raw EntropyScore to unified EntropyAnalysis
pub fn analyze_entropy(
    raw: &EntropyScore,
    original_complexity: u32,
    config: &EntropyConfig,
) -> EntropyAnalysis {
    let dampening_factor = calculate_dampening_factor(
        raw.token_entropy,
        raw.pattern_repetition,
        config,
    );

    let adjusted = (original_complexity as f64 * dampening_factor) as u32;
    let was_applied = dampening_factor < 1.0;

    let reasoning = build_reasoning(raw, dampening_factor, was_applied);

    EntropyAnalysis {
        entropy_score: raw.token_entropy,
        pattern_repetition: raw.pattern_repetition,
        branch_similarity: raw.branch_similarity,
        dampening_factor,
        dampening_was_applied: was_applied,
        original_complexity,
        adjusted_complexity: adjusted,
        reasoning,
    }
}
```

#### aggregate_entropy() - Single Aggregation Function
```rust
/// Pure function: Aggregate entropy from multiple functions
///
/// Uses length-weighted averaging for entropy values,
/// sums for complexity values.
pub fn aggregate_entropy<'a>(
    items: impl Iterator<Item = (&'a EntropyAnalysis, usize)>, // (entropy, function_length)
) -> Option<EntropyAnalysis> {
    let data: Vec<_> = items.collect();
    if data.is_empty() {
        return None;
    }

    let total_length: usize = data.iter().map(|(_, len)| len).sum();
    if total_length == 0 {
        return None;
    }

    // Weighted averages
    let entropy_score = weighted_avg(&data, total_length, |e| e.entropy_score);
    let pattern_repetition = weighted_avg(&data, total_length, |e| e.pattern_repetition);
    let branch_similarity = weighted_avg(&data, total_length, |e| e.branch_similarity);
    let dampening_factor = weighted_avg(&data, total_length, |e| e.dampening_factor);

    // Sums
    let original_complexity = data.iter().map(|(e, _)| e.original_complexity).sum();
    let adjusted_complexity = data.iter().map(|(e, _)| e.adjusted_complexity).sum();

    Some(EntropyAnalysis {
        entropy_score,
        pattern_repetition,
        branch_similarity,
        dampening_factor,
        dampening_was_applied: dampening_factor < 1.0,
        original_complexity,
        adjusted_complexity,
        reasoning: vec![format!(
            "Aggregated from {} functions (weighted by length)",
            data.len()
        )],
    })
}

fn weighted_avg<F>(
    data: &[(&EntropyAnalysis, usize)],
    total_length: usize,
    f: F,
) -> f64
where
    F: Fn(&EntropyAnalysis) -> f64,
{
    data.iter()
        .map(|(e, len)| f(e) * (*len as f64))
        .sum::<f64>()
        / total_length as f64
}
```

### Data Flow (After Consolidation)

```
┌─────────────────────────────────────────────────────────────────┐
│                        EXTRACTION PHASE                          │
├─────────────────────────────────────────────────────────────────┤
│  extractor.rs                                                    │
│    │                                                             │
│    ▼                                                             │
│  calculate_entropy() → EntropyScore (raw)                       │
│    │                                                             │
│    ▼                                                             │
│  analyze_entropy() → EntropyAnalysis                            │
│    │                                                             │
│    └──► ExtractedFunctionData.entropy: Option<EntropyAnalysis>  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         ADAPTER PHASE                            │
├─────────────────────────────────────────────────────────────────┤
│  metrics_adapter.rs                                              │
│    │                                                             │
│    └──► FunctionMetrics.entropy: Option<EntropyAnalysis>        │
│           (direct passthrough, no transformation)                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        ANALYSIS PHASE                            │
├─────────────────────────────────────────────────────────────────┤
│  god_object.rs / scoring.rs                                      │
│    │                                                             │
│    ├──► UnifiedDebtItem.entropy: Option<EntropyAnalysis>        │
│    │                                                             │
│    └──► aggregate_entropy(functions) → EntropyAnalysis          │
│           │                                                      │
│           └──► GodObjectIndicators.aggregated_entropy            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         OUTPUT PHASE                             │
├─────────────────────────────────────────────────────────────────┤
│  output/unified/types.rs                                         │
│    │                                                             │
│    └──► JSON/TUI directly uses EntropyAnalysis                  │
│           - entropy_score                                        │
│           - dampening_factor                                     │
│           - adjusted_complexity                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Stage 1: Create Unified Type
**Goal**: Add `EntropyAnalysis` to entropy_core.rs without breaking existing code

1. Add `EntropyAnalysis` struct to `src/complexity/entropy_core.rs`
2. Add `analyze_entropy()` pure function
3. Add `aggregate_entropy()` pure function
4. Add comprehensive tests

**Files Changed**: `src/complexity/entropy_core.rs`

### Stage 2: Update FunctionMetrics
**Goal**: Replace `entropy_score: Option<EntropyScore>` with `entropy: Option<EntropyAnalysis>`

1. Update `FunctionMetrics` in `src/core/mod.rs`
2. Update extractor to produce `EntropyAnalysis` directly
3. Update metrics adapter (should simplify - direct passthrough)
4. Run tests, fix any breakages

**Files Changed**:
- `src/core/mod.rs`
- `src/extraction/extractor.rs`
- `src/extraction/adapters/metrics.rs`

### Stage 3: Update UnifiedDebtItem
**Goal**: Consolidate entropy fields into single field

1. Replace in `UnifiedDebtItem`:
   ```rust
   // Before
   pub entropy_details: Option<EntropyDetails>,
   pub entropy_adjusted_cognitive: Option<u32>,
   pub entropy_dampening_factor: Option<f64>,

   // After
   pub entropy: Option<EntropyAnalysis>,  // Single field
   ```
2. Update scoring code to use new field
3. Update formatters/output to use new structure

**Files Changed**:
- `src/priority/unified_scorer.rs`
- `src/builders/unified_analysis_phases/phases/scoring.rs`
- Multiple output formatters

### Stage 4: Update God Object Analysis
**Goal**: Use consolidated types and single aggregation

1. Update `GodObjectAggregatedMetrics.aggregated_entropy` type
2. Remove `aggregate_entropy_metrics()` and `aggregate_entropy_from_raw()`
3. Use single `aggregate_entropy()` from entropy_core
4. Fix god_object.rs to NOT hardcode entropy to None

**Files Changed**:
- `src/priority/god_object_aggregation.rs`
- `src/builders/unified_analysis_phases/phases/god_object.rs`

### Stage 5: Remove Deprecated Types
**Goal**: Clean up old types

1. Remove `priority::unified_scorer::EntropyDetails` (now in entropy_core)
2. Remove `core::EntropyDetails` (now in entropy_core)
3. Remove duplicate `complexity::entropy::EntropyScore` (use entropy_core)
4. Update all imports

**Files Changed**: Multiple files with import updates

### Stage 6: Verify End-to-End
**Goal**: Ensure entropy flows correctly through entire pipeline

1. Run full test suite
2. Test with real codebase (self-analysis)
3. Verify TUI Score Breakdown shows entropy for god objects
4. Verify JSON output has complete entropy data

## Testing Strategy

### Unit Tests (entropy_core.rs)
```rust
#[test]
fn test_analyze_entropy_applies_dampening() {
    let raw = EntropyScore { token_entropy: 0.15, /* low entropy */ ... };
    let result = analyze_entropy(&raw, 100, &EntropyConfig::default());

    assert!(result.dampening_was_applied);
    assert!(result.adjusted_complexity < 100);
}

#[test]
fn test_aggregate_entropy_weighted_average() {
    let e1 = EntropyAnalysis { entropy_score: 0.4, ... };
    let e2 = EntropyAnalysis { entropy_score: 0.6, ... };

    let result = aggregate_entropy(
        [(&e1, 100), (&e2, 200)].into_iter()
    ).unwrap();

    // (100*0.4 + 200*0.6) / 300 ≈ 0.533
    assert!((result.entropy_score - 0.533).abs() < 0.01);
}
```

### Integration Tests
```rust
#[test]
fn test_entropy_flows_to_god_object_output() {
    let result = analyze_codebase("test_fixtures/god_object.rs");
    let god_objects = result.god_objects();

    assert!(!god_objects.is_empty());
    for god in god_objects {
        assert!(god.god_object_indicators.unwrap().aggregated_entropy.is_some(),
            "Entropy should propagate to god object output");
    }
}
```

## Migration Guide

### For Internal Code
Replace scattered entropy types with single import:
```rust
// Before
use crate::priority::unified_scorer::EntropyDetails;
use crate::core::EntropyDetails as CoreEntropyDetails;

// After
use crate::complexity::entropy_core::EntropyAnalysis;
```

### For External Consumers
JSON output structure changes:
```json
// Before (inconsistent)
{
  "entropy_details": { ... },
  "entropy_adjusted_cognitive": 45,
  "entropy_dampening_factor": 0.8
}

// After (unified)
{
  "entropy": {
    "entropy_score": 0.35,
    "pattern_repetition": 0.6,
    "branch_similarity": 0.2,
    "dampening_factor": 0.8,
    "dampening_was_applied": true,
    "original_complexity": 56,
    "adjusted_complexity": 45,
    "reasoning": ["Low entropy (0.35) with high repetition (0.6)"]
  }
}
```

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking JSON output format | Staged rollout, document changes |
| Missing entropy data | Comprehensive integration tests |
| Performance regression | Benchmark before/after |
| Merge conflicts | Single PR per stage |

## Non-Goals

- Changing entropy calculation algorithms
- Adding new entropy metrics
- Performance optimization (separate spec)
- Multi-language entropy support (already exists)

## Related Specs

- None (this is foundational cleanup)

## Appendix: Current Type Inventory

### Types to Keep (Modified)
- `complexity::entropy_core::EntropyScore` - raw calculation, add conversion method

### Types to Add
- `complexity::entropy_core::EntropyAnalysis` - unified analysis type

### Types to Remove
- `priority::unified_scorer::EntropyDetails`
- `core::EntropyDetails`
- `complexity::entropy::EntropyScore` (duplicate)

### Functions to Consolidate
- `aggregate_entropy_metrics()` → `aggregate_entropy()`
- `aggregate_entropy_from_raw()` → `aggregate_entropy()`
- `aggregate_from_raw_metrics()` → uses `aggregate_entropy()`

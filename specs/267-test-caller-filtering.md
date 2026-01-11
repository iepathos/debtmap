---
number: 267
title: Test Caller Filtering for Dependency Metrics
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-01-10
---

# Specification 267: Test Caller Filtering for Dependency Metrics

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently counts all callers equally when calculating dependency metrics like blast radius and upstream caller count. This creates false positives where well-tested code is penalized for having comprehensive test coverage.

### Problem Analysis

A real-world example from `cargo-cargofmt/src/formatting/overflow.rs`:
- **90 upstream callers** reported
- **85+ are test functions** (e.g., `multiline_literal_string_preserved`, `already_vertical_not_modified`)
- **Only ~5 are production callers**
- **Blast radius: 121** (marked as "critical")

The code is actually well-designed with small focused functions and comprehensive tests, but debtmap flags it as high-priority debt because test coverage inflates the "blast radius" metric.

### Root Cause

The `aggregate_dependency_metrics` function in `src/priority/god_object_aggregation.rs:139-160` collects all callers without distinguishing test vs. production:

```rust
for item in members {
    unique_callers.extend(item.upstream_callers.iter().cloned());  // No test filtering
}
```

The existing `CallGraph::is_test_function()` and `CallGraph::find_test_only_functions()` infrastructure in `src/priority/call_graph/test_analysis.rs` is not utilized during dependency aggregation.

## Objective

Separate test callers from production callers in dependency metrics to:
1. Report accurate production blast radius for change risk assessment
2. Recognize high test coverage as a positive signal, not a penalty
3. Provide separate test caller counts for test health visibility
4. Reduce false positives for well-tested code

## Requirements

### Functional Requirements

#### FR-1: Separate Caller Classification
- Classify each upstream caller as either "test" or "production"
- Use existing `CallGraph::is_test_function()` for classification
- Handle cases where call graph is unavailable (fallback to heuristics)

#### FR-2: Dual Dependency Counts
- Track `upstream_production_callers: Vec<String>` separately from `upstream_test_callers: Vec<String>`
- Calculate `production_blast_radius` using only production callers
- Maintain backwards compatibility with existing `upstream_callers` field

#### FR-3: Scoring Impact
- Use `production_blast_radius` for debt scoring calculations
- Use `production_upstream_count` in `calculate_dependency_factor()`
- Do not penalize code for having test callers

#### FR-4: Heuristic Fallback
When call graph data is unavailable, classify callers by heuristic:
- Caller name contains `test_` prefix → test
- Caller path contains `/tests/` or `#[cfg(test)]` module → test
- Caller name matches common test patterns (`should_`, `it_`, `spec_`) → test

### Non-Functional Requirements

#### NFR-1: Performance
- Classification should add <5% overhead to dependency analysis
- Use lazy evaluation for test classification when possible

#### NFR-2: Accuracy
- Correctly classify >95% of callers as test vs. production
- False positive rate for test classification <2%

## Acceptance Criteria

- [ ] `UnifiedDebtItem` has new fields `upstream_production_callers` and `upstream_test_callers`
- [ ] `DependencyMetrics` struct includes `production_upstream_count` and `test_upstream_count`
- [ ] `aggregate_dependency_metrics()` separates test from production callers
- [ ] `calculate_dependency_factor()` uses production caller count only
- [ ] LLM markdown output shows both counts: "Upstream Callers: 5 production, 85 test"
- [ ] Blast radius calculation uses production callers only
- [ ] Existing test function detection in call graph is utilized
- [ ] Heuristic fallback works when call graph is unavailable
- [ ] Unit tests verify correct classification for known test patterns
- [ ] Integration test with `overflow.rs`-like scenario shows reduced false positive score

## Technical Details

### Implementation Approach

#### Phase 1: Data Structure Updates

Update `src/output/unified/dependencies.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyMetrics {
    pub upstream_count: usize,           // Total (for backwards compat)
    pub production_upstream_count: usize, // NEW: Production only
    pub test_upstream_count: usize,       // NEW: Test only
    pub downstream_count: usize,
    // ... existing fields
}
```

Update `src/priority/mod.rs` `UnifiedDebtItem`:
```rust
pub struct UnifiedDebtItem {
    pub upstream_callers: Vec<String>,           // Total (backwards compat)
    pub upstream_production_callers: Vec<String>, // NEW
    pub upstream_test_callers: Vec<String>,       // NEW
    // ... existing fields
}
```

#### Phase 2: Classification Logic

Create new module `src/priority/caller_classification.rs`:
```rust
/// Pure function to classify a caller as test or production
pub fn classify_caller(
    caller: &str,
    call_graph: Option<&CallGraph>,
) -> CallerType {
    // Try call graph first
    if let Some(cg) = call_graph {
        let func_id = FunctionId::from(caller);
        if cg.is_test_function(&func_id) {
            return CallerType::Test;
        }
    }

    // Fallback to heuristics
    classify_by_heuristics(caller)
}

fn classify_by_heuristics(caller: &str) -> CallerType {
    let test_patterns = [
        "test_", "tests::", "_test", "::test::",
        "should_", "it_", "spec_", "verify_",
    ];

    if test_patterns.iter().any(|p| caller.contains(p)) {
        CallerType::Test
    } else {
        CallerType::Production
    }
}
```

#### Phase 3: Aggregation Updates

Modify `src/priority/god_object_aggregation.rs`:
```rust
pub fn aggregate_dependency_metrics(
    members: &[&UnifiedDebtItem],
    call_graph: Option<&CallGraph>,
) -> DependencyAggregation {
    let mut production_callers: HashSet<String> = HashSet::new();
    let mut test_callers: HashSet<String> = HashSet::new();

    for item in members {
        for caller in &item.upstream_callers {
            match classify_caller(caller, call_graph) {
                CallerType::Production => production_callers.insert(caller.clone()),
                CallerType::Test => test_callers.insert(caller.clone()),
            };
        }
    }

    DependencyAggregation {
        production_callers: production_callers.into_iter().collect(),
        test_callers: test_callers.into_iter().collect(),
        production_upstream_count: production_callers.len(),
        test_upstream_count: test_callers.len(),
        // ...
    }
}
```

#### Phase 4: Scoring Updates

Modify `src/priority/scoring/calculation.rs`:
```rust
/// Calculate dependency factor from PRODUCTION upstream count only
pub fn calculate_dependency_factor(production_upstream_count: usize) -> f64 {
    // Linear scaling with cap at 10.0 for 20+ dependencies
    ((production_upstream_count as f64) / 2.0).min(10.0)
}
```

### Architecture Changes

- New module: `src/priority/caller_classification.rs`
- Modified: `aggregate_dependency_metrics()` signature to accept optional `CallGraph`
- Modified: Scoring pipeline to pass call graph through aggregation

### Data Structures

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerType {
    Production,
    Test,
}

#[derive(Debug, Clone)]
pub struct DependencyAggregation {
    pub production_callers: Vec<String>,
    pub test_callers: Vec<String>,
    pub production_upstream_count: usize,
    pub test_upstream_count: usize,
    pub downstream_callees: Vec<String>,
    pub downstream_count: usize,
}
```

### APIs and Interfaces

New public functions:
- `classify_caller(caller: &str, call_graph: Option<&CallGraph>) -> CallerType`
- `classify_by_heuristics(caller: &str) -> CallerType`

Modified signatures:
- `aggregate_dependency_metrics(members, call_graph)` - adds optional call graph parameter

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/priority/god_object_aggregation.rs`
  - `src/priority/scoring/calculation.rs`
  - `src/output/unified/dependencies.rs`
  - `src/io/writers/llm_markdown.rs`
- **External Dependencies**: None (uses existing call graph infrastructure)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_classify_caller_by_name_patterns() {
    assert_eq!(classify_by_heuristics("test_parse_array"), CallerType::Test);
    assert_eq!(classify_by_heuristics("should_reflow_long_lines"), CallerType::Test);
    assert_eq!(classify_by_heuristics("process_file"), CallerType::Production);
    assert_eq!(classify_by_heuristics("main"), CallerType::Production);
}

#[test]
fn test_aggregate_separates_test_and_production() {
    let members = create_test_members_with_mixed_callers();
    let (prod, test, _, _) = aggregate_dependency_metrics(&members, None);

    assert!(prod.iter().all(|c| !c.contains("test_")));
    assert!(test.iter().all(|c| c.contains("test_")));
}
```

### Integration Tests

Create `tests/test_caller_filtering.rs`:
- Analyze a real file with many test callers
- Verify production blast radius is lower than total blast radius
- Verify scoring uses production count only

### Performance Tests

- Benchmark classification overhead on 10,000 callers
- Ensure <5% performance impact on overall analysis

## Documentation Requirements

- **Code Documentation**: Document `CallerType` enum and classification functions
- **User Documentation**: Update output format docs to explain dual caller counts
- **Architecture Updates**: Document caller classification in ARCHITECTURE.md

## Implementation Notes

### Edge Cases

1. **Lambda/closure callers**: Classify based on containing function
2. **Generic test harness callers**: Add to heuristics list
3. **Mixed callers** (test calling production which calls target): Only direct callers matter

### Backwards Compatibility

- Keep `upstream_callers` and `upstream_count` for backwards compatibility
- New fields are additive
- Old consumers continue working with total counts

## Migration and Compatibility

- No breaking changes to existing API
- New fields added to output formats
- Existing integrations continue working unchanged
- LLM output format enhanced with additional detail

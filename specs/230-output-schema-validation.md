---
number: 230
title: Output Invariant Testing and Schema Validation
category: testing
priority: high
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 230: Output Invariant Testing and Schema Validation

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap generates JSON output (`debtmap.json`) containing technical debt analysis results. Currently, output correctness is only verified through manual inspection. Analysis revealed several data quality issues:

- Floating-point precision noise (`1.5697499999999998`)
- Values potentially outside valid ranges (coverage > 1.0, negative scores)
- Priority/score threshold inconsistencies
- Duplicate entries (addressed separately in spec 231)

### Why Not Postmortem for Output Validation?

Postmortem is designed for **validating untrusted input** where:
- Users need comprehensive error feedback
- All errors should be accumulated
- Data comes from external sources

For **output validation**, the situation differs:
- We control the code producing output
- Validation errors indicate **bugs in our code**
- Fail-fast is appropriate (find and fix the bug)
- We want **invariant assertions**, not schema validation

### Better Approach

1. **Debug assertions** for runtime invariants
2. **Property-based testing** with proptest
3. **Integration tests** validating output structure
4. **Type safety** preventing invalid states

## Objective

Ensure debtmap output correctness through comprehensive invariant testing, property-based testing, and type-system guarantees rather than runtime schema validation.

## Requirements

### Functional Requirements

1. **Add Invariant Assertions**: Use `debug_assert!` for output invariants:
   - Score >= 0
   - Coverage in 0.0..=1.0
   - Confidence in 0.0..=1.0
   - Priority matches score thresholds

2. **Property-Based Testing**: Use proptest to generate random valid structures:
   - All valid `UnifiedOutput` structures serialize correctly
   - All serialized JSON can be deserialized back
   - Round-trip preserves all data

3. **Integration Test Suite**: Validate output against known schema:
   - Output has required fields
   - Values are within expected ranges
   - Cross-field relationships are consistent

4. **Numeric Precision**: Round floating-point values for clean output:
   - Scores: 2 decimal places
   - Coverage/confidence: 4 decimal places
   - Remove floating-point noise

### Non-Functional Requirements

- Tests run in < 5 seconds
- No runtime overhead in release builds (debug_assert!)
- Clear test failure messages

## Acceptance Criteria

- [ ] Debug assertions added for all output invariants
- [ ] Property-based tests for `UnifiedOutput` serialization
- [ ] Integration test validates debtmap.json structure
- [ ] Floating-point values rounded appropriately
- [ ] Test suite runs in CI
- [ ] Documentation of output invariants

## Technical Details

### Implementation Approach

#### 1. Invariant Assertions

```rust
// src/output/unified.rs

impl UnifiedDebtItemOutput {
    /// Assert all invariants hold before serialization
    fn assert_invariants(&self) {
        match self {
            UnifiedDebtItemOutput::Function(f) => {
                debug_assert!(f.score >= 0.0, "Score must be non-negative");
                debug_assert!(f.score <= 1000.0, "Score exceeds maximum");

                if let Some(cov) = f.metrics.coverage {
                    debug_assert!(
                        (0.0..=1.0).contains(&cov),
                        "Coverage {} out of range [0, 1]", cov
                    );
                }

                if let Some(purity) = &f.purity_analysis {
                    debug_assert!(
                        (0.0..=1.0).contains(&purity.confidence),
                        "Confidence {} out of range [0, 1]", purity.confidence
                    );
                }

                // Priority must match score thresholds
                let expected_priority = Priority::from_score(f.score);
                debug_assert_eq!(
                    f.priority, expected_priority,
                    "Priority {:?} doesn't match score {} (expected {:?})",
                    f.priority, f.score, expected_priority
                );
            }
            UnifiedDebtItemOutput::File(f) => {
                debug_assert!(f.score >= 0.0, "Score must be non-negative");
                debug_assert!(
                    (0.0..=1.0).contains(&f.metrics.coverage),
                    "Coverage {} out of range [0, 1]", f.metrics.coverage
                );
            }
        }
    }
}

// Call assertions before serialization
pub fn convert_to_unified_format(...) -> UnifiedOutput {
    // ... existing code ...

    #[cfg(debug_assertions)]
    for item in &unified_items {
        item.assert_invariants();
    }

    UnifiedOutput { ... }
}
```

#### 2. Numeric Precision Rounding

```rust
// src/output/unified.rs

/// Round score to 2 decimal places
fn round_score(score: f64) -> f64 {
    (score * 100.0).round() / 100.0
}

/// Round percentage/ratio to 4 decimal places
fn round_ratio(ratio: f64) -> f64 {
    (ratio * 10000.0).round() / 10000.0
}

impl FunctionDebtItemOutput {
    fn from_function_item(item: &UnifiedDebtItem, include_scoring_details: bool) -> Self {
        let score = round_score(item.unified_score.final_score.value());
        // ...
        FunctionDebtItemOutput {
            score,
            metrics: FunctionMetricsOutput {
                coverage: item.transitive_coverage.as_ref().map(|c| round_ratio(c.transitive)),
                entropy_score: item.entropy_details.as_ref().map(|e| round_ratio(e.entropy_score)),
                // ...
            },
            // ...
        }
    }
}
```

#### 3. Property-Based Testing

```rust
// src/output/unified_tests.rs

use proptest::prelude::*;

prop_compose! {
    fn arb_function_metrics()
        (cyclomatic in 1u32..1000,
         cognitive in 1u32..1000,
         length in 1usize..10000,
         nesting in 0u32..20,
         coverage in prop::option::of(0.0f64..=1.0),
         entropy in prop::option::of(0.0f64..=1.0))
        -> FunctionMetricsOutput
    {
        FunctionMetricsOutput {
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            length,
            nesting_depth: nesting,
            coverage,
            uncovered_lines: None,
            entropy_score: entropy,
        }
    }
}

prop_compose! {
    fn arb_function_debt_item()
        (score in 0.0f64..=500.0,
         metrics in arb_function_metrics(),
         file in "[a-z]+\\.rs")
        -> FunctionDebtItemOutput
    {
        FunctionDebtItemOutput {
            score: round_score(score),
            priority: Priority::from_score(score),
            category: "Testing".to_string(),
            location: UnifiedLocation {
                file,
                line: Some(1),
                function: Some("test".to_string()),
                file_context_label: None,
            },
            metrics,
            // ... other fields with valid defaults
        }
    }
}

proptest! {
    #[test]
    fn test_function_debt_item_serialization_roundtrip(
        item in arb_function_debt_item()
    ) {
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: FunctionDebtItemOutput = serde_json::from_str(&json).unwrap();

        // Verify key fields preserved
        prop_assert_eq!(item.score, deserialized.score);
        prop_assert_eq!(item.priority, deserialized.priority);
    }

    #[test]
    fn test_score_always_non_negative(item in arb_function_debt_item()) {
        prop_assert!(item.score >= 0.0);
    }

    #[test]
    fn test_priority_matches_score_thresholds(item in arb_function_debt_item()) {
        let expected = Priority::from_score(item.score);
        prop_assert_eq!(item.priority, expected);
    }

    #[test]
    fn test_coverage_in_valid_range(item in arb_function_debt_item()) {
        if let Some(cov) = item.metrics.coverage {
            prop_assert!(cov >= 0.0 && cov <= 1.0);
        }
    }
}
```

#### 4. Integration Test

```rust
// tests/output_validation.rs

use serde_json::Value;
use std::fs;

#[test]
fn test_debtmap_output_structure() {
    // Run debtmap on test project
    let output = Command::new("cargo")
        .args(["run", "--", "--json", "test-project/"])
        .output()
        .expect("Failed to run debtmap");

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    // Validate structure
    assert!(json["format_version"].is_string());
    assert!(json["metadata"]["debtmap_version"].is_string());
    assert!(json["summary"]["total_items"].is_number());
    assert!(json["items"].is_array());

    // Validate each item
    for (i, item) in json["items"].as_array().unwrap().iter().enumerate() {
        let score = item["score"].as_f64().unwrap();
        assert!(score >= 0.0, "Item {} has negative score: {}", i, score);

        let priority = item["priority"].as_str().unwrap();
        let expected_priority = match score {
            s if s >= 100.0 => "critical",
            s if s >= 50.0 => "high",
            s if s >= 20.0 => "medium",
            _ => "low",
        };
        assert_eq!(
            priority, expected_priority,
            "Item {} priority mismatch: score={}, priority={}, expected={}",
            i, score, priority, expected_priority
        );

        // Validate coverage if present
        if let Some(cov) = item["metrics"]["coverage"].as_f64() {
            assert!(
                cov >= 0.0 && cov <= 1.0,
                "Item {} coverage out of range: {}", i, cov
            );
        }
    }
}

#[test]
fn test_no_floating_point_noise() {
    let output_path = "debtmap.json";
    let content = fs::read_to_string(output_path).unwrap();

    // Check for typical floating-point noise patterns
    let noise_patterns = [
        "9999999999",  // e.g., 1.9999999999
        "0000000001",  // e.g., 1.0000000001
    ];

    for pattern in noise_patterns {
        assert!(
            !content.contains(pattern),
            "Found floating-point noise: {}",
            pattern
        );
    }
}
```

### Architecture Changes

- New file: `src/output/invariants.rs` (invariant assertions)
- New file: `tests/output_validation.rs` (integration tests)
- New file: `src/output/proptest.rs` (property-based test helpers)
- Modified: `src/output/unified.rs` (rounding, assertions)

## Dependencies

- **Prerequisites**: None
- **Dev Dependencies**: `proptest` (already in workspace)
- **Affected Components**: `src/output/unified.rs`

## Testing Strategy

- **Unit Tests**: Invariant assertions catch invalid states in debug builds
- **Property Tests**: Proptest validates output structure invariants
- **Integration Tests**: Full output validation against schema
- **CI Integration**: All tests run on every PR

## Documentation Requirements

- **Code Documentation**: Document output invariants in rustdoc
- **User Documentation**: Document output format guarantees
- **Architecture Updates**: Document testing strategy

## Implementation Notes

1. **Debug vs Release**: `debug_assert!` has zero cost in release builds. Use for invariants that should never fail in correct code.

2. **Test Coverage**: Property tests provide broader coverage than example-based tests.

3. **Rounding Strategy**: Round at serialization time, not during computation, to preserve internal precision.

## Migration and Compatibility

- **No breaking changes**: Output format unchanged
- **Cleaner output**: Floating-point noise removed
- **Better error detection**: Invariant violations caught in tests

# External API Detection Testing Strategy

## Overview
The external API detection feature uses static analysis heuristics to determine if public functions without callers are likely part of an external API (and thus shouldn't be automatically removed).

## Test Coverage

### 1. Unit Tests (`src/priority/external_api_detector.rs`)
Located in the module itself, these tests verify core detection logic:

- **test_lib_rs_detection**: Functions in `lib.rs` are flagged as APIs
- **test_constructor_pattern**: Common constructors (`new`, `from_*`) are detected
- **test_deep_internal_module**: Deep internal paths are NOT flagged as APIs
- **test_api_pattern_names**: API naming patterns (`get_*`, `set_*`) are recognized
- **test_private_function**: Private functions are never APIs

### 2. Comprehensive Test Suite (`tests/external_api_detection_tests.rs`)
Extensive test coverage with 13 test cases:

#### Detection Logic Tests
- **test_lib_rs_functions_are_apis**: Verifies lib.rs functions score high
- **test_mod_rs_functions_are_likely_apis**: Module root functions get points
- **test_common_constructors_are_apis**: Standard Rust patterns recognized
- **test_api_pattern_prefixes**: Common API prefixes detected
- **test_specific_api_prefixes**: `public_`, `api_`, `export_` prefixes

#### Exclusion Tests
- **test_internal_paths_not_apis**: `/internal/`, `/private/` paths excluded
- **test_deep_vs_shallow_paths**: Path depth affects scoring
- **test_non_public_functions_never_apis**: Visibility enforcement

#### Feature Tests
- **test_enhanced_hints_generation**: Hints include API indicators
- **test_test_helper_detection**: Test helpers identified separately
- **test_complexity_impact_hints**: Complexity analysis integrated
- **test_score_threshold_for_api_detection**: Scoring threshold validation
- **test_real_world_patterns**: Patterns from serde, tokio, etc.

### 3. Integration Tests (`tests/external_api_detector_integration_test.rs`)
End-to-end workflow tests with 6 scenarios:

- **test_workflow_public_api_in_lib_rs**: Full workflow for lib.rs APIs
- **test_workflow_internal_public_function**: Internal functions handled correctly
- **test_workflow_private_function**: Private functions never flagged
- **test_workflow_mod_rs_with_api_pattern**: Combined indicators work
- **test_action_recommendations_based_on_api_detection**: Actions differ by detection
- **test_complexity_hints_integration**: Complexity and API detection combine

## Heuristic Scoring System

The detection uses a confidence scoring system (threshold: 4 points):

### High Value Indicators (3 points)
- Function in `lib.rs`
- Constructor pattern (`new`, `default`)

### Medium Value Indicators (2 points)
- Function in `mod.rs`
- In public module hierarchy
- API naming pattern (`get_*`, `set_*`, etc.)
- Builder/factory pattern
- Common trait methods
- Public API prefixes

### Low Value Indicators (1 point)
- Shallow module path (≤3 levels)

### Negative Indicators (-1 point)
- Deep module path (>5 levels)

## Test Execution

Run all external API tests:
```bash
# Unit tests only
cargo test external_api_detector::tests

# Comprehensive test suite
cargo test --test external_api_detection_tests

# Integration tests
cargo test --test external_api_detector_integration_test

# All external API tests
cargo test external_api
```

## Validation in Real Usage

The enhanced detection produces output like:
```
├─ ACTION: Verify external usage before removal or deprecation
├─ VISIBILITY: public function with no callers
│  • ⚠️ Likely external API - verify before removing
│  •   • Defined in lib.rs (library root)
│  •   • Constructor/initialization pattern
│  •   • Shallow module path (likely public)
└─ WHY: Public function 'new' appears to be external API - verify usage before action
```

vs. non-API public functions:
```
├─ ACTION: Remove unused public function (no API indicators)
├─ VISIBILITY: public function with no callers
│  • Public but no external API indicators found
└─ WHY: Public function 'helper' has no callers and no external API indicators
```

## Coverage Metrics

- **Core Logic**: 100% of detection functions tested
- **Edge Cases**: Deep paths, internal modules, visibility combinations
- **Integration**: Full workflow from detection to output formatting
- **Real-world Patterns**: Based on popular Rust crates

## Future Enhancements

Potential improvements to consider:
1. Check if function is re-exported in parent modules
2. Analyze doc comments for API documentation patterns
3. Check for `#[doc(hidden)]` or other attributes
4. Integration with cargo doc to see what's publicly documented
5. Machine learning model trained on popular crates
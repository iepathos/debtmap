---
number: 233
title: Improve Output Data Quality
category: optimization
priority: medium
status: draft
dependencies: [230, 231, 232]
created: 2025-12-13
---

# Specification 233: Improve Output Data Quality

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: [230, 231, 232]

## Context

Beyond the specific bugs addressed in specs 230-232, the debtmap output has several data quality issues that reduce user trust and utility:

### Issues Identified

1. **Function field contains filenames**: Many entries show `"function": "filename.rs"` instead of actual function names. This occurs for file-level "god object" analysis incorrectly reported as Function type items.

2. **High "Unknown" function role rate**: ~50% of items have `function_role: "Unknown"`, reducing the value of role-based filtering/prioritization.

3. **Generic recommendations**: Many items get identical recommendations like "Split into 5 modules by responsibility" regardless of specific context.

4. **Uniform confidence values**: Many entries show exactly `confidence: 0.95`, suggesting default values rather than calculated confidence.

### Evidence

```json
// Function field contains filename
{
  "type": "Function",
  "location": {
    "file": "./src/builders/parallel_unified_analysis.rs",
    "line": 1,
    "function": "parallel_unified_analysis.rs"  // Should be a function name!
  }
}

// Generic recommendation
{
  "recommendation": {
    "action": "Split into 5 modules by responsibility"  // Same for many items
  }
}
```

## Objective

Improve debtmap output quality by:
1. Correctly distinguishing file-level and function-level debt items
2. Improving function role classification
3. Generating context-specific recommendations
4. Calculating actual confidence values

## Requirements

### Functional Requirements

1. **Fix File vs Function Classification**:
   - File-level debt must use `"type": "File"`, not `"type": "Function"`
   - `"function"` field must contain actual function names, not filenames
   - Add validation to catch this issue

2. **Improve Function Role Classification**:
   - Reduce "Unknown" classification rate to < 20%
   - Add heuristics for common patterns (test functions, builders, handlers)
   - Document classification criteria

3. **Context-Specific Recommendations**:
   - Use debt item specifics to generate unique recommendations
   - Include relevant metrics in recommendations
   - Avoid generic "Split into N modules" advice

4. **Calculate Real Confidence**:
   - Replace default 0.95 confidence with calculated values
   - Base confidence on evidence strength (call graph coverage, pattern match quality)
   - Document confidence calculation methodology

### Non-Functional Requirements

- No performance regression
- Maintain output format compatibility
- Improve data quality metrics measurably

## Acceptance Criteria

- [ ] No `"type": "Function"` items with filename in `"function"` field
- [ ] `function_role: "Unknown"` rate < 20% (down from ~50%)
- [ ] < 10% of recommendations are identical across items
- [ ] Confidence values show meaningful variation (not all 0.95)
- [ ] Tests verify classification correctness
- [ ] Documentation updated for classification criteria

## Technical Details

### 1. Fix File vs Function Classification

The issue: File-level god object analysis is being emitted as Function items.

**Root Cause Investigation**:
```bash
rg '"type": "Function"' -B5 -A10 | grep -A5 '"function".*\.rs"'
```

**Fix in debt item creation**:

```rust
// src/priority/mod.rs or wherever debt items are created

fn create_debt_item(metrics: &FileMetrics, location: &Location) -> DebtItem {
    // Check if this is file-level debt (god object analysis)
    if is_file_level_debt(metrics) {
        return DebtItem::File(FileDebtItem {
            metrics: FileDebtMetrics::from(metrics),
            // ...
        });
    }

    // Function-level debt
    DebtItem::Function(UnifiedDebtItem {
        location: Location {
            file: location.file.clone(),
            line: location.line,
            function: location.function.clone(),  // Must be actual function name!
        },
        // ...
    })
}

/// Detect if debt is file-level (god object) vs function-level
fn is_file_level_debt(metrics: &FileMetrics) -> bool {
    // God object analysis typically has:
    // - Multiple methods/responsibilities
    // - No specific function identified
    // - Line 1 (file-level)
    metrics.god_object_analysis.is_some()
        && metrics.line == Some(1)
}
```

**Validation**:
```rust
impl UnifiedDebtItemOutput {
    fn validate(&self) -> Result<(), ValidationError> {
        if let UnifiedDebtItemOutput::Function(f) = self {
            // Function name should not be a filename
            if f.location.function.as_ref().map_or(false, |name| name.ends_with(".rs")) {
                return Err(ValidationError::InvalidFunctionName(
                    f.location.function.clone()
                ));
            }
        }
        Ok(())
    }
}
```

### 2. Improve Function Role Classification

Current roles: `Unknown`, `PureLogic`, `EntryPoint`, `IOBoundary`, `TestHelper`, etc.

**Enhanced Classification Heuristics**:

```rust
// src/priority/semantic_classifier/mod.rs

pub fn classify_function_role(
    name: &str,
    attributes: &[Attribute],
    body_analysis: &FunctionBodyAnalysis,
    call_graph: &CallGraph,
) -> FunctionRole {
    // 1. Test functions (high confidence)
    if has_test_attribute(attributes) || name.starts_with("test_") {
        return FunctionRole::TestHelper;
    }

    // 2. Entry points (main, handlers)
    if is_entry_point(name, attributes) {
        return FunctionRole::EntryPoint;
    }

    // 3. Builder pattern
    if name.starts_with("with_") || name.starts_with("build") || name == "new" {
        return FunctionRole::Builder;
    }

    // 4. Pure logic (no I/O, no mutations)
    if body_analysis.is_pure && body_analysis.no_io_calls {
        return FunctionRole::PureLogic;
    }

    // 5. I/O boundary (file, network, database calls)
    if body_analysis.has_io_calls {
        return FunctionRole::IOBoundary;
    }

    // 6. Transformer (takes input, returns transformed output)
    if is_transformer_pattern(name, body_analysis) {
        return FunctionRole::Transformer;
    }

    // 7. Validator (returns bool or Result for validation)
    if is_validator_pattern(name, body_analysis) {
        return FunctionRole::Validator;
    }

    // 8. Event handler patterns
    if name.starts_with("on_") || name.starts_with("handle_") {
        return FunctionRole::EventHandler;
    }

    // Default: unknown only when no signals detected
    FunctionRole::Unknown
}

fn is_transformer_pattern(name: &str, body: &FunctionBodyAnalysis) -> bool {
    let transformer_prefixes = ["to_", "into_", "from_", "convert_", "parse_", "format_"];
    transformer_prefixes.iter().any(|p| name.starts_with(p))
        || (body.takes_self_ref && body.returns_value)
}

fn is_validator_pattern(name: &str, body: &FunctionBodyAnalysis) -> bool {
    let validator_prefixes = ["is_", "has_", "can_", "should_", "validate_", "check_"];
    validator_prefixes.iter().any(|p| name.starts_with(p))
        && (body.returns_bool || body.returns_result)
}
```

### 3. Context-Specific Recommendations

**Replace generic recommendations with specific ones**:

```rust
// src/priority/scoring/concise_recommendation.rs

pub fn generate_recommendation(item: &UnifiedDebtItem) -> RecommendationOutput {
    let mut action = String::new();
    let mut steps = Vec::new();

    match &item.debt_type {
        DebtType::GodObject { methods, responsibilities, .. } => {
            // Specific to the god object
            action = format!(
                "Split {} methods across {} focused modules",
                methods, responsibilities
            );
            steps.push(format!("Extract {} responsibility into dedicated module",
                item.top_responsibility().unwrap_or("core")));
            steps.push("Define clear interfaces between modules".to_string());
            steps.push("Move related tests to new module locations".to_string());
        }
        DebtType::ComplexityHotspot { cyclomatic, cognitive } => {
            if *cyclomatic > 20 {
                action = format!(
                    "Reduce cyclomatic complexity from {} to <10 by extracting conditionals",
                    cyclomatic
                );
                steps.push("Identify independent condition branches".to_string());
                steps.push("Extract each branch to separate function".to_string());
            } else if *cognitive > 15 {
                action = format!(
                    "Reduce cognitive complexity from {} by flattening nested logic",
                    cognitive
                );
                steps.push("Use early returns to reduce nesting".to_string());
                steps.push("Extract nested loops into helper functions".to_string());
            }
        }
        DebtType::TestingGap { coverage, cyclomatic, .. } => {
            let tests_needed = estimate_tests_needed(*cyclomatic, *coverage);
            action = format!(
                "Add {} tests to cover {:.0}% of {} decision points",
                tests_needed, (1.0 - coverage) * 100.0, cyclomatic
            );
            steps.push(format!("Test {} happy path scenarios", tests_needed / 2));
            steps.push(format!("Test {} error cases", tests_needed / 2));
        }
        _ => {
            // Fallback with metrics context
            action = format!(
                "Refactor function (complexity: {}, length: {} lines)",
                item.cyclomatic_complexity, item.function_length
            );
        }
    }

    RecommendationOutput {
        action,
        priority: None,
        implementation_steps: steps,
    }
}

fn estimate_tests_needed(cyclomatic: u32, coverage: f64) -> u32 {
    let uncovered_branches = (cyclomatic as f64 * (1.0 - coverage)).ceil() as u32;
    uncovered_branches.max(1)
}
```

### 4. Calculate Real Confidence

**Replace default 0.95 with evidence-based confidence**:

```rust
// src/analyzers/purity_detector.rs

pub fn calculate_purity_confidence(analysis: &PurityAnalysis) -> f32 {
    let mut confidence = 0.5;  // Base confidence

    // Increase confidence based on evidence
    if analysis.call_graph_complete {
        confidence += 0.2;  // Full call graph available
    }
    if analysis.all_callees_analyzed {
        confidence += 0.15;  // All called functions analyzed
    }
    if !analysis.has_unknown_calls {
        confidence += 0.1;  // No opaque function calls
    }
    if analysis.closure_analysis_complete {
        confidence += 0.05;  // All closures analyzed
    }

    // Decrease confidence for uncertainty
    if analysis.has_unsafe_blocks {
        confidence -= 0.1;  // Unsafe code is harder to analyze
    }
    if analysis.has_ffi_calls {
        confidence -= 0.15;  // FFI calls are opaque
    }
    if analysis.has_dynamic_dispatch {
        confidence -= 0.1;  // vtable calls are unpredictable
    }

    confidence.clamp(0.1, 1.0)
}
```

**Document confidence calculation**:
```rust
/// Purity confidence is calculated based on:
/// - Base: 0.5
/// - +0.2: Complete call graph
/// - +0.15: All callees analyzed
/// - +0.1: No unknown calls
/// - +0.05: Closures analyzed
/// - -0.1: Unsafe blocks present
/// - -0.15: FFI calls present
/// - -0.1: Dynamic dispatch present
/// Range: 0.1 to 1.0
```

### Architecture Changes

- Modified: `src/priority/mod.rs` (file vs function classification)
- Modified: `src/priority/semantic_classifier/mod.rs` (role classification)
- Modified: `src/priority/scoring/concise_recommendation.rs` (recommendations)
- Modified: `src/analyzers/purity_detector.rs` (confidence calculation)
- New: `src/output/validation.rs` (output validation)

## Dependencies

- **Prerequisites**:
  - Spec 230: Output Invariant Testing (for validation)
  - Spec 231: Fix Duplicate Debt Items
  - Spec 232: Fix Dampened Complexity Calculation
- **Affected Components**: Multiple modules across priority, analyzers, output

## Testing Strategy

- **Unit Tests**:
  - Test file vs function classification
  - Test role classification heuristics
  - Test recommendation generation
  - Test confidence calculation

- **Integration Tests**:
  - Verify no Function items with filename in function field
  - Verify Unknown role rate < 20%
  - Verify recommendation diversity

- **Metrics Collection**:
  - Before/after comparison of Unknown rate
  - Before/after comparison of recommendation uniqueness
  - Confidence value distribution analysis

```rust
#[test]
fn test_function_field_not_filename() {
    let output = run_debtmap_on_test_project();

    for item in output.items {
        if let UnifiedDebtItemOutput::Function(f) = item {
            assert!(
                !f.location.function.as_ref().map_or(false, |n| n.ends_with(".rs")),
                "Function field should not be a filename: {:?}",
                f.location.function
            );
        }
    }
}

#[test]
fn test_unknown_role_rate_below_threshold() {
    let output = run_debtmap_on_test_project();

    let function_items: Vec<_> = output.items.iter()
        .filter_map(|i| match i {
            UnifiedDebtItemOutput::Function(f) => Some(f),
            _ => None,
        })
        .collect();

    let unknown_count = function_items.iter()
        .filter(|f| f.function_role == FunctionRole::Unknown)
        .count();

    let unknown_rate = unknown_count as f64 / function_items.len() as f64;
    assert!(
        unknown_rate < 0.20,
        "Unknown role rate {} exceeds 20% threshold",
        unknown_rate
    );
}

#[test]
fn test_recommendation_diversity() {
    let output = run_debtmap_on_test_project();

    let recommendations: Vec<_> = output.items.iter()
        .map(|i| match i {
            UnifiedDebtItemOutput::File(f) => f.recommendation.action.clone(),
            UnifiedDebtItemOutput::Function(f) => f.recommendation.action.clone(),
        })
        .collect();

    let unique_count = recommendations.iter().collect::<HashSet<_>>().len();
    let uniqueness_rate = unique_count as f64 / recommendations.len() as f64;

    assert!(
        uniqueness_rate > 0.5,
        "Recommendation uniqueness {} below 50% threshold (too many identical)",
        uniqueness_rate
    );
}
```

## Documentation Requirements

- **Code Documentation**: Document classification criteria and confidence calculation
- **User Documentation**: Explain function roles and their meaning
- **Architecture Updates**: Document recommendation generation algorithm

## Implementation Notes

1. **Incremental Improvement**: These are quality improvements, not bug fixes. Implement incrementally.

2. **Metrics Tracking**: Add telemetry to track quality metrics over time.

3. **User Feedback**: Consider adding feedback mechanism for incorrect classifications.

4. **Balance**: Don't over-classify - "Unknown" is better than wrong classification.

## Migration and Compatibility

- **Output format unchanged**: Same JSON structure
- **Values will improve**: More accurate roles, diverse recommendations
- **No breaking changes**: Consumers may see different values but format is stable

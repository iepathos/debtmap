# God Object Detection

## Overview

DebtMap includes sophisticated god object detection that identifies files and types that have grown too large and taken on too many responsibilities. God objects are a significant source of technical debt as they:

- Violate the Single Responsibility Principle
- Become difficult to maintain and test
- Create bottlenecks in development
- Increase the risk of bugs due to high coupling

## Detection Criteria

A file or type is classified as a god object based on multiple thresholds:

### Language-Specific Thresholds

#### Rust
- **Max Methods**: 20 (includes both impl methods and standalone functions)
- **Max Fields**: 15
- **Max Responsibilities**: 5
- **Max Lines**: 1000
- **Max Complexity**: 200

#### Python
- **Max Methods**: 15
- **Max Fields**: 10
- **Max Responsibilities**: 3
- **Max Lines**: 500
- **Max Complexity**: 150

#### JavaScript/TypeScript
- **Max Methods**: 15
- **Max Fields**: 20
- **Max Responsibilities**: 3
- **Max Lines**: 500
- **Max Complexity**: 150

### Confidence Levels

The detector assigns confidence levels based on how many thresholds are exceeded:

- **Definite**: 5 violations - Clear god object requiring immediate refactoring
- **Probable**: 3-4 violations - Likely god object that should be refactored
- **Possible**: 1-2 violations - Potential god object worth reviewing
- **NotGodObject**: 0 violations - Within acceptable limits

## Scoring System

### Base Score Calculation

The god object score is calculated using multiple factors:

1. **Method Factor**: `min(method_count / threshold, 3.0)`
2. **Field Factor**: `min(field_count / threshold, 3.0)`
3. **Responsibility Factor**: `min(responsibility_count / 3, 3.0)`
4. **Size Factor**: `min(lines_of_code / threshold, 3.0)`

### Score Enforcement

- **Minimum Score**: Any file/type exceeding at least one threshold receives a minimum score of 100 points
- **Severity Scaling**: Score increases with the number of violations
- **Formula**: `max(base_score * 50 * violation_count, 100)` for god objects

This ensures that god objects are properly prioritized in the technical debt analysis.

## Responsibility Detection

Responsibilities are inferred from method names using common prefixes:

- **Data Access**: get, set
- **Computation**: calculate, compute
- **Validation**: validate, check, verify, ensure
- **Persistence**: save, load, store, retrieve, fetch
- **Construction**: create, build, new, make, init
- **Communication**: send, receive, handle, manage
- **Modification**: update, modify, change, edit
- **Deletion**: delete, remove, clear, reset
- **State Query**: is, has, can, should, will
- **Processing**: process, transform

## Detection Examples

### Example 1: Large Rust Module

A file like `rust_call_graph.rs` with 270 standalone functions would be detected as:
- **Is God Object**: Yes
- **Method Count**: 270
- **Confidence**: Definite
- **Score**: >1000 (severe violation)
- **Recommendation**: Break into multiple focused modules

### Example 2: Complex Class

A Python class with 25 methods and 12 fields would be detected as:
- **Is God Object**: Yes
- **Method Count**: 25
- **Field Count**: 12
- **Confidence**: Probable
- **Score**: ~150-200
- **Recommendation**: Split by responsibility groups

## Output Display

God object indicators appear in the analysis output:

### File-Level Display
```
⚠️ God Object: 270 methods, 0 fields, 8 responsibilities
Score: 1350 (Confidence: Definite)
```

### Function-Level Display
When analyzing individual functions in a god object file:
```
├─ ⚠️ God Object: 45 methods, 20 fields, 5 responsibilities
│      Score: 250 (Confidence: Probable)
```

## Refactoring Recommendations

When a god object is detected, DebtMap provides:

1. **Suggested Module Splits**: Based on responsibility groups
2. **Method Groupings**: Methods grouped by their inferred responsibilities
3. **Priority Ordering**: Most cohesive splits recommended first
4. **Size Estimates**: Approximate lines of code for each split module

## Integration with Scoring

God objects impact the overall technical debt score through:

1. **File Metrics Multiplier**: `2.0 + god_object_score` (normalized to 0-1 range)
2. **Priority Boost**: God objects receive higher priority in debt rankings
3. **Cascading Impact**: Functions within god objects inherit elevated scores

## Best Practices

To avoid god objects:

1. **Follow Single Responsibility Principle**: Each module should have one clear purpose
2. **Regular Refactoring**: Split modules before they reach thresholds
3. **Monitor Growth**: Track method and field counts as modules evolve
4. **Use Composition**: Prefer smaller, composable units over large monoliths
5. **Clear Boundaries**: Define clear module interfaces and responsibilities
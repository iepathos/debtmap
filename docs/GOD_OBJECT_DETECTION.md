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

### Confidence-Based Classification (Spec 174)

DebtMap uses a confidence-based approach to classify method responsibilities, replacing the previous unconditional "utilities" fallback with evidence-based classification. This significantly improves the quality of refactoring recommendations.

#### Classification Strategy

Method responsibilities are inferred using multiple signals:

1. **Name Heuristics**: Pattern matching on method names (e.g., `save_*`, `validate_*`)
2. **I/O Detection**: Analyzing actual I/O operations in method bodies
3. **Behavioral Analysis**: Categorizing methods by their behavior patterns

Each classification receives a confidence score (0.0 to 1.0) based on signal strength.

#### Confidence Thresholds

- **Minimum Confidence**: 0.50 (50%)
  - Classifications below this threshold are rejected
  - Methods remain in their original location instead of being extracted

- **Utilities Threshold**: 0.60 (60%)
  - Higher bar for "utilities" classification to prevent over-classification
  - Reduces utilities classification rate from ~30% to <10%

- **Module Split Confidence**: 0.65 (65%)
  - Required for recommending structural module splits
  - Ensures high-confidence evidence before suggesting refactoring

#### Classification Results

When a method is classified, the result includes:

- **Category**: The responsibility category (or `None` if confidence too low)
- **Confidence Score**: Numeric confidence (0.0-1.0)
- **Signals Used**: Which signals contributed to the classification

Example:
```rust
ClassificationResult {
    category: Some("data_persistence"),
    confidence: 0.85,
    signals_used: vec![SignalType::NameHeuristic, SignalType::IoDetection]
}
```

#### Low Confidence Handling

Methods with low confidence are handled conservatively:

- Returned with `category: None`
- Kept in original location (not extracted)
- Logged at DEBUG level for analysis and tuning
- Tracked in classification metrics for monitoring

#### Observability

Classification metrics are tracked and emitted:

- **Total methods**: Number of methods attempted to classify
- **Classified methods**: Successfully classified above threshold
- **Unclassified methods**: Rejected due to low confidence
- **Utilities count**: Methods classified as utilities
- **Utilities rate**: Should be <10% (warning if higher)

Example metrics output:
```
INFO: Classification metrics: total=100, classified=72, unclassified=28, utilities=8 (8.0%)
```

### Responsibility Categories

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

### Module Split Requirements

Module splits are only recommended when:

1. **Sufficient Methods**: At least 5 methods in the responsibility group
2. **High Confidence**: Average confidence ≥ 0.65 across all methods
3. **Clear Responsibility**: Well-defined category (not "unclassified")

This prevents premature or low-confidence refactoring recommendations.

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

## Type-Based Clustering (Spec 181)

### Overview

Type-based clustering is an advanced refactoring recommendation strategy that groups functions and methods by their type affinity rather than behavioral patterns. This approach is particularly effective for:

1. **Data-centric modules**: Files with many parameter-heavy functions
2. **Utilities modules**: When behavioral clustering produces generic "helpers" or "utilities" modules
3. **Type ownership patterns**: Encouraging idiomatic Rust design with clear type ownership

### When Type-Based Clustering is Used

DebtMap automatically applies type-based clustering when:

- **Behavioral clustering produces utilities modules**: When behavioral analysis generates modules named "utilities", "helpers", or "utils"
- **Parameter-heavy functions**: Files with many functions that work with specific types
- **Empty behavioral results**: When behavioral clustering produces no actionable splits

### Type Affinity Analysis

Type-based clustering analyzes:

1. **Input Types**: Parameter types for each function/method
2. **Output Types**: Return types for each function/method
3. **Primary Type**: The type that appears most frequently in parameters
4. **Type Clusters**: Groups of functions working with the same types

Functions are clustered together when they share:
- The same primary input type
- Similar type signatures
- Related data transformations

### Example Output

When type-based clustering identifies a refactoring opportunity, it provides:

```
Recommended Module Split: priority_item
  Core Type: PriorityItem
  Methods: create_priority_item, update_priority, compare_items, format_item
  Data Flow: String -> PriorityItem -> String

  Suggested Type Definition:
  pub struct PriorityItem {
      priority: u32,
      name: String,
  }

  impl PriorityItem {
      pub fn new(name: String, priority: u32) -> Self {
          PriorityItem { name, priority }
      }

      pub fn update_priority(&mut self, new_priority: u32) {
          self.priority = new_priority;
      }

      // ... additional methods
  }
```

### Type Ownership Principles

Type-based recommendations follow idiomatic Rust patterns:

1. **Single Type Ownership**: Each module owns one primary type
2. **Method Organization**: Methods grouped by the type they operate on
3. **Clear Boundaries**: Explicit input/output types for each module
4. **Data Transformation**: Pure functions that transform data

### Behavioral vs Type-Based Clustering

| Aspect | Behavioral Clustering | Type-Based Clustering |
|--------|----------------------|----------------------|
| **Focus** | Method call patterns and naming | Type signatures and data flow |
| **Best For** | Method-heavy god objects | Data-centric utilities files |
| **Output** | Behavioral groups (I/O, Query, etc.) | Type-centric modules |
| **Rust Idioms** | Method categorization | Type ownership patterns |
| **When Used** | Primary strategy for 50+ methods | Fallback when behavioral produces utilities |

### Quality Criteria

Type-based splits are preferred when:

1. They avoid generic module names (no "utilities" or "helpers")
2. Each cluster has ≥3 methods working with the same type
3. Clear type affinity score (measuring how strongly methods relate to the type)
4. Well-defined data flow with input/output types

### Integration with Pipeline

Type-based clustering integrates with the god object detection pipeline:

1. **Priority 1**: Behavioral clustering for method-heavy files (50+ methods, 500+ LOC)
2. **Fallback**: If behavioral produces utilities modules → try type-based
3. **Quality Check**: Use type-based if it produces better-named modules
4. **Priority 2**: Cross-domain analysis for struct-heavy files
5. **Priority 3**: Small god objects with behavioral/type-based fallback

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
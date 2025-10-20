# God Object Detection

## Overview

Debtmap includes sophisticated god object detection that identifies files and types that have grown too large and taken on too many responsibilities. God objects are a significant source of technical debt as they:

- Violate the Single Responsibility Principle
- Become difficult to maintain and test
- Create bottlenecks in development
- Increase the risk of bugs due to high coupling

This chapter explains how Debtmap identifies god objects, calculates their scores, and provides actionable refactoring recommendations.

## Detection Criteria

A file or type is classified as a god object based on five key metrics:

1. **Method Count** - Total number of methods/functions
2. **Field Count** - Number of struct/class fields
3. **Responsibility Count** - Distinct responsibilities inferred from method names (max_traits in config)
4. **Lines of Code** - Total lines in the file
5. **Complexity Sum** - Combined cyclomatic complexity of all functions

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

These thresholds can be customized per-language in your `.debtmap.toml` configuration file.

## File-Level Aggregation

An important feature of Debtmap's god object detection is its **file-level aggregation strategy**. When analyzing a file, Debtmap:

1. Finds the largest type (struct/class) by `method_count + field_count × 2`
2. Counts standalone functions in the file
3. Combines them: `total_methods = type_methods + standalone_functions`

This means files with many standalone functions (like `rust_call_graph.rs` with 270 functions) will be detected as god objects even without a large type. This is crucial for identifying:

- Pure functional modules with excessive functions
- Utility files that have grown too large
- Mixed paradigm files (structs + many helper functions)

**Example:** A file containing a struct with 15 methods plus 10 standalone functions will be analyzed as having 25 total methods, likely triggering god object detection.

See `src/organization/god_object_detector.rs:66-97` for implementation details.

## Confidence Levels

Debtmap assigns confidence levels based on how many thresholds are violated:

- **Definite** (5 violations) - Clear god object requiring immediate refactoring
- **Probable** (3-4 violations) - Likely god object that should be refactored
- **Possible** (1-2 violations) - Potential god object worth reviewing
- **NotGodObject** (0 violations) - Within acceptable limits

The final determination also requires `god_object_score >= 70.0`. Both criteria must be met for a definite god object classification.

See `src/organization/god_object_analysis.rs:229-270` and `src/organization/god_object_detector.rs:152-163`.

## Scoring Algorithms

Debtmap provides three scoring algorithms to accommodate different analysis needs.

### Simple Scoring

The base scoring algorithm calculates god object score using four factors:

```
method_factor = min(method_count / max_methods, 3.0)
field_factor = min(field_count / max_fields, 3.0)
responsibility_factor = min(responsibility_count / 3, 3.0)
size_factor = min(lines_of_code / max_lines, 3.0)

base_score = method_factor × field_factor × responsibility_factor × size_factor
```

**Score Enforcement:**

- If `violation_count > 0`: `final_score = max(base_score × 50 × violation_count, 100)`
- Else: `final_score = base_score × 10`

The minimum score of 100 ensures that any god object receives sufficient priority in the technical debt analysis.

### Complexity-Weighted Scoring

Unlike raw method counting, this algorithm weights each method by its cyclomatic complexity. This ensures that 100 simple functions (complexity 1-3) score better than 10 highly complex functions (complexity 17+).

The formula is similar to simple scoring, but uses `weighted_method_count` (sum of complexity weights) instead of raw counts:

```
method_factor = min(weighted_method_count / max_methods, 3.0)
```

Additionally, a **complexity factor** is applied:

- Average complexity < 3.0: `0.7` (reward simple functions)
- Average complexity > 10.0: `1.5` (penalize complex functions)
- Otherwise: `1.0`

The final score becomes:

```
final_score = max(base_score × 50 × complexity_factor × violation_count, 100)
```

This approach better reflects the true maintainability burden of a large module.

See `src/organization/god_object_analysis.rs:142-209`.

### Purity-Weighted Scoring (Advanced)

**Available for Rust only** (requires `syn::ItemFn` analysis)

This advanced scoring variant reduces the impact of pure functions, preventing pure functional modules from being unfairly penalized. The algorithm:

1. Analyzes each function for purity using three levels:
   - **Pure** (no side effects): weight multiplier `0.3`
   - **Probably Pure** (likely no side effects): weight multiplier `0.5`
   - **Impure** (has side effects): weight multiplier `1.0`

2. Combines complexity and purity weights:
   ```
   total_weight = complexity_weight × purity_multiplier
   ```

3. Tracks the `PurityDistribution`:
   - `pure_count`, `probably_pure_count`, `impure_count`
   - `pure_weight_contribution`, `probably_pure_weight_contribution`, `impure_weight_contribution`

This approach dramatically reduces scores for files with many pure helper functions while still flagging stateful god objects.

See `src/organization/god_object_detector.rs:196-258` and `src/organization/purity_analyzer.rs`.

## Responsibility Detection

Responsibilities are inferred from method names using common prefixes. Debtmap recognizes 28 standard prefixes grouped into 10 categories:

| Prefix(es) | Responsibility Category |
|------------|------------------------|
| `get`, `set` | Data Access |
| `calculate`, `compute` | Computation |
| `validate`, `check`, `verify`, `ensure` | Validation |
| `save`, `load`, `store`, `retrieve`, `fetch` | Persistence |
| `create`, `build`, `new`, `make`, `init` | Construction |
| `send`, `receive`, `handle`, `manage` | Communication |
| `update`, `modify`, `change`, `edit` | Modification |
| `delete`, `remove`, `clear`, `reset` | Deletion |
| `is`, `has`, `can`, `should`, `will` | State Query |
| `process`, `transform` | Processing |

**Fallback:** If a prefix doesn't match any category, Debtmap creates a default responsibility: `"{Prefix} Operations"` (with capitalized first letter).

Responsibility count directly affects:
- God object scoring (via `responsibility_factor`)
- Refactoring recommendations (methods grouped by responsibility)

See `src/organization/god_object_detector.rs:378-454`.

## Refactoring Recommendations

When `is_god_object = true`, Debtmap generates **recommended module splits** using the `recommend_module_splits` function. This feature:

1. Groups methods by their inferred responsibilities
2. Creates a `ModuleSplit` for each responsibility group containing:
   - `suggested_name` (e.g., "DataAccessManager", "ValidationManager")
   - `methods_to_move` (list of method names)
   - `responsibility` (category name)
   - `estimated_lines` (approximate LOC for the new module)

3. Orders splits by cohesion (most focused responsibility groups first)

**Example output:**
```
Recommended Splits:
  1. DataAccessManager (12 methods, ~150 lines)
  2. ValidationManager (8 methods, ~100 lines)
  3. PersistenceManager (5 methods, ~75 lines)
```

This provides an actionable roadmap for breaking down god objects into focused, single-responsibility modules.

See `src/organization/god_object_detector.rs:165-177` and `src/organization/god_object_analysis.rs:40-45`.

## Configuration

### TOML Configuration

Add a `[god_object_detection]` section to your `.debtmap.toml`:

```toml
[god_object_detection]
enabled = true

[god_object_detection.rust]
max_methods = 20
max_fields = 15
max_traits = 5      # max_traits = max responsibilities
max_lines = 1000
max_complexity = 200

[god_object_detection.python]
max_methods = 15
max_fields = 10
max_traits = 3
max_lines = 500
max_complexity = 150

[god_object_detection.javascript]
max_methods = 15
max_fields = 20
max_traits = 3
max_lines = 500
max_complexity = 150
```

**Note:** `enabled` defaults to `true`. Set to `false` to disable god object detection entirely (equivalent to `--no-god-object` CLI flag).

See `src/config.rs:500-582`.

### CLI Options

Debtmap provides several CLI flags to control god object detection behavior:

#### `--no-god-object`

Disables god object detection entirely.

```bash
debtmap analyze . --no-god-object
```

**Use case:** When you only want function-level complexity analysis without file-level aggregation.

#### `--aggregate-only`

Shows only file-level god object scores, hiding individual function details.

```bash
debtmap analyze . --aggregate-only
```

**Use case:** High-level overview of which files are god objects without function-by-function breakdowns.

#### `--no-aggregation`

Disables file-level aggregation, showing only individual function metrics.

```bash
debtmap analyze . --no-aggregation
```

**Use case:** Detailed function-level analysis without combining into file scores.

#### `--aggregation-method <METHOD>`

Chooses how to combine function scores into file-level scores:

- `sum` - Add all function scores
- `weighted_sum` - Weight by complexity (default)
- `logarithmic_sum` - Logarithmic scaling for large files
- `max_plus_average` - Max score + average of others

```bash
debtmap analyze . --aggregation-method logarithmic_sum
```

#### `--min-problematic <N>`

Sets minimum number of problematic functions required for file-level aggregation.

```bash
debtmap analyze . --min-problematic 3
```

**Use case:** Avoid flagging files with only 1-2 complex functions as god objects.

See `features.json:65-71` and `features.json:507-512`.

## Output Display

### File-Level Display

When a god object is detected, Debtmap displays:

```
⚠️ God Object: 270 methods, 0 fields, 8 responsibilities
Score: 1350 (Confidence: Definite)
```

### Function-Level Display

Within a god object file, individual functions show:

```
├─ ⚠️ God Object: 45 methods, 20 fields, 5 responsibilities
│      Score: 250 (Confidence: Probable)
```

The `⚠️ God Object` indicator makes it immediately clear which files need architectural refactoring.

## Integration with File-Level Scoring

God object detection affects the overall technical debt prioritization through a **god object multiplier**:

```
god_object_multiplier = 2.0 + normalized_god_object_score
```

Where `normalized_god_object_score` is scaled to 0-1 range.

This means:
1. God objects receive **2-3× higher priority** in debt rankings
2. Functions within god objects may inherit elevated scores due to architectural concerns
3. The cascading impact ensures god objects surface in the "top 10 most problematic" lists

This integration ensures that architectural debt (god objects) is weighted appropriately alongside function-level complexity.

See `features.json:570` and file-level scoring documentation.

## Metrics Tracking (Advanced)

For teams tracking god object evolution over time, Debtmap provides `GodObjectMetrics` with:

- **Snapshots** - Historical god object data per file
- **Trends** - Improving/Stable/Worsening classification (based on ±10 point score changes)
- **New God Objects** - Files that crossed the threshold
- **Resolved God Objects** - Files that were refactored below thresholds

This enables longitudinal analysis: "Are we reducing god objects sprint-over-sprint?"

See `src/organization/god_object_metrics.rs:1-228`.

## Examples and Case Studies

### Example 1: Large Rust Module

**File:** `rust_call_graph.rs` with 270 standalone functions

**Detection:**
- **Is God Object:** Yes
- **Method Count:** 270
- **Field Count:** 0 (no struct)
- **Responsibilities:** 8
- **Confidence:** Definite
- **Score:** >1000 (severe violation)

**Recommendation:** Break into multiple focused modules:
- `CallGraphBuilder` (construction methods)
- `CallGraphAnalyzer` (analysis methods)
- `CallGraphFormatter` (output methods)

### Example 2: Complex Python Class

**File:** `data_manager.py` with class containing 25 methods and 12 fields

**Detection:**
- **Is God Object:** Yes
- **Method Count:** 25
- **Field Count:** 12
- **Responsibilities:** 6 (Data Access, Validation, Persistence, etc.)
- **Confidence:** Probable
- **Score:** ~150-200

**Recommendation:** Split by responsibility:
- `DataAccessLayer` (get/set methods)
- `DataValidator` (validate/check methods)
- `DataPersistence` (save/load methods)

### Example 3: Mixed Paradigm File

**File:** `utils.rs` with small struct (5 methods, 3 fields) + 20 standalone functions

**Detection:**
- **Is God Object:** Yes
- **Total Methods:** 25 (5 + 20)
- **Field Count:** 3
- **Confidence:** Probable
- **Score:** ~120

**Note:** Without file-level aggregation, this would be missed. The struct alone is fine, but combined with standalone functions, it indicates an overgrown utility module.

## Troubleshooting

### "Why is my functional module flagged as a god object?"

**Answer:** Debtmap aggregates standalone functions with struct methods. A file with 100 pure helper functions will be flagged, even though each function is simple.

**Solutions:**
1. Use **purity-weighted scoring** (Rust only) via complexity-weighted analysis - pure functions contribute 0.3× weight
2. Split the module into smaller, focused utility modules
3. Use `--min-problematic` to raise the threshold for file-level aggregation

### "My god object score seems too high"

**Answer:** The scoring algorithm uses exponential scaling (`base_score × 50 × violation_count`) to ensure god objects are prioritized.

**Solutions:**
1. Check the violation count - 5 violations means severe issues
2. Review each metric - are method count, field count, responsibilities, LOC, and complexity all high?
3. Consider if the score accurately reflects maintainability burden

### "Can I disable god object detection for specific files?"

**Answer:** Currently, god object detection is global. However, you can:
1. Use `--no-god-object` to disable entirely
2. Use `--no-aggregation` to skip file-level analysis
3. Adjust thresholds in `.debtmap.toml` to be more lenient

## Best Practices

To avoid god objects:

1. **Follow Single Responsibility Principle** - Each module should have one clear purpose
2. **Regular Refactoring** - Split modules before they reach thresholds
3. **Monitor Growth** - Track method and field counts as modules evolve
4. **Use Composition** - Prefer smaller, composable units over large monoliths
5. **Clear Boundaries** - Define clear module interfaces and responsibilities
6. **Leverage Purity** - Keep pure functions separate from stateful logic (reduces scores in Rust)
7. **Set Project Thresholds** - Customize `.debtmap.toml` to match your team's standards

## Configuration Tradeoffs

**Strict Thresholds** (e.g., Rust: 10 methods):
- ✅ Catch problems early
- ✅ Enforce strong modularity
- ❌ May flag legitimate large modules
- ❌ More noise in reports

**Lenient Thresholds** (e.g., Rust: 50 methods):
- ✅ Reduce false positives
- ✅ Focus on egregious violations
- ❌ Miss real god objects
- ❌ Allow technical debt to grow

**Recommended:** Start with defaults, then adjust based on your codebase's characteristics. Use metrics tracking to monitor trends over time.

## Related Documentation

- [File-Level Scoring](./file-level-scoring.md) - How god objects affect overall file scores
- [Configuration](./configuration.md) - Complete `.debtmap.toml` reference
- [CLI Reference](./cli-reference.md) - All command-line options
- [Tiered Prioritization](./tiered-prioritization.md) - How god objects are prioritized

## Summary

God object detection is a powerful architectural analysis feature that:

- Identifies files/types violating single responsibility principle
- Provides multiple scoring algorithms (simple, complexity-weighted, purity-weighted)
- Generates actionable refactoring recommendations
- Integrates with file-level scoring for holistic debt prioritization
- Supports customization via TOML config and CLI flags

By combining quantitative metrics (method count, LOC, complexity) with qualitative analysis (responsibility detection, purity), Debtmap helps teams systematically address architectural debt.

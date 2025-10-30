# God Object Detection

## Overview

Debtmap includes sophisticated god object detection that identifies files and types that have grown too large and taken on too many responsibilities. God objects (also called "god classes" or "god modules") are a significant source of technical debt as they:

- Violate the Single Responsibility Principle
- Become difficult to maintain and test
- Create bottlenecks in development
- Increase the risk of bugs due to high coupling
- Have high coupling with many other modules
- Are hard to test effectively

This chapter explains how Debtmap identifies god objects, calculates their scores, and provides actionable refactoring recommendations.

## Detection Criteria

Debtmap uses two distinct detection strategies depending on the file structure:

### God Class Criteria

A struct/class is classified as a god class when it violates multiple thresholds:

1. **Method Count** - Number of impl methods on the struct
2. **Field Count** - Number of struct/class fields
3. **Responsibility Count** - Distinct responsibilities inferred from method names (max_traits in config)
4. **Lines of Code** - Estimated lines for the struct and its impl blocks
5. **Complexity Sum** - Combined cyclomatic complexity of struct methods

**Note:** All five criteria are evaluated by the `determine_confidence` function to calculate confidence levels. Each criterion that exceeds its threshold contributes to the violation count.

### God Module Criteria

A file is classified as a god module when it has excessive standalone functions:

1. **Standalone Function Count** - Total standalone functions (not in impl blocks)
2. **Responsibility Count** - Distinct responsibilities across all functions
3. **Lines of Code** - Total lines in the file
4. **Complexity Sum** - Combined cyclomatic complexity (estimated as `function_count × 5`)

**Key Difference:** God class detection focuses on a single struct's methods, while god module detection counts standalone functions across the entire file.

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

**Note:** TypeScript uses the same thresholds as JavaScript since both languages have similar structural patterns. The implementation treats them identically for god object detection purposes.

These thresholds can be customized per-language in your `.debtmap.toml` configuration file.

## God Class vs God Module Detection

Debtmap distinguishes between two distinct types of god objects:

### God Class Detection

A **god class** is a single struct/class with excessive methods and fields. Debtmap analyzes the largest type in a file using:

1. Find the largest type (struct/class) by `method_count + field_count × 2`
2. Count **only the impl methods** for that struct
3. Check against thresholds:
   - Rust: >20 methods, >15 fields
   - Python: >15 methods, >10 fields
   - JavaScript/TypeScript: >15 methods, >20 fields

**Example:** A struct with 25 methods and 18 fields would be flagged as a god class.

### God Module Detection

A **god module** is a file with excessive standalone functions (no dominant struct). Debtmap counts standalone functions when:

1. No struct/class is found, OR
2. The file has many standalone functions outside of any impl blocks

**Implementation Detail:** Debtmap uses the `DetectionType` enum with three variants:
- `GodClass` - Single struct with excessive methods/fields
- `GodFile` - File with excessive functions or lines of code
- `GodModule` - Alias for `GodFile` (both represent the same detection type)

The `GodModule` variant is provided for clarity when discussing files with many standalone functions, but internally it's the same as `GodFile`.

**Example:** A file like `rust_call_graph.rs` with 270 standalone functions would be flagged as a god module (using the `GodFile`/`GodModule` detection type).

### Why Separate Analysis?

Previously, Debtmap combined standalone functions with struct methods, causing **false positives** for functional/procedural modules. The current implementation analyzes them separately to:

- Avoid penalizing pure functional modules
- Distinguish between architectural issues (god class) and organizational issues (god module)
- Provide more accurate refactoring recommendations

**Key Distinction:** A file containing a struct with 15 methods plus 20 standalone functions is analyzed as:
- **God Class:** No (15 methods < 20 threshold)
- **God Module:** Possibly (20 standalone functions, approaching threshold)

See `src/organization/god_object_detector.rs:449-505` for implementation details.

## Confidence Levels

Debtmap assigns confidence levels based **solely on the number of thresholds violated**:

- **Definite** (5 violations) - All five metrics exceed thresholds - clear god object requiring immediate refactoring
- **Probable** (3-4 violations) - Most metrics exceed thresholds - likely god object that should be refactored
- **Possible** (1-2 violations) - Some metrics exceed thresholds - potential god object worth reviewing
- **NotGodObject** (0 violations) - All metrics within acceptable limits

**Note:** The confidence level is determined by violation count alone. The god object score (calculated separately) is used for prioritization and ranking, but does not affect the confidence classification.

**Example:** Consider two files both with `violation_count=2` (Possible confidence):
- File A: 21 methods, 16 fields (just over the threshold)
- File B: 100 methods, 50 fields (severely over the threshold)

Both receive the same "Possible" confidence level, but File B will have a much higher god object score for prioritization purposes. This separation ensures consistent confidence classification while still allowing scores to reflect severity.

See `src/organization/god_object_analysis.rs:236-268` for the `determine_confidence` function.

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

This advanced scoring variant combines both **complexity weighting** and **purity analysis**, building on top of complexity-weighted scoring to further reduce the impact of pure functions. This prevents pure functional modules from being unfairly penalized. The algorithm:

1. Analyzes each function for purity using three levels:
   - **Pure** (no side effects): Functions with read-only operations, no I/O, no mutation
     - Weight multiplier: `0.3`
     - Examples: `calculate_sum()`, `format_string()`, `is_valid()`

   - **Probably Pure** (likely no side effects): Functions that appear pure but may have hidden side effects
     - Weight multiplier: `0.5`
     - Examples: Functions using trait methods (could have side effects), generic operations

   - **Impure** (has side effects): Functions with clear side effects like I/O, mutation, external calls
     - Weight multiplier: `1.0`
     - Examples: `save_to_file()`, `update_state()`, `send_request()`

2. Purity Detection Heuristics:
   - **Pure indicators**: No `mut` references, no I/O operations, no external function calls
   - **Impure indicators**: File/network operations, mutable state, database access, logging
   - **Probably Pure**: Generic functions, trait method calls, or ambiguous patterns

3. Combines complexity and purity weights to calculate the total contribution:
   ```
   total_weight = complexity_weight × purity_multiplier
   ```

   This means pure functions get both the complexity-based weight AND the purity multiplier applied together.

   **Example:** A pure function with complexity 5 contributes only `5 × 0.3 = 1.5` to the weighted count (compared to 5.0 for an impure function of the same complexity).

4. Tracks the `PurityDistribution`:
   - `pure_count`, `probably_pure_count`, `impure_count`
   - `pure_weight_contribution`, `probably_pure_weight_contribution`, `impure_weight_contribution`

**Impact:** A file with 100 pure helper functions (total complexity 150) might have a weighted method count of only `150 × 0.3 = 45`, avoiding false positives while still catching stateful god objects with many impure methods.

See `src/organization/god_object_detector.rs:196-258` and `src/organization/purity_analyzer.rs`.

## Responsibility Detection

Responsibilities are inferred from method names using common prefixes. Debtmap recognizes the following categories:

| Prefix(es) | Responsibility Category |
|------------|------------------------|
| `format`, `render`, `write`, `print` | Formatting & Output |
| `parse`, `read`, `extract` | Parsing & Input |
| `filter`, `select`, `find` | Filtering & Selection |
| `transform`, `convert`, `map`, `apply` | Transformation |
| `get`, `set` | Data Access |
| `validate`, `check`, `verify`, `is` | Validation |
| `calculate`, `compute` | Computation |
| `create`, `build`, `new` | Construction |
| `save`, `load`, `store` | Persistence |
| `process`, `handle` | Processing |
| `send`, `receive` | Communication |
| *(no prefix match)* | Utilities |

**Note:** `Utilities` serves as both a category in the responsibility list and the fallback when no prefix matches. In the implementation, `Utilities` is included in `RESPONSIBILITY_CATEGORIES` with an empty prefixes array, making it the catch-all category returned by `infer_responsibility_from_method` when no other category matches.

**Distinct Responsibility Counting:** Debtmap counts the number of **unique** responsibility categories used by a struct/module's methods. A high responsibility count (e.g., >5) indicates the module is handling too many different concerns, violating the Single Responsibility Principle.

Responsibility count directly affects:
- God object scoring (via `responsibility_factor`)
- Refactoring recommendations (methods grouped by responsibility for suggested splits)
- Detection confidence (counted as one of the five violation criteria)

See `src/organization/god_object_analysis.rs:318-388` for the `infer_responsibility_from_method` function.

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

### Example 3: Mixed Paradigm File (God Module)

**File:** `utils.rs` with small struct (5 methods, 3 fields) + 60 standalone functions

**Detection:**
- **God Class (struct):** No (5 methods < 20 threshold, 3 fields < 15 threshold)
- **God Module (file):** Yes (60 standalone functions > 50 threshold)
- **Confidence:** Probable
- **Score:** ~120

**Analysis:** The struct and standalone functions are analyzed separately. The struct is not a god class, but the file is a god module due to the excessive standalone functions. This indicates an overgrown utility module that should be split into smaller, focused modules.

**Recommendation:** Split standalone functions into focused utility modules:
- `StringUtils` (formatting, parsing)
- `FileUtils` (file operations)
- `MathUtils` (calculations)

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

### Code Examples

#### Split by Responsibility

```rust
// Before: UserManager (god object)
struct UserManager { ... }

// After: Split into focused modules
struct AuthService { ... }
struct ProfileService { ... }
struct PermissionService { ... }
struct NotificationService { ... }
```

#### Extract Common Functionality

```rust
// Extract shared dependencies
struct ServiceContext {
    db: Database,
    cache: Cache,
    logger: Logger,
}

// Each service gets a reference
struct AuthService<'a> {
    context: &'a ServiceContext,
}
```

#### Use Composition

```rust
// Compose services instead of inheriting
struct UserFacade {
    auth: AuthService,
    profile: ProfileService,
    permissions: PermissionService,
}

impl UserFacade {
    fn login(&mut self, credentials: Credentials) -> Result<Session> {
        self.auth.login(credentials)
    }
}
```

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

# Note: The configuration field is named 'max_traits' for historical reasons,
# but it controls the maximum number of responsibilities/concerns, not Rust traits.
# This is a legacy naming issue from early development.

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

### Tuning for Your Project

**Strict mode (smaller modules):**
```toml
[god_object_detection.rust]
max_methods = 15
max_fields = 10
max_traits = 3
```

**Lenient mode (larger modules acceptable):**
```toml
[god_object_detection.rust]
max_methods = 30
max_fields = 20
max_traits = 7
```

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

### Normalization

The `normalized_god_object_score` is scaled to the 0-1 range using:

```
normalized_score = min(god_object_score / max_expected_score, 1.0)
```

Where `max_expected_score` is typically based on the maximum score in the analysis (e.g., 1000 for severe violations).

### Impact on Prioritization

This multiplier means:
1. **Non-god objects** (score = 0): multiplier = 2.0 (baseline)
2. **Moderate god objects** (score = 200): multiplier ≈ 2.2-2.5
3. **Severe god objects** (score = 1000+): multiplier ≈ 3.0 (maximum)

**Result:** God objects receive **2-3× higher priority** in debt rankings, ensuring that:
- Functions within god objects inherit elevated scores due to architectural concerns
- God objects surface in the "top 10 most problematic" lists
- Architectural debt is weighted appropriately alongside function-level complexity

See file-level scoring documentation for complete details on how this multiplier integrates into the overall debt calculation.

## Metrics Tracking (Advanced)

For teams tracking god object evolution over time, Debtmap provides `GodObjectMetrics` with:

- **Snapshots** - Historical god object data per file
- **Trends** - Improving/Stable/Worsening classification (based on ±10 point score changes)
- **New God Objects** - Files that crossed the threshold
- **Resolved God Objects** - Files that were refactored below thresholds

This enables longitudinal analysis: "Are we reducing god objects sprint-over-sprint?"

See `src/organization/god_object_metrics.rs:1-228`.

## Troubleshooting

### "Why is my functional module flagged as a god object?"

**Answer:** Debtmap now analyzes god classes (structs) separately from god modules (standalone functions). If your functional module with 100 pure helper functions is flagged, it's being detected as a **god module** (not a god class), which indicates the file has grown too large and should be split for better organization.

**Solutions:**
1. **Accept the finding**: 100+ functions in one file is difficult to navigate and maintain, even if each function is simple
2. **Split by responsibility**: Organize functions into smaller, focused modules (e.g., `string_utils.rs`, `file_utils.rs`, `math_utils.rs`)
3. **Use purity-weighted scoring** (Rust only): Pure functions contribute only 0.3× weight, dramatically reducing scores for functional modules
4. **Adjust thresholds**: Increase `max_methods` in `.debtmap.toml` if your project standards allow larger modules

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

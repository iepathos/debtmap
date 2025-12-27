# Function-Level Scoring

Function-level scoring identifies specific functions needing attention for targeted improvements. This subsection covers the scoring formula, constructor detection, role classification, and role multipliers.

## Overview

Function-level scoring combines complexity, coverage, and dependency metrics to calculate a priority score for each function. The formula uses coverage as a multiplicative dampener rather than an additive factor, reflecting that testing gaps amplify existing complexity.

**Key Principle**: Untested complex code is riskier than well-tested complex code. Coverage acts as a multiplier that reduces the score for well-tested functions and preserves the full score for untested functions.

## Scoring Formula

The function-level scoring formula consists of three stages:

### Stage 1: Factor Calculation

**Complexity Factor** (`src/priority/scoring/calculation.rs:55-59`):
```
complexity_factor = raw_complexity / 2.0  (clamped to 0-10)
```

Complexity of 20+ maps to the maximum factor of 10.0. This provides linear scaling with a reasonable cap.

**Dependency Factor** (`src/priority/scoring/calculation.rs:62-66`):
```
dependency_factor = upstream_count / 2.0  (capped at 10.0)
```

20+ upstream dependencies map to the maximum factor of 10.0.

### Stage 2: Base Score Calculation

**Without Coverage Data** (`src/priority/scoring/calculation.rs:119-129`):
```
base_score = (complexity_factor × 10 × 0.50) + (dependency_factor × 10 × 0.25)
```

Weights:
- **50%** weight on complexity
- **25%** weight on dependencies
- **25%** reserved for debt pattern adjustments

**With Coverage Data** (`src/priority/scoring/calculation.rs:70-82`):
```
coverage_multiplier = 1.0 - coverage_percent
base_score = base_score_no_coverage × coverage_multiplier
```

Coverage acts as a dampening multiplier:
- **0% coverage** (multiplier = 1.0): Full base score preserved
- **50% coverage** (multiplier = 0.5): Half the base score
- **100% coverage** (multiplier = 0.0): Near-zero score

### Stage 3: Role Multiplier

The final score applies a role-based multiplier:

```
final_score = base_score × role_multiplier
```

See [Role Multipliers](#role-multipliers) for the specific values.

### Complete Example

```
Function: calculate_risk_score()
  Cyclomatic: 12, Cognitive: 18
  Coverage: 20%
  Upstream dependencies: 8

Step 1: Calculate factors
  complexity_factor = (12 + 18) / 2 / 2.0 = 7.5 (capped at 10)
  dependency_factor = 8 / 2.0 = 4.0

Step 2: Base score (no coverage)
  base = (7.5 × 10 × 0.50) + (4.0 × 10 × 0.25)
  base = 37.5 + 10.0 = 47.5

Step 3: Apply coverage multiplier
  coverage_multiplier = 1.0 - 0.20 = 0.80
  score_with_coverage = 47.5 × 0.80 = 38.0

Step 4: Apply role multiplier (PureLogic = 1.2)
  final_score = 38.0 × 1.2 = 45.6
```

## Metrics

### Cyclomatic Complexity

Counts decision points (if, match, loops, boolean operators). Guides the number of test cases needed for full branch coverage.

**Interpretation**:
- 1-5: Low complexity, easy to test
- 6-10: Moderate complexity, reasonable test effort
- 11-20: High complexity, significant test effort
- 20+: Very high complexity, consider refactoring

### Cognitive Complexity

Measures how difficult code is to understand, accounting for:
- Nesting depth (deeper nesting = higher penalty)
- Control flow breaks
- Recursion

**Why Cognitive Gets Higher Weight** (`src/config/scoring.rs:367-373`):
- Cyclomatic weight: 30%
- Cognitive weight: 70%

Cognitive complexity correlates better with bug density because it measures comprehension difficulty, not just branching paths.

### Coverage Percentage

Direct line coverage from LCOV data. Functions with 0% coverage receive maximum urgency.

**Coverage Dampening** (`src/priority/scoring/calculation.rs:8-21`):
- Test code automatically receives 0.0 multiplier (near-zero score)
- Production code: multiplier = 1.0 - coverage_percent

### Dependency Count

Upstream callers indicate impact radius. Functions with many callers are riskier to modify.

## Constructor Detection

Debtmap identifies simple constructors to prevent false positives where trivial initialization functions are flagged as critical business logic.

### Detection Strategy

A function is classified as a constructor if it meets these criteria (`src/analyzers/rust_constructor_detector.rs:1-50`):

**1. Name Pattern Match**:
- Exact: `new`, `default`, `empty`, `zero`, `any`
- Prefix: `from_*`, `with_*`, `create_*`, `make_*`, `build_*`

**2. Complexity Thresholds**:
- Cyclomatic complexity: <= 2
- Cognitive complexity: <= 3
- Function length: < 15 lines
- Nesting depth: <= 1

**3. AST Analysis** (when enabled):
- Return type: `Self`, `Result<Self, E>`, or `Option<Self>`
- Body pattern: Struct initialization, no loops
- No complex control flow

### Return Type Classification

The AST detector classifies return types (`src/analyzers/rust_constructor_detector.rs:36-42`):

| Return Type | Classification |
|-------------|----------------|
| `Self` | OwnedSelf |
| `Result<Self, E>` | ResultSelf |
| `Option<Self>` | OptionSelf |
| `&Self`, `&mut Self` | RefSelf (builder pattern) |
| Other | Other |

### Body Pattern Analysis

The constructor detector visits the function body (`src/analyzers/rust_constructor_detector.rs:104-130`):

```rust
// Tracks these patterns:
struct BodyPattern {
    struct_init_count: usize,  // Struct initialization expressions
    self_refs: usize,          // References to Self
    field_assignments: usize,  // Field assignment expressions
    has_if: bool,              // Contains if expression
    has_match: bool,           // Contains match expression
    has_loop: bool,            // Contains any loop
    early_returns: usize,      // Return statements
}
```

**Constructor-Like Pattern** (`src/analyzers/rust_constructor_detector.rs:152-158`):
- Has struct initialization AND no loops, OR
- Has Self references AND no loops AND no match AND no field assignments

### Examples

**Detected as Constructor** (classified as IOWrapper, score reduced):
```rust
fn new() -> Self {
    Self { field: 0 }
}

fn from_config(config: Config) -> Self {
    Self {
        timeout: config.timeout,
        retries: 3,
    }
}

fn try_new(value: i32) -> Result<Self, Error> {
    if value > 0 {
        Ok(Self { value })
    } else {
        Err(Error::InvalidValue)
    }
}
```

**NOT Detected as Constructor** (remains PureLogic):
```rust
// Has loop - disqualified
fn process_items() -> Self {
    let mut result = Self::new();
    for item in items {
        result.add(item);
    }
    result
}

// High complexity - disqualified
fn create_complex(data: Data) -> Result<Self> {
    validate(&data)?;
    // ... 30 lines of logic
    Ok(Self { ... })
}
```

## Role Classification

Functions are classified by semantic role to adjust their priority scores appropriately.

### Classification Order

The classifier applies rules in precedence order (`src/priority/semantic_classifier/mod.rs:47-114`):

1. **EntryPoint**: Main functions, CLI handlers, routes
2. **Debug**: Functions with debug/diagnostic patterns
3. **Constructor**: Simple object construction (enhanced detection)
4. **EnumConverter**: Match-based enum to value conversion
5. **Accessor**: Getters, is_*, has_* methods
6. **DataFlow**: High transformation ratio (spec 126)
7. **PatternMatch**: Pattern matching functions
8. **IOWrapper**: File/network I/O thin wrappers
9. **Orchestrator**: Functions coordinating other functions
10. **PureLogic**: Default for unclassified functions

### Entry Point Detection

Entry points are identified by:
- Call graph analysis: No upstream callers
- Name patterns: `main`, `handle_*`, `run_*`, `execute_*`

### Debug Function Detection

Debug/diagnostic functions are detected via (`src/priority/semantic_classifier/mod.rs:59-61`):
- Name patterns: `debug_*`, `print_*`, `dump_*`, `trace_*`, `*_diagnostics`, `*_stats`
- Low complexity threshold
- Output-focused behavior

### Accessor Detection

Accessor methods are identified when (`src/priority/semantic_classifier/mod.rs:147-177`):
- Name matches accessor pattern: `id`, `name`, `get_*`, `is_*`, `has_*`, `as_*`, `to_*`
- Cyclomatic complexity <= 2
- Cognitive complexity <= 1
- Length < 10 lines
- (With AST) Simple field access body

## Role Multipliers

Each role receives a score multiplier based on test priority importance (`src/config/scoring.rs:307-333`):

| Role | Multiplier | Rationale |
|------|------------|-----------|
| **PureLogic** | 1.2 | Core business logic deserves high test priority |
| **Unknown** | 1.0 | Default, no adjustment |
| **EntryPoint** | 0.9 | Often integration tested, slight reduction |
| **Orchestrator** | 0.8 | Coordinates tested functions, reduced priority |
| **IOWrapper** | 0.7 | Thin I/O wrappers, integration tested |
| **PatternMatch** | 0.6 | Simple pattern dispatch, lower priority |
| **Debug** | 0.3 | Diagnostic functions, lowest priority |

### Multiplier Rationale

**PureLogic (1.2x)**: Business rules and algorithms should have comprehensive unit tests. They're easy to test in isolation and contain the core value of the application.

**Orchestrator (0.8x)**: Orchestrators coordinate other tested functions. If the delegated functions are well-tested, the orchestrator is partially covered through integration.

**IOWrapper (0.7x)**: Thin I/O wrappers are often tested via integration tests. Unit testing them provides limited value compared to integration testing.

**Debug (0.3x)**: Diagnostic and debug functions have the lowest test priority. They're not production-critical and are often exercised manually during development.

### Configuration

Role multipliers are configurable in `.debtmap.toml`:

```toml
[role_multipliers]
pure_logic = 1.2
orchestrator = 0.8
io_wrapper = 0.7
entry_point = 0.9
pattern_match = 0.6
debug = 0.3
unknown = 1.0
```

### Role Multiplier Clamping

To prevent extreme score swings, multipliers can be clamped (`src/config/scoring.rs:457-493`):

```toml
[scoring.role_multiplier]
clamp_min = 0.3    # Floor for all multipliers
clamp_max = 1.8    # Ceiling for all multipliers
enable_clamping = true
```

## Complexity Weight Configuration

The balance between cyclomatic and cognitive complexity is configurable (`src/config/scoring.rs:335-381`):

```toml
[complexity_weights]
cyclomatic = 0.3   # 30% weight
cognitive = 0.7    # 70% weight
max_cyclomatic = 50.0
max_cognitive = 100.0
```

**Default Rationale**:
- Cognitive complexity (70%) correlates better with bug density
- Cyclomatic complexity (30%) guides test case count
- Combined weighting provides balanced assessment

## Score Normalization

Raw scores undergo normalization for display (`src/priority/scoring/calculation.rs:174-206`):

| Score Range | Method | Formula |
|-------------|--------|---------|
| 0-10 | Linear | score (unchanged) |
| 10-100 | Square root | 10.0 + sqrt(score - 10.0) × 3.33 |
| 100+ | Logarithmic | 41.59 + ln(score / 100.0) × 10.0 |

This multi-phase approach:
- Preserves distinctions for low scores
- Moderately dampens medium scores
- Strongly dampens extreme values

## See Also

- [File-Level Scoring](file-level.md): Aggregate file scoring
- [Role-Based Adjustments](role-based.md): Detailed role adjustment mechanics
- [Rebalanced Scoring](rebalanced.md): Alternative scoring algorithm
- [Data Flow Scoring](data-flow.md): Purity-based adjustments

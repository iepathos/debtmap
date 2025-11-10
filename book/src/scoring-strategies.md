# Scoring Strategies

Debtmap provides two complementary scoring approaches: **file-level** and **function-level**. Understanding when to use each approach helps you make better refactoring decisions and prioritize work effectively.

## Overview

Different refactoring scenarios require different levels of granularity:
- **File-level scoring**: Identifies architectural issues and planning major refactoring initiatives
- **Function-level scoring**: Pinpoints specific hot spots for targeted improvements

This chapter explains both approaches, when to use each, and how to interpret the results.

## File-Level Scoring

File-level scoring aggregates metrics across all functions in a file to identify architectural problems and module-level refactoring opportunities.

### Formula

```
File Score = Size × Complexity × Coverage Factor × Density × GodObject × FunctionScores
```

**Note**: This is a conceptual formula showing the multiplicative relationship between factors. The actual implementation in `src/priority/file_metrics.rs` includes additional normalization steps and conditional adjustments. See source code for exact calculation details.

Where each factor is calculated as:
- **Size** = `sqrt(total_lines / 100)`
- **Complexity** = `(avg_complexity / 5.0) × sqrt(total_complexity / 50.0)`
- **Coverage Factor** = `((1.0 - coverage_percent) × 2.0) + 1.0`
- **Density** = `1.0 + ((function_count - 50) × 0.02)` if function_count > 50, else 1.0
- **GodObject** = `2.0 + god_object_score` if detected
- **FunctionScores** = `sum(function_scores) / 10`

### Factors

**Size Factor**: `sqrt(total_lines / 100)`
- Larger files have higher impact
- Square root dampens the effect to avoid over-penalizing large files
- Rationale: Refactoring a 1000-line file affects more code than a 100-line file

**Complexity Factor**: Combines average and total complexity
- `(average_cyclomatic + total_cyclomatic / function_count) / 2`
- Balances per-function and aggregate complexity
- Rationale: Both concentrated complexity and spread-out complexity matter

**Coverage Factor**: `(coverage_gap × 2.0) + 1.0` where `coverage_gap = 1.0 - coverage_percent`
- Lower coverage increases score multiplicatively
- Range: 1.0 (100% coverage) to 3.0 (0% coverage)
- Formula expands to: `((1.0 - coverage_percent) × 2.0) + 1.0`
- Example: 50% coverage → gap=0.5 → factor=(0.5×2.0)+1.0 = 2.0x
- Rationale: Untested files amplify existing complexity and risk through a multiplicative factor greater than 1.0
- Note: Earlier versions used `1.0 - coverage_percent` (range 0-1); current implementation uses expanded range 1-3 for stronger emphasis

**Density Factor**: Penalizes files with excessive function count
- Triggers when function count > 50
- Formula: `1.0 + ((function_count - 50) * 0.02)` if function_count > 50, else 1.0
- Creates a gradual linear increase: 51 functions = 1.02x, 75 functions = 1.50x, 100 functions = 2.0x
- Example: A file with 75 functions gets 1.0 + ((75 - 50) * 0.02) = 1.0 + 0.50 = 1.50x multiplier
- Rationale: Files with many functions likely violate single responsibility

**God Object Multiplier**: `2.0 + god_object_score` when detected
- Applies when god object detection flags the file
- Range: 2.0 (borderline) to 3.0 (severe god object)
- Rationale: God objects need immediate architectural attention

**Function Scores**: `sum(all_function_scores) / 10`
- Normalized sum of individual function debt scores
- Provides baseline before modifiers

### Use Cases

**1. Planning Major Refactoring Initiatives**

```bash
# Show top 10 files needing architectural refactoring
debtmap analyze . --aggregate-only --top 10
```

Use when:
- Planning sprint or quarterly refactoring work
- Deciding which modules to split
- Prioritizing architectural improvements
- Allocating team resources

**Note**: File-level scoring is enabled with the `--aggregate-only` flag (a boolean flag—no value needed), which changes output to show only file-level metrics instead of function-level details.

**2. Identifying Architectural Issues**

File-level scoring excels at finding:
- God objects with too many responsibilities
- Files with poor cohesion
- Modules that should be split
- Files with too many functions

```bash
# Focus on architectural problems
debtmap analyze . --aggregate-only --filter Architecture
```

**3. Breaking Up Monolithic Modules**

```bash
# Find files with excessive function counts
debtmap analyze . --aggregate-only --min-problematic 50
```

**4. Evaluating Overall Codebase Health**

```bash
# Generate file-level report for executive summary
debtmap analyze . --aggregate-only --format markdown -o report.md
```

### Aggregation Methods

Debtmap supports multiple aggregation methods for file-level scores, configurable via CLI or configuration file.

#### Weighted Sum (Default)

**Formula**: `Σ(function_score × complexity_weight × coverage_weight)`

```bash
debtmap analyze . --aggregation-method weighted_sum
```

Or via configuration:
```toml
[aggregation]
method = "weighted_sum"
```

**Characteristics**:
- Weights functions by their complexity and coverage gaps
- Emphasizes high-impact functions over trivial ones
- Best for most use cases where you want to focus on significant issues

**Best for**: Standard codebases where you want proportional emphasis on complex, untested code

#### Simple Sum

**Formula**: `Σ(function_scores)`

```toml
[aggregation]
method = "sum"
```

**Characteristics**:
- Adds all function scores directly without weighting
- Treats all functions equally regardless of complexity
- Useful for broad overview and trend analysis

**Best for**: Getting a raw count-based view of technical debt across all functions

#### Logarithmic Sum

**Formula**: `log(1 + Σ(function_scores))`

```toml
[aggregation]
method = "logarithmic_sum"
```

**Characteristics**:
- Dampens impact of many small issues to prevent score explosion
- Prevents files with hundreds of minor issues from dominating
- Creates more balanced comparisons across files of different sizes

**Best for**: Legacy codebases with many small issues where you want to avoid extreme scores

#### Max Plus Average

**Formula**: `max_score × 0.6 + avg_score × 0.4`

```toml
[aggregation]
method = "max_plus_average"
```

**Characteristics**:
- Considers worst function (60%) plus average of all functions (40%)
- Balances worst-case and typical-case scenarios
- Highlights files with both a critical hot spot and general issues

**Best for**: Identifying files with concentrated complexity alongside general code quality concerns

#### Choosing an Aggregation Method

| Codebase Type | Recommended Method | Rationale |
|---------------|-------------------|-----------|
| New/Modern | `weighted_sum` | Proportional emphasis on real issues |
| Legacy with many small issues | `logarithmic_sum` | Prevents score explosion |
| Mixed quality | `max_plus_average` | Balances hot spots with overall quality |
| Trend analysis | `sum` | Simple, consistent metric over time |

**Performance Note**: All aggregation methods have O(n) complexity where n = number of functions. Performance differences are negligible for typical codebases (<100k functions). Choose based on prioritization strategy, not performance concerns.

### Configuration

> **IMPORTANT**: The configuration file must be named **`.debtmap.toml`** (not `debtmap.yml` or other variants) and placed in your project root directory.

```toml
[aggregation]
method = "weighted_sum"
min_problematic = 3              # Need 3+ problematic functions for file-level score

[god_object_detection]
enabled = true
max_methods = 20
max_fields = 15
max_responsibilities = 5
```

## Function-Level Scoring

Function-level scoring identifies specific functions needing attention for targeted improvements.

### Formula

```
Base Score = (Complexity Factor × 10 × 0.50) + (Dependency Factor × 10 × 0.25)
Coverage Multiplier = 1.0 - coverage_percent
Final Score = Base Score × Coverage Multiplier × Role Multiplier
```

**Formula Breakdown:**
1. **Complexity Factor**: Raw complexity / 2.0, clamped to 0-10 range (complexity of 20+ maps to 10.0)
2. **Dependency Factor**: Upstream dependency count / 2.0, capped at 10.0 (20+ dependencies map to 10.0)
3. **Base Score**: (Complexity Factor × 10 × 0.50) + (Dependency Factor × 10 × 0.25)
   - 50% weight on complexity, 25% weight on dependencies
4. **Coverage Multiplier**: 1.0 - coverage_percent (0% coverage = 1.0, 100% coverage = 0.0)
5. **Final Score**: Base Score × Coverage Multiplier × Role Multiplier

**Why Hard-Coded Weights?** The base weights (0.50 for complexity, 0.25 for dependencies) are intentionally not configurable to:
- **Ensure consistency**: Scores remain comparable across projects and teams
- **Prevent instability**: Avoid extreme configurations that break prioritization
- **Simplify configuration**: Reduce cognitive load for users
- **Maintain calibration**: Weights are empirically tuned based on analysis of real codebases

You can still customize prioritization significantly through configurable `role_multipliers`, `coverage_weights`, and normalization settings.

**Note**: Coverage acts as a dampening multiplier rather than an additive factor. Lower coverage (higher multiplier) increases the final score, making untested complex code a higher priority. Role multipliers and coverage weights remain configurable to allow customization while maintaining stable base calculations.

**Migration Note**: Earlier versions used an additive model with weights (Complexity × 0.35) + (Coverage × 0.50) + (Dependency × 0.15). The current model (spec 122) uses coverage as a multiplicative dampener, which better reflects that testing gaps amplify existing complexity rather than adding to it.

### Metrics

**Cyclomatic Complexity**
- Counts decision points (if, match, loops)
- Guides test case count

**Cognitive Complexity**
- Measures understanding difficulty
- Accounts for nesting depth

**Coverage Percentage**
- Direct line coverage from LCOV
- 0% coverage = maximum urgency

**Dependency Count**
- Upstream callers + downstream callees
- Higher dependencies = higher impact

**Role Multiplier**

Functions are classified by role, and each role receives a multiplier based on its architectural importance:

| Role | Multiplier | Description |
|------|------------|-------------|
| **Pure logic** | 1.2x | Core business rules and algorithms |
| **Unknown** | 1.0x | Functions without clear classification |
| **Entry point** | 0.9x | Public APIs, main functions, HTTP handlers |
| **Orchestrator** | 0.8x | Functions that coordinate other functions |
| **IO wrapper** | 0.7x | Simple file/network I/O wrappers |
| **Pattern match** | 0.6x | Functions primarily doing pattern matching |

**Note**: Role multipliers are configurable via the `[role_multipliers]` section in `.debtmap.toml`. The multipliers have been rebalanced to be less extreme than earlier versions - pure logic was reduced from 1.5x to 1.2x, while orchestrator and IO wrapper were increased to better reflect their importance in modern codebases.

### Constructor Detection

Debtmap includes intelligent constructor detection to prevent false positives where trivial initialization functions are misclassified as critical business logic.

**Problem**: Simple constructors like `new()`, `default()`, or `from_config()` often have low complexity but were being flagged as high-priority pure logic functions.

**Solution**: Constructor detection automatically identifies and classifies these functions as `IOWrapper` (low priority) instead of `PureLogic` (high priority).

**Detection Criteria**:

A function is considered a simple constructor if it meets ALL of the following:

1. **Name matches a constructor pattern** (configurable):
   - Exact match: `new`, `default`, `empty`, `zero`, `any`
   - Prefix match: `from_*`, `with_*`, `create_*`, `make_*`, `build_*`, `of_*`

2. **Low cyclomatic complexity** (≤ 2 by default)
3. **Short length** (< 15 lines by default)
4. **Minimal nesting** (≤ 1 level by default)
5. **Low cognitive complexity** (≤ 3 by default)

**Example**:

```rust
// Simple constructor - detected and classified as IOWrapper
fn new() -> Self {
    Self {
        field1: 0,
        field2: String::new(),
    }
}

// Complex factory - NOT detected as constructor, remains PureLogic
fn create_with_validation(data: Data) -> Result<Self> {
    validate(&data)?;
    // ... 30 lines of logic
    Ok(Self { ... })
}
```

**Configuration**:

Constructor detection is fully configurable in `.debtmap.toml`:

```toml
[classification.constructors]
# Enable AST-based constructor detection (default: true)
# When enabled, uses Abstract Syntax Tree analysis for accurate detection
# Disable only if experiencing performance issues with very large codebases
ast_detection = true

# Constructor name patterns
patterns = [
    "new",
    "default",
    "from_",
    "with_",
    "create_",
    "make_",
    "build_",
    "of_",
    "empty",
    "zero",
    "any",
]

# Complexity thresholds
max_cyclomatic = 2     # Maximum cyclomatic complexity
max_cognitive = 3      # Maximum cognitive complexity
max_length = 15        # Maximum lines
max_nesting = 1        # Maximum nesting depth
```

**Customization Example**:

To add custom constructor patterns or adjust thresholds:

```toml
[classification.constructors]
ast_detection = true      # Keep AST detection enabled (recommended)

patterns = [
    "new",
    "default",
    "from_",
    "with_",
    "init_",        # Add custom pattern
    "setup_",       # Add custom pattern
]
max_cyclomatic = 3    # Allow slightly more complex constructors
max_length = 20       # Allow longer constructors
```

To disable AST-based detection (if experiencing performance issues):

```toml
[classification.constructors]
ast_detection = false     # Fall back to pattern-only matching
# Note: May reduce detection accuracy but improves performance
```

**Performance and Disabling**:

Constructor detection is **always enabled** and cannot be fully disabled, as it's integral to accurate priority scoring. However, you can:

1. **Disable AST analysis** (shown above): Falls back to pattern-only matching, reducing accuracy but improving performance for very large codebases (100k+ functions)
2. **Adjust thresholds**: Make detection more lenient by increasing `max_cyclomatic`, `max_cognitive`, or `max_length`
3. **Remove patterns**: Delete specific patterns from the `patterns` list to exclude them from detection

**Performance Impact**:
- AST-based detection: Negligible impact (<5% overhead) for typical codebases
- Pattern-only detection: Near-zero performance impact
- Recommendation: Keep `ast_detection = true` unless profiling shows it's a bottleneck

**Accuracy Trade-offs**:
- With AST: 95%+ accuracy in identifying simple constructors
- Without AST: ~70% accuracy, more false negatives

This feature is part of spec 117 and helps reduce false positives in priority scoring.

### Role-Based Adjustments

DebtMap uses a sophisticated two-stage role adjustment mechanism to ensure that scores accurately reflect both the testing strategy appropriate for each function type and the architectural importance of different roles.

#### Why Role-Based Adjustments?

**Problem**: Traditional scoring treats all functions equally, leading to false positives:

1. **Entry points** (CLI handlers, HTTP routes, `main` functions) typically use integration tests rather than unit tests
   - Flagging them for "low unit test coverage" misses that they're tested differently
   - They orchestrate other code but contain minimal business logic

2. **Pure business logic** functions should have comprehensive unit tests
   - Easy to test in isolation with deterministic inputs/outputs
   - Core value of the application lives here

3. **I/O wrappers** are often tested implicitly through integration tests
   - Thin abstractions over file system, network, or database operations
   - Unit testing them provides limited value compared to integration testing

**Solution**: DebtMap applies role-based adjustments in two stages to address both coverage expectations and architectural importance.

#### Stage 1: Role-Based Coverage Weighting

The first stage adjusts coverage penalty expectations based on function role. This prevents functions that use different testing strategies from unfairly dominating the priority list.

**How It Works**:

For each function, DebtMap:
1. Detects the function's role (entry point, pure logic, I/O wrapper, etc.)
2. Applies a coverage weight multiplier based on that role
3. Reduces or increases the coverage penalty accordingly

**Default Coverage Weights** (configurable in `.debtmap.toml`):

| Function Role    | Coverage Weight | Impact on Scoring |
|------------------|-----------------|-------------------|
| Pure Logic       | 1.2             | Higher coverage penalty (should have unit tests) |
| Unknown          | 1.0             | Standard penalty |
| Pattern Match    | 1.0             | Standard penalty |
| Orchestrator     | 0.8             | Reduced penalty (partially integration tested) |
| I/O Wrapper      | 0.7             | Reduced penalty (often integration tested) |
| Entry Point      | 0.6             | Significantly reduced penalty (integration tested) |

**Example Score Changes**:

**Before role-based coverage adjustment**:
```
Function: handle_request (Entry Point)
  Complexity: 5
  Coverage: 0%
  Raw Coverage Penalty: 1.0 (full penalty)
  Score: 8.5 (flagged as high priority)
```

**After role-based coverage adjustment**:
```
Function: handle_request (Entry Point)
  Complexity: 5
  Coverage: 0%
  Adjusted Coverage Penalty: 0.4 (60% reduction via 0.6 weight)
  Score: 4.2 (medium priority - more realistic)

  Rationale: Entry points are integration tested, not unit tested.
  This function is likely tested via API/CLI integration tests.
```

**Comparison with Pure Logic**:
```
Function: calculate_discount (Pure Logic)
  Complexity: 5
  Coverage: 0%
  Adjusted Coverage Penalty: 1.2 (20% increase via 1.2 weight)
  Score: 9.8 (critical priority)

  Rationale: Pure logic should have unit tests.
  This function needs immediate test coverage.
```

#### Stage 2: Role Multiplier

The second stage applies a final role-based multiplier to reflect architectural importance. This multiplier is **clamped by default** to prevent extreme score swings.

**Configuration** (`.debtmap.toml` under `[scoring.role_multiplier]`):

```toml
[scoring.role_multiplier]
clamp_min = 0.3           # Minimum multiplier (default: 0.3)
clamp_max = 1.8           # Maximum multiplier (default: 1.8)
enable_clamping = true    # Enable clamping (default: true)
```

**Clamp Range Rationale**:
- **Default [0.3, 1.8]**: Balances differentiation with stability
- **Lower bound (0.3)**: I/O wrappers still contribute 30% of base score (not invisible)
- **Upper bound (1.8)**: Critical entry points don't overwhelm other issues (max 180%)
- **Configurable**: Adjust based on project priorities

**Example with Clamping**:
```
Function: process_data (Complex Pure Logic)
  Base Score: 45.0
  Unclamped Role Multiplier: 2.5
  Clamped Multiplier: 1.8 (clamp_max)
  Final Score: 45.0 × 1.8 = 81.0

  Effect: Prevents one complex function from dominating entire priority list
```

#### Why Two Stages?

The separation of coverage weight adjustment and role multiplier ensures they work together without interfering:

**Stage 1 (Coverage Weight)**: Adjusts testing expectations
- **Question**: "How much should we penalize missing unit tests for this type of function?"
- **Example**: Entry points get 60% of normal coverage penalty (they're integration tested)

**Stage 2 (Role Multiplier)**: Adjusts architectural importance
- **Question**: "How important is this function relative to others with similar complexity?"
- **Example**: Critical entry points might get a 1.2x multiplier (clamped), while simple I/O wrappers get 0.5x (clamped)

**Independent Contributions**:
```
1. Calculate base score from complexity + dependencies
2. Apply coverage weight by role → adjusted coverage penalty
3. Combine into preliminary score
4. Apply clamped role multiplier → final score
```

This approach ensures:
- Coverage adjustments don't interfere with role multiplier
- Both mechanisms contribute independently
- Clamping prevents instability from extreme multipliers

#### How This Reduces False Positives

**False Positive #1: Entry Points Flagged for Low Coverage**

**Before**:
```
Top Priority Items:
1. main() - Score: 9.2 (0% unit test coverage)
2. handle_cli_command() - Score: 8.8 (5% unit test coverage)
3. run_server() - Score: 8.5 (0% unit test coverage)
```

**After**:
```
Top Priority Items:
1. calculate_tax() - Score: 9.8 (0% coverage, Pure Logic)
2. validate_payment() - Score: 9.2 (10% coverage, Pure Logic)
3. main() - Score: 4.2 (0% coverage, Entry Point - integration tested)
```

**Result**: Business logic functions that actually need unit tests rise to the top.

**False Positive #2: I/O Wrappers Over-Prioritized**

**Before**:
```
Function: read_config_file
  Complexity: 3
  Coverage: 0%
  Score: 7.5 (high priority)

  Issue: This is a thin wrapper over std::fs::read_to_string.
  Unit testing it provides minimal value vs integration tests.
```

**After**:
```
Function: read_config_file
  Complexity: 3
  Coverage: 0%
  Adjusted Coverage Weight: 0.7
  Score: 3.2 (low priority)

  Rationale: I/O wrappers are integration tested.
  Focus on business logic instead.
```

#### Configuration Examples

**Emphasize Pure Logic Testing**:
```toml
[scoring.role_coverage_weights]
pure_logic = 1.5        # Strong penalty for untested pure logic
entry_point = 0.5       # Minimal penalty for untested entry points
io_wrapper = 0.5        # Minimal penalty for untested I/O wrappers
```

**Conservative Approach (Smaller Adjustments)**:
```toml
[scoring.role_coverage_weights]
pure_logic = 1.1        # Slight increase
entry_point = 0.9       # Slight decrease
io_wrapper = 0.9        # Slight decrease
```

**Disable Multiplier Clamping** (not recommended for production):
```toml
[scoring.role_multiplier]
enable_clamping = false   # Allow unclamped multipliers
# Warning: May cause unstable prioritization
```

#### Verification

To see how role-based adjustments affect your codebase:

```bash
# Show detailed scoring breakdown
debtmap analyze . --verbose

# Compare with role adjustments disabled
debtmap analyze . --config minimal.toml
```

**Sample verbose output**:
```
Function: src/handlers/request.rs:handle_request
  Role: Entry Point
  Complexity: 5
  Coverage: 0%
  Coverage Weight: 0.6 (Entry Point adjustment)
  Adjusted Coverage Penalty: 0.4 (reduced from 1.0)
  Base Score: 15.0
  Role Multiplier: 1.2 (clamped from 1.5)
  Final Score: 18.0

  Interpretation:
    - Entry point gets 60% coverage penalty instead of 100%
    - Likely tested via integration tests
    - Still flagged due to complexity, but not over-penalized for coverage
```

#### Benefits Summary

- **Fewer false positives**: Entry points and I/O wrappers no longer dominate priority lists
- **Better resource allocation**: Testing efforts focus on pure logic where unit tests provide most value
- **Recognition of testing strategies**: Integration tests are valued equally with unit tests
- **Stable prioritization**: Clamping prevents extreme multipliers from causing volatile rankings
- **Configurable**: Adjust weights and clamp ranges to match your project's testing philosophy

### Use Cases

**1. Identifying Specific Hot Spots**

```bash
# Show top 20 functions needing attention
debtmap analyze . --top 20
```

Use when:
- Planning individual developer tasks
- Assigning specific refactoring work
- Identifying functions to test first
- Code review focus

**2. Sprint Planning for Developers**

```bash
# Get function-level tasks for this sprint
debtmap analyze . --top 10 --format json -o sprint-tasks.json
```

**3. Writing Unit Tests**

```bash
# Find untested complex functions
debtmap analyze . --lcov coverage.lcov --filter Testing --top 15
```

**4. Targeted Performance Optimization**

```bash
# Find complex hot paths
debtmap analyze . --filter Performance --context --top 10
```

### Configuration

Complete configuration file example showing all scoring-related sections.

**File name**: `.debtmap.toml` (must be placed in your project root)

```toml
# .debtmap.toml - Complete scoring configuration

# Role multipliers (applied to final score after coverage multiplier)
[role_multipliers]
pure_logic = 1.2             # Core business rules and algorithms
unknown = 1.0                # Functions without clear classification
entry_point = 0.9            # Public APIs, main functions, HTTP handlers
orchestrator = 0.8           # Functions that coordinate other functions
io_wrapper = 0.7             # File/network I/O wrappers
pattern_match = 0.6          # Functions primarily doing pattern matching

# Aggregation settings (for file-level scoring)
[aggregation]
method = "weighted_sum"      # Options: weighted_sum, sum, logarithmic_sum, max_plus_average
min_problematic = 3          # Minimum number of problematic functions to report file

# Normalization settings (for advanced multi-phase normalization)
[normalization]
linear_threshold = 10.0       # Scores below this use linear scaling (1:1 mapping)
logarithmic_threshold = 100.0 # Scores above this use logarithmic dampening
sqrt_multiplier = 3.33        # Applied to scores between linear and log thresholds
log_multiplier = 10.0         # Applied to scores above logarithmic threshold
show_raw_scores = true        # Display both normalized (0-100) and raw scores in output
```

**Note on Scoring Weights**: The base complexity and dependency weights are hard-coded for consistency across environments. However, you can customize prioritization significantly through configurable options:

**What's Configurable:**
- `role_multipliers` - Adjust importance of different function types (pure logic, entry points, I/O wrappers)
- `coverage_weights` - Role-specific coverage penalty adjustments
- `normalization` settings - Control score scaling and range
- `aggregation.method` - Choose how function scores combine into file scores

**What's Hard-Coded:**
- Base complexity weight (50%) and dependency weight (25%)
- Coverage multiplier formula: `1.0 - coverage_percent`

**Impact**: While base weights are fixed, the configurable multipliers and weights provide significant control over final rankings and priorities. A function with `role_multiplier = 1.5` and `coverage_weight = 1.2` can have 80% higher priority than the same function with default settings.

**Note**: The configuration file must be named `.debtmap.toml` (not `debtmap.yml` or other variants) and placed in your project root directory.

## When to Use Each Approach

### Use File-Level Scoring When:

✅ Planning architectural refactoring
✅ Quarterly or annual planning
✅ Deciding which modules to split
✅ Executive summaries and high-level reports
✅ Team capacity planning
✅ Identifying god objects
✅ Module reorganization

**Command:**
```bash
debtmap analyze . --aggregate-only
```

### Use Function-Level Scoring When:

✅ Sprint planning
✅ Individual developer task assignment
✅ Writing specific unit tests
✅ Code review preparation
✅ Pair programming sessions
✅ Daily or weekly development work
✅ Targeted hot spot fixes

**Command:**
```bash
debtmap analyze . --top 20
```

### Use Both Together:

Many workflows benefit from both views:

```bash
# Step 1: Identify problematic files
debtmap analyze . --aggregate-only --top 5 -o files.json

# Step 2: Drill into specific file
debtmap analyze src/problematic/module.rs --format terminal
```

## Comparison Examples

### Example 1: God Object Detection

**Command**:
```bash
debtmap analyze src/services/user_service.rs --aggregate-only
```

**File-Level View:**
```
src/services/user_service.rs - Score: 245.8
  - 850 lines, 45 methods
  - God Object: 78% score
  - Action: Split into UserAuth, UserProfile, UserNotifications
```

**Command**:
```bash
debtmap analyze src/services/user_service.rs --top 5
```

**Function-Level View:**
```
src/services/user_service.rs:142 - authenticate_user() - Score: 8.5
src/services/user_service.rs:298 - update_profile() - Score: 7.2
src/services/user_service.rs:456 - send_notification() - Score: 6.8
```

**Decision**: File-level score (245.8) correctly identifies architectural issue. Individual functions aren't exceptionally complex, but the file has too many responsibilities. **Solution**: Split the file.

### Example 2: Targeted Function Fix

**Command**:
```bash
debtmap analyze src/parsers/expression.rs --aggregate-only
```

**File-Level View:**
```
src/parsers/expression.rs - Score: 45.2
  - 320 lines, 12 functions
  - No god object detected
```

**Command**:
```bash
debtmap analyze src/parsers/expression.rs --top 5
```

**Function-Level View:**
```
src/parsers/expression.rs:89 - parse_complex_expression() - Score: 9.1
  - Cyclomatic: 22, Cognitive: 35
  - Coverage: 0%
  - Action: Add tests and refactor
```

**Decision**: File as a whole is acceptable, but one function needs attention. **Solution**: Focus on that specific function.

### Example 3: Balanced Refactoring

**Command**:
```bash
debtmap analyze src/analysis/scoring.rs --aggregate-only --coverage-file coverage.lcov
```

**File-Level View:**
```
src/analysis/scoring.rs - Score: 125.6
  - 580 lines, 18 functions
  - High complexity, low coverage
```

**Command**:
```bash
debtmap analyze src/analysis/scoring.rs --coverage-file coverage.lcov --top 5
```

**Function-Level View:**
```
calculate_score() - Score: 8.8 (15% coverage)
apply_weights() - Score: 8.2 (10% coverage)
normalize_results() - Score: 7.5 (0% coverage)
```

**Decision**: Both file and functions need work. **Solution**: Add tests first (function-level), then consider splitting if complexity persists (file-level).

## Score Normalization

Both scoring approaches normalize to a 0-10 scale for consistency.

### Normalization Strategies

**Default: Linear Clamping**

The default normalization uses simple linear clamping to the 0-100 range:

- **Formula**: Score is clamped between 0.0 and 100.0
- **Behavior**: No transformation, just boundary enforcement
- **Usage**: Production output uses this method

This ensures scores stay within the expected range without additional transformations.

**Advanced: Multi-Phase Normalization**

For more sophisticated normalization, debtmap provides multi-phase scaling with different formulas for different score ranges:

**Phase 1 - Linear (scores < 10)**:
- Formula: `normalized = raw_score`
- Behavior: 1:1 mapping, no scaling
- Rationale: Preserve low score distinctions

**Phase 2 - Square Root (scores 10-100)**:
- Formula: `normalized = 10.0 + sqrt(raw_score - 10.0) × 3.33`
- Behavior: Moderate dampening
- Rationale: Balance between linear and logarithmic

**Phase 3 - Logarithmic (scores > 100)**:
- Formula: `normalized = 41.59 + ln(raw_score / 100.0) × 10.0`
- Behavior: Strong dampening of extreme values
- Rationale: Prevent outliers from dominating

This multi-phase approach dampens extreme values while preserving distinctions in the normal range. Configure via `[normalization]` section in `.debtmap.toml`.

### Configuration

```toml
[normalization]
linear_threshold = 10.0       # Scores below this use linear scaling (1:1 mapping)
logarithmic_threshold = 100.0 # Scores above this use logarithmic dampening
sqrt_multiplier = 3.33        # Applied to scores between linear and log thresholds
log_multiplier = 10.0         # Applied to scores above logarithmic threshold
show_raw_scores = true        # Display both normalized (0-10) and raw scores in output
```

**Explanation**:
- **linear_threshold**: Scores below this value are mapped 1:1 (no scaling)
- **logarithmic_threshold**: Scores above this value are dampened logarithmically to prevent extreme values
- **sqrt_multiplier**: Square root scaling applied to mid-range scores (between linear and logarithmic thresholds)
- **log_multiplier**: Logarithmic dampening factor for very high scores
- **show_raw_scores**: When enabled, output includes both the normalized 0-10 score and the raw calculated score

## Best Practices

### Workflow Integration

**Week 1: File-Level Assessment**
```bash
# Identify architectural problems
debtmap analyze . --aggregate-only --top 10
```

**Week 2-4: Function-Level Work**
```bash
# Work through specific functions
debtmap analyze src/target/module.rs
```

**Monthly: Compare Progress**
```bash
debtmap compare --before baseline.json --after current.json
```

### Team Collaboration

- **Architects**: Use file-level scores for strategic planning
- **Tech Leads**: Use both for sprint planning
- **Developers**: Use function-level for daily work
- **QA**: Use function-level for test prioritization

### CI/CD Integration

```bash
# Gate: No new file-level regressions
debtmap analyze . --aggregate-only --format json -o file-scores.json

# Gate: No new critical function-level issues
debtmap analyze . --min-priority critical --format json -o critical-items.json
```

## Troubleshooting

**Issue**: File-level scores seem too high

**Solution**: Check aggregation method:
```toml
[aggregation]
method = "logarithmic_sum"  # Dampen scores
```

**Issue**: Function-level scores all similar

**Solution**: Adjust role multipliers to create more differentiation:
```toml
[role_multipliers]
pure_logic = 1.5     # Emphasize business logic more
io_wrapper = 0.5     # De-emphasize I/O wrappers more
```

**Note**: Base scoring weights (complexity 50%, dependency 25%) are hard-coded and cannot be configured.

**Issue**: Too many low-priority items

**Solution**: Use minimum thresholds:
```toml
[thresholds]
minimum_debt_score = 3.0
```

## Rebalanced Debt Scoring (Spec 136)

Debtmap now includes an advanced **rebalanced scoring algorithm** that prioritizes actual code quality issues—complexity, coverage gaps, and structural problems—over pure file size concerns.

### Enabling Rebalanced Scoring

> **IMPORTANT**: Rebalanced scoring is enabled through your `.debtmap.toml` configuration file, **not via CLI flags**. Add the `[scoring_rebalanced]` section to activate it.

**Default Behavior**: By default, debtmap uses the standard scoring algorithm described earlier in this chapter. To use rebalanced scoring, add the `[scoring_rebalanced]` section to your config:

```toml
# .debtmap.toml
[scoring_rebalanced]
preset = "balanced"  # Activates rebalanced scoring with balanced preset
```

**Relationship to Standard Scoring**:
- Rebalanced scoring **supplements** standard scoring, providing an alternative prioritization strategy
- Both algorithms can coexist - choose which to use based on your needs
- File-level and function-level scoring both work with rebalanced scoring
- Output format remains the same, only score calculations differ

**Migration Path**:
1. **Test first**: Add `[scoring_rebalanced]` section to a test config file
2. **Compare**: Run analysis with both standard and rebalanced scoring on same codebase
3. **Evaluate**: Review how priorities change (large simple files rank lower, complex untested code ranks higher)
4. **Adopt**: Once satisfied, switch your primary config to use rebalanced scoring
5. **Tune**: Adjust preset or custom weights based on your team's priorities

**Quick Start**:
```bash
# Create test config with rebalanced scoring
cat > .debtmap-rebalanced.toml <<EOF
[scoring_rebalanced]
preset = "balanced"
EOF

# Compare results
debtmap analyze . --format terminal                            # Standard scoring
debtmap analyze . --config .debtmap-rebalanced.toml --format terminal  # Rebalanced scoring
```

### Philosophy

Traditional scoring often over-emphasizes file size, causing large but simple files to rank higher than complex, untested code. The rebalanced algorithm fixes this by:

1. **De-emphasizing size**: Reduces size weight from ~1.5 to 0.3 (80% reduction)
2. **Emphasizing quality**: Increases weights for complexity (1.0) and coverage gaps (1.0)
3. **Additive bonuses**: Provides +20 bonus for complex + untested code (not multiplicative)
4. **Context-aware thresholds**: Integrates with file type classification from Spec 135

### Multi-Dimensional Scoring

The rebalanced algorithm computes five scoring components:

| Component | Weight | Range | Description |
|-----------|--------|-------|-------------|
| **Complexity** | 1.0 | 0-100 | Cyclomatic + cognitive complexity |
| **Coverage Gap** | 1.0 | 0-80 | Testing coverage deficit with complexity bonus |
| **Structural** | 0.8 | 0-60 | God objects and architectural issues |
| **Size** | 0.3 | 0-30 | File size (reduced from previous ~1.5) |
| **Code Smells** | 0.6 | 0-40 | Long functions, deep nesting, impure logic |

**Weighted Total Formula**:
```
weighted_total = (complexity × 1.0) + (coverage × 1.0) + (structural × 0.8)
                 + (size × 0.3) + (smells × 0.6)

normalized_score = (weighted_total / 237.0) × 200.0  // Normalize to 0-200 range
```

### Scoring Presets

Debtmap provides four presets for different prioritization strategies:

#### Balanced (Default)
```toml
[scoring_rebalanced]
preset = "balanced"
```

Weights:
- Complexity: 1.0, Coverage: 1.0, Structural: 0.8, Size: 0.3, Smells: 0.6

**Use when**: Standard development with focus on actual code quality

#### Quality-Focused
```toml
[scoring_rebalanced]
preset = "quality-focused"
```

Weights:
- Complexity: 1.2, Coverage: 1.1, Structural: 0.9, Size: 0.2, Smells: 0.7

**Use when**: Maximum emphasis on code quality, minimal concern for file size

#### Test-Coverage-Focused
```toml
[scoring_rebalanced]
preset = "test-coverage"
```

Weights:
- Complexity: 0.8, Coverage: 1.3, Structural: 0.6, Size: 0.2, Smells: 0.5

**Use when**: Prioritizing test coverage improvements

#### Size-Focused (Legacy)
```toml
[scoring_rebalanced]
preset = "size-focused"
```

Weights:
- Complexity: 0.5, Coverage: 0.4, Structural: 0.6, Size: 1.5, Smells: 0.3

**Use when**: Maintaining legacy scoring behavior, file size is primary concern

### Custom Weights

You can define custom weights in `.debtmap.toml`:

```toml
[scoring_rebalanced]
complexity_weight = 1.2
coverage_weight = 1.0
structural_weight = 0.8
size_weight = 0.2
smell_weight = 0.7
```

### Severity Levels

The rebalanced algorithm assigns severity based on normalized score and risk factors:

| Severity | Criteria | Description |
|----------|----------|-------------|
| **CRITICAL** | Score > 120 OR (complexity > 60 AND coverage > 40) | Requires immediate attention |
| **HIGH** | Score > 80 OR (complexity > 40 AND coverage > 20) OR structural > 50 | High priority for next sprint |
| **MEDIUM** | Score > 40 OR single moderate issue | Plan for future sprint |
| **LOW** | Everything else | Minor concerns, size-only issues |

**Evaluation Logic**: Severity is assigned based on the **first matching criteria** (logical OR). An item needs to satisfy **only ONE condition** to qualify for that severity level. For example, a function with score=90 is HIGH severity even if complexity and coverage are both low, because it meets the "Score > 80" condition.

### Example Prioritization

**Complex Untested Function** (HIGH priority):
```rust
fn process_payment(cart: &Cart, user: &User) -> Result<Receipt> {
    // 150 lines, cyclomatic: 42, cognitive: 77
    // Coverage: 38%

    // Rebalanced Score:
    // - Complexity: 100.0 (very high)
    // - Coverage: 57.2 (gap × 0.6 + 20 bonus for complex+untested)
    // - Structural: 0.0
    // - Size: 0.0 (function-level scoring)
    // - Smells: 25.0 (long function)
    // Total: 95.3 → CRITICAL severity
}
```

**Large Simple Function** (LOW priority):
```rust
fn format_report(data: &ReportData) -> String {
    // 2000 lines, cyclomatic: 3, cognitive: 5
    // Coverage: 100%

    // Rebalanced Score:
    // - Complexity: 0.0 (trivial)
    // - Coverage: 0.0 (well tested)
    // - Structural: 0.0
    // - Size: 0.0 (function-level scoring)
    // - Smells: 15.0 (long but simple)
    // Total: 3.2 → LOW severity
}
```

**Result**: Complex untested code ranks 30× higher than large simple code.

### Integration with File Classification (Spec 135)

The rebalanced scoring integrates with context-aware file size thresholds:

```rust
use debtmap::organization::file_classifier::{classify_file, get_threshold};

let file_type = classify_file(source, path);
let threshold = get_threshold(&file_type, function_count, lines);

// Apply context-aware scoring:
// - Generated code: 0.1× size multiplier
// - Test code: Lenient thresholds (650 lines)
// - Business logic: Strict thresholds (400 lines)
```

### Generated Code Detection

The rebalanced scoring automatically detects and reduces scores for generated code:

**Detection Markers** (first 20 lines):
- "DO NOT EDIT"
- "automatically generated"
- "AUTO-GENERATED"
- "@generated"
- "Code generated by"

**Generated Code Score Adjustment**:
```rust
if is_generated_code(source) {
    size_score *= 0.1;  // 90% reduction
}
```

### Scoring Rationale

Each debt item includes a detailed rationale explaining the score:

```
Debt Item: src/payment/processor.rs:142 - process_payment()
Score: 95.3 (CRITICAL)

Primary factors:
  - High cyclomatic complexity (+100.0)
  - Significant coverage gap (+57.2)

Bonuses:
  - Complex + untested: +20 bonus applied
  - Code smells detected (+25.0)

Context adjustments:
  - Size de-emphasized (weight: 0.3)
```

### Migration from Legacy Scoring

**Breaking Changes**:
- Scores will change significantly for all debt items
- Large files with low complexity will rank lower
- Complex untested code will rank higher
- Size-based prioritization reduced by 80%

**Restoring Legacy Behavior**:
```toml
[scoring_rebalanced]
preset = "size-focused"
```

**Gradual Migration**:
1. Run analysis with both algorithms: `debtmap analyze . --legacy-scoring`
2. Compare results to understand impact
3. Adjust team priorities based on new rankings
4. Switch to rebalanced scoring after validation

See [Migration Guide](./migration-guide.md) for detailed migration instructions.

### Configuration Reference

Complete configuration example:

```toml
# .debtmap.toml

[scoring_rebalanced]
# Use a preset (balanced, quality-focused, test-coverage, size-focused)
preset = "balanced"

# Or define custom weights
complexity_weight = 1.0
coverage_weight = 1.0
structural_weight = 0.8
size_weight = 0.3
smell_weight = 0.6
```

### When to Use Rebalanced Scoring

✅ **Use rebalanced scoring when**:
- You want to prioritize code quality over file size
- Complex untested code is a concern
- You're building new features and need quality focus
- Your team values testability and maintainability

❌ **Use legacy/size-focused when**:
- You're managing a legacy codebase with large files
- File size reduction is the primary concern
- You need compatibility with existing workflows
- Your team's priority is file splitting over quality

### Performance

The rebalanced scoring algorithm has minimal performance impact:
- Same O(n) complexity as legacy scoring
- No additional file I/O required
- Parallel processing compatible
- Adds ~5% to analysis time for rationale generation

## Score-Based Prioritization with Exponential Scaling (Spec 171)

DebtMap uses exponential scaling and risk boosting to amplify high-severity technical debt items, ensuring critical issues stand out clearly in priority lists. This section explains how these mechanisms work and how to configure them for your project.

### Why Exponential Scaling?

Traditional linear multipliers create uniform gaps between scores:
- Linear 2x multiplier: Score 50 → 100, Score 100 → 200 (uniform +50 and +100 gaps)

Exponential scaling creates growing gaps that make critical issues impossible to miss:
- Exponential scaling (^1.4): Score 50 → 279, Score 100 → 1000 (gaps grow dramatically)

**Key Benefits**:
- **Visual Separation**: Critical items have dramatically higher scores than medium items
- **Natural Clustering**: Similar-severity items cluster together in ranked lists
- **Actionable Ordering**: Work through the list from top to bottom with confidence
- **No Arbitrary Thresholds**: Pure score-based ranking eliminates debates about tier boundaries

### How Exponential Scaling Works

After calculating the base score (complexity + coverage + dependencies), DebtMap applies pattern-specific exponential scaling:

**Formula**:
```
scaled_score = base_score ^ exponent
```

**Pattern-Specific Exponents** (configurable in `.debtmap.toml`):

| Pattern Type | Default Exponent | Rationale |
|--------------|------------------|-----------|
| God Objects | 1.4 | Highest amplification - architectural issues deserve top priority |
| Long Functions | 1.3 | High amplification - major refactoring candidates |
| Complex Functions | 1.2 | Moderate amplification - complexity issues |
| Primitive Obsession | 1.1 | Light amplification - design smell but lower urgency |

### Example: God Object Scaling (exponent = 1.4)

Comparing three God Objects with different base scores:

| Base Score | Calculation | Scaled Score | Amplification |
|------------|-------------|--------------|---------------|
| 10 | 10^1.4 | 25.1 | 2.5x |
| 50 | 50^1.4 | 279.5 | 5.6x |
| 100 | 100^1.4 | 1000.0 | 10x |

**Result**: The highest-severity God Object (score 100) gets 10x amplification, while a minor issue (score 10) only gets 2.5x. This creates clear visual separation in your priority list.

### Risk Boosting

After exponential scaling, DebtMap applies additional risk multipliers based on architectural position:

**Risk Multipliers** (applied multiplicatively):

```rust
final_score = scaled_score × risk_multiplier
```

| Risk Factor | Multiplier | Rationale |
|-------------|-----------|-----------|
| High dependency count (10+ callers) | 1.2x | Harder to refactor safely, affects more code |
| Entry point (main, CLI handlers, routes) | 1.15x | Failures cascade to all downstream code |
| Low test coverage (<30%) | 1.1x | Riskier to modify without tests |

**Example**:
```
Function: process_payment (God Object)
  Base Score: 85.0
  Exponentially Scaled: 85^1.4 = 554.3
  Risk Factors:
    - Entry point: ×1.15
    - Low coverage (15%): ×1.1
  Final Score: 554.3 × 1.15 × 1.1 = 701.7
```

### Complete Scoring Pipeline

DebtMap processes scores through multiple stages:

```
1. Base Score Calculation
   ↓
   Weighted sum of:
   - Coverage factor (40% weight)
   - Complexity factor (40% weight)
   - Dependency factor (20% weight)

2. Exponential Scaling
   ↓
   Pattern-specific exponent applied:
   - God Objects: ^1.4
   - Long Functions: ^1.3
   - etc.

3. Risk Boosting
   ↓
   Architectural position multipliers:
   - High dependencies: ×1.2
   - Entry points: ×1.15
   - Low coverage: ×1.1

4. Final Score
   ↓
   Used for ranking (no tier bucketing)

5. Output
   ↓
   Sorted descending by final score
```

### Configuration

You can customize exponential scaling parameters in `.debtmap.toml`:

```toml
[priority.scaling.god_object]
exponent = 1.5              # Increase amplification for God Objects
min_threshold = 30.0        # Only scale scores above 30
max_threshold = 500.0       # Cap scaled scores at 500

[priority.scaling.long_function]
exponent = 1.3              # Default amplification
min_threshold = 0.0         # No minimum threshold
max_threshold = 1000.0      # High cap for extreme cases

[priority.scaling.complex_function]
exponent = 1.2              # Moderate amplification
min_threshold = 20.0        # Scale scores above 20
max_threshold = 800.0       # Cap at 800
```

**Configuration Parameters**:
- **exponent**: The exponential scaling factor (higher = more amplification)
- **min_threshold**: Minimum base score to apply scaling (prevents amplifying trivial issues)
- **max_threshold**: Maximum scaled score (prevents extreme outliers)

### Tuning Guidelines

**Increase amplification when**:
- Critical issues aren't standing out enough in your priority list
- Team needs stronger signal about what to tackle first
- You have many medium-severity items obscuring high-severity ones

**Decrease amplification when**:
- Priority list feels too top-heavy (too many "critical" items)
- Scores are getting too large (e.g., thousands)
- You want more gradual transitions between severity levels

**Example: More Aggressive God Object Detection**
```toml
[priority.scaling.god_object]
exponent = 1.6              # Higher amplification
min_threshold = 20.0        # Start scaling earlier
max_threshold = 2000.0      # Allow higher caps
```

### Comparing With vs Without Exponential Scaling

**Without Exponential Scaling (Linear Multipliers)**:
```
Priority List:
1. God Object (base: 85) → final: 170 (2x multiplier)
2. Long Function (base: 80) → final: 160 (2x multiplier)
3. Complex Function (base: 75) → final: 150 (2x multiplier)
4. Medium Issue (base: 70) → final: 140 (2x multiplier)
```
**Problem**: Gaps are uniform (10 points). Hard to distinguish critical from medium issues.

**With Exponential Scaling**:
```
Priority List:
1. God Object (base: 85) → scaled: 554 → with risk: 701
2. Long Function (base: 80) → scaled: 447 → with risk: 492
3. Complex Function (base: 75) → scaled: 357 → with risk: 357
4. Medium Issue (base: 70) → scaled: 282 → with risk: 282
```
**Result**: Clear separation. God Object stands out as 2.5x higher than medium issues.

### Score-Based Ranking vs Tier-Based Ranking

DebtMap uses pure score-based ranking (not tier-based) for finer granularity:

**Traditional Tier-Based Ranking**:
```
Critical: Items with score ≥ 200
High: Items with score 100-199
Medium: Items with score 50-99
Low: Items with score < 50
```
**Problem**: All "Critical" items look equally important, even if one has score 201 and another has score 1000.

**Score-Based Ranking**:
```
1. process_payment - Score: 1247.3
2. UserService.authenticate - Score: 891.2
3. calculate_tax - Score: 654.1
...
```
**Benefits**:
- Every item has a unique priority position
- Natural ordering - work from highest to lowest
- No arbitrary boundaries or threshold debates
- Finer-grained decision making

**Compatibility Note**: For tools expecting Priority enums, scores can be mapped to tiers:
- Score ≥ 200: Critical
- Score ≥ 100: High
- Score ≥ 50: Medium
- Score < 50: Low

However, the primary output uses raw scores for maximum granularity.

### Practical Examples

**Example 1: Identifying Architectural Hot Spots**

```bash
debtmap analyze . --top 10
```

**Output**:
```
Top 10 Technical Debt Items (Sorted by Score)

1. src/services/user_service.rs:45 - UserService::authenticate
   Score: 1247.3 | Pattern: God Object | Coverage: 12%
   → 45 methods, 892 lines, high complexity
   → Risk factors: Entry point (×1.15), High dependencies (×1.2)

2. src/payment/processor.rs:142 - process_payment
   Score: 891.2 | Pattern: Complex Function | Coverage: 8%
   → Cyclomatic: 42, Cognitive: 77
   → Risk factors: Entry point (×1.15), Low coverage (×1.1)

3. src/reporting/generator.rs:234 - generate_monthly_report
   Score: 654.1 | Pattern: Long Function | Coverage: 45%
   → 287 lines, moderate complexity
   → Risk factors: High dependencies (×1.2)
```

**Action**: Focus on top 3 items first - they have dramatically higher scores than items 4-10.

**Example 2: Monitoring Exponential Scaling Impact**

```bash
# Analyze with verbose output to see scaling details
debtmap analyze . --verbose --top 5
```

**Verbose Output**:
```
Function: src/services/user_service.rs:45 - UserService::authenticate
  Base Score: 85.0
  Pattern: God Object
  Exponential Scaling (^1.4): 85.0^1.4 = 554.3
  Risk Boosting:
    - Entry point: ×1.15 → 637.4
    - High dependencies (15 callers): ×1.2 → 764.9
    - Low coverage (12%): ×1.1 → 841.4
  Final Score: 841.4
```

**Insight**: Base score of 85 amplified to 841 through exponential scaling and risk boosting - a 9.9x total amplification.

### When to Use Exponential Scaling

✅ **Use exponential scaling when**:
- You need clear visual separation between critical and medium issues
- Your priority list has too many "high priority" items
- You want top issues to stand out dramatically
- You prefer score-based ranking over tier-based bucketing

✅ **Adjust exponents when**:
- Default amplification doesn't match your team's priorities
- Certain patterns (e.g., God Objects) deserve more/less emphasis
- You're tuning the balance between different debt types

✅ **Tune thresholds when**:
- Scores are getting too large (increase max_threshold)
- Trivial issues are being amplified (increase min_threshold)
- You want to cap extreme outliers (adjust max_threshold)

### Performance Impact

Exponential scaling has negligible performance impact:
- **Computation**: Simple `powf()` operation per item
- **Overhead**: <1% additional analysis time
- **Scalability**: Works with parallel processing (no synchronization needed)
- **Memory**: No additional data structures required

### See Also

- [Tiered Prioritization](./tiered-prioritization.md) - Understanding tier-based classification
- [Configuration](./configuration.md) - Scoring and aggregation configuration
- [Analysis Guide](./analysis-guide.md) - Detailed metric explanations
- [File Classification](./file-classification.md) - Context-aware file size thresholds (Spec 135)
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Technical details of exponential scaling implementation

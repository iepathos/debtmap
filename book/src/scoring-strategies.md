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
File Score = Size × Complexity × Coverage Multiplier × Density × GodObject × FunctionScores
```

Where each factor is calculated as:
- **Size** = `sqrt(total_lines / 100)`
- **Complexity** = `(avg_complexity / 5.0) × sqrt(total_complexity / 50.0)`
- **Coverage Multiplier** = `1.0 - coverage_percent`
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

**Coverage Multiplier**: `1.0 - coverage_percent`
- Lower coverage increases score multiplicatively
- Range: 0.0 (100% coverage) to 1.0 (0% coverage)
- Rationale: Untested files amplify existing complexity and risk
- Note: Earlier versions used an additive "Coverage Factor" formula; current implementation uses multiplicative dampening

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

**Note**: File-level scoring is enabled with the `--aggregate-only` flag, which changes output to show only file-level metrics instead of function-level details.

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

### Configuration

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

**Note**: Coverage acts as a dampening multiplier rather than an additive factor. Lower coverage (higher multiplier) increases the final score, making untested complex code a higher priority. The weights (0.50 for complexity, 0.25 for dependencies) are hard-coded in the implementation.

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

**Note on Scoring Weights**: The base scoring weights for complexity (50%) and dependencies (25%) are hard-coded in the implementation and not configurable via the config file. Coverage is applied as a multiplicative dampener (1.0 - coverage_percent), not as an additive weight.

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

**File-Level View:**
```
src/services/user_service.rs - Score: 245.8
  - 850 lines, 45 methods
  - God Object: 78% score
  - Action: Split into UserAuth, UserProfile, UserNotifications
```

**Function-Level View:**
```
src/services/user_service.rs:142 - authenticate_user() - Score: 8.5
src/services/user_service.rs:298 - update_profile() - Score: 7.2
src/services/user_service.rs:456 - send_notification() - Score: 6.8
```

**Decision**: File-level score (245.8) correctly identifies architectural issue. Individual functions aren't exceptionally complex, but the file has too many responsibilities. **Solution**: Split the file.

### Example 2: Targeted Function Fix

**File-Level View:**
```
src/parsers/expression.rs - Score: 45.2
  - 320 lines, 12 functions
  - No god object detected
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

**File-Level View:**
```
src/analysis/scoring.rs - Score: 125.6
  - 580 lines, 18 functions
  - High complexity, low coverage
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

The default normalization (`normalize_final_score`) uses simple linear clamping:

```rust
score_normalized = raw_score.clamp(0.0, 100.0)
```

This ensures scores stay within the expected 0-100 range without additional transformations. This is the normalization method used in production output.

**Advanced: Multi-Phase Normalization**

For more sophisticated normalization, debtmap provides `normalize_final_score_with_metadata` which uses different scaling for different score ranges:

```rust
score_normalized = if raw_score < 10.0 {
    raw_score  // Linear below 10
} else if raw_score < 100.0 {
    10.0 + (raw_score - 10.0).sqrt() * 3.33  // Square root 10-100
} else {
    41.59 + (raw_score / 100.0).ln() * 10.0  // Logarithmic above 100
}
```

This multi-phase approach dampens extreme values while preserving distinctions in the normal range. Note that this advanced normalization is available but may not be used by default in all outputs.

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

## See Also

- [Tiered Prioritization](./tiered-prioritization.md) - Understanding tier-based classification
- [Configuration](./configuration.md) - Scoring and aggregation configuration
- [Analysis Guide](./analysis-guide.md) - Detailed metric explanations

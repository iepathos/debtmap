# Role-Based Adjustments

DebtMap uses a sophisticated two-stage role adjustment mechanism to ensure that scores accurately reflect both the testing strategy appropriate for each function type and the architectural importance of different roles.

## Why Role-Based Adjustments?

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

## Stage 1: Role-Based Coverage Weighting

The first stage adjusts coverage penalty expectations based on function role. This prevents functions that use different testing strategies from unfairly dominating the priority list.

### How It Works

For each function, DebtMap:
1. Detects the function's role (entry point, pure logic, I/O wrapper, etc.)
2. Applies a coverage weight multiplier based on that role
3. Reduces or increases the coverage penalty accordingly

### Default Coverage Weights

The `RoleCoverageWeights` struct (`src/config/scoring.rs:384-413`) defines these defaults:

| Function Role    | Coverage Weight | Impact on Scoring |
|------------------|-----------------|-------------------|
| Pure Logic       | 1.0             | Standard penalty (should have unit tests) |
| Pattern Match    | 1.0             | Standard penalty |
| Unknown          | 1.0             | Standard penalty |
| Orchestrator     | 0.8             | Reduced penalty (partially integration tested) |
| Entry Point      | 0.6             | Significantly reduced penalty (integration tested) |
| I/O Wrapper      | 0.5             | Reduced penalty (integration tested) |
| Debug            | 0.3             | Minimal penalty (low priority for testing) |

**Source**: Default values from `src/config/scoring.rs:429-455`

### Coverage Expectations by Role

DebtMap also defines role-specific coverage targets in `CoverageExpectations` (`src/priority/scoring/coverage_expectations.rs:107-133`):

| Role | Minimum | Target | Maximum |
|------|---------|--------|---------|
| Pure | 90% | 95% | 100% |
| Business Logic | 80% | 90% | 95% |
| Validation | 85% | 92% | 98% |
| State Management | 75% | 85% | 90% |
| Utilities | 75% | 85% | 95% |
| Error Handling | 70% | 80% | 90% |
| Orchestration | 65% | 75% | 85% |
| I/O Operations | 60% | 70% | 80% |
| Configuration | 60% | 70% | 80% |
| Initialization | 50% | 65% | 75% |
| Performance | 40% | 50% | 60% |
| Debug | 20% | 30% | 40% |

### Example Score Changes

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
  Adjusted Coverage Penalty: 1.0 (standard penalty)
  Score: 9.8 (critical priority)

  Rationale: Pure logic should have unit tests.
  This function needs immediate test coverage.
```

## Stage 2: Role Multiplier

The second stage applies a final role-based multiplier to reflect architectural importance. This multiplier is **clamped by default** to prevent extreme score swings.

### Role Multiplier Defaults

The `RoleMultipliers` struct (`src/config/scoring.rs:207-236`) defines these multipliers:

| Role | Multiplier | Impact |
|------|------------|--------|
| Pure Logic | 1.2 | +20% (prioritized for testing) |
| Unknown | 1.0 | No adjustment |
| Entry Point | 0.9 | -10% (integration tested) |
| Orchestrator | 0.8 | -20% (higher-level tests) |
| I/O Wrapper | 0.7 | -30% (often integration tested) |
| Pattern Match | 0.6 | -40% (less complex) |
| Debug | 0.3 | -70% (lowest priority) |

**Source**: Default values from `src/config/scoring.rs:307-333`

### Multiplier Clamping

The `RoleMultiplierConfig` (`src/config/scoring.rs:457-493`) controls clamping:

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
  Final Score: 45.0 x 1.8 = 81.0

  Effect: Prevents one complex function from dominating entire priority list
```

## Why Two Stages?

The separation of coverage weight adjustment and role multiplier ensures they work together without interfering:

**Stage 1 (Coverage Weight)**: Adjusts testing expectations
- **Question**: "How much should we penalize missing unit tests for this type of function?"
- **Example**: Entry points get 60% of normal coverage penalty (they're integration tested)

**Stage 2 (Role Multiplier)**: Adjusts architectural importance
- **Question**: "How important is this function relative to others with similar complexity?"
- **Example**: Pure logic gets a 1.2x multiplier, while debug functions get 0.3x

### Scoring Pipeline

The functional scoring pipeline (`src/priority/scoring/coverage_scoring.rs:20-31`):

```rust
pub fn calculate_coverage_score(
    actual_coverage: f64,
    role: &str,
    expectations: &CoverageExpectations,
) -> f64 {
    let range = expectations.for_role(role);
    let gap = CoverageGap::calculate(actual_coverage, range);

    calculate_gap_score(&gap)
        .pipe(|score| weight_by_severity(score, gap.severity))
        .pipe(|score| weight_by_role(score, role))
}
```

**Independent Contributions**:
1. Calculate base score from complexity + dependencies
2. Apply coverage weight by role -> adjusted coverage penalty
3. Combine into preliminary score
4. Apply clamped role multiplier -> final score

This approach ensures:
- Coverage adjustments don't interfere with role multiplier
- Both mechanisms contribute independently
- Clamping prevents instability from extreme multipliers

## How This Reduces False Positives

### False Positive #1: Entry Points Flagged for Low Coverage

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

### False Positive #2: I/O Wrappers Over-Prioritized

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
  Adjusted Coverage Weight: 0.5
  Role Multiplier: 0.7
  Score: 2.6 (low priority)

  Rationale: I/O wrappers are integration tested.
  Focus on business logic instead.
```

## Configuration Examples

### Emphasize Pure Logic Testing

```toml
[scoring.role_coverage_weights]
pure_logic = 1.5        # Strong penalty for untested pure logic
entry_point = 0.5       # Minimal penalty for untested entry points
io_wrapper = 0.4        # Minimal penalty for untested I/O wrappers
debug = 0.2             # Minimal penalty for debug code
```

### Conservative Approach (Smaller Adjustments)

```toml
[scoring.role_coverage_weights]
pure_logic = 1.1        # Slight increase
entry_point = 0.9       # Slight decrease
io_wrapper = 0.8        # Slight decrease
orchestrator = 0.9      # Slight decrease
```

### Disable Multiplier Clamping (Not Recommended)

```toml
[scoring.role_multiplier]
enable_clamping = false   # Allow unclamped multipliers
# Warning: May cause unstable prioritization
```

### Strict Clamping Range

```toml
[scoring.role_multiplier]
clamp_min = 0.5           # More conservative minimum
clamp_max = 1.5           # More conservative maximum
enable_clamping = true
```

## Verification

To see how role-based adjustments affect your codebase:

```bash
# Show detailed scoring breakdown
debtmap analyze . --verbose

# Compare with role adjustments disabled (using minimal config)
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
  Role Multiplier: 0.9 (clamped)
  Final Score: 13.5

  Interpretation:
    - Entry point gets 60% coverage penalty instead of 100%
    - Likely tested via integration tests
    - Still flagged due to complexity, but not over-penalized for coverage
```

## Benefits Summary

- **Fewer false positives**: Entry points and I/O wrappers no longer dominate priority lists
- **Better resource allocation**: Testing efforts focus on pure logic where unit tests provide most value
- **Recognition of testing strategies**: Integration tests are valued equally with unit tests
- **Stable prioritization**: Clamping prevents extreme multipliers from causing volatile rankings
- **Configurable**: Adjust weights and clamp ranges to match your project's testing philosophy

## See Also

- [Semantic Classification](../semantic-classification.md) - How roles are detected
- [File-Level Scoring](file-level.md) - Aggregating function scores to file level
- [Function-Level Scoring](function-level.md) - Detailed function scoring mechanics
- [Coverage Integration](../coverage-integration.md) - How coverage data is integrated

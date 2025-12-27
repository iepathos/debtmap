# Scoring Configuration

Debtmap uses a weighted scoring model to calculate technical debt priority. This chapter explains how to configure scoring weights, role multipliers, and related settings that affect how functions and files are prioritized.

## Quick Reference

Here's a quick overview of all scoring defaults (from `src/config/scoring.rs`):

| Configuration | Default Value | Purpose |
|---------------|---------------|---------|
| **Scoring Weights** | | |
| `coverage` | 0.50 | Weight for test coverage gaps |
| `complexity` | 0.35 | Weight for code complexity |
| `dependency` | 0.15 | Weight for dependency criticality |
| **Role Multipliers** | | |
| `pure_logic` | 1.2 | Prioritize pure computation |
| `orchestrator` | 0.8 | Reduce for delegation functions |
| `io_wrapper` | 0.7 | Reduce for I/O wrappers |
| `entry_point` | 0.9 | Slight reduction for main/CLI |
| `pattern_match` | 0.6 | Reduce for pattern matching |
| `debug` | 0.3 | Debug/diagnostic functions |
| `unknown` | 1.0 | No adjustment |
| **Role Coverage Weights** | | |
| `entry_point` | 0.6 | Reduce coverage penalty |
| `orchestrator` | 0.8 | Reduce coverage penalty |
| `pure_logic` | 1.0 | No reduction |
| `io_wrapper` | 0.5 | Reduce for I/O wrappers |
| `pattern_match` | 1.0 | Standard penalty |
| `debug` | 0.3 | Lowest coverage expectations |
| `unknown` | 1.0 | Standard penalty |
| **Role Multiplier Clamping** | | |
| `clamp_min` | 0.3 | Minimum multiplier |
| `clamp_max` | 1.8 | Maximum multiplier |
| `enable_clamping` | true | Enable clamping |

## Scoring Weights

The `[scoring]` section controls how different factors contribute to the overall debt score. Debtmap uses a **weighted sum model** where weights must sum to 1.0.

```toml
[scoring]
coverage = 0.50      # Weight for test coverage gaps (default: 0.50)
complexity = 0.35    # Weight for code complexity (default: 0.35)
dependency = 0.15    # Weight for dependency criticality (default: 0.15)
```

**Active weights** (used in scoring):
- `coverage` - Prioritizes untested code (default: 0.50)
- `complexity` - Identifies complex areas (default: 0.35)
- `dependency` - Considers impact radius (default: 0.15)

**Unused weights** (reserved for future features):
- `semantic` - Not currently used (default: 0.00)
- `security` - Not currently used (default: 0.00)
- `organization` - Not currently used (default: 0.00)

**Validation rules:**
- All weights must be between 0.0 and 1.0
- Active weights (coverage + complexity + dependency) must sum to 1.0 (±0.001 tolerance)
- If weights don't sum to 1.0, they will be automatically normalized

**Example - Prioritize complexity over coverage:**
```toml
[scoring]
coverage = 0.30
complexity = 0.55
dependency = 0.15
```

**Source:** `src/config/scoring.rs:14-40` (ScoringWeights)

## Complexity Weights

The `[complexity_weights]` section controls how cyclomatic and cognitive complexity are combined in the final scoring:

```toml
[complexity_weights]
cyclomatic = 0.3      # Weight for cyclomatic complexity (default: 0.3)
cognitive = 0.7       # Weight for cognitive complexity (default: 0.7)
max_cyclomatic = 50.0 # Maximum cyclomatic for normalization (default: 50.0)
max_cognitive = 100.0 # Maximum cognitive for normalization (default: 100.0)
```

**Why cognitive complexity is weighted higher:**
- Cognitive complexity correlates better with bug density
- Cyclomatic complexity is a proxy for test cases needed
- The 70/30 split balances maintainability with testability

**Source:** `src/config/scoring.rs:335-381` (ComplexityWeightsConfig)

## Role Multipliers

Role multipliers adjust complexity scores based on a function's semantic role:

```toml
[role_multipliers]
pure_logic = 1.2        # Prioritize pure computation (default: 1.2)
orchestrator = 0.8      # Reduce for delegation functions (default: 0.8)
io_wrapper = 0.7        # Reduce for I/O wrappers (default: 0.7)
entry_point = 0.9       # Slight reduction for main/CLI (default: 0.9)
pattern_match = 0.6     # Reduce for pattern matching (default: 0.6)
debug = 0.3             # Debug/diagnostic functions (default: 0.3)
unknown = 1.0           # No adjustment (default: 1.0)
```

These multipliers help reduce false positives by recognizing that different function types have naturally different complexity levels. The **debug** role has the lowest multiplier (0.3) since debug and diagnostic functions typically have low testing priority.

**Source:** `src/config/scoring.rs:206-333` (RoleMultipliers)

## Role-Based Scoring Configuration

DebtMap uses a two-stage role adjustment mechanism to accurately score functions based on their architectural role and testing strategy. This section explains how to configure both stages.

### Stage 1: Role Coverage Weights

The first stage adjusts how much coverage gaps penalize different function types. This recognizes that not all functions need the same level of unit test coverage.

**Configuration** (`.debtmap.toml` under `[scoring.role_coverage_weights]`):

```toml
[scoring.role_coverage_weights]
entry_point = 0.6       # Reduce coverage penalty (often integration tested)
orchestrator = 0.8      # Reduce coverage penalty (tested via higher-level tests)
pure_logic = 1.0        # Pure logic should have unit tests, no reduction (default: 1.0)
io_wrapper = 0.5        # I/O wrappers are integration tested (default: 0.5)
pattern_match = 1.0     # Standard penalty
debug = 0.3             # Debug functions have lowest coverage expectations (default: 0.3)
unknown = 1.0           # Standard penalty (default behavior)
```

**Rationale**:

| Function Role | Weight | Why This Value? |
|---------------|--------|----------------|
| **Entry Point** | 0.6 | CLI handlers, HTTP routes, `main` functions are integration tested, not unit tested |
| **Orchestrator** | 0.8 | Coordination functions tested via higher-level tests |
| **Pure Logic** | 1.0 | Core business logic should have unit tests (default: 1.0) |
| **I/O Wrapper** | 0.5 | File/network operations tested via integration tests (default: 0.5) |
| **Pattern Match** | 1.0 | Standard coverage expectations |
| **Debug** | 0.3 | Debug/diagnostic functions have lowest testing priority (default: 0.3) |
| **Unknown** | 1.0 | Default when role cannot be determined |

**Example Impact**:

```toml
# Emphasize pure logic testing strongly
[scoring.role_coverage_weights]
pure_logic = 1.5        # 50% higher penalty for untested logic
entry_point = 0.5       # 50% lower penalty for untested entry points
io_wrapper = 0.4        # 60% lower penalty for untested I/O

# Conservative approach (smaller adjustments)
[scoring.role_coverage_weights]
pure_logic = 1.1        # Only 10% increase
entry_point = 0.9       # Only 10% decrease
```

**How It Works**:

When a function has 0% coverage:
- **Entry Point** (weight 0.6): Gets 60% penalty instead of 100% penalty
- **Pure Logic** (weight 1.0): Gets 100% penalty (standard emphasis on testing)
- **I/O Wrapper** (weight 0.5): Gets 50% penalty

This prevents entry points from dominating the priority list due to low unit test coverage while emphasizing the importance of testing pure business logic.

**Source:** `src/config/scoring.rs:383-455` (RoleCoverageWeights)

### Stage 2: Role Multiplier with Clamping

The second stage applies a final role-based multiplier to reflect architectural importance. This multiplier is **clamped by default** to prevent extreme score variations.

**Configuration** (`.debtmap.toml` under `[scoring.role_multiplier]`):

```toml
[scoring.role_multiplier]
clamp_min = 0.3           # Minimum multiplier (default: 0.3)
clamp_max = 1.8           # Maximum multiplier (default: 1.8)
enable_clamping = true    # Enable clamping (default: true)
```

**Parameters**:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `clamp_min` | 0.3 | Minimum allowed multiplier - prevents functions from becoming invisible |
| `clamp_max` | 1.8 | Maximum allowed multiplier - prevents extreme score spikes |
| `enable_clamping` | true | Whether to apply clamping (disable for prototyping only) |

**Clamp Range Rationale**:

**Default [0.3, 1.8]**: Balances differentiation with stability
- **Lower bound (0.3)**: I/O wrappers still contribute 30% of their base score
  - Prevents them from becoming invisible in the priority list
  - Ensures simple wrappers aren't completely ignored

- **Upper bound (1.8)**: Critical functions get at most 180% of base score
  - Prevents one complex function from dominating the entire list
  - Maintains balanced prioritization across different issues

**When to Adjust Clamp Range**:

```toml
# Wider range for more differentiation
[scoring.role_multiplier]
clamp_min = 0.2           # Allow more reduction
clamp_max = 2.5           # Allow more emphasis

# Narrower range for more stability
[scoring.role_multiplier]
clamp_min = 0.5           # Less reduction
clamp_max = 1.5           # Less emphasis

# Disable clamping (not recommended for production)
[scoring.role_multiplier]
enable_clamping = false   # Allow unclamped multipliers
# Warning: May cause unstable prioritization
```

**When to Disable Clamping**:
- **Prototyping**: Testing extreme multiplier values for custom scoring strategies
- **Special cases**: Very specific project needs requiring wide multiplier ranges
- **Not recommended** for production use as it can lead to unstable prioritization

**Example Impact**:

Without clamping:
```
Function: critical_business_logic (Pure Logic)
  Base Score: 45.0
  Role Multiplier: 2.5 (unclamped)
  Final Score: 112.5 (dominates entire list)
```

With clamping (default):
```
Function: critical_business_logic (Pure Logic)
  Base Score: 45.0
  Role Multiplier: 1.8 (clamped from 2.5)
  Final Score: 81.0 (high priority, but balanced)
```

**Source:** `src/config/scoring.rs:457-493` (RoleMultiplierConfig)

### Complete Example Configuration

Here's a complete example showing both stages configured together:

```toml
# Stage 1: Coverage weight adjustments
[scoring.role_coverage_weights]
pure_logic = 1.0        # Pure logic should have unit tests (default: 1.0)
entry_point = 0.6       # Reduce penalty for integration-tested entry points
orchestrator = 0.8      # Partially reduce penalty for orchestrators
io_wrapper = 0.5        # I/O wrappers are integration tested (default: 0.5)
pattern_match = 1.0     # Standard
debug = 0.3             # Debug functions have lowest coverage expectations (default: 0.3)
unknown = 1.0           # Standard

# Stage 2: Role multiplier with clamping
[scoring.role_multiplier]
clamp_min = 0.3         # I/O wrappers contribute at least 30%
clamp_max = 1.8         # Critical functions get at most 180%
enable_clamping = true  # Keep clamping enabled for stability
```

### How the Two Stages Work Together

The two-stage approach ensures role-based coverage adjustments and architectural importance multipliers work independently:

**Example Workflow**:
```
1. Calculate base score from complexity (10) and dependencies (5)
   -> Base = 15.0

2. Stage 1: Apply coverage weight based on role (Entry Point, weight 0.6)
   -> Coverage penalty reduced from 1.0 to 0.4
   -> Preliminary score = 15.0 * 0.4 = 6.0

3. Stage 2: Apply clamped role multiplier (Entry Point, multiplier 1.2)
   -> Clamped to [0.3, 1.8] -> stays 1.2
   -> Final score = 6.0 * 1.2 = 7.2
```

**Key Benefits**:
- Coverage adjustments don't interfere with role multiplier
- Both mechanisms contribute independently to final score
- Clamping prevents instability from extreme values
- Configuration flexibility for different project needs

### Verification

To see how role-based adjustments affect your codebase:

```bash
# Show detailed scoring breakdown
debtmap analyze . --verbose

# Look for lines like:
#   Coverage Weight: 0.6 (Entry Point adjustment)
#   Adjusted Coverage Penalty: 0.4 (reduced from 1.0)
#   Role Multiplier: 1.2 (clamped from 1.5)
```

For more details on how role-based adjustments reduce false positives, see the [Role-Based Adjustments](../scoring-strategies.md#role-based-adjustments) section in the Scoring Strategies guide.

## Score Normalization

The `[normalization]` section controls how raw scores are normalized to a 0-10 scale:

```toml
[normalization]
linear_threshold = 10.0         # Use linear scaling below this value
logarithmic_threshold = 100.0   # Use logarithmic scaling above this value
sqrt_multiplier = 3.33          # Multiplier for square root scaling
log_multiplier = 10.0           # Multiplier for logarithmic scaling
show_raw_scores = true          # Show both raw and normalized scores
```

Normalization ensures scores are comparable across different codebases and prevents extreme outliers from dominating the results.

**Source:** `src/config/scoring.rs:557-610` (NormalizationConfig)

## Context Multipliers

The `[context_multipliers]` section dampens scores for non-production code (spec 191):

```toml
[context_multipliers]
examples = 0.1              # 90% reduction for example files
tests = 0.2                 # 80% reduction for test files
benchmarks = 0.3            # 70% reduction for benchmarks
build_scripts = 0.3         # 70% reduction for build scripts
documentation = 0.1         # 90% reduction for documentation
enable_context_dampening = false  # Disabled by default (use --context flag)
```

When enabled with `--context`, these multipliers reduce false positive urgency scores for code that doesn't represent production complexity.

**Source:** `src/config/scoring.rs:612-676` (ContextMultipliers)

## Data Flow Scoring

The `[data_flow_scoring]` section configures analysis of data flow patterns (spec 218):

```toml
[data_flow_scoring]
enabled = true              # Enable data flow scoring
purity_weight = 0.4         # Weight for function purity
refactorability_weight = 0.3 # Weight for refactorability
pattern_weight = 0.3        # Weight for pattern recognition
```

Data flow scoring rewards functions with:
- Pure data transformations (no side effects)
- High refactorability potential
- Recognized functional patterns (map, filter, fold)

**Source:** `src/config/scoring.rs:678-724` (DataFlowScoringConfig)

## Rebalanced Scoring Presets

The `[rebalanced_scoring]` section allows using predefined presets or custom weights (spec 136):

```toml
[rebalanced_scoring]
preset = "balanced"         # Preset: balanced, quality-focused, size-focused, test-coverage
# Or override with custom weights:
# complexity_weight = 0.35
# coverage_weight = 0.30
# structural_weight = 0.15
# size_weight = 0.10
# smell_weight = 0.10
```

**Available Presets:**

| Preset | Focus | Best For |
|--------|-------|----------|
| `balanced` | Even distribution | General use |
| `quality-focused` | Complexity + coverage | Mature projects |
| `size-focused` | File/function size | Refactoring projects |
| `test-coverage` | Coverage gaps | Test improvement efforts |

**Source:** `src/config/scoring.rs:495-554` (RebalancedScoringConfig)

## Related Documentation

- [Scoring Strategies](../scoring-strategies.md) - In-depth explanation of scoring algorithms
- [Role-Based Adjustments](../scoring-strategies.md#role-based-adjustments) - How semantic roles affect scoring
- [Thresholds Configuration](thresholds.md) - Configure when code is flagged as debt
- [Tiered Prioritization](../tiered-prioritization.md) - How scores map to priority tiers

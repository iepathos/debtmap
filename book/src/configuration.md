# Configuration

Debtmap is highly configurable through a `.debtmap.toml` file. This chapter explains how to customize Debtmap's behavior for your project's specific needs.

## Config Files

Debtmap uses **TOML format** for configuration files (`.debtmap.toml`). TOML provides a clear, readable syntax well-suited for configuration.

### Creating a Configuration File

Debtmap looks for a `.debtmap.toml` file in the current directory and up to 10 parent directories. To create an initial configuration:

```bash
debtmap init
```

This command creates a `.debtmap.toml` file with sensible defaults.

### Configuration File Discovery

When you run `debtmap`, it searches for `.debtmap.toml` starting in your current directory and traversing up to 10 parent directories. The first configuration file found is used.

If no configuration file is found, Debtmap uses built-in defaults that work well for most projects.

### Basic Example

Here's a minimal `.debtmap.toml` configuration:

```toml
[scoring]
coverage = 0.50      # 50% weight for test coverage gaps
complexity = 0.35    # 35% weight for code complexity
dependency = 0.15    # 15% weight for dependency criticality

[thresholds]
complexity = 10
max_file_length = 500
max_function_length = 50

[languages]
enabled = ["rust", "python", "javascript", "typescript"]
```

## Scoring Configuration

### Scoring Weights

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

### Role Multipliers

Role multipliers adjust complexity scores based on a function's semantic role:

```toml
[role_multipliers]
pure_logic = 1.2        # Prioritize pure computation (default: 1.2)
orchestrator = 0.8      # Reduce for delegation functions (default: 0.8)
io_wrapper = 0.7        # Reduce for I/O wrappers (default: 0.7)
entry_point = 0.9       # Slight reduction for main/CLI (default: 0.9)
pattern_match = 0.6     # Reduce for pattern matching (default: 0.6)
unknown = 1.0           # No adjustment (default: 1.0)
```

These multipliers help reduce false positives by recognizing that different function types have naturally different complexity levels.

### Role-Based Scoring Configuration

DebtMap uses a two-stage role adjustment mechanism to accurately score functions based on their architectural role and testing strategy. This section explains how to configure both stages.

#### Stage 1: Role Coverage Weights

The first stage adjusts how much coverage gaps penalize different function types. This recognizes that not all functions need the same level of unit test coverage.

**Configuration** (`.debtmap.toml` under `[scoring.role_coverage_weights]`):

```toml
[scoring.role_coverage_weights]
entry_point = 0.6       # Reduce coverage penalty (often integration tested)
orchestrator = 0.8      # Reduce coverage penalty (tested via higher-level tests)
pure_logic = 1.2        # Increase penalty (should have unit tests)
io_wrapper = 0.7        # Reduce penalty (often integration tested)
pattern_match = 1.0     # Standard penalty
unknown = 1.0           # Standard penalty (default behavior)
```

**Rationale**:

| Function Role | Weight | Why This Value? |
|---------------|--------|----------------|
| **Entry Point** | 0.6 | CLI handlers, HTTP routes, `main` functions are integration tested, not unit tested |
| **Orchestrator** | 0.8 | Coordination functions tested via higher-level tests |
| **Pure Logic** | 1.2 | Core business logic should have comprehensive unit tests |
| **I/O Wrapper** | 0.7 | File/network operations tested via integration tests |
| **Pattern Match** | 1.0 | Standard coverage expectations |
| **Unknown** | 1.0 | Default when role cannot be determined |

**Example Impact**:

```toml
# Emphasize pure logic testing strongly
[scoring.role_coverage_weights]
pure_logic = 1.5        # 50% higher penalty for untested logic
entry_point = 0.5       # 50% lower penalty for untested entry points
io_wrapper = 0.5        # 50% lower penalty for untested I/O

# Conservative approach (smaller adjustments)
[scoring.role_coverage_weights]
pure_logic = 1.1        # Only 10% increase
entry_point = 0.9       # Only 10% decrease
```

**How It Works**:

When a function has 0% coverage:
- **Entry Point** (weight 0.6): Gets 40% penalty instead of 100% penalty
- **Pure Logic** (weight 1.2): Gets 120% penalty (extra emphasis on testing)
- **I/O Wrapper** (weight 0.7): Gets 70% penalty

This prevents entry points from dominating the priority list due to low unit test coverage while emphasizing the importance of testing pure business logic.

#### Stage 2: Role Multiplier with Clamping

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

#### Complete Example Configuration

Here's a complete example showing both stages configured together:

```toml
# Stage 1: Coverage weight adjustments
[scoring.role_coverage_weights]
pure_logic = 1.2        # Emphasize testing pure logic
entry_point = 0.6       # Reduce penalty for integration-tested entry points
orchestrator = 0.8      # Partially reduce penalty for orchestrators
io_wrapper = 0.7        # Reduce penalty for integration-tested I/O
pattern_match = 1.0     # Standard
unknown = 1.0           # Standard

# Stage 2: Role multiplier with clamping
[scoring.role_multiplier]
clamp_min = 0.3         # I/O wrappers contribute at least 30%
clamp_max = 1.8         # Critical functions get at most 180%
enable_clamping = true  # Keep clamping enabled for stability
```

#### How the Two Stages Work Together

The two-stage approach ensures role-based coverage adjustments and architectural importance multipliers work independently:

**Example Workflow**:
```
1. Calculate base score from complexity (10) and dependencies (5)
   → Base = 15.0

2. Stage 1: Apply coverage weight based on role (Entry Point, weight 0.6)
   → Coverage penalty reduced from 1.0 to 0.4
   → Preliminary score = 15.0 × 0.4 = 6.0

3. Stage 2: Apply clamped role multiplier (Entry Point, multiplier 1.2)
   → Clamped to [0.3, 1.8] → stays 1.2
   → Final score = 6.0 × 1.2 = 7.2
```

**Key Benefits**:
- Coverage adjustments don't interfere with role multiplier
- Both mechanisms contribute independently to final score
- Clamping prevents instability from extreme values
- Configuration flexibility for different project needs

#### Verification

To see how role-based adjustments affect your codebase:

```bash
# Show detailed scoring breakdown
debtmap analyze . --verbose

# Look for lines like:
#   Coverage Weight: 0.6 (Entry Point adjustment)
#   Adjusted Coverage Penalty: 0.4 (reduced from 1.0)
#   Role Multiplier: 1.2 (clamped from 1.5)
```

For more details on how role-based adjustments reduce false positives, see the [Role-Based Adjustments](./scoring-strategies.md#role-based-adjustments) section in the Scoring Strategies guide.

## Thresholds Configuration

### Basic Thresholds

Control when code is flagged as technical debt:

```toml
[thresholds]
complexity = 10                      # Cyclomatic complexity threshold
duplication = 50                     # Duplication threshold
max_file_length = 500                # Maximum lines per file
max_function_length = 50             # Maximum lines per function
```

### Minimum Thresholds

Filter out trivial functions that aren't really technical debt:

```toml
[thresholds]
minimum_debt_score = 2.0              # Only show items with debt score ≥ 2.0
minimum_cyclomatic_complexity = 3     # Ignore functions with cyclomatic < 3
minimum_cognitive_complexity = 5      # Ignore functions with cognitive < 5
minimum_risk_score = 2.0              # Only show Risk items with score ≥ 2.0
```

These minimum thresholds help focus on significant issues by filtering out simple functions with minor complexity.

### Validation Thresholds

The `[thresholds.validation]` subsection configures limits for the `debtmap validate` command:

```toml
[thresholds.validation]
max_average_complexity = 10.0         # Maximum allowed average complexity (default: 10.0)
max_high_complexity_count = 100       # Maximum high complexity functions (default: 100)
max_debt_items = 2000                 # Maximum technical debt items (default: 2000)
max_total_debt_score = 1000           # Maximum total debt score (default: 1000)
max_codebase_risk_score = 7.0         # Maximum codebase risk score (default: 7.0)
max_high_risk_functions = 50          # Maximum high-risk functions (default: 50)
min_coverage_percentage = 0.0         # Minimum required coverage % (default: 0.0)
max_debt_density = 50.0               # Maximum debt per 1000 LOC (default: 50.0)
```

Use `debtmap validate` in CI to enforce code quality standards:

```bash
# Fail build if validation thresholds are exceeded
debtmap validate
```

## Language Configuration

### Enabling Languages

Specify which languages to analyze:

```toml
[languages]
enabled = ["rust", "python", "javascript", "typescript"]
```

### Language-Specific Features

Configure features for individual languages:

```toml
[languages.rust]
detect_dead_code = false        # Rust: disabled by default (compiler handles it)
detect_complexity = true
detect_duplication = true

[languages.python]
detect_dead_code = true
detect_complexity = true
detect_duplication = true

[languages.javascript]
detect_dead_code = true
detect_complexity = true
detect_duplication = true

[languages.typescript]
detect_dead_code = true
detect_complexity = true
detect_duplication = true
```

**Note:** Rust's dead code detection is disabled by default since the Rust compiler already provides excellent unused code warnings.

## Exclusion Patterns

### File and Directory Exclusion

Use glob patterns to exclude files and directories from analysis:

```toml
[ignore]
patterns = [
    "target/**",              # Rust build output
    "venv/**",                # Python virtual environment
    "node_modules/**",        # JavaScript dependencies
    "*.min.js",               # Minified files
    "benches/**",             # Benchmark code
    "tests/**/*",             # Test files
    "**/test_*.rs",           # Test files (prefix)
    "**/*_test.rs",           # Test files (suffix)
    "**/fixtures/**",         # Test fixtures
    "**/mocks/**",            # Mock implementations
    "**/stubs/**",            # Stub implementations
    "**/examples/**",         # Example code
    "**/demo/**",             # Demo code
]
```

**Glob pattern syntax:**
- `*` - Matches any characters except `/`
- `**` - Matches any characters including `/` (recursive)
- `?` - Matches a single character
- `[abc]` - Matches any character in the set

**Note:** Function-level filtering (e.g., ignoring specific function name patterns) is handled by role detection and context-aware analysis rather than explicit ignore patterns. See the Context-Aware Detection section for function-level filtering options.

## Display Configuration

Control how results are displayed:

```toml
[display]
tiered = true           # Use tiered priority display (default: true)
items_per_tier = 5      # Show 5 items per tier (default: 5)
```

When `tiered = true`, Debtmap groups results into priority tiers (Critical, High, Medium, Low) and shows the top items from each tier.

## Output Configuration

Set the default output format:

```toml
[output]
default_format = "terminal"    # Options: "terminal", "json", "markdown"
```

**Supported formats:**
- `"terminal"` - Human-readable colored output for the terminal (default)
- `"json"` - Machine-readable JSON for integration with other tools
- `"markdown"` - Markdown format for documentation and reports

This can be overridden with the `--format` CLI flag:

```bash
debtmap analyze --format json      # JSON output
debtmap analyze --format markdown  # Markdown output
```

## Normalization Configuration

Control how raw scores are normalized to a 0-10 scale:

```toml
[normalization]
linear_threshold = 10.0         # Use linear scaling below this value
logarithmic_threshold = 100.0   # Use logarithmic scaling above this value
sqrt_multiplier = 3.33          # Multiplier for square root scaling
log_multiplier = 10.0           # Multiplier for logarithmic scaling
show_raw_scores = true          # Show both raw and normalized scores
```

Normalization ensures scores are comparable across different codebases and prevents extreme outliers from dominating the results.

## Advanced Configuration

### Entropy-Based Complexity Scoring

Entropy analysis helps identify repetitive code patterns (like large match statements) that inflate complexity metrics:

```toml
[entropy]
enabled = true                      # Enable entropy analysis (default: true)
weight = 1.0                        # Weight in complexity adjustment (default: 1.0)
min_tokens = 20                     # Minimum tokens for analysis (default: 20)
pattern_threshold = 0.7             # Pattern similarity threshold (default: 0.7)
entropy_threshold = 0.4             # Low entropy threshold (default: 0.4)
branch_threshold = 0.8              # Branch similarity threshold (default: 0.8)
use_classification = false          # Use smarter token classification (default: false)

# Maximum reductions to prevent over-correction
max_repetition_reduction = 0.20     # Max 20% reduction for repetition (default: 0.20)
max_entropy_reduction = 0.15        # Max 15% reduction for low entropy (default: 0.15)
max_branch_reduction = 0.25         # Max 25% reduction for similar branches (default: 0.25)
max_combined_reduction = 0.30       # Max 30% total reduction (default: 0.30)
```

Entropy scoring reduces false positives from functions like parsers and state machines that have high cyclomatic complexity but are actually simple and maintainable.

### God Object Detection

Configure detection of classes/structs with too many responsibilities:

```toml
[god_object_detection]
enabled = true

# Rust-specific thresholds
[god_object_detection.rust]
max_methods = 20        # Maximum methods before flagging (default: 20)
max_fields = 15         # Maximum fields before flagging (default: 15)
max_traits = 5          # Maximum implemented traits
max_lines = 1000        # Maximum lines of code
max_complexity = 200    # Maximum total complexity

# Python-specific thresholds
[god_object_detection.python]
max_methods = 15
max_fields = 10
max_traits = 3
max_lines = 500
max_complexity = 150

# JavaScript-specific thresholds
[god_object_detection.javascript]
max_methods = 15
max_fields = 20         # JavaScript classes often have more properties
max_traits = 3
max_lines = 500
max_complexity = 150
```

**Note:** Different languages have different defaults. Rust allows more methods since trait implementations add methods, while JavaScript classes should be smaller.

### Context-Aware Detection

Enable context-aware pattern detection to reduce false positives:

```toml
[context]
enabled = false         # Opt-in (default: false)

# Custom context rules
[[context.rules]]
name = "allow_blocking_in_main"
pattern = "blocking_io"
action = "allow"
priority = 100
reason = "Main function can use blocking I/O"

[context.rules.context]
role = "main"

# Function pattern configuration
[context.function_patterns]
test_patterns = ["test_*", "bench_*"]
config_patterns = ["load_*_config", "parse_*_config"]
handler_patterns = ["handle_*", "*_handler"]
init_patterns = ["initialize_*", "setup_*"]
```

Context-aware detection adjusts severity based on where code appears (main functions, test code, configuration loaders, etc.).

### Error Handling Detection

Configure detection of error handling anti-patterns:

```toml
[error_handling]
detect_async_errors = true          # Detect async error issues (default: true)
detect_context_loss = true          # Detect error context loss (default: true)
detect_propagation = true           # Analyze error propagation (default: true)
detect_panic_patterns = true        # Detect panic/unwrap usage (default: true)
detect_swallowing = true            # Detect swallowed errors (default: true)

# Custom error patterns
[[error_handling.custom_patterns]]
name = "custom_panic"
pattern = "my_panic_macro"
pattern_type = "macro_name"
severity = "high"
description = "Custom panic macro usage"
remediation = "Replace with Result-based error handling"

# Severity overrides
[[error_handling.severity_overrides]]
pattern = "unwrap"
context = "test"
severity = "low"        # Unwrap is acceptable in test code
```

### External API Configuration

Mark functions as public API for enhanced testing recommendations:

```toml
[external_api]
detect_external_api = false         # Auto-detect public APIs (default: false)
api_functions = []                  # Explicitly mark API functions
api_files = []                      # Explicitly mark API files
```

When enabled, public API functions receive higher priority for test coverage.

### Additional Advanced Options

Debtmap supports additional advanced configuration options:

- **`[loc]`** - Lines of code counting configuration. Controls whether to include tests (`include_tests`), generated files (`include_generated`), comments (`count_comments`), and blank lines (`count_blank_lines`) in LOC counts. All default to false.

- **`[tiers]`** - Tier threshold configuration for prioritization. Allows customization of complexity and dependency thresholds for different priority tiers (T2, T3, T4). Used internally for tiered reporting.

- **`[complexity_thresholds]`** - Enhanced complexity detection thresholds. Configures minimum total, cyclomatic, and cognitive complexity thresholds for flagging functions. Supplements the basic `[thresholds]` section with more granular control.

These options are advanced features with sensible defaults. Most users won't need to configure them explicitly.

## CLI Integration

CLI flags can override configuration file settings:

```bash
# Override complexity threshold
debtmap analyze --threshold-complexity 15

# Provide coverage file
debtmap analyze --coverage-file coverage.json

# Enable context-aware detection
debtmap analyze --context

# Override output format
debtmap analyze --format json
```

### Configuration Precedence

Debtmap resolves configuration values in the following order (highest to lowest priority):

1. **CLI flags** - Command-line arguments (e.g., `--threshold-complexity 15`)
2. **Configuration file** - Settings from `.debtmap.toml`
3. **Built-in defaults** - Debtmap's sensible default values

This allows you to set project-wide defaults in `.debtmap.toml` while customizing specific runs with CLI flags.

## Configuration Validation

### Automatic Validation

Debtmap automatically validates your configuration when loading:

- **Scoring weights** must sum to 1.0 (±0.001 tolerance)
- **Individual weights** must be between 0.0 and 1.0
- **Invalid configurations** fall back to defaults with a warning

### Normalization

If scoring weights don't sum exactly to 1.0, Debtmap automatically normalizes them:

```toml
# Input (sums to 0.80)
[scoring]
coverage = 0.40
complexity = 0.30
dependency = 0.10

# Automatically normalized to:
# coverage = 0.50
# complexity = 0.375
# dependency = 0.125
```

### Debug Validation

To verify which configuration file is being loaded, check debug logs:

```bash
RUST_LOG=debug debtmap analyze
```

Look for log messages like:
```
DEBUG debtmap::config: Loaded config from /path/to/.debtmap.toml
```

## Complete Configuration Example

Here's a comprehensive configuration showing all major sections:

```toml
# Scoring configuration
[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15

# Basic thresholds
[thresholds]
complexity = 10
duplication = 50
max_file_length = 500
max_function_length = 50
minimum_debt_score = 2.0
minimum_cyclomatic_complexity = 3
minimum_cognitive_complexity = 5
minimum_risk_score = 2.0

# Validation thresholds for CI
[thresholds.validation]
max_average_complexity = 10.0
max_high_complexity_count = 100
max_debt_items = 2000
max_total_debt_score = 1000
max_codebase_risk_score = 7.0
max_high_risk_functions = 50
min_coverage_percentage = 0.0
max_debt_density = 50.0

# Language configuration
[languages]
enabled = ["rust", "python", "javascript", "typescript"]

[languages.rust]
detect_dead_code = false
detect_complexity = true
detect_duplication = true

# Exclusion patterns
[ignore]
patterns = [
    "target/**",
    "node_modules/**",
    "tests/**/*",
    "**/*_test.rs",
]

# Display configuration
[display]
tiered = true
items_per_tier = 5

# Output configuration
[output]
default_format = "terminal"

# Entropy configuration
[entropy]
enabled = true
weight = 1.0
min_tokens = 20

# God object detection
[god_object_detection]
enabled = true

[god_object_detection.rust]
max_methods = 20
max_fields = 15
```

## Configuration Best Practices

### For Strict Quality Standards

```toml
[scoring]
coverage = 0.60         # Emphasize test coverage
complexity = 0.30
dependency = 0.10

[thresholds]
minimum_debt_score = 3.0        # Higher bar for flagging issues
max_function_length = 30        # Enforce smaller functions

[thresholds.validation]
max_average_complexity = 8.0    # Stricter complexity limits
max_debt_items = 500            # Stricter debt limits
min_coverage_percentage = 80.0  # Require 80% coverage
```

### For Legacy Codebases

```toml
[scoring]
coverage = 0.30         # Reduce coverage weight (legacy code often lacks tests)
complexity = 0.50       # Focus on complexity
dependency = 0.20

[thresholds]
minimum_debt_score = 5.0        # Only show highest priority items
minimum_cyclomatic_complexity = 10   # Filter out moderate complexity

[thresholds.validation]
max_debt_items = 10000          # Accommodate large debt
max_total_debt_score = 5000     # Higher limits for legacy code
```

### For Open Source Libraries

```toml
[scoring]
coverage = 0.55         # Prioritize test coverage (public API)
complexity = 0.30
dependency = 0.15

[external_api]
detect_external_api = true      # Flag untested public APIs

[thresholds.validation]
min_coverage_percentage = 90.0  # High coverage for public API
max_high_complexity_count = 20  # Keep complexity low
```

## Troubleshooting

### Configuration Not Loading

**Check file location:**
```bash
# Ensure file is named .debtmap.toml (note the dot prefix)
ls -la .debtmap.toml

# Debtmap searches current directory + 10 parent directories
pwd
```

**Check file syntax:**
```bash
# Verify TOML syntax is valid
debtmap analyze 2>&1 | grep -i "failed to parse"
```

### Weights Don't Sum to 1.0

**Error message:**
```
Warning: Invalid scoring weights: Active scoring weights must sum to 1.0, but sum to 0.800. Using defaults.
```

**Fix:** Ensure coverage + complexity + dependency = 1.0

```toml
[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15    # Sum = 1.0 ✓
```

### No Results Shown

**Possible causes:**
1. Minimum thresholds too high
2. All code excluded by ignore patterns
3. No supported languages in project

**Solutions:**
```toml
# Lower minimum thresholds
[thresholds]
minimum_debt_score = 1.0
minimum_cyclomatic_complexity = 1

# Check language configuration
[languages]
enabled = ["rust", "python", "javascript", "typescript"]

# Review ignore patterns
[ignore]
patterns = [
    # Make sure you're not excluding too much
]
```

## Related Chapters

- [Getting Started](./getting-started.md) - Initial setup and basic usage
- [Analysis Guide](./analysis-guide.md) - Understanding scoring and prioritization
- [Output Formats](./output-formats.md) - Formatting and exporting results

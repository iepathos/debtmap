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

### Scoring Presets (Rebalanced Scoring)

The **`[scoring_rebalanced]`** section provides preset scoring configurations optimized for different project types and priorities. This is an alternative to manually configuring individual scoring weights.

**Configuration:**

```toml
[scoring_rebalanced]
preset = "balanced"              # Use a preset: balanced, quality-focused, size-focused, test-coverage

# Optional: Override specific weights within the preset
complexity_weight = 1.2          # Increase complexity emphasis
coverage_weight = 0.9            # Slightly reduce coverage emphasis
```

**Available Presets:**

| Preset | Complexity | Coverage | Structural | Size | Smell | Use Case |
|--------|-----------|----------|------------|------|-------|----------|
| **balanced** | 1.0 | 1.0 | 0.8 | 0.3 | 0.6 | General-purpose projects (default) |
| **quality-focused** | 1.2 | 1.1 | 0.9 | 0.2 | 0.7 | High-quality codebases, libraries |
| **size-focused** | 0.5 | 0.4 | 0.6 | 1.5 | 0.3 | Large legacy codebases |
| **test-coverage** | 0.8 | 1.3 | 0.6 | 0.2 | 0.5 | Projects improving test coverage |

**Weight Descriptions:**

- **complexity_weight**: Emphasis on cyclomatic/cognitive complexity
- **coverage_weight**: Emphasis on test coverage gaps
- **structural_weight**: Emphasis on architectural issues (dependencies, coupling)
- **size_weight**: Emphasis on code size (lines, file length)
- **smell_weight**: Emphasis on code smells and anti-patterns

**Preset Details:**

**1. Balanced (Default)**
```toml
[scoring_rebalanced]
preset = "balanced"
# complexity=1.0, coverage=1.0, structural=0.8, size=0.3, smell=0.6
```
- Standard distribution for most projects
- Equal weight on complexity and coverage
- Moderate emphasis on structure and smells
- Low emphasis on size (size alone rarely indicates debt)

**Use when:** Starting a new project, general-purpose analysis, no specific priorities

**2. Quality-Focused**
```toml
[scoring_rebalanced]
preset = "quality-focused"
# complexity=1.2, coverage=1.1, structural=0.9, size=0.2, smell=0.7
```
- Higher emphasis on complexity and coverage
- Strong structural analysis
- High smell detection
- Minimal size consideration

**Use when:** Building libraries, high-quality standards, public APIs, strict code review

**3. Size-Focused**
```toml
[scoring_rebalanced]
preset = "size-focused"
# complexity=0.5, coverage=0.4, structural=0.6, size=1.5, smell=0.3
```
- Prioritizes large files and functions
- Reduced complexity/coverage emphasis
- Useful for identifying architectural problems in large codebases
- Lower smell sensitivity

**Use when:** Legacy codebases with large files, refactoring for modularity, initial cleanup

**4. Test Coverage**
```toml
[scoring_rebalanced]
preset = "test-coverage"
# complexity=0.8, coverage=1.3, structural=0.6, size=0.2, smell=0.5
```
- Heavy emphasis on coverage gaps
- Moderate complexity consideration
- Moderate structural analysis
- Low size/smell emphasis

**Use when:** Improving test coverage, preparing for production, increasing quality metrics

**Customizing Presets:**

You can override individual weights within a preset:

```toml
[scoring_rebalanced]
preset = "balanced"              # Start with balanced preset
complexity_weight = 1.5          # But emphasize complexity more
coverage_weight = 0.8            # And reduce coverage slightly
```

This allows fine-tuning without configuring all weights from scratch.

**Relationship with Base Scoring Configuration:**

`scoring_rebalanced` is an alternative to the basic `[scoring]` section:
- **Use `[scoring]`**: For simple weighted sum model (coverage, complexity, dependency)
- **Use `[scoring_rebalanced]`**: For multi-dimensional scoring with presets

Do not use both in the same configuration file.

**Source:** Configuration type defined in `src/config/scoring.rs:383-407`, presets implemented in `src/priority/scoring/rebalanced.rs:59-128`

### Role Multipliers

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

### Role-Based Scoring Configuration

DebtMap uses a two-stage role adjustment mechanism to accurately score functions based on their architectural role and testing strategy. This section explains how to configure both stages.

#### Stage 1: Role Coverage Weights

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

### Coverage Expectations

The **`[coverage_expectations]`** section defines target coverage levels for different function categories. This configuration works alongside `role_coverage_weights` but serves a different purpose:

- **`role_coverage_weights`**: Adjusts penalty weights when coverage is low (affects scoring)
- **`coverage_expectations`**: Defines what "acceptable" coverage looks like per category (sets targets)

**Configuration:**

```toml
[coverage_expectations]
# Pure computation functions
pure.min = 0.90              # Pure functions should have 90-100% coverage
pure.target = 1.00

# Business logic functions
business_logic.min = 0.80    # Business logic needs 80-95% coverage
business_logic.target = 0.95

# State management functions
state_management.min = 0.75  # State management needs 75-90% coverage
state_management.target = 0.90

# I/O operations
io_operations.min = 0.60     # I/O often tested via integration tests (60-80%)
io_operations.target = 0.80

# Validation functions
validation.min = 0.85        # Validation should be well-tested (85-98%)
validation.target = 0.98

# Error handling
error_handling.min = 0.70    # Error paths need coverage (70-90%)
error_handling.target = 0.90

# Configuration loaders
configuration.min = 0.60     # Config loading (60-80%)
configuration.target = 0.80

# Initialization functions
initialization.min = 0.50    # Setup functions (50-75%)
initialization.target = 0.75

# Orchestration functions
orchestration.min = 0.65     # Coordination functions (65-85%)
orchestration.target = 0.85

# Utility functions
utilities.min = 0.75         # Helper utilities (75-95%)
utilities.target = 0.95

# Debug functions
debug.min = 0.20             # Debug/diagnostic functions (20-40%)
debug.target = 0.40

# Performance-critical functions
performance.min = 0.40       # Performance code (40-60%)
performance.target = 0.60
```

**Coverage Categories:**

| Category | Min | Target | Rationale |
|----------|-----|--------|-----------|
| **Pure Functions** | 90% | 100% | Easiest to test, no excuses for low coverage |
| **Business Logic** | 80% | 95% | Core functionality should be thoroughly tested |
| **State Management** | 75% | 90% | Stateful operations need careful testing |
| **I/O Operations** | 60% | 80% | Often covered by integration tests |
| **Validation** | 85% | 98% | Input validation critical for security |
| **Error Handling** | 70% | 90% | Error paths need explicit testing |
| **Configuration** | 60% | 80% | Config loading often integration tested |
| **Initialization** | 50% | 75% | Setup code may rely on integration tests |
| **Orchestration** | 65% | 85% | Coordination tested at higher levels |
| **Utilities** | 75% | 95% | Reusable helpers should be well-tested |
| **Debug** | 20% | 40% | Diagnostic code has low testing priority |
| **Performance** | 40% | 60% | Perf code may prioritize benchmarks over tests |

**How It Works:**

Coverage expectations define ranges for what constitutes acceptable coverage for each function category. When Debtmap analyzes a function:

1. Function is classified into a category (pure, business_logic, etc.)
2. Current coverage is compared against the category's expectations
3. Coverage penalty is calculated based on distance from target
4. `role_coverage_weights` then adjusts the penalty's impact on final score

**Example Workflow:**

```
Function: validate_email() - classified as "validation"
Current coverage: 75%
Expectations: min=85%, target=98%

Coverage gap: (98% - 75%) / (98% - 85%) = 23% / 13% = 1.77x below target
Base penalty: 1.77 (for being 77% below target)
Role weight adjustment: 1.0 (validation functions have standard penalty)
Final coverage penalty: 1.77
```

**Why Separate from Role Coverage Weights?**

This separation allows flexible configuration:
- **Expectations** define absolute standards per category
- **Weights** control how much penalties affect final scores
- Projects can have different standards (strict vs lenient) while maintaining consistent relative weights

**Source:** Configuration type defined in `src/priority/scoring/coverage_expectations.rs:108-133`

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

**Note:** The TOML configuration accepts `max_file_length` (shown above), which maps to the internal struct field `max_file_lines`. Both names refer to the same setting.

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
max_high_complexity_count = 100       # DEPRECATED: Use max_debt_density instead (default: 100)
max_debt_items = 2000                 # DEPRECATED: Use max_debt_density instead (default: 2000)
max_total_debt_score = 10000          # Maximum total debt score (default: 10000)
max_codebase_risk_score = 7.0         # Maximum codebase risk score (default: 7.0)
max_high_risk_functions = 50          # DEPRECATED: Use max_debt_density instead (default: 50)
min_coverage_percentage = 0.0         # Minimum required coverage % (default: 0.0)
max_debt_density = 50.0               # Maximum debt per 1000 LOC (default: 50.0)
```

**Deprecated Fields (v0.3.0+):**

The following validation thresholds are **deprecated** since v0.3.0 and will be removed in v1.0:

- `max_high_complexity_count` - Replaced by `max_debt_density` (scale-independent)
- `max_debt_items` - Replaced by `max_debt_density` (scale-independent)
- `max_high_risk_functions` - Replaced by `max_debt_density` (scale-independent)

**Migration:** Use `max_debt_density` instead, which provides a scale-independent metric (debt per 1000 lines of code). This allows the same threshold to work across codebases of different sizes.

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

### Evidence Display Configuration

The **`[output]`** section also controls how detection evidence is displayed in analysis results. This is useful for understanding why Debtmap flagged specific functions and for debugging false positives.

**Configuration:**

```toml
[output]
default_format = "terminal"
evidence_verbosity = "Standard"      # Options: Minimal, Standard, Verbose, VeryVerbose
min_confidence_warning = 0.80        # Flag detections below this confidence (default: 0.80)

# Optional: Filter which signals appear in evidence
[output.signal_filters]
min_signal_weight = 0.1              # Hide signals with weight < 0.1
exclude_categories = []               # Exclude specific signal categories
```

**Evidence Verbosity Levels:**

| Level | Value | What It Shows | Use Case |
|-------|-------|---------------|----------|
| **Minimal** | 0 | Category and confidence only | Production reports, high-level overview |
| **Standard** | 1 | Signal summary (default) | Regular development workflow |
| **Verbose** | 2 | Detailed signal breakdown | Investigating specific issues |
| **VeryVerbose** | 3 | All signals including low-weight | Debugging false positives |

**Example Output by Verbosity Level:**

**Minimal:**
```
Function: process_order
Detection: Orchestrator (confidence: 0.85)
```

**Standard (default):**
```
Function: process_order
Detection: Orchestrator (confidence: 0.85)
Evidence:
  - High delegation ratio (0.75)
  - Multiple function calls (8 callees)
```

**Verbose:**
```
Function: process_order
Detection: Orchestrator (confidence: 0.85)
Evidence:
  - Delegation ratio: 0.75 (weight: 0.35)
  - Function calls: 8 callees (weight: 0.30)
  - Low cyclomatic: 4 (weight: 0.20)
  - Cognitive complexity: 3 (weight: 0.15)
```

**VeryVerbose:**
```
Function: process_order
Detection: Orchestrator (confidence: 0.85)
Evidence:
  - Delegation ratio: 0.75 (weight: 0.35, contribution: 0.263)
  - Function calls: 8 callees (weight: 0.30, contribution: 0.255)
  - Low cyclomatic: 4 (weight: 0.20, contribution: 0.170)
  - Cognitive complexity: 3 (weight: 0.15, contribution: 0.128)
  - No async operations (weight: 0.05, contribution: 0.043)
  - Few conditionals: 1 (weight: 0.03, contribution: 0.026)
```

**Confidence Threshold:**

The `min_confidence_warning` setting flags detections with low confidence scores:

```toml
[output]
min_confidence_warning = 0.80        # Warn if confidence < 80%
```

Detections below this threshold will be marked with a warning indicator in output:
```
Function: unclear_function
Detection: Orchestrator (confidence: 0.72) ⚠️  LOW CONFIDENCE
```

**Use cases:**
- **0.90+**: Very strict, only high-confidence detections
- **0.80** (default): Balanced, flags uncertain classifications
- **0.70**: More permissive, accepts moderate confidence
- **0.60 or lower**: Permissive, shows most detections

**Signal Filtering:**

Filter which signals appear in evidence display:

```toml
[output.signal_filters]
min_signal_weight = 0.1              # Hide signals contributing < 10%
exclude_categories = ["performance", "style"]  # Exclude specific categories
```

This helps focus on the most important evidence when debugging or presenting analysis results.

**When to Adjust Verbosity:**

| Scenario | Recommended Setting |
|----------|-------------------|
| Daily development | `Standard` (default) |
| CI/CD reports | `Minimal` |
| Investigating false positives | `Verbose` or `VeryVerbose` |
| Debugging detection logic | `VeryVerbose` with low `min_signal_weight` |
| Performance analysis | `Standard` with `signal_filters` |

**Source:** Configuration types defined in `src/config/display.rs:20-30` (EvidenceVerbosity enum) and `src/config/display.rs:123-135` (OutputConfig)

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

### Pure Mapping Pattern Detection

Configure detection of pure mapping patterns to reduce false positives from exhaustive match expressions:

```toml
[mapping_patterns]
enabled = true                      # Enable mapping pattern detection (default: true)
complexity_reduction = 0.30         # Reduce complexity by 30% (default: 0.30)
min_branches = 3                    # Minimum match arms to consider (default: 3)
```

**What are pure mapping patterns?**

Pure mapping patterns are exhaustive match expressions that transform input to output without side effects. These patterns have high cyclomatic complexity due to many branches, but are actually simple and maintainable because:

- Each branch is independent and straightforward
- No mutation or side effects occur
- The pattern is predictable and easy to understand
- Adding new cases requires minimal changes

**Example:**
```rust
fn status_to_string(status: Status) -> &'static str {
    match status {
        Status::Success => "success",
        Status::Pending => "pending",
        Status::Failed => "failed",
        Status::Cancelled => "cancelled",
        // ... many more cases
    }
}
```

This function has high cyclomatic complexity (one branch per case), but is simple to maintain. Mapping pattern detection recognizes this and reduces the complexity score appropriately.

**Configuration options:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `enabled` | true | Enable mapping pattern detection |
| `complexity_reduction` | 0.30 | Percentage to reduce complexity (0.0-1.0) |
| `min_branches` | 3 | Minimum match arms to be considered a mapping pattern |

**Example configuration:**

```toml
# Conservative reduction
[mapping_patterns]
complexity_reduction = 0.20         # Only 20% reduction

# Aggressive reduction for codebases with many mapping patterns
[mapping_patterns]
complexity_reduction = 0.50         # 50% reduction

# Disable if you want to see all match complexity
[mapping_patterns]
enabled = false
```

**When to adjust:**

- **Increase `complexity_reduction`** if you have many simple mapping functions being flagged as complex
- **Decrease `complexity_reduction`** if you want more conservative adjustments
- **Increase `min_branches`** to only apply reduction to very large match statements
- **Disable entirely** if you want raw complexity scores without adjustment

### External API Configuration

Mark functions as public API for enhanced testing recommendations:

```toml
[external_api]
detect_external_api = false         # Auto-detect public APIs (default: false)
api_functions = []                  # Explicitly mark API functions
api_files = []                      # Explicitly mark API files
```

When enabled, public API functions receive higher priority for test coverage.

### Classification Configuration

The **`[classification]`** section controls how Debtmap classifies functions by their semantic role (constructor, accessor, data flow, etc.). This classification drives role-based adjustments and reduces false positives.

```toml
[classification]
# Constructor detection
[classification.constructors]
detect_constructors = true            # Enable constructor detection (default: true)
constructor_patterns = ["new", "create", "build", "from"]  # Common constructor names

# Accessor detection
[classification.accessors]
detect_accessors = true               # Enable accessor/getter detection (default: true)
accessor_patterns = ["get_*", "set_*", "is_*", "has_*"]   # Common accessor patterns

# Data flow detection
[classification.data_flow]
detect_data_flow = true               # Enable data flow analysis (default: true)
```

**Configuration Options:**

| Section | Option | Default | Description |
|---------|--------|---------|-------------|
| `constructors` | `detect_constructors` | true | Identify constructor functions |
| `constructors` | `constructor_patterns` | ["new", "create", "build", "from"] | Name patterns for constructors |
| `accessors` | `detect_accessors` | true | Identify accessor/getter functions |
| `accessors` | `accessor_patterns` | ["get_*", "set_*", "is_*", "has_*"] | Name patterns for accessors |
| `data_flow` | `detect_data_flow` | true | Enable data flow analysis |

**Why Classification Matters:**

Classification helps Debtmap understand function intent and apply appropriate complexity adjustments:

- **Constructors** typically have boilerplate initialization code with naturally higher complexity
- **Accessors** are simple getters/setters that shouldn't be flagged as debt
- **Data flow functions** (mappers, filters) have predictable patterns that inflate metrics

By detecting these patterns, Debtmap reduces false positives and focuses on genuine technical debt.

### Additional Advanced Options

Debtmap supports additional advanced configuration options:

#### Lines of Code Configuration

The **`[loc]`** section controls how lines of code are counted for metrics and reporting:

```toml
[loc]
include_tests = false         # Exclude test files from LOC counts (default: false)
include_generated = false     # Exclude generated files from LOC counts (default: false)
count_comments = false        # Include comment lines in LOC counts (default: false)
count_blank_lines = false     # Include blank lines in LOC counts (default: false)
```

**Configuration options:**

| Option | Default | Description |
|--------|---------|-------------|
| `include_tests` | false | Whether to include test files in LOC metrics |
| `include_generated` | false | Whether to include generated files in LOC metrics |
| `count_comments` | false | Whether to count comment lines as LOC |
| `count_blank_lines` | false | Whether to count blank lines as LOC |

**Example - Strict LOC counting:**
```toml
[loc]
include_tests = false         # Focus on production code
include_generated = false     # Exclude auto-generated code
count_comments = false        # Only count executable code
count_blank_lines = false     # Exclude whitespace
```

#### Tier Configuration

The **`[tiers]`** section configures tier threshold boundaries for prioritization:

```toml
[tiers]
t2_complexity_threshold = 15      # Complexity threshold for Tier 2 (default: 15)
t2_dependency_threshold = 10      # Dependency threshold for Tier 2 (default: 10)
t3_complexity_threshold = 10      # Complexity threshold for Tier 3 (default: 10)
show_t4_in_main_report = false    # Show Tier 4 items in main report (default: false)
```

**Tier priority levels:**
- **Tier 1 (Critical)**: Highest priority items
- **Tier 2 (High)**: Items above `t2_*` thresholds
- **Tier 3 (Medium)**: Items above `t3_*` thresholds
- **Tier 4 (Low)**: Items below all thresholds

**Example - Stricter tier boundaries:**
```toml
[tiers]
t2_complexity_threshold = 12      # Lower threshold = more items in high priority
t2_dependency_threshold = 8
t3_complexity_threshold = 8
show_t4_in_main_report = true     # Include low-priority items
```

#### Enhanced Complexity Thresholds

The **`[complexity_thresholds]`** section provides more granular control over complexity detection, supplementing the basic `[thresholds]` section with additional filters for flagging functions.

**Configuration:**

```toml
[complexity_thresholds]
minimum_total_complexity = 5         # Minimum combined complexity (default: 5)
minimum_cyclomatic_complexity = 3    # Minimum cyclomatic complexity (default: 3)
minimum_cognitive_complexity = 5     # Minimum cognitive complexity (default: 5)
minimum_match_arms = 4               # Minimum match arms (default: 4)
minimum_if_else_chain = 3            # Minimum if-else chain length (default: 3)
minimum_function_length = 10         # Minimum function length in lines (default: 10)
entry_point_multiplier = 0.8         # Multiplier for entry points (default: 0.8)
```

**Configuration Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `minimum_total_complexity` | 5 | Ignore functions with total complexity below this |
| `minimum_cyclomatic_complexity` | 3 | Ignore functions with cyclomatic complexity below this |
| `minimum_cognitive_complexity` | 5 | Ignore functions with cognitive complexity below this |
| `minimum_match_arms` | 4 | Minimum match arms to consider complex |
| `minimum_if_else_chain` | 3 | Minimum if-else chain length to flag |
| `minimum_function_length` | 10 | Ignore short functions (lines) |
| `entry_point_multiplier` | 0.8 | Adjust thresholds for entry points (main, CLI handlers) |

**Difference from Basic Thresholds:**

The basic `[thresholds]` section sets when to flag code as debt:
```toml
[thresholds]
complexity = 10                      # Flag functions with cyclomatic >= 10
max_function_length = 50             # Flag functions with >= 50 lines
```

The `[complexity_thresholds]` section adds **minimum filters** to ignore trivial functions:
```toml
[complexity_thresholds]
minimum_cyclomatic_complexity = 3    # Ignore functions with cyclomatic < 3
minimum_function_length = 10         # Ignore functions with < 10 lines
```

**Use Case:**

Enhanced thresholds help filter noise from simple functions that technically exceed basic thresholds but aren't true debt:

```rust
// Function with cyclomatic=1 (very simple)
fn get_name(&self) -> &str {
    &self.name
}
// Ignored by minimum_cyclomatic_complexity = 3

// Function with cyclomatic=8, length=12 lines
fn process_order(order: Order) -> Result<()> {
    // Contains multiple branches and error handling
}
// Flagged because it exceeds both minimum thresholds
```

**Entry Point Multiplier:**

The `entry_point_multiplier` relaxes thresholds for entry points (main functions, CLI handlers, HTTP routes) that naturally have higher complexity:

```toml
[complexity_thresholds]
minimum_cyclomatic_complexity = 5    # Standard minimum
entry_point_multiplier = 0.8         # Entry points: 5 * 0.8 = 4 minimum
```

This recognizes that entry points coordinate multiple operations and shouldn't be held to the same standards as pure logic functions.

**When to Adjust:**

- **Increase minimums** if you're getting too many trivial functions flagged
- **Decrease minimums** if you want to catch smaller issues early
- **Adjust entry_point_multiplier** to be more/less strict on entry points

Most users won't need to configure this section explicitly as defaults work well for typical projects.

**Source:** Configuration type defined in `src/complexity/threshold_manager.rs:17-46`

#### Orchestration Adjustment

The **`[orchestration_adjustment]`** section configures complexity reduction for orchestrator functions that primarily delegate to other functions. This works in conjunction with **orchestrator detection** to identify and appropriately score coordination functions.

**How It Works:**

Orchestration handling is a two-stage process:

1. **Detection** (`OrchestratorDetectionConfig` in `src/config/detection.rs:5-25`) - Identifies functions as orchestrators based on:
   - `max_cyclomatic`: Maximum cyclomatic complexity threshold (default: 5)
   - `min_delegation_ratio`: Minimum ratio of delegated calls (default: 0.2)
   - `min_meaningful_callees`: Minimum number of meaningful function calls (default: 2)
   - `cognitive_weight`: Weight for cognitive complexity consideration (default: 0.7)

2. **Adjustment** (`OrchestrationAdjustmentConfig` in `src/priority/scoring/orchestration_adjustment.rs:95-115`) - Applies score reductions to detected orchestrators:
   - `base_orchestrator_reduction`: Base complexity reduction (default: 0.20)
   - `max_quality_bonus`: Additional reduction for high-quality composition (default: 0.10)
   - `max_total_reduction`: Maximum combined reduction cap (default: 0.31)

**Configuration:**

```toml
[orchestration_adjustment]
enabled = true                        # Enable orchestration scoring adjustments (default: true)
base_orchestrator_reduction = 0.20    # Base reduction for detected orchestrators (default: 0.20)
max_quality_bonus = 0.10              # Additional reduction for well-structured delegation (default: 0.10)
max_total_reduction = 0.31            # Maximum combined reduction (default: 0.31)
```

**Rationale:**

Orchestrator functions coordinate multiple operations but don't contain complex logic themselves. They naturally have higher cyclomatic complexity due to multiple call sites, but this doesn't represent true technical debt. The detection stage identifies these patterns, and the adjustment stage reduces their complexity scores to prevent over-penalization.

**Source:** Configuration types defined in `src/config/detection.rs` and `src/priority/scoring/orchestration_adjustment.rs`

#### Boilerplate Detection

The **`[boilerplate_detection]`** section identifies and reduces penalties for boilerplate code patterns. Boilerplate code often inflates complexity metrics without representing true technical debt—it's necessary, repetitive code that doesn't warrant the same scrutiny as business logic.

**Configuration:**

```toml
[boilerplate_detection]
enabled = true                        # Enable boilerplate detection (default: true)
detect_constructors = true            # Detect constructor boilerplate (default: true)
detect_error_conversions = true       # Detect error conversion boilerplate (default: true)
detect_trait_impls = true             # Detect trait implementation boilerplate (default: true)
detect_builders = true                # Detect builder pattern boilerplate (default: true)
detect_test_boilerplate = true        # Detect test setup boilerplate (default: true)
complexity_reduction = 0.20           # Reduce complexity by 20% (default: 0.20)
min_impl_blocks = 20                  # Minimum impl blocks to consider boilerplate (default: 20)
```

**Configuration Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `enabled` | true | Enable boilerplate pattern detection |
| `detect_constructors` | true | Identify constructor initialization boilerplate |
| `detect_error_conversions` | true | Identify error type conversion boilerplate |
| `detect_trait_impls` | true | Identify standard trait implementations |
| `detect_builders` | true | Identify builder pattern methods |
| `detect_test_boilerplate` | true | Identify test setup and fixtures |
| `complexity_reduction` | 0.20 | Percentage to reduce complexity for detected boilerplate (0.0-1.0) |
| `min_impl_blocks` | 20 | Minimum impl blocks before considering type boilerplate-heavy |

**What Boilerplate Patterns Are Detected:**

**1. Constructor Boilerplate** (`detect_constructors`)

Repetitive struct initialization code:

```rust
// Constructor boilerplate - gets 20% complexity reduction
impl Config {
    pub fn new(
        host: String,
        port: u16,
        timeout: Duration,
        retries: u32,
        enable_logging: bool,
        max_connections: usize,
    ) -> Self {
        Self {
            host,
            port,
            timeout,
            retries,
            enable_logging,
            max_connections,
        }
    }
}
```

This has high cyclomatic complexity due to many fields, but it's simple field assignment—not genuine complexity.

**2. Error Conversion Boilerplate** (`detect_error_conversions`)

Standard error type conversions using `From` and `Into` traits:

```rust
// Error conversion boilerplate - gets 20% complexity reduction
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Json(err)
    }
}

// Display trait implementation
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Json(e) => write!(f, "JSON error: {}", e),
            AppError::Network(e) => write!(f, "Network error: {}", e),
        }
    }
}
```

These trait implementations are necessary but contain no business logic—they're mechanical transformations.

**3. Trait Implementation Boilerplate** (`detect_trait_impls`)

Standard trait implementations like `Debug`, `Clone`, `PartialEq`, `Default`:

```rust
// Trait boilerplate - gets 20% complexity reduction
impl std::fmt::Debug for CustomType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("CustomType")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("status", &self.status)
            .field("data", &self.data)
            .finish()
    }
}

impl Clone for CustomType {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            status: self.status,
            data: self.data.clone(),
        }
    }
}
```

**4. Builder Pattern Boilerplate** (`detect_builders`)

Builder pattern methods with many setters:

```rust
// Builder boilerplate - gets 20% complexity reduction
impl ConfigBuilder {
    pub fn host(mut self, host: String) -> Self {
        self.host = Some(host);
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    // ... many more setter methods
}
```

Builder methods are repetitive by design—each setter follows the same pattern.

**5. Test Boilerplate** (`detect_test_boilerplate`)

Test setup, fixtures, and helper functions:

```rust
// Test boilerplate - gets 20% complexity reduction
#[cfg(test)]
mod tests {
    fn setup_test_db() -> TestDatabase {
        TestDatabase::new()
            .with_schema("test_schema")
            .with_migrations()
            .build()
    }

    fn create_test_user() -> User {
        User {
            id: 1,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            role: Role::User,
        }
    }

    // Many similar fixture creation functions...
}
```

**When Boilerplate Reduction Applies:**

The complexity reduction (default 20%) is applied when:
1. Pattern is detected (e.g., constructor, trait impl)
2. Function fits boilerplate characteristics:
   - Uniform structure across multiple similar functions
   - Low average complexity per function
   - High number of impl blocks (> `min_impl_blocks`)

**Example Impact:**

```
Function: Config::new (constructor)
  Raw Complexity: 15 (many parameters)
  Boilerplate Detected: Yes (constructor pattern)
  Adjusted Complexity: 15 × (1 - 0.20) = 12
  Result: Lower priority in debt report
```

**When to Adjust:**

```toml
# Stricter detection - only reduce obvious boilerplate
[boilerplate_detection]
complexity_reduction = 0.10          # Only 10% reduction
min_impl_blocks = 30                 # Require many impl blocks

# More aggressive - reduce more boilerplate
[boilerplate_detection]
complexity_reduction = 0.30          # 30% reduction
min_impl_blocks = 10                 # Lower threshold

# Disable specific patterns
[boilerplate_detection]
detect_constructors = true           # Keep constructor detection
detect_error_conversions = false     # But disable error conversion detection
```

**Why This Matters:**

Without boilerplate detection, constructor-heavy code and trait implementations dominate debt reports, obscuring genuine complexity issues in business logic. The 20% reduction helps focus attention on code that actually needs refactoring.

**Source:** Configuration type defined in `src/organization/boilerplate_detector.rs:257-274`

#### Functional Analysis

The **`[functional_analysis]`** section configures detection of functional programming patterns. Pure functions and immutable patterns typically represent well-designed, maintainable code that should receive favorable scoring treatment.

**Configuration:**

```toml
[functional_analysis]
enabled = true                        # Enable functional pattern detection (default: true)
detect_pure_functions = true          # Detect pure functions (default: true)
detect_higher_order = true            # Detect higher-order functions (default: true)
detect_immutable_patterns = true      # Detect immutable data patterns (default: true)
```

**Configuration Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `enabled` | true | Enable functional programming analysis |
| `detect_pure_functions` | true | Identify functions without side effects or I/O |
| `detect_higher_order` | true | Identify functions that accept/return functions as parameters |
| `detect_immutable_patterns` | true | Identify immutable data structure usage |

**What Each Detection Identifies:**

**1. Pure Functions** (`detect_pure_functions`)

Pure functions have no side effects and always return the same output for the same input. Debtmap looks for:
- No I/O operations (file access, network calls, console output)
- No mutable state modifications
- No external dependencies beyond parameters
- Deterministic return values

**Example patterns detected:**
```rust
// Pure function - simple transformation
fn calculate_total(items: &[Item]) -> f64 {
    items.iter().map(|item| item.price).sum()
}

// Pure function - data transformation
fn normalize_scores(scores: &[f64], max: f64) -> Vec<f64> {
    scores.iter().map(|s| s / max).collect()
}
```

**2. Higher-Order Functions** (`detect_higher_order`)

Higher-order functions accept functions as parameters or return functions as results. These are architectural patterns indicating functional composition:

**Example patterns detected:**
```rust
// Accepts function as parameter
fn apply_transform<F>(data: Vec<i32>, transform: F) -> Vec<i32>
where F: Fn(i32) -> i32 {
    data.into_iter().map(transform).collect()
}

// Returns function
fn make_multiplier(factor: i32) -> impl Fn(i32) -> i32 {
    move |x| x * factor
}
```

**3. Immutable Patterns** (`detect_immutable_patterns`)

Immutable patterns use persistent data structures and avoid mutation. Debtmap recognizes:
- Iterator chains (`.map().filter().collect()`)
- Functional transformations without mutation
- Copy-on-write patterns (`Cow<'_, T>`)
- Builder patterns that consume and return new instances

**Example patterns detected:**
```rust
// Iterator chain transformation
fn process_items(items: Vec<Item>) -> Vec<ProcessedItem> {
    items
        .into_iter()
        .filter(|item| item.is_valid())
        .map(|item| item.process())
        .collect()
}

// Immutable update pattern
fn update_config(config: Config, new_timeout: u64) -> Config {
    Config { timeout: new_timeout, ..config }
}
```

**Scoring Impact:**

Functions exhibiting functional patterns receive favorable treatment:
- **Pure functions**: Reduced coverage penalties (easier to test, fewer edge cases)
- **Higher-order functions**: Recognized as architectural patterns, not accidental complexity
- **Immutable patterns**: Lower risk scores due to predictable behavior

**Tuning Configuration:**

The underlying `FunctionalAnalysisConfig` (defined in `src/analysis/functional_composition.rs:17-28`) provides additional tuning parameters with preset modes:

```rust
// Preset modes available (not directly configurable via TOML)
FunctionalAnalysisConfig::balanced()  // Default preset
FunctionalAnalysisConfig::strict()    // Stricter purity requirements
FunctionalAnalysisConfig::lenient()   // More permissive detection
```

**Source:** Configuration type defined in `src/analysis/functional_composition.rs:17-28`

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
max_high_complexity_count = 100       # DEPRECATED: Use max_debt_density
max_debt_items = 2000                 # DEPRECATED: Use max_debt_density
max_total_debt_score = 10000
max_codebase_risk_score = 7.0
max_high_risk_functions = 50          # DEPRECATED: Use max_debt_density
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

# Classification configuration
[classification.constructors]
detect_constructors = true
constructor_patterns = ["new", "create", "build", "from"]

[classification.accessors]
detect_accessors = true
accessor_patterns = ["get_*", "set_*", "is_*", "has_*"]

[classification.data_flow]
detect_data_flow = true

# Advanced analysis
[orchestration_adjustment]
enabled = true
min_delegation_ratio = 0.6
complexity_reduction = 0.25

[boilerplate_detection]
enabled = true
detect_constructors = true
detect_error_conversions = true
complexity_reduction = 0.20

[functional_analysis]
enabled = true
detect_pure_functions = true
detect_higher_order = true
detect_immutable_patterns = true
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

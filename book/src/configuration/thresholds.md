# Thresholds Configuration

This subsection covers threshold configuration in Debtmap, including basic thresholds, minimum thresholds for filtering, and validation thresholds for CI/CD pipelines.

## Overview

Thresholds control what gets flagged as technical debt. Debtmap provides multiple threshold categories:

- **Basic thresholds** - Core complexity and size limits
- **Minimum thresholds** - Filter out trivial functions
- **Validation thresholds** - CI/CD quality gates
- **Coverage expectations** - Role-based test coverage requirements

## Basic Thresholds

Basic thresholds define when code is flagged as technical debt. Configure them in the `[thresholds]` section of `.debtmap.toml`.

**Source:** `src/config/thresholds.rs:83-118` (`ThresholdsConfig`)

```toml
[thresholds]
complexity = 10                # Cyclomatic complexity threshold (default: 10)
duplication = 50               # Duplication threshold in lines (default: 50)
max_file_length = 500          # Maximum lines per file (default: 500)
max_function_length = 50       # Maximum lines per function (default: 50)
```

### Configuration Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `complexity` | `u32` | 10 | Cyclomatic complexity threshold |
| `duplication` | `u32` | 50 | Minimum duplicate lines to flag |
| `max_file_length` | `usize` | 500 | Maximum lines per file |
| `max_function_length` | `usize` | 50 | Maximum lines per function |

### CLI Override

You can override basic thresholds from the command line:

```bash
# Override cyclomatic complexity threshold
debtmap analyze . --threshold-complexity 15

# Override duplication threshold
debtmap analyze . --threshold-duplication 30

# Combine multiple threshold flags
debtmap analyze . --threshold-complexity 15 --threshold-duplication 30
```

## Minimum Thresholds

Minimum thresholds filter out trivial functions that aren't significant technical debt. This helps focus analysis on meaningful issues.

**Source:** `src/config/thresholds.rs:90-109` (`ThresholdsConfig`)

```toml
[thresholds]
# Filter items below these scores
minimum_debt_score = 2.0              # Only show debt score >= 2.0 (default: none)
minimum_risk_score = 2.0              # Only show risk score >= 2.0 (default: none)
min_score_threshold = 3.0             # Hide LOW severity items (default: 3.0)

# Complexity minimums (filter out simple functions)
minimum_cyclomatic_complexity = 3     # Ignore cyclomatic < 3 (default: none)
minimum_cognitive_complexity = 5      # Ignore cognitive < 5 (default: none)
```

### Configuration Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `minimum_debt_score` | `f64` | None | Minimum debt score to include in results |
| `minimum_risk_score` | `f64` | None | Minimum risk score (0-10) to include |
| `min_score_threshold` | `f64` | 3.0 | Score threshold for recommendations |
| `minimum_cyclomatic_complexity` | `u32` | None | Minimum cyclomatic to analyze |
| `minimum_cognitive_complexity` | `u32` | None | Minimum cognitive to analyze |

### Use Cases

**Focus on High-Priority Issues:**
```toml
[thresholds]
minimum_debt_score = 5.0        # Only show significant debt
minimum_risk_score = 4.0        # Only show meaningful risk
```

**Legacy Codebase (Reduce Noise):**
```toml
[thresholds]
minimum_cyclomatic_complexity = 10   # Ignore moderate complexity
minimum_cognitive_complexity = 15    # Focus on worst offenders
```

## Validation Thresholds

Validation thresholds are used by the `debtmap validate` command to enforce quality gates in CI/CD pipelines.

**Source:** `src/config/thresholds.rs:120-196` (`ValidationThresholds`)

### Primary Quality Metrics (Scale-Independent)

These metrics work for codebases of any size:

```toml
[thresholds.validation]
# Primary quality metrics
max_average_complexity = 10.0      # Maximum average complexity per function (default: 10.0)
max_debt_density = 50.0            # Debt items per 1000 LOC (default: 50.0)
max_codebase_risk_score = 7.0      # Overall risk score 0-10 (default: 7.0)

# Optional metrics
min_coverage_percentage = 0.0       # Minimum test coverage % (default: 0.0 - disabled)

# Safety net
max_total_debt_score = 10000        # Maximum total debt (default: 10000)
```

### Configuration Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_average_complexity` | `f64` | 10.0 | Maximum average complexity per function |
| `max_debt_density` | `f64` | 50.0 | Maximum debt per 1000 LOC (scale-independent) |
| `max_codebase_risk_score` | `f64` | 7.0 | Maximum overall risk score (0-10) |
| `min_coverage_percentage` | `f64` | 0.0 | Minimum required test coverage (0 = disabled) |
| `max_total_debt_score` | `u32` | 10000 | Safety ceiling for total debt |

### Using Validation in CI/CD

```bash
# Run validation (exits with error if thresholds exceeded)
debtmap validate . --config .debtmap.toml

# Example CI/CD pipeline
debtmap analyze . --output report.json
debtmap validate . --config .debtmap.toml || exit 1
```

### Deprecated Fields

The following fields are deprecated since v0.3.0 and will be removed in v1.0:

| Deprecated Field | Replacement |
|------------------|-------------|
| `max_high_complexity_count` | Use `max_debt_density` |
| `max_debt_items` | Use `max_debt_density` |
| `max_high_risk_functions` | Use `max_debt_density` + `max_codebase_risk_score` |

**Migration Example:**

```toml
# Old (deprecated)
[thresholds.validation]
max_debt_items = 100

# New (scale-independent)
[thresholds.validation]
max_debt_density = 50.0  # Works for any codebase size
```

## Coverage Expectations

Coverage expectations define role-based test coverage requirements. Different function types have different testing strategies.

**Source:** `src/priority/scoring/coverage_expectations.rs:103-173`

```toml
# High expectations for pure functions
[coverage_expectations.pure]
min = 90.0                # Minimum acceptable coverage
target = 95.0             # Target/ideal coverage
max = 100.0               # Maximum meaningful coverage

# Moderate expectations for I/O operations
[coverage_expectations.io_operations]
min = 60.0
target = 70.0
max = 80.0

# Low expectations for debug/diagnostic code
[coverage_expectations.debug]
min = 20.0
target = 30.0
max = 40.0
```

### Default Coverage Expectations by Role

| Function Role | Min | Target | Max | Rationale |
|---------------|-----|--------|-----|-----------|
| Pure Logic | 90% | 95% | 100% | Easy to unit test, should be comprehensive |
| Business Logic | 80% | 90% | 95% | Core functionality requires thorough testing |
| Validation | 85% | 92% | 98% | Input validation is critical |
| State Management | 75% | 85% | 90% | State transitions need coverage |
| Error Handling | 70% | 80% | 90% | Error paths should be tested |
| Orchestration | 65% | 75% | 85% | Tested via higher-level tests |
| Configuration | 60% | 70% | 80% | Often integration tested |
| I/O Operations | 60% | 70% | 80% | Often integration tested |
| Initialization | 50% | 65% | 75% | Setup code with less testing priority |
| Performance | 40% | 50% | 60% | Optimization code with lower priority |
| Debug | 20% | 30% | 40% | Diagnostic code has lowest priority |

### Complete Coverage Configuration

```toml
# Strict quality standards
[coverage_expectations.pure]
min = 95.0
target = 98.0
max = 100.0

[coverage_expectations.business_logic]
min = 90.0
target = 95.0
max = 98.0

[coverage_expectations.validation]
min = 92.0
target = 96.0
max = 100.0

# More lenient for I/O
[coverage_expectations.io_operations]
min = 50.0
target = 65.0
max = 75.0
```

## Complexity Thresholds

For fine-grained control over function complexity detection, use the `[complexity_thresholds]` section.

**Source:** `src/complexity/threshold_manager.rs:16-58` (`ComplexityThresholds`)

```toml
[complexity_thresholds]
# Core complexity metrics
minimum_total_complexity = 8        # Sum of cyclomatic + cognitive (default: 8)
minimum_cyclomatic_complexity = 5   # Decision points (default: 5)
minimum_cognitive_complexity = 10   # Mental effort to understand (default: 10)

# Structural complexity metrics
minimum_match_arms = 4              # Maximum match/switch arms (default: 4)
minimum_if_else_chain = 3           # Maximum if-else chain length (default: 3)
minimum_function_length = 20        # Minimum lines before flagging (default: 20)

# Role-based multipliers (adjust thresholds by function role)
entry_point_multiplier = 1.5        # main(), handlers - more lenient (default: 1.5)
core_logic_multiplier = 1.0         # Business logic - standard (default: 1.0)
utility_multiplier = 0.8            # Getters, setters - stricter (default: 0.8)
test_function_multiplier = 2.0      # Test functions - most lenient (default: 2.0)
```

### Threshold Presets

Use `--threshold-preset` to apply predefined configurations:

```bash
# Strict preset (new projects, libraries)
debtmap analyze . --threshold-preset strict

# Balanced preset (default - typical projects)
debtmap analyze . --threshold-preset balanced

# Lenient preset (legacy codebases)
debtmap analyze . --threshold-preset lenient
```

**Preset Comparison:**

| Threshold | Strict | Balanced | Lenient |
|-----------|--------|----------|---------|
| Cyclomatic Complexity | 3 | 5 | 10 |
| Cognitive Complexity | 7 | 10 | 20 |
| Total Complexity | 5 | 8 | 15 |
| Function Length | 15 | 20 | 50 |
| Match Arms | 3 | 4 | 8 |
| If-Else Chain | 2 | 3 | 5 |

**Source:** `src/complexity/threshold_manager.rs:120-148` (`from_preset`)

### Role-Based Multiplier Examples

Role multipliers adjust ALL thresholds for different function types:

**Entry Point with Balanced preset (1.5x multiplier):**
- Cyclomatic threshold: 7.5 (5 x 1.5)
- Cognitive threshold: 15 (10 x 1.5)
- Total threshold: 12 (8 x 1.5)

**Utility function with Balanced preset (0.8x multiplier):**
- Cyclomatic threshold: 4 (5 x 0.8)
- Cognitive threshold: 8 (10 x 0.8)
- Total threshold: 6.4 (8 x 0.8)

### Validation Rules

Debtmap validates thresholds to prevent misconfiguration:

**Source:** `src/complexity/threshold_manager.rs:191-217`

- Core complexity metrics must be > 0
- Role multipliers must be positive (> 0)
- Invalid configurations fall back to defaults with a warning

## File Size Thresholds

Context-aware file size limits vary by file type to avoid unrealistic recommendations.

**Source:** `src/config/thresholds.rs:293-375` (`FileSizeThresholds`)

```toml
[thresholds.file_size]
business_logic = 400          # Strict limit for business logic (default: 400)
test_code = 650               # Moderate limit for tests (default: 650)
declarative_config = 1200     # Lenient for config files (default: 1200)
generated_code = 5000         # Very lenient for generated (default: 5000)
proc_macro = 500              # Moderate-strict for macros (default: 500)
build_script = 300            # Strict for build scripts (default: 300)
min_lines_per_function = 3.0  # Safety threshold (default: 3.0)
```

### File-Specific Overrides

Use glob patterns for file-specific thresholds:

```toml
[thresholds.file_size.overrides]
"src/generated/*.rs" = 10000    # Allow large generated files
"src/migrations/*.rs" = 2000    # Allow larger migration files
```

## Related Topics

- [Configuration Overview](../configuration.md) - Full configuration reference
- [Threshold Configuration Guide](../threshold-configuration.md) - Extended guide with examples
- [Scoring Strategies](../scoring-strategies.md) - How thresholds affect scoring
- [Validation and Quality Gates](../validation-gates.md) - CI/CD integration

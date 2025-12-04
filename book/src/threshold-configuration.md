# Threshold Configuration

Debtmap uses configurable thresholds to determine when code complexity, duplication, or structural issues should be flagged as technical debt. This chapter explains how to configure thresholds to match your project's quality standards.

## Overview

Thresholds control what gets flagged as technical debt. You can configure thresholds using:

1. **Preset configurations** - Quick start with strict, balanced, or lenient settings
2. **CLI flags** - Override thresholds for a single analysis run
3. **Configuration file** - Project-specific thresholds in `.debtmap.toml`

## Threshold Presets

Debtmap provides three preset threshold configurations to match different project needs:

### Preset Comparison

| Threshold | Strict | Balanced (Default) | Lenient |
|-----------|--------|-------------------|---------|
| Cyclomatic Complexity | 3 | 5 | 10 |
| Cognitive Complexity | 7 | 10 | 20 |
| Total Complexity | 5 | 8 | 15 |
| Function Length (lines) | 15 | 20 | 50 |
| Match Arms | 3 | 4 | 8 |
| If-Else Chain | 2 | 3 | 5 |
| **Role Multipliers** | | | |
| Entry Point Multiplier | 1.2x | 1.5x | 2.0x |
| Utility Multiplier | 0.6x | 0.8x | 1.0x |
| Test Function Multiplier | 3.0x | 2.0x | 3.0x |

### When to Use Each Preset

**Strict Preset**
- New projects aiming for high code quality standards
- Libraries and reusable components
- Critical systems requiring high reliability
- Teams enforcing strict coding standards
```bash
debtmap analyze . --threshold-preset strict
```

**Balanced Preset (Default)**
- Typical production applications
- Projects with moderate complexity tolerance
- Recommended starting point for most projects
- Good balance between catching issues and avoiding false positives

```bash
debtmap analyze .  # Uses balanced preset by default
```

**Lenient Preset**
- Legacy codebases during initial assessment
- Complex domains (compilers, scientific computing)
- Gradual debt reduction strategies
- Temporary relaxation during major refactoring
```bash
debtmap analyze . --threshold-preset lenient
```

## Understanding Complexity Thresholds

Debtmap tracks multiple complexity metrics and uses **conjunction logic**: a function must exceed **ALL** thresholds to be flagged as technical debt.

### Cyclomatic Complexity

Counts decision points in code: `if`, `while`, `for`, `match`, `&&`, `||`, etc.

- **What it measures**: Number of independent paths through code
- **Why it matters**: More paths = harder to test completely
- **Default threshold**: 5

### Cognitive Complexity

Measures the mental effort required to understand code by weighing nested structures and breaks in linear flow.

- **What it measures**: How hard code is to read and comprehend
- **Why it matters**: High cognitive load = maintenance burden
- **Default threshold**: 10

### Total Complexity

Sum of cyclomatic and cognitive complexity.

- **What it measures**: Combined complexity burden
- **Why it matters**: Catches functions high in either metric
- **Default threshold**: 8

### Function Length

Number of lines of code in the function body.

- **What it measures**: Physical size of function
- **Why it matters**: Long functions are hard to understand and test
- **Default threshold**: 20 lines

### Structural Complexity

Additional metrics for specific patterns:

- **Match arms**: Flags large match/switch statements (default: 4)
- **If-else chains**: Flags long conditional chains (default: 3)

**Critical**: Debtmap uses **conjunction logic** - functions are flagged only when they meet **ALL** of these conditions simultaneously:
- Cyclomatic complexity >= adjusted cyclomatic threshold
- Cognitive complexity >= adjusted cognitive threshold
- Function length >= minimum function length
- Total complexity (cyclomatic + cognitive) >= adjusted total threshold

The thresholds are first adjusted by role-based multipliers, then all four checks must pass for the function to be flagged.

### Threshold Validation

Debtmap validates critical thresholds to prevent misconfiguration. The following validation rules apply (src/complexity/threshold_manager.rs:191-217):

**Core Complexity Metrics** (must not be zero):
- `minimum_total_complexity` > 0
- `minimum_cyclomatic_complexity` > 0
- `minimum_cognitive_complexity` > 0

**Role Multipliers** (must be positive):
- `entry_point_multiplier` > 0
- `core_logic_multiplier` > 0
- `utility_multiplier` > 0
- `test_function_multiplier` > 0

**Note**: Structural thresholds (`minimum_match_arms`, `minimum_if_else_chain`, `minimum_function_length`) are not validated and can be set to any value including zero. Zero values effectively disable those checks.

If any validated field fails validation, Debtmap will reject the configuration with an error message and use default values instead.

## Role-Based Multipliers

Debtmap automatically adjusts thresholds based on function role, recognizing that different types of functions have different complexity expectations:

| Function Role | Multiplier | Effect | Examples |
|---------------|------------|--------|----------|
| Entry Points | 1.2x - 2.0x (preset-specific) | More lenient | `main()`, HTTP handlers, CLI commands |
| Core Logic | 1.0x | Standard | Business logic, algorithms |
| Utility Functions | 0.6x - 1.0x (preset-specific) | Stricter | Getters, setters, simple helpers |
| Test Functions | 2.0x - 3.0x (preset-specific) | Most lenient | Unit tests, integration tests |
| Unknown Functions | 1.0x (defaults to core logic) | Standard | Functions that don't match any role pattern |

**Note**: Some multipliers vary by preset:
- **Entry Points**: Strict=1.2x, Balanced=1.5x, Lenient=2.0x
- **Utility Functions**: Strict=0.6x, Balanced=0.8x, Lenient=1.0x
- **Test Functions**: Strict=3.0x, Balanced=2.0x, Lenient=3.0x

**How multipliers work:**

A higher multiplier makes thresholds more lenient by adjusting ALL thresholds. The multiplier values vary by preset - for example, entry point functions use 1.2x (strict), 1.5x (balanced), or 2.0x (lenient).

**Example: Entry Point function with Balanced preset (multiplier = 1.5x):**
- Cyclomatic threshold: 7.5 (5 × 1.5)
- Cognitive threshold: 15 (10 × 1.5)
- Total threshold: 12 (8 × 1.5)
- Length threshold: 30 lines (20 × 1.5)

**The function is flagged only if ALL conditions are met:**
- Cyclomatic complexity >= 7.5 AND
- Cognitive complexity >= 15 AND
- Function length >= 30 lines AND
- Total complexity (cyclomatic + cognitive) >= 12

**Comparison across roles (Balanced preset):**

| Role | Cyclomatic | Cognitive | Total | Length | Flagged When |
|------|-----------|-----------|-------|--------|--------------|
| Entry Point (1.5x) | 7.5 | 15 | 12 | 30 | ALL conditions met |
| Core Logic (1.0x) | 5 | 10 | 8 | 20 | ALL conditions met |
| Utility (0.8x) | 4 | 8 | 6.4 | 16 | ALL conditions met |
| Test (2.0x) | 10 | 20 | 16 | 40 | ALL conditions met |

**Note**: Entry point multipliers differ by preset. With the strict preset, entry points use 1.2x (cyclomatic=3.6, cognitive=8.4), while the lenient preset uses 2.0x (cyclomatic=20, cognitive=40).

This allows test functions and entry points to be more complex without false positives, while keeping utility functions clean and simple.

## CLI Threshold Flags

Override thresholds for a single analysis run using command-line flags:

### Preset-Based Configuration (Recommended)

Use `--threshold-preset` to apply a predefined threshold configuration:

```bash
# Use strict preset (cyclomatic=3, cognitive=7, total=5, length=15)
debtmap analyze . --threshold-preset strict

# Use balanced preset (default - cyclomatic=5, cognitive=10, total=8, length=20)
debtmap analyze . --threshold-preset balanced

# Use lenient preset (cyclomatic=10, cognitive=20, total=15, length=50)
debtmap analyze . --threshold-preset lenient
```

### Individual Threshold Overrides

You can also override specific thresholds:

```bash
# Override cyclomatic complexity threshold (legacy flag, default: 10)
debtmap analyze . --threshold-complexity 15

# Override duplication threshold in lines (default: 50)
debtmap analyze . --threshold-duplication 30

# Combine multiple threshold flags
debtmap analyze . --threshold-complexity 15 --threshold-duplication 30
```

**Note**:
- `--threshold-preset` provides the most comprehensive threshold configuration (includes all complexity metrics and role multipliers)
- Individual flags like `--threshold-complexity` are legacy flags that only set a single cyclomatic complexity threshold, without configuring cognitive complexity, total complexity, function length, or role multipliers
- For full control over all complexity metrics and role-based multipliers, use the `.debtmap.toml` configuration file
- CLI flags override configuration file settings for that run only

## Configuration File

For project-specific thresholds, create a `.debtmap.toml` file in your project root.

### Complexity Thresholds Configuration

The `[complexity_thresholds]` section in `.debtmap.toml` allows fine-grained control over function complexity detection:

```toml
[complexity_thresholds]
# Core complexity metrics
minimum_total_complexity = 8        # Sum of cyclomatic + cognitive
minimum_cyclomatic_complexity = 5   # Decision points (if, match, etc.)
minimum_cognitive_complexity = 10   # Mental effort to understand code

# Structural complexity metrics
minimum_match_arms = 4              # Maximum match/switch arms
minimum_if_else_chain = 3           # Maximum if-else chain length
minimum_function_length = 20        # Minimum lines before flagging

# Role-based multipliers (applied to all thresholds above)
entry_point_multiplier = 1.5        # main(), handlers, CLI commands
core_logic_multiplier = 1.0         # Standard business logic
utility_multiplier = 0.8            # Getters, setters, helpers
test_function_multiplier = 2.0      # Unit tests, integration tests
```

**Note**: The multipliers are applied to thresholds before comparison. For example, with `entry_point_multiplier = 1.5` and `minimum_cyclomatic_complexity = 5`, an entry point function would be flagged at cyclomatic complexity 7.5 (5 × 1.5).

**Validation**: Core complexity metrics (`minimum_total_complexity`, `minimum_cyclomatic_complexity`, `minimum_cognitive_complexity`) and all role multipliers must be positive (> 0). Zero or negative values for these fields will cause validation errors and Debtmap will use default values. Structural thresholds (`minimum_match_arms`, `minimum_if_else_chain`, `minimum_function_length`) are not validated and can be set to zero to disable those checks.

### Complete Example

```toml
# Legacy threshold settings (simple configuration)
# Note: For comprehensive control, use [complexity_thresholds] instead
[thresholds]
complexity = 15                # Cyclomatic complexity threshold (legacy)
cognitive = 20                 # Cognitive complexity threshold (legacy)
max_file_length = 500         # Maximum file length in lines

# Validation thresholds for CI/CD
[thresholds.validation]
max_average_complexity = 10.0      # Maximum average complexity across codebase
max_debt_density = 50.0           # Maximum debt items per 1000 LOC
max_codebase_risk_score = 7.0     # Maximum overall risk score
min_coverage_percentage = 0.0     # Minimum test coverage (0 = disabled)
max_total_debt_score = 10000      # Safety net for total debt score

# God object detection
[god_object]
enabled = true

# Rust-specific thresholds
[god_object.rust]
max_methods = 20        # Maximum methods before flagging as god object
max_fields = 15         # Maximum fields
max_traits = 5          # Maximum trait implementations
max_lines = 1000        # Maximum lines in impl block
max_complexity = 200    # Maximum total complexity

# Python-specific thresholds
[god_object.python]
max_methods = 15
max_fields = 10
max_traits = 3
max_lines = 500
max_complexity = 150

# JavaScript/TypeScript-specific thresholds
[god_object.javascript]
max_methods = 15
max_fields = 20
max_traits = 3
max_lines = 500
max_complexity = 150
```

**Configuration Section Notes:**
- **`[thresholds]`**: Legacy/simple threshold configuration. Sets basic complexity thresholds without role multipliers or comprehensive metric control.
- **`[complexity_thresholds]`**: Modern/comprehensive threshold configuration. Provides fine-grained control over all complexity metrics, structural thresholds, and role-based multipliers. Use this for full control.
- **Recommendation**: For new projects, use `[complexity_thresholds]` for comprehensive configuration. The `[thresholds]` section is maintained for backward compatibility.

### Using Configuration File

```bash
# Initialize with default configuration
debtmap init

# Edit .debtmap.toml to customize thresholds
# Then run analysis (automatically uses config file)
debtmap analyze .

# Validate against thresholds in CI/CD
debtmap validate . --config .debtmap.toml
```

## God Object Thresholds

God objects are classes/structs with too many responsibilities. Debtmap uses language-specific thresholds to detect them:

### Rust Thresholds

```toml
[god_object.rust]
max_methods = 20        # Methods in impl blocks
max_fields = 15         # Struct fields
max_traits = 5          # Trait implementations
max_lines = 1000        # Lines in impl blocks
max_complexity = 200    # Total complexity
```

### Python Thresholds

```toml
[god_object.python]
max_methods = 15
max_fields = 10
max_traits = 3          # Base classes
max_lines = 500
max_complexity = 150
```

### JavaScript/TypeScript Thresholds

```toml
[god_object.javascript]
max_methods = 15
max_fields = 20
max_traits = 3          # Extended classes
max_lines = 500
max_complexity = 150
```

**Why language-specific thresholds?**

Different languages have different idioms:
- **Rust**: Encourages small traits and composition, so lower thresholds
- **Python**: Duck typing allows more fields, but fewer methods
- **JavaScript**: Prototype-based, typically has more properties

## Validation Thresholds

Use validation thresholds in CI/CD pipelines to enforce quality gates:

### Scale-Independent Metrics (Recommended)

These metrics work for codebases of any size:

```toml
[thresholds.validation]
# Average complexity per function (default: 10.0)
max_average_complexity = 10.0

# Debt items per 1000 lines of code (default: 50.0)
max_debt_density = 50.0

# Overall risk score 0-10 (default: 7.0)
max_codebase_risk_score = 7.0
```

### Optional Metrics

```toml
[thresholds.validation]
# Minimum test coverage percentage (default: 0.0 = disabled)
min_coverage_percentage = 80.0

# Safety net for total debt score (default: 10000)
max_total_debt_score = 5000
```

### Using Validation in CI/CD

```bash
# Run validation (exits with error if thresholds exceeded)
debtmap validate . --config .debtmap.toml

# Example CI/CD workflow
debtmap analyze . --output report.json
debtmap validate . --config .debtmap.toml || exit 1
```

**CI/CD Best Practices:**
- Start with lenient thresholds to establish baseline
- Gradually tighten thresholds as you pay down debt
- Use `max_debt_density` for stable quality metric
- Track trends over time, not just point-in-time values

## Tuning Guidelines

How to choose and adjust thresholds for your project:

### 1. Start with Defaults

Begin with the balanced preset to understand your codebase:

```bash
debtmap analyze .
```

Review the output to see what gets flagged and what doesn't.

### 2. Run Baseline Analysis

Understand your current state:

```bash
# Analyze and save results
debtmap analyze . --output baseline.json

# Review high-priority items
debtmap analyze . --top 20
```

### 3. Adjust Based on Project Type

**New Projects:**
- Use strict preset to enforce high quality from the start
- Prevents accumulation of technical debt

**Typical Projects:**
- Use balanced preset (recommended)
- Good middle ground for most teams

**Legacy Codebases:**
- Use lenient preset initially
- Focus on worst offenders first
- Gradually tighten thresholds as you refactor

### 4. Fine-Tune in Configuration File

Create `.debtmap.toml` and adjust specific thresholds:

```bash
# Initialize config file
debtmap init

# Edit .debtmap.toml
# Adjust thresholds based on your baseline analysis
```

### 5. Validate and Iterate

```bash
# Test your thresholds
debtmap validate . --config .debtmap.toml

# Adjust if needed
# Iterate until you find the right balance
```

### Troubleshooting Threshold Configuration

**Too many false positives?**
- Increase thresholds or switch to lenient preset
- Check if role multipliers are appropriate
- Review god object thresholds for your language

**Missing important issues?**
- Decrease thresholds or switch to strict preset
- Verify `.debtmap.toml` is being loaded
- Check for suppression patterns hiding issues

**Different standards for tests?**
- Don't worry - role multipliers automatically handle this
- Test functions get 2-3x multiplier by default

**Inconsistent results?**
- Ensure `.debtmap.toml` is in project root
- CLI flags override config file - remove them for consistency
- Use `--config` flag to specify config file explicitly

## Examples

### Example 1: Quick Analysis with Strict Preset

```bash
# Use strict thresholds for new project
debtmap analyze . --threshold-preset strict
```

### Example 2: Custom CLI Thresholds

```bash
# Analyze with custom thresholds (no config file)
debtmap analyze . \
  --threshold-complexity 15 \
  --threshold-duplication 30
```

### Example 3: Project-Specific Configuration

```bash
# Initialize configuration
debtmap init

# Creates .debtmap.toml - edit to customize
# Example: Increase complexity threshold to 15

# Run analysis with project config
debtmap analyze .
```

### Example 4: CI/CD Validation

```bash
# Create strict validation configuration
cat > .debtmap.toml << EOF
[thresholds.validation]
max_average_complexity = 8.0
max_debt_density = 30.0
max_codebase_risk_score = 6.0
min_coverage_percentage = 75.0
EOF

# Run in CI/CD pipeline
debtmap analyze . --output report.json
debtmap validate . --config .debtmap.toml
```

### Example 5: Gradual Debt Reduction

```bash
# Month 1: Start lenient
debtmap analyze . --threshold-preset lenient --output month1.json

# Month 2: Switch to balanced
debtmap analyze . --threshold-preset balanced --output month2.json

# Month 3: Tighten further
debtmap analyze . --threshold-preset strict --output month3.json

# Compare progress
debtmap analyze . --output current.json
# Review trend: month1.json -> month2.json -> month3.json -> current.json
```

## Decision Tree: Choosing Your Preset

```
Start here: What kind of project are you working on?
│
├─ New project or library
│  └─ Use STRICT preset
│     └─ Prevent debt accumulation from day one
│
├─ Existing production application
│  └─ What's your goal?
│     ├─ Maintain current quality
│     │  └─ Use BALANCED preset
│     │     └─ Good default for most teams
│     │
│     └─ Reduce existing debt gradually
│        └─ Start with LENIENT preset
│           └─ Focus on worst issues first
│           └─ Tighten thresholds over time
│
└─ Legacy codebase or complex domain
   └─ Use LENIENT preset
      └─ Avoid overwhelming with false positives
      └─ Create baseline and improve incrementally
```

## Best Practices

1. **Start with defaults** - Don't over-customize initially
2. **Track trends** - Monitor debt over time, not just point values
3. **Be consistent** - Use same thresholds across team
4. **Document choices** - Comment your `.debtmap.toml` to explain custom thresholds
5. **Automate validation** - Run `debtmap validate` in CI/CD
6. **Review regularly** - Reassess thresholds quarterly
7. **Gradual tightening** - Don't make thresholds stricter too quickly
8. **Trust role multipliers** - Let Debtmap handle different function types

## Related Topics

- [Getting Started](getting-started.md) - Initial setup and first analysis
- [CLI Reference](cli-reference.md) - Complete command-line flag documentation
- [Configuration](configuration.md) - Full `.debtmap.toml` reference
- [Scoring Strategies](scoring-strategies.md) - How thresholds affect debt scores
- [God Object Detection](god-object-detection.md) - Deep dive into god object analysis

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

Debtmap tracks multiple complexity metrics. A function must exceed **ALL** thresholds to be flagged:

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

**Important**: Functions are only flagged when they exceed ALL applicable thresholds, not just one.

## Role-Based Multipliers

Debtmap automatically adjusts thresholds based on function role, recognizing that different types of functions have different complexity expectations:

| Function Role | Multiplier | Effect | Examples |
|---------------|------------|--------|----------|
| Entry Points | 1.5x | More lenient | `main()`, HTTP handlers, CLI commands |
| Core Logic | 1.0x | Standard | Business logic, algorithms |
| Utility Functions | 0.8x | Stricter | Getters, setters, simple helpers |
| Test Functions | 2.0x - 3.0x | Most lenient | Unit tests, integration tests |

**How multipliers work:**

A higher multiplier makes thresholds more lenient. For example, with balanced preset (cyclomatic=5):
- Entry point: flagged at complexity 8 (5 × 1.5)
- Core logic: flagged at complexity 5 (5 × 1.0)
- Utility function: flagged at complexity 4 (5 × 0.8)
- Test function: flagged at complexity 10+ (5 × 2.0-3.0)

This allows test functions and entry points to be more complex without false positives, while keeping utility functions clean and simple.

## CLI Threshold Flags

Override thresholds for a single analysis run:

### Complexity Threshold

```bash
# Set cyclomatic complexity threshold
debtmap analyze . --threshold-complexity 15
```

### Duplication Threshold

```bash
# Set duplication threshold (in lines)
debtmap analyze . --threshold-duplication 30
```

### Combining Flags

```bash
# Use custom thresholds for both complexity and duplication
debtmap analyze . --threshold-complexity 15 --threshold-duplication 30
```

### Preset Flag

```bash
# Use a preset configuration
debtmap analyze . --threshold-preset strict
```

**Note**: CLI flags override configuration file settings for that run only.

## Configuration File

For project-specific thresholds, create a `.debtmap.toml` file in your project root.

### Basic Threshold Configuration

```toml
[thresholds]
# Complexity thresholds
complexity = 15
cognitive = 20
max_file_length = 500

# Structural thresholds
match_arms = 6
if_else_chain = 4
function_length = 25
```

### Complete Example

```toml
# Complexity thresholds for flagging functions
[thresholds]
complexity = 15                # Cyclomatic complexity threshold
cognitive = 20                 # Cognitive complexity threshold
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

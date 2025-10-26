# Threshold Configuration

Debtmap provides flexible threshold configuration through presets and custom settings. This chapter covers threshold presets (strict, balanced, lenient), language-specific thresholds, and tuning guidelines.

## Overview

Thresholds determine when code is flagged for:
- Complexity violations
- God object detection
- Function length limits
- Parameter count
- Nesting depth

## Threshold Presets

### Strict Preset

**Purpose:** High code quality standards for new projects

**Use when:**
- Starting new greenfield projects
- Building critical infrastructure
- Aiming for maximum maintainability

**Values:**
```toml
[thresholds]
preset = "strict"
max_cyclomatic_complexity = 5
max_cognitive_complexity = 10
max_function_length = 25
max_parameter_count = 3
max_nesting_depth = 3
```

**Command:**
```bash
debtmap analyze . --threshold-preset strict
```

### Balanced Preset (Default)

**Purpose:** Typical project standards

**Use when:**
- Working on established projects
- Balancing quality and velocity
- Industry-standard expectations

**Values:**
```toml
[thresholds]
preset = "balanced"
max_cyclomatic_complexity = 10
max_cognitive_complexity = 15
max_function_length = 50
max_parameter_count = 5
max_nesting_depth = 4
```

**Command:**
```bash
debtmap analyze . --threshold-preset balanced
```

### Lenient Preset

**Purpose:** Legacy code or complex domains

**Use when:**
- Analyzing legacy codebases
- Working in complex domains (compilers, graphics, etc.)
- Gradual improvement strategy

**Values:**
```toml
[thresholds]
preset = "lenient"
max_cyclomatic_complexity = 20
max_cognitive_complexity = 25
max_function_length = 100
max_parameter_count = 7
max_nesting_depth = 5
```

**Command:**
```bash
debtmap analyze . --threshold-preset lenient
```

## Language-Specific Thresholds

### Rust Thresholds

```toml
[thresholds.rust]
max_methods = 20
max_fields = 15
max_traits = 5
max_lines = 1000
max_complexity = 200
```

### Python Thresholds

```toml
[thresholds.python]
max_methods = 15
max_fields = 10
max_traits = 3
max_lines = 500
max_complexity = 150
```

### JavaScript/TypeScript Thresholds

```toml
[thresholds.javascript]
max_methods = 15
max_fields = 20
max_traits = 3
max_lines = 500
max_complexity = 150
```

## Custom Threshold Configuration

### Create .debtmap.toml

```toml
[thresholds]
# Use preset as base
preset = "balanced"

# Override specific thresholds
max_cyclomatic_complexity = 12
max_cognitive_complexity = 18

# Complexity-specific
max_function_length = 60
max_parameter_count = 4
max_nesting_depth = 4

[thresholds.rust]
max_methods = 18
max_fields = 12
max_complexity = 180

[thresholds.python]
max_methods = 12
max_fields = 8
max_complexity = 120
```

### Apply Configuration

```bash
debtmap analyze . --config .debtmap.toml
```

## God Object Thresholds

Configure detection sensitivity:

```toml
[god_object]
max_methods = 20
max_fields = 15
max_responsibilities = 3
max_lines = 1000
max_complexity = 200

# Minimum violations to flag
min_violations = 3

# Score threshold (default: 70.0)
score_threshold = 70.0
```

## Tuning Guidelines

### Progressive Tightening

Start lenient, gradually tighten:

```bash
# Week 1: Establish baseline
debtmap analyze . --threshold-preset lenient

# Week 4: Move to balanced
debtmap analyze . --threshold-preset balanced

# Month 3: Consider strict for new code
debtmap analyze new_features/ --threshold-preset strict
```

### Path-Specific Thresholds

Apply different thresholds to different paths:

```toml
[[thresholds.paths]]
pattern = "src/core/**"
preset = "strict"

[[thresholds.paths]]
pattern = "src/legacy/**"
preset = "lenient"

[[thresholds.paths]]
pattern = "src/**"
preset = "balanced"
```

### Category-Specific Thresholds

```toml
[thresholds.categories]
complexity = { max = 10, severity = "high" }
organization = { max = 20, severity = "medium" }
code_smell = { max = 5, severity = "low" }
```

## Best Practices

**Start conservative:**
- Begin with lenient preset
- Measure baseline debt density
- Gradually tighten over sprints

**Adapt to domain:**
- Business logic: strict thresholds
- Infrastructure code: balanced
- Legacy code: lenient initially

**Monitor impact:**
- Track debt density over time
- Measure developer feedback
- Adjust based on false positive rate

**Different rules for different code:**
- Core modules: stricter
- Tests: more lenient
- Scripts/tools: lenient
- Public APIs: strict

## Command-Line Override

Override config file thresholds:

```bash
# Override complexity threshold
debtmap analyze . --threshold-complexity 8

# Override multiple thresholds
debtmap analyze . \
  --threshold-complexity 10 \
  --max-function-length 40 \
  --max-parameter-count 4
```

## Use Cases

### Enforce Quality Standards

```bash
# CI/CD quality gate
debtmap validate . --threshold-preset strict --max-debt-density 30.0
```

### Legacy Code Analysis

```bash
# Gentle analysis for legacy
debtmap analyze legacy/ --threshold-preset lenient
```

### New Feature Development

```bash
# Strict standards for new code
debtmap analyze src/new_feature/ --threshold-preset strict
```

### Gradual Improvement

```bash
# Progressive threshold tightening
# Month 1
debtmap validate . --threshold-preset lenient --max-debt-density 100.0

# Month 3
debtmap validate . --threshold-preset balanced --max-debt-density 75.0

# Month 6
debtmap validate . --threshold-preset strict --max-debt-density 50.0
```

## Troubleshooting

### Too Many Violations

**Issue:** Overwhelming number of debt items

**Solution:**
- Start with lenient preset
- Use `--top N` to focus on worst
- Gradually tighten thresholds
- Focus on categories incrementally

### Too Few Violations

**Issue:** Not catching obvious issues

**Solution:**
- Use strict preset
- Lower complexity thresholds
- Enable all analysis features
- Review suppression patterns

### Inconsistent Results

**Issue:** Results vary across runs

**Solution:**
- Use config file instead of CLI args
- Commit `.debtmap.toml` to version control
- Document threshold decisions
- Version control threshold changes

## See Also

- [Validation and Quality Gates](validation-gates.md) - Using thresholds in CI/CD
- [Configuration](configuration.md) - Complete configuration reference
- [God Object Detection](god-object-detection.md) - God object thresholds

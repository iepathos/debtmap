# Validation and Quality Gates

The `validate` command enables automated quality gates for continuous integration, ensuring that code changes don't exceed technical debt thresholds. This chapter covers validation configuration and CI/CD integration.

## Overview

Validation and quality gates help you:
- Enforce code quality standards automatically
- Prevent technical debt from accumulating
- Gate deployments on quality metrics
- Track compliance over time
- Fail builds that exceed thresholds

## Basic Usage

### Simple Validation

```bash
# Validate against default thresholds
debtmap validate .

# Validate with custom debt density threshold
debtmap validate . --max-debt-density 50.0

# Validate with configuration file
debtmap validate . --config .debtmap.toml
```

### Exit Codes

The validate command uses exit codes for automation:
- **0**: Validation passed, no threshold violations
- **1**: Validation failed, thresholds exceeded
- **2**: Error occurred during validation

## Configuration

### Debt Density Threshold

Debt density is measured as debt score per 1000 lines of code:

```bash
# Strict threshold
debtmap validate . --max-debt-density 25.0

# Balanced threshold (default)
debtmap validate . --max-debt-density 50.0

# Lenient threshold
debtmap validate . --max-debt-density 100.0
```

### Configuration File

Create `.debtmap.toml` for comprehensive validation rules:

```toml
[thresholds]
max_debt_density = 50.0
max_cyclomatic_complexity = 10
max_cognitive_complexity = 15
max_function_length = 50

[validation]
fail_on_critical = true
fail_on_high = false
min_coverage_percent = 70.0

[exclusions]
paths = ["tests/", "examples/", "benches/"]
```

### Threshold Presets

Use predefined threshold presets:

```bash
# Strict standards
debtmap validate . --threshold-preset strict

# Balanced standards (default)
debtmap validate . --threshold-preset balanced

# Lenient for legacy code
debtmap validate . --threshold-preset lenient
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Quality Gate

on:
  pull_request:
  push:
    branches: [main, develop]

jobs:
  validate-debt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Install debtmap
        run: cargo install debtmap

      - name: Run validation
        run: debtmap validate . --max-debt-density 50.0

      - name: Upload results
        if: always()
        uses: actions/upload-artifact@v2
        with:
          name: debt-report
          path: debt-report.json
```

### GitLab CI

```yaml
debt-validation:
  stage: test
  script:
    - cargo install debtmap
    - debtmap validate . --config .debtmap.toml --format json --output debt-report.json
  artifacts:
    reports:
      codequality: debt-report.json
    when: always
  rules:
    - if: '$CI_PIPELINE_SOURCE == "merge_request_event"'
```

### Jenkins Pipeline

```groovy
pipeline {
    agent any

    stages {
        stage('Quality Gate') {
            steps {
                sh 'cargo install debtmap'
                sh 'debtmap validate . --max-debt-density 50.0 --format json --output debt-report.json'
            }
            post {
                always {
                    archiveArtifacts artifacts: 'debt-report.json'
                }
                failure {
                    echo 'Quality gate failed: debt threshold exceeded'
                }
            }
        }
    }
}
```

## Custom Validation Rules

### Fail on Specific Categories

Configure validation to fail on specific debt types:

```toml
[validation]
fail_on_categories = ["Fixme", "CodeSmell", "Organization"]
min_severity = "high"
```

### Coverage-Based Validation

Require minimum test coverage:

```bash
debtmap validate . \
  --coverage-file coverage.lcov \
  --min-coverage 70.0 \
  --max-debt-density 50.0
```

### Complexity Gates

Enforce complexity limits:

```toml
[thresholds]
max_cyclomatic_complexity = 10
max_cognitive_complexity = 15
max_nesting_depth = 4
max_function_lines = 50
max_parameter_count = 5
```

## Progressive Quality Gates

### Ratcheting Strategy

Gradually improve thresholds over time:

```bash
# Month 1: Establish baseline
debtmap validate . --max-debt-density 100.0

# Month 2: Tighten slightly
debtmap validate . --max-debt-density 90.0

# Month 3: Continue improvement
debtmap validate . --max-debt-density 80.0

# Goal: Eventually reach strict threshold
debtmap validate . --max-debt-density 50.0
```

### File-Based Gates

Apply different thresholds to different parts of codebase:

```toml
[[validation.rules]]
paths = ["src/core/"]
max_debt_density = 25.0
min_coverage = 85.0

[[validation.rules]]
paths = ["src/experimental/"]
max_debt_density = 100.0
min_coverage = 50.0
```

## Best Practices

**Start lenient, tighten gradually:**
- Begin with lenient thresholds to avoid blocking development
- Track baseline debt density
- Reduce thresholds incrementally
- Communicate changes to team

**Use appropriate thresholds:**
- **New projects**: strict (25-50 debt density)
- **Active projects**: balanced (50-75 debt density)
- **Legacy projects**: lenient (75-150 debt density)

**Combine multiple metrics:**
- Debt density for overall health
- Coverage for testing
- Complexity for maintainability
- Category-specific rules for focus areas

**Provide actionable feedback:**
- Generate detailed reports on failure
- Include improvement suggestions
- Link to specific debt items
- Show trend over time

**Integrate early:**
- Add validation to PR checks
- Run on every commit to main branches
- Include in pre-merge requirements
- Track metrics in dashboards

## Use Cases

### Pre-Merge Quality Gate

```bash
#!/bin/bash
# pre-merge-check.sh

echo "Running quality gate validation..."

debtmap validate . \
  --max-debt-density 50.0 \
  --coverage-file coverage.lcov \
  --format json \
  --output validation-result.json

if [ $? -ne 0 ]; then
  echo "❌ Quality gate failed"
  echo "Review debt items and improve code before merging"
  cat validation-result.json
  exit 1
fi

echo "✓ Quality gate passed"
```

### Release Gate

```bash
# Only allow release if debt is below threshold
debtmap validate . --max-debt-density 40.0 || {
  echo "Cannot release: technical debt too high"
  exit 1
}

echo "Proceeding with release..."
```

### Continuous Monitoring

```bash
# Daily validation with trending
debtmap validate . --format json --output "validation-$(date +%Y%m%d).json"

# Alert if trend is negative
# (Compare with previous days)
```

## Troubleshooting

### Validation Unexpectedly Failing

**Issue:** Validation fails but debt seems acceptable

**Solution:**
- Review debt density calculation (debt per 1000 LOC)
- Check if coverage file is being used correctly
- Verify threshold configuration in .debtmap.toml
- Run analyze command to see detailed breakdown

### Different Results Locally vs CI

**Issue:** Validation passes locally but fails in CI

**Solution:**
- Ensure same debtmap version in CI and locally
- Check for uncommitted files locally
- Verify CI uses same configuration file
- Compare coverage files if used

### Too Many False Positives

**Issue:** Validation fails on acceptable code

**Solution:**
- Adjust thresholds in .debtmap.toml
- Use suppression patterns for known exceptions
- Configure exclusion paths for generated code
- Enable context providers to reduce false positives

## See Also

- [Compare Analysis](compare-analysis.md) - Track improvements over time
- [Configuration](configuration.md) - Validation configuration options
- [Threshold Configuration](threshold-configuration.md) - Understanding thresholds
- [CLI Reference](cli-reference.md) - Validate command options

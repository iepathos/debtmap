# Compare Analysis

The `compare` command enables tracking technical debt improvements over time by comparing analysis results before and after code changes. This is essential for validating refactoring efforts and preventing regressions.

## Overview

Compare analysis helps you:
- Track specific item improvements (target location tracking)
- Analyze overall project health trends
- Detect new critical debt items (regression detection)
- Calculate improvement percentages
- Validate refactoring success

## Basic Usage

### Compare Two Analysis Results

```bash
# Run analysis before changes
debtmap analyze . --format json --output before.json

# Make code changes...

# Run analysis after changes
debtmap analyze . --format json --output after.json

# Compare results
debtmap compare --before before.json --after after.json
```

### Output Formats

Generate comparison reports in different formats:

```bash
# Terminal output (default)
debtmap compare --before before.json --after after.json

# Markdown report
debtmap compare --before before.json --after after.json --format markdown

# JSON output for tooling
debtmap compare --before before.json --after after.json --format json

# Save to file
debtmap compare --before before.json --after after.json --format markdown --output comparison.md
```

## Target Location Tracking

Track specific item improvements by specifying a target location:

```bash
# Track improvement of specific function
debtmap compare \
  --before before.json \
  --after after.json \
  --target-location "src/analyzer.rs:analyze_complexity:142"
```

### Using Implementation Plans

Reference an implementation plan that contains the target location:

```bash
debtmap compare \
  --before before.json \
  --after after.json \
  --plan IMPLEMENTATION_PLAN.md
```

The plan file should contain a target location in format:
```
Target: src/analyzer.rs:analyze_complexity:142
```

## Comparison Features

### Metrics Comparison

Compare before/after metrics:
- Total debt score
- Debt density (per 1000 LOC)
- Item counts by category
- Coverage percentages
- Complexity distributions

### Improvement Percentages

Automatic calculation of:
- Overall debt reduction %
- Category-specific improvements
- Individual item improvements
- Coverage improvements

### Regression Detection

Identify new problems introduced:
- New critical items
- Increased complexity
- Coverage decreases
- New debt categories

### Project Health Trends

Track overall codebase health:
- Debt trajectory (improving/degrading)
- Test coverage trends
- Complexity distribution changes
- Risk profile evolution

## CI/CD Integration

### Quality Gate Example

```bash
#!/bin/bash
# ci-debt-check.sh

# Analyze current state
debtmap analyze . --format json --output after.json

# Compare with baseline
debtmap compare \
  --before baseline.json \
  --after after.json \
  --format json \
  --output comparison.json

# Check for regressions
REGRESSIONS=$(jq '.new_critical_items | length' comparison.json)

if [ "$REGRESSIONS" -gt 0 ]; then
  echo "ERROR: $REGRESSIONS new critical items detected"
  exit 1
fi

echo "✓ No technical debt regressions"
```

### GitHub Actions Example

```yaml
name: Debt Comparison

on: [pull_request]

jobs:
  compare-debt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
        with:
          fetch-depth: 0

      - name: Checkout base branch
        run: git checkout ${{ github.base_ref }}

      - name: Analyze before
        run: debtmap analyze . --format json --output before.json

      - name: Checkout PR branch
        run: git checkout ${{ github.head_ref }}

      - name: Analyze after
        run: debtmap analyze . --format json --output after.json

      - name: Compare results
        run: |
          debtmap compare \
            --before before.json \
            --after after.json \
            --format markdown \
            --output comparison.md

      - name: Comment on PR
        uses: actions/github-script@v5
        with:
          script: |
            const fs = require('fs');
            const comparison = fs.readFileSync('comparison.md', 'utf8');
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: comparison
            });
```

## Use Cases

### Validate Refactoring

```bash
# Before refactoring
debtmap analyze . --format json --output pre-refactor.json

# Perform refactoring...

# After refactoring
debtmap analyze . --format json --output post-refactor.json

# Verify improvement
debtmap compare \
  --before pre-refactor.json \
  --after post-refactor.json \
  --plan REFACTORING_PLAN.md
```

### Track Sprint Progress

```bash
# Beginning of sprint
debtmap analyze . --format json --output sprint-start.json

# End of sprint
debtmap analyze . --format json --output sprint-end.json

# Sprint retrospective
debtmap compare \
  --before sprint-start.json \
  --after sprint-end.json \
  --format markdown \
  --output sprint-debt-report.md
```

### Monitor Long-Term Trends

```bash
# Weekly analysis
debtmap analyze . --format json --output "debt-$(date +%Y%m%d).json"

# Compare against last week
debtmap compare \
  --before debt-$(date -d '7 days ago' +%Y%m%d).json \
  --after debt-$(date +%Y%m%d).json
```

## Best Practices

**Establish baselines:**
- Create baseline analysis at stable points (releases, sprint starts)
- Store baseline files in version control or artifact storage
- Document baseline creation criteria

**Use consistent analysis options:**
- Always use same flags for before/after analysis
- Include coverage files for both analyses
- Use same threshold configurations

**Automate comparisons:**
- Integrate into CI/CD pipelines
- Generate reports automatically
- Set up alerts for regressions

**Track specific improvements:**
- Use target location tracking for focused refactoring
- Reference implementation plans
- Validate that targeted items improved

**Review trends regularly:**
- Weekly/sprint comparisons for active development
- Release-to-release for longer-term trends
- Track improvement velocity

## Troubleshooting

### Comparison Shows Unexpected Changes

**Issue:** Differences in unmodified code

**Solution:**
- Ensure same analysis options used for both runs
- Check that coverage files match (if used)
- Verify same version of debtmap
- Compare file paths and line numbers for shifts

### Target Location Not Found

**Issue:** Target location not present in results

**Solution:**
- Verify location format: `file:function:line`
- Check that function wasn't renamed or moved
- Ensure both before/after analyses include the file
- Use `--target-location` with updated location if code moved

### Missing Regression Detection

**Issue:** Known regressions not detected

**Solution:**
- Lower threshold for what constitutes a regression
- Check that new items are truly new (not just renamed)
- Review severity classifications
- Ensure before.json includes all baselines

## See Also

- [Validation and Quality Gates](validation-gates.md) - Setting up quality gates
- [CLI Reference](cli-reference.md) - Compare command options
- [Output Formats](output-formats.md) - Understanding comparison output

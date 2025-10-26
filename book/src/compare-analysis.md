# Compare Analysis

The `compare` command enables you to track technical debt changes over time by comparing two analysis results. This is essential for validating refactoring efforts, detecting regressions in pull requests, and monitoring project health trends.

## Overview

The compare command analyzes differences between "before" and "after" debtmap analyses, providing:

- **Target location tracking** - Monitor specific functions or files you're refactoring
- **Project health metrics** - Track overall debt trends across your codebase
- **Regression detection** - Identify new critical debt items introduced
- **Improvement tracking** - Measure and celebrate debt reduction
- **CI/CD integration** - Automate quality gates in your pipeline

## Basic Usage

### Command Syntax

```bash
debtmap compare \
  --before path/to/before.json \
  --after path/to/after.json \
  --plan IMPLEMENTATION_PLAN.md \
  --format json \
  --output comparison.json
```

### Command-Line Options

| Option | Required | Description |
|--------|----------|-------------|
| `--before FILE` | Yes | Path to "before" analysis JSON |
| `--after FILE` | Yes | Path to "after" analysis JSON |
| `--plan FILE` | No | Implementation plan to extract target location |
| `--target-location LOCATION` | No | Manual target location (alternative to `--plan`) |
| `--format FORMAT` | No | Output format: `json` or `markdown` (default: `json`) |
| `--output FILE` | No | Output file path (default: stdout) |

**Note**: `--plan` and `--target-location` are mutually exclusive. Use one or neither, but not both.

## Target Location Tracking

Target location tracking allows you to monitor specific code locations through refactoring changes.

### Location Format

Target locations use the format: `file:function:line`

Examples:
- `src/main.rs:complex_function:42`
- `lib/parser.rs:parse_expression:156`
- `api/handler.rs:process_request:89`

### Specifying Target Locations

#### Option 1: Via Implementation Plan

Create an `IMPLEMENTATION_PLAN.md` file with a target location section:

```markdown
# Implementation Plan

## Target Item
**Location**: ./src/example.rs:complex_function:45
**Current Debt Score**: 85.5
**Severity**: critical

## Problem Analysis
The `complex_function` has high cognitive complexity...

## Proposed Solution
1. Extract nested conditionals into separate functions
2. Use early returns to reduce nesting depth
3. Add comprehensive unit tests
```

Then run compare with the plan:

```bash
debtmap compare --before before.json --after after.json --plan IMPLEMENTATION_PLAN.md
```

#### Option 2: Manual Target Location

Specify the target directly via command-line:

```bash
debtmap compare \
  --before before.json \
  --after after.json \
  --target-location "src/example.rs:complex_function:45"
```

### Matching Strategies

Debtmap uses intelligent matching to find your target item even when code changes:

| Strategy | When Used | Confidence |
|----------|-----------|------------|
| **Exact** | Location matches exactly | 1.0 |
| **FuzzyFunction** | Function moved but name unchanged | 0.8 - 0.95 |
| **FuzzyFile** | File changed but function exists | 0.6 - 0.8 |

The comparison result includes the match strategy and confidence score used.

### Target Status Values

After comparing, the target item will have one of these statuses:

- **Resolved** - Item no longer exists in after analysis (debt eliminated!)
- **Improved** - Item exists but with lower debt score
- **Unchanged** - Item exists with similar metrics (within 5%)
- **Regressed** - Item exists but got worse
- **NotFoundBefore** - Item didn't exist in before analysis
- **NotFound** - Item not found in either analysis

## Project Health Metrics

The compare command tracks project-wide health metrics to show overall trends.

### Tracked Metrics

```json
{
  "project_health": {
    "before": {
      "total_debt_score": 450.5,
      "total_items": 25,
      "critical_items": 5,
      "high_priority_items": 12,
      "average_score": 18.02
    },
    "after": {
      "total_debt_score": 380.2,
      "total_items": 22,
      "critical_items": 3,
      "high_priority_items": 10,
      "average_score": 17.28
    },
    "changes": {
      "debt_score_change": -70.3,
      "debt_score_change_pct": -15.6,
      "items_change": -3,
      "critical_items_change": -2
    }
  }
}
```

### Understanding Metrics

- **total_debt_score** - Sum of all debt item scores
- **total_items** - Total number of debt items detected
- **critical_items** - Items with score ≥ 60
- **high_priority_items** - Items with score ≥ 40
- **average_score** - Mean debt score across all items
- **debt_score_change** - Absolute change in total debt
- **debt_score_change_pct** - Percentage change in total debt

### Debt Trends

The comparison calculates an overall debt trend based on the percentage change:

- **Improving** - Debt decreased by more than 5%
- **Stable** - Debt changed by less than 5% (within normal variance)
- **Regressing** - Debt increased by more than 5%

## Regression Detection

Regressions are new critical debt items (score ≥ 60) that appear in the after analysis.

### What Counts as a Regression

A regression is detected when:
1. An item exists in the after analysis
2. The item does NOT exist in the before analysis
3. The item has a debt score ≥ 60.0 (critical severity)

### Regression Output

```json
{
  "regressions": [
    {
      "location": "src/new_feature.rs:process_data:23",
      "score": 72.5,
      "debt_type": "high_complexity",
      "description": "Function has cyclomatic complexity of 12 and cognitive complexity of 15"
    }
  ]
}
```

### Using Regressions in CI/CD

Fail your CI build if regressions are detected:

```bash
# Run comparison
debtmap compare --before before.json --after after.json --output result.json

# Check for regressions
REGRESSION_COUNT=$(jq '.summary.new_critical_count' result.json)

if [ "$REGRESSION_COUNT" -gt 0 ]; then
  echo "❌ Found $REGRESSION_COUNT new critical debt items"
  exit 1
fi
```

## Improvement Tracking

Debtmap tracks two types of improvements:

### Improvement Types

1. **Resolved** - Debt item completely eliminated (no longer in after analysis)
2. **ScoreReduced** - Debt item still exists but score reduced by ≥ 30%

### Improvement Metrics

When comparing target items, debtmap calculates:

```json
{
  "improvements": {
    "score_reduction_pct": 45.2,
    "complexity_reduction_pct": 38.7,
    "coverage_improvement_pct": 25.0
  }
}
```

- **score_reduction_pct** - Percentage reduction in overall debt score
- **complexity_reduction_pct** - Reduction in cyclomatic/cognitive complexity
- **coverage_improvement_pct** - Increase in test coverage

### Improvement Items List

```json
{
  "improvements": [
    {
      "location": "src/example.rs:complex_function:45",
      "before_score": 85.5,
      "after_score": 32.1,
      "improvement_type": "ScoreReduced"
    },
    {
      "location": "src/legacy.rs:old_code:120",
      "before_score": 65.0,
      "after_score": null,
      "improvement_type": "Resolved"
    }
  ]
}
```

## Before/After Metrics

For target items, debtmap compares detailed metrics:

### Target Metrics Structure

```json
{
  "target_item": {
    "location": "src/example.rs:complex_function:45",
    "match_strategy": "Exact",
    "match_confidence": 1.0,
    "matched_items_count": 1,
    "before": {
      "score": 85.5,
      "cyclomatic_complexity": 8,
      "cognitive_complexity": 15,
      "coverage": 45.0,
      "function_length": 120,
      "nesting_depth": 4
    },
    "after": {
      "score": 32.1,
      "cyclomatic_complexity": 3,
      "cognitive_complexity": 5,
      "coverage": 85.0,
      "function_length": 45,
      "nesting_depth": 2
    },
    "improvements": {
      "score_reduction_pct": 62.5,
      "complexity_reduction_pct": 66.7,
      "coverage_improvement_pct": 88.9
    },
    "status": "Improved"
  }
}
```

### Metric Aggregation

When multiple items match the target location (due to fuzzy matching), metrics are aggregated:

- **score** - Average across matched items
- **cyclomatic_complexity** - Average
- **cognitive_complexity** - Average
- **coverage** - Average
- **function_length** - Average
- **nesting_depth** - Maximum (worst case)

The `matched_items_count` field tells you how many items were aggregated.

### Validating Refactoring Success

Use these metrics to verify your refactoring achieved its goals:

```bash
# Example: Verify complexity reduction
COMPLEXITY_REDUCTION=$(jq '.target_item.improvements.complexity_reduction_pct' result.json)

if (( $(echo "$COMPLEXITY_REDUCTION < 30" | bc -l) )); then
  echo "⚠️ Complexity reduction ($COMPLEXITY_REDUCTION%) below 30% target"
fi
```

## Output Formats

### JSON Format

The default JSON format provides complete structured data for programmatic use:

```bash
debtmap compare --before before.json --after after.json --format json
```

The JSON output includes:
- `metadata` - Comparison metadata (date, files, target location)
- `target_item` - Target comparison details (if specified)
- `project_health` - Project-wide metrics and trends
- `regressions` - List of new critical items
- `improvements` - List of resolved/reduced items
- `summary` - High-level summary statistics

### Markdown Format

For human-readable reports:

```bash
debtmap compare --before before.json --after after.json --format markdown
```

The markdown output provides a formatted report suitable for:
- Pull request comments
- Documentation
- Email reports
- Team dashboards

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Technical Debt Check

on: [pull_request]

jobs:
  debt-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0  # Need history for before/after

      - name: Install debtmap
        run: cargo install debtmap

      - name: Analyze main branch
        run: |
          git checkout main
          debtmap analyze --output before.json

      - name: Analyze PR branch
        run: |
          git checkout ${{ github.head_ref }}
          debtmap analyze --output after.json

      - name: Compare analyses
        run: |
          debtmap compare \
            --before before.json \
            --after after.json \
            --format markdown \
            --output comparison.md

      - name: Check for regressions
        run: |
          REGRESSIONS=$(debtmap compare \
            --before before.json \
            --after after.json \
            --format json | jq '.summary.new_critical_count')

          if [ "$REGRESSIONS" -gt 0 ]; then
            echo "❌ Found $REGRESSIONS new critical debt items"
            exit 1
          fi

      - name: Post comparison to PR
        uses: actions/github-script@v6
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

### GitLab CI Example

```yaml
debt_check:
  stage: test
  script:
    # Analyze main branch
    - git fetch origin main
    - git checkout origin/main
    - debtmap analyze --output before.json

    # Analyze current branch
    - git checkout $CI_COMMIT_SHA
    - debtmap analyze --output after.json

    # Compare and fail on regressions
    - debtmap compare --before before.json --after after.json --output result.json
    - |
      REGRESSIONS=$(jq '.summary.new_critical_count' result.json)
      if [ "$REGRESSIONS" -gt 0 ]; then
        echo "Failed: $REGRESSIONS new critical items"
        exit 1
      fi
  artifacts:
    paths:
      - before.json
      - after.json
      - result.json
    expire_in: 1 week
```

### Best Practices for CI/CD

1. **Store analyses as artifacts** - Keep before/after JSON for debugging
2. **Set appropriate thresholds** - Don't fail on minor increases
3. **Track trends over time** - Store comparison results for historical analysis
4. **Use target locations for focused work** - Track specific refactoring efforts
5. **Generate reports** - Use markdown format for team visibility
6. **Gate deployments** - Require debt improvements for major releases

## Practical Examples

### Example 1: Basic Comparison

Compare two analyses without targeting specific items:

```bash
# Run before analysis
debtmap analyze --output before.json

# Make changes to codebase...

# Run after analysis
debtmap analyze --output after.json

# Compare
debtmap compare --before before.json --after after.json --format markdown
```

### Example 2: Tracking Function Refactoring

Monitor a specific function you're refactoring:

```bash
# Create implementation plan
cat > IMPLEMENTATION_PLAN.md <<EOF
# Refactoring complex_function

## Target Item
**Location**: ./src/parser.rs:parse_expression:156
**Current Debt Score**: 78.5
**Severity**: critical
EOF

# Run before analysis
debtmap analyze --output before.json

# Refactor the function...

# Run after analysis
debtmap analyze --output after.json

# Compare with target tracking
debtmap compare \
  --before before.json \
  --after after.json \
  --plan IMPLEMENTATION_PLAN.md \
  --format json \
  --output result.json

# Check improvement
jq '.target_item.status' result.json  # Should show "Improved" or "Resolved"
jq '.target_item.improvements.score_reduction_pct' result.json
```

### Example 3: Detecting PR Regressions

Check if a pull request introduces new complexity:

```bash
# Analyze base branch
git checkout main
debtmap analyze --output main.json

# Analyze PR branch
git checkout feature/new-feature
debtmap analyze --output feature.json

# Compare
debtmap compare \
  --before main.json \
  --after feature.json \
  --output comparison.json

# Extract summary
jq '.summary' comparison.json

# Example output:
# {
#   "target_improved": false,
#   "new_critical_count": 2,
#   "resolved_count": 0,
#   "overall_debt_trend": "Regressing"
# }
```

### Example 4: Monitoring Project Health Over Releases

Track overall project health across releases:

```bash
# Analyze release v1.0
git checkout v1.0
debtmap analyze --output v1.0.json

# Analyze release v1.1
git checkout v1.1
debtmap analyze --output v1.1.json

# Compare
debtmap compare \
  --before v1.0.json \
  --after v1.1.json \
  --format json \
  --output v1.0-to-v1.1.json

# Check trend
jq '.project_health.changes.debt_score_change_pct' v1.0-to-v1.1.json
# Example: -12.5 (12.5% debt reduction - good!)
```

### Example 5: Full CI/CD Workflow

Complete workflow for continuous debt monitoring:

```bash
#!/bin/bash
# ci-debt-check.sh

set -e

BEFORE="before.json"
AFTER="after.json"
RESULT="comparison.json"
PLAN="IMPLEMENTATION_PLAN.md"

# Step 1: Analyze baseline (main branch)
echo "📊 Analyzing baseline..."
git checkout main
debtmap analyze --output "$BEFORE"

# Step 2: Analyze current branch
echo "📊 Analyzing current branch..."
git checkout -
debtmap analyze --output "$AFTER"

# Step 3: Run comparison
echo "🔍 Comparing analyses..."
if [ -f "$PLAN" ]; then
  debtmap compare \
    --before "$BEFORE" \
    --after "$AFTER" \
    --plan "$PLAN" \
    --format json \
    --output "$RESULT"
else
  debtmap compare \
    --before "$BEFORE" \
    --after "$AFTER" \
    --format json \
    --output "$RESULT"
fi

# Step 4: Extract metrics
NEW_CRITICAL=$(jq '.summary.new_critical_count' "$RESULT")
RESOLVED=$(jq '.summary.resolved_count' "$RESULT")
TREND=$(jq -r '.summary.overall_debt_trend' "$RESULT")
TARGET_STATUS=$(jq -r '.target_item.status // "N/A"' "$RESULT")

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📈 Technical Debt Comparison Results"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "New Critical Items: $NEW_CRITICAL"
echo "Resolved Items: $RESOLVED"
echo "Overall Trend: $TREND"
echo "Target Status: $TARGET_STATUS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Step 5: Quality gate
if [ "$NEW_CRITICAL" -gt 0 ]; then
  echo "❌ FAILED: New critical debt items introduced"
  exit 1
fi

if [ "$TREND" = "Regressing" ]; then
  echo "⚠️  WARNING: Overall debt is regressing"
  # Don't fail, just warn
fi

if [ "$TARGET_STATUS" = "Regressed" ]; then
  echo "❌ FAILED: Target item regressed"
  exit 1
fi

echo "✅ PASSED: No regressions detected"
```

### Example 6: Interpreting Different Status Outcomes

Understanding what each status means:

```bash
# Run comparison
debtmap compare --before before.json --after after.json --output result.json

# Check target status
STATUS=$(jq -r '.target_item.status' result.json)

case "$STATUS" in
  "Resolved")
    echo "🎉 Success! Target item completely eliminated."
    echo "The function/file no longer appears in debt analysis."
    ;;
  "Improved")
    REDUCTION=$(jq '.target_item.improvements.score_reduction_pct' result.json)
    echo "✅ Improvement! Debt score reduced by $REDUCTION%"
    ;;
  "Unchanged")
    echo "➡️  No significant change (within 5% threshold)"
    echo "Consider further refactoring if debt is still high."
    ;;
  "Regressed")
    BEFORE=$(jq '.target_item.before.score' result.json)
    AFTER=$(jq '.target_item.after.score' result.json)
    echo "❌ Regression! Score increased from $BEFORE to $AFTER"
    echo "Review recent changes to this location."
    ;;
  "NotFoundBefore")
    echo "ℹ️  Target didn't exist in before analysis"
    echo "This is a new code location."
    ;;
esac
```

## Troubleshooting

### Target Location Not Found

**Problem**: `Error: Target item not found in before analysis`

**Solutions**:
1. Verify the location format: `file:function:line`
2. Check that the file path is relative to project root
3. Ensure the function exists in the before analysis
4. Try a fuzzy match by using just `file:function` (omit line number)

**Example**:
```bash
# Instead of exact line number
--target-location "src/parser.rs:parse_expression:156"

# Try without line number for fuzzy matching
--target-location "src/parser.rs:parse_expression"
```

### Location Format Errors

**Problem**: Invalid location format

**Solution**: Ensure format is `file:function:line` with:
- File path relative to project root (can start with `./`)
- Function name exactly as it appears in code
- Line number (optional for fuzzy matching)

**Valid examples**:
```bash
src/main.rs:complex_function:42
./lib/parser.rs:parse_expr:89
api/handlers.rs:process_request
```

### JSON Parsing Errors

**Problem**: `Error parsing JSON file`

**Solutions**:
1. Verify the file is valid JSON: `jq . before.json`
2. Ensure the file is a debtmap analysis output
3. Check file permissions and path
4. Regenerate the analysis if corrupted

### Understanding Target Status Values

| Status | Meaning | Action Required |
|--------|---------|-----------------|
| `Resolved` | Item eliminated | ✅ Celebrate! Document what worked |
| `Improved` | Score reduced | ✅ Good progress, consider pushing further |
| `Unchanged` | No significant change | ⚠️ Review approach, may need different refactoring |
| `Regressed` | Score increased | ❌ Investigate and fix before merging |
| `NotFoundBefore` | New item | ℹ️ Normal for new code, ensure it's not high debt |
| `NotFound` | Missing in both | ❌ Check location format and file path |

### Handling Missing Files

**Problem**: `No such file or directory`

**Solutions**:
```bash
# Verify files exist
ls -la before.json after.json

# Check current directory
pwd

# Use absolute paths if needed
debtmap compare \
  --before /absolute/path/to/before.json \
  --after /absolute/path/to/after.json
```

### Interpreting Edge Cases

**Empty After Analysis**:
```json
{
  "summary": {
    "resolved_count": 25,
    "overall_debt_trend": "Improving"
  }
}
```
All debt items resolved - excellent work!

**Empty Before Analysis**:
```json
{
  "summary": {
    "new_critical_count": 15,
    "overall_debt_trend": "Regressing"
  }
}
```
New project or first analysis - establish baseline for future comparisons.

**Identical Analyses**:
```json
{
  "summary": {
    "overall_debt_trend": "Stable",
    "new_critical_count": 0,
    "resolved_count": 0
  }
}
```
No changes detected - either no code changes or changes were neutral to debt.

## Related Documentation

- [Validation Command](validation.md) - Validate implementation plans match analysis
- [Prodigy Integration](prodigy-integration.md) - Automated refactoring workflows
- [Output Formats](output-formats.md) - Understanding analysis JSON structure
- [Scoring Strategies](scoring-strategies.md) - How debt scores are calculated
- [CI/CD Integration](ci-cd.md) - Advanced pipeline configurations

## Summary

The compare command is a powerful tool for:

- ✅ Validating refactoring efforts with concrete metrics
- ✅ Detecting regressions before they reach production
- ✅ Tracking project health trends over time
- ✅ Automating quality gates in CI/CD pipelines
- ✅ Celebrating and documenting technical debt victories

Use it regularly to maintain visibility into your codebase's technical health and ensure continuous improvement.

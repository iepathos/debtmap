# Compare Analysis

The `compare` command enables you to track technical debt changes over time by comparing two analysis results. This is essential for validating refactoring efforts, detecting regressions in pull requests, and monitoring project health trends.

## Implementation Status

**All Features Available Now**:
- ✅ Target location tracking with intelligent fuzzy matching
- ✅ Detailed improvement percentage calculations (per-item)
- ✅ Multiple output formats (JSON, Markdown, Terminal)
- ✅ Implementation plan parsing for target extraction
- ✅ Four match strategies (Exact, FunctionLevel, ApproximateName, FileLevel)
- ✅ Resolved items tracking (debt eliminated)
- ✅ Improved items detection (score reduction ≥ 30%)
- ✅ New critical items detection (regressions)
- ✅ Project health metrics and trends
- ✅ CI/CD integration support

## Overview

The compare command analyzes differences between "before" and "after" debtmap analyses, providing:

- **Target location tracking** - Monitor specific code locations through refactoring with fuzzy matching
- **Validation tracking** - Verify debt items are resolved or improved
- **Project health metrics** - Track overall debt trends across your codebase
- **Regression detection** - Identify new critical debt items introduced (score ≥ 60.0)
- **Improvement tracking** - Measure and celebrate debt reduction with detailed per-item metrics
- **CI/CD integration** - Automate quality gates in your pipeline

## Basic Usage

### Command Syntax

```bash
debtmap compare \
  --before path/to/before.json \
  --after path/to/after.json \
  --output validation.json
```

### Command-Line Options

| Option | Required | Description |
|--------|----------|-------------|
| `--before FILE` | Yes | Path to "before" analysis JSON |
| `--after FILE` | Yes | Path to "after" analysis JSON |
| `--output FILE` | No | Output file path (default: stdout) |
| `--plan FILE` | No | Implementation plan to extract target location |
| `--target-location LOCATION` | No | Manual target location (format: `file:function:line`) |
| `--format FORMAT` | No | Output format: `json`, `markdown`, or `terminal` (default: json) |

All comparison features are available now, including target location tracking, fuzzy matching, and multiple output formats.

## Target Location Tracking

Target location tracking allows you to monitor specific code locations through refactoring changes. The compare command uses intelligent fuzzy matching to find your target even when code is moved or renamed.

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

Debtmap uses intelligent matching to find your target item even when code changes. The matcher tries multiple strategies in order, using the most precise match available:

| Strategy | When Used | Confidence |
|----------|-----------|------------|
| **Exact** | `file:function:line` matches exactly | 1.0 |
| **FunctionLevel** | `file:function` matches (any line) | 0.8 |
| **ApproximateName** | Fuzzy name matching finds similar function | 0.6 |
| **FileLevel** | All items in file match | 0.4 |

The comparison result includes the match strategy and confidence score used, along with the count of matched items (useful when fuzzy matching finds multiple candidates).

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
- **critical_items** - Items with score ≥ 60.0 (critical threshold)
- **high_priority_items** - Items with score ≥ 40.0 (high priority threshold)
- **average_score** - Mean debt score across all items
- **debt_score_change** - Absolute change in total debt
- **debt_score_change_pct** - Percentage change in total debt

### Debt Trends

The comparison calculates an overall debt trend based on the percentage change:

- **Improving** - Debt decreased by more than 5%
- **Stable** - Debt changed by less than 5% (within normal variance)
- **Regressing** - Debt increased by more than 5%

## Regression Detection

Regressions are new critical debt items (score ≥ 60.0) that appear in the after analysis.

### What Counts as a Regression

A regression is detected when:
1. An item exists in the after analysis
2. The item does NOT exist in the before analysis
3. The item has a debt score ≥ 60.0 (critical severity threshold)

### Regression Output

The compare command returns a `ComparisonResult` with detailed regression information:

```json
{
  "regressions": [
    {
      "location": "src/new_feature.rs:process_data:23",
      "score": 65.5,
      "debt_type": "high_complexity",
      "description": "Function has cyclomatic complexity of 12 and cognitive complexity of 15"
    }
  ]
}
```

Each regression item includes:
- **location** - Full path with function and line number
- **score** - Debt score (≥ 60.0 for regressions)
- **debt_type** - Type of debt detected (e.g., "high_complexity", "god_object")
- **description** - Human-readable explanation of the issue

### Using Regressions in CI/CD

Fail your CI build if regressions are detected:

```bash
# Run comparison
debtmap compare --before before.json --after after.json --output result.json

# Check for regressions
REGRESSION_COUNT=$(jq '.regressions | length' result.json)

if [ "$REGRESSION_COUNT" -gt 0 ]; then
  echo "❌ Regression detected - $REGRESSION_COUNT new critical debt items found"
  jq '.regressions[]' result.json
  exit 1
fi

# Check overall debt trend
TREND=$(jq -r '.summary.overall_debt_trend' result.json)
if [ "$TREND" = "Regressing" ]; then
  echo "⚠️ Warning: Overall debt is increasing"
fi
```

## Improvement Tracking

The compare command tracks improvements as a list of `ImprovementItem` objects with detailed before/after metrics.

### Improvement Types

The `improvement_type` field indicates the kind of improvement:

- **Resolved** - Debt item completely eliminated (no longer in after analysis)
- **ScoreReduced** - Overall debt score reduced significantly (≥ 30% reduction)
- **ComplexityReduced** - Cyclomatic or cognitive complexity decreased
- **CoverageImproved** - Test coverage increased

### Improvement Items Structure

```json
{
  "improvements": [
    {
      "location": "src/example.rs:complex_function:45",
      "before_score": 68.5,
      "after_score": 35.2,
      "improvement_type": "ScoreReduced"
    },
    {
      "location": "src/legacy.rs:old_code:120",
      "before_score": 72.0,
      "after_score": null,
      "improvement_type": "Resolved"
    },
    {
      "location": "src/utils.rs:helper_function:88",
      "before_score": 45.0,
      "after_score": 28.0,
      "improvement_type": "ComplexityReduced"
    }
  ]
}
```

Each improvement item includes:
- **location** - Full path with function and line number
- **before_score** - Original debt score
- **after_score** - New debt score (null if resolved)
- **improvement_type** - Type of improvement achieved

## Before/After Metrics

When you specify a target location (via `--plan` or `--target-location`), the compare command provides detailed before/after metrics for that specific code location.

### Target Item Comparison

```json
{
  "target_item": {
    "location": "src/example.rs:complex_function:45",
    "match_strategy": "Exact",
    "match_confidence": 1.0,
    "matched_items_count": 1,
    "before": {
      "score": 68.5,
      "cyclomatic_complexity": 8,
      "cognitive_complexity": 15,
      "coverage": 45.0,
      "function_length": 120,
      "nesting_depth": 4
    },
    "after": {
      "score": 35.1,
      "cyclomatic_complexity": 3,
      "cognitive_complexity": 5,
      "coverage": 85.0,
      "function_length": 45,
      "nesting_depth": 2
    },
    "improvements": {
      "score_reduction_pct": 48.8,
      "complexity_reduction_pct": 66.7,
      "coverage_improvement_pct": 88.9
    },
    "status": "Improved"
  }
}
```

### Target Metrics Fields

Each `TargetMetrics` object (before/after) includes:
- **score** - Unified debt score
- **cyclomatic_complexity** - Cyclomatic complexity metric
- **cognitive_complexity** - Cognitive complexity metric
- **coverage** - Test coverage percentage
- **function_length** - Lines of code in function
- **nesting_depth** - Maximum nesting depth

### Improvement Percentages

The `improvements` object provides percentage improvements:
- **score_reduction_pct** - Percentage reduction in overall debt score
- **complexity_reduction_pct** - Reduction in cyclomatic/cognitive complexity
- **coverage_improvement_pct** - Increase in test coverage

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

Use the comparison output to verify your refactoring:

```bash
# Check target status
STATUS=$(jq -r '.target_item.status' result.json)
SCORE_REDUCTION=$(jq '.target_item.improvements.score_reduction_pct' result.json)

echo "Target Status: $STATUS"
echo "Score Reduction: ${SCORE_REDUCTION}%"

# Check for improvements
IMPROVEMENT_COUNT=$(jq '.improvements | length' result.json)
echo "Improvements: $IMPROVEMENT_COUNT items"

# Verify no regressions
REGRESSION_COUNT=$(jq '.regressions | length' result.json)
if [ "$REGRESSION_COUNT" -eq 0 ]; then
  echo "✅ No regressions detected!"
else
  echo "⚠️ $REGRESSION_COUNT new critical items"
fi
```

## Output Formats

### JSON Format

The default JSON format provides complete comparison results:

```bash
debtmap compare --before before.json --after after.json --output result.json
```

The `ComparisonResult` JSON output includes:
- `metadata` - Comparison metadata (date, file paths, target location)
- `target_item` - Target item comparison with before/after metrics (if specified)
- `project_health` - Project-wide health metrics comparison
- `regressions` - List of new critical items
- `improvements` - List of improved/resolved items
- `summary` - Summary statistics and overall debt trend

Example output:
```json
{
  "metadata": {
    "comparison_date": "2024-01-15T10:30:00Z",
    "before_file": "before.json",
    "after_file": "after.json",
    "target_location": "src/example.rs:complex_function:45"
  },
  "target_item": {
    "status": "Improved",
    "improvements": {
      "score_reduction_pct": 48.8
    }
  },
  "summary": {
    "target_improved": true,
    "new_critical_count": 0,
    "resolved_count": 3,
    "overall_debt_trend": "Improving"
  }
}
```

### Markdown Format

Generate human-readable markdown reports for pull request comments:

```bash
debtmap compare --before before.json --after after.json --format markdown
```

The markdown output is suitable for:
- Pull request comments
- Documentation
- Email reports
- Team dashboards

### Terminal Format

Display colorized output directly in the terminal:

```bash
debtmap compare --before before.json --after after.json --format terminal
```

The terminal format provides:
- Color-coded status indicators
- Formatted tables for metrics
- Human-readable summaries
- Easy scanning of results

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
            --output comparison.json

      - name: Check comparison result
        run: |
          TREND=$(jq -r '.summary.overall_debt_trend' comparison.json)
          REGRESSION_COUNT=$(jq '.regressions | length' comparison.json)
          IMPROVEMENT_COUNT=$(jq '.improvements | length' comparison.json)

          echo "Debt Trend: $TREND"
          echo "Regressions: $REGRESSION_COUNT"
          echo "Improvements: $IMPROVEMENT_COUNT"

          # Fail on regression
          if [ "$REGRESSION_COUNT" -gt 0 ]; then
            echo "❌ Regression detected"
            jq '.regressions[]' comparison.json
            exit 1
          fi

          # Warn if debt is increasing
          if [ "$TREND" = "Regressing" ]; then
            echo "⚠️ Warning: Overall debt is increasing"
          fi

      - name: Post comparison to PR
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const comparison = JSON.parse(fs.readFileSync('comparison.json', 'utf8'));

            const body = `## Technical Debt Comparison

            **Overall Trend:** ${comparison.summary.overall_debt_trend}
            **Regressions:** ${comparison.summary.new_critical_count}
            **Improvements:** ${comparison.summary.resolved_count}

            ${comparison.improvements.length > 0 ? `
            ### Improvements
            ${comparison.improvements.map(i => `- ${i.location}: ${i.before_score.toFixed(1)} → ${i.after_score ? i.after_score.toFixed(1) : 'resolved'}`).join('\n')}
            ` : ''}

            ${comparison.regressions.length > 0 ? `
            ### ⚠️ Regressions
            ${comparison.regressions.map(r => `- ${r.location}: ${r.score.toFixed(1)} (${r.debt_type})`).join('\n')}
            ` : ''}`;

            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: body
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

    # Compare and check status
    - debtmap compare --before before.json --after after.json --output comparison.json
    - |
      TREND=$(jq -r '.summary.overall_debt_trend' comparison.json)
      REGRESSION_COUNT=$(jq '.regressions | length' comparison.json)

      echo "Debt Trend: $TREND"
      echo "Regressions: $REGRESSION_COUNT"

      if [ "$REGRESSION_COUNT" -gt 0 ]; then
        echo "Failed: Regression detected"
        jq '.regressions[]' comparison.json
        exit 1
      fi
  artifacts:
    paths:
      - before.json
      - after.json
      - comparison.json
    expire_in: 1 week
```

### Best Practices for CI/CD

1. **Store analyses as artifacts** - Keep before/after JSON for debugging
2. **Check status field** - Use `status` to determine pass/fail
3. **Track completion percentage** - Monitor progress toward debt resolution
4. **Review improvements** - Celebrate and document successful refactorings
5. **Act on remaining issues** - Create follow-up tasks for unresolved items
6. **Set completion thresholds** - Require minimum completion percentage for merges

## Practical Examples

### Example 1: Basic Comparison

Compare two analyses to track debt changes:

```bash
# Run before analysis
debtmap analyze --output before.json

# Make changes to codebase...

# Run after analysis
debtmap analyze --output after.json

# Compare
debtmap compare --before before.json --after after.json --output comparison.json

# Check results
cat comparison.json | jq '.'
# Output shows: target_item, project_health, regressions, improvements, summary
```

### Example 2: Validating Function Refactoring

Validate your refactoring work with target location tracking:

```bash
# Run before analysis
debtmap analyze --output before.json

# Identify critical items to fix
jq '.items[] | select(.unified_score.final_score >= 60.0)' before.json

# Refactor the high-priority functions...

# Run after analysis
debtmap analyze --output after.json

# Compare and validate with target location
debtmap compare \
  --before before.json \
  --after after.json \
  --target-location "src/example.rs:complex_function:45" \
  --output comparison.json

# Check target status
STATUS=$(jq -r '.target_item.status' comparison.json)
SCORE_REDUCTION=$(jq '.target_item.improvements.score_reduction_pct' comparison.json)

echo "Target Status: $STATUS"
echo "Score Reduction: ${SCORE_REDUCTION}%"

# Review all improvements
jq '.improvements[]' comparison.json
```

### Example 3: Detecting PR Regressions

Check if a pull request introduces new critical debt:

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

# Check for regressions
REGRESSION_COUNT=$(jq '.regressions | length' comparison.json)
TREND=$(jq -r '.summary.overall_debt_trend' comparison.json)

echo "Regressions: $REGRESSION_COUNT"
echo "Debt Trend: $TREND"

# Example output structure:
jq '.' comparison.json
# {
#   "summary": {
#     "overall_debt_trend": "Improving",  // or "Regressing"
#     "new_critical_count": 0,
#     "resolved_count": 3
#   },
#   "regressions": [],
#   "improvements": [...],
#   "project_health": {...}
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
  --output v1.0-to-v1.1.json

# Check project health metrics
echo "Before (v1.0):"
jq '.project_health.before' v1.0-to-v1.1.json

echo "After (v1.1):"
jq '.project_health.after' v1.0-to-v1.1.json

# Check overall trend
TREND=$(jq -r '.summary.overall_debt_trend' v1.0-to-v1.1.json)
DEBT_CHANGE=$(jq '.project_health.changes.debt_score_change_pct' v1.0-to-v1.1.json)
echo "Debt Trend: $TREND"
echo "Debt Score Change: ${DEBT_CHANGE}%"
```

### Example 5: Full CI/CD Workflow

Complete workflow for continuous debt monitoring:

```bash
#!/bin/bash
# ci-debt-check.sh

set -e

BEFORE="before.json"
AFTER="after.json"
COMPARISON="comparison.json"

# Step 1: Analyze baseline (main branch)
echo "📊 Analyzing baseline..."
git checkout main
debtmap analyze --output "$BEFORE"

# Step 2: Analyze current branch
echo "📊 Analyzing current branch..."
git checkout -
debtmap analyze --output "$AFTER"

# Step 3: Run comparison
echo "🔍 Running comparison..."
debtmap compare \
  --before "$BEFORE" \
  --after "$AFTER" \
  --output "$COMPARISON"

# Step 4: Extract metrics
TREND=$(jq -r '.summary.overall_debt_trend' "$COMPARISON")
REGRESSION_COUNT=$(jq '.regressions | length' "$COMPARISON")
IMPROVEMENT_COUNT=$(jq '.improvements | length' "$COMPARISON")
RESOLVED_COUNT=$(jq '.summary.resolved_count' "$COMPARISON")

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📈 Debt Comparison Results"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Trend: $TREND"
echo "Regressions: $REGRESSION_COUNT"
echo "Improvements: $IMPROVEMENT_COUNT"
echo "Resolved: $RESOLVED_COUNT"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Step 5: Quality gate
if [ "$REGRESSION_COUNT" -gt 0 ]; then
  echo "❌ FAILED: Regression detected"
  jq '.regressions[]' "$COMPARISON"
  exit 1
fi

if [ "$TREND" = "Regressing" ]; then
  echo "⚠️  WARNING: Overall debt is increasing"
  # Don't fail, just warn
fi

if [ "$RESOLVED_COUNT" -gt 0 ]; then
  echo "🎉 SUCCESS: $RESOLVED_COUNT debt items resolved!"
fi

echo "✅ PASSED: No regressions detected"
```

### Example 6: Interpreting Comparison Results

Understanding the comparison output:

```bash
# Run comparison
debtmap compare --before before.json --after after.json --output comparison.json

# Check debt trend
TREND=$(jq -r '.summary.overall_debt_trend' comparison.json)
REGRESSION_COUNT=$(jq '.regressions | length' comparison.json)
IMPROVEMENT_COUNT=$(jq '.improvements | length' comparison.json)

case "$TREND" in
  "Improving")
    echo "🎉 Success! Debt is decreasing"
    echo "Improvements: $IMPROVEMENT_COUNT"
    jq '.improvements[] | "\(.location): \(.improvement_type)"' comparison.json
    ;;
  "Stable")
    echo "➡️  Stable - no significant debt change"
    echo "Improvements: $IMPROVEMENT_COUNT"
    echo "Regressions: $REGRESSION_COUNT"
    ;;
  "Regressing")
    echo "❌ Warning! Debt is increasing"
    echo "New critical items: $REGRESSION_COUNT"
    jq '.regressions[] | "\(.location): \(.score) (\(.debt_type))"' comparison.json
    ;;
esac

# Check if target improved (if target was specified)
if jq -e '.target_item' comparison.json > /dev/null; then
  TARGET_STATUS=$(jq -r '.target_item.status' comparison.json)
  echo "Target Status: $TARGET_STATUS"
fi
```

## Troubleshooting

### Understanding Debt Trends

**Issue**: Confused about what the debt trend means

**Solution**: Check the `summary.overall_debt_trend` field in comparison output:

- `Improving` - Total debt decreased by more than 5%
- `Stable` - Total debt changed by less than 5% (within normal variance)
- `Regressing` - Total debt increased by more than 5%

**Check the trend**:
```bash
TREND=$(jq -r '.summary.overall_debt_trend' comparison.json)
DEBT_CHANGE=$(jq '.project_health.changes.debt_score_change_pct' comparison.json)
echo "Debt Trend: $TREND (${DEBT_CHANGE}% change)"
```

### No Improvements Detected

**Issue**: Made changes but comparison shows no improvements

**Possible causes**:
1. Changes didn't reduce debt scores by ≥30% (improvement threshold)
2. Refactored items had scores <60.0 (not tracked as critical)
3. Changes were neutral (e.g., code moved but complexity unchanged)

**Solution**: Check the details:
```bash
# Compare before/after project health
jq '.project_health.before' result.json
jq '.project_health.after' result.json

# Look for critical items in before analysis
jq '.items[] | select(.unified_score.final_score >= 60.0)' before.json
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
| `Resolved` | Item eliminated completely | ✅ Celebrate! Item no longer exists |
| `Improved` | Score reduced significantly | ✅ Good progress, verify metrics improved |
| `Unchanged` | No significant change | ⚠️ Review approach, may need different strategy |
| `Regressed` | Item got worse | ❌ Investigate and fix before merging |
| `NotFoundBefore` | Item didn't exist before | ℹ️ New code, ensure quality is acceptable |
| `NotFound` | Item not found in either | ⚠️ Check target location format |

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

**All Items Resolved**:
```json
{
  "summary": {
    "resolved_count": 25,
    "new_critical_count": 0,
    "overall_debt_trend": "Improving"
  },
  "project_health": {
    "after": {
      "total_items": 0,
      "critical_items": 0
    }
  }
}
```
All debt items resolved - excellent work!

**New Project (Empty Before)**:
```json
{
  "summary": {
    "new_critical_count": 15,
    "resolved_count": 0,
    "overall_debt_trend": "Stable"
  },
  "project_health": {
    "before": {
      "total_items": 0
    }
  }
}
```
New project or first analysis - establish baseline for future comparisons.

**No Changes**:
```json
{
  "summary": {
    "overall_debt_trend": "Stable",
    "new_critical_count": 0,
    "resolved_count": 0
  },
  "improvements": [],
  "regressions": []
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

The compare command provides validation for refactoring efforts:

**Current Capabilities:**
- ✅ Target location tracking with intelligent fuzzy matching
- ✅ Detect regressions (new critical items with score ≥ 60.0)
- ✅ Track resolved items and improvements (≥30% score reduction)
- ✅ Detailed per-item improvement metrics with before/after scores
- ✅ Multiple output formats (JSON, Markdown, Terminal)
- ✅ Implementation plan parsing for target extraction
- ✅ Project-wide health metrics and debt trends
- ✅ Automate quality gates in CI/CD pipelines

Use the compare command regularly to maintain visibility into your codebase's technical health and ensure continuous improvement. All features are fully implemented and ready for production use.

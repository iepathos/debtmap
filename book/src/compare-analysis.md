# Compare Analysis

The `compare` command enables you to track technical debt changes over time by comparing two analysis results. This is essential for validating refactoring efforts, detecting regressions in pull requests, and monitoring project health trends.

## Implementation Status

**Current Implementation (Available Now)**:
- ✅ Basic validation comparing before/after analyses
- ✅ Resolved items tracking (debt eliminated)
- ✅ Improved items detection (score reduction ≥ 30%)
- ✅ New critical items detection (regressions)
- ✅ Unchanged critical items tracking
- ✅ Project health summaries
- ✅ JSON output format
- ✅ CI/CD integration support

**Planned Features (Coming Soon)**:
- 🚧 Target location tracking with fuzzy matching
- 🚧 Detailed improvement percentage calculations (per-item)
- 🚧 Markdown and terminal output formats
- 🚧 Implementation plan parsing for target extraction
- 🚧 Match strategies (Exact, FuzzyFunction, FuzzyFile)

**Note**: This chapter documents both current and planned features. Current implementation focuses on validation workflows (resolved, improved, new critical items). Full comparison features including target tracking are under development.

## Overview

The compare command analyzes differences between "before" and "after" debtmap analyses, providing:

- **Validation tracking** - Verify debt items are resolved or improved
- **Project health metrics** - Track overall debt trends across your codebase
- **Regression detection** - Identify new critical debt items introduced (score ≥ 8.0)
- **Improvement tracking** - Measure and celebrate debt reduction
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

**Currently Available**: Basic comparison with JSON output showing resolved, improved, and new critical items.

**Planned Options** (not yet implemented):
- `--plan FILE` - Implementation plan to extract target location
- `--target-location LOCATION` - Manual target location
- `--format FORMAT` - Output format (markdown, terminal)

## Target Location Tracking

> **⚠️ Planned Feature**: Target location tracking with fuzzy matching is currently under development. The types are defined in `src/comparison/types.rs` but not yet implemented in the compare command. This section describes the planned functionality.

Target location tracking will allow you to monitor specific code locations through refactoring changes.

### Location Format (Planned)

Target locations will use the format: `file:function:line`

Examples:
- `src/main.rs:complex_function:42`
- `lib/parser.rs:parse_expression:156`
- `api/handler.rs:process_request:89`

### Specifying Target Locations (Planned)

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

### Matching Strategies (Planned)

Debtmap will use intelligent matching to find your target item even when code changes:

| Strategy | When Used | Confidence |
|----------|-----------|------------|
| **Exact** | Location matches exactly | 1.0 |
| **FuzzyFunction** | Function moved but name unchanged | 0.8 - 0.95 |
| **FuzzyFile** | File changed but function exists | 0.6 - 0.8 |

The comparison result will include the match strategy and confidence score used.

### Target Status Values (Planned)

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
- **critical_items** - Items with score ≥ 8.0 (critical threshold)
- **high_priority_items** - Items requiring attention
- **average_score** - Mean debt score across all items
- **debt_score_change** - Absolute change in total debt
- **debt_score_change_pct** - Percentage change in total debt

### Debt Trends

The comparison calculates an overall debt trend based on the percentage change:

- **Improving** - Debt decreased by more than 5%
- **Stable** - Debt changed by less than 5% (within normal variance)
- **Regressing** - Debt increased by more than 5%

## Regression Detection

Regressions are new critical debt items (score ≥ 8.0) that appear in the after analysis.

### What Counts as a Regression

A regression is detected when:
1. An item exists in the after analysis
2. The item does NOT exist in the before analysis
3. The item has a debt score ≥ 8.0 (critical severity threshold)

### Regression Output

> **Note**: Current implementation tracks new critical items. Full regression details (debt_type, description) are planned for future releases.

**Current Output Structure** (ValidationResult):
```json
{
  "gaps": {
    "new_critical_item_1": {
      "description": "New critical debt item detected",
      "location": "src/new_feature.rs:process_data",
      "severity": "critical",
      "suggested_fix": "...",
      "current_score": 9.5
    }
  },
  "remaining_issues": [
    "New critical item: src/new_feature.rs:process_data (score: 9.5)"
  ]
}
```

**Planned Output Structure** (ComparisonResult):
```json
{
  "regressions": [
    {
      "location": "src/new_feature.rs:process_data:23",
      "score": 9.5,
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

# Check completion status (current implementation)
STATUS=$(jq -r '.status' result.json)
COMPLETION=$(jq '.completion_percentage' result.json)

if [ "$STATUS" = "regression_detected" ]; then
  echo "❌ Regression detected - new critical debt items found"
  exit 1
fi

# Or check for remaining critical issues
REMAINING=$(jq '.remaining_issues | length' result.json)
if [ "$REMAINING" -gt 0 ]; then
  echo "⚠️ Warning: $REMAINING remaining issues"
  jq '.remaining_issues[]' result.json
fi
```

## Improvement Tracking

Debtmap tracks two types of improvements:

### Improvement Types

1. **Resolved** - Debt item completely eliminated (no longer in after analysis)
2. **Improved** - Debt item still exists but score reduced by ≥ 30%

### Current Implementation

The current implementation provides improvements as human-readable strings in the ValidationResult:

```json
{
  "improvements": [
    "2 high priority items resolved",
    "Average complexity reduction: 35%",
    "3 items showed improvement (score reduction ≥ 30%)"
  ],
  "completion_percentage": 75.0,
  "status": "good_progress"
}
```

### Planned: Detailed Improvement Metrics

Future versions will include per-item improvement metrics:

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

### Planned: Improvement Items List

```json
{
  "improvements": [
    {
      "location": "src/example.rs:complex_function:45",
      "before_score": 10.5,
      "after_score": 4.1,
      "improvement_type": "ScoreReduced"
    },
    {
      "location": "src/legacy.rs:old_code:120",
      "before_score": 9.0,
      "after_score": null,
      "improvement_type": "Resolved"
    }
  ]
}
```

## Before/After Metrics

> **⚠️ Planned Feature**: Detailed target item tracking with before/after metrics is under development.

### Current Implementation

The current implementation provides before/after summaries at the project level:

```json
{
  "before_summary": {
    "total_items": 25,
    "high_priority_items": 8,
    "average_score": 6.5
  },
  "after_summary": {
    "total_items": 22,
    "high_priority_items": 5,
    "average_score": 5.2
  }
}
```

### Planned: Target Metrics Structure

Future versions will support detailed target item comparison:

```json
{
  "target_item": {
    "location": "src/example.rs:complex_function:45",
    "match_strategy": "Exact",
    "match_confidence": 1.0,
    "matched_items_count": 1,
    "before": {
      "score": 10.5,
      "cyclomatic_complexity": 8,
      "cognitive_complexity": 15,
      "coverage": 45.0,
      "function_length": 120,
      "nesting_depth": 4
    },
    "after": {
      "score": 4.1,
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

### Planned: Metric Aggregation

When multiple items match the target location (due to fuzzy matching), metrics will be aggregated:

- **score** - Average across matched items
- **cyclomatic_complexity** - Average
- **cognitive_complexity** - Average
- **coverage** - Average
- **function_length** - Average
- **nesting_depth** - Maximum (worst case)

The `matched_items_count` field will tell you how many items were aggregated.

### Validating Refactoring Success

Use the current validation output to verify your refactoring:

```bash
# Check validation completion
COMPLETION=$(jq '.completion_percentage' result.json)
STATUS=$(jq -r '.status' result.json)

echo "Completion: ${COMPLETION}%"
echo "Status: $STATUS"

# Check for improvements
jq '.improvements[]' result.json

# Verify no regressions
REMAINING=$(jq '.remaining_issues | length' result.json)
if [ "$REMAINING" -eq 0 ]; then
  echo "✅ All issues resolved!"
else
  echo "⚠️ $REMAINING issues remaining"
fi
```

## Output Formats

### JSON Format (Current)

The current JSON format provides validation results:

```bash
debtmap compare --before before.json --after after.json --output result.json
```

The ValidationResult JSON output includes:
- `completion_percentage` - Percentage of debt resolved/improved
- `status` - Overall status (good_progress, regression_detected, etc.)
- `improvements` - List of improvement descriptions
- `remaining_issues` - List of issues still present
- `gaps` - Detailed gap analysis with locations and scores
- `before_summary` - Summary of before analysis
- `after_summary` - Summary of after analysis

Example output:
```json
{
  "completion_percentage": 75.0,
  "status": "good_progress",
  "improvements": [
    "2 high priority items resolved",
    "Average complexity reduction: 35%"
  ],
  "remaining_issues": [
    "1 unchanged critical item: src/legacy.rs:old_function"
  ],
  "gaps": {},
  "before_summary": {
    "total_items": 25,
    "high_priority_items": 8,
    "average_score": 6.5
  },
  "after_summary": {
    "total_items": 22,
    "high_priority_items": 5,
    "average_score": 5.2
  }
}
```

### Markdown Format (Planned)

> **🚧 Coming Soon**: Markdown output format for human-readable reports.

Planned for pull request comments and documentation:

```bash
debtmap compare --before before.json --after after.json --format markdown
```

The markdown output will be suitable for:
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
            --output validation.json

      - name: Check validation result
        run: |
          STATUS=$(jq -r '.status' validation.json)
          COMPLETION=$(jq '.completion_percentage' validation.json)

          echo "Validation Status: $STATUS"
          echo "Completion: ${COMPLETION}%"

          # Fail on regression
          if [ "$STATUS" = "regression_detected" ]; then
            echo "❌ Regression detected"
            jq '.remaining_issues[]' validation.json
            exit 1
          fi

          # Warn on incomplete
          if (( $(echo "$COMPLETION < 50" | bc -l) )); then
            echo "⚠️ Warning: Only ${COMPLETION}% complete"
          fi

      - name: Post validation to PR
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const validation = JSON.parse(fs.readFileSync('validation.json', 'utf8'));

            const body = `## Debt Validation Results

            **Status:** ${validation.status}
            **Completion:** ${validation.completion_percentage}%

            ### Improvements
            ${validation.improvements.map(i => `- ${i}`).join('\n')}

            ${validation.remaining_issues.length > 0 ? `
            ### Remaining Issues
            ${validation.remaining_issues.map(i => `- ${i}`).join('\n')}
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
    - debtmap compare --before before.json --after after.json --output validation.json
    - |
      STATUS=$(jq -r '.status' validation.json)
      COMPLETION=$(jq '.completion_percentage' validation.json)

      echo "Status: $STATUS"
      echo "Completion: ${COMPLETION}%"

      if [ "$STATUS" = "regression_detected" ]; then
        echo "Failed: Regression detected"
        jq '.remaining_issues[]' validation.json
        exit 1
      fi
  artifacts:
    paths:
      - before.json
      - after.json
      - validation.json
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
debtmap compare --before before.json --after after.json --output validation.json

# Check results
cat validation.json | jq '.'
# Output shows: completion_percentage, status, improvements, remaining_issues
```

### Example 2: Validating Function Refactoring

> **Note**: Target location tracking is planned. Current implementation validates overall improvements.

Validate your refactoring work:

```bash
# Run before analysis
debtmap analyze --output before.json

# Identify high-priority items to fix
jq '.items[] | select(.unified_score.final_score >= 8.0)' before.json

# Refactor the high-priority functions...

# Run after analysis
debtmap analyze --output after.json

# Compare and validate
debtmap compare \
  --before before.json \
  --after after.json \
  --output validation.json

# Check validation results
STATUS=$(jq -r '.status' validation.json)
COMPLETION=$(jq '.completion_percentage' validation.json)

echo "Status: $STATUS"
echo "Completion: ${COMPLETION}%"

# Review improvements
jq '.improvements[]' validation.json
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
  --output validation.json

# Check status
STATUS=$(jq -r '.status' validation.json)
echo "Validation Status: $STATUS"

# Example output structure:
jq '.' validation.json
# {
#   "completion_percentage": 100.0,
#   "status": "all_resolved",  // or "regression_detected"
#   "improvements": ["All items resolved"],
#   "remaining_issues": [],
#   "gaps": {},
#   "before_summary": {...},
#   "after_summary": {...}
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

# Check summaries
echo "Before (v1.0):"
jq '.before_summary' v1.0-to-v1.1.json

echo "After (v1.1):"
jq '.after_summary' v1.0-to-v1.1.json

# Calculate improvement
BEFORE_AVG=$(jq '.before_summary.average_score' v1.0-to-v1.1.json)
AFTER_AVG=$(jq '.after_summary.average_score' v1.0-to-v1.1.json)
echo "Average score change: $BEFORE_AVG → $AFTER_AVG"
```

### Example 5: Full CI/CD Workflow

Complete workflow for continuous debt monitoring:

```bash
#!/bin/bash
# ci-debt-check.sh

set -e

BEFORE="before.json"
AFTER="after.json"
VALIDATION="validation.json"

# Step 1: Analyze baseline (main branch)
echo "📊 Analyzing baseline..."
git checkout main
debtmap analyze --output "$BEFORE"

# Step 2: Analyze current branch
echo "📊 Analyzing current branch..."
git checkout -
debtmap analyze --output "$AFTER"

# Step 3: Run validation
echo "🔍 Running validation..."
debtmap compare \
  --before "$BEFORE" \
  --after "$AFTER" \
  --output "$VALIDATION"

# Step 4: Extract metrics
STATUS=$(jq -r '.status' "$VALIDATION")
COMPLETION=$(jq '.completion_percentage' "$VALIDATION")
IMPROVEMENTS=$(jq '.improvements | length' "$VALIDATION")
REMAINING=$(jq '.remaining_issues | length' "$VALIDATION")

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📈 Debt Validation Results"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Status: $STATUS"
echo "Completion: ${COMPLETION}%"
echo "Improvements: $IMPROVEMENTS"
echo "Remaining Issues: $REMAINING"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Step 5: Quality gate
if [ "$STATUS" = "regression_detected" ]; then
  echo "❌ FAILED: Regression detected"
  jq '.remaining_issues[]' "$VALIDATION"
  exit 1
fi

if (( $(echo "$COMPLETION < 50" | bc -l) )); then
  echo "⚠️  WARNING: Completion below 50%"
  # Don't fail, just warn
fi

if [ "$STATUS" = "all_resolved" ]; then
  echo "🎉 SUCCESS: All debt items resolved!"
fi

echo "✅ PASSED: No regressions detected"
```

### Example 6: Interpreting Different Status Outcomes

Understanding what each validation status means:

```bash
# Run validation
debtmap compare --before before.json --after after.json --output validation.json

# Check validation status
STATUS=$(jq -r '.status' validation.json)
COMPLETION=$(jq '.completion_percentage' validation.json)

case "$STATUS" in
  "all_resolved")
    echo "🎉 Success! All debt items eliminated."
    echo "Completion: 100%"
    ;;
  "good_progress")
    echo "✅ Good progress! Completion: ${COMPLETION}%"
    jq '.improvements[]' validation.json
    ;;
  "some_improvement")
    echo "➡️  Some improvement detected: ${COMPLETION}%"
    echo "Remaining issues to address:"
    jq '.remaining_issues[]' validation.json
    ;;
  "regression_detected")
    echo "❌ Regression detected!"
    echo "New critical items or increased debt:"
    jq '.remaining_issues[]' validation.json
    ;;
  "no_change")
    echo "ℹ️  No significant changes detected"
    echo "Consider refactoring high-priority items."
    ;;
esac
```

## Troubleshooting

### Understanding Validation Status

**Issue**: Confused about what validation status means

**Solution**: Check the `status` field in validation output:

- `all_resolved` - All debt items from before analysis are resolved (100% completion)
- `good_progress` - Significant improvements made (typically >70% completion)
- `some_improvement` - Some items improved but work remains (<70% completion)
- `regression_detected` - New critical items detected or debt increased
- `no_change` - No significant changes in debt metrics

**Check completion percentage**:
```bash
COMPLETION=$(jq '.completion_percentage' validation.json)
echo "Validation is ${COMPLETION}% complete"
```

### No Improvements Detected

**Issue**: Made changes but validation shows no improvements

**Possible causes**:
1. Changes didn't reduce debt scores by ≥30% (improvement threshold)
2. Refactored items had scores <8.0 (not tracked as critical)
3. Changes were neutral (e.g., code moved but complexity unchanged)

**Solution**: Check the details:
```bash
# Compare before/after summaries
jq '.before_summary' validation.json
jq '.after_summary' validation.json

# Look for high-priority items in before analysis
jq '.items[] | select(.unified_score.final_score >= 8.0)' before.json
```

### JSON Parsing Errors

**Problem**: `Error parsing JSON file`

**Solutions**:
1. Verify the file is valid JSON: `jq . before.json`
2. Ensure the file is a debtmap analysis output
3. Check file permissions and path
4. Regenerate the analysis if corrupted

### Understanding Validation Status Values

| Status | Meaning | Action Required |
|--------|---------|-----------------|
| `all_resolved` | All items eliminated | ✅ Celebrate! Document what worked |
| `good_progress` | Significant improvement | ✅ Good progress, verify remaining items |
| `some_improvement` | Partial improvement | ⚠️ Continue refactoring remaining issues |
| `regression_detected` | New critical debt | ❌ Investigate and fix before merging |
| `no_change` | No significant change | ⚠️ Review approach, may need different strategy |

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

The compare command provides validation for refactoring efforts:

**Current Capabilities:**
- ✅ Validate debt resolution with before/after comparison
- ✅ Detect regressions (new critical items with score ≥ 8.0)
- ✅ Track resolved items and improvements (≥30% score reduction)
- ✅ Calculate completion percentage for validation workflows
- ✅ Automate quality gates in CI/CD pipelines
- ✅ Generate structured JSON output for programmatic use

**Coming Soon:**
- 🚧 Target location tracking with fuzzy matching
- 🚧 Detailed per-item improvement metrics
- 🚧 Markdown and terminal output formats
- 🚧 Implementation plan parsing for target extraction

Use the compare command regularly to maintain visibility into your codebase's technical health and ensure continuous improvement. The current implementation focuses on validation workflows - perfect for CI/CD integration and refactoring validation.

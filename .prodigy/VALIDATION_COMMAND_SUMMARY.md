# Debtmap Validation Command Implementation

## Overview

The `prodigy-validate-debtmap-improvement` command validates technical debt improvements by analyzing comparison results from the `debtmap compare` command.

## Command Usage

```bash
ARGUMENTS="--comparison <path> --output <path>" ./target/debug/prodigy-validate-debtmap-improvement
```

### Parameters

- `--comparison <path>` (required): Path to comparison JSON from `debtmap compare`
- `--output <path>` (optional): Path to write validation results (default: `.prodigy/debtmap-validation.json`)

### Environment Variables

- `PRODIGY_AUTOMATION=true`: Suppress verbose output
- `PRODIGY_VALIDATION=true`: Alternative automation flag

## Input Format

The command expects a `ComparisonResult` JSON from `debtmap compare`:

```json
{
  "target_item": {
    "location": "file:function:line",
    "before": { "score": 81.9, ... },
    "after": { "score": 15.2, ... },
    "improvements": {
      "score_reduction_pct": 81.4,
      ...
    },
    "status": "Improved"
  },
  "project_health": {
    "before": { "total_debt_score": 1247.3, ... },
    "after": { "total_debt_score": 1182.6, ... },
    "changes": { "debt_score_change_pct": -5.2, ... }
  },
  "regressions": [...],
  "summary": { ... }
}
```

## Output Format

Validation results JSON:

```json
{
  "completion_percentage": 72.3,
  "status": "incomplete",
  "improvements": [
    "Target item score reduced by 81.4% (81.9 → 15.2)",
    "Overall project debt reduced by 5.2%"
  ],
  "remaining_issues": [
    "1 new critical debt item introduced"
  ],
  "gaps": {
    "regression_0": {
      "description": "New complex helper function",
      "location": "file:function:line",
      "severity": "high",
      "suggested_fix": "Simplify using pure functional patterns",
      "current_score": 65.3
    }
  },
  "target_summary": {
    "location": "file:function:line",
    "score_before": 81.9,
    "score_after": 15.2,
    "improvement_percent": 81.4,
    "status": "improved"
  },
  "project_summary": {
    "total_debt_before": 1247.3,
    "total_debt_after": 1182.6,
    "improvement_percent": -5.2,
    "items_resolved": 12,
    "items_new": 1
  }
}
```

## Scoring Algorithm

The improvement score is calculated as a weighted composite:

```
improvement_score =
  target_component * 0.5 +           # 50%: Target item improvement
  project_health_component * 0.3 +   # 30%: Overall debt improvement
  no_regression_component * 0.2      # 20%: No new critical items
```

Where:
- `target_component` = target item score reduction percentage (0-100)
- `project_health_component` = min(100, abs(debt_change_pct) * 10)
- `no_regression_component` = max(0, 100 - regression_count * 20)

### Status Determination

- `complete`: improvement_score >= 75.0%
- `incomplete`: improvement_score < 75.0%

## Gap Detection

The command identifies specific improvement gaps:

1. **Insufficient Target Improvement** - Target item unchanged or minimally improved
2. **Regression Introduced** - New critical debt items created
3. **Project Health Degradation** - Overall debt increased

## Test Results

### Successful Case (76.3%)
- Target: 81.4% reduction
- Project: 5.2% debt reduction
- Regressions: 0
- Status: **complete**

### Incomplete Case (72.3%)
- Target: 81.4% reduction
- Project: 5.2% debt reduction
- Regressions: 1 new critical item
- Status: **incomplete** (regression penalty)

### Unchanged Target (20.3%)
- Target: 0.5% reduction (effectively unchanged)
- Project: 0.02% debt reduction
- Regressions: 0
- Status: **incomplete** (target not improved)

## Integration with Prodigy Workflows

The validation command is designed to be called by Prodigy workflows:

1. Workflow captures before state (`debtmap analyze`)
2. Claude implements improvements
3. Workflow captures after state (`debtmap analyze`)
4. Workflow generates comparison (`debtmap compare`)
5. **This command validates improvement** (using comparison.json)
6. If incomplete, workflow can retry or refine

## Error Handling

- Missing `--comparison` parameter: Error with clear message
- Nonexistent comparison file: Error with file path
- Invalid JSON: Error with parse details
- All errors exit with non-zero code

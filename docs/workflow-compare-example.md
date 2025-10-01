# Debtmap Compare Command - Workflow Integration

This document demonstrates how to integrate the `debtmap compare` command into automated workflows for technical debt management.

## Overview

The `debtmap compare` command efficiently compares two debtmap analysis results to:
- Identify resolved, improved, worsened, and new technical debt items
- Calculate project health metrics and trends
- Track specific target items through refactoring efforts
- Generate validation results for automated workflows

**Key Benefits:**
- **Efficiency**: Reduces validation input from 40MB+ to <10KB (99.975% reduction)
- **Accuracy**: Purpose-built comparison logic handles edge cases
- **Integration**: Structured JSON output for automated processing
- **Flexibility**: Multiple output formats (JSON, markdown, terminal)

## Basic Usage

### Simple Comparison

Compare two analysis snapshots:

```bash
debtmap compare \
  --before debtmap-before.json \
  --after debtmap-after.json \
  --output comparison-result.json \
  --format json
```

### Target-Specific Comparison

Track a specific debt item through refactoring:

```bash
debtmap compare \
  --before debtmap-before.json \
  --after debtmap-after.json \
  --plan implementation-plan.md \
  --output validation-result.json \
  --format json
```

The `--plan` flag extracts the target location from the implementation plan and includes detailed target item comparison in the output.

## Workflow Integration Examples

### Example 1: Pre-Commit Validation

Validate that changes don't introduce regressions:

```bash
#!/bin/bash
# Run analysis before changes
debtmap analyze --output debtmap-before.json

# Make changes...

# Run analysis after changes
debtmap analyze --output debtmap-after.json

# Compare results
debtmap compare \
  --before debtmap-before.json \
  --after debtmap-after.json \
  --output validation.json \
  --format json

# Check for regressions
NEW_ITEMS=$(jq -r '.regressions.new_items | length' validation.json)
WORSENED=$(jq -r '.regressions.worsened_items | length' validation.json)

if [ "$NEW_ITEMS" -gt 0 ] || [ "$WORSENED" -gt 0 ]; then
  echo "❌ Regressions detected!"
  jq '.summary.status' validation.json
  exit 1
fi

echo "✅ No regressions detected"
```

### Example 2: Automated Debt Fixing Workflow

Track progress through iterative debt fixing:

```bash
#!/bin/bash
BEFORE="debtmap-baseline.json"
PLAN="implementation-plan.md"
AFTER="debtmap-current.json"

# Initial baseline
debtmap analyze --output "$BEFORE"

# Fix debt items (could be automated or manual)
# ... fixing code ...

# Re-analyze
debtmap analyze --output "$AFTER"

# Compare with target tracking
debtmap compare \
  --before "$BEFORE" \
  --after "$AFTER" \
  --plan "$PLAN" \
  --output validation.json \
  --format json

# Check target item status
TARGET_STATUS=$(jq -r '.target_item.status' validation.json)
METRICS_IMPROVED=$(jq -r '.target_item.metrics_improved' validation.json)

if [ "$TARGET_STATUS" = "improved" ] && [ "$METRICS_IMPROVED" = "true" ]; then
  echo "✅ Target debt item successfully improved!"

  # Extract improvement details
  jq -r '.target_item | "Location: \(.location)\nBefore: \(.before_score)\nAfter: \(.after_score)\nDelta: \(.score_delta)"' validation.json
else
  echo "⚠️ Target item not fully resolved"
  jq -r '.target_item.gaps[]' validation.json
fi
```

### Example 3: CI/CD Pipeline Integration

```yaml
# .github/workflows/debt-check.yml
name: Technical Debt Check

on: [pull_request]

jobs:
  debt-analysis:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0

      - name: Install Debtmap
        run: cargo install --path .

      - name: Analyze baseline (main branch)
        run: |
          git checkout main
          debtmap analyze --output debtmap-baseline.json

      - name: Analyze PR branch
        run: |
          git checkout ${{ github.head_ref }}
          debtmap analyze --output debtmap-pr.json

      - name: Compare results
        run: |
          debtmap compare \
            --before debtmap-baseline.json \
            --after debtmap-pr.json \
            --output comparison.json \
            --format json

      - name: Validate no regressions
        run: |
          NEW_HIGH=$(jq -r '[.regressions.new_items[] | select(.severity == "high")] | length' comparison.json)
          WORSENED_HIGH=$(jq -r '[.regressions.worsened_items[] | select(.after_severity == "high")] | length' comparison.json)

          if [ "$NEW_HIGH" -gt 0 ] || [ "$WORSENED_HIGH" -gt 0 ]; then
            echo "❌ High-severity technical debt regressions detected!"
            jq '.regressions' comparison.json
            exit 1
          fi

      - name: Post results as comment
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const comparison = JSON.parse(fs.readFileSync('comparison.json', 'utf8'));

            const comment = `## Technical Debt Analysis

            **Status**: ${comparison.summary.status}

            ### Changes
            - ✅ Resolved: ${comparison.summary.resolved_count}
            - 📈 Improved: ${comparison.summary.total_improvements}
            - 📉 Worsened: ${comparison.summary.total_regressions}

            ### Project Health
            - Total items: ${comparison.project_health.total_items_before} → ${comparison.project_health.total_items_after}
            - High priority: ${comparison.project_health.high_priority_before} → ${comparison.project_health.high_priority_after}
            `;

            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: comment
            });
```

### Example 4: MapReduce Workflow Integration

For the Prodigy MapReduce workflow that fixes multiple debt items in parallel:

```bash
#!/bin/bash
# This replaces the manual jq-based comparison in prodigy-compare-debt-results

BEFORE="$1"
AFTER="$2"
SUCCESSFUL="$3"
FAILED="$4"

# Run efficient comparison
debtmap compare \
  --before "$BEFORE" \
  --after "$AFTER" \
  --output comparison-result.json \
  --format json

# Extract metrics for commit message
RESOLVED=$(jq -r '.summary.resolved_count' comparison-result.json)
IMPROVED=$(jq -r '.summary.total_improvements' comparison-result.json)
STATUS=$(jq -r '.summary.status' comparison-result.json)

BEFORE_ITEMS=$(jq -r '.project_health.total_items_before' comparison-result.json)
AFTER_ITEMS=$(jq -r '.project_health.total_items_after' comparison-result.json)
BEFORE_HIGH=$(jq -r '.project_health.high_priority_before' comparison-result.json)
AFTER_HIGH=$(jq -r '.project_health.high_priority_after' comparison-result.json)

# Create commit
git add -A
git commit -m "fix: eliminate $SUCCESSFUL technical debt items via MapReduce

Processed debt items in parallel:
- Successfully fixed: $SUCCESSFUL items
- Failed to fix: $FAILED items

Technical Debt Analysis:
- Status: $STATUS
- Items resolved: $RESOLVED
- Items improved: $IMPROVED
- Total items: $BEFORE_ITEMS → $AFTER_ITEMS
- High priority: $BEFORE_HIGH → $AFTER_HIGH

Analysis performed by debtmap compare command."
```

## Output Format Reference

### Comparison Result Structure

```json
{
  "metadata": {
    "before_analysis_date": "2025-10-01T10:00:00Z",
    "after_analysis_date": "2025-10-01T11:00:00Z",
    "comparison_date": "2025-10-01T11:05:00Z"
  },
  "target_item": {
    "location": "src/example.rs:45",
    "status": "improved",
    "before_score": 85.5,
    "after_score": 42.3,
    "score_delta": -43.2,
    "before_severity": "high",
    "after_severity": "medium",
    "metrics_improved": true,
    "gaps": []
  },
  "project_health": {
    "total_items_before": 45,
    "total_items_after": 37,
    "high_priority_before": 12,
    "high_priority_after": 8,
    "avg_score_before": 58.3,
    "avg_score_after": 45.7
  },
  "improvements": {
    "resolved_items": [...],
    "improved_items": [...]
  },
  "regressions": {
    "worsened_items": [...],
    "new_items": [...]
  },
  "summary": {
    "status": "significant progress",
    "resolved_count": 8,
    "total_improvements": 15,
    "total_regressions": 2
  }
}
```

## Best Practices

1. **Baseline Establishment**: Always create a baseline analysis before starting work
2. **Frequent Comparisons**: Run comparisons after each significant change
3. **Target Tracking**: Use `--plan` flag to track specific debt items through implementation
4. **Automation**: Integrate into CI/CD to prevent regressions
5. **Threshold Setting**: Define acceptable thresholds for new/worsened items

## Performance Considerations

- **File Size**: Compare handles multi-MB JSON files efficiently
- **Memory Usage**: Streams data to minimize memory footprint
- **Speed**: Typical comparison completes in <1 second for 40MB+ files
- **Output Size**: JSON output is typically <10KB regardless of input size

## Troubleshooting

### No target item found
If target item is not found in analysis:
- Verify location format matches debtmap output
- Check if file was renamed or function moved
- Ensure analysis includes the target file

### Large number of "new" items
This can occur if:
- Analysis configuration changed
- File paths changed (e.g., directory restructure)
- Detection thresholds adjusted

Use the `--plan` flag with explicit location to focus on specific items.

## Related Commands

- `debtmap analyze`: Generate initial analysis
- `/prodigy-validate-debt-fix`: Validate specific debt fixes
- `/prodigy-compare-debt-results`: Claude command using compare

## Further Reading

- [Debtmap CLI Reference](./cli-reference.md)
- [Workflow Integration Guide](./workflow-integration.md)
- [Technical Debt Analysis](./technical-debt-analysis.md)

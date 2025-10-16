# Validate Debtmap Improvement Command

Validates that technical debt improvements have been made by analyzing the compact comparison output from `debtmap compare`.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-validate-debtmap-improvement --comparison <comparison-json-file> [--previous-validation <filepath>] [--output <filepath>]
```

Examples:
- `/prodigy-validate-debtmap-improvement --comparison .prodigy/comparison.json --output .prodigy/debtmap-validation.json`
- `/prodigy-validate-debtmap-improvement --comparison .prodigy/comparison.json --previous-validation .prodigy/debtmap-validation.json --output .prodigy/debtmap-validation.json`

## What This Command Does

1. **Reads Compact Comparison**
   - Loads JSON comparison output from `debtmap compare` command
   - Contains target item analysis, regressions, improvements, and project health
   - Validates that improvements were made

2. **Analyzes Improvement Quality**
   - Checks if target debt item was successfully improved
   - Validates that no new critical issues were introduced (regressions)
   - Ensures overall project technical debt improved

3. **Outputs Validation Result**
   - Produces JSON-formatted validation result for Prodigy to parse
   - Includes improvement percentage and detailed gap analysis
   - Provides actionable feedback for incomplete improvements

## Execution Process

### Step 1: Parse Arguments and Load Data

The command will:
- Parse $ARGUMENTS to extract:
  - `--comparison` parameter: Path to comparison JSON (from `debtmap compare`)
  - `--previous-validation` parameter: Path to previous validation result (optional)
  - `--output` parameter: Filepath to write results (defaults to `.prodigy/debtmap-validation.json`)
- Load comparison JSON and validate it contains comparison output
- If `--previous-validation` provided, load it to track progress:
  ```json
  {
    "completion_percentage": 72.3,
    "attempt_number": 1
  }
  ```

### Step 2: Analyze Target Item Improvement

The comparison.json contains a `target_item` field with detailed analysis:

```json
{
  "target_item": {
    "location": "src/analyzers/rust_analyzer.rs:build_call_graph:523",
    "before": {
      "unified_score": { "final_score": 81.9 },
      "complexity": { "cognitive": 22 }
    },
    "after": {
      "unified_score": { "final_score": 15.2 },
      "complexity": { "cognitive": 3 }
    },
    "improvement": {
      "score_reduction": 66.7,
      "score_reduction_percent": 81.4,
      "complexity_reduction": 19,
      "status": "significantly_improved"
    }
  }
}
```

Extract:
- **Target score improvement**: `improvement.score_reduction_percent`
- **Target status**: `improvement.status` (significantly_improved, moderately_improved, slightly_improved, unchanged, degraded)
- **Complexity reduction**: `improvement.complexity_reduction`

### Step 3: Check for Regressions

The comparison.json contains a `regressions` array with new critical debt items:

```json
{
  "regressions": [
    {
      "location": "src/analyzers/rust_analyzer.rs:process_helper:589",
      "score": 65.3,
      "description": "New complex helper function introduced during refactoring"
    }
  ]
}
```

Calculate regression penalty:
- Each new critical item (score >= 60) reduces improvement score by 20%
- Maximum regression penalty: 100% (complete failure)

### Step 4: Analyze Project Health

The comparison.json contains overall project metrics:

```json
{
  "project_health": {
    "total_debt_score_before": 1247.3,
    "total_debt_score_after": 1182.6,
    "improvement": 64.7,
    "improvement_percent": 5.2,
    "items_before": 1293,
    "items_after": 1285,
    "items_resolved": 12,
    "items_new": 4
  }
}
```

Extract:
- **Overall debt improvement**: `project_health.improvement_percent`
- **Items resolved vs new**: Compare `items_resolved` vs `items_new`

### Step 5: Calculate Improvement Score and Track Progress

Calculate improvement percentage using the formula:

```
target_component = target_item.improvement.score_reduction_percent
regression_penalty = min(100, len(regressions) * 20)
no_regression_component = max(0, 100 - regression_penalty)
project_health_component = min(100, project_health.improvement_percent * 10)

improvement_score = (
    target_component * 0.5 +           # 50%: Target item improved
    project_health_component * 0.3 +   # 30%: Overall debt improved
    no_regression_component * 0.2      # 20%: No new critical items
)
```

**Track attempt progress** if previous validation provided:
```python
previous_completion = previous_validation.get("completion_percentage")
current_completion = improvement_score
attempt_number = previous_validation.get("attempt_number", 0) + 1

if previous_completion:
    change = current_completion - previous_completion
    direction = "regression" if change < -5 else "progress" if change > 5 else "stable"

    if direction == "regression":
        recommendation = "CRITICAL: Stop refactoring. Return to original plan and complete remaining items."
    elif direction == "stable":
        recommendation = "Progress stalled. Focus on completing specific plan items rather than refactoring."
    else:
        recommendation = "Continue completing remaining plan items."
```

### Step 6: Identify Remaining Work vs Regressions

**CRITICAL**: Distinguish between "work to complete" and "problems to fix".

If improvement score < threshold (75%), analyze the comparison to extract:

#### A. Remaining Plan Items (Primary Focus)

Read the plan file and cross-reference with comparison to identify:
```json
"remaining_plan_items": [
  "Stage 3: Extract output capture logic - Not started",
  "Stage 4: Extract validation processing - Not started",
  "Stage 5: Final cleanup and documentation - Not started"
]
```

**How to extract**:
- Parse IMPLEMENTATION_PLAN.md to find stages/items
- Check which stages are marked complete vs incomplete
- List incomplete stages in this section

#### B. Completed Items (Preserve These)

From the comparison, identify what HAS improved:
```json
"completed_items": [
  "Target item complexity reduced by 40%",
  "Extracted step initialization logic",
  "Added test coverage for error paths"
]
```

**How to extract**:
- Look at `target_item.improvement.status`
- Check `improvements` array in comparison
- List specific achievements

#### C. Regressions to Fix (Secondary - Only if Blocking)

From comparison `regressions` array:
```json
"regressions_to_fix": [
  {
    "location": "src/executor.rs:new_helper:123",
    "score": 65.3,
    "issue": "New complex helper function introduced",
    "blocks_tests": false,
    "action": "Only fix if causes test failures, otherwise ignore"
  }
]
```

**How to determine**:
- Regressions that cause test failures → Must fix
- Regressions that don't block tests → Ignore for now
- Focus on completing plan items instead

### Step 7: Write Validation Results

**CRITICAL**: Write validation results to the output file:

1. **Use output location from `--output` parameter**:
   - This should have been parsed from $ARGUMENTS
   - If not provided, use default `.prodigy/debtmap-validation.json`

2. **Write JSON to file**:
   - Create parent directories if needed
   - Write the JSON validation result to the file
   - Ensure file is properly closed and flushed

3. **Do NOT output JSON to stdout** - Prodigy will read from the file

The JSON format is:

```json
{
  "completion_percentage": 72.3,
  "status": "incomplete",
  "attempt_number": 1,

  "completed_items": [
    "Target item complexity reduced by 40%",
    "Extracted step initialization logic",
    "Extracted command execution logic",
    "Added test coverage for error paths"
  ],

  "remaining_plan_items": [
    "Stage 3: Extract output capture logic - Not started",
    "Stage 4: Extract validation processing - Not started",
    "Stage 5: Final cleanup and documentation - Not started"
  ],

  "regressions_to_fix": [
    {
      "location": "src/executor.rs:new_helper:123",
      "score": 65.3,
      "issue": "New complex helper function introduced",
      "blocks_tests": false,
      "priority": "low"
    }
  ],

  "trend_analysis": {
    "direction": "progress",
    "previous_completion": null,
    "change": null,
    "recommendation": "Continue completing remaining plan items"
  },

  "target_summary": {
    "location": "src/analyzers/rust_analyzer.rs:build_call_graph:523",
    "score_before": 81.9,
    "score_after": 49.0,
    "improvement_percent": 40.2,
    "status": "moderately_improved"
  },

  "project_summary": {
    "total_debt_before": 1247.3,
    "total_debt_after": 1210.5,
    "improvement_percent": 3.0,
    "items_resolved": 8,
    "items_new": 2
  }
}
```

## Validation Rules

### Improvement Scoring

- **90-100%**: Excellent improvement - target resolved, no regressions, project health improved
- **75-89%**: Good improvement - significant target progress, minimal regressions
- **60-74%**: Moderate improvement - target improved but regressions or incomplete work
- **40-59%**: Minor improvement - target barely improved or significant regressions
- **Below 40%**: Insufficient improvement or major regressions

### Component Weights

1. **Target Item (50%)**
   - Primary goal - must make significant progress on target debt item
   - Score reduction >= 75% → 100 points
   - Score reduction 50-75% → 70 points
   - Score reduction 25-50% → 40 points
   - Score reduction < 25% → 10 points

2. **Project Health (30%)**
   - Overall debt should improve or stay stable
   - 5% improvement → 50 points
   - 0% change → 0 points
   - Negative change → negative points (capped at -100)

3. **No Regressions (20%)**
   - New critical items significantly impact score
   - 0 regressions → 100 points
   - 1 regression → 80 points
   - 2 regressions → 60 points
   - 3+ regressions → 40 points or less

## Comparison JSON Format

The comparison.json file is generated by `debtmap compare` and contains:

```json
{
  "metadata": {
    "before_file": ".prodigy/debtmap-before.json",
    "after_file": ".prodigy/debtmap-after.json",
    "plan_file": ".prodigy/IMPLEMENTATION_PLAN.md",
    "timestamp": "2025-10-01T10:30:00Z"
  },
  "target_item": {
    "location": "file:function:line",
    "before": { /* full debt item */ },
    "after": { /* full debt item or null if resolved */ },
    "improvement": {
      "score_reduction": 66.7,
      "score_reduction_percent": 81.4,
      "complexity_reduction": 19,
      "status": "significantly_improved"
    }
  },
  "project_health": {
    "total_debt_score_before": 1247.3,
    "total_debt_score_after": 1182.6,
    "improvement": 64.7,
    "improvement_percent": 5.2,
    "items_before": 1293,
    "items_after": 1285,
    "items_resolved": 12,
    "items_new": 4
  },
  "regressions": [
    {
      "location": "file:function:line",
      "score": 65.3,
      "description": "New complex function",
      "item": { /* full debt item */ }
    }
  ],
  "improvements": [
    {
      "location": "file:function:line",
      "score_before": 72.1,
      "score_after": 42.3,
      "improvement": 29.8
    }
  ],
  "summary": {
    "target_improved": true,
    "regressions_count": 1,
    "improvements_count": 8,
    "net_improvement": true
  }
}
```

## Automation Mode Behavior

**Automation Detection**: Checks for `PRODIGY_AUTOMATION=true` or `PRODIGY_VALIDATION=true` environment variables.

**In Automation Mode**:
- Skip interactive prompts
- Output minimal progress messages
- Always output JSON result at the end
- Exit with appropriate code (0 for complete, 1 for incomplete)

## Error Handling

The command will:
- Handle missing or malformed comparison JSON files gracefully
- Work with partial comparison outputs
- Provide clear error messages
- Always output valid JSON (even on errors)

## Example Validation Outputs

### Successful Validation (85%)
```json
{
  "completion_percentage": 85.0,
  "status": "complete",
  "improvements": [
    "Target item score reduced by 81.4% (81.9 → 15.2)",
    "Cognitive complexity reduced by 19 points",
    "Overall project debt reduced by 5.2%",
    "Resolved 12 debt items, introduced 4 new items"
  ],
  "remaining_issues": [],
  "gaps": {}
}
```

### Incomplete Improvement (65%)
```json
{
  "completion_percentage": 65.0,
  "status": "incomplete",
  "improvements": [
    "Target item score reduced by 40.2% (81.9 → 49.0)",
    "Reduced complexity by 8 points"
  ],
  "remaining_issues": [
    "1 new critical debt item introduced",
    "Target improvement insufficient (40% vs 75% goal)"
  ],
  "gaps": {
    "insufficient_target_improvement": {
      "description": "Target function still above complexity threshold",
      "location": "src/analyzers/rust_analyzer.rs:build_call_graph:523",
      "severity": "medium",
      "suggested_fix": "Further extract helper functions to reduce complexity below 10",
      "original_score": 81.9,
      "current_score": 49.0,
      "target_score": 20.0
    },
    "regression_introduced": {
      "description": "New complex helper function created during refactoring",
      "location": "src/analyzers/rust_analyzer.rs:process_node:589",
      "severity": "high",
      "suggested_fix": "Simplify process_node using pure functional patterns",
      "current_score": 65.3
    }
  }
}
```

### Validation with Major Regressions (35%)
```json
{
  "completion_percentage": 35.0,
  "status": "incomplete",
  "improvements": [
    "Target item score reduced by 50.1% (81.9 → 40.8)"
  ],
  "remaining_issues": [
    "3 new critical debt items introduced during refactoring",
    "Overall project debt increased by 2.3%"
  ],
  "gaps": {
    "major_regressions": {
      "description": "Refactoring created 3 new complex functions",
      "severity": "critical",
      "suggested_fix": "Simplify new helper functions or consolidate logic differently",
      "new_items": [
        "src/analyzers/rust_analyzer.rs:process_helper_a:589 (score: 67.2)",
        "src/analyzers/rust_analyzer.rs:process_helper_b:623 (score: 58.1)",
        "src/analyzers/rust_analyzer.rs:validate_result:701 (score: 62.4)"
      ]
    }
  }
}
```

## Integration with Workflows

This command is designed to work with Prodigy workflows:

1. **Workflow captures before state** (debtmap analyze)
2. **Workflow runs debtmap fix command** (Claude implementation)
3. **Workflow captures after state** (debtmap analyze)
4. **Workflow generates comparison** (`debtmap compare`)
5. **This command validates improvement** (using comparison.json)
6. **If incomplete, workflow triggers completion logic**
7. **Process repeats up to max_attempts**

## Important Implementation Notes

1. **Parse arguments correctly** - Extract comparison and output paths from $ARGUMENTS
2. **Read compact comparison.json** - Much smaller than before/after files (10KB vs 40MB)
3. **Extract improvement metrics** - Use pre-calculated values from comparison
4. **Calculate composite score** - 50% target, 30% project health, 20% no regressions
5. **Write JSON to file**:
   - Use path from `--output` parameter, or default `.prodigy/debtmap-validation.json`
   - Create parent directories if they don't exist
   - Write complete JSON validation result to the file
6. **Always write valid JSON** to the file, even if validation fails
7. **Exit code 0** indicates command ran successfully (regardless of validation result)
8. **Improvement percentage** determines if validation passed based on threshold
9. **Gap details** help subsequent commands fix remaining issues
10. **Do NOT output JSON to stdout** - only progress messages should go to stdout
11. **Trust comparison.json data** - All analysis already done by `debtmap compare`
12. **Focus on actionable gaps** - Identify specific remaining work needed

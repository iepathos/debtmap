# Validate Debtmap Improvement Command

Validates that technical debt improvements have been made by comparing debtmap JSON output before and after changes.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-validate-debtmap-improvement --before <before-json-file> --after <after-json-file> --plan <plan-file> [--output <filepath>]
```

Examples:
- `/prodigy-validate-debtmap-improvement --before .prodigy/debtmap-before.json --after .prodigy/debtmap-after.json --plan .prodigy/IMPLEMENTATION_PLAN.md --output .prodigy/debtmap-validation.json`

## What This Command Does

1. **Compares Debtmap States**
   - Loads JSON output from before and after the fix attempt
   - Identifies changes in debt items and overall metrics
   - Validates that improvements were made

2. **Analyzes Improvement Quality**
   - Checks if high-priority debt items were addressed
   - Validates that technical debt score improved
   - Ensures no new critical issues were introduced

3. **Outputs Validation Result**
   - Produces JSON-formatted validation result for Prodigy to parse
   - Includes improvement percentage and detailed gap analysis
   - Provides actionable feedback for incomplete improvements

## Execution Process

### Step 1: Parse Arguments, Load Data, and Identify Target Item

The command will:
- Parse $ARGUMENTS to extract:
  - `--before` parameter with path to pre-fix debtmap JSON
  - `--after` parameter with path to post-fix debtmap JSON
  - **`--plan` parameter with path to implementation plan (REQUIRED)**
  - `--output` parameter with filepath (required when called from workflow)
- If missing parameters, fail with error message
- If no `--output` parameter, default to `.prodigy/debtmap-validation.json`
- Load both JSON files and validate they contain debtmap output

**CRITICAL - Identify the target debt item:**
1. Read the implementation plan file (markdown)
2. Extract the target location from the plan:
   - Look for "**Location**:" or "Problem location" in the plan
   - Parse format: `./path/to/file.rs:function_name:line_number`
   - Example: `./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120`
3. This is the ONLY item to validate - ignore all other debt items

### Step 2: Extract Target Item from Before/After States

**CRITICAL**: Only compare the single target item identified in Step 1.

1. **Find target in before JSON:**
   - Search `before['items']` for item matching target location
   - Match on: `location.file`, `location.function`, `location.line`
   - Store as `before_target_item`

2. **Find target in after JSON:**
   - Search `after['items']` for item matching target location
   - Store as `after_target_item`
   - If not found → item was completely resolved! (100% improvement)

3. **Extract metrics for comparison:**
   - From `before_target_item`:
     - Score: `unified_score.final_score`
     - Complexity: `debt_type` details (cyclomatic, cognitive)
     - Coverage: from `debt_type.TestingGap.coverage` if present
   - From `after_target_item`:
     - Same metrics as above

### Step 3: Calculate Target Item Improvement

Compare ONLY the target item's metrics:

**If target item resolved (not in after):**
- Item completely eliminated from debt report
- Improvement score: 100%
- Status: "complete"

**If target item still present:**
Calculate improvement based on:
1. **Score reduction**: `(before_score - after_score) / before_score * 100`
2. **Complexity reduction**: Check if cyclomatic/cognitive improved
3. **Coverage improvement**: Check if test coverage increased
4. **Metrics changes**: Any other relevant metrics that improved

### Step 4: Calculate Improvement Score (ENHANCED - Target + Project Health)

**CRITICAL**: Validate both target improvement AND overall project health to catch regressions.

**Why**: Refactoring can shift debt to new locations. Must check:
1. Target item improved (primary goal)
2. No new critical debt introduced (regression check)
3. Overall project debt didn't increase (holistic view)

```python
# 1. Find and compare target item
target_before = find_item(before, target_location)
target_after = find_item(after, target_location)

if not target_after:
    # Item completely resolved
    target_score_reduction = 100.0
else:
    # Calculate target item improvements
    before_score = target_before['unified_score']['final_score']
    after_score = target_after['unified_score']['final_score']
    target_score_reduction = max(0, (before_score - after_score) / before_score * 100)

    before_complexity = get_complexity(target_before)
    after_complexity = get_complexity(target_after)
    target_complexity_reduction = max(0, (before_complexity - after_complexity) / before_complexity * 100)

    before_coverage = get_coverage(target_before)
    after_coverage = get_coverage(target_after)
    target_coverage_improvement = max(0, after_coverage - before_coverage)

# 2. Check for NEW critical debt items (regression detection)
before_critical_items = {item_key(i) for i in before['items'] if i['unified_score']['final_score'] >= 60.0}
after_critical_items = {item_key(i) for i in after['items'] if i['unified_score']['final_score'] >= 60.0}
new_critical_items = after_critical_items - before_critical_items

# Calculate regression penalty
regression_penalty = min(100, len(new_critical_items) * 20)  # -20% per new critical item

# 3. Check overall project debt
total_debt_before = before['total_debt_score']
total_debt_after = after['total_debt_score']
total_debt_improvement = max(0, (total_debt_before - total_debt_after) / total_debt_before * 100) if total_debt_before > 0 else 0

# 4. Calculate composite improvement score
# Primary: Target item improved (50% weight)
all_target_improvements = [target_score_reduction, target_complexity_reduction, target_coverage_improvement]
max_target_improvement = max(all_target_improvements)
other_target_improvements = sum(all_target_improvements) - max_target_improvement
target_component = max_target_improvement * 0.7 + other_target_improvements * 0.15

# Secondary: Project health (30% weight)
project_health_component = total_debt_improvement if regression_penalty == 0 else 0

# Tertiary: No regressions (20% weight)
no_regression_component = 100 - regression_penalty

# Final score
improvement_score = (
    target_component * 0.5 +           # 50%: Target improved
    project_health_component * 0.3 +   # 30%: Overall debt improved
    no_regression_component * 0.2      # 20%: No new critical items
)

status = 'complete' if improvement_score >= 75.0 else 'incomplete'
```

**Key features:**
- ✅ Target item is primary focus (50% weight)
- ✅ Detects and penalizes new critical debt items
- ✅ Validates overall project health
- ✅ 75% threshold achievable with good refactoring + no regressions
- ✅ Fails if refactoring creates more problems than it solves

### Step 5: Identify Improvement Gaps (ENHANCED - Target + Regressions)

If improvement score < threshold (75%), identify specific gaps:

**A. Target Item Gaps**:

1. **Target Item Still Present** (if item not resolved):
   - Include target item location and current metrics
   - Show what changed vs what didn't change
   - Identify which metrics need more work

2. **Insufficient Score Reduction**:
   - Target item score reduced by less than 50%
   - Example: 81.9 → 70.0 (14.5% reduction) - need ~50% for good score
   - Suggest: More aggressive refactoring needed

3. **Complexity Not Reduced Enough**:
   - Cyclomatic/cognitive complexity still high
   - Example: 17 → 12 (29% reduction) - need ~40% for good score
   - Suggest: Extract more pure functions, reduce nesting

4. **Coverage Not Improved**:
   - Test coverage still low or unchanged
   - Example: 0% → 0% (no improvement)
   - Suggest: Add comprehensive tests for all branches

**B. Regression Gaps** (NEW - Critical!):

5. **New Critical Debt Items Introduced**:
   - List all new debt items with score >= 60.0
   - Show location, score, and debt type
   - Example: "Refactoring created 3 new complex helper functions"
   - Suggest: Simplify extracted functions, ensure they're pure

6. **Overall Project Debt Increased**:
   - Total debt score went UP instead of down
   - Example: 5,234 → 5,456 (+4.2%)
   - Suggest: Review refactoring approach - may be shifting rather than reducing debt

**Gap structure including regressions:**
```json
{
  "target_item_not_improved": {
    "description": "Target debt item still has high score",
    "location": "./src/file.rs:function:line",
    "severity": "high",
    "before_score": 81.9,
    "after_score": 75.0,
    "score_reduction_pct": 8.4,
    "suggested_fix": "Need more aggressive refactoring - extract pure functions"
  },
  "new_critical_debt_0": {
    "description": "New high-complexity helper function introduced",
    "location": "./src/file.rs:process_with_cross_module:156",
    "severity": "critical",
    "score": 65.3,
    "debt_type": "Complexity",
    "suggested_fix": "Simplify this extracted function - break into smaller pieces"
  },
  "project_debt_increased": {
    "description": "Overall project technical debt increased",
    "severity": "high",
    "before_total": 5234.2,
    "after_total": 5456.8,
    "change_pct": 4.2,
    "suggested_fix": "Review refactoring approach - may be shifting debt instead of reducing it"
  }
}
```

### Step 6: Write Validation Results

**CRITICAL**: Write validation results to the output file:

1. **Use output location from `--output` parameter**:
   - This should have been parsed from $ARGUMENTS
   - If not provided, use default `.prodigy/debtmap-validation.json`

2. **Write JSON to file**:
   - Create parent directories if needed
   - Write the JSON validation result to the file
   - Ensure file is properly closed and flushed

3. **Do NOT output JSON to stdout** - Prodigy will read from the file

The JSON format (ENHANCED - includes regression detection):

**Successful validation (target improved, no regressions):**
```json
{
  "completion_percentage": 82.0,
  "status": "complete",
  "target_item": {
    "location": "./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120",
    "before_score": 81.9,
    "after_score": 14.5,
    "score_reduction_pct": 82.3,
    "complexity_reduction_pct": 64.7,
    "coverage_improvement_pct": 50.0
  },
  "project_health": {
    "total_debt_before": 5234.2,
    "total_debt_after": 4987.6,
    "improvement_pct": 4.7,
    "new_critical_items": 0,
    "regression_penalty": 0
  },
  "improvements": [
    "Target item score reduced from 81.9 to 14.5 (82.3% reduction)",
    "Cyclomatic complexity reduced from 17 to 6 (64.7% reduction)",
    "Test coverage improved from 0% to 50%",
    "Overall project debt reduced by 4.7%",
    "No new critical debt items introduced"
  ],
  "remaining_issues": [],
  "gaps": {}
}
```

**Failed validation (regressions detected):**
```json
{
  "completion_percentage": 45.0,
  "status": "incomplete",
  "target_item": {
    "location": "./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120",
    "before_score": 81.9,
    "after_score": 15.2,
    "score_reduction_pct": 81.4
  },
  "project_health": {
    "total_debt_before": 5234.2,
    "total_debt_after": 5456.8,
    "improvement_pct": -4.2,
    "new_critical_items": 3,
    "regression_penalty": 60
  },
  "improvements": [
    "Target item score reduced from 81.9 to 15.2 (81.4% reduction)"
  ],
  "remaining_issues": [
    "3 new critical debt items introduced (regression!)",
    "Overall project debt increased by 4.2%",
    "Refactoring shifted complexity to helper functions"
  ],
  "gaps": {
    "new_critical_debt_0": {
      "description": "New high-complexity helper function",
      "location": "./src/builders/call_graph.rs:process_with_cross_module:156",
      "severity": "critical",
      "score": 65.3,
      "debt_type": "Complexity",
      "suggested_fix": "Simplify this extracted function"
    },
    "new_critical_debt_1": {
      "description": "New high-complexity helper function",
      "location": "./src/builders/call_graph.rs:handle_cross_module_fallback:189",
      "severity": "critical",
      "score": 58.7,
      "debt_type": "Complexity",
      "suggested_fix": "Break this into smaller pure functions"
    },
    "project_debt_increased": {
      "description": "Overall project debt increased",
      "severity": "high",
      "before_total": 5234.2,
      "after_total": 5456.8,
      "change_pct": 4.2,
      "suggested_fix": "Review refactoring - may be shifting debt instead of reducing it"
    }
  }
}
```

## Validation Rules

### Improvement Scoring

- **90-100%**: Excellent improvement - major debt resolved, no regression
- **75-89%**: Good improvement - significant progress on high-priority items
- **60-74%**: Moderate improvement - some progress but gaps remain
- **40-59%**: Minor improvement - mostly cosmetic changes
- **Below 40%**: Insufficient improvement or regression

### Priority Categories

1. **Critical (Score >= 8)**
   - Must be addressed for high completion percentage
   - Each unresolved critical item reduces score by 15-20%
   - New critical items reduce score by 25%

2. **High Priority (Score 6-8)**
   - Important for good completion percentage
   - Each unresolved item reduces score by 8-12%
   - Progress on these items counts significantly

3. **Medium Priority (Score 4-6)**
   - Nice to have improvements
   - Each unresolved item reduces score by 3-5%
   - Can compensate for other gaps

4. **Low Priority (Score < 4)**
   - Minimal impact on overall score
   - Useful for edge case improvements
   - Each unresolved item reduces score by 1-2%

## Automation Mode Behavior

**Automation Detection**: Checks for `PRODIGY_AUTOMATION=true` or `PRODIGY_VALIDATION=true` environment variables.

**In Automation Mode**:
- Skip interactive prompts
- Output minimal progress messages
- Always output JSON result at the end
- Exit with appropriate code (0 for complete, 1 for incomplete)

## Error Handling

The command will:
- Handle missing or malformed JSON files gracefully
- Work with partial debtmap outputs
- Provide clear error messages
- Always output valid JSON (even on errors)

## Example Validation Outputs

### Successful Validation (85%)
```json
{
  "completion_percentage": 85.0,
  "status": "complete",
  "improvements": [
    "Resolved 3 of 4 critical debt items",
    "Reduced project debt score from 6.2 to 4.8",
    "Added comprehensive test coverage to auth module"
  ],
  "remaining_issues": [
    "1 medium-priority complexity issue in parser.rs"
  ],
  "gaps": {}
}
```

### Incomplete Improvement (65%)
```json
{
  "completion_percentage": 65.0,
  "status": "incomplete",
  "improvements": [
    "Reduced complexity in 2 functions",
    "Added some test coverage"
  ],
  "remaining_issues": [
    "2 critical debt items unresolved",
    "New complexity introduced in util.rs"
  ],
  "gaps": {
    "critical_debt_unresolved": {
      "description": "High-priority authentication function still too complex",
      "location": "src/auth.rs:authenticate_user:45",
      "severity": "critical",
      "suggested_fix": "Extract pure functions for validation logic",
      "original_score": 9.2,
      "current_score": 9.2
    },
    "regression_detected": {
      "description": "New complexity introduced during refactoring",
      "location": "src/util.rs:process_data:78",
      "severity": "high",
      "suggested_fix": "Simplify the newly added conditional logic",
      "original_score": null,
      "current_score": 7.8
    }
  }
}
```

### Validation Failure
```json
{
  "completion_percentage": 0.0,
  "status": "failed",
  "improvements": [],
  "remaining_issues": ["Unable to compare: malformed debtmap JSON"],
  "gaps": {},
  "raw_output": "Error details here"
}
```

## Integration with Workflows

This command is designed to work with Prodigy workflows:

1. **Workflow captures before state**
2. **Workflow runs debtmap fix command**
3. **Workflow captures after state**
4. **This command validates improvement**
5. **If incomplete, workflow triggers completion logic**
6. **Process repeats up to max_attempts**

## Important Implementation Notes

**CRITICAL CHANGES** - This command now validates SINGLE items, not all items:

1. **Parse arguments correctly** - Extract before, after, **plan**, and output paths from $ARGUMENTS
2. **Read the plan file** to identify the target debt item (file, function, line)
3. **Filter before/after JSON** to only compare the single target item
4. **Calculate improvement** based on target item's score/complexity/coverage changes
5. **Write JSON to file**:
   - Use path from `--output` parameter, or default `.prodigy/debtmap-validation.json`
   - Create parent directories if they don't exist
   - Write complete JSON validation result to the file
6. **Always write valid JSON** to the file, even if validation fails
7. **Exit code 0** indicates command ran successfully (regardless of validation result)
8. **Improvement percentage** determines if validation passed based on threshold
9. **Gap details** should reference ONLY the target item, not other items
10. **Keep JSON compact** - Prodigy will parse it programmatically
11. **Do NOT output JSON to stdout** - only progress messages should go to stdout
12. **Focus on target item metrics** - its score, complexity, coverage only

**Implementation approach:**
- Use Python script or shell script with jq to parse JSON
- Match items by comparing `location.file`, `location.function`, `location.line`
- If target not found in after → 100% improvement (resolved!)
- If target still present → calculate percentage improvements in each metric
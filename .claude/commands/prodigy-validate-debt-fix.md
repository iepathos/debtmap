---
name: prodigy-validate-debt-fix
description: Validate that a debt item has been successfully fixed
---

# Validate Debt Fix

Validates that a specific tech debt item identified by debtmap has been successfully resolved.

## Parameters

- `--json`: Complete JSON object for the original debt item
- `--output`: Path to write validation results (required for workflow integration)

## Process

### Step 1: Parse the Original Debt Item

Extract key information from the provided JSON:
- File path: `item.location.file`
- Function name: `item.location.function`
- Line number: `item.location.line`
- Original metrics:
  - Cyclomatic complexity: `item.cyclomatic_complexity`
  - Cognitive complexity: `item.cognitive_complexity`
  - Nesting depth: `item.nesting_depth`
  - Function length: `item.function_length`
  - Coverage: `item.unified_score.coverage_factor`
  - Final score: `item.unified_score.final_score`
  - Recommended action: `item.recommendation.primary_action`

### Step 2: Re-analyze the Function

Run targeted analysis on the specific function to get current metrics:
```bash
# Extract the function and analyze its current state
# This could use debtmap's single-function analysis if available
# Or parse the file to extract current metrics
```

### Step 3: Compare Metrics

Compare original vs current metrics to determine if the debt was resolved:

#### For Refactoring Fixes
If the recommended action included refactoring:
- **Complexity Reduction**: Check if cyclomatic complexity decreased by at least 30%
- **Cognitive Complexity**: Should be reduced significantly
- **Nesting Depth**: Should be <= 3 (or reduced by at least 1 level)
- **Function Length**: Should be < 50 lines (or reduced by at least 30%)

#### For Test Coverage Fixes
If the recommended action was to add tests:
- **Coverage**: Check if coverage factor improved (ideally to 0, meaning full coverage)
- **Test Files**: Verify test files were added or modified
- **Test Count**: Ensure multiple test cases were added

#### For Combined Fixes
If both refactoring and tests were recommended:
- Apply both sets of criteria above
- At least one metric must show significant improvement

### Step 4: Calculate Validation Score

Calculate a percentage score based on improvements:

```
validation_score = weighted_average(
    complexity_improvement * 0.3,
    coverage_improvement * 0.3,
    length_improvement * 0.2,
    nesting_improvement * 0.2
)
```

Where each improvement is calculated as:
- `improvement = (original - current) / original * 100`
- Capped at 100% for each metric

### Step 5: Determine Gaps

If validation score < threshold (90%), identify specific gaps:

1. **Insufficient Refactoring**:
   - Function still too complex (cyclomatic > 10)
   - Deep nesting still present (> 3 levels)
   - Function still too long (> 50 lines)

2. **Missing Tests**:
   - No test file found for the module
   - Coverage still below acceptable threshold
   - Critical branches not covered

3. **Partial Fix**:
   - Some improvement but not enough
   - Wrong type of fix applied (e.g., only formatting changes)

### Step 6: Write Validation Results

Write results to the specified output file in JSON format:

```json
{
  "validated": true/false,
  "score": 85.5,
  "original_metrics": {
    "cyclomatic_complexity": 15,
    "cognitive_complexity": 20,
    "coverage_factor": 10,
    "function_length": 75,
    "nesting_depth": 4
  },
  "current_metrics": {
    "cyclomatic_complexity": 8,
    "cognitive_complexity": 10,
    "coverage_factor": 0,
    "function_length": 45,
    "nesting_depth": 2
  },
  "improvements": {
    "complexity_reduced": 46.7,
    "coverage_improved": 100,
    "length_reduced": 40,
    "nesting_reduced": 50
  },
  "gaps": [
    "Cyclomatic complexity still above ideal threshold of 5",
    "Consider extracting additional helper functions"
  ],
  "recommendation": "Fix is 85% complete. Minor improvements still possible."
}
```

## Validation Thresholds

The validation considers a fix successful if:

1. **For Critical Issues (Score >= 9)**:
   - Validation score >= 90%
   - Complexity reduced by at least 40%
   - Tests added if coverage was zero

2. **For High Priority (Score 7-9)**:
   - Validation score >= 85%
   - Significant improvement in primary metric
   - Some tests added

3. **For Medium Priority (Score 4-6)**:
   - Validation score >= 80%
   - Measurable improvement in at least one metric

4. **For Low Priority (Score < 4)**:
   - Validation score >= 75%
   - Any improvement counts

## Success Criteria

The validation passes when:
- [ ] Current metrics show significant improvement
- [ ] Validation score meets the threshold (default 90%)
- [ ] Primary recommended action was addressed
- [ ] No regression in other metrics
- [ ] Results written to output file

## Notes

- This command is designed for the debtmap-reduce workflow
- It provides feedback for the fix-debt-item command
- Gaps identified can be used by prodigy-complete-debt-fix
- Validation is more lenient than spec validation (90% vs 100%)
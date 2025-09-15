# Validate Debtmap Improvement Command

Validates that technical debt improvements have been made by comparing debtmap JSON output before and after changes.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-validate-debtmap-improvement --before <before-json-file> --after <after-json-file> [--output <filepath>]
```

Examples:
- `/prodigy-validate-debtmap-improvement --before .prodigy/debtmap-before.json --after .prodigy/debtmap-after.json --output .prodigy/debtmap-validation.json`

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

### Step 1: Parse Arguments and Load Data

The command will:
- Parse $ARGUMENTS to extract:
  - `--before` parameter with path to pre-fix debtmap JSON
  - `--after` parameter with path to post-fix debtmap JSON
  - `--output` parameter with filepath (required when called from workflow)
- If missing parameters, fail with error message
- If no `--output` parameter, default to `.prodigy/debtmap-validation.json`
- Load both JSON files and validate they contain debtmap output

### Step 2: Compare Overall Metrics

Compare high-level improvements:
- **Total Debt Items**: Before vs after count
- **High Priority Items**: Critical debt items (score >= 8)
- **Average Complexity**: Overall complexity metrics
- **Coverage Gaps**: Functions with poor test coverage
- **Technical Debt Score**: Overall project health

### Step 3: Identify Specific Improvements

Track improvements in individual debt items:
- **Resolved Items**: Debt items that no longer appear
- **Improved Items**: Items with reduced complexity/better coverage
- **New Items**: Any new debt introduced (negative impact)
- **Unchanged Critical Items**: High-priority items still present

### Step 4: Calculate Improvement Score

Calculate improvement percentage based on:

```
improvement_score = weighted_average(
    resolved_high_priority * 0.4,    // Fixing critical items
    overall_score_improvement * 0.3,  // Project-wide improvement
    complexity_reduction * 0.2,       // Specific metrics improved
    no_new_critical_debt * 0.1       // No regression
)
```

Where:
- `resolved_high_priority` = percentage of high-priority items fixed
- `overall_score_improvement` = improvement in total debt score
- `complexity_reduction` = average complexity reduction across modified functions
- `no_new_critical_debt` = penalty if new critical issues introduced

### Step 5: Identify Improvement Gaps

If improvement score < threshold (75%), identify specific gaps:

1. **Insufficient Impact**:
   - High-priority debt items still present
   - Only cosmetic changes made (formatting, comments)
   - Wrong items addressed (low-priority vs high-priority)

2. **Incomplete Implementation**:
   - Partial refactoring that didn't reduce complexity enough
   - Tests added but coverage still insufficient
   - Function shortened but still too complex

3. **Regression Issues**:
   - New debt items introduced
   - Existing items made worse
   - Test coverage decreased

4. **Missing Core Work**:
   - Primary recommendation not addressed
   - Complex functions left untouched
   - Critical coverage gaps remain

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

The JSON format is:

```json
{
  "completion_percentage": 82.0,
  "status": "incomplete",
  "improvements": [
    "Resolved 2 high-priority debt items",
    "Reduced average cyclomatic complexity by 25%",
    "Added test coverage for 3 critical functions"
  ],
  "remaining_issues": [
    "1 critical debt item still present",
    "Complex function in auth.rs needs refactoring"
  ],
  "gaps": {
    "critical_debt_remaining": {
      "description": "High-priority debt item still present in user authentication",
      "location": "src/auth.rs:authenticate_user:45",
      "severity": "high",
      "suggested_fix": "Apply functional programming patterns to reduce complexity",
      "original_score": 9.2,
      "current_score": 9.2
    },
    "insufficient_refactoring": {
      "description": "Function complexity reduced but still above threshold",
      "location": "src/parser.rs:parse_config:123",
      "severity": "medium",
      "suggested_fix": "Extract helper functions using pure functional patterns",
      "original_complexity": 15,
      "current_complexity": 12,
      "target_complexity": 8
    }
  },
  "before_summary": {
    "total_items": 45,
    "high_priority_items": 8,
    "average_score": 6.2
  },
  "after_summary": {
    "total_items": 41,
    "high_priority_items": 6,
    "average_score": 5.8
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

1. **Parse arguments correctly** - Extract before, after, and output paths from $ARGUMENTS
2. **Write JSON to file**:
   - Use path from `--output` parameter, or default `.prodigy/debtmap-validation.json`
   - Create parent directories if they don't exist
   - Write complete JSON validation result to the file
3. **Always write valid JSON** to the file, even if validation fails
4. **Exit code 0** indicates command ran successfully (regardless of validation result)
5. **Improvement percentage** determines if validation passed based on threshold
6. **Gap details** help subsequent commands fix remaining issues
7. **Keep JSON compact** - Prodigy will parse it programmatically
8. **Do NOT output JSON to stdout** - only progress messages should go to stdout
9. **Focus on technical debt metrics** - complexity, coverage, function length, nesting
10. **Prioritize high-impact improvements** - critical debt items matter most
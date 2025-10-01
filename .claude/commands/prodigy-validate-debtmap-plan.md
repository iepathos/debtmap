---
name: validate-debtmap-plan
description: Validate that implementation plan fully addresses the tech debt item
arguments:
  - name: before
    description: Path to debtmap analysis before planning
    type: string
    required: true
  - name: plan
    description: Path to the implementation plan
    type: string
    required: true
  - name: output
    description: Path to write validation results
    type: string
    required: true
---

# Validate Implementation Plan

Verify that the implementation plan created by `/prodigy-debtmap-plan` will actually resolve the technical debt item identified in the debtmap analysis.

## Validation Process

### Step 1: Load Inputs

```bash
# Load the debtmap analysis
cat $ARG_before

# Load the implementation plan
cat $ARG_plan
```

### Step 2: Extract Target Debt Item

From the debtmap analysis, extract the top priority item:

```bash
jq '.items[0] | {
  score: .unified_score.final_score,
  location: .location,
  debt_type: .debt_type,
  action: .recommendation.primary_action,
  rationale: .recommendation.rationale,
  expected_impact: .expected_impact,
  complexity: .cyclomatic_complexity,
  coverage: .transitive_coverage
}' $ARG_before
```

### Step 3: Validate Plan Coverage

Check that the plan addresses all aspects of the debt item:

#### 3.1: Location Match
- [ ] Plan targets the correct file and function
- [ ] Plan identifies the same issue as debtmap

#### 3.2: Action Alignment
Based on the debt type, verify appropriate approach:

**For GOD OBJECT / High Complexity:**
- [ ] Plan extracts functions into modules (not just moving code)
- [ ] Plan separates concerns (I/O from logic, pure from impure)
- [ ] Plan uses functional programming patterns
- [ ] Plan breaks work into 3-5 incremental phases
- [ ] Each phase reduces actual complexity (not just relocates it)

**For LOW COVERAGE:**
- [ ] Plan extracts pure logic from I/O code (if applicable)
- [ ] Plan adds comprehensive test cases
- [ ] Plan covers edge cases and error conditions
- [ ] Plan includes specific test scenarios

**For CODE DUPLICATION:**
- [ ] Plan identifies all instances of duplication
- [ ] Plan extracts to shared module/function
- [ ] Plan maintains backward compatibility

#### 3.3: Success Criteria Validation

Verify the plan has:
- [ ] Specific, measurable success criteria
- [ ] Expected metrics improvement (complexity, coverage, lines)
- [ ] Test strategy for each phase
- [ ] Rollback plan

#### 3.4: Phase Structure Validation

Check that phases are:
- [ ] Independently valuable (can commit after each)
- [ ] Properly ordered (no phase depends on later phases)
- [ ] Realistic in scope (10-20 functions per phase)
- [ ] Testable (clear verification steps)

#### 3.5: Anti-Pattern Detection

Ensure the plan does NOT:
- [ ] Create helper methods only used in tests
- [ ] Break up legitimate patterns (match/visitor)
- [ ] Add complexity to reduce complexity
- [ ] Skip testing between phases
- [ ] Try to fix everything at once

### Step 4: Calculate Validation Score

Score the plan on these dimensions (0-100 each):

**Coverage Score (0-100):**
- Does plan address the root cause? (40 points)
- Does plan target correct location? (20 points)
- Does plan align with recommended action? (20 points)
- Does plan use appropriate patterns? (20 points)

**Structure Score (0-100):**
- Are phases incremental? (25 points)
- Are phases independently valuable? (25 points)
- Are success criteria clear? (25 points)
- Is testing strategy defined? (25 points)

**Feasibility Score (0-100):**
- Is scope realistic? (33 points)
- Are phases properly ordered? (33 points)
- Is rollback plan included? (34 points)

**Quality Score (0-100):**
- Follows functional programming? (25 points)
- Avoids anti-patterns? (25 points)
- Includes specific metrics? (25 points)
- Has measurable criteria? (25 points)

**Overall Score:**
```
overall_score = (coverage + structure + feasibility + quality) / 4
```

### Step 5: Identify Gaps

For any dimension scoring < 75, identify specific gaps:

```json
{
  "gaps": [
    {
      "dimension": "coverage",
      "score": 60,
      "issue": "Plan doesn't extract I/O from business logic",
      "recommendation": "Add a phase to separate pure functions from I/O operations"
    }
  ]
}
```

### Step 6: Generate Validation Result

Write validation results to `$ARG_output`:

```json
{
  "validation_type": "plan",
  "timestamp": "<ISO8601 timestamp>",
  "target": {
    "file": "<from debtmap>",
    "function": "<from debtmap>",
    "score": <debt score>,
    "debt_type": "<from debtmap>"
  },
  "scores": {
    "coverage": <0-100>,
    "structure": <0-100>,
    "feasibility": <0-100>,
    "quality": <0-100>,
    "overall": <0-100>
  },
  "validation": {
    "passed": <overall >= 75>,
    "threshold": 75,
    "message": "<summary of validation>"
  },
  "gaps": [
    {
      "dimension": "<coverage|structure|feasibility|quality>",
      "score": <0-100>,
      "issue": "<what's wrong>",
      "recommendation": "<how to fix>"
    }
  ],
  "recommendations": [
    "<Specific improvement 1>",
    "<Specific improvement 2>"
  ]
}
```

### Step 7: Output Summary

Print a human-readable summary:

```
Plan Validation Results
======================

Target: <file>:<function> (Score: <debt_score>)
Debt Type: <type>

Validation Scores:
  Coverage:    <score>/100
  Structure:   <score>/100
  Feasibility: <score>/100
  Quality:     <score>/100

Overall: <overall>/100 (Threshold: 75)

Status: <PASS|FAIL>

<If PASS>
✓ Plan is ready for implementation

<If FAIL>
✗ Plan needs improvements:
  - <gap 1>
  - <gap 2>

Recommendations:
  - <recommendation 1>
  - <recommendation 2>
```

## Validation Criteria

### Coverage (Root Cause)

**Good Plan:**
- Identifies the actual problem (not just symptoms)
- Addresses root cause directly
- Targets correct file/function/lines
- Aligns with debtmap recommendation

**Bad Plan:**
- Fixes symptoms instead of root cause
- Targets wrong code location
- Ignores debtmap recommendation
- Adds tests without fixing underlying issues

### Structure (Phases)

**Good Plan:**
- 3-5 phases, each independently valuable
- Clear progression (phase N builds on N-1)
- Each phase is testable and committable
- Success criteria are specific

**Bad Plan:**
- Too many phases (>5) or too few (<3 for complex items)
- Phases depend on future phases
- Unclear success criteria
- Can't commit after each phase

### Feasibility (Realistic)

**Good Plan:**
- Scope is achievable
- Phases are properly ordered
- Includes rollback strategy
- Accounts for testing time

**Bad Plan:**
- Trying to fix too much at once
- Unrealistic timelines
- No contingency planning
- Missing testing strategy

### Quality (Approach)

**Good Plan:**
- Uses functional programming patterns
- Separates I/O from business logic
- Extracts pure functions
- Avoids helper method anti-patterns

**Bad Plan:**
- Creates test-only helpers
- Adds complexity to reduce complexity
- Breaks up clear patterns
- Ignores functional principles

## Gap Analysis

For each dimension scoring < 75, provide:

1. **What's wrong**: Specific issue with the plan
2. **Why it matters**: Impact on success
3. **How to fix**: Concrete recommendation

Example gaps:

```json
{
  "dimension": "coverage",
  "score": 60,
  "issue": "Plan creates 5 helper methods that are only called from tests",
  "recommendation": "Extract reusable pure functions that are used in production code, not test-only helpers"
}
```

```json
{
  "dimension": "structure",
  "score": 50,
  "issue": "Phase 3 requires completing Phase 4 first (circular dependency)",
  "recommendation": "Reorder phases so Phase 3 comes after Phase 4, or merge them into a single phase"
}
```

## Success Criteria

The plan passes validation if:
- [ ] Overall score >= 75
- [ ] All dimensions >= 50 (no catastrophic failures)
- [ ] Target location matches debtmap
- [ ] Approach aligns with debt type
- [ ] No critical anti-patterns detected

## Output Format

**REQUIRED**: Write validation results to `$ARG_output` in JSON format as specified above.

The workflow will use this to determine if the plan can proceed to implementation or needs revision.

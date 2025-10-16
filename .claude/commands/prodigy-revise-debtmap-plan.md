---
name: revise-debtmap-plan
description: Revise implementation plan based on validation gaps
arguments:
  - name: gaps
    description: Validation gaps from plan validation
    type: string
    required: true
  - name: plan
    description: Path to the implementation plan to revise
    type: string
    default: .prodigy/IMPLEMENTATION_PLAN.md
---

# Revise Implementation Plan

Update the implementation plan to address validation gaps identified by `/prodigy-validate-debtmap-plan`.

## Process

**CRITICAL**: This command revises the plan file but **NEVER DELETES IT**. The workflow requires IMPLEMENTATION_PLAN.md throughout execution.

### Step 1: Load Current Plan and Gaps

```bash
# Load the current plan
cat $ARG_plan

# Parse validation gaps
echo "$ARG_gaps" | jq '.'
```

### Step 2: Analyze Gaps

For each gap, understand:
- Which dimension failed (coverage, structure, feasibility, quality)
- What the specific issue is
- What the recommendation suggests

### Step 3: Revise Plan

Based on the gaps, revise the plan:

**For Coverage Gaps** (plan doesn't address root cause):
- Re-read the debtmap analysis to understand the real problem
- Adjust phases to target the actual issue
- Ensure plan addresses the recommended action
- Verify correct file/function is targeted

**For Structure Gaps** (phases not incremental):
- Reorganize phases to be independently valuable
- Ensure proper ordering (no circular dependencies)
- Make success criteria more specific
- Clarify testing strategy for each phase

**For Feasibility Gaps** (scope unrealistic):
- Break large phases into smaller ones
- Adjust phase scope to be more achievable
- Add more detail to rollback plan
- Include time estimates for testing

**For Quality Gaps** (approach issues):
- Replace helper methods with pure function extraction
- Add I/O separation to appropriate phases
- Include functional programming patterns
- Remove anti-patterns from the approach

### Step 4: Update the Plan File

Rewrite the plan at `$ARG_plan` with improvements:

```bash
cat > $ARG_plan << 'EOF'
# Implementation Plan: <Updated Title>

## Problem Summary
<Updated to match debtmap analysis>

## Target State
<Updated with specific metrics>

## Implementation Phases

### Phase 1: <Revised based on gaps>
...

EOF
```

### Step 5: Validate Improvements

Verify the revised plan addresses all gaps:

- [ ] Coverage gap resolved (targets root cause)
- [ ] Structure gap resolved (phases incremental)
- [ ] Feasibility gap resolved (realistic scope)
- [ ] Quality gap resolved (proper approach)

### Step 6: Output Summary

```
Plan Revised
===========

Addressed gaps:
  - <gap 1>
  - <gap 2>

Changes made:
  - <change 1>
  - <change 2>

Ready for re-validation
```

## Revision Guidelines

### Addressing Coverage Gaps

If the plan doesn't target the root cause:

1. **Re-read the debtmap analysis** carefully
2. **Understand the debt_type**:
   - GOD_OBJECT → Extract modules, separate concerns
   - LOW_COVERAGE → Extract pure logic, add tests
   - HIGH_COMPLEXITY → Reduce actual complexity, not just move code
3. **Align with primary_action** from debtmap
4. **Target the correct location** (file, function, lines)

### Addressing Structure Gaps

If phases aren't incremental:

1. **Check dependencies** - ensure phase N doesn't need phase N+1
2. **Make each phase valuable** - can commit and merge after each
3. **Add specific criteria** - "All tests pass" → "15 new unit tests covering edge cases X, Y, Z"
4. **Include testing steps** - exactly what to run and verify

### Addressing Feasibility Gaps

If scope is unrealistic:

1. **Split large phases** - if >20 functions, break into sub-phases
2. **Add contingency** - "If tests fail, revert and adjust"
3. **Include estimates** - "Phase 1: ~2 hours" helps set expectations
4. **Plan for unknowns** - "May need additional extraction based on test results"

### Addressing Quality Gaps

If approach has issues:

1. **Remove test-only helpers** from the plan
2. **Add pure function extraction** where appropriate
3. **Include I/O separation** for orchestration code
4. **Use functional patterns** - map/filter/fold instead of loops
5. **Avoid anti-patterns** - no helper explosion, no breaking clean patterns

## Common Revision Patterns

### Pattern 1: Helper Method Explosion

**Gap**: "Plan creates 5 helper methods only called from tests"

**Revision**:
- Remove helper methods from plan
- Add phase to extract reusable pure functions
- Ensure extracted functions are used in production code
- Test the pure functions, not test-only helpers

### Pattern 2: I/O Not Separated

**Gap**: "Plan doesn't separate I/O from business logic"

**Revision**:
- Add Phase 1: "Extract pure business logic functions"
- Keep I/O in thin wrappers
- Test the pure functions
- Don't test the I/O wrappers directly

### Pattern 3: Circular Dependencies

**Gap**: "Phase 3 requires completing Phase 4 first"

**Revision**:
- Reorder phases: 4 → 3
- Or merge phases 3 and 4
- Ensure linear progression

### Pattern 4: Missing Root Cause

**Gap**: "Plan adds tests but doesn't fix underlying complexity"

**Revision**:
- Add refactoring phases BEFORE test phases
- Extract functions to reduce complexity
- Then add tests to the extracted functions
- Focus on root cause first

## Output

After revising the plan, the workflow will automatically re-run validation to verify improvements.

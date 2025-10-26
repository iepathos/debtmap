---
name: debtmap-plan
description: Analyze tech debt and create a phased implementation plan
arguments:
  - name: before
    description: Path to debtmap analysis file
    type: string
    required: true
  - name: output
    description: Path to write the implementation plan
    type: string
    default: .prodigy/IMPLEMENTATION_PLAN.md
---

# Create Implementation Plan for Top Tech Debt Item

Analyze the debtmap output and create a detailed, phased implementation plan for fixing **ONLY** the **#1 highest priority** technical debt item.

**CRITICAL**: This workflow fixes ONE item at a time. Do NOT try to fix multiple debt items. Focus exclusively on the top-ranked item from the debtmap analysis.

## Scope

**What to fix**: `.items[0]` - The #1 top priority item ONLY
**What NOT to fix**: `.items[1]`, `.items[2]`, or any other items
**Approach**: Focused, targeted fix for ONE specific debt item

If you encounter other debt items while implementing, **IGNORE THEM**. They will be addressed in future workflow runs.

## Process

### Step 1: Load Debtmap Analysis

Read the debtmap analysis passed as an argument:

```bash
cat $ARG_before
```

**Extract ONLY the #1 top priority item** (`.items[0]`):

```bash
jq '.items[0] | {
  score: .unified_score.final_score,
  location: .location,
  debt_type: .debt_type,
  action: .recommendation.primary_action,
  rationale: .recommendation.rationale,
  implementation_steps: .recommendation.implementation_steps,
  expected_impact: .expected_impact,
  coverage_info: .transitive_coverage,
  complexity: .cyclomatic_complexity
}' $ARG_before
```

### Step 2: Verify You're Targeting ONLY the Top Item

**IMPORTANT**: Before proceeding, confirm:
- You are looking at `.items[0]` (the first/top item ONLY)
- You are NOT looking at `.items[1]`, `.items[2]`, etc.
- Your plan will address THIS ONE ITEM and nothing else

Output the target for confirmation:
```
Target: <file>:<function_name>:<line>
Score: <unified_score>
Debt Type: <debt_type>
Action: <primary_action>
```

**CRITICAL**: The location must be in the format `file:function:line` (from the `.location` field in debtmap JSON). This exact format is required for later workflow steps.

### Step 3: Analyze the Problem

Based on the debt type of **THIS ONE ITEM**, understand what needs to be fixed:

**For GOD OBJECT / High Complexity:**
- Identify the distinct responsibilities in the code
- Look for patterns that can be extracted
- Consider functional programming refactoring approaches
- Don't just move code - reduce actual complexity

**For LOW COVERAGE:**
- Determine if it's orchestration/I/O code (should extract pure logic)
- Or business logic (needs direct testing)
- Identify edge cases and scenarios to test

**For CODE DUPLICATION:**
- Find all instances of the duplicated code
- Determine the best abstraction pattern
- Plan extraction to a shared module

### Step 4: Read the Target Code

Read **ONLY** the file(s) identified in `.items[0]` to understand the current implementation:

```bash
# Read the target file
cat <file_path_from_debtmap>
```

Analyze:
- Current structure and responsibilities
- Dependencies and coupling
- Complexity sources
- Testing gaps

### Step 5: Create the Implementation Plan

**CRITICAL REMINDER**: Your plan must address **ONLY the #1 top priority item** from `.items[0]`. Do NOT include fixes for other items.

Create a file at `$ARG_output` with this structure:

```markdown
# Implementation Plan: <Brief Description>

## Problem Summary

**Location**: <location from debtmap - format depends on debt level:
  - For function-level debt: ./file.rs:function:line (e.g., ./src/main.rs:process_data:42)
  - For file-level debt: ./file.rs:file:0 (e.g., ./src/lib.rs:file:0)
  IMPORTANT: Use exactly :file:0 for file-level debt (not :file:1), with NO additional text after the line number>
**Priority Score**: <unified_score from debtmap>
**Debt Type**: <debt_type from debtmap>
**Current Metrics**:
- Lines of Code: <from debtmap>
- Functions: <from debtmap>
- Cyclomatic Complexity: <from debtmap>
- Coverage: <from debtmap>

**Issue**: <primary_action and rationale from debtmap>

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: <expected_impact.complexity_reduction>
- Coverage Improvement: <expected_impact.coverage_improvement>
- Risk Reduction: <expected_impact.risk_reduction>

**Success Criteria**:
- [ ] <Specific, measurable criteria>
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting

## Implementation Phases

Break the work into 3-5 incremental phases. Each phase should:
- Be independently testable
- Result in working, committed code
- Build on the previous phase

### Phase 1: <Name>

**Goal**: <What this phase accomplishes>

**Changes**:
- <Specific change 1>
- <Specific change 2>

**Testing**:
- <How to verify this phase works>

**Success Criteria**:
- [ ] <Specific criteria for this phase>
- [ ] All tests pass
- [ ] Ready to commit

### Phase 2: <Name>

**Goal**: <What this phase accomplishes>

**Changes**:
- <Specific change 1>
- <Specific change 2>

**Testing**:
- <How to verify this phase works>

**Success Criteria**:
- [ ] <Specific criteria for this phase>
- [ ] All tests pass
- [ ] Ready to commit

### Phase 3: <Name>

[Continue for each phase...]

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib` to verify existing tests pass
2. Run `cargo clippy` to check for warnings
3. [Any phase-specific testing]

**Final verification**:
1. `just ci` - Full CI checks
2. `cargo tarpaulin` - Regenerate coverage
3. `debtmap analyze` - Verify improvement

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the failure
3. Adjust the plan
4. Retry

## Notes

<Any additional context, gotchas, or considerations>
```

### Step 5: Validate the Plan

Before writing the plan, ensure:

1. **Each phase is independently valuable**
   - Can commit after each phase
   - Tests pass after each phase
   - Code works after each phase

2. **Phases are ordered correctly**
   - Earlier phases don't depend on later ones
   - Build complexity gradually
   - Test coverage increases progressively

3. **Plan is realistic**
   - Not trying to fix everything at once
   - Focused on the specific debt item
   - Achievable in 3-5 phases

4. **Success criteria are measurable**
   - Can objectively verify completion
   - Aligned with debtmap metrics
   - Include test coverage targets

### Step 6: Write the Plan

**CRITICAL**: This file will be used throughout the workflow. Do NOT delete it after creation or at any point during the workflow execution.

Write the complete implementation plan to `$ARG_output`:

```bash
# Example of writing the plan
cat > $ARG_output << 'EOF'
# Implementation Plan: Extract Pure Functions from WorkflowExecutor

## Problem Summary
[...]
EOF
```

### Step 7: Output Summary

After creating the plan, output a brief summary:

```
Created implementation plan at $ARG_output

Target: <file:line>
Priority: <score>
Phases: <number>

Next step: Run `/prodigy-debtmap-implement` to execute the plan
```

## Important Guidelines

### For God Object Refactoring:

**DO:**
- Extract pure functions that can be unit tested
- Separate I/O from business logic
- Create focused modules with single responsibilities
- Use functional programming patterns
- Keep changes incremental (10-20 functions per phase)

**DON'T:**
- Try to refactor everything at once
- Create helper methods only used in tests
- Break up legitimate patterns (match/visitor)
- Add complexity to reduce complexity
- Skip testing between phases

### For Coverage Improvements:

**DO:**
- Extract pure logic from I/O code first
- Test the extracted pure functions
- Cover edge cases and error conditions
- Follow existing test patterns

**DON'T:**
- Force tests on orchestration code
- Test implementation details
- Add tests without understanding the code
- Ignore test failures

### Plan Quality Checklist:

- [ ] Problem clearly identified from debtmap output
- [ ] Target state is specific and measurable
- [ ] 3-5 phases, each independently valuable
- [ ] Each phase has clear success criteria
- [ ] Testing strategy defined
- [ ] Rollback plan included
- [ ] Plan is realistic and achievable

## Output Format

The plan MUST be written to `$ARG_output` in the format specified above.

The workflow will pass this plan to the implementation command.

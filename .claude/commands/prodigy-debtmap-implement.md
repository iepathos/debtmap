---
name: debtmap-implement
description: Execute the phased implementation plan for tech debt fix
arguments:
  - name: plan
    description: Path to the implementation plan file
    type: string
    default: .prodigy/IMPLEMENTATION_PLAN.md
---

# Execute Implementation Plan

Follow the implementation plan created by `/prodigy-debtmap-plan` to fix **ONLY the ONE** technical debt item identified in the plan.

**CRITICAL**: This command implements the plan for ONE specific debt item. Do NOT attempt to fix other debt items while executing this plan. Stay focused on the target specified in the plan.

## Process

**CRITICAL**: **NEVER DELETE IMPLEMENTATION_PLAN.md** - The workflow requires this file for validation even after all phases are complete. Do not remove it under any circumstances.

### Step 1: Load the Plan

Read the implementation plan:

```bash
cat $ARG_plan
```

Extract key information:
- **Target debt item** (file, function, location)
- Problem location
- Number of phases
- Success criteria for each phase
- Testing strategy

**VERIFY**: Confirm the plan targets ONE specific item and you understand exactly what you're fixing.

### Step 2: Execute Phase by Phase

For each phase in the plan:

#### 2.1: Read Phase Details

Understand what needs to be done:
- Goal of this phase
- Specific changes required
- Testing approach
- Success criteria

#### 2.2: Implement Changes

Make the changes specified in the phase:

**For Extraction/Refactoring:**
1. Read the target file(s)
2. Identify the code to extract
3. Create new module/function if needed
4. Move/refactor the code
5. Update imports and references
6. Ensure all existing tests still compile

**For Adding Tests:**
1. Identify test scenarios from the plan
2. Write test cases following existing patterns
3. Ensure tests cover edge cases
4. Run tests to verify they pass

#### 2.3: Verify Phase Completion

After implementing changes for the phase:

```bash
# Run tests
cargo test --lib

# Check for warnings
cargo clippy -- -D warnings

# Verify formatting
cargo fmt --check
```

All checks must pass before proceeding.

#### 2.4: Commit the Phase

Create a commit for this phase:

```bash
git add -A
git commit -m "<phase-type>: <phase-name>

- <specific change 1>
- <specific change 2>

Phase <N> of <total>: <phase goal>
Tests: All passing
Status: <success criteria met>"
```

**Important**: Do NOT include attribution text like "🤖 Generated with Claude Code" or "Co-Authored-By: Claude" in commit messages.

#### 2.5: Update Plan Status

Mark the phase as complete in the plan:

```bash
# Update the phase status to show completion
sed -i '' 's/- \[ \] Phase N:/- [x] Phase N:/' $ARG_plan
```

### Step 3: Verify Final State

After all phases are complete:

#### 3.1: Run Full Test Suite

```bash
just test
```

Ensure all tests pass.

#### 3.2: Check Code Quality

```bash
just lint
just fmt-check
```

No warnings or formatting issues allowed.

#### 3.3: Regenerate Coverage (if tests were added)

```bash
cargo tarpaulin --config .tarpaulin.toml --out Lcov --output-dir target/coverage --timeout 120
```

This updates the coverage data for validation.

### Step 4: Summary

Output a summary of what was accomplished:

```
Implementation Complete!

Phases executed: <N>
Commits created: <N>
Tests: All passing
Linting: Clean
Formatting: Clean

The workflow validation will verify the debt improvement.
```

## Phase Execution Guidelines

### General Principles

1. **Fix ONLY the target item**: Do not fix other debt items you encounter
2. **Work incrementally**: Complete one phase fully before starting the next
3. **Test frequently**: Run tests after each significant change
4. **Commit working code**: Only commit when tests pass
5. **Follow the plan**: Stick to the phases as designed
6. **Stop if blocked**: Don't proceed if a phase fails
7. **Stay focused**: If you see other issues, ignore them - we fix ONE item at a time

### Error Handling

If a phase fails:

1. **Analyze the failure**
   - Read error messages carefully
   - Check test output
   - Review clippy warnings

2. **Attempt to fix**
   - Make targeted fixes
   - Re-run tests
   - If fixed, continue

3. **If still failing**
   - Document the issue
   - Output the error details
   - The `on_incomplete` handler in the workflow will be triggered

### Code Quality Standards

**Every change must:**
- Pass all existing tests
- Pass clippy with `-D warnings`
- Be properly formatted
- Follow functional programming principles
- Maintain or improve coverage
- Not introduce new technical debt

### Functional Programming Patterns

When refactoring, prefer:
- Pure functions over stateful methods
- Function composition over deep nesting
- Iterator chains over imperative loops
- Pattern matching over if-else chains
- Immutability by default
- Separation of I/O and business logic

## Testing Strategy

### For Extraction Phases

1. **Verify existing tests still pass**
   ```bash
   cargo test --lib <module>::tests
   ```

2. **Add tests for extracted functions**
   - Test the new pure functions
   - Cover edge cases
   - Test error conditions

### For Refactoring Phases

1. **Ensure behavior is unchanged**
   ```bash
   cargo test --lib
   ```

2. **Verify no performance regression**
   - Check that tests run in similar time
   - Monitor for significant slowdowns

## Commit Message Format

For each phase commit:

```
<type>: <description>

- <bullet point 1>
- <bullet point 2>

Phase <N>/<total>: <phase name>
<additional context>
```

Where `<type>` is one of:
- `refactor`: Code restructuring
- `test`: Adding tests
- `feat`: New functionality
- `fix`: Bug fixes
- `chore`: Maintenance

## Important Notes

- **NEVER DELETE IMPLEMENTATION_PLAN.md**: The workflow needs this file for validation
- **Do not skip phases**: Each phase builds on the previous one
- **Do not combine phases**: Keep commits focused on one phase
- **Do not modify the plan**: Follow it as designed (but never delete it)
- **Do commit frequently**: After each phase
- **Do test thoroughly**: Before each commit

## Troubleshooting

### Tests Fail After Changes

1. Check if the failure is related to your changes
2. Read the test output carefully
3. Fix the code to make tests pass
4. Don't commit until tests pass

### Clippy Warnings

1. Read the warning message
2. Apply the suggested fix
3. Re-run clippy
4. Don't commit with warnings

### Module Not Found Errors

1. Check import paths
2. Verify module is declared in parent
3. Ensure file is in correct location
4. Update `mod.rs` or `lib.rs` if needed

## Success Criteria

Before considering the implementation complete:

- [ ] All phases executed successfully
- [ ] All phases committed separately
- [ ] All tests passing (`just test`)
- [ ] No clippy warnings (`just lint`)
- [ ] Properly formatted (`just fmt-check`)
- [ ] Coverage regenerated (if tests added)
- [ ] Implementation matches the plan

## Output

After completing all phases, provide a summary of:
- Number of phases completed
- Number of commits created
- Current test status
- Any remaining work or notes

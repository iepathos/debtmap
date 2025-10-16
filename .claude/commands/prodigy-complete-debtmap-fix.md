# Complete Debtmap Fix Command

**PRIMARY OBJECTIVE: Complete the original implementation plan items** - NOT to fix new problems or refactor existing code.

This command is called when validation shows incomplete progress. Your job is to **finish what was started**, not start new improvements.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-complete-debtmap-fix --plan <plan-file> --validation <validation-file> --attempt <number>
```

Examples:
- `/prodigy-complete-debtmap-fix --plan .prodigy/IMPLEMENTATION_PLAN.md --validation .prodigy/debtmap-validation.json --attempt 2`

## What This Command Does

**CRITICAL UNDERSTANDING**: You are in a recovery situation. A previous implementation attempt was partially successful but didn't reach the 75% completion threshold. Your job is to:

1. **Complete Remaining Plan Items** (PRIMARY GOAL)
   - Read the IMPLEMENTATION_PLAN.md to see the original plan
   - Check validation to see what's been completed
   - Focus ONLY on completing the remaining planned items
   - DO NOT start new refactoring or improvements

2. **Avoid Making Things Worse** (CRITICAL)
   - If completion percentage is DECREASING across attempts → STOP REFACTORING
   - Regressions indicate you're solving the wrong problem
   - Return to the original plan and execute it as written

3. **Verify Completion**
   - Re-checks implementation after completing plan items
   - Ensures original objectives are met
   - Outputs completion status

## Execution Process

### Step 1: Parse Input and Understand Context

The command will:
- Extract `--plan` parameter: Path to IMPLEMENTATION_PLAN.md
- Extract `--validation` parameter: Path to validation JSON with completion status
- Extract `--attempt` parameter: Current attempt number (1-5)
- Read both files to understand:
  - What was the original plan?
  - What's been completed so far?
  - What remains to be done?
  - Are we progressing or regressing?

**CRITICAL CHECK**: If this is attempt 2+ and completion percentage decreased:
```
Example:
  Attempt 1: 72.3% complete
  Attempt 2: 51.2% complete  ← REGRESSION!

Action: STOP trying new approaches. Return to original plan and complete it exactly as written.
```

### Step 2: Identify Remaining Work vs Regressions

**Read the validation JSON** to distinguish:

1. **Remaining Plan Items** (PRIMARY FOCUS)
   ```json
   "remaining_plan_items": [
     "Extract output capture logic",
     "Extract validation processing",
     "Final cleanup and documentation"
   ]
   ```
   → **ACTION**: Complete these items from the original plan

2. **Completed Items** (DON'T UNDO)
   ```json
   "completed_items": [
     "Extracted step initialization logic",
     "Extracted command execution logic"
   ]
   ```
   → **ACTION**: Preserve these improvements, don't refactor them

3. **Regressions** (SECONDARY - only if blocking tests)
   ```json
   "regressions_to_fix": [
     {
       "location": "src/executor.rs:new_helper:123",
       "issue": "New complex function introduced"
     }
   ]
   ```
   → **ACTION**: Only fix if they cause test failures. Otherwise ignore them.

**Decision Framework**:
- Focus 90% effort on completing remaining plan items
- Focus 10% effort on regressions that block tests
- Focus 0% effort on "improvements" not in original plan

### Step 3: Apply Functional Programming Fixes

For each gap, apply targeted functional programming solutions:

#### Critical Complexity Issues
**Gap**: "High-priority debt item still present"
**Functional Programming Solution**:
- **Extract pure functions**: Separate I/O from business logic
- **Use pattern matching**: Replace complex if-else chains
- **Apply function composition**: Build complex behavior from simple functions
- **Implement early returns**: Reduce nesting with guard clauses
- **Use iterator chains**: Replace imperative loops

```rust
// Example: Transform complex imperative code
// Before: High cyclomatic complexity
fn complex_authentication(user: &User, request: &Request) -> Result<Token> {
    if user.is_active {
        if user.has_permission(&request.resource) {
            if request.is_valid() {
                if !user.is_locked() {
                    // Generate token logic...
                } else {
                    return Err("User locked");
                }
            } else {
                return Err("Invalid request");
            }
        } else {
            return Err("No permission");
        }
    } else {
        return Err("User inactive");
    }
}

// After: Functional decomposition with pure functions
fn is_user_eligible(user: &User) -> Result<(), &'static str> {
    match (user.is_active, user.is_locked()) {
        (false, _) => Err("User inactive"),
        (true, true) => Err("User locked"),
        (true, false) => Ok(()),
    }
}

fn has_access(user: &User, resource: &str) -> bool {
    user.has_permission(resource)
}

fn authenticate_user(user: &User, request: &Request) -> Result<Token> {
    is_user_eligible(user)?;

    ensure!(request.is_valid(), "Invalid request");
    ensure!(has_access(user, &request.resource), "No permission");

    generate_token(user, request)
}
```

#### Missing Test Coverage
**Gap**: "Critical branches not covered"
**Solution**:
- Add comprehensive test cases for pure functions
- Test error conditions and edge cases
- Use property-based testing for complex logic
- Mock external dependencies properly

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_eligibility_inactive() {
        let user = User { is_active: false, locked: false };
        assert_eq!(is_user_eligible(&user), Err("User inactive"));
    }

    #[test]
    fn test_user_eligibility_locked() {
        let user = User { is_active: true, locked: true };
        assert_eq!(is_user_eligible(&user), Err("User locked"));
    }

    #[test]
    fn test_user_eligibility_valid() {
        let user = User { is_active: true, locked: false };
        assert!(is_user_eligible(&user).is_ok());
    }
}
```

#### Deep Nesting Issues
**Gap**: "Function nesting still too deep"
**Functional Solution**:
- Use early returns with guard clauses
- Extract nested logic into pure helper functions
- Apply Option/Result combinators
- Use pattern matching to flatten conditions

```rust
// Before: Deep nesting
fn process_data(input: &Data) -> Result<Output> {
    if input.is_valid() {
        if let Some(processed) = input.preprocess() {
            if processed.meets_criteria() {
                if let Ok(result) = transform(processed) {
                    if result.is_complete() {
                        return Ok(result.finalize());
                    }
                }
            }
        }
    }
    Err("Processing failed")
}

// After: Functional pipeline with early returns
fn is_processable(input: &Data) -> Result<ProcessedData> {
    ensure!(input.is_valid(), "Invalid input");
    input.preprocess().ok_or("Preprocessing failed")
}

fn process_data(input: &Data) -> Result<Output> {
    let processed = is_processable(input)?;
    ensure!(processed.meets_criteria(), "Criteria not met");

    transform(processed)?
        .and_then(|result| {
            ensure!(result.is_complete(), "Incomplete result");
            Ok(result.finalize())
        })
}
```

#### Function Length Issues
**Gap**: "Function still too long"
**Functional Decomposition**:
- Extract logical sections into pure functions
- Separate validation from processing
- Use function composition for complex workflows
- Keep main function as orchestration only

### Step 3: Complete Remaining Plan Items

**CRITICAL**: **NEVER DELETE IMPLEMENTATION_PLAN.md** - The workflow requires this file for validation. Even if you complete all plan items, the workflow still needs this file. Do not remove it.

**Read IMPLEMENTATION_PLAN.md** and identify incomplete items:

Example plan structure:
```markdown
## Stage 1: Extract Step Initialization
Status: ✅ Complete

## Stage 2: Extract Command Execution
Status: ✅ Complete

## Stage 3: Extract Output Capture
Status: ❌ Not Started

## Stage 4: Extract Validation Processing
Status: ❌ Not Started

## Stage 5: Final Cleanup
Status: ❌ Not Started
```

**For each incomplete stage**:
1. Follow the approach described in the plan
2. Implement it as specified
3. Don't try to improve the approach
4. Mark stage as complete when done

### Step 4: Handle Multiple Attempts (Regression Prevention)

**Attempt 1**: Complete next 2-3 plan items
- Focus on plan items, not validation gaps
- Use conservative implementation
- Commit each completed item

**Attempt 2+**: Check for regression first
```python
if current_completion < previous_completion:
    # REGRESSION DETECTED
    # Stop trying new things
    # Review what got undone
    # Complete original plan items only
else:
    # Continue completing plan items
```

**NEVER**:
- Start new refactoring not in plan
- "Improve" existing completed work
- Chase validation gaps by adding abstractions

### Step 6: Verify No Regression

After applying fixes, ensure improvements don't introduce new issues:

```bash
# Run tests to ensure functionality preserved
just test

# Check formatting and linting
just fmt-check && just lint

# Verify no new compilation errors
just build-release
```

### Step 5: Commit Plan Item Completions

Create a clear commit documenting completion of plan items (NOT gap fixes):

```bash
git add -A
git commit -m "fix: complete implementation plan items [stage numbers]

Completed remaining implementation plan items:
- [Stage 3]: Extract output capture logic
- [Stage 4]: Extract validation processing

Plan progress: [N]/[Total] stages complete
Validation: [completion]% (threshold: 75%)

Following original implementation plan approach.
No new refactoring or abstractions introduced.
"
```

**Commit Message Focus**:
- What plan stages were completed
- NOT what gaps were fixed
- NOT what improvements were made
- Keep it about completing the original work

## Functional Programming Strategies by Gap Type

### For "Critical debt item still present"
1. **Extract pure business logic**: Separate core logic from side effects
2. **Use type-driven design**: Leverage Rust's type system for correctness
3. **Apply function composition**: Chain simple functions for complex behavior
4. **Implement immutable data flow**: Avoid mutation where possible
5. **Use pattern matching**: Replace complex branching logic

### For "Insufficient refactoring"
1. **Identify decision logic**: Extract boolean expressions into named predicates
2. **Create data transformation pipelines**: Use iterator chains over loops
3. **Separate concerns**: Different functions for different responsibilities
4. **Use Option/Result combinators**: Chain operations gracefully
5. **Apply the functional core, imperative shell pattern**

### For "Regression detected"
1. **Review the changes**: Identify what introduced new complexity
2. **Apply functional patterns to new code**: Ensure additions follow functional principles
3. **Extract shared logic**: If similar patterns exist, create reusable functions
4. **Use immutability**: Prevent unexpected mutations
5. **Add comprehensive tests**: Ensure new behavior is well-tested

### For "Missing test coverage"
1. **Test pure functions in isolation**: Easy to test, no mocking needed
2. **Use property-based testing**: Test invariants and relationships
3. **Test composition chains**: Verify pipelines work end-to-end
4. **Mock only at boundaries**: Keep I/O mocking minimal
5. **Test error paths**: Ensure error handling is robust

## Automation Mode Behavior

**In Automation Mode** (`PRODIGY_AUTOMATION=true`):
- Parse gaps from environment or arguments
- Apply functional programming fixes systematically
- Output progress for each improvement
- **ALWAYS commit fixes** (required for Prodigy validation)
- Return JSON result indicating completion

## Error Handling

The command will:
- Handle malformed gap data gracefully
- Skip gaps that can't be auto-fixed using functional patterns
- Report any gaps that couldn't be resolved
- Always output valid completion status
- Preserve existing functionality during refactoring

## Example Gap Resolution

### Input Gap
```json
{
  "critical_debt_remaining": {
    "description": "High-priority authentication function still too complex",
    "location": "src/auth.rs:authenticate_user:45",
    "severity": "critical",
    "suggested_fix": "Extract pure functions for validation logic",
    "original_score": 9.2,
    "current_score": 9.2
  }
}
```

### Applied Solution
1. **Extract pure validation functions** from authentication logic
2. **Use pattern matching** for user state validation
3. **Create function composition** for authentication pipeline
4. **Add comprehensive tests** for each pure function
5. **Implement early returns** to reduce nesting

### Output Result
```json
{
  "completion_percentage": 95.0,
  "status": "complete",
  "gaps_fixed": [
    "Extracted 4 pure functions from authenticate_user",
    "Reduced cyclomatic complexity from 15 to 6",
    "Added 12 test cases covering all error paths",
    "Eliminated 3 levels of nesting using early returns"
  ],
  "files_modified": [
    "src/auth.rs",
    "tests/auth_test.rs"
  ],
  "functional_improvements": [
    "Pure function extraction",
    "Pattern matching for state validation",
    "Function composition for authentication pipeline",
    "Immutable data flow implementation"
  ]
}
```

## Success Criteria

The command succeeds when:
1. All critical and high severity gaps are addressed using functional programming
2. At least 90% of medium severity gaps resolved
3. Tests pass after functional refactoring
4. No new technical debt introduced
5. **All fixes are committed to git** (REQUIRED for validation)
6. Code follows functional programming principles

## Important Notes

1. **NEVER DELETE IMPLEMENTATION_PLAN.md** - The workflow requires this file throughout execution
2. **Always apply functional programming principles** when fixing technical debt
3. **Preserve existing functionality** during refactoring
4. **Focus on pure functions** - easier to test and reason about
5. **Use immutable data structures** where possible
6. **Separate I/O from business logic** - core principle for maintainable code
7. **Create composable functions** - build complex behavior from simple parts
8. **Output valid JSON** for workflow parsing
9. **MUST create git commit** - Prodigy requires this to verify fixes were made
10. **Document functional patterns used** in commit messages
# Complete Debtmap Fix Command

Completes a partial technical debt fix by addressing validation gaps and remaining debt items.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-complete-debtmap-fix [--gaps <validation-gaps-json>]
```

Examples:
- `/prodigy-complete-debtmap-fix --gaps ${validation.gaps}` with specific gaps from validation

## What This Command Does

1. **Receives Validation Gaps**
   - Gets list of remaining debt items from debtmap validation
   - Parses gap details including locations, severity, and specific metrics
   - Prioritizes fixes by impact and feasibility

2. **Completes Technical Debt Resolution**
   - Addresses each gap systematically using functional programming principles
   - Focuses on high-priority debt items first
   - Implements targeted fixes based on validation feedback

3. **Verifies Completion**
   - Re-checks implementation after fixes
   - Ensures gaps are addressed without introducing new debt
   - Outputs completion status

## Execution Process

### Step 1: Parse Input

The command will:
- Extract validation gaps from $ARGUMENTS (`--gaps` parameter)
- If no gaps provided, analyze current debtmap state to identify remaining issues
- Parse gaps JSON to understand what specific improvements are needed
- Prioritize gaps by severity and impact

### Step 2: Analyze Technical Debt Gaps

Prioritize gaps by:
- **Critical severity (score >= 8)**: Fix immediately
- **High severity (score 6-8)**: Address next
- **Medium severity (score 4-6)**: Fix if feasible
- **Low severity (score < 4)**: Optional improvements

Gap types to handle:
- **Unresolved critical complexity**: Functions with high cyclomatic complexity
- **Missing test coverage**: Functions with inadequate test coverage
- **Deep nesting**: Functions with excessive nesting depth
- **Function length**: Overly long functions
- **New technical debt**: Regression issues introduced during initial fix

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

### Step 4: Incremental Improvement Strategy

For each gap:

1. **Identify the core issue** causing the debt
2. **Apply minimal functional refactoring** that addresses the specific gap
3. **Preserve existing improvements** - don't undo previous fixes
4. **Verify metric improvement** using functional programming principles
5. **Ensure no regression** in other areas

### Step 5: Handle Multiple Attempts

This command may be called multiple times (max_attempts: 3 in workflow):

**Attempt 1**: Address critical gaps using conservative functional patterns
- Focus on highest impact debt items
- Apply safe refactoring with pure function extraction

**Attempt 2**: Apply more aggressive functional programming patterns
- Use advanced patterns like function composition
- Consider more comprehensive restructuring

**Attempt 3**: Make pragmatic improvements for threshold
- Focus on achieving minimum viable improvement
- Document remaining technical debt for future work

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

### Step 7: Commit Functional Improvements

Create a clear commit documenting the functional programming improvements:

```bash
git add -A
git commit -m "fix: complete technical debt resolution with functional patterns

- Applied functional programming principles to address validation gaps:
  * Extracted N pure functions from complex imperative code
  * Replaced nested conditionals with pattern matching
  * Used function composition for data transformation pipelines
  * Separated I/O operations from business logic
  * Added comprehensive test coverage for pure functions

- Specific improvements:
  * Reduced cyclomatic complexity in [function] from X to Y
  * Added test coverage for critical error paths
  * Eliminated deep nesting through early returns
  * Extracted helper functions using immutable data flow

- Gaps addressed: [list specific gaps resolved]
- Functions improved: [list of functions with their files]
"
```

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

1. **Always apply functional programming principles** when fixing technical debt
2. **Preserve existing functionality** during refactoring
3. **Focus on pure functions** - easier to test and reason about
4. **Use immutable data structures** where possible
5. **Separate I/O from business logic** - core principle for maintainable code
6. **Create composable functions** - build complex behavior from simple parts
7. **Output valid JSON** for workflow parsing
8. **MUST create git commit** - Prodigy requires this to verify fixes were made
9. **Document functional patterns used** in commit messages
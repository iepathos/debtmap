---
name: prodigy-complete-debt-fix
description: Complete a partially fixed debt item based on validation gaps
---

# Complete Debt Fix

Completes the resolution of a tech debt item that was partially fixed, addressing specific gaps identified by validation.

## Parameters

- `--json`: Complete JSON object for the original debt item
- `--gaps`: Array of specific gaps identified by validation (from `${validation.gaps}`)

## Process

### Step 1: Parse Parameters

Extract information from the provided parameters:
- Original debt item details from JSON
- Specific gaps that need to be addressed
- Previous fix attempts (if available in context)

### Step 2: Analyze the Gaps

Common gap types and their solutions:

#### Complexity Gaps
**Gap**: "Cyclomatic complexity still above ideal threshold"
**Solution**: 
- Further extract pure functions for decision logic
- Consolidate similar branches using pattern matching
- Convert nested conditions to early returns

**Gap**: "Function still too long (X lines)"
**Solution**:
- Extract logical sections into named functions
- Move initialization code to builders/factories
- Separate validation from processing logic

#### Coverage Gaps
**Gap**: "Critical branches not covered"
**Solution**:
- Add specific test cases for uncovered branches
- Include edge cases and error conditions
- Test all enum variants and match arms

**Gap**: "No test file found for module"
**Solution**:
- Create appropriate test module
- Add comprehensive test suite
- Cover happy path, edge cases, and errors

#### Nesting Gaps
**Gap**: "Deep nesting still present (> 3 levels)"
**Solution**:
- Use early returns to reduce nesting
- Extract nested logic into helper functions
- Convert to functional style with combinators

### Step 3: Apply Targeted Fixes

Based on the specific gaps, apply focused improvements:

```rust
// Example: If gap is "Cyclomatic complexity still above threshold"
// Focus on the most complex branches

// Before: Complex nested conditions
if condition_a {
    if condition_b {
        if condition_c {
            // deep nesting
        }
    }
}

// After: Early returns and extracted logic
if !condition_a {
    return early_result;
}

if !meets_criteria(condition_b, condition_c) {
    return default_result;
}

// simplified flow
```

### Step 4: Incremental Improvement Strategy

For each gap:

1. **Identify the minimal change** that addresses the gap
2. **Preserve existing fixes** - don't undo previous improvements
3. **Focus on the specific metric** mentioned in the gap
4. **Verify no regression** in other metrics

### Step 5: Handle Multiple Attempts

This command may be called multiple times (max_attempts: 3 in workflow):

**Attempt 1**: Address the highest priority gaps
- Focus on the most impactful improvements
- Apply conservative refactoring

**Attempt 2**: If still incomplete, try alternative approaches
- Consider different refactoring patterns
- Add more comprehensive tests

**Attempt 3**: Final push for acceptable threshold
- Make pragmatic trade-offs
- Document any remaining technical debt

### Step 6: Verify Improvements

After applying fixes, verify that gaps are addressed:

```bash
# Run tests to ensure nothing broke
just test

# Check if the specific metric improved
# (This would be done by the next validation cycle)
```

### Step 7: Commit the Completion

Create a commit documenting the gap resolution:

```bash
git add -A
git commit -m "fix: complete debt resolution for [function_name]

- Addressed gaps: [list specific gaps]
- Applied: [specific fixes made]
- Function: [item.location.function] in [item.location.file]
- Validation improvement: [estimated improvement]
"
```

## Gap-Specific Strategies

### For "Complexity still too high"
1. Look for repeated patterns to consolidate
2. Extract classification/categorization logic
3. Use functional composition instead of imperative code
4. Consider if the complexity is inherent (e.g., state machines)

### For "Coverage still insufficient"
1. Focus on uncovered critical paths first
2. Add parameterized tests for similar cases
3. Test error conditions and edge cases
4. Mock external dependencies if needed

### For "Function still too long"
1. Group related statements into logical blocks
2. Extract each block as a named function
3. Keep the main function as orchestration
4. Ensure extracted functions are reusable

### For "Nesting too deep"
1. Invert conditions and use early returns
2. Extract nested loops into iterator chains
3. Use Option/Result combinators
4. Flatten using pattern matching

## Handling Validation Feedback

The validation provides specific feedback that guides the completion:

- **"Consider extracting additional helper functions"** → Look for logical groups to extract
- **"Add tests for error conditions"** → Focus on error path coverage
- **"Reduce cognitive complexity"** → Simplify control flow
- **"Separate I/O from business logic"** → Extract pure functions

## Success Criteria

The completion is successful when:
- [ ] All specified gaps are addressed
- [ ] No regression in previously fixed metrics
- [ ] Tests still pass
- [ ] Code remains maintainable
- [ ] Changes are committed

## Notes

- This command works in conjunction with prodigy-validate-debt-fix
- It receives gaps from the validation step
- May be called up to 3 times per debt item
- Should make incremental improvements, not complete rewrites
- Focuses on reaching the 90% validation threshold
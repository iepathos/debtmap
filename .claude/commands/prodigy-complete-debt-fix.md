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

Common gap types and their solutions using functional programming:

#### Complexity Gaps
**Gap**: "Cyclomatic complexity still above ideal threshold"
**Solution** (Functional approach):
- Extract pure functions for decision logic (no side effects)
- Use pattern matching instead of if-else chains
- Convert nested conditions to early returns
- Replace imperative loops with iterator chains
- Use Option/Result combinators

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
// Apply functional programming patterns

// Before: Complex nested conditions (imperative)
let mut result = Vec::new();
if condition_a {
    if condition_b {
        for item in items {
            if condition_c(item) {
                result.push(transform(item));
            }
        }
    }
}

// After: Functional composition with pure functions
fn should_process(a: bool, b: bool) -> bool {
    a && b
}

fn process_items(items: &[Item]) -> Vec<Result> {
    items.iter()
        .filter(|item| condition_c(item))
        .map(|item| transform(item))
        .collect()
}

// Main logic using functional patterns
let result = if should_process(condition_a, condition_b) {
    process_items(&items)
} else {
    Vec::new()
};
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
git commit -m "fix: complete debt resolution with functional patterns

- Addressed gaps: [list specific gaps]
- Applied functional programming patterns:
  * Extracted N pure functions
  * Replaced loops with iterator chains
  * Used pattern matching for control flow
  * Separated I/O from business logic
- Function: [item.location.function] in [item.location.file]
- Validation improvement: [estimated improvement]
"
```

## Gap-Specific Strategies

### For "Complexity still too high" - Functional Approach
1. **Extract pure functions**: No side effects, deterministic outputs
2. **Use pattern matching**: Replace if-else chains with match expressions
3. **Iterator chains**: Replace loops with map/filter/fold
4. **Function composition**: Build complex behavior from simple functions
5. **Immutability**: Use `&self` instead of `&mut self` where possible
6. **Type-driven design**: Use the type system to enforce invariants
7. Consider if the complexity is inherent (e.g., state machines, parsers)

### For "Coverage still insufficient"
1. Focus on uncovered critical paths first
2. Add parameterized tests for similar cases
3. Test error conditions and edge cases
4. Mock external dependencies if needed

### For "Function still too long" - Functional Decomposition
1. **Identify pure logic**: Extract calculations and transformations
2. **Separate I/O from logic**: Pure functions for business logic
3. **Create small, composable functions**: Each does one thing well
4. **Use function composition**: Chain simple functions for complex behavior
5. **Extract decision logic**: Pure predicates for conditions
6. **Keep orchestration thin**: Main function just coordinates

### For "Nesting too deep" - Functional Flattening
1. **Early returns with guard clauses**: Reduce nesting depth
2. **Iterator chains**: Replace nested loops with flat chains
   ```rust
   // Instead of nested loops
   items.iter()
       .flat_map(|x| x.children.iter())
       .filter(|c| c.is_valid())
       .collect()
   ```
3. **Option/Result combinators**: Chain operations without nesting
   ```rust
   value.and_then(|v| process(v))
        .map(|r| transform(r))
        .unwrap_or_default()
   ```
4. **Pattern matching**: Flatten complex conditionals
5. **Extract pure predicates**: Named boolean functions

## Handling Validation Feedback - Functional Programming Focus

The validation provides specific feedback that guides the completion:

- **"Consider extracting additional helper functions"** → Extract pure functions with no side effects
- **"Add tests for error conditions"** → Test pure functions in isolation
- **"Reduce cognitive complexity"** → Use functional patterns (map, filter, fold)
- **"Separate I/O from business logic"** → Create pure core with I/O shell
- **"Simplify control flow"** → Use pattern matching and combinators
- **"Reduce mutation"** → Prefer immutable data structures
- **"Extract decision logic"** → Create pure predicates and classifiers

### Functional Programming Principles to Apply

1. **Pure Functions**: No side effects, deterministic
2. **Immutability**: Avoid `mut` where possible
3. **Function Composition**: Build from small functions
4. **Type Safety**: Use enums and strong types
5. **Iterator Patterns**: Prefer chains over loops
6. **Pattern Matching**: Replace complex conditionals

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
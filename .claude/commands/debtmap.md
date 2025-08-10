---
name: debtmap
description: Analyze tech debt with debtmap, fix the top priority item, test, and commit
---

# Fix Top Priority Tech Debt

Use debtmap to analyze the repository and identify tech debt, then fix the highest priority item.

## Process

### Step 1: Generate Coverage Data
Run the following command to generate LCOV coverage data:
```
cargo tarpaulin --out lcov --output-dir target/coverage --timeout 120
```
- Verify the file `target/coverage/lcov.info` was created
- If tarpaulin fails, note the error and proceed with analysis without coverage

### Step 2: Initial Analysis
Run debtmap to analyze the current tech debt:
```
debtmap analyze . --lcov target/coverage/lcov.info
```
- **Important**: Save the output and note:
  - "Total debt score" value
  - "Overall coverage" percentage (if available)
- If LCOV file is missing, run without the `--lcov` flag

### Step 3: Identify Priority
From the debtmap output, identify the top priority issue:

1. **FIRST PRIORITY**: Check "TOP 5 TESTING RECOMMENDATIONS" section
   - Look for items with ROI score ≥ 5
   - Start with the highest ROI score
   
2. **SECOND PRIORITY**: If no high-ROI testing opportunities exist
   - Check "COMPLEXITY HOTSPOTS" section
   - Focus on functions with highest complexity scores
   
3. **THIRD PRIORITY**: "CRITICAL RISK FUNCTIONS"
   - Only if they also appear in testing recommendations

### Step 4: Plan the Fix
Based on the priority type, create an implementation plan:

**For Testing Priorities (ROI ≥ 5):**
- Identify the function/module needing tests
- Plan test cases for:
  - Happy path scenarios
  - Edge cases and boundary conditions
  - Error conditions and invalid inputs
  - Any uncovered branches or paths

**For Complexity Hotspots:**
- Analyze the complex function
- Plan refactoring using functional patterns:
  - Replace loops with iterators
  - Convert if-else chains to pattern matching
  - Extract pure functions from side-effect code
  - Simplify nested logic
  - Break large functions into smaller, composable units

### Step 5: Implement the Fix
Apply the planned changes:

**For Testing:**
- Write comprehensive test cases
- Ensure all identified scenarios are covered
- Use descriptive test names
- Follow existing test patterns in the codebase

**For Refactoring:**
- Apply functional programming patterns
- Maintain backwards compatibility
- Preserve all existing functionality
- Keep changes focused and incremental

### Step 6: Verify Changes
Run the following commands in order:
```
just ci
```
- All tests must pass
- No clippy warnings allowed
- Code must be properly formatted

### Step 7: Regenerate Coverage
If you added tests, regenerate coverage:
```
cargo tarpaulin --out lcov --output-dir target/coverage --timeout 120
```

### Step 8: Final Analysis
Run debtmap again to measure improvement:
```
debtmap analyze . --lcov target/coverage/lcov.info
```
- Note the new values:
  - "Total debt score"
  - "Overall coverage" percentage (if tests were added)
- Calculate changes:
  - Debt score change: initial score - final score
  - Coverage change: final coverage - initial coverage (if applicable)
- Document specific improvements achieved

### Step 9: Commit Changes
Create a descriptive commit message:

**For test additions:**
```
test: add comprehensive tests for [module/function name]

- Added [number] test cases covering [specific scenarios]
- Coverage improvement: +[X.XX]% (from [initial]% to [final]%)
- Debt score reduction: -[amount] (from [initial] to [final])

Tech debt category: Testing coverage (ROI optimization)
```

**For complexity reduction:**
```
refactor: reduce complexity in [module/function name]

- [Specific refactoring applied, e.g., "Replaced nested loops with iterator chain"]
- Complexity reduction: [metric change, e.g., "cognitive complexity from 15 to 8"]
- Debt score reduction: -[amount] (from [initial] to [final])

Tech debt category: Complexity reduction
```

## Important Instructions

**IMPORTANT**: When making ANY commits, do NOT include attribution text like "🤖 Generated with Claude Code" or "Co-Authored-By: Claude" in commit messages. Keep commits clean and focused on the actual changes.

## Success Criteria

Complete each step in order:
- [ ] Coverage data generated with cargo tarpaulin (or noted if unavailable)
- [ ] Initial debtmap analysis completed with baseline debt score recorded
- [ ] Top priority issue identified following the prioritization strategy
- [ ] Implementation plan created based on issue type (testing vs complexity)
- [ ] Fix implemented following the plan
- [ ] All tests passing (cargo test)
- [ ] No clippy warnings (cargo clippy)
- [ ] Code properly formatted (cargo fmt)
- [ ] Coverage regenerated if tests were added
- [ ] Final debtmap analysis shows debt score change
- [ ] Changes committed with descriptive message including metrics

## Notes

- Always work on one issue at a time for focused, measurable improvements
- If debtmap shows no significant debt (score < 100), consider the codebase healthy
- Testing priorities with ROI ≥ 5 provide the best return on investment
- Complexity refactoring should preserve all existing functionality
- Each commit should show measurable debt reduction

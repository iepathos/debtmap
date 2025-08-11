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
- **CRITICAL - RECORD ALL INITIAL VALUES**: You MUST save and note:
  - "Total debt score" value (initial baseline)
  - "Overall coverage" percentage (initial baseline - REQUIRED if lcov is available)
  - Total function count (initial baseline)
- If LCOV file is missing, run without the `--lcov` flag but note "Coverage: not measured"

### Step 3: Identify Priority
From the debtmap output, identify the top priority issue:

**CRITICAL VALIDATION**: First examine ALL items in "TOP 5 TESTING RECOMMENDATIONS"
- Count how many items have ROI score ≥ 5
- If ZERO items have ROI ≥ 5, you MUST skip to SECOND PRIORITY
- If ANY items have ROI ≥ 5, proceed with FIRST PRIORITY

1. **FIRST PRIORITY**: Testing with ROI ≥ 5
   - ONLY select from items with ROI score ≥ 5
   - Start with the highest ROI score ≥ 5
   - **IMPORTANT**: If ALL testing recommendations have ROI < 5, DO NOT select any testing items
   - Instead, immediately proceed to SECOND PRIORITY
   
2. **SECOND PRIORITY**: Complexity Hotspots (when no ROI ≥ 5 exists)
   - Check "COMPLEXITY HOTSPOTS" section
   - Focus on functions with highest complexity scores
   - This is the correct priority when testing ROI is low
   
3. **THIRD PRIORITY**: "CRITICAL RISK FUNCTIONS"
   - Only if they also appear in testing recommendations with ROI ≥ 5

### Step 4: Plan the Fix
Based on the priority type, create an implementation plan:

**VALIDATION CHECK**: Confirm you selected the correct priority:
- If working on testing: Verify the selected item has ROI ≥ 5
- If working on complexity: Verify ALL testing items had ROI < 5
- If unsure, re-read Step 3 and re-evaluate

**For Testing Priorities (ROI ≥ 5):**
- First, assess if the function is orchestration or I/O code
- If it's an orchestration/I/O function with trivial complexity:
  - Consider if it delegates to already-tested functions
  - Extract any pure business logic into separate testable functions
  - Move formatting/parsing logic to dedicated modules
  - Keep thin I/O wrappers untested (they're not the real debt)
- For actual business logic functions:
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

**For Orchestration and I/O Functions:**
- Extract pure logic into testable functions:
  - Move formatting logic to pure functions that return strings
  - Extract parsing/validation to separate modules
  - Create pure functions for decision logic (e.g., "should_generate_report")
- Keep I/O operations in thin wrappers that call the pure functions
- Write tests for the extracted pure functions, not the I/O wrappers
- Consider moving business logic to appropriate modules (e.g., `parsers`, `formatters`, `validators`)

**For Testing Business Logic:**
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
- **CRITICAL**: Record ALL of these values:
  - "Total debt score" (final value)
  - "Overall coverage" percentage (final value)
  - Total function count (final value)
- Calculate and document ALL changes:
  - Debt score change: initial score - final score
  - Coverage percentage change: final coverage% - initial coverage%
  - Function count change: final count - initial count
- **REQUIRED**: If coverage was measured, you MUST include the coverage change in the commit message
- Document specific improvements achieved

### Step 9: Commit Changes
Create a descriptive commit message:

**For test additions:**
```
test: add comprehensive tests for [module/function name]

- Added [number] test cases covering [specific scenarios]
- Coverage change: +[X.XX]% (from [initial]% to [final]%)
- Debt score change: [+/-amount] (from [initial] to [final])
- Function count change: +[number] (from [initial] to [final])

Tech debt category: Testing coverage (ROI optimization)

IMPORTANT: You MUST include the actual coverage percentage change if coverage was measured.
Even if coverage didn't increase (e.g., when adding test functions), state: "Coverage: unchanged at X.XX%"
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

**MANDATORY COMMIT MESSAGE REQUIREMENTS**:
Every commit MUST include these metrics if they were measured:
1. Coverage change: ALWAYS include if lcov was used (e.g., "+2.5% (from 48.2% to 50.7%)" or "unchanged at 52.3%")
2. Debt score change: ALWAYS include (e.g., "-150 (from 3735 to 3585)")  
3. Function count change: Include if it changed (e.g., "+23 (from 1228 to 1251)")

If coverage wasn't measured, explicitly state: "Coverage: not measured (no lcov data)"

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

## Orchestration and I/O Function Guidelines

When debtmap flags orchestration or I/O functions as untested:

1. **Recognize the pattern**: Functions with cyclomatic complexity = 1 that coordinate modules or perform I/O are not the real debt
2. **Extract testable logic**: Instead of testing I/O directly, extract pure functions that can be unit tested
3. **Follow functional programming principles**: 
   - Pure core: Business logic in pure functions
   - Imperative shell: Thin orchestration/I/O wrappers that don't need testing
4. **Common patterns to extract**:
   - Formatting functions: Extract logic that builds strings from data
   - Parsing functions: Move to dedicated parser modules
   - Decision functions: Extract "should we do X" logic from "do X" execution
   - Coordination logic: Extract "how to coordinate" from "perform coordination"
5. **Don't force unit tests on**: 
   - Functions that just print to stdout
   - Simple delegation to other modules
   - Module orchestration that just sequences calls
   - File I/O wrappers
   - Network I/O operations

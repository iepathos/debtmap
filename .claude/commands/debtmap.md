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
Run debtmap to analyze the current tech debt and get the top recommendation:
```
debtmap analyze . --lcov target/coverage/lcov.info --top 1
```
- **CRITICAL - RECORD ALL INITIAL VALUES**: You MUST save and note:
  - Current test coverage percentage (if lcov is available)
  - Complexity metrics from the top recommendation
  - Function/file being analyzed
- If LCOV file is missing, run without the `--lcov` flag but note "Coverage: not measured"

### Step 3: Identify Priority
The debtmap output now shows the #1 TOP RECOMMENDATION with a unified priority score:

The recommendation will include:
- **SCORE**: Unified priority score (higher = more critical)
- **TEST GAP**: The specific function/file needing attention
- **ACTION**: What needs to be done (refactor, add tests, etc.)
- **IMPACT**: Expected improvements (coverage %, complexity reduction, risk reduction)
- **WHY**: Explanation of why this is the top priority

Priority categories:
1. **CRITICAL (Score 10.0)**: Functions with high complexity and zero coverage
2. **HIGH (Score 7-9)**: Important business logic with test gaps
3. **MEDIUM (Score 4-6)**: Moderate complexity or coverage issues
4. **LOW (Score 1-3)**: Minor improvements

### Step 4: Plan the Fix
Based on the ACTION specified in the top recommendation:

**For "Refactor to reduce complexity" actions:**
- Analyze the complex function
- Plan refactoring using functional patterns:
  - Replace loops with iterators
  - Convert if-else chains to pattern matching
  - Extract pure functions from side-effect code
  - Simplify nested logic
  - Break large functions into smaller, composable units
- After refactoring, add comprehensive tests

**For "Add X unit tests" actions:**
- First, assess if the function is orchestration or I/O code
- If it's an orchestration/I/O function:
  - Extract any pure business logic into separate testable functions
  - Move formatting/parsing logic to dedicated modules
  - Keep thin I/O wrappers untested (they're not the real debt)
- For actual business logic functions:
  - Plan test cases for:
    - Happy path scenarios
    - Edge cases and boundary conditions
    - Error conditions and invalid inputs
    - Any uncovered branches or paths

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
debtmap analyze . --lcov target/coverage/lcov.info --top 1
```
- **CRITICAL**: Note the new top recommendation
- Compare with initial analysis:
  - Has the original issue been resolved?
  - What is the new top priority?
  - Document coverage improvements if tests were added
- **REQUIRED**: If coverage was measured, you MUST include the coverage change in the commit message
- Document specific improvements achieved

### Step 9: Commit Changes
Create a descriptive commit message:

**For test additions:**
```
test: add comprehensive tests for [module/function name]

- Added [number] test cases covering [specific scenarios]
- Coverage improvement: [describe impact if measured]
- Resolved priority: [describe the specific issue from debtmap]

Tech debt: Priority score [X] issue resolved
```

**For complexity reduction:**
```
refactor: reduce complexity in [module/function name]

- [Specific refactoring applied, e.g., "Replaced nested loops with iterator chain"]
- Complexity reduced from [X] to [Y]
- Resolved priority: [describe the specific issue from debtmap]

Tech debt: Priority score [X] issue resolved
```

## Important Instructions

**IMPORTANT**: When making ANY commits, do NOT include attribution text like "🤖 Generated with Claude Code" or "Co-Authored-By: Claude" in commit messages. Keep commits clean and focused on the actual changes.

**COMMIT MESSAGE FOCUS**:
Commit messages should focus on:
1. What was changed (refactoring or tests added)
2. The specific improvement made
3. The priority score of the resolved issue
4. Coverage impact if applicable

## Success Criteria

Complete each step in order:
- [ ] Coverage data generated with cargo tarpaulin (or noted if unavailable)
- [ ] Initial debtmap analysis completed with top priority identified
- [ ] Implementation plan created based on the ACTION specified
- [ ] Fix implemented following the plan
- [ ] All tests passing (cargo test)
- [ ] No clippy warnings (cargo clippy)
- [ ] Code properly formatted (cargo fmt)
- [ ] Coverage regenerated if tests were added
- [ ] Final debtmap analysis shows improvement
- [ ] Changes committed with descriptive message

## Notes

- Always work on one issue at a time for focused, measurable improvements
- The unified priority score considers complexity, coverage, and risk factors
- Priority score 10.0 indicates critical issues requiring immediate attention
- Complexity refactoring should preserve all existing functionality
- Each commit should resolve the identified priority issue

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

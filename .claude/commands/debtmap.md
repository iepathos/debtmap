---
name: debtmap
description: Analyze tech debt with debtmap, fix the top priority item, test, and commit
---

# Fix Top Priority Tech Debt

Use debtmap to analyze the repository and identify tech debt, then fix the highest priority item.

## Process

### Step 1: Generate Coverage Data and Record Baseline
Run the following command to generate LCOV coverage data:
```
cargo tarpaulin --out lcov --output-dir target/coverage --timeout 120
```
- Verify the file `target/coverage/lcov.info` was created
- If tarpaulin fails, note the error and proceed with analysis without coverage
- The baseline coverage will be recorded from debtmap's output in Step 2

### Step 2: Initial Analysis
Run debtmap to analyze the current tech debt and get the top recommendation:
```
debtmap analyze . --lcov target/coverage/lcov.info --top 1
```
- **CRITICAL**: Record the BASELINE values shown at the bottom:
  - TOTAL DEBT SCORE (e.g., "TOTAL DEBT SCORE: 3735")
  - OVERALL COVERAGE (e.g., "OVERALL COVERAGE: 48.32%")
- Store these as your BASELINE values for comparison later
- Note the top recommendation details:
  - Priority SCORE value
  - TEST GAP location (file:line and function)
  - ACTION required
  - IMPACT predictions
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

### Step 3.5: Evaluate Refactoring Approach (for Complexity Issues)

When debtmap identifies a function with high complexity (>10), evaluate different refactoring strategies:

**CRITICAL: Before extracting helper methods, consider these approaches in order:**

1. **Static Pure Function Extraction (PREFERRED)**
   - Extract classification/decision logic as static pure functions
   - These functions should be reusable across the codebase
   - Example: `classify_type(name: &str) -> Type` instead of multiple helpers
   - Benefits: Testable in isolation, reduces main function complexity, functional style

2. **Pattern Consolidation**
   - Look for repeated patterns in conditionals
   - Use match expressions with guards instead of if-else chains
   - Combine similar branches using pattern matching
   - Example: Replace multiple if-else checking string patterns with a single match

3. **Functional Composition**
   - Use `.map()`, `.filter()`, `.fold()` instead of loops
   - Chain operations instead of intermediate variables
   - Extract predicates as pure functions

**WARNING: Avoid these anti-patterns:**
- Creating multiple single-use helper methods that are only called from tests
- Extracting helpers that aren't used in production code paths
- Over-engineering: 5+ helper methods for a 15-line function
- Adding complexity to reduce complexity

**Validation Check:**
Before implementing, verify your approach will:
- Actually reduce the complexity score (not just move it)
- Keep or improve test coverage (not decrease it)
- Result in fewer total functions (or same number with better structure)
- Use functional programming patterns where appropriate

### Step 3.7: Quick Refactoring Decision Tree

For functions with complexity > 10:

```
Is it a visitor pattern or large switch/match?
├─ YES → Don't refactor, add tests if needed
└─ NO → Continue
   │
   Does it classify/categorize inputs?
   ├─ YES → Extract as pure static function
   └─ NO → Continue
      │
      Does it have repeated similar conditions?
      ├─ YES → Consolidate with pattern matching
      └─ NO → Continue
         │
         Does it have nested loops?
         ├─ YES → Convert to iterator chains
         └─ NO → Consider if refactoring is needed
```

### Step 4: Plan the Fix
Based on the ACTION specified in the top recommendation:

**For "Refactor to reduce complexity" actions:**

1. **Analyze the function structure:**
   - Is it mostly a large switch/match on different cases? → Keep as-is, it's already functional
   - Is it performing classification/categorization? → Extract as pure static function
   - Is it orchestrating I/O operations? → Extract business logic only
   - Is it a visitor pattern implementation? → Consider if complexity is inherent

2. **Choose refactoring strategy based on complexity source:**
   
   **Pattern: Multiple similar conditionals**
   ```rust
   // Before: Complexity 15
   if name.contains("async") || name.contains("await") { 
       CallType::Async 
   } else if name.starts_with("handle_") { 
       CallType::Delegate 
   } else if name.starts_with("map") { 
       CallType::Pipeline 
   } else { 
       CallType::Direct 
   }
   
   // After: Extract as pure function
   fn classify_call_type(name: &str) -> CallType {
       match () {
           _ if name.contains("async") || name.contains("await") => CallType::Async,
           _ if name.starts_with("handle_") => CallType::Delegate,
           _ if name.starts_with("map") => CallType::Pipeline,
           _ => CallType::Direct,
       }
   }
   ```

   **Pattern: Nested loops and conditions**
   - Replace with iterator chains and functional combinators
   - Extract predicates as named functions

   **Pattern: Large match/switch statement**
   - Often this is the CORRECT pattern - don't refactor
   - Cyclomatic complexity ≠ bad code
   - Visitor patterns naturally have high complexity

3. **Test the refactoring impact:**
   - Count functions before: `find src -name "*.rs" | xargs grep -E "^\s*(pub\s+)?fn\s+" | wc -l`
   - Apply refactoring
   - Count functions after
   - If function count increased by >20%, reconsider approach

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
- This will update the lcov.info file for the final analysis

### Step 8: Final Analysis
Run debtmap again to verify improvement:
```
debtmap analyze . --lcov target/coverage/lcov.info --top 1
```
- **CRITICAL**: Record the NEW values:
  - TOTAL DEBT SCORE
  - OVERALL COVERAGE percentage
- Calculate the changes:
  - Coverage change: NEW% - BASELINE%
  - Debt score change: BASELINE - NEW
- Verify the original issue is resolved (should no longer be #1 priority)
- Note what the new top priority is

### Step 8.5: Understanding Metrics Changes

**Interpret your metrics changes:**

**Debt Score Changes:**
- **Decreased**: The refactoring/tests successfully reduced technical debt
- **Increased slightly (<100 points)**: Often due to test functions being counted (they have 0% coverage)
- **Increased significantly (>100 points)**: May indicate added complexity without corresponding benefit

**Coverage Changes:**
- **Increased**: Tests are covering more production code
- **Decreased slightly (<1%)**: Usually because new test functions aren't covered by other tests
- **Decreased significantly (>1%)**: New production code may lack tests

**Function Count Changes:**
- **Same or slight decrease**: Good refactoring that consolidated logic
- **Increase <10**: Acceptable for better code structure
- **Increase >20**: Consider if the refactoring added too much abstraction

**Understanding Trade-offs:**
- Test functions add to debt score (limitation of the tool)
- Extracting pure functions may temporarily increase metrics but improve maintainability
- Focus on whether the specific issue was resolved, not just raw metrics
- Consider long-term maintainability over short-term metrics

### Step 9: Commit Changes
Create a descriptive commit message using the values recorded from debtmap:

**For test additions:**
```
test: add comprehensive tests for [module/function name]

- Added [number] test cases covering [specific scenarios]
- Coverage: +X.XX% (from BASELINE% to NEW%)
- Debt score: [+/-]XX (from BASELINE to NEW)
- Resolved: Priority [SCORE] - [function] with [coverage]% coverage

Tech debt: Fixed top priority issue
```

**For complexity reduction:**
```
refactor: reduce complexity in [module/function name]

- [Specific refactoring applied, e.g., "Replaced nested loops with iterator chain"]
- Complexity reduced from [X] to [Y]
- Coverage: +X.XX% (from BASELINE% to NEW%) [if coverage changed]
- Debt score: [+/-]XX (from BASELINE to NEW)
- Resolved: Priority [SCORE] - [function] complexity [X]

Tech debt: Fixed top priority issue
```

**Important**: Use the exact coverage percentages and debt scores from debtmap's output, not from tarpaulin directly.

## Important Instructions

**IMPORTANT**: When making ANY commits, do NOT include attribution text like "🤖 Generated with Claude Code" or "Co-Authored-By: Claude" in commit messages. Keep commits clean and focused on the actual changes.

**COMMIT MESSAGE REQUIREMENTS**:
Every commit MUST include:
1. What was changed (refactoring or tests added)
2. **Coverage change with actual percentages from debtmap** (e.g., "+3.15% (from 48.32% to 51.47%)")
3. **Debt score change with actual values from debtmap** (e.g., "-150 (from 3735 to 3585)")
4. The priority score and description of resolved issue
5. If coverage wasn't measured, state: "Coverage: not measured (no lcov data)"

**Note**: Always use the OVERALL COVERAGE percentage shown by debtmap, not the line coverage from tarpaulin.

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

## Common Pitfalls to Avoid

### When Refactoring for Complexity:

❌ **DON'T:**
- Extract helper methods that are only called once
- Create test-only helper functions (helpers not used in production code)
- Break apart a clear match/switch into multiple functions
- Add abstraction layers for simple logic
- Refactor visitor pattern implementations (they're meant to have many branches)
- Create 5+ helper methods for a 15-line function

✅ **DO:**
- Extract reusable classification/decision logic as static pure functions
- Use functional patterns (map, filter, fold) where appropriate
- Consolidate similar patterns into single functions
- Keep related logic together
- Accept that some functions legitimately have high complexity
- Test the extracted pure functions thoroughly

### Understanding Debt Score Impact:

**Why debt score might increase after adding tests:**
- Test functions are counted in metrics but have 0% coverage themselves
- Each test function adds ~5-10 points to debt score
- This is a current limitation of the debtmap tool

**What to focus on:**
- Whether the TARGET function's complexity was reduced
- Whether the specific issue identified was resolved
- Overall code quality and maintainability
- Whether the refactoring follows functional programming principles

### Functional Programming Preferences:

**Prefer these patterns:**
- Pure functions over stateful methods
- Static methods for classification/utility functions
- Match expressions with guards over if-else chains
- Iterator chains over imperative loops
- Function composition over deep nesting
- Immutability by default

**Example of good refactoring:**
```rust
// Extract classification logic as pure static function
impl MyStruct {
    // This can be tested in isolation and reused
    fn classify_item(name: &str) -> ItemType {
        match () {
            _ if name.starts_with("test_") => ItemType::Test,
            _ if name.contains("_impl") => ItemType::Implementation,
            _ => ItemType::Regular,
        }
    }
    
    // Main function uses the pure classifier
    fn process(&mut self, name: &str) {
        let item_type = Self::classify_item(name);
        // ... rest of logic
    }
}
```

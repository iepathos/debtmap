# Complexity-Based Recommendations

Debtmap generates actionable recommendations based on function complexity tiers. This document explains how complexity is classified and what recommendations you can expect for each tier.

## Complexity Tier System

Debtmap uses a four-tier system to classify function complexity based on cyclomatic and cognitive complexity metrics:

### Low Complexity (Tier 1)

**Thresholds:**
- Cyclomatic complexity < 8
- Cognitive complexity < 15

**Characteristics:**
- Well-structured, easy to understand
- Simple control flow with minimal branching
- Easy to test and maintain
- Clear single responsibility

**Examples:**
- Simple validation functions
- Accessor/getter methods
- Small utility functions
- Functions with 2-3 conditional branches

**Recommendation Strategy:**
- **Primary Action:** Maintain current patterns
- **Goal:** Keep complexity low during future changes
- **Effort:** Minimal (0.5 hours)

**Sample Recommendation:**
```
Primary Action: Maintain current low complexity

Rationale: Function has low complexity (6/6). Continue following current
patterns to keep it maintainable.

Steps:
1. Add tests to preserve behavior during future changes
   Impact: +safety for refactoring
   Difficulty: Easy
```

### Moderate Complexity (Tier 2)

**Thresholds:**
- Cyclomatic complexity: 8-14
- Cognitive complexity: 15-24

**Characteristics:**
- Manageable but approaching maintainability limits
- Business logic with moderate branching
- Slightly harder to test but still reasonable
- May benefit from preventive refactoring

**Examples:**
- State reconciliation functions
- Configuration validators with multiple checks
- Moderate orchestration logic
- Functions with 4-8 decision points

**Recommendation Strategy:**
- **Primary Action:** Optional preventive refactoring
- **Goal:** Reduce to single-digit complexity (target ~5-8)
- **Rationale:** Prevent future complexity growth
- **Effort:** 1-2 hours

**Sample Recommendation:**
```
Primary Action: Optional: Reduce complexity from 9 to ~6 for future-proofing

Rationale: Moderate complexity (9/16). Below threshold but maintainable.
Preventive refactoring will ease future changes.

Steps:
1. Add tests before refactoring (if coverage < 80%)
   Impact: +safety net for refactoring
   Difficulty: Medium

2. Extract most complex section into focused function
   Impact: -3 complexity
   Difficulty: Medium

3. Verify tests still pass
   Impact: Confirmed refactoring safe
   Difficulty: Easy
```

### High Complexity (Tier 3)

**Thresholds:**
- Cyclomatic complexity: 15-24
- Cognitive complexity: 25-39

**Characteristics:**
- Exceeds maintainability thresholds
- Complex orchestration or large case statements
- Difficult to test comprehensively
- High bug risk and maintenance burden

**Examples:**
- Complex state machines
- Large switch/match statements
- Functions with nested conditionals
- Business logic with 10+ branches

**Recommendation Strategy:**
- **Primary Action:** Refactoring required
- **Goal:** Reduce to moderate complexity (target ~10)
- **Rationale:** Complexity exceeds safe limits
- **Effort:** 2-4 hours

**Sample Recommendation:**
```
Primary Action: Reduce complexity from 20 to ~10

Rationale: High complexity (20/30). Exceeds maintainability thresholds.
Refactoring required.

Steps:
1. Add tests before refactoring (if coverage < 80%)
   Impact: +safety net for refactoring
   Difficulty: Medium

2. Extract most complex section into focused function
   Impact: -10 complexity
   Difficulty: Hard

3. Verify tests still pass
   Impact: Confirmed refactoring safe
   Difficulty: Easy
```

### Very High Complexity (Tier 4)

**Thresholds:**
- Cyclomatic complexity ≥ 25
- Cognitive complexity ≥ 40

**Characteristics:**
- Critical complexity requiring immediate attention
- "God functions" with tangled logic
- Extremely difficult to test
- High defect probability
- Significant maintenance burden

**Examples:**
- God functions doing multiple responsibilities
- Complex parsers without proper decomposition
- Legacy code with accumulated complexity
- Functions with 15+ decision points

**Recommendation Strategy:**
- **Primary Action:** Significant refactoring required
- **Goal:** Reduce to high tier (target 10-15)
- **Rationale:** Critical complexity risk
- **Effort:** 4-8 hours

**Sample Recommendation:**
```
Primary Action: Reduce complexity from 40 to ~15

Rationale: Very high complexity (40/60). Critical complexity requiring
significant refactoring.

Steps:
1. Add comprehensive tests before refactoring
   Impact: +safety net for major changes
   Difficulty: Hard

2. Extract most complex sections into focused functions
   Impact: -25 complexity
   Difficulty: Hard

3. Verify all tests pass after each extraction
   Impact: Confirmed incremental safety
   Difficulty: Medium
```

## Complexity Pattern-Specific Recommendations

In addition to tier-based recommendations, Debtmap detects specific complexity patterns and provides targeted advice:

### High Nesting Pattern

**Detection Criteria:**
- Nesting depth ≥ 3
- Cognitive/Cyclomatic ratio > 1.5

**Recommended Approach:**
1. Apply early returns for error conditions
2. Extract nested conditionals into predicate functions
3. Reduce nesting to < 3 levels

**Impact:** Reduces cognitive complexity by 30-50%

### High Branching Pattern

**Detection Criteria:**
- Many decision points (cyclomatic ≥ 15)
- Low nesting depth
- High branch count

**Recommended Approach:**
1. Identify decision clusters (related conditional logic)
2. Extract clusters into focused functions
3. Split into 2-4 functions by responsibility

**Impact:** Reduces cyclomatic complexity to < 10 per function

### Mixed Complexity Pattern

**Detection Criteria:**
- Both high nesting AND high branching
- Nesting ≥ 3 AND cyclomatic ≥ 15

**Recommended Approach:**
1. **Phase 1:** Apply early returns and guard clauses (reduce nesting)
2. **Phase 2:** Extract functions from flattened structure (reduce branching)
3. Two-phase approach ensures safety

**Impact:** Reduces both metrics significantly

### Chaotic Structure Pattern

**Detection Criteria:**
- High token entropy (> 0.40)
- Inconsistent control flow patterns
- Unpredictable structure

**Recommended Approach:**
1. Standardize error handling patterns
2. Group related state transitions
3. Re-run entropy calculation to verify improvement

**Impact:** Makes code predictable before refactoring

## Target Complexity Calculation

Debtmap calculates appropriate targets based on current tier:

| Current Tier | Target Strategy | Example |
|-------------|----------------|---------|
| Low (5) | Maintain or slight improvement | 5 → 4 |
| Moderate (9) | Aim for single-digit | 9 → 6 |
| Moderate (12) | Aim for single-digit | 12 → 8 |
| High (20) | Aim for moderate | 20 → 10 |
| Very High (40) | Significant reduction | 40 → 15 |

**Key Principles:**
- Targets are always ≤ current complexity (never suggest increasing)
- Targets are realistic based on current state
- Moderate tier (< 10) gets preventive targets (5-6)
- Moderate tier (≥ 10) gets single-digit targets (8)
- High and Very High tiers get substantial reductions

## Effort Estimation

Effort estimates are based on:
- **Complexity reduction needed:** ~1.5 hours per 10 points of cyclomatic complexity
- **Tests to add:** ~0.2 hours (12 minutes) per test
- **Pattern complexity:** Chaotic structures add +0.5-1.0 hours

**Examples:**
- Low complexity (6/6): 0.5 hours (maintenance)
- Moderate complexity (9/16): 1-2 hours (optional refactoring)
- High complexity (20/30): 2-4 hours (required refactoring)
- Very high complexity (40/60): 4-8 hours (significant refactoring)

## Using Recommendations

### Step-by-Step Process

1. **Review the primary action** to understand what needs to be done
2. **Read the rationale** to understand why it's important
3. **Follow the implementation steps** in order
4. **Use the provided commands** as starting points
5. **Verify using the last step** (typically running tests)

### Command Interpretation

Recommendations include executable commands where possible:

```rust
cargo test function_name::     // Run tests for specific function
cargo clippy                   // Check for complexity warnings
cargo test --all               // Run full test suite
```

For language-specific recommendations, commands are adapted to your codebase language (Rust, Python, JavaScript, TypeScript).

### Integration with CI/CD

You can integrate recommendations into your workflow:

1. **Pre-commit hooks:** Check for Very High complexity functions
2. **Code review:** Include Debtmap output for context
3. **Sprint planning:** Use effort estimates for task sizing
4. **Technical debt tracking:** Track tier distribution over time

## Metrics Reference

### Cyclomatic Complexity
- Counts independent paths through code
- Each `if`, `match`, `while`, `for`, `&&`, `||` adds +1
- Threshold: < 10 (industry standard)

### Cognitive Complexity
- Measures how difficult code is to understand
- Nested structures multiply the score
- Accounts for mental load, not just paths
- Threshold: < 15 (maintainable), < 25 (acceptable)

### Nesting Depth
- Maximum levels of nested control structures
- Threshold: < 3 levels
- Deep nesting increases cognitive load exponentially

### Token Entropy
- Measures structural predictability (Shannon entropy)
- Range: 0.0 (predictable) to 1.0 (chaotic)
- Threshold: < 0.35 (consistent structure)

## Further Reading

- [Tiered Prioritization](./tiered-prioritization.md) - How Debtmap scores and prioritizes debt
- [Entropy Documentation](./entropy.md) - Understanding structural entropy
- [Output Format Guide](./output-format-guide.md) - Understanding Debtmap reports

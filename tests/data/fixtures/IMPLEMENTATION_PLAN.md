# Implementation Plan

## Target Item
**Location**: ./src/example.rs:complex_function:45
**Current Debt Score**: 85.5
**Severity**: critical

## Problem Analysis
The `complex_function` has high cognitive complexity (15) due to deeply nested conditionals (4 levels deep) and a cyclomatic complexity of 8.

## Proposed Solution
1. Extract nested conditionals into separate functions
2. Use early returns to reduce nesting depth
3. Add comprehensive unit tests

## Implementation Steps

### Step 1: Extract Helper Functions
- Create `is_all_positive(x, y, z)` helper
- Create `calculate_positive_sum(x, y, z)` helper
- Create `calculate_mixed_sum(x, y, z)` helper

### Step 2: Refactor Main Function
- Replace nested if statements with helper calls
- Use early returns for edge cases
- Simplify control flow

### Step 3: Add Tests
- Unit tests for each helper function
- Integration tests for main function
- Edge case coverage

## Success Criteria
- Cognitive complexity < 5
- Nesting depth <= 2
- Test coverage > 80%
- All existing tests still pass

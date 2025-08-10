---
number: 09
title: Fix Complexity Calculation Bugs
category: bug-fix
priority: critical
status: draft
dependencies: []
created: 2025-01-10
---

# Specification 09: Fix Complexity Calculation Bugs

**Category**: bug-fix
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Current complexity detection has critical bugs that make metrics unreliable:

1. **Cyclomatic Complexity Bug**: Functions show cyclomatic complexity of 1 despite having branches
2. **Counting Issue**: 601 files showing exactly 601 functions suggests incorrect counting
3. **Cognitive-Cyclomatic Mismatch**: Functions with cognitive complexity of 15+ but cyclomatic of 1
4. **Unrealistic Averages**: Average complexity of 1.5 is far too low for real codebases

## Objective

Fix the core complexity calculation bugs to provide accurate baseline metrics before adding enhancement features.

## Requirements

### Functional Requirements

1. **Fix Cyclomatic Complexity**
   - Count all conditional branches (if, else, elif)
   - Count all loops (for, while, do-while)
   - Count all switch cases
   - Count try-catch blocks
   - Count logical operators (&&, ||)

2. **Fix Cognitive Complexity**
   - Correctly track nesting depth
   - Apply nesting penalties properly
   - Count all cognitive complexity contributors

3. **Fix Function Counting**
   - Accurately count functions per file
   - Include all function types (methods, lambdas, closures)
   - Avoid double-counting or missing functions

### Non-Functional Requirements

1. **Correctness**: Match standard complexity definitions
2. **Performance**: Maintain current performance levels
3. **Consistency**: Same code always produces same metrics

## Acceptance Criteria

- [ ] Cyclomatic complexity matches manual calculation
- [ ] Cognitive complexity correctly reflects nesting
- [ ] Function counts are accurate per file
- [ ] Average complexity aligns with expectations (3-8 range)
- [ ] All existing tests pass
- [ ] New tests validate bug fixes

## Technical Details

### Bugs to Fix

1. **Cyclomatic Calculator** (`src/complexity/cyclomatic.rs`)
   - Not incrementing for all branch types
   - Missing logical operator counting
   - Incorrect base value calculation

2. **Cognitive Calculator** (`src/complexity/cognitive.rs`)
   - Nesting level not properly maintained
   - Nesting penalty not applied correctly
   - Missing some complexity contributors

3. **Function Counter** (`src/analyzers/`)
   - Incorrect AST traversal
   - Missing certain function types
   - File-level aggregation errors

## Dependencies

- **Affected Components**:
  - `src/complexity/cyclomatic.rs`
  - `src/complexity/cognitive.rs`
  - `src/analyzers/*` (all language analyzers)
  - `src/core/metrics.rs`

## Testing Strategy

- **Unit Tests**: Test each bug fix individually
- **Integration Tests**: Validate against known code samples
- **Regression Tests**: Ensure no new bugs introduced

## Documentation Requirements

- Document the bugs and their fixes
- Update complexity calculation documentation
- Add examples of correct calculations
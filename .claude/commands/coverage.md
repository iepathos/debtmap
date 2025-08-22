---
name: coverage
description: Improve test coverage by adding tests for the least covered file
---

# Improve Test Coverage

Analyze code coverage, identify the file with lowest coverage, and add comprehensive tests to improve coverage.

## Process

1. **Run coverage analysis** - Execute `just coverage` to generate coverage report
2. **Identify target** - Find the file with lowest test coverage from the report
3. **Analyze uncovered code** - Examine the file to understand what functionality needs testing
4. **Write tests** - Create comprehensive tests for the uncovered functionality using:
   - Functional programming patterns where appropriate
   - Pure functions when possible
   - Immutability principles
   - Property-based testing for complex logic
5. **Verify tests** - Run `cargo nextest run` to ensure all tests pass
6. **Measure improvement** - Run coverage again to verify improvement
7. **Commit changes** - Create a clear commit describing the coverage improvement

## Important Instructions

**IMPORTANT**: When making ANY commits, do NOT include attribution text like "🤖 Generated with Claude Code" or "Co-Authored-By: Claude" in commit messages. Keep commits clean and focused on the actual changes.

## Implementation Steps

1. First, I'll run the coverage analysis to identify areas needing improvement
2. Parse the coverage report JSON to find the file with lowest coverage and store initial metrics
3. Read and analyze that file to understand its functionality
4. Write comprehensive tests focusing on:
   - Critical paths and edge cases
   - Error handling scenarios
   - Boundary conditions
   - Integration with other components
5. Ensure tests follow project patterns and conventions
6. Run tests to verify they pass
7. Run coverage again to measure improvement
8. Calculate the coverage improvement percentage
9. Commit with the improvement metrics in the message

## Success Criteria

- [ ] Coverage analysis completed
- [ ] Lowest coverage file identified
- [ ] Tests written for uncovered functionality
- [ ] All tests passing
- [ ] Coverage measurably improved
- [ ] Changes committed with metrics

## Technical Approach

When writing tests, I will:
- Study existing test patterns in the codebase
- Use the same test utilities and helpers
- Follow Rust testing best practices
- Focus on behavior over implementation details
- Ensure tests are deterministic and isolated
- Add both unit and integration tests where appropriate

## Commit Message Format

The commit message will follow this format:
```
test: improve coverage for [filename] (+X%)

- Added tests for uncovered functionality
- Coverage improved from Y% to Z%
- Focus on critical paths and edge cases
```

Where X is the improvement percentage, Y is the initial coverage, and Z is the final coverage.
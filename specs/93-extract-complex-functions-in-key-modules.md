---
number: 93
title: Extract Complex Functions in Key Modules
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-09-07
---

# Specification 93: Extract Complex Functions in Key Modules

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Several key modules contain functions with high cyclomatic and cognitive complexity, contributing significantly to the technical debt score. The top offenders include `src/complexity/entropy.rs` (1,663 lines), `src/analyzers/rust.rs` (1,586 lines), and `src/priority/scoring/debt_item.rs` (2,036 lines). Functions exceeding 50 lines with nested conditionals and loops are particularly problematic. Extracting these into smaller, pure functions would reduce complexity scores by an estimated 30-50 debt points per file.

## Objective

Systematically refactor complex functions in high-impact modules by extracting them into smaller, focused, pure functions that are easier to understand, test, and maintain.

## Requirements

### Functional Requirements
- Identify all functions exceeding complexity thresholds (cyclomatic > 10, cognitive > 15)
- Extract complex logic into pure functions with single responsibilities
- Preserve all existing functionality and behavior
- Improve code readability and maintainability
- Enable better unit testing of extracted components

### Non-Functional Requirements
- Each extracted function should be under 30 lines
- Cyclomatic complexity per function should not exceed 7
- Cognitive complexity per function should not exceed 10
- Maintain or improve performance characteristics
- Follow functional programming principles where applicable

## Acceptance Criteria

- [ ] All functions with cyclomatic complexity > 10 are refactored
- [ ] No function exceeds 50 lines of code
- [ ] Average function complexity reduced by at least 40%
- [ ] All existing tests continue to pass
- [ ] New unit tests added for extracted functions
- [ ] Technical debt score reduced by at least 30 points per target file
- [ ] Code review confirms improved readability

## Technical Details

### Target Modules and Functions

1. **src/complexity/entropy.rs**
   - Primary targets: Functions calculating entropy scores
   - Extraction strategy: Separate calculation, normalization, and aggregation

2. **src/analyzers/rust.rs**
   - Primary targets: AST traversal and analysis functions
   - Extraction strategy: Separate parsing, validation, and metric calculation

3. **src/priority/scoring/debt_item.rs**
   - Primary targets: Scoring and recommendation generation
   - Extraction strategy: Extract scoring rules and recommendation logic

4. **src/config.rs**
   - Primary targets: Configuration validation and loading
   - Extraction strategy: Separate validation rules and default handling

5. **src/scoring/enhanced_scorer.rs**
   - Primary targets: Multi-factor scoring calculations
   - Extraction strategy: Extract individual factor calculations

### Refactoring Patterns

```rust
// Before: Complex monolithic function
fn calculate_complexity_score(ast: &Ast) -> Score {
    let mut score = 0.0;
    // 100+ lines of nested logic
    for node in ast.nodes() {
        if node.is_function() {
            // Complex calculation
            for child in node.children() {
                // More nesting
            }
        }
    }
    // More logic
    score
}

// After: Extracted pure functions
fn calculate_complexity_score(ast: &Ast) -> Score {
    ast.nodes()
        .filter(|node| node.is_function())
        .map(|node| calculate_node_score(node))
        .sum()
}

fn calculate_node_score(node: &Node) -> Score {
    let base_score = calculate_base_complexity(node);
    let child_score = calculate_child_complexity(node);
    combine_scores(base_score, child_score)
}

fn calculate_base_complexity(node: &Node) -> Score {
    // Focused calculation
}

fn calculate_child_complexity(node: &Node) -> Score {
    node.children()
        .map(|child| score_child_node(child))
        .sum()
}
```

### Extraction Strategies

1. **Extract Nested Loops**:
   - Convert to iterator chains with map/filter/fold
   - Extract loop bodies into separate functions
   - Use functional composition

2. **Extract Conditional Logic**:
   - Replace complex if-else with pattern matching
   - Extract condition checks into predicate functions
   - Use early returns to reduce nesting

3. **Extract Calculations**:
   - Separate data transformation from business logic
   - Create pure calculation functions
   - Extract validation logic

4. **Extract Side Effects**:
   - Isolate I/O operations
   - Separate pure computation from effects
   - Use Result/Option for error handling

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - All callers of refactored functions
  - Test suites for affected modules
- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**: Add comprehensive tests for each extracted function
- **Property Tests**: Use for pure mathematical functions
- **Integration Tests**: Ensure refactored code maintains behavior
- **Performance Tests**: Verify no performance regression
- **Complexity Metrics**: Measure before/after complexity scores

## Documentation Requirements

- **Function Documentation**: Document purpose of each extracted function
- **Refactoring Log**: Track what was extracted and why
- **Code Comments**: Add clarifying comments where needed
- **Architecture Notes**: Update if significant structural changes

## Implementation Notes

1. **Incremental Approach**:
   - Start with the highest complexity functions
   - Refactor one function at a time
   - Run tests after each extraction
   - Commit frequently

2. **Tools and Automation**:
   ```bash
   # Identify complex functions
   cargo clippy -- -W clippy::cognitive_complexity
   
   # Measure complexity before/after
   tokei --sort lines src/
   
   # Run debtmap to verify improvement
   cargo run --bin debtmap analyze src/
   ```

3. **Review Checklist**:
   - [ ] Function does one thing well
   - [ ] No deeply nested code
   - [ ] Clear input/output contract
   - [ ] Testable in isolation
   - [ ] Meaningful function names

4. **Common Extractions**:
   - Validation logic → `validate_*` functions
   - Transformations → `transform_*` functions
   - Calculations → `calculate_*` functions
   - Predicates → `is_*` or `has_*` functions

## Migration and Compatibility

- No breaking changes to public APIs
- Internal refactoring only
- Maintain function signatures where possible
- Gradual refactoring with feature flags if needed
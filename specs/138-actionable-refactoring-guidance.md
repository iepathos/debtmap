---
number: 138
title: Actionable Refactoring Guidance with Code Examples
category: optimization
priority: medium
status: draft
dependencies: [137]
created: 2025-10-27
---

# Specification 138: Actionable Refactoring Guidance with Code Examples

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 137 (call graph analysis)

## Context

Current refactoring recommendations vary wildly in actionability:

**Current Issues**:
1. **Overly Detailed**: Some recommendations have 13-step action plans
2. **Too Generic**: "Split into core/io/utils" provides no specific guidance
3. **No Code Examples**: Users don't see what "extract pure function" means
4. **Inconsistent Format**: Different issue types have different guidance styles
5. **Missing Prerequisites**: Don't explain what to do first

**Example (Current Output)**:
```
ACTION: Add 26 tests for 62% coverage gap, then refactor complexity 42 into 15 functions
  - 1. Add tests for uncovered lines: 122, 146-147, 149, 156, 160-161 and 7 more ranges
  - 2. Currently ~26 of 42 branches are uncovered (38% coverage)
  - 3. Write 21 tests to cover critical uncovered branches first
  - 4. Extract 15 pure functions from 42 branches:
  - 5.   • Group ~2 related branches per function
  - 6.   • Target complexity ≤3 per extracted function
  ... (continues for 13 steps)
```

**Problems**:
- Too many steps overwhelm the user
- No code examples showing how to extract functions
- Doesn't explain which branches to target first
- Generic patterns don't apply to specific code

## Objective

Provide concise, actionable refactoring guidance with concrete code examples that show users exactly what changes to make, prioritized by impact and difficulty.

## Requirements

### Functional Requirements

1. **Concise Action Plans**
   - Maximum 3-5 high-level steps per recommendation
   - Focus on what to do, not how to do every detail
   - Prioritize steps by impact (highest impact first)
   - Group related micro-steps into single actions

2. **Code Examples**
   - Show before/after code for common refactorings
   - Extract actual code snippets from analyzed file
   - Demonstrate pure function extraction
   - Show how to separate I/O from logic
   - Illustrate test structure for complex functions

3. **Pattern-Specific Guidance**
   - Different guidance for different complexity patterns:
     - Nested conditionals → Guard clauses + extracted predicates
     - Long parameter lists → Parameter objects
     - Multiple responsibilities → Function extraction by concern
     - Complex state management → State machines or builders
   - Detect patterns using AST analysis
   - Provide pattern-specific examples

4. **Prioritization**
   - Start with highest-impact, lowest-effort changes
   - Show estimated impact (complexity reduction, test count)
   - Indicate difficulty level (easy/medium/hard)
   - Suggest incremental approach for large refactorings

5. **Tool Integration**
   - Provide commands to run (e.g., "cargo test", "cargo clippy")
   - Suggest IDE refactoring tools when applicable
   - Generate test stubs when recommending test coverage
   - Link to relevant documentation

### Non-Functional Requirements

1. **Clarity**: Recommendations should be understandable in <60 seconds
2. **Completeness**: Provide enough detail to start work immediately
3. **Relevance**: Code examples should reflect actual analyzed code
4. **Consistency**: All issue types use similar recommendation format
5. **Performance**: Generating recommendations should not significantly slow analysis

## Acceptance Criteria

- [ ] All recommendations have 3-5 high-level steps (not 13+)
- [ ] Complex function issues include code example showing extraction
- [ ] Coverage gap issues include test stub examples
- [ ] God object issues reference specific functions from call graph analysis (Spec 137)
- [ ] Each step shows estimated impact (e.g., "-5 complexity", "+3 tests")
- [ ] Difficulty level is indicated (🟢 Easy, 🟡 Medium, 🔴 Hard)
- [ ] Pattern-specific guidance for nested conditionals, long functions, etc.
- [ ] Before/after code examples use actual function signatures when available
- [ ] Test stub generation for uncovered branches
- [ ] Commands to run are provided (e.g., cargo test --test test_name)
- [ ] Incremental refactoring path shown for large changes
- [ ] Integration test shows actionable recommendations for ripgrep

## Technical Details

### Implementation Approach

1. **Recommendation Template System**
   ```rust
   pub trait ActionableRecommendation {
       fn generate(&self, issue: &DebtIssue) -> Recommendation;
   }

   #[derive(Debug, Clone)]
   pub struct Recommendation {
       summary: String,           // One-line summary
       steps: Vec<ActionStep>,    // 3-5 high-level steps
       code_examples: Vec<CodeExample>,
       estimated_impact: Impact,
       difficulty: Difficulty,
       prerequisites: Vec<String>,
   }

   #[derive(Debug, Clone)]
   pub struct ActionStep {
       description: String,
       impact: String,           // e.g., "-10 complexity", "+5 tests"
       difficulty: Difficulty,
       substeps: Vec<String>,    // Optional detail, hidden by default
       commands: Vec<String>,    // Commands to run
   }

   #[derive(Debug, Clone)]
   pub struct CodeExample {
       title: String,
       before: String,
       after: String,
       explanation: String,
   }

   #[derive(Debug, Clone, Copy)]
   pub enum Difficulty {
       Easy,    // <30 min
       Medium,  // 30min-2hr
       Hard,    // >2hr or requires design decisions
   }

   #[derive(Debug, Clone)]
   pub struct Impact {
       complexity_reduction: i32,
       test_count_increase: usize,
       risk_reduction: f64,
   }
   ```

2. **Pattern Detection and Specific Guidance**
   ```rust
   pub fn detect_complexity_pattern(func: &syn::ItemFn) -> ComplexityPattern {
       let visitor = PatternVisitor::new();
       visitor.visit_item_fn(func);

       if visitor.nested_depth > 3 {
           ComplexityPattern::NestedConditionals {
               depth: visitor.nested_depth,
               branches: visitor.branch_count,
           }
       } else if visitor.param_count > 5 {
           ComplexityPattern::LongParameterList {
               count: visitor.param_count,
           }
       } else if visitor.responsibilities > 2 {
           ComplexityPattern::MultipleResponsibilities {
               count: visitor.responsibilities,
           }
       } else if visitor.state_mutations > 10 {
           ComplexityPattern::ComplexStateManagement {
               mutations: visitor.state_mutations,
           }
       } else {
           ComplexityPattern::Generic
       }
   }

   impl ActionableRecommendation for ComplexityPattern {
       fn generate(&self, issue: &DebtIssue) -> Recommendation {
           match self {
               ComplexityPattern::NestedConditionals { depth, branches } => {
                   generate_nested_conditional_guidance(issue, *depth, *branches)
               }
               ComplexityPattern::LongParameterList { count } => {
                   generate_parameter_object_guidance(issue, *count)
               }
               ComplexityPattern::MultipleResponsibilities { count } => {
                   generate_extraction_guidance(issue, *count)
               }
               // ... other patterns
           }
       }
   }
   ```

3. **Code Example Generation**
   ```rust
   pub fn generate_extraction_example(
       original_func: &syn::ItemFn,
       issue: &DebtIssue
   ) -> CodeExample {
       // Extract actual function signature
       let sig = &original_func.sig;

       // Find a complex conditional to extract
       let complex_condition = find_complex_conditional(original_func);

       let before = format!(
           "fn {}(...) {{\n    if {} {{\n        // Complex logic\n    }}\n}}",
           sig.ident,
           complex_condition.as_ref()
               .map(|c| format_expr(c))
               .unwrap_or("complex_condition".to_string())
       );

       let after = format!(
           "fn {}(...) {{\n    if is_valid_state(&state) {{\n        process_valid_state(state);\n    }}\n}}\n\n\
            fn is_valid_state(state: &State) -> bool {{\n    {}\n}}\n\n\
            fn process_valid_state(state: State) {{\n    // Extracted logic\n}}",
           sig.ident,
           complex_condition.as_ref()
               .map(|c| format_expr(c))
               .unwrap_or("state.is_ready() && state.has_data()".to_string())
       );

       CodeExample {
           title: "Extract Predicate Function".to_string(),
           before,
           after,
           explanation: "Extract complex conditional into a named predicate function \
                        that clearly expresses intent and can be tested independently."
               .to_string(),
       }
   }
   ```

4. **Test Stub Generation**
   ```rust
   pub fn generate_test_stubs(
       func: &syn::ItemFn,
       uncovered_branches: &[BranchInfo]
   ) -> Vec<String> {
       let func_name = &func.sig.ident;

       uncovered_branches.iter()
           .take(5) // Limit to top 5 most important
           .map(|branch| {
               let test_name = format!("test_{}_{}", func_name, branch.description.to_snake_case());

               format!(
                   "#[test]\nfn {}() {{\n    // Arrange\n    let input = todo!(\"Create input for: {}\");\n\n    \
                    // Act\n    let result = {}(input);\n\n    \
                    // Assert\n    assert!(todo!(\"Verify branch: {}\"));\n}}",
                   test_name,
                   branch.description,
                   func_name,
                   branch.description
               )
           })
           .collect()
   }
   ```

5. **Concise Step Generation**
   ```rust
   pub fn generate_concise_steps(issue: &DebtIssue) -> Vec<ActionStep> {
       match &issue.kind {
           IssueKind::ComplexFunction { complexity, coverage, .. } => {
               let pattern = detect_complexity_pattern(&issue.function);

               vec![
                   ActionStep {
                       description: format!("Add {} tests for critical uncovered branches",
                                          estimate_critical_tests(coverage)),
                       impact: format!("+{} tests, reduce risk", estimate_critical_tests(coverage)),
                       difficulty: Difficulty::Easy,
                       substeps: vec![],
                       commands: vec![format!("cargo test {}_test", issue.function.sig.ident)],
                   },
                   ActionStep {
                       description: match pattern {
                           ComplexityPattern::NestedConditionals { .. } =>
                               "Extract nested conditions into guard clauses and predicate functions".to_string(),
                           ComplexityPattern::LongParameterList { .. } =>
                               "Replace parameter list with parameter object or builder".to_string(),
                           ComplexityPattern::MultipleResponsibilities { count } =>
                               format!("Split into {} focused functions (one per responsibility)", count),
                           _ => "Extract complex logic into smaller, testable functions".to_string(),
                       },
                       impact: format!("-{} complexity, +{} functions",
                                      complexity.cyclomatic / 2,
                                      estimate_extracted_functions(complexity)),
                       difficulty: Difficulty::Medium,
                       substeps: vec![],
                       commands: vec!["cargo clippy".to_string()],
                   },
                   ActionStep {
                       description: "Verify all tests pass and complexity reduced".to_string(),
                       impact: "Confirmed improvement".to_string(),
                       difficulty: Difficulty::Easy,
                       substeps: vec![],
                       commands: vec!["cargo test".to_string(), "debtmap analyze .".to_string()],
                   },
               ]
           }
           // ... other issue types
       }
   }
   ```

### Output Format

```rust
pub fn format_recommendation(rec: &Recommendation) -> String {
    let mut output = String::new();

    // Summary
    output.push_str(&format!("ACTION: {}\n", rec.summary));
    output.push_str(&format!("IMPACT: {} | DIFFICULTY: {}\n\n",
                            format_impact(&rec.estimated_impact),
                            format_difficulty(rec.difficulty)));

    // Prerequisites (if any)
    if !rec.prerequisites.is_empty() {
        output.push_str("PREREQUISITES:\n");
        for prereq in &rec.prerequisites {
            output.push_str(&format!("  ⚠ {}\n", prereq));
        }
        output.push('\n');
    }

    // Steps
    output.push_str("STEPS:\n");
    for (i, step) in rec.steps.iter().enumerate() {
        let difficulty_icon = match step.difficulty {
            Difficulty::Easy => "🟢",
            Difficulty::Medium => "🟡",
            Difficulty::Hard => "🔴",
        };

        output.push_str(&format!("  {}. {} {}\n", i + 1, difficulty_icon, step.description));
        output.push_str(&format!("     Impact: {}\n", step.impact));

        if !step.commands.is_empty() {
            output.push_str(&format!("     Run: {}\n", step.commands.join("; ")));
        }

        output.push('\n');
    }

    // Code examples
    if !rec.code_examples.is_empty() {
        output.push_str("CODE EXAMPLE:\n");
        for example in &rec.code_examples {
            output.push_str(&format!("\n  {}\n", example.title));
            output.push_str(&format!("  {}\n\n", example.explanation));

            output.push_str("  Before:\n");
            for line in example.before.lines() {
                output.push_str(&format!("    {}\n", line));
            }

            output.push_str("\n  After:\n");
            for line in example.after.lines() {
                output.push_str(&format!("    {}\n", line));
            }
            output.push('\n');
        }
    }

    output
}
```

## Dependencies

- **Prerequisites**:
  - Spec 137: Use call graph analysis for specific function recommendations
- **Affected Components**:
  - `src/debt/recommendations.rs` - New module for recommendation generation
  - `src/debt/patterns.rs` - New module for pattern detection
  - `src/io/output.rs` - Use new recommendation format
  - `src/analysis/code_examples.rs` - New module for example generation
- **External Dependencies**:
  - `syn` (already in use) for AST analysis
  - `quote` for code generation
  - `heck` for case conversion (test name generation)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_pattern_detection() {
    let code = r#"
        fn complex_nested(x: i32, y: i32, z: i32) -> bool {
            if x > 0 {
                if y > 0 {
                    if z > 0 {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        }
    "#;

    let func = parse_function(code);
    let pattern = detect_complexity_pattern(&func);

    assert!(matches!(pattern, ComplexityPattern::NestedConditionals { depth: 3, .. }));
}

#[test]
fn test_concise_step_count() {
    let issue = create_complex_function_issue(42, 38.7);
    let steps = generate_concise_steps(&issue);

    assert!(steps.len() <= 5, "Should have at most 5 steps, got {}", steps.len());
}

#[test]
fn test_test_stub_generation() {
    let func = parse_function("fn foo(x: i32) -> bool { x > 0 }");
    let branches = vec![
        BranchInfo { description: "positive value".to_string(), line: 1 },
        BranchInfo { description: "negative value".to_string(), line: 1 },
    ];

    let stubs = generate_test_stubs(&func, &branches);

    assert_eq!(stubs.len(), 2);
    assert!(stubs[0].contains("#[test]"));
    assert!(stubs[0].contains("test_foo_positive_value"));
}

#[test]
fn test_code_example_uses_actual_signature() {
    let code = r#"
        fn process_data(config: &Config, data: Vec<u8>) -> Result<Output> {
            if config.validate {
                validate_data(&data)?;
            }
            transform(data)
        }
    "#;

    let func = parse_function(code);
    let issue = create_issue_from_function(&func);
    let example = generate_extraction_example(&func, &issue);

    assert!(example.before.contains("process_data"));
    assert!(example.after.contains("fn is_")); // Extracted predicate
}
```

### Integration Tests

```rust
#[test]
fn test_ripgrep_actionable_recommendations() {
    let issues = analyze_file("../ripgrep/crates/core/flags/hiargs.rs").unwrap();

    // Find the complex function issue (complexity 42)
    let complex_issue = issues.iter()
        .find(|i| matches!(i.kind, IssueKind::ComplexFunction { complexity, .. } if complexity.cyclomatic == 42))
        .expect("Should find complexity 42 issue");

    let recommendation = generate_recommendation(complex_issue);

    // Should have concise steps
    assert!(recommendation.steps.len() <= 5);

    // Should have code example
    assert!(!recommendation.code_examples.is_empty());

    // Should have realistic impact estimate
    assert!(recommendation.estimated_impact.complexity_reduction > 10);

    // Should use actual function name
    assert!(recommendation.code_examples[0].before.contains("from_low_args"));
}
```

### Manual Review Tests

```rust
#[test]
fn test_recommendation_readability() {
    let issue = create_complex_function_issue(42, 38.7);
    let recommendation = generate_recommendation(&issue);
    let formatted = format_recommendation(&recommendation);

    // Manual inspection criteria:
    // - Can understand in <60 seconds?
    // - Steps are actionable?
    // - Code example is clear?
    // - Impact estimate makes sense?

    // Output for manual review
    println!("{}", formatted);

    // Automated checks
    assert!(formatted.len() < 2000, "Recommendation should be concise");
    assert!(formatted.contains("STEPS:"));
    assert!(formatted.contains("CODE EXAMPLE:"));
}
```

## Documentation Requirements

### Code Documentation

- Document pattern detection algorithms
- Explain recommendation generation process
- Provide examples of each pattern type
- Document impact estimation formulas

### User Documentation

- Guide to interpreting recommendations
- Examples of following recommendations step-by-step
- Explanation of difficulty levels
- Tips for tackling hard refactorings

### Architecture Updates

Update ARCHITECTURE.md:
- Add section on recommendation generation
- Document pattern detection approach
- Explain code example generation

## Implementation Notes

### Balancing Detail and Brevity

**Too Detailed** (Current):
```
1. Add tests for uncovered lines: 122, 146-147, 149, 156, 160-161...
2. Currently ~26 of 42 branches are uncovered (38% coverage)
3. Write 21 tests to cover critical uncovered branches first
4. Extract 15 pure functions from 42 branches
... (9 more steps)
```

**Too Vague**:
```
1. Add tests
2. Refactor complexity
3. Verify
```

**Right Balance** (Target):
```
1. 🟢 Add 5-7 tests for critical uncovered branches (lines 122, 146-147, 156)
   Impact: +7 tests, reduce risk
   Run: cargo test test_from_low_args

2. 🟡 Extract complex conditionals into 3-4 focused functions
   Impact: -15 complexity, +4 functions
   Run: cargo clippy

3. 🟢 Verify improvements
   Impact: Confirmed -15 complexity
   Run: cargo test && debtmap analyze src/
```

### Pattern Library

Build library of common patterns with templates:
- Nested conditionals → Guard clauses
- Long parameter lists → Parameter objects
- Multiple responsibilities → Function extraction
- Complex state → State machines
- Error handling mess → Result chains

### Code Example Quality

Good examples:
- Use actual function names when possible
- Show realistic before/after code
- Explain the "why" not just the "what"
- Demonstrate testability improvement

Bad examples:
- Generic "foo" and "bar" functions
- Too simplified to be useful
- No explanation of benefit

### Incremental Refactoring Paths

For large refactorings:
```
Phase 1 (Easy): Extract 2-3 most obvious functions
  Impact: -8 complexity
  Time: 30 min

Phase 2 (Medium): Separate I/O from logic
  Impact: -10 complexity, improve testability
  Time: 1-2 hours

Phase 3 (Hard): Restructure control flow
  Impact: -15 complexity, clearer intent
  Time: 2-4 hours
```

## Migration and Compatibility

### Breaking Changes

- Output format changes significantly
- JSON structure for recommendations will change

### Backward Compatibility

- Add `--legacy-format` flag for old style
- JSON output includes both old and new format initially
- Deprecate old format after 2 releases

## Success Metrics

- Average recommendation has 3-5 steps (not 13+)
- >80% of recommendations include code examples
- User survey shows >70% find recommendations actionable
- Time to implement recommendations decreases by >30%
- Code example relevance rated >4/5 by users
- Pattern detection accuracy >75% on validation corpus

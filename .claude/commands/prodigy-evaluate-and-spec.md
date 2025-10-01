# /prodigy-evaluate-and-spec

Perform a comprehensive evaluation of Prodigy's current functionality, identify technical debt, implementation gaps, and areas for improvement, then automatically generate detailed specifications for each identified issue.

## Variables

None required - this command performs a full system evaluation

## Execute

### Phase 0: Vision and Goals Alignment

1. **Review Project Vision**
   - Read VISION.md to understand core goals and non-goals
   - Understand design principles and success metrics
   - Review feature priorities (Must Have, Should Have, Nice to Have)
   - Note explicitly stated non-goals to avoid
   - Use vision as north star for all evaluations

2. **Evaluation Context**
   Evaluate against these core principles from VISION.md:
   - **Simplicity First**: Single-machine excellence before distributed complexity
   - **Reliability Above All**: Zero data loss, graceful degradation
   - **Developer Experience**: 5-minute onboarding, self-documenting
   - **Functional Programming**: Immutability, pure functions, composition
   - **Pragmatic Automation**: Human in the loop, transparent operation

3. **Success Metrics to Check**
   From VISION.md success metrics:
   - New user to productive in < 5 minutes
   - Zero panics in production code
   - < 20 lines per function average
   - < 2 minute build time
   - < 20 MB binary size

### Phase 1: Comprehensive Codebase Analysis

1. **Structural Analysis**
   - Analyze directory structure and module organization
   - Map component dependencies and relationships
   - Identify architectural patterns and anti-patterns
   - Assess code organization and separation of concerns
   - Review module boundaries and interfaces

2. **Deprecated Code Detection**
   - Search for deprecated commands, aliases, and parameters
   - Find TODO, FIXME, XXX, HACK comments
   - Identify commented-out code blocks
   - Look for legacy compatibility layers
   - Find unused feature flags and conditionals

3. **Dependency Analysis**
   - Audit all dependencies in Cargo.toml
   - Identify unused dependencies with cargo-udeps
   - Find duplicate functionality across dependencies
   - Analyze dependency tree depth and complexity
   - Check for security vulnerabilities in dependencies

4. **Code Quality Metrics**
   - Count unwrap() and panic!() calls
   - Analyze error handling patterns
   - Measure cyclomatic complexity
   - Check function and module sizes
   - Identify code duplication

### Phase 2: Implementation Gap Analysis

1. **Feature Completeness Review**
   - Compare implemented features vs documentation claims
   - Identify partially implemented features
   - Find stub functions and placeholder code
   - Review test coverage for critical paths
   - Check for missing error cases

2. **Architecture Consistency**
   - Identify duplicate implementations of similar functionality
   - Find inconsistent patterns across modules
   - Detect abstraction leaks
   - Review interface consistency
   - Check for proper separation of concerns

3. **Performance Analysis**
   - Identify potential performance bottlenecks
   - Find unnecessary allocations and clones
   - Check for inefficient algorithms
   - Review async/await usage patterns
   - Analyze resource management

4. **Storage and State Management**
   - Review storage implementations and redundancy
   - Analyze session management complexity
   - Check for state consistency issues
   - Review persistence patterns
   - Identify potential data races

### Phase 3: Issue Categorization and Prioritization

1. **Issue Categories**
   ```
   CRITICAL: Data loss risk, crashes, security issues
   HIGH: Major functionality gaps, severe tech debt
   MEDIUM: Performance issues, code quality problems
   LOW: Minor improvements, nice-to-have features
   ```

2. **Evaluation Criteria**
   - User impact severity
   - Implementation complexity
   - Risk of regression
   - Maintenance burden
   - Security implications
   - Performance impact

3. **Priority Matrix**
   ```
   High Impact + Low Effort = Do First
   High Impact + High Effort = Do Next
   Low Impact + Low Effort = Quick Wins
   Low Impact + High Effort = Defer/Skip
   ```

### Phase 4: Issue Documentation

For each identified issue, document:

1. **Issue Summary**
   - Clear description of the problem
   - Current behavior vs expected behavior
   - Impact on users and developers
   - Root cause analysis

2. **Evidence Collection**
   - Code locations and line numbers
   - Specific examples from codebase
   - Metrics and measurements
   - Test failures or gaps

3. **Proposed Solution**
   - High-level approach
   - Alternative solutions considered
   - Implementation complexity estimate
   - Risk assessment

### Phase 5: Specification Generation

1. **Determine Spec Requirements**
   - Analyze existing specs to find highest number
   - Group related issues if they should be addressed together
   - Separate unrelated issues into individual specs
   - Establish implementation dependencies

2. **Generate Specifications**
   For each issue identified:
   ```markdown
   ---
   number: {next_available_number}
   title: {descriptive_title}
   category: {foundation|optimization|testing|compatibility}
   priority: {critical|high|medium|low}
   status: draft
   dependencies: [{related_spec_numbers}]
   created: {current_date}
   ---

   # Specification {number}: {title}

   ## Context
   {background_and_problem_description}

   ## Objective
   {clear_goal_statement}

   ## Requirements
   ### Functional Requirements
   - {specific_requirements}

   ### Non-Functional Requirements
   - {performance_security_usability}

   ## Acceptance Criteria
   - [ ] {measurable_criteria}

   ## Technical Details
   {implementation_approach}

   ## Dependencies
   {prerequisites_and_affected_components}

   ## Testing Strategy
   {test_approach}

   ## Documentation Requirements
   {required_documentation_updates}
   ```

3. **Specification Categories**
   - **Foundation**: Core architecture, error handling, storage
   - **Optimization**: Performance, dependency reduction, code cleanup
   - **Testing**: Test coverage, test infrastructure, CI/CD
   - **Compatibility**: Breaking changes, migrations, upgrades

### Phase 6: Evaluation Report Generation

1. **Create Comprehensive Report**
   ```markdown
   # Prodigy Technical Evaluation Report

   ## Executive Summary
   {high_level_findings}

   ## Metrics Summary
   - Total Issues Found: {count}
   - Critical Issues: {count}
   - Lines of Code: {count}
   - Technical Debt Score: {score}

   ## Critical Issues
   {detailed_critical_issues}

   ## High Priority Improvements
   {high_priority_list}

   ## Technical Debt Analysis
   {debt_categories_and_impact}

   ## Recommendations
   {prioritized_action_items}

   ## Generated Specifications
   {list_of_created_specs}
   ```

### Phase 7: Git Commit Process

1. **Stage All Generated Files**
   - Stage evaluation report
   - Stage all new specification files
   - Verify all files are properly formatted

2. **Create Descriptive Commit**
   ```
   add: technical evaluation and improvement specs {first_num}-{last_num}

   Comprehensive evaluation identified {total} issues:
   - {critical_count} critical issues requiring immediate attention
   - {high_count} high priority improvements
   - {medium_count} medium priority enhancements
   - {low_count} low priority optimizations

   Generated specifications:
   - Spec {num}: {title} (priority: {level})
   [list each spec]

   See PRODIGY_EVALUATION_REPORT.md for full analysis.
   ```

   **IMPORTANT**: Never add Claude attribution or emoji to git commits. Keep commits clean and professional.

## Code Quality Standards

### Idiomatic Rust Requirements
All generated specifications should promote:
- **Ownership and Borrowing**: Proper use of Rust's ownership system
- **Error Handling**: Use `Result<T, E>` instead of panics
- **Pattern Matching**: Exhaustive matching over if-else chains
- **Iterator Chains**: Prefer iterators over manual loops
- **Type Safety**: Leverage Rust's type system for correctness
- **Zero-Cost Abstractions**: Use abstractions that compile to efficient code

### Functional Programming Best Practices
Specifications should emphasize:
- **Immutability by Default**: Avoid mutable state where possible
- **Pure Functions**: Functions without side effects for business logic
- **Function Composition**: Build complex behavior from simple functions
- **Higher-Order Functions**: Use map, filter, fold instead of loops
- **Separation of Concerns**: Pure core with imperative shell pattern
- **Data Transformation Pipelines**: Chain operations over mutation
- **Algebraic Data Types**: Use enums for modeling domain logic

### Anti-Patterns to Identify
Look for and create specs to fix:
- **Imperative loops** that should be iterator chains
- **Mutable accumulation** that should be fold/reduce
- **Side effects in business logic** that should be pure
- **Complex conditionals** that should be pattern matching
- **Shared mutable state** that should be message passing
- **Object-oriented patterns** that should be functional
- **Inheritance hierarchies** that should be composition

### Example Transformations
```rust
// Bad: Imperative with mutation
let mut result = vec![];
for item in items {
    if item.is_valid() {
        result.push(item.transform());
    }
}

// Good: Functional with iterators
let result: Vec<_> = items
    .into_iter()
    .filter(|item| item.is_valid())
    .map(|item| item.transform())
    .collect();

// Bad: Mutable state accumulation
let mut sum = 0;
for value in values {
    sum += value;
}

// Good: Fold/reduce
let sum = values.iter().sum::<i32>();

// Bad: Side effects mixed with logic
fn process_data(data: &Data) -> Result<Output> {
    log::info!("Processing data");
    let result = transform(data)?;
    database.save(&result)?;
    send_notification(&result);
    Ok(result)
}

// Good: Pure core with separated I/O
fn transform_data(data: &Data) -> Result<Output> {
    transform(data) // Pure function
}

fn process_data(data: &Data, ctx: &Context) -> Result<Output> {
    let result = transform_data(data)?; // Pure
    ctx.save(&result)?;                 // I/O at boundary
    ctx.notify(&result);                // I/O at boundary
    Ok(result)
}
```

## Evaluation Checklist

### Code Quality Issues to Detect
- [ ] Unwrap/panic usage in production code
- [ ] Missing error handling
- [ ] Inconsistent error types
- [ ] Code duplication
- [ ] Dead code
- [ ] Overly complex functions
- [ ] Poor test coverage
- [ ] Missing documentation

### Architectural Issues to Identify
- [ ] Duplicate implementations
- [ ] Inconsistent patterns
- [ ] Tight coupling
- [ ] Abstraction leaks
- [ ] Circular dependencies
- [ ] God objects/modules
- [ ] Missing interfaces
- [ ] Poor separation of concerns

### Performance Issues to Find
- [ ] Unnecessary allocations
- [ ] Inefficient algorithms
- [ ] Blocking I/O in async code
- [ ] Resource leaks
- [ ] Missing caching
- [ ] Redundant computations
- [ ] Large binary size
- [ ] Slow build times

### Maintenance Issues to Detect
- [ ] Deprecated dependencies
- [ ] Unused dependencies
- [ ] Outdated patterns
- [ ] Technical debt
- [ ] Missing tests
- [ ] Unclear code
- [ ] Magic numbers/strings
- [ ] Hardcoded values

## Example Issues and Specifications

### Example 1: Duplicate Storage Systems
**Issue**: Three parallel storage implementations
**Spec Generated**: "Consolidate Storage Systems"
**Priority**: Critical
**Solution**: Unify to single global storage

### Example 2: Poor Error Handling
**Issue**: 140+ unwrap() calls
**Spec Generated**: "Fix Critical Unwrap Calls"
**Priority**: Critical
**Solution**: Replace with proper Result handling

### Example 3: Unused Dependencies
**Issue**: 40% of dependencies unused
**Spec Generated**: "Remove Unused Dependencies"
**Priority**: High
**Solution**: Audit and remove unnecessary deps

### Example 4: Complex MapReduce State
**Issue**: Overly complex state machine
**Spec Generated**: "Simplify MapReduce State Machine"
**Priority**: Medium
**Solution**: Refactor to simpler design

## Output Files

The command generates:
1. `PRODIGY_EVALUATION_REPORT.md` - Full evaluation report
2. `specs/{number}-{title}.md` - Individual specification for each issue
3. Git commit with all changes

## Success Criteria

The evaluation is successful when:
- All major issues are identified and documented
- Specifications are generated for each actionable issue
- Issues are properly prioritized
- Implementation dependencies are established
- Report provides clear improvement roadmap
- All files are committed to git

## Important Constraints from Vision

### Never Create Specs For
Based on VISION.md non-goals, never create specifications for:
- Distributed execution (until single-machine is perfect)
- Database storage (until file storage proves inadequate)
- Kubernetes/container orchestration (premature optimization)
- Microservices architecture (keep monolith)
- Complex abstractions (prefer simple solutions)
- General AI framework features
- IDE/editor functionality
- Version control features beyond Git integration

### Always Prioritize
When creating specs, always favor:
- Removing complexity over adding features
- File-based solutions over databases
- Single-machine optimization over distribution
- Functional patterns over OOP
- Explicit behavior over magic
- Boring solutions that work over clever tricks

## Notes

- Focus on actionable issues with clear solutions
- Avoid speculative or "nice to have" improvements
- Prioritize simplification over adding features
- Consider implementation effort vs benefit
- Group related issues when they share solutions
- Keep specifications focused and achievable
- Don't create specs for issues already being addressed
- Always check against VISION.md before suggesting improvements
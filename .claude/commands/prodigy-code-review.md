# /prodigy-code-review

Conduct a comprehensive code review of the current project or specified components. This command performs static analysis, identifies potential issues, ensures code quality standards are met, and provides detailed feedback on implementation patterns.

## Variables

SCOPE: $ARGUMENTS (optional - specify scope like "src/parser", "tests", specific files, or omit for entire codebase)

## Execute

### Phase 1: Project Analysis

1. **Read Project Context**
   - Read .prodigy files (PROJECT.md, ARCHITECTURE.md, etc.)
   - Understand project goals, architecture patterns, and coding standards
   - Identify recently completed specifications from ROADMAP.md

2. **Determine Review Scope**
   - If SCOPE specified: Focus on specific files/directories
   - If no SCOPE: Review recent changes since last major commit
   - Prioritize areas with recent modifications or new implementations
   - Check git status for uncommitted changes

### Phase 2: Static Analysis

1. **Code Quality Checks**
   - Run `cargo check` for compiler warnings
   - Execute `cargo clippy` with extra attention to architecture violation locations

2. **Structure Review**
   - **Dependency analysis**: Compare actual dependencies against dependency_graph.json
   - **Coupling review**: Focus on modules with high coupling_scores
   - **Circular dependencies**: Validate cycles identified in dependency_graph.json
   - **Architecture compliance**: Cross-reference code against architecture.json patterns
   - **API consistency**: Use conventions.json naming_patterns for validation

3. **Security and Safety Analysis**
   - Identify unsafe code blocks and validate their necessity
   - Review error handling patterns and panic conditions
   - Check for potential security vulnerabilities
   - Analyze input validation and boundary conditions
   - Review memory safety and resource management

### Phase 3: Implementation Quality Assessment

1. **Code Pattern Analysis**
   - Review Rust idioms and best practices usage
   - Check proper use of ownership, borrowing, and lifetimes
   - Analyze iterator usage vs manual loops
   - Validate error handling patterns (Result vs Option)
   - Review async/await patterns if applicable

2. **Performance Considerations**
   - Identify potential performance bottlenecks
   - Review allocation patterns and unnecessary clones
   - Check for inefficient algorithms or data structures
   - Analyze hot paths and optimization opportunities
   - Review concurrent code for race conditions

3. **Maintainability Review**
   - Assess code readability and complexity
   - Review naming conventions and documentation
   - Check for code duplication and refactoring opportunities
   - Analyze function and module size and cohesion
   - Review test coverage and quality

### Phase 4: Architecture and Design Review

1. **Design Pattern Compliance**
   - Verify adherence to project architecture from ARCHITECTURE.md
   - Check design pattern implementation (Factory, Builder, etc.)
   - Review abstraction levels and interface design
   - Validate dependency injection and inversion patterns

2. **API Design Review**
   - Analyze public interface consistency and usability
   - Review method signatures and return types
   - Check for breaking changes in public APIs
   - Validate documentation completeness for public APIs

3. **Integration Points**
   - Review external dependency usage
   - Check integration patterns with other modules
   - Analyze database/storage interaction patterns
   - Review configuration and environment handling

### Phase 5: Testing and Documentation Review

1. **Test Quality Assessment**
   - Review test coverage using `cargo tarpaulin`
   - Identify untested critical functions
   - Check for adequate test assertions
   - Validate test naming and organization
   - Ensure edge cases are covered

2. **Documentation Review**
   - Check inline documentation (rustdoc) completeness
   - Review code comments for clarity and necessity
   - Validate README and project documentation
   - Check for outdated or incorrect documentation
   - Review example code in documentation

### Phase 6: Specification Compliance

1. **Requirements Verification**
   - Cross-reference implementation against specifications
   - Verify all acceptance criteria are met
   - Check for feature completeness and correctness
   - Validate business logic implementation

2. **Convention Adherence**
   - Verify compliance with CONVENTIONS.md standards
   - Check consistent code style across the project
   - Review naming conventions and structure
   - Validate commit message format and git practices

### Phase 7: Recommendations and Action Items

1. **Issue Categorization**
   **Severity levels**:
   - **Critical**: Security vulnerabilities, data corruption risks, crashes
   - **High**: Significant bugs, performance issues, architecture violations
   - **Medium**: Code quality issues, missing tests, documentation gaps
   - **Low**: Style issues, minor improvements, refactoring opportunities

2. **Improvement Suggestions**
   - Specific code refactoring recommendations
   - Architecture improvements
   - Test coverage enhancements
   - Documentation updates
   - Performance optimizations
   - Security hardening

3. **Action Plan**
   - Prioritized list of issues to address
   - Issues ordered by severity and impact
   - Suggested implementation order
   - Potential breaking changes to consider
   - Long-term architectural considerations

### Phase 8: Temporary Specification Generation & Git Commit

**CRITICAL FOR AUTOMATION**: When running in automation mode, generate a temporary specification file containing actionable implementation instructions for the issues found, then commit it.

1. **Spec File Creation**
   - Create directory: `specs/` if it doesn't exist
   - Generate filename: `iteration-{timestamp}-improvements.md`
   - Write comprehensive implementation spec

2. **Spec Content Requirements**
   ```markdown
   # Iteration {N}: Code Quality Improvements
   
   ## Overview
   Temporary specification for code improvements identified in automated review.
   
   ## Issues to Address
   
   ### 1. {Issue Title}
   **Severity**: {severity}
   **Category**: {category}
   **File**: {file_path}
   **Line**: {line_number}
   
   #### Current Code:
   ```{language}
   {actual_problematic_code}
   ```
   
   #### Required Change:
   ```{language}
   {improved_code_example}
   ```
   
   #### Implementation Notes:
   - {specific_instruction_1}
   - {specific_instruction_2}
   
   ## Success Criteria
   - [ ] {specific_criterion_1}
   - [ ] {specific_criterion_2}
   - [ ] All files compile without warnings
   - [ ] Tests pass
   ```

3. **Include Actual Code Examples**
   - Read the problematic code from files
   - Show exact current code that needs changing
   - Provide specific improved code examples
   - Include necessary imports/dependencies

4. **Actionable Instructions**
   - Each issue must have specific, implementable instructions
   - Include file paths, line numbers, exact changes
   - Provide context for why changes are needed
   - Include validation steps

5. **Git Commit (Required for automation)**
   - Stage the created spec file: `git add specs/temp/iteration-{timestamp}-improvements.md`
   - Commit with message: `review: generate improvement spec for iteration-{timestamp}-improvements`
   - **IMPORTANT**: Do NOT add any attribution text like "🤖 Generated with [Claude Code]" or "Co-Authored-By: Claude" to commit messages. Keep commits clean and focused on the change itself.
   - If no issues found, do not create spec or commit

## Review Criteria

### Code Quality Standards
- **Correctness**: Code works as intended without bugs
- **Readability**: Code is clear and self-documenting
- **Maintainability**: Code is easy to modify and extend
- **Performance**: Code meets performance requirements
- **Security**: Code is free from security vulnerabilities

### Rust-Specific Criteria
- **Memory Safety**: Proper ownership and borrowing patterns
- **Error Handling**: Comprehensive Result/Option usage
- **Concurrency**: Safe concurrent code patterns
- **Idioms**: Proper use of Rust language features
- **Dependencies**: Appropriate crate selection and usage

### Architecture Compliance
- **Modularity**: Proper separation of concerns
- **Abstraction**: Appropriate abstraction levels
- **Dependencies**: Clean dependency management
- **Interfaces**: Well-designed public APIs
- **Patterns**: Consistent design pattern usage

## Automation Mode Behavior

**Automation Detection**: The command detects automation mode when:
- Environment variable `PRODIGY_AUTOMATION=true` is set
- Called from within a Prodigy workflow context

**Git-Native Automation Flow**:
1. Analyze code and identify issues
2. If issues found: Create temporary spec file and commit it
3. If no issues found: Report "No issues found" and exit without creating commits
4. Always provide a brief summary of actions taken

**Output Format in Automation Mode**:
- Minimal console output focusing on key actions
- Clear indication of whether spec was created and committed
- Brief summary of issues found (if any)
- No JSON output required

**Example Automation Output**:
```
✓ Code review completed
✓ Found 3 issues requiring attention
✓ Generated spec: iteration-1708123456-improvements.md
✓ Committed: review: generate improvement spec for iteration-1708123456-improvements
```

**Example No Issues Output**:
```
✓ Code review completed  
✓ No issues found - code quality is good
```

## Output Format

1. **Executive Summary**
   - Overall code quality assessment
   - Critical issues requiring immediate attention
   - General recommendations for improvement

2. **Detailed Findings**
   - File-by-file analysis with specific issues
   - Code snippets with suggested improvements
   - Cross-references to relevant specifications

3. **Metrics and Statistics**
   - Code complexity metrics
   - Test coverage statistics
   - Performance benchmark results
   - Lint and warning counts

4. **Integration with prodigy improve**
   - In automation mode: Creates and commits temporary spec files
   - `prodigy improve` will extract spec from git commits and apply fixes
   - Creates a complete audit trail through git history

## Integration with Development Workflow

### Pre-merge Reviews
- Validate changes before merging to main branch
- Ensure new code meets quality standards
- Check for regressions and breaking changes
- Verify specification compliance

### Regular Health Checks
- Periodic comprehensive reviews
- Architecture drift detection
- Technical debt assessment
- Performance regression monitoring

### Specification Validation
- Post-implementation specification reviews
- Acceptance criteria verification
- Documentation synchronization
- Context file updates validation

## Example Usage

```
/prodigy-code-review
/prodigy-code-review "src/parser"
/prodigy-code-review "src/parser/inventory.rs src/parser/manifest.rs"
/prodigy-code-review "tests"
```

## Advanced Features

### Git Integration
- Focus review on recent commits or specific commit ranges
- Compare implementation against previous versions
- Identify code churn and stability metrics
- Track technical debt over time

### Custom Review Profiles
- Different review criteria for different project phases
- Specialized reviews for performance-critical code
- Security-focused reviews for sensitive components
- API stability reviews for public interfaces

### Automated Suggestions
- Generate specific code improvement suggestions
- Provide refactoring recommendations with examples
- Suggest performance optimizations
- Recommend additional test cases

## Quality Gates

### Minimum Standards
- All code must compile without warnings
- Critical clippy lints must be addressed
- All tests must pass consistently
- Public APIs must be documented

### Best Practice Enforcement
- Follow established architecture patterns
- Maintain consistent error handling
- Use appropriate Rust idioms
- Maintain good test coverage

### Continuous Improvement
- Track code quality metrics over time
- Identify recurring issue patterns
- Suggest process improvements
- Monitor architectural health

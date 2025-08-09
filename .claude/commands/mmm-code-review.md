# /mmm-code-review

Conduct a comprehensive code review of the current project or specified components. This command performs static analysis, identifies potential issues, ensures code quality standards are met, and provides detailed feedback on implementation patterns.

## Variables

SCOPE: $ARGUMENTS (optional - specify scope like "src/parser", "tests", specific files, or omit for entire codebase)
MMM_CONTEXT_AVAILABLE: Environment variable indicating .mmm context availability
MMM_CONTEXT_DIR: Path to .mmm/context/ directory with analysis data

## Execute

### Phase 1: Context-Driven Project Analysis

1. **Load MMM Analysis Context (Priority)**
   - Check if `MMM_CONTEXT_AVAILABLE=true` and load context data
   - Read `.mmm/context/technical_debt.json` for known issues and hotspots
   - Load `.mmm/context/architecture.json` for violations and patterns
   - Parse `.mmm/context/conventions.json` for style violations
   - Review `.mmm/context/test_coverage.json` for coverage gaps
   - Check `.mmm/metrics/current.json` for quality baselines
   - Use `.mmm/context/dependency_graph.json` for coupling analysis

2. **Fallback to Traditional Context**
   - If context unavailable, read .mmm files (PROJECT.md, ARCHITECTURE.md, etc.)
   - Understand project goals, architecture patterns, and coding standards
   - Identify recently completed specifications from ROADMAP.md

3. **Context-Aware Priority Processing**
   - **Use context data to prioritize review areas**:
     - API violations from architecture.json + error handling debt
     - Hotspots with high complexity_score + benchmark regressions
     - Security-tagged debt_items + unsafe code patterns
     - Critical_gaps and untested_functions from context
     - Low doc_coverage areas from metrics
   - **Context amplification**: Use debt_items tags to identify critical issues

3. **Determine Review Scope**
   - If SCOPE specified: Focus on specific files/directories
   - If no SCOPE: Review recent changes since last major commit
   - Prioritize areas with recent modifications or new implementations
   - Check git status for uncommitted changes

### Phase 2: Context-Enhanced Static Analysis

1. **Context-Guided Quality Checks**
   - **Pre-analyze with context**: Skip areas already identified as clean in technical_debt.json
   - **Target hotspots**: Focus clippy analysis on files with high complexity_score
   - **Convention-driven**: Use conventions.json violations to guide formatting checks
   - Run `cargo check` and compare warnings against known debt_items
   - Execute `cargo clippy` with extra attention to architecture violation locations

2. **Context-Aware Structure Review**
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

### Phase 5: Context-Driven Testing and Documentation Review

1. **Context-Enhanced Test Quality Assessment**
   - **Coverage targeting**: Focus on untested_functions from test_coverage.json
   - **Critical gaps**: Prioritize critical_gaps with high risk assessment
   - **Hotspot testing**: Ensure hotspots with high change_frequency have adequate tests
   - **Metrics-driven**: Compare actual coverage against file_coverage percentages
   - **Benchmark validation**: Cross-reference with benchmark_results from metrics

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

### Phase 7: Context-Driven Recommendations and Action Items

1. **Context-Enhanced Issue Categorization**
   **Base severity from context data**:
   - **Critical**: debt_items with impact >= 8, security-tagged items, hotspots with risk_level="High"
   - **High**: debt_items with impact >= 6, architecture violations severity="High", critical_gaps
   - **Medium**: debt_items with impact >= 4, convention violations, moderate complexity hotspots
   - **Low**: debt_items with impact < 4, style issues, documentation gaps
   
   **Priority amplification**:
   - Issues with matching debt_items tags get priority boost
   - Context provides specific locations and impact scores for precise prioritization
   - Example: Hotspots with complexity_score > 15 = Critical
   - Example: Debt_items tagged "security" = Critical

2. **Context-Informed Improvement Suggestions**
   - **Debt-specific**: Use exact debt_items descriptions and locations for targeted fixes
   - **Hotspot-driven**: Target complexity hotspots with specific refactoring approaches
   - **Duplication elimination**: Address similarity mappings from duplication_map
   - **Convention alignment**: Fix specific violations from conventions.json
   - **Coverage improvements**: Target untested_functions and critical_gaps precisely
   - **Architecture compliance**: Address specific violations with suggested patterns
   
   **Context-enhanced handling**:
   - Use debt_items tags to identify critical issues automatically
   - Leverage metrics trends to show performance/quality impacts
   - Provide specific file:line references from context data

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
- Environment variable `MMM_AUTOMATION=true` is set
- Called from within an MMM workflow context

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

4. **Integration with mmm improve**
   - In automation mode: Creates and commits temporary spec files
   - `mmm improve` will extract spec from git commits and apply fixes
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
/mmm-code-review
/mmm-code-review "src/parser"
/mmm-code-review "src/parser/inventory.rs src/parser/manifest.rs"
/mmm-code-review "tests"
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

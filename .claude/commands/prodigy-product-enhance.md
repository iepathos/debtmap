# /prodigy-product-enhance

Analyze the codebase from a product management perspective, focusing on user value, feature opportunities, and user experience enhancements rather than code quality metrics. This command identifies improvements that would enhance the product's value proposition and user satisfaction.

## Variables

SCOPE: $ARGUMENTS (optional - specify scope like "cli", "api", specific features, or omit for entire product)
FOCUS: $PRODIGY_FOCUS (optional - focus directive from prodigy CLI, e.g., "onboarding", "api", "cli-ux")

## Execute

### Phase 1: Product Context and User Understanding

1. **Read Project Context**
   - Read .prodigy context files (PROJECT.md, ARCHITECTURE.md, ROADMAP.md)
   - Understand product goals, user personas, and value proposition
   - Identify core features and their intended use cases
   - Review completed and planned features from ROADMAP.md

2. **Parse Focus Directive (if provided)**
   - If FOCUS environment variable is set, interpret the product area
   - Adjust analysis priorities based on focus:
     - "onboarding" → First-run experience, tutorials, getting started
     - "api" → Developer experience, API design, integration points
     - "cli-ux" → Command ergonomics, help text, error messages
     - "documentation" → User guides, examples, API documentation
     - "features" → Missing functionality, partial implementations
   - Note: Focus affects prioritization, not what is analyzed

3. **Determine Analysis Scope**
   - If SCOPE specified: Focus on specific features/components
   - If no SCOPE: Analyze entire product experience
   - Prioritize user-facing components and interfaces
   - Consider the complete user journey

### Phase 2: User Experience Analysis

1. **Command Line Interface Review**
   - Analyze CLI ergonomics and discoverability
   - Review help text clarity and completeness
   - Check error messages for helpfulness and actionability
   - Evaluate command consistency and predictability
   - Assess output formatting and verbosity options

2. **First-Run Experience**
   - Evaluate onboarding flow for new users
   - Check for sensible defaults and zero-config operation
   - Review initial setup requirements and complexity
   - Analyze getting started documentation
   - Identify friction points in initial usage

3. **Error Handling and Recovery**
   - Review error messages from user perspective
   - Check for actionable error guidance
   - Evaluate failure recovery mechanisms
   - Analyze edge case handling
   - Review progress indicators for long operations

### Phase 3: Feature Completeness Assessment

1. **Core Feature Analysis**
   - Identify partially implemented features
   - Find missing complementary features
   - Evaluate feature discoverability
   - Check for feature consistency
   - Analyze feature integration points

2. **User Workflow Support**
   - Map common user workflows
   - Identify workflow gaps or inefficiencies
   - Check for automation opportunities
   - Evaluate batch operation support
   - Review pipeline integration capabilities

3. **Integration Opportunities**
   - Analyze third-party tool integration needs
   - Review ecosystem compatibility
   - Check for standard format support (JSON, YAML, etc.)
   - Evaluate CI/CD integration features
   - Identify plugin or extension points

### Phase 4: Developer Experience (if applicable)

1. **API Design Review**
   - Evaluate API consistency and predictability
   - Review naming conventions and clarity
   - Check for comprehensive examples
   - Analyze error responses and codes
   - Review versioning and compatibility

2. **Documentation Quality**
   - Assess documentation completeness
   - Review example quality and coverage
   - Check for common use case coverage
   - Evaluate troubleshooting guides
   - Analyze API reference completeness

3. **Developer Tooling**
   - Review debugging capabilities
   - Check for development aids (dry-run, verbose modes)
   - Evaluate testing support features
   - Analyze performance profiling options
   - Review configuration flexibility

### Phase 5: Performance from User Perspective

1. **Perceived Performance**
   - Identify operations that feel slow to users
   - Check for missing progress indicators
   - Evaluate feedback mechanisms
   - Analyze startup time impact
   - Review resource usage visibility

2. **Workflow Efficiency**
   - Identify repetitive tasks that could be automated
   - Check for batch operation support
   - Evaluate caching effectiveness
   - Analyze command shortcuts and aliases
   - Review configuration persistence

### Phase 6: Product Enhancement Opportunities

1. **Feature Prioritization**
   - High impact, low effort improvements
   - User-requested features (from issues/feedback)
   - Competitive feature analysis
   - Platform-specific enhancements
   - Accessibility improvements

2. **User Experience Improvements**
   - UI/UX consistency enhancements
   - Workflow streamlining opportunities
   - Error message improvements
   - Documentation gaps
   - Onboarding enhancements

3. **Value Addition Opportunities**
   - New use case support
   - Integration possibilities
   - Automation features
   - Collaboration features
   - Monitoring and observability

### Phase 7: Temporary Specification Generation & Git Commit

**CRITICAL FOR AUTOMATION**: Generate a temporary specification file containing product enhancement proposals, then commit it.

1. **Spec File Creation**
   - Create directory: `specs/` if it doesn't exist
   - Generate filename: `iteration-{timestamp}-product-enhancements.md`
   - Write comprehensive enhancement spec

2. **Spec Content Requirements**
   ```markdown
   # Iteration {N}: Product Enhancements
   
   ## Overview
   Temporary specification for product enhancements identified from user perspective.
   {IF FOCUS: "Focus area: {FOCUS}"}
   
   ## Enhancement Proposals
   {IF FOCUS: Prioritize enhancements related to focus area first}
   
   ### 1. {Enhancement Title}
   **Impact**: {High/Medium/Low}
   **Effort**: {Small/Medium/Large}
   **Category**: {UX/Feature/Integration/Documentation}
   **Component**: {affected_component}
   
   #### User Story
   As a {user_type}, I want to {goal} so that {benefit}.
   
   #### Current State
   {description_of_current_limitation}
   
   #### Proposed Enhancement
   {detailed_enhancement_description}
   
   #### Implementation Approach
   - {implementation_step_1}
   - {implementation_step_2}
   
   #### Success Metrics
   - {how_to_measure_success}
   
   ## Success Criteria
   - [ ] {specific_criterion_1}
   - [ ] {specific_criterion_2}
   - [ ] User documentation updated
   - [ ] Examples provided
   ```

3. **User-Centric Focus**
   - Frame all enhancements as user stories
   - Include concrete use cases
   - Provide before/after examples
   - Focus on user value, not technical implementation

4. **Actionable Proposals**
   - Each enhancement must be implementable
   - Include specific user benefits
   - Provide clear success metrics
   - Consider backward compatibility

5. **Git Commit (Required for automation)**
   - Stage the created spec file: `git add specs/temp/iteration-{timestamp}-product-enhancements.md`
   - Commit with message: `product: enhance {primary_focus} for iteration-{timestamp}`
   - If no enhancements identified, do not create spec or commit

## Analysis Criteria

### User Value Standards
- **Usefulness**: Feature solves real user problems
- **Usability**: Feature is easy to discover and use
- **Efficiency**: Feature saves user time or effort
- **Reliability**: Feature works consistently
- **Delight**: Feature exceeds user expectations

### Product Excellence Criteria
- **Completeness**: Features are fully implemented
- **Consistency**: Similar features work similarly
- **Integration**: Features work well together
- **Documentation**: Features are well explained
- **Support**: Features consider user support needs

### Market Fit Criteria
- **Differentiation**: Unique value proposition
- **Competition**: Feature parity where needed
- **Adoption**: Low barriers to entry
- **Growth**: Enables user success
- **Retention**: Encourages continued use

## Automation Mode Behavior

**Automation Detection**: The command detects automation mode when:
- Environment variable `PRODIGY_AUTOMATION=true` is set
- Called from within a Prodigy workflow context

**Git-Native Automation Flow**:
1. Analyze product from user perspective
2. If enhancements found: Create temporary spec file and commit it
3. If no enhancements found: Report "Product meets current user needs" and exit
4. Always provide a brief summary of proposed enhancements

**Output Format in Automation Mode**:
- Minimal console output focusing on key enhancements
- Clear indication of spec creation and commit
- Brief summary of enhancement proposals
- User impact focus

**Example Automation Output**:
```
✓ Product analysis completed
✓ Found 4 enhancement opportunities
✓ Generated spec: iteration-1708123456-product-enhancements.md
✓ Committed: product: enhance cli-ux for iteration-1708123456
```

**Example No Enhancements Output**:
```
✓ Product analysis completed
✓ Product meets current user needs - no immediate enhancements identified
```

## Output Format

1. **Executive Summary**
   - Overall product maturity assessment
   - High-impact enhancement opportunities
   - Quick wins vs long-term improvements

2. **Detailed Findings**
   - Feature-by-feature analysis
   - User journey mapping results
   - Competitive gap analysis

3. **Enhancement Roadmap**
   - Prioritized enhancement list
   - Implementation effort estimates
   - User impact assessments

4. **Success Metrics**
   - User satisfaction indicators
   - Adoption metrics
   - Feature usage projections

## Integration with Development Workflow

### Product Planning
- Input for roadmap prioritization
- User feedback incorporation
- Feature request validation
- Market fit assessment

### Pre-Release Reviews
- Feature completeness checks
- User experience validation
- Documentation review
- Onboarding flow testing

### Post-Release Analysis
- Feature adoption tracking
- User feedback integration
- Enhancement opportunity identification
- Success metric evaluation

## Example Usage

```
/prodigy-product-enhance
/prodigy-product-enhance "cli"
/prodigy-product-enhance "api documentation"
/prodigy-product-enhance "onboarding"
```

## Advanced Features

### User Persona Analysis
- Different enhancements for different user types
- Persona-specific workflow optimization
- Targeted documentation improvements
- Role-based feature suggestions

### Competitive Analysis
- Feature comparison with similar tools
- Best practice adoption opportunities
- Market differentiation suggestions
- Integration ecosystem analysis

### Growth Opportunities
- Viral feature suggestions
- Community building features
- Collaboration enhancements
- Platform expansion possibilities

## Product Excellence Gates

### Minimum Standards
- Core features must be complete
- Documentation must exist for all features
- Error messages must be helpful
- Getting started must be < 5 minutes

### Best Practice Guidelines
- Follow platform conventions
- Provide sensible defaults
- Support common workflows
- Enable power user features

### Continuous Improvement
- Track user satisfaction metrics
- Monitor feature adoption
- Collect user feedback
- Iterate based on usage data

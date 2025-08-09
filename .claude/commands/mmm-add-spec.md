# /mmm-add-spec

Generate a new specification document based on a feature description. This command creates a properly formatted specification that follows project conventions and integrates with the existing specification system.

## Variables

DESCRIPTION: $ARGUMENTS (required - natural language description of the feature to implement)

## Execute

### Phase 1: Project Context Analysis

1. **Read Project Context**
   - Read .mmm context files to understand current project state
   - Files read in order: .mmm/PROJECT.md, .mmm/ARCHITECTURE.md, .mmm/CONVENTIONS.md, .mmm/ROADMAP.md, .mmm/DECISIONS.md
   - Understand existing capabilities and architectural patterns
   - Identify current technology stack and dependencies

2. **Analyze Existing Specifications**
   - Scan specs/ directory to find existing specification files
   - Parse frontmatter from each spec file to extract metadata
   - Determine next specification number by finding highest existing number + 1
   - Skip SPEC_INDEX.md if it exists (deprecated)
   - Review existing specifications to understand format and depth
   - Identify related specifications and potential dependencies
   - Determine appropriate specification category (foundation, parallel, storage, etc.)

3. **Feature Analysis**
   - Parse the provided DESCRIPTION to extract core requirements
   - Identify functional and non-functional requirements
   - Determine integration points with existing system
   - Assess complexity and implementation scope

### Phase 2: Specification Structure Planning

1. **Determine Specification Details**
   - Generate next specification number (highest existing + 1)
   - Choose descriptive filename: `{number}-{kebab-case-title}.md`
   - Place all specs directly in specs/ directory
   - Determine priority level and category

2. **Requirements Analysis**
   - Break down feature description into specific requirements
   - Identify acceptance criteria and success metrics
   - Determine dependencies on other specifications
   - Identify potential risks and challenges

3. **Technical Design Planning**
   - Analyze architectural implications
   - Identify new modules or components needed
   - Determine data structures and interfaces
   - Plan integration with existing codebase

### Phase 3: Specification Generation

1. **Create Specification Document**
   - Generate complete specification following project template
   - Include comprehensive objective and context
   - Define detailed acceptance criteria with measurable outcomes
   - Specify technical requirements and constraints

2. **Specification Sections**
   - **Header**: Title, number, category, priority
   - **Context**: Background and motivation for the feature
   - **Objective**: Clear statement of what needs to be achieved
   - **Requirements**: Detailed functional and non-functional requirements
   - **Acceptance Criteria**: Specific, testable criteria for completion
   - **Technical Details**: Implementation approach and considerations
   - **Dependencies**: Prerequisites and related specifications
   - **Testing Strategy**: How the feature will be validated
   - **Documentation**: Required documentation updates
   - **Migration**: Any migration or compatibility considerations

3. **Integration Considerations**
   - Define how feature integrates with existing architecture
   - Specify API changes or new interfaces
   - Identify configuration requirements

### Phase 4: Validation and Refinement

1. **Specification Review**
   - Ensure all requirements are clear and testable
   - Verify acceptance criteria are specific and measurable
   - Check for completeness and consistency
   - Validate technical feasibility

2. **Dependency Analysis**
   - Identify prerequisite specifications
   - Check for circular dependencies
   - Determine implementation order constraints
   - Validate integration points

3. **Quality Assurance**
   - Ensure specification follows project conventions
   - Verify proper formatting and structure
   - Check for clarity and unambiguous language
   - Validate technical accuracy

### Phase 5: Context Updates

1. **Update PROJECT.md (if needed)**
   - Add new planned capability to "What Will Exist" section if major feature
   - Update feature list with new specification if significant
   - Adjust project scope if significantly changed
   - Update complexity assessment if needed

### Phase 6: File Organization

1. **Create Specification File**
   - Write specification to specs/{number}-{kebab-case-title}.md
   - Ensure proper file permissions and formatting
   - Validate file structure and content
   - Include YAML frontmatter with metadata at the very beginning of the file

### Phase 7: Commit Changes

1. **Stage All Changes**
   - Stage the new specification file
   - Stage any updated context files (PROJECT.md if modified)

2. **Create Commit**
   - Use descriptive commit message format: "add: spec {NUMBER} - {title}"
   - Include brief description of the specification purpose
   - Reference the specification number in commit message
   - Ensure all related changes are included in single commit

3. **Verify Commit**
   - Check that all modified files are included
   - Verify commit message follows project conventions
   - Ensure no unrelated changes are included
   - Confirm specification file exists with proper frontmatter

## Specification Template Structure

### Standard Format
```markdown
---
number: {NUMBER}
title: {TITLE}
category: {foundation|parallel|storage|compatibility|testing|optimization}
priority: {critical|high|medium|low}
status: draft
dependencies: [{list of prerequisite spec numbers}]
created: {YYYY-MM-DD}
---

# Specification {NUMBER}: {TITLE}

**Category**: {foundation|parallel|storage|compatibility|testing|optimization}
**Priority**: {critical|high|medium|low}
**Status**: draft
**Dependencies**: {list of prerequisite specs}

## Context

{Background information and motivation}

## Objective

{Clear, concise statement of what needs to be achieved}

## Requirements

### Functional Requirements
- {Specific functional requirements}

### Non-Functional Requirements
- {Performance, security, usability requirements}

## Acceptance Criteria

- [ ] {Specific, testable criterion 1}
- [ ] {Specific, testable criterion 2}
- [ ] {Additional criteria...}

## Technical Details

### Implementation Approach
{High-level implementation strategy}

### Architecture Changes
{Required architectural modifications}

### Data Structures
{New data structures or modifications}

### APIs and Interfaces
{New or modified interfaces}

## Dependencies

- **Prerequisites**: {Required prior specifications}
- **Affected Components**: {Existing components that will be modified}
- **External Dependencies**: {New external dependencies}

## Testing Strategy

- **Unit Tests**: {Unit testing approach}
- **Integration Tests**: {Integration testing requirements}
- **Performance Tests**: {Performance validation}
- **User Acceptance**: {User-facing validation}

## Documentation Requirements

- **Code Documentation**: {Required inline documentation}
- **User Documentation**: {User-facing documentation updates}
- **Architecture Updates**: {ARCHITECTURE.md updates needed}

## Implementation Notes

{Additional implementation considerations, gotchas, or best practices}

## Migration and Compatibility

{Any breaking changes, migration requirements, or compatibility considerations}
```

## Feature Categories

### Foundation Specifications
- Core architecture and infrastructure
- Basic data structures and algorithms
- Essential system components
- Build and deployment systems

### Parallel Specifications
- Concurrent processing features
- Multi-threading and async capabilities
- Performance optimization through parallelism
- Resource sharing and synchronization

### Storage Specifications
- Data persistence and retrieval
- Database integration and optimization  
- Caching and memory management
- File system and storage abstractions

### Compatibility Specifications
- Integration with external systems
- API compatibility and versioning
- Cross-platform support
- Legacy system integration

### Testing Specifications
- Test infrastructure and frameworks
- Automated testing pipelines
- Performance and load testing
- Quality assurance processes

### Optimization Specifications
- Performance improvements
- Resource usage optimization
- Algorithm and data structure improvements
- System efficiency enhancements

## Quality Standards

### Requirement Clarity
- All requirements must be specific and unambiguous
- Acceptance criteria must be testable and measurable
- Technical details must be sufficient for implementation
- Dependencies must be clearly identified

### Completeness
- All aspects of the feature must be covered
- Integration points must be specified
- Testing requirements must be defined
- Documentation needs must be identified

### Consistency
- Follow established project terminology
- Align with existing architectural patterns
- Maintain consistent specification format
- Use standard project conventions

## Example Usage

```
/mmm-add-spec "Add user authentication system with JWT tokens"
/mmm-add-spec "Implement caching layer for database queries"
/mmm-add-spec "Add REST API endpoints for project management"
/mmm-add-spec "Create automated backup system for project data"
```

## Integration with Development Workflow

### Specification Lifecycle
1. **Generation**: Created with /mmm-add-spec command
2. **Review**: Technical review and refinement
3. **Approval**: Stakeholder approval and finalization
4. **Implementation**: Actual feature development
5. **Validation**: Testing and acceptance verification
6. **Completion**: Delete spec file and commit with implementation

### Traceability
- Link specifications to implementation commits
- Track progress through acceptance criteria
- Monitor dependencies and prerequisites
- Maintain audit trail of changes

### Continuous Improvement
- Learn from implemented specifications
- Refine template based on experience
- Improve requirement gathering process
- Enhance specification quality over time

## Error Handling

### Invalid Descriptions
- Provide guidance for unclear descriptions
- Suggest more specific requirements
- Ask clarifying questions when needed
- Recommend breaking down complex features

### Dependency Conflicts
- Identify circular dependencies
- Suggest alternative approaches
- Recommend prerequisite specifications
- Flag implementation order issues

### Template Validation
- Ensure all required sections are present
- Validate specification format and structure
- Check for consistency with project standards
- Verify technical feasibility

## Advanced Features

### Smart Categorization
- Automatically categorize specifications based on description
- Suggest appropriate priority levels
- Recommend implementation phases
- Identify likely dependencies

### Template Customization
- Adapt template based on feature type
- Include category-specific sections
- Adjust detail level based on complexity
- Customize for different project phases

### Specification Analytics
- Track specification complexity over time
- Analyze implementation success rates
- Identify common requirement patterns
- Monitor specification quality metrics

# Refine Multiple Specs Command

Refines multiple incomplete or unfocused specifications by addressing validation gaps in batch. This command efficiently handles refinement of multiple specs identified as incomplete during validation.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-refine-specs <spec-files> --gaps <validation-gaps-json>
```

Examples:
- `/prodigy-refine-specs specs/102-auth.md specs/103-session.md --gaps ${validation.gaps}`
- `/prodigy-refine-specs ${validation.incomplete_specs} --gaps ${validation.gaps}`

## What This Command Does

1. **Receives Batch Validation Feedback**
   - Gets list of incomplete spec files
   - Receives gaps for each spec from validation
   - Understands specific improvements needed per spec

2. **Refines Multiple Specs Efficiently**
   - Processes each incomplete spec
   - Adds missing sections where needed
   - Clarifies vague requirements
   - Makes acceptance criteria testable
   - Enhances technical details

3. **Handles Inter-Spec Dependencies**
   - Ensures consistency across related specs
   - Updates dependency references if needed
   - Maintains logical separation of concerns

4. **Commits Changes Atomically**
   - Updates all refined specs in one operation
   - Commits with clear message about batch refinements
   - Maintains traceability of changes

## Execution Process

### Step 1: Parse Input and Analyze Gaps

- Extract spec files list from $ARGUMENTS
- Parse --gaps parameter containing validation data for each spec
- Build refinement plan for each incomplete spec
- Identify common issues across specs

### Step 2: Load Specs and Gaps

For each spec file in the list:
- Read current spec content
- Extract specific gaps for this spec from gaps JSON
- Determine refinement priority and approach
- Check for related specs that might be affected

### Step 3: Batch Refinement Strategy

Determine refinement approach for each spec:

#### Enhancement Strategy
For specs with minor gaps (completion 70-99%):
- Add missing sections
- Clarify requirements
- Improve acceptance criteria
- Add technical details
- Fix testability issues

#### Major Revision Strategy
For specs with significant gaps (completion < 70%):
- Restructure for clarity
- Add comprehensive technical details
- Rewrite vague sections
- Ensure all required sections present

#### Consistency Strategy
For related specs:
- Ensure consistent terminology
- Align dependency declarations
- Verify no overlapping functionality
- Maintain clear boundaries

### Step 4: Apply Refinements

Process each spec systematically:

#### Missing Sections
Add any missing required sections:
```markdown
## Testing Strategy
- Unit tests for authentication logic
- Integration tests for session management
- Performance tests for token validation
- Security tests for password handling
```

#### Vague Requirements
Transform vague statements into specific ones:
```markdown
# Before
- System should handle many users
- Sessions should timeout appropriately

# After
- System must support 10,000 concurrent authenticated users
- Sessions must expire after 24 hours of inactivity
```

#### Testability Issues
Make all acceptance criteria verifiable:
```markdown
# Before
- [ ] Good security practices
- [ ] Efficient session handling

# After
- [ ] Passwords are hashed using bcrypt with cost factor 12
- [ ] Session lookup completes in under 50ms for 95% of requests
```

### Step 5: Update Spec Files

For each spec requiring refinement:
1. Read the existing spec file
2. Apply all necessary refinements
3. Preserve existing good content
4. Update file with enhanced content
5. Ensure proper formatting maintained

### Step 6: Verify Consistency

After all refinements:
- Check inter-spec dependencies are valid
- Ensure no functionality gaps remain
- Verify no unintended overlap created
- Confirm all original requirements covered

### Step 7: Generate Summary

Create a summary of refinements made:
```
Refined 2 specifications:
- specs/102-authentication.md: Added testing strategy, clarified security requirements
- specs/103-session-management.md: Enhanced technical details, made criteria testable
```

## Refinement Rules

### Priority Order

Process specs in order of:
1. Specs with highest severity gaps
2. Specs that are dependencies for others
3. Specs with lowest completion percentage
4. Remaining specs

### Consistency Rules

When refining multiple specs:
- Use consistent terminology across all specs
- Maintain clear separation of concerns
- Ensure dependency declarations match
- Avoid creating overlapping functionality

### Quality Standards

All refined specs must have:
- Clear, measurable objectives
- Testable acceptance criteria
- Sufficient technical detail for implementation
- Comprehensive testing strategy
- Proper dependency declarations

## Error Handling

- If spec file not found: Log error and continue with other specs
- If gaps JSON invalid: Attempt to parse what's available
- If refinement fails: Report which specs couldn't be refined
- If no gaps provided: Skip refinement, report as complete

## Batch Processing Optimization

To efficiently handle multiple specs:
1. Load all specs into memory first
2. Analyze gaps collectively
3. Apply refinements in batch
4. Write all changes at once
5. Generate single commit for all changes

## Output Format

The command outputs:
1. Updated spec files with refinements applied
2. Console summary of changes made
3. List of any specs that couldn't be refined

Example console output:
```
Refining 3 incomplete specifications...

✓ specs/102-authentication.md
  - Added comprehensive testing strategy
  - Clarified password complexity requirements
  - Enhanced technical implementation details

✓ specs/103-session-management.md
  - Made all acceptance criteria testable
  - Added performance requirements
  - Specified session storage approach

✓ specs/104-authorization.md
  - Added missing dependencies section
  - Clarified role-based access control requirements
  - Enhanced security considerations

Successfully refined 3 specifications
```

## Integration with Workflow

This command is designed to work seamlessly with:
- `/prodigy-validate-all-specs` output
- Workflow validation.gaps variable
- Workflow validation.incomplete_specs variable
- Git context variables for tracking changes
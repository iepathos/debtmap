# Implement Spec Command

Implements a Git Good specification by reading the spec file, executing the implementation, and updating .prodigy context files.  Read the files in .prodigy to get general project context.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-implement-spec <spec-identifier>
```

Examples: 
- `/prodigy-implement-spec 01` to implement the project structure specification
- `/prodigy-implement-spec iteration-1234567890-improvements` to implement a temporary improvement spec

## What This Command Does

1. **Reads the Project Context**
   - Read the .prodigy context files to get general understanding of the project.
   - Files are read in this order to build context:
     - PROJECT.md (current state and capabilities)
     - ARCHITECTURE.md (system design)
     - CONVENTIONS.md (coding standards)
     - ROADMAP.md (progress tracking)
     - DECISIONS.md (technical decisions)
2. **Reads the Specification**
   - Locates the spec file based on the provided identifier ($ARGUMENTS)
   - **Permanent specs**: Located in specs/ subdirectory (e.g., 01-some-spec.md)
   - **Temporary specs**: Located in specs/temp/ (e.g., iteration-1234567890-improvements.md)
   - Parses the specification content and requirements
   - Identifies implementation tasks and success criteria

3. **Implements the Specification**
   - Creates necessary files and directories
   - Writes implementation code according to the spec
   - Follows conventions defined in CONVENTIONS.md
   - Ensures all success criteria are met

4. **Updates Context Files**
   - Updates PROJECT.md with new capabilities and current state
   - Updates ARCHITECTURE.md with implementation details
   - Updates ROADMAP.md to mark spec as completed
   - Adds new decisions to DECISIONS.md if needed
   - Documents any new conventions in CONVENTIONS.md

5. **Validates Implementation**
   - Runs tests if applicable
   - Runs lint checks
   - Verifies success criteria from the spec

6. **Commits Changes**
   - Creates a git commit with appropriate message
   - Follows commit message format from CONVENTIONS.md

## Execution Process

### Step 1: Read Context Files and Locate Specification

The command will:
- First check if a spec identifier was provided ($ARGUMENTS)
- If no identifier provided, fail with: "Error: Spec identifier is required. Usage: /prodigy-implement-spec <spec-identifier>"
- Read all .prodigy context files in order (PROJECT.md, ARCHITECTURE.md, CONVENTIONS.md, ROADMAP.md, DECISIONS.md)
- Build comprehensive understanding of project state and conventions
- Locate specification file using $ARGUMENTS:
  - **Numeric IDs** (e.g., "01", "08a", "67"): Find spec file matching pattern `specs/{number}-*.md`
  - **Iteration IDs** (e.g., "iteration-1234567890-improvements"): Find $ARGUMENTS.md directly in specs/temp/
- Read the corresponding spec file
- Extract implementation requirements and success criteria

### Step 2: Analyze Current State

Before implementing:
- Review current codebase structure
- Check for existing related code
- Identify dependencies and prerequisites

### Step 3: Implementation

Based on the spec type:
- **Foundation specs**: Create core structures and modules
- **Parallel specs**: Implement concurrent processing features
- **Storage specs**: Add storage optimization features
- **Compatibility specs**: Ensure Git compatibility
- **Testing specs**: Create test suites and benchmarks
- **Optimization specs**: Improve performance

### Step 4: Context Updates

Update .prodigy files (skip for temporary iteration specs):
- **Permanent specs only**:
  - **PROJECT.md**: Update "Current State" percentage and "What Exists"
  - **ARCHITECTURE.md**: Add architectural details for new components
  - **DECISIONS.md**: Add ADRs for significant implementation choices
  - **CONVENTIONS.md**: Document any new patterns discovered
- **Temporary specs**: Skip context updates, focus on implementing fixes

### Step 5: Validation and Commit

Final steps:
- Run `cargo fmt` and `cargo clippy`
- Run `cargo test` if tests exist
- **Delete spec file**: Remove the implemented spec file after successful implementation (both permanent and temporary specs)
- **Report modified files** (for automation tracking):
  - List all files that were created, modified, or deleted
  - Include brief description of changes made
  - Format: "Modified: src/main.rs", "Created: tests/new_test.rs", "Deleted: specs/67-worktree-cleanup-after-merge.md"
- **Git commit (REQUIRED for automation)**:
  - Stage all changes: `git add .`
  - **Permanent specs**: "feat: implement spec {number} - {title}"
  - **Temporary specs**: "fix: apply improvements from spec {spec-id}"
  - **IMPORTANT**: Do NOT add any attribution text like "🤖 Generated with [Claude Code]" or "Co-Authored-By: Claude" to commit messages. Keep commits clean and focused on the change itself.
  - Include modified files in commit body for audit trail

## Implementation Guidelines

1. **Follow Existing Patterns**
   - Use the module organization from ARCHITECTURE.md
   - Follow naming conventions from CONVENTIONS.md
   - Maintain consistency with existing code

2. **Incremental Progress**
   - Implement specs in order when possible
   - Ensure each spec builds on previous work
   - Don't skip prerequisites

3. **Documentation**
   - Add inline documentation for new code
   - Update module-level documentation
   - Keep .prodigy files current

4. **Testing**
   - Add unit tests for new functionality
   - Create integration tests where applicable
   - Ensure existing tests still pass

## CRITICAL: Autonomous Implementation Requirements

**YOU MUST WORK AUTONOMOUSLY - NO USER QUESTIONS ALLOWED**

This command runs in automation workflows that require commits. You MUST:

### 1. Make All Implementation Decisions Autonomously

- **NEVER ask the user for implementation direction, approach, or preferences**
- Use your technical judgment based on:
  - Spec's explicit requirements and acceptance criteria
  - Project architecture and conventions from .prodigy files
  - Priority level (critical > high > medium > low)
  - Incremental progress principle: working code > perfect code
- If multiple valid approaches exist, choose the one that:
  - Best satisfies the acceptance criteria
  - Follows existing project patterns
  - Minimizes risk and breaking changes
  - Can be completed and committed in this session

### 2. Handle Complex Specs with Phased Implementation

When a spec requires significant architectural changes:

**DO THIS** (Autonomous phased approach):
1. Break the spec into logical phases automatically
2. Implement Phase 1 completely (the most critical part)
3. Commit Phase 1 with substantive progress
4. Document remaining phases in the spec's "Implementation Notes" section
5. Update spec status from "draft" to "in-progress" or "partial"

**NEVER DO THIS**:
- Ask user "which phase should I implement?"
- Stop without committing because "it's too complex"
- Request clarification on what to prioritize
- Provide multiple options and ask user to choose

### 3. Determine Phase 1 Priority Automatically

Use this decision framework for complex specs:

1. **Critical bugs/contradictions**: Fix the most severe user-facing issue first
2. **Validation/testing**: Add validation that prevents future issues
3. **Foundation work**: Create data structures and types needed for full implementation
4. **Non-breaking changes**: Implement parts that don't affect existing functionality
5. **Documentation**: Document findings and remaining work

Example for Spec 134 (metric contradictions):
- Phase 1: Add validation layer to detect contradictions (non-breaking, immediate value)
- Phase 2: Fix visibility counting bug (critical user-facing issue)
- Phase 3: Unify function counting systems (architectural change)

### 4. Minimum Commit Requirements

Every execution MUST produce at least one commit with substantive changes:

**Acceptable commits include**:
- Implemented functionality (even if partial)
- New test cases that expose the problem
- Validation code that detects issues
- Refactored code that improves structure
- Fixed bugs identified during investigation
- Documentation of investigation findings in code comments

**NOT acceptable**:
- Only reading files
- Only analysis without code changes
- Only asking questions
- Only documenting "what could be done"

### 5. When Genuinely Blocked

If you discover a blocker (missing dependency, conflicting requirement, unclear spec):

**DO THIS**:
1. Implement what you CAN implement around the blocker
2. Add TODO comments documenting the specific blocker
3. Update spec with "Blockers" section explaining the issue
4. Commit your partial progress
5. The commit message should indicate partial implementation

**Example**:
```
feat: implement spec 134 phase 1 - add metric validation layer

- Add validation functions to detect metric contradictions
- Create test cases for known inconsistencies
- Document findings in god_object_metrics.rs

Note: Full unification of counting systems requires architectural
changes documented in spec. This phase provides immediate value by
detecting contradictions.

Relates to spec 134.
```

### 6. Time Management

- Aim to complete Phase 1 within reasonable time (30-60 minutes)
- If Phase 1 is taking longer, reduce scope further and commit what works
- **Never spend >90 minutes without committing something**
- Progress > Perfection

### 7. Decision-Making Examples

**Scenario**: "The spec says to fix visibility counting, but I found three different places it's counted"

**Wrong Response**: "There are three places to fix. Which should I prioritize?"

**Correct Response**:
1. Fix the most critical one (the one used in user-facing output)
2. Add TODO comments for the other two
3. Commit the fix with notes about remaining work

**Scenario**: "The spec requires changes to core architecture across 5 modules"

**Wrong Response**: "This is too complex. Should I create a plan document instead?"

**Correct Response**:
1. Identify the smallest valuable subset (e.g., validation layer)
2. Implement that subset completely
3. Commit with clear message indicating Phase 1 of larger work
4. Update spec with implementation plan for remaining phases

**Scenario**: "I'm not sure if we should use approach A or B"

**Wrong Response**: "Which approach do you prefer?"

**Correct Response**:
1. Evaluate both against acceptance criteria
2. Choose the one that best fits project patterns
3. Implement it
4. Document the decision in code comments or DECISIONS.md
5. Commit

### 8. Quality Standards Still Apply

Autonomous doesn't mean sloppy:
- Code must compile
- Tests must pass
- Follow project conventions
- Use proper error handling
- Add appropriate documentation

But remember: **Working incremental progress beats perfect unshipped code**

## Automation Mode Behavior

**Automation Detection**: The command detects automation mode when:
- Environment variable `PRODIGY_AUTOMATION=true` is set
- Called from within a Prodigy workflow context

**Git-Native Automation Flow**:
1. Read spec file and implement all required changes
2. Stage all changes and commit with descriptive message
3. Provide brief summary of work completed
4. Always commit changes (no interactive confirmation)

**Output Format in Automation Mode**:
- Minimal console output focusing on key actions
- Clear indication of files modified
- Confirmation of git commit
- Brief summary of implementation

**Example Automation Output**:
```
✓ Implementing spec: iteration-1708123456-improvements
✓ Modified: src/main.rs (fixed error handling)
✓ Modified: src/database.rs (added unit tests)
✓ Created: tests/integration_test.rs
✓ Committed: fix: apply improvements from spec iteration-1708123456-improvements
```

## Error Handling

The command will:
- Fail gracefully if spec doesn't exist
- Report validation failures clearly
- Rollback changes if tests fail
- Provide helpful error messages

## Example Workflow

```
/prodigy-implement-spec 67
```

This would:
1. Find and read `specs/67-worktree-cleanup-after-merge.md`
2. Implement the worktree cleanup functionality
3. Update orchestrator cleanup method
4. Update PROJECT.md to show new capability
5. Run cargo fmt and clippy
6. Delete the spec file `specs/67-worktree-cleanup-after-merge.md`
7. Commit: "feat: implement spec 67 - worktree cleanup after merge"

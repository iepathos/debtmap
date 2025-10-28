# Implement Spec Command

Implements a Prodigy specification by reading the spec file and executing the implementation.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-implement-spec <spec-identifier>
```

Examples: 
- `/prodigy-implement-spec 01` to implement the project structure specification
- `/prodigy-implement-spec iteration-1234567890-improvements` to implement a temporary improvement spec

## What This Command Does

1. **Reads the Specification**
   - Locates the spec file based on the provided identifier ($ARGUMENTS)
   - **Permanent specs**: Located in specs/ subdirectory (e.g., 01-some-spec.md)
   - **Temporary specs**: Located in specs/temp/ (e.g., iteration-1234567890-improvements.md)
   - Parses the specification content and requirements
   - Identifies implementation tasks and success criteria

2. **Implements the Specification**
   - Creates necessary files and directories
   - Writes implementation code according to the spec
   - Follows existing code patterns and conventions
   - Ensures all success criteria are met

3. **Validates Implementation**
   - Runs tests if applicable
   - Runs lint checks (cargo fmt, cargo clippy)
   - Verifies success criteria from the spec

4. **Commits Changes**
   - Creates a git commit with appropriate message
   - Follows clean commit message format (no AI attribution)

## Execution Process

### Step 1: Locate and Read Specification

The command will:
- First check if a spec identifier was provided ($ARGUMENTS)
- If no identifier provided, fail with: "Error: Spec identifier is required. Usage: /prodigy-implement-spec <spec-identifier>"
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

### Step 4: Validation and Commit

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
   - Study similar code in the codebase before implementing
   - Use consistent module organization
   - Follow existing naming conventions
   - Maintain consistency with existing code style

2. **Incremental Progress**
   - Implement specs in order when possible
   - Ensure each spec builds on previous work
   - Don't skip prerequisites

3. **Documentation**
   - Add inline documentation for new code
   - Update module-level documentation
   - Document complex logic and design decisions

4. **Testing**
   - Add unit tests for new functionality
   - Create integration tests where applicable
   - Ensure existing tests still pass

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
4. Run cargo fmt and clippy
5. Delete the spec file `specs/67-worktree-cleanup-after-merge.md`
6. Commit: "feat: implement spec 67 - worktree cleanup after merge"

# Implement Spec Command

Implements a Git Good specification by reading the spec file, executing the implementation, and updating .mmm context files.  Read the files in .mmm to get general project context.

Arguments: $ARGUMENTS

## Usage

```
/mmm-implement-spec <spec-identifier>
```

Examples: 
- `/mmm-implement-spec 01` to implement the project structure specification
- `/mmm-implement-spec iteration-1234567890-improvements` to implement a temporary improvement spec

## What This Command Does

1. **Reads the Project Context**
   - Read the .mmm context files to get general understanding of the project.
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
- If no identifier provided, fail with: "Error: Spec identifier is required. Usage: /mmm-implement-spec <spec-identifier>"
- Read all .mmm context files in order (PROJECT.md, ARCHITECTURE.md, CONVENTIONS.md, ROADMAP.md, DECISIONS.md)
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

Update .mmm files (skip for temporary iteration specs):
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
   - Keep .mmm files current

4. **Testing**
   - Add unit tests for new functionality
   - Create integration tests where applicable
   - Ensure existing tests still pass

## Automation Mode Behavior

**Automation Detection**: The command detects automation mode when:
- Environment variable `MMM_AUTOMATION=true` is set
- Called from within an MMM workflow context

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
/mmm-implement-spec 67
```

This would:
1. Find and read `specs/67-worktree-cleanup-after-merge.md`
2. Implement the worktree cleanup functionality
3. Update orchestrator cleanup method
4. Update PROJECT.md to show new capability
5. Run cargo fmt and clippy
6. Delete the spec file `specs/67-worktree-cleanup-after-merge.md`
7. Commit: "feat: implement spec 67 - worktree cleanup after merge"

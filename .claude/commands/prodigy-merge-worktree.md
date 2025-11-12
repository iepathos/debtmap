# MMM Merge Worktree Command

Intelligently merges MMM worktree branches with automatic conflict resolution into the current branch (or repository's default branch if in detached HEAD state).

Arguments: $ARGUMENTS

## Usage

```
/prodigy-merge-worktree <source-branch> [target-branch]
```

Arguments:
- `source-branch`: Branch to merge FROM (required)
- `target-branch`: Branch to merge TO (optional, defaults to current branch or main/master)

Examples:
- `/prodigy-merge-worktree prodigy-performance-1234567890` (uses current branch)
- `/prodigy-merge-worktree prodigy-performance-1234567890 feature/my-feature` (explicit target)
- `/prodigy-merge-worktree prodigy-security-1234567891 main` (explicit main)

## Execute

1. **Parse Arguments**
   - Split $ARGUMENTS on whitespace to extract source and optional target branch
   - First argument: source_branch (required)
   - Second argument: target_branch (optional)
   - If no source branch provided, fail with: "Error: Branch name is required. Usage: /prodigy-merge-worktree <source-branch> [target-branch]"

2. **Determine Target Branch**
   - If target_branch argument provided (second argument exists):
     - Verify target branch exists using `git rev-parse --verify refs/heads/$target_branch`
     - If exists, use it; otherwise fail with: "Error: Target branch '$target_branch' does not exist"
   - Otherwise (for backward compatibility):
     - Get the current branch using `git rev-parse --abbrev-ref HEAD`
     - If the current branch is a valid branch name (not HEAD), use it as the target
     - Otherwise, fall back to the default branch:
       - Check if 'main' branch exists using `git rev-parse --verify refs/heads/main`
       - If main exists, use 'main', otherwise use 'master'
   - Switch to the target branch if not already on it

3. **Attempt Standard Merge**
   - Execute `git merge --no-ff <source-branch>` to preserve commit history (use first argument)
   - If successful (no conflicts), the merge commit is automatically created - verify with `git status` and exit
   - If conflicts occur, proceed to step 4

4. **Handle Merge Conflicts** (if any)
   - Detect and analyze all conflicted files
   - Understand the intent of changes from both branches
   - Resolve conflicts intelligently based on context
   - Preserve functionality from both branches where possible

5. **Apply Resolution**
   - Resolve conflict markers in all files
   - Stage resolved files
   - Create detailed merge commit explaining resolutions

6. **Verify Merge**
   - Run basic validation (syntax checks, etc.)
   - Ensure no conflict markers remain
   - Commit the merge with comprehensive message
   - **CRITICAL**: Verify working directory is clean with `git status --porcelain`
   - If any uncommitted changes remain, stage and commit them with message: "chore: finalize merge cleanup"

## Conflict Resolution Strategy

### Priority Order
1. **Functionality**: Ensure code remains functional
2. **Latest Intent**: Prefer changes that represent newest understanding
3. **Completeness**: Include additions from both branches
4. **Safety**: When uncertain, preserve both versions with clear separation

### Resolution Patterns

**Function/Method Conflicts**:
- If same function modified differently, analyze which version is more complete
- Merge beneficial changes from both when possible
- Preserve all test additions

**Import/Dependency Conflicts**:
- Combine imports from both branches
- Remove duplicates
- Maintain correct ordering

**Documentation Conflicts**:
- Merge documentation additions
- Prefer more comprehensive explanations
- Combine examples from both branches

**New File Conflicts**:
- If same filename but different content, rename one with branch suffix
- Alert in merge commit about the rename

**Deletion Conflicts**:
- If deleted in one branch but modified in another, prefer modification
- Document the decision in merge commit

## Merge Commit Format

```
Merge worktree '<source-branch>' into <target-branch>

Successfully merged with <N> conflicts resolved:

Resolved Conflicts:
- path/to/file1.rs: Combined performance improvements with security fixes
- path/to/file2.py: Merged test additions from both branches
- path/to/file3.md: Combined documentation updates

Resolution Strategy:
<Brief explanation of how conflicts were resolved>

Original commits from worktree:
<List of commits being merged>
```

**IMPORTANT**: Do NOT add any attribution text like "🤖 Generated with [Claude Code]" or "Co-Authored-By: Claude <noreply@anthropic.com>" to merge commit messages. Keep commits clean and focused on the merge itself to avoid bloating git history.

## Error Handling

**If merge cannot be completed**:
1. Abort the merge to maintain clean state
2. Provide clear error message with:
   - Which files have unresolvable conflicts
   - Why they couldn't be resolved automatically
   - Suggested manual steps

**Common unresolvable scenarios**:
- Binary file conflicts
- Fundamental architectural conflicts
- Mutually exclusive changes

## Best Practices

1. **Always verify** the target branch is correct before merging
2. **Run tests** after merge to ensure functionality
3. **Review** the merge commit to understand what was merged
4. **Clean up** the worktree after successful merge

## Example Workflow

```bash
# Check what needs merging
$ prodigy worktree ls
Active MMM worktrees:
  prodigy-performance-1234567890 - /path/to/.prodigy/worktrees/... (focus: performance)
  prodigy-security-1234567891 - /path/to/.prodigy/worktrees/... (focus: security)

# Merge first worktree
$ claude /prodigy-merge-worktree prodigy-performance-1234567890
Attempting merge...
Found 2 conflicts in:
  - src/main.rs
  - src/lib.rs
Resolving conflicts...
✓ src/main.rs: Combined performance optimization with existing structure
✓ src/lib.rs: Merged both import additions
Creating merge commit...
✓ Successfully merged 'prodigy-performance-1234567890' into master

# Merge second worktree (may have conflicts with first merge)
$ claude /prodigy-merge-worktree prodigy-security-1234567891
Attempting merge...
Found 3 conflicts...
<resolution details>
✓ Successfully merged 'prodigy-security-1234567891' into master
```

## Automation Support

When `MMM_AUTOMATION=true` is set:
- No interactive prompts should be shown
- If branch name is missing or invalid, fail with clear error message
- Merges to the current branch of the worktree (or falls back to default branch if in detached HEAD state)

## Notes

- This command requires git 2.5+ for worktree support
- Always backs up current state before attempting merge
- Preserves full git history from worktree branches
- Can be run multiple times safely (idempotent)
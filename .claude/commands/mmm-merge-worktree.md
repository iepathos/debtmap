# MMM Merge Worktree Command

Intelligently merges MMM worktree branches with automatic conflict resolution into the repository's default branch (main or master).

Arguments: $ARGUMENTS

## Usage

```
/mmm-merge-worktree <branch-name>
```

Examples:
- `/mmm-merge-worktree mmm-performance-1234567890`
- `/mmm-merge-worktree mmm-security-1234567891`

## Execute

1. **Get Branch Name**
   - The branch name is provided as: $ARGUMENTS
   - If no branch name provided (empty $ARGUMENTS), fail with: "Error: Branch name is required. Usage: /mmm-merge-worktree <branch-name>"

2. **Determine Default Branch**
   - Check if 'main' branch exists using `git rev-parse --verify refs/heads/main`
   - If main exists, use 'main', otherwise use 'master'
   - Switch to the default branch

3. **Attempt Standard Merge**
   - Execute `git merge --no-ff $ARGUMENTS` to preserve commit history
   - If successful, create merge commit and exit

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
Merge worktree '$ARGUMENTS' into main

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
$ mmm worktree ls
Active MMM worktrees:
  mmm-performance-1234567890 - /path/to/.mmm/worktrees/... (focus: performance)
  mmm-security-1234567891 - /path/to/.mmm/worktrees/... (focus: security)

# Merge first worktree
$ claude /mmm-merge-worktree mmm-performance-1234567890
Attempting merge...
Found 2 conflicts in:
  - src/main.rs
  - src/lib.rs
Resolving conflicts...
✓ src/main.rs: Combined performance optimization with existing structure
✓ src/lib.rs: Merged both import additions
Creating merge commit...
✓ Successfully merged 'mmm-performance-1234567890' into master

# Merge second worktree (may have conflicts with first merge)
$ claude /mmm-merge-worktree mmm-security-1234567891
Attempting merge...
Found 3 conflicts...
<resolution details>
✓ Successfully merged 'mmm-security-1234567891' into master
```

## Automation Support

When `MMM_AUTOMATION=true` is set:
- No interactive prompts should be shown
- If branch name is missing or invalid, fail with clear error message
- Always merges to the default branch (main or master)

## Notes

- This command requires git 2.5+ for worktree support
- Always backs up current state before attempting merge
- Preserves full git history from worktree branches
- Can be run multiple times safely (idempotent)
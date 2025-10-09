# Merge Master Into Current Branch

Merges the repository's default branch (main or master) into the current branch, handling any merge conflicts that arise.

## Execute

1. **Determine Default Branch**
   - Check if 'main' branch exists using `git rev-parse --verify refs/heads/main`
   - If main exists, use 'main', otherwise use 'master'
   - Store as DEFAULT_BRANCH

2. **Get Current Branch**
   - Get the current branch name using `git rev-parse --abbrev-ref HEAD`
   - If on a detached HEAD, fail with clear error message

3. **Fetch Latest Changes**
   - Execute `git fetch origin` to get latest remote changes
   - Ensure we have the most recent version of the default branch

4. **Attempt Merge**
   - Execute `git merge origin/$DEFAULT_BRANCH`
   - If successful, the merge is complete

5. **Handle Merge Conflicts** (if any)
   - Detect all conflicted files using `git diff --name-only --diff-filter=U`
   - For each conflicted file:
     - Analyze the conflict markers
     - Understand the intent of changes from both branches
     - Resolve conflicts intelligently:
       - Combine additions where possible
       - Prefer newer implementations over older ones
       - Maintain all test additions
       - Preserve functionality from both branches
   - Apply resolutions to files
   - Stage resolved files with `git add`

6. **Complete Merge**
   - Once all conflicts are resolved, complete the merge with:
     ```
     git commit -m "Merge $DEFAULT_BRANCH into current branch

     Resolved conflicts in:
     - [list of resolved files]

     Applied intelligent conflict resolution to maintain functionality from both branches."
     ```

## Conflict Resolution Strategy

### Priority Order
1. **Functionality**: Ensure code remains functional after merge
2. **Test Coverage**: Preserve all test additions from both branches
3. **Latest Changes**: When in doubt, prefer changes from the default branch
4. **Completeness**: Include additions from both branches where possible

### Common Conflict Patterns

**Import/Use Statements**:
- Combine imports from both branches
- Remove duplicates
- Maintain alphabetical ordering where it exists

**Function Modifications**:
- If same function modified differently, analyze which is more complete
- Prefer version with better error handling or more features
- If both add different features, try to combine them

**Struct/Class Changes**:
- Merge field additions from both branches
- If same field has different types, prefer the one from default branch
- Preserve all method additions

**Documentation**:
- Merge documentation additions
- Prefer more comprehensive explanations
- Combine examples from both branches

**Configuration Files**:
- Merge new configuration entries
- For conflicting values, prefer default branch
- Preserve all new dependencies

## Error Handling

**If merge cannot be completed**:
1. List all unresolvable conflicts
2. Provide clear guidance on manual resolution needed
3. Leave repository in a state where user can manually resolve

**Common unresolvable scenarios**:
- Binary file conflicts
- Fundamental architectural conflicts that require human decision
- Mutually exclusive business logic changes

## Automation Support

When `PRODIGY_AUTOMATION=true` is set:
- Automatically resolve all conflicts where possible
- No interactive prompts
- If conflicts cannot be auto-resolved, fail with clear error message listing problematic files

## Example Output

**Successful merge without conflicts**:
```
Fetching latest changes from origin...
Merging origin/master into feature-branch...
Already up to date or fast-forward merge completed.
```

**Successful merge with conflicts resolved**:
```
Fetching latest changes from origin...
Merging origin/master into feature-branch...
Auto-merging src/lib.rs
CONFLICT (content): Merge conflict in src/lib.rs
Auto-merging src/main.rs
CONFLICT (content): Merge conflict in src/main.rs

Resolving conflicts...
✓ src/lib.rs: Combined module additions from both branches
✓ src/main.rs: Merged configuration changes, preserved all features

Creating merge commit...
✓ Successfully merged master into feature-branch with 2 conflicts resolved
```

## Notes

- This command is designed to be safe and idempotent
- Always creates a merge commit to preserve history
- Works with both 'main' and 'master' as default branches
- Particularly useful in CI/CD workflows and automated merging scenarios
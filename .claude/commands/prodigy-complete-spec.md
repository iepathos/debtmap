# Complete Spec Implementation Command

Completes a partial specification implementation by addressing validation gaps and missing requirements.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-complete-spec <spec-identifier> [--gaps <validation-gaps-json>]
```

Examples:
- `/prodigy-complete-spec 01` to complete implementation of spec 01
- `/prodigy-complete-spec iteration-1234567890-improvements --gaps ${validation.gaps}` with specific gaps

## What This Command Does

1. **Receives Validation Gaps**
   - Gets list of missing/incomplete items from validation
   - Parses gap details including locations and severity
   - Prioritizes fixes by severity

2. **Completes Implementation**
   - Addresses each gap systematically
   - Focuses on missing requirements first
   - Implements suggested fixes from validation

3. **Verifies Completion**
   - Re-checks implementation after fixes
   - Ensures all gaps are addressed
   - Outputs completion status

## Execution Process

### Step 1: Parse Input

The command will:
- Extract spec identifier from $ARGUMENTS
- Check for --gaps parameter with validation data
- If no gaps provided, run quick validation to identify them
- Parse gaps JSON to understand what needs fixing

### Step 2: Analyze Gaps

Prioritize gaps by:
- **Critical severity**: Fix immediately
- **High severity**: Address next
- **Medium severity**: Fix if time permits
- **Low severity**: Optional improvements

Gap types to handle:
- **Missing code**: Implement required functionality
- **Missing tests**: Add test coverage
- **Missing documentation**: Update docs
- **Code quality issues**: Refactor as needed

### Step 3: Implement Fixes

For each gap:

#### Missing Functionality
- Read location from gap details
- Implement missing code at specified location
- Follow existing patterns and conventions
- Ensure integration with existing code

#### Missing Tests
- Create test file if needed
- Add unit tests for uncovered functions
- Include edge cases and error scenarios
- Run tests to verify they pass

#### Missing Documentation
- Add inline documentation
- Update module-level docs
- Ensure clarity and completeness

#### Code Quality Issues
- Apply suggested fixes
- Refactor for clarity
- Follow project conventions

### Step 4: Validate Fixes

After completing gaps:
- Re-read modified files
- Verify each gap is addressed
- Check that no new issues introduced
- Ensure all tests still pass

### Step 5: Commit Fixes

**CRITICAL**: Commit all fixes to git:
- Stage all modified files with `git add`
- Create a commit with message: `fix: complete spec $ARG implementation gaps`
- Include list of gaps fixed in commit body
- **IMPORTANT**: Do NOT add any attribution text like "🤖 Generated with [Claude Code]" or "Co-Authored-By: Claude" to commit messages. Keep commits clean and focused on the change itself.
- This commit is REQUIRED - Prodigy uses it to verify fixes were made

### Step 6: Output Result

Output JSON result indicating completion:

```json
{
  "completion_percentage": 100.0,
  "status": "complete",
  "gaps_fixed": [
    "Added unit tests for cleanup_worktree",
    "Implemented error recovery handling",
    "Updated documentation"
  ],
  "files_modified": [
    "src/worktree.rs",
    "src/orchestrator.rs",
    "tests/worktree_test.rs"
  ]
}
```

## Gap Resolution Strategies

### Missing Tests
```rust
// For a function like:
fn cleanup_worktree(name: &str) -> Result<()> {
    // implementation
}

// Add comprehensive tests:
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cleanup_worktree_success() {
        // Test successful cleanup
    }
    
    #[test]
    fn test_cleanup_worktree_not_found() {
        // Test error handling
    }
}
```

### Missing Error Handling
```rust
// Transform basic implementation:
worktree.cleanup()?;

// Into robust error handling:
if let Err(e) = worktree.cleanup() {
    log::warn!("Cleanup failed: {}, attempting fallback", e);
    fallback_cleanup(&worktree)?;
}
```

### Missing Documentation
```rust
/// Cleans up a git worktree and its associated resources.
/// 
/// # Arguments
/// * `name` - The name of the worktree to clean up
/// 
/// # Returns
/// * `Ok(())` if cleanup successful
/// * `Err` if cleanup fails
/// 
/// # Example
/// ```
/// cleanup_worktree("feature-branch")?;
/// ```
fn cleanup_worktree(name: &str) -> Result<()> {
    // implementation
}
```

## Automation Mode Behavior

**In Automation Mode** (`PRODIGY_AUTOMATION=true`):
- Parse gaps from environment or arguments
- Fix all gaps without prompts
- Output progress for each fix
- **ALWAYS commit fixes** (required for Prodigy validation)
- Return JSON result at end

## Error Handling

The command will:
- Handle malformed gap data gracefully
- Skip gaps that can't be auto-fixed
- Report any gaps that couldn't be resolved
- Always output valid JSON result

## Example Workflows

### Workflow Integration
```yaml
- claude: "/prodigy-implement-spec $ARG"
  validate:
    type: spec_coverage
    command: "/prodigy-validate-spec $ARG"
    threshold: 95
    on_incomplete:
      strategy: patch_gaps
      claude: "/prodigy-complete-spec $ARG --gaps ${validation.gaps}"
      max_attempts: 3
```

### Gap Data Format
When receiving gaps from validation:
```json
{
  "missing_tests": {
    "description": "No tests for cleanup_worktree",
    "location": "src/worktree.rs:234",
    "severity": "high",
    "suggested_fix": "Add unit tests"
  },
  "error_handling": {
    "description": "Missing error recovery",
    "location": "src/orchestrator.rs:567",
    "severity": "medium",
    "suggested_fix": "Add fallback mechanism"
  }
}
```

## Success Criteria

The command succeeds when:
1. All critical and high severity gaps are fixed
2. At least 95% of medium severity gaps addressed
3. Tests pass after fixes
4. No new issues introduced
5. **All fixes are committed to git** (REQUIRED for validation)

## Output Examples

### Successful Completion
```json
{
  "completion_percentage": 100.0,
  "status": "complete",
  "gaps_fixed": [
    "Added 5 unit tests for worktree cleanup",
    "Implemented error recovery with fallback",
    "Added comprehensive documentation"
  ],
  "files_modified": [
    "src/worktree.rs",
    "tests/worktree_test.rs"
  ]
}
```

### Partial Completion
```json
{
  "completion_percentage": 92.0,
  "status": "incomplete",
  "gaps_fixed": [
    "Added critical tests",
    "Fixed error handling"
  ],
  "gaps_remaining": [
    "Minor documentation updates"
  ],
  "files_modified": [
    "src/worktree.rs"
  ]
}
```

## Important Notes

1. **Always preserve existing functionality** when fixing gaps
2. **Follow project conventions** from CONVENTIONS.md
3. **Run tests** after each significant fix
4. **Output valid JSON** for workflow parsing
5. **MUST create git commit** - Prodigy requires this to verify fixes were made
6. **Commit atomically** - all fixes in one commit with clear message
# Validate Spec Implementation Command

Validates that a specification has been completely and correctly implemented by checking code changes against spec requirements.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-validate-spec <spec-identifier> [--output <filepath>]
```

Examples:
- `/prodigy-validate-spec 01` to validate implementation of spec 01
- `/prodigy-validate-spec 01 --output .prodigy/validation-result.json` to validate and save to file
- `/prodigy-validate-spec iteration-1234567890-improvements` to validate temporary improvement spec

## What This Command Does

1. **Reads the Specification**
   - Locates the spec file based on provided identifier
   - If spec file was already deleted (after implementation), reconstructs requirements from git history
   - Extracts all implementation requirements and success criteria

2. **Analyzes Implementation**
   - Reviews recent git changes (since before implementation)
   - Checks each requirement against actual code changes
   - Verifies success criteria are met
   - Identifies any missing or incomplete implementations

3. **Outputs Validation Result**
   - Produces JSON-formatted validation result for Prodigy to parse
   - Includes completion percentage and detailed gap analysis
   - Provides actionable feedback for incomplete items

## Execution Process

### Step 1: Determine Output Location and Read Specification

The command will:
- Parse $ARGUMENTS to extract:
  - The spec identifier (required)
  - The `--output` parameter with filepath (required when called from workflow)
- If no spec identifier, fail with error message
- If no `--output` parameter, default to `.prodigy/validation-result.json`
- Try to locate spec file in standard locations:
  - Numeric IDs: `specs/{number}-*.md`
  - Iteration IDs: `specs/temp/{id}.md`
- If spec file not found (likely deleted after implementation):
  - Check git history for recently deleted spec files
  - Reconstruct requirements from commit messages and diffs

### Step 2: Analyze ALL Implementation Commits

**CRITICAL**: When running in a Prodigy worktree, you MUST analyze ALL commits in the current session, including recovery commits from previous validation attempts.

Review implementation by:
- **Determine commit range**: Get ALL commits in worktree using `git log HEAD --not master --oneline` (or appropriate base branch)
- **This ensures you include**:
  - Initial implementation commits
  - Recovery commits from `on_incomplete` attempts
  - Any other commits in the worktree session
- **Analyze files changed**: For EACH commit in range, check what files were modified
- **Read current state**: Read the current working tree state of modified files (not historical versions)
- **Check test coverage**: Verify tests were added for new code
- **Verify coding standards**: Ensure standards compliance

**Example for spec 118 scenario**:
```bash
# Get all worktree commits (returns 2 commits):
# e696c9c fix: complete spec 118 implementation gaps  <- MUST analyze this!
# 2e8b1e4 feat: implement spec 118 - generalize book documentation commands

# Analyze files from BOTH commits to see complete implementation
```

**Why this matters**: If the first implementation was incomplete (85%), and a recovery commit fixed the gaps (making it 100%), you MUST include the recovery commit in your analysis or you'll incorrectly report the same gaps again.

### Step 3: Validate Against Requirements (Using Current Working Tree State)

**IMPORTANT**: Validate against the CURRENT state of files in the working tree, not historical versions. This ensures you catch fixes made in recovery commits.

For each spec requirement:
- **Code Requirements**: Check if required files/functions exist in the CURRENT working tree
- **Test Requirements**: Verify tests were added (check current test files)
- **Documentation**: Ensure docs were updated (read current doc files)
- **Architecture**: Confirm design patterns followed (check current code structure)
- **Success Criteria**: Validate each criterion is met (based on current state)

**Validation Process**:
1. List ALL files that were modified across ALL commits in worktree
2. Read the CURRENT content of those files (not historical versions)
3. Check each spec requirement against current file content
4. Mark requirement as "implemented" ONLY if current files satisfy it
5. Mark as "missing" if current files don't satisfy it

**Example**: If spec requires "Create docs/book-documentation-workflow.md":
- ✓ CORRECT: Read current working tree, find the file exists → mark as implemented
- ✗ WRONG: Only check first commit, miss that recovery commit created it → mark as missing

### Step 4: Generate Validation Report

Create detailed validation result:
- Calculate completion percentage
- List implemented requirements
- Identify missing/incomplete items
- Provide specific gaps with locations
- Suggest fixes for gaps

### Step 5: Output JSON Result

**CRITICAL**: Write validation results to the output file:

1. **Use output location from `--output` parameter**:
   - This should have been parsed from $ARGUMENTS
   - If not provided, use default `.prodigy/validation-result.json`

2. **Write JSON to file**:
   - Create parent directories if needed
   - Write the JSON validation result to the file
   - Ensure file is properly closed and flushed

3. **Do NOT output JSON to stdout** - Prodigy will read from the file

The JSON format is:

```json
{
  "completion_percentage": 95.0,
  "status": "incomplete",
  "implemented": [
    "Created worktree cleanup function",
    "Added cleanup to orchestrator",
    "Implemented error handling"
  ],
  "missing": [
    "Unit tests for cleanup function"
  ],
  "gaps": {
    "tests_missing": {
      "description": "No unit tests for worktree_cleanup function",
      "location": "src/orchestrator.rs:145",
      "severity": "medium",
      "suggested_fix": "Add unit tests covering cleanup scenarios"
    }
  }
}
```

## Validation Rules

### Completion Scoring

- **100%**: All requirements fully implemented with tests
- **90-99%**: Core functionality complete, minor gaps
- **70-89%**: Most requirements met, some important gaps
- **50-69%**: Partial implementation, significant gaps
- **Below 50%**: Major implementation issues

### Requirement Categories

1. **Critical (Must Have)**
   - Core functionality from spec
   - Error handling
   - Security considerations
   - Each missing item reduces score by 15-20%

2. **Important (Should Have)**
   - Tests and documentation
   - Performance optimizations
   - Code quality standards
   - Each missing item reduces score by 5-10%

3. **Nice to Have**
   - Additional improvements
   - Extended documentation
   - Each missing item reduces score by 1-3%

## Automation Mode Behavior

**Automation Detection**: Checks for `PRODIGY_AUTOMATION=true` or `PRODIGY_VALIDATION=true` environment variables.

**In Automation Mode**:
- Skip interactive prompts
- Output minimal progress messages
- Always output JSON result at the end
- Exit with appropriate code (0 for complete, 1 for incomplete)

## Error Handling

The command will:
- Handle missing spec files gracefully
- Work with partial implementations
- Provide clear error messages
- Always output valid JSON (even on errors)

## Example Validation Outputs

### Successful Validation (100%)
```json
{
  "completion_percentage": 100.0,
  "status": "complete",
  "implemented": [
    "All core functionality",
    "Complete test coverage",
    "Documentation updated"
  ],
  "missing": [],
  "gaps": {}
}
```

### Incomplete Implementation (85%)
```json
{
  "completion_percentage": 85.0,
  "status": "incomplete",
  "implemented": [
    "Core worktree cleanup logic",
    "Integration with orchestrator"
  ],
  "missing": [
    "Unit tests",
    "Error recovery handling"
  ],
  "gaps": {
    "missing_tests": {
      "description": "No tests for cleanup_worktree function",
      "location": "src/worktree.rs:234",
      "severity": "high",
      "suggested_fix": "Add tests for success and error cases"
    },
    "incomplete_error_handling": {
      "description": "Missing error recovery when cleanup fails",
      "location": "src/orchestrator.rs:567",
      "severity": "medium",
      "suggested_fix": "Add fallback cleanup mechanism"
    }
  }
}
```

### Validation Failure
```json
{
  "completion_percentage": 0.0,
  "status": "failed",
  "implemented": [],
  "missing": ["Unable to validate: spec not found and no recent implementation detected"],
  "gaps": {},
  "raw_output": "Error details here"
}
```

## Integration with Workflows

This command is designed to work with Prodigy workflows:

1. **Workflow calls validation command**
2. **Command outputs JSON result**
3. **Prodigy parses result and checks threshold**
4. **If incomplete, workflow triggers retry logic**
5. **Process repeats up to max_attempts**

## Important Implementation Notes

1. **Parse arguments correctly** - Extract spec ID and the `--output` parameter from $ARGUMENTS
2. **Write JSON to file**:
   - Use path from `--output` parameter, or default `.prodigy/validation-result.json`
   - Create parent directories if they don't exist
   - Write complete JSON validation result to the file
4. **Always write valid JSON** to the file, even if validation fails
5. **Exit code 0** indicates command ran successfully (regardless of validation result)
6. **Completion percentage** determines if validation passed based on threshold
7. **Gap details** help subsequent commands fix issues
8. **Keep JSON compact** - Prodigy will parse it programmatically
9. **Do NOT output JSON to stdout** - only progress messages should go to stdout

## Troubleshooting Validation Issues

### Issue: Validation reports same gaps after recovery commit fixed them

**Symptoms**:
- First validation shows 85% complete with gaps
- Recovery commit fixes all gaps
- Second validation still shows 85% with same gaps

**Root Cause**: Not analyzing ALL commits in worktree, missing recovery commits

**Fix**:
1. Run `git log HEAD --not master --oneline` to see ALL worktree commits
2. Analyze files from EVERY commit in the output
3. Read CURRENT working tree state of files, not historical versions
4. Validate against current state

**Example**:
```bash
# Shows 2 commits in worktree
git log HEAD --not master --oneline
# e696c9c fix: complete spec 118 implementation gaps  <- Don't miss this!
# 2e8b1e4 feat: implement spec 118

# Check if docs/book-documentation-workflow.md exists NOW
ls -la docs/book-documentation-workflow.md  # File exists!

# Therefore: Mark "Create docs/book-documentation-workflow.md" as implemented
```

### Issue: Validation marks items as missing that actually exist

**Root Cause**: Reading old commit state instead of current working tree

**Fix**: Always use current file state, not git history:
```bash
# ✓ CORRECT: Read current file
cat docs/book-documentation-workflow.md

# ✗ WRONG: Read file from specific commit
git show 2e8b1e4:docs/book-documentation-workflow.md  # May not exist in this commit!
```
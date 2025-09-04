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

### Step 2: Analyze Recent Changes

Review implementation by:
- Getting list of files changed in recent commits
- Reading modified files to understand implementation
- Checking test coverage for new code
- Verifying coding standards compliance

### Step 3: Validate Against Requirements

For each spec requirement:
- **Code Requirements**: Check if required files/functions exist
- **Test Requirements**: Verify tests were added
- **Documentation**: Ensure docs were updated
- **Architecture**: Confirm design patterns followed
- **Success Criteria**: Validate each criterion is met

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
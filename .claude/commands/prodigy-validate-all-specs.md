# Validate All Specs Command

Validates multiple specifications for completeness, focus, and alignment with original intent. This command supports batch validation of multiple specs created from a single request.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-validate-all-specs <spec-files> --original <description> [--output <filepath>]
```

Examples:
- `/prodigy-validate-all-specs specs/102-auth.md specs/103-session.md --original "Add authentication and session management"`
- `/prodigy-validate-all-specs ${step.files_added:specs/*.md} --original '$ARG' --output .prodigy/spec-validation.json`

## What This Command Does

1. **Reads Multiple Specifications**
   - Accepts space-separated list of spec files
   - Parses each specification's requirements and criteria
   - Extracts implementation scope from each spec

2. **Validates Against Original Intent**
   - Compares collective specs with original description
   - Ensures all requested features are covered
   - Identifies any missing requirements across all specs

3. **Checks Each Spec Quality**
   - Verifies each spec is focused (single responsibility)
   - Ensures requirements are testable
   - Checks for clear acceptance criteria
   - Validates no unnecessary overlap between specs

4. **Outputs Aggregated Validation Result**
   - JSON result with overall and per-spec completeness scores
   - Lists any gaps or issues in each spec
   - Provides incomplete_specs list for refinement

## Execution Process

### Step 1: Parse Arguments

- Extract spec files list from first part of $ARGUMENTS
- Extract original description after --original flag
- Parse --output parameter (default: `.prodigy/spec-validation.json`)
- Handle empty file list gracefully (error if no specs provided)

### Step 2: Read and Analyze Each Spec

For each spec file provided:
- Read spec file content
- Parse YAML frontmatter for metadata
- Check for required sections:
  - Context: Clear background provided
  - Objective: Specific and measurable
  - Requirements: Complete functional/non-functional requirements
  - Acceptance Criteria: Testable and complete
  - Technical Details: Sufficient for implementation
  - Dependencies: Properly identified
  - Testing Strategy: Comprehensive

### Step 3: Evaluate Spec Focus

For each spec, determine if it's well-focused:
- Single clear objective
- 3-8 acceptance criteria
- Cohesive requirements
- Can be implemented in 1-2 days
- No mixing of unrelated concerns

### Step 4: Cross-Spec Analysis

Analyze the collection of specs:
- Check for appropriate separation of concerns
- Identify any missing functionality from original request
- Verify no unnecessary duplication
- Ensure logical grouping of features

### Step 5: Generate Aggregated Validation Report

Create comprehensive JSON validation result:

```json
{
  "overall_completion_percentage": 95.0,
  "all_focused": true,
  "specs": [
    {
      "file": "specs/102-authentication.md",
      "completion_percentage": 100.0,
      "is_focused": true,
      "status": "complete",
      "gaps": {},
      "quality": {
        "has_context": true,
        "has_objective": true,
        "has_requirements": true,
        "has_acceptance_criteria": true,
        "has_technical_details": true,
        "has_testing_strategy": true
      }
    },
    {
      "file": "specs/103-session-management.md",
      "completion_percentage": 90.0,
      "is_focused": true,
      "status": "incomplete",
      "gaps": {
        "missing_tests": {
          "description": "Test strategy not comprehensive",
          "severity": "medium",
          "suggested_fix": "Add integration test scenarios for session timeout"
        }
      },
      "quality": {
        "has_context": true,
        "has_objective": true,
        "has_requirements": true,
        "has_acceptance_criteria": true,
        "has_technical_details": true,
        "has_testing_strategy": false
      }
    }
  ],
  "incomplete_specs": ["specs/103-session-management.md"],
  "original_intent_coverage": {
    "covered": [
      "User authentication with JWT",
      "Session management with timeout",
      "Password hashing and validation"
    ],
    "missing": []
  },
  "validation_timestamp": "2024-01-18T12:00:00Z",
  "gaps": {
    "specs/103-session-management.md": {
      "missing_tests": {
        "description": "Test strategy not comprehensive",
        "severity": "medium"
      }
    }
  },
  "status": "incomplete",
  "validation_type": "batch_spec_completeness"
}
```

### Step 6: Write JSON Output

Write the aggregated validation result to the output file, ensuring:
- Proper JSON formatting
- All specs are included in the report
- Clear identification of incomplete specs for refinement
- Comprehensive gap analysis for iterative improvement

## Validation Rules

### Completeness Scoring

Overall completion percentage is calculated as:
- Average of all individual spec completion percentages
- Weighted by spec complexity (if applicable)

### Focus Analysis

A spec collection is considered focused when:
- Each spec has a single responsibility
- No unnecessary overlap between specs
- Clear boundaries between specifications
- All specs together cover the original request

### Gap Identification

Gaps are identified at two levels:
1. **Per-Spec Gaps**: Missing or incomplete sections in individual specs
2. **Coverage Gaps**: Missing functionality from the original request

## Error Handling

- If no specs provided: Return error with message
- If spec files don't exist: List missing files in error
- If invalid spec format: Include parsing errors in validation result
- If original description missing: Use empty string as fallback

## Output Structure

The command always outputs a valid JSON file that can be used by:
- The workflow validation step to determine if refinement is needed
- The refinement command to understand what needs improvement
- The reporting command to summarize what was created
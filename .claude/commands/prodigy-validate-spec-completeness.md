# Validate Spec Completeness Command

Validates that a newly generated specification is complete, focused, and ready for implementation.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-validate-spec-completeness <spec-identifier> <original-description> [--output <filepath>]
```

Examples:
- `/prodigy-validate-spec-completeness 01 "Add user authentication" --output .prodigy/spec-validation.json`
- `/prodigy-validate-spec-completeness 02 "Implement caching layer"`

## What This Command Does

1. **Reads the Generated Specification**
   - Locates spec file based on identifier
   - Parses requirements and acceptance criteria
   - Extracts implementation scope

2. **Validates Against Original Intent**
   - Compares spec with original description
   - Ensures all requested features are covered
   - Identifies any missing requirements

3. **Checks Spec Quality**
   - Verifies spec is focused (single responsibility)
   - Ensures requirements are testable
   - Checks for clear acceptance criteria
   - Identifies if spec needs to be split

4. **Outputs Validation Result**
   - JSON result with completeness score
   - Lists any gaps or issues
   - Recommends if spec should be split

## Execution Process

### Step 1: Parse Arguments and Read Spec

- Extract spec identifier from $ARGUMENTS
- Extract original description (everything after spec ID)
- Parse --output parameter (default: `.prodigy/spec-validation.json`)
- Read spec file from `specs/{number}-*.md`

### Step 2: Analyze Spec Completeness

Check for required sections:
- **Context**: Must provide clear background
- **Objective**: Must be specific and measurable
- **Requirements**: Must cover all aspects of original description
- **Acceptance Criteria**: Must be testable and complete
- **Technical Details**: Must be sufficient for implementation
- **Dependencies**: Must be identified
- **Testing Strategy**: Must be comprehensive

### Step 3: Evaluate Spec Focus

Determine if spec should be split:
- **Too Broad**: Multiple unrelated features
- **Too Complex**: More than 10 acceptance criteria
- **Mixed Concerns**: Combines different system layers
- **Large Scope**: Would take >2 days to implement

Signs spec is well-focused:
- Single clear objective
- 3-8 acceptance criteria
- Cohesive requirements
- Can be implemented in 1-2 days

### Step 4: Check Against Original Description

Compare spec with original request:
- All requested features included
- No significant omissions
- Intent accurately captured
- Scope appropriately defined

### Step 5: Generate Validation Report

Create JSON validation result with:
- Completeness percentage
- List of missing elements
- Recommendation to split (if needed)
- Specific gaps to address

### Step 6: Write JSON Output

Write validation result to output file:

```json
{
  "completion_percentage": 85.0,
  "status": "incomplete",
  "validation_type": "spec_completeness",
  "spec_quality": {
    "has_context": true,
    "has_objective": true,
    "has_requirements": true,
    "has_acceptance_criteria": true,
    "has_technical_details": false,
    "has_testing_strategy": true
  },
  "completeness_issues": [
    "Missing technical implementation details",
    "Acceptance criteria too vague for testing"
  ],
  "focus_analysis": {
    "is_focused": false,
    "reason": "Spec combines authentication and authorization - should be split",
    "suggested_splits": [
      "Authentication mechanism (JWT tokens, login/logout)",
      "Authorization system (roles, permissions, access control)"
    ]
  },
  "original_intent_coverage": {
    "covered": [
      "User login functionality",
      "JWT token generation"
    ],
    "missing": [
      "Password reset flow",
      "Session management"
    ]
  },
  "gaps": {
    "missing_technical_details": {
      "description": "No implementation approach specified",
      "severity": "high",
      "suggested_fix": "Add technical details section with architecture decisions"
    },
    "vague_criteria": {
      "description": "Acceptance criteria 'System should be secure' is not testable",
      "severity": "medium",
      "suggested_fix": "Replace with specific security requirements (e.g., 'Passwords must be hashed with bcrypt', 'Tokens expire after 24 hours')"
    },
    "scope_too_broad": {
      "description": "Spec includes both authentication and authorization",
      "severity": "high",
      "suggested_fix": "Split into two specs: one for authentication, one for authorization"
    }
  }
}
```

## Validation Rules

### Completeness Scoring

- **100%**: All sections complete, focused, testable
- **90-99%**: Minor gaps in details or criteria
- **70-89%**: Missing important sections or too broad
- **50-69%**: Significant gaps or needs major refactoring
- **Below 50%**: Spec needs complete rewrite

### Quality Checks

1. **Testability**
   - Each acceptance criterion can be verified
   - Requirements are specific and measurable
   - Clear success/failure conditions

2. **Focus**
   - Single responsibility principle
   - Cohesive set of requirements
   - Reasonable implementation scope

3. **Completeness**
   - All original requirements addressed
   - No ambiguous language
   - Sufficient detail for implementation

4. **Clarity**
   - Unambiguous requirements
   - Clear technical approach
   - Well-defined boundaries

## Spec Splitting Criteria

Recommend splitting when:
- More than 10 acceptance criteria
- Multiple unrelated features
- Different architectural layers mixed
- Implementation would exceed 2 days
- Requirements span multiple modules

## Output Examples

### Well-Focused, Complete Spec
```json
{
  "completion_percentage": 100.0,
  "status": "complete",
  "validation_type": "spec_completeness",
  "spec_quality": {
    "has_context": true,
    "has_objective": true,
    "has_requirements": true,
    "has_acceptance_criteria": true,
    "has_technical_details": true,
    "has_testing_strategy": true
  },
  "completeness_issues": [],
  "focus_analysis": {
    "is_focused": true,
    "reason": "Single clear objective with cohesive requirements"
  },
  "original_intent_coverage": {
    "covered": ["All requested features"],
    "missing": []
  },
  "gaps": {}
}
```

### Spec Needs Splitting
```json
{
  "completion_percentage": 70.0,
  "status": "incomplete",
  "validation_type": "spec_completeness",
  "focus_analysis": {
    "is_focused": false,
    "reason": "Combines multiple unrelated features",
    "suggested_splits": [
      "User authentication (login, logout, session management)",
      "User profile management (CRUD operations)",
      "Email notification system"
    ]
  },
  "gaps": {
    "needs_splitting": {
      "description": "Spec is too broad and should be split into 3 separate specs",
      "severity": "critical",
      "suggested_fix": "Create separate specs for each suggested split"
    }
  }
}
```

## Important Implementation Notes

1. **Always validate against original description** - Ensure intent is preserved
2. **Check for testability** - All criteria must be verifiable
3. **Assess implementation scope** - Keep specs focused and achievable
4. **Write JSON to file** - Use --output parameter or default location
5. **Include specific fixes** - Provide actionable suggestions for gaps
6. **Consider splitting early** - Better to have focused specs than bloated ones
7. **Exit code 0** - Command success (regardless of validation result)
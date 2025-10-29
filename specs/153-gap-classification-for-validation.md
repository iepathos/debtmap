---
number: 153
title: Gap Classification System for Validation
category: testing
priority: high
status: draft
dependencies: []
created: 2025-10-29
---

# Specification 153: Gap Classification System for Validation

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Prodigy workflows currently fail when validation is incomplete (<100%), even when the missing items are deferred/optional requirements rather than critical functionality. This creates a rigid system that doesn't distinguish between:

1. **Critical gaps**: Core functionality missing
2. **Important gaps**: Tests or documentation missing
3. **Optional gaps**: Deferred validation tests, nice-to-have features

**Real-world failure case (spec 137)**:
- Validation returned 91.7% complete
- Gap: "Integration test for ripgrep standard.rs" (explicitly deferred in spec)
- Recovery command (`/prodigy-complete-spec`) correctly determined no fixes needed
- Workflow failed with: "commit_required=true but no commits were created"
- **Root cause**: Workflow assumed incomplete validation always requires code changes

**Problems with current system**:
1. No way to mark gaps as "required" vs "optional/deferred"
2. Workflow always fails if validation <100%
3. `commit_required: true` assumes all gaps need code changes
4. No distinction between functional completeness and perfection
5. Forces implementation of deferred items or manual workflow overrides

## Objective

Implement a gap classification system that distinguishes between required, important, and optional gaps, allowing workflows to proceed when core functionality is complete while flagging legitimate implementation issues.

## Requirements

### Functional Requirements

1. **Gap Classification Schema**
   - Add `requirement_type` field to gap data: "required" | "important" | "optional"
   - Allow validation to specify which gaps block workflow completion
   - Enable workflows to filter gaps by requirement type
   - Support backward compatibility with existing gap structures

2. **Enhanced Validation Output**
   - Include `required_gaps` array with only blocking gaps
   - Separate `optional_gaps` for informational purposes
   - Add `functional_completion_percentage` (ignoring optional gaps)
   - Maintain existing `completion_percentage` for full scoring

3. **Workflow Threshold Options**
   - Support `functional_threshold` (defaults to 100% of required items)
   - Support `total_threshold` (defaults to 100% of all items)
   - Allow workflows to choose which threshold to enforce
   - Enable hybrid approach: require 100% functional, allow 80% total

4. **Smart Commit Detection**
   - Workflow only requires commit if `required_gaps` exist
   - Allow no-commit completion if only optional gaps remain
   - Provide clear messaging about what gaps remain
   - Support manual override for edge cases

5. **Gap Classification in Commands**
   - Update `/prodigy-validate-spec` to classify gaps by requirement type
   - Update `/prodigy-complete-spec` to prioritize required gaps
   - Add `--require-all` flag to treat optional gaps as required
   - Add `--functional-only` flag to ignore optional gaps

### Non-Functional Requirements

1. **Backward Compatibility**: Existing workflows continue to work without modification
2. **Clear Semantics**: Gap classification is intuitive and well-documented
3. **Flexibility**: Support different rigor levels for different spec types
4. **Transparency**: Users understand why workflows pass or fail
5. **Maintainability**: Gap classification rules are easy to update

## Acceptance Criteria

- [ ] Gap data structure includes `requirement_type` field
- [ ] Validation output includes both `functional_completion_percentage` and `completion_percentage`
- [ ] Validation separates `required_gaps` from `optional_gaps`
- [ ] Workflows support `functional_threshold` configuration option
- [ ] `/prodigy-validate-spec` classifies gaps based on spec requirements
- [ ] `/prodigy-complete-spec` prioritizes required gaps over optional ones
- [ ] Workflow `commit_required` is conditional on `required_gaps` being non-empty
- [ ] Spec 137 scenario passes validation with 100% functional completion
- [ ] Workflows can enforce different thresholds for different spec categories
- [ ] Documentation explains gap classification system
- [ ] Backward compatibility maintained for existing workflows
- [ ] Integration tests verify gap classification behavior

## Technical Details

### Implementation Approach

#### 1. Enhanced Gap Data Structure

```json
{
  "gap_id": {
    "description": "No integration test validating god object analysis on ripgrep standard.rs",
    "location": "tests/ directory",
    "severity": "medium",
    "requirement_type": "optional",
    "suggested_fix": "This test was deferred as it validates quality, not functionality",
    "rationale": "Spec explicitly noted this test was 'deferred (validation test)'"
  }
}
```

**Requirement types**:
- **`required`**: Core functionality, must be implemented for spec to be complete
- **`important`**: Tests, documentation, error handling - should be implemented but not blocking
- **`optional`**: Deferred items, nice-to-have features, quality validation tests

#### 2. Enhanced Validation Output Format

```json
{
  "completion_percentage": 91.7,
  "functional_completion_percentage": 100.0,
  "status": "functionally_complete",
  "implemented": [
    "Call pattern-based responsibility naming",
    "Interface size estimation",
    "Unit tests for both features"
  ],
  "required_gaps": [],
  "important_gaps": [],
  "optional_gaps": [
    "Integration test for ripgrep standard.rs"
  ],
  "gaps": {
    "ripgrep_validation_test": {
      "description": "No integration test validating god object analysis on ripgrep standard.rs",
      "location": "tests/ directory",
      "severity": "medium",
      "requirement_type": "optional",
      "suggested_fix": "The spec explicitly noted this test was 'deferred (validation test)'",
      "rationale": "This is a validation test to verify recommendation quality on real-world code, not a critical functional requirement"
    }
  },
  "summary": {
    "total_requirements": 12,
    "implemented_requirements": 11,
    "required_implemented": 10,
    "required_total": 10,
    "important_implemented": 1,
    "important_total": 1,
    "optional_implemented": 0,
    "optional_total": 1
  }
}
```

**Status values**:
- `complete`: 100% of all requirements (including optional)
- `functionally_complete`: 100% of required requirements
- `incomplete`: Missing required requirements

#### 3. Workflow Configuration Extensions

```yaml
# workflows/implement.yml
commands:
  - claude: "/prodigy-implement-spec $ARG"
    commit_required: true
    validate:
      claude: "/prodigy-validate-spec $ARG --output .prodigy/validation-result.json"
      result_file: ".prodigy/validation-result.json"

      # NEW: Support multiple threshold types
      thresholds:
        functional: 100    # Required gaps must be 100% complete
        total: 90          # Total completion can be 90%

      # Alternative: single threshold mode (backward compatible)
      # threshold: 100

      on_incomplete:
        # Only runs if required_gaps is non-empty OR functional_completion < threshold
        claude: "/prodigy-complete-spec $ARG --gaps ${validation.required_gaps} --priority required"
        max_attempts: 5
        fail_workflow: false

        # NEW: Conditional commit requirement
        commit_required:
          when: "required_gaps_exist"  # Only require commit if fixing required gaps
```

#### 4. Gap Classification Logic in `/prodigy-validate-spec`

```rust
// Pseudo-code for gap classification
fn classify_gap(gap: &ValidationGap, spec: &Specification) -> RequirementType {
    // Check if spec explicitly marks this as optional/deferred
    if spec.deferred_requirements.contains(&gap.requirement_id) {
        return RequirementType::Optional;
    }

    // Check spec metadata for requirement classification
    if let Some(requirement) = spec.find_requirement(&gap.requirement_id) {
        if requirement.tags.contains("deferred") ||
           requirement.tags.contains("optional") {
            return RequirementType::Optional;
        }

        if requirement.tags.contains("validation-test") ||
           requirement.tags.contains("quality-check") {
            // Validation tests are optional unless spec says otherwise
            return RequirementType::Optional;
        }
    }

    // Classify by gap type
    match gap.gap_type {
        GapType::MissingCoreFunction => RequirementType::Required,
        GapType::MissingIntegration => RequirementType::Required,
        GapType::MissingErrorHandling => RequirementType::Important,
        GapType::MissingTests => RequirementType::Important,
        GapType::MissingDocumentation => RequirementType::Important,
        GapType::ValidationTest => RequirementType::Optional,
        GapType::PerformanceOptimization => RequirementType::Optional,
        GapType::NiceToHave => RequirementType::Optional,
    }
}

fn calculate_functional_completion(validation: &ValidationResult) -> f64 {
    let required_gaps: Vec<_> = validation.gaps
        .iter()
        .filter(|g| g.requirement_type == RequirementType::Required)
        .collect();

    if validation.summary.required_total == 0 {
        return 100.0; // No required items = functionally complete
    }

    let implemented = validation.summary.required_implemented;
    let total = validation.summary.required_total;

    (implemented as f64 / total as f64) * 100.0
}
```

#### 5. Spec Metadata for Requirement Classification

Extend spec frontmatter to support requirement classification:

```markdown
---
number: 137
title: Call Pattern-Based Analysis and Interface Size Estimation
category: foundation
priority: high
status: draft
dependencies: [133]
created: 2025-10-27

# NEW: Requirement classification
requirements:
  required:
    - "Call pattern-based responsibility naming"
    - "Interface size estimation"
  important:
    - "Unit tests for call pattern detection"
    - "Unit tests for interface size estimation"
  optional:
    - "Integration test for ripgrep standard.rs"

deferred:
  - "Integration test for ripgrep standard.rs"
  - reason: "External dependency not in test fixtures, deferred to future validation"
---

## Acceptance Criteria

- [x] Intra-module call graph built (required)
- [x] Functions grouped into cohesive clusters (required)
...
- [ ] ripgrep standard.rs validation test (optional, deferred)
```

#### 6. Updated `/prodigy-complete-spec` Behavior

```bash
# Command signature
/prodigy-complete-spec <spec-id> [--gaps <gaps-json>] [--priority <required|important|all>]

# Examples:
/prodigy-complete-spec 137 --gaps ${validation.required_gaps} --priority required
/prodigy-complete-spec 137 --gaps ${validation.gaps} --priority all
```

**Logic**:
1. Parse `--priority` flag (default: "required")
2. Filter gaps by priority level
3. If no gaps match priority filter:
   - Output: "No {priority} gaps to fix, implementation is functionally complete"
   - Create NO commit
   - Return success with 100% functional completion
4. Otherwise:
   - Fix filtered gaps
   - Create commit with fixed gaps
   - Return completion status

#### 7. Workflow Threshold Evaluation

```rust
// Pseudo-code for threshold checking
fn should_trigger_recovery(
    validation: &ValidationResult,
    config: &WorkflowConfig
) -> bool {
    // Support both old and new threshold configs
    let (functional_threshold, total_threshold) = match &config.thresholds {
        Some(thresholds) => (thresholds.functional, thresholds.total),
        None => {
            // Backward compatibility: single threshold applies to both
            let threshold = config.threshold.unwrap_or(100.0);
            (threshold, threshold)
        }
    };

    // Check functional threshold (required gaps only)
    if validation.functional_completion_percentage < functional_threshold {
        return true;
    }

    // Check total threshold (all gaps)
    if validation.completion_percentage < total_threshold {
        // Only trigger if there are important gaps
        // (optional gaps don't block by default)
        return !validation.important_gaps.is_empty();
    }

    false
}

fn should_require_commit(
    validation: &ValidationResult,
    recovery_config: &RecoveryConfig
) -> bool {
    match &recovery_config.commit_required {
        CommitRequirement::Always => true,
        CommitRequirement::Never => false,
        CommitRequirement::Conditional { when } => {
            match when.as_str() {
                "required_gaps_exist" => !validation.required_gaps.is_empty(),
                "any_gaps_exist" => !validation.gaps.is_empty(),
                "important_gaps_exist" => {
                    !validation.required_gaps.is_empty() ||
                    !validation.important_gaps.is_empty()
                }
                _ => true // Default to requiring commit
            }
        }
    }
}
```

### Architecture Changes

**Modified components**:
1. **Prodigy workflow engine** (`implement.yml`)
   - Add threshold evaluation logic
   - Support conditional commit requirements
   - Parse new validation output format

2. **`/prodigy-validate-spec` command**
   - Add gap classification logic
   - Output enhanced validation format
   - Read spec metadata for requirement types

3. **`/prodigy-complete-spec` command**
   - Filter gaps by priority
   - Support no-op completion for optional-only gaps
   - Update commit logic to be conditional

4. **Spec template**
   - Add requirement classification section
   - Document deferred items clearly
   - Include rationale for optional requirements

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequirementType {
    Required,
    Important,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationGap {
    pub description: String,
    pub location: String,
    pub severity: Severity,
    pub requirement_type: RequirementType,
    pub suggested_fix: String,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub total_requirements: usize,
    pub implemented_requirements: usize,
    pub required_total: usize,
    pub required_implemented: usize,
    pub important_total: usize,
    pub important_implemented: usize,
    pub optional_total: usize,
    pub optional_implemented: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub completion_percentage: f64,
    pub functional_completion_percentage: f64,
    pub status: ValidationStatus,
    pub implemented: Vec<String>,
    pub required_gaps: Vec<String>,
    pub important_gaps: Vec<String>,
    pub optional_gaps: Vec<String>,
    pub gaps: HashMap<String, ValidationGap>,
    pub summary: ValidationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationStatus {
    Complete,
    FunctionallyComplete,
    Incomplete,
}
```

## Dependencies

- **Prerequisites**: None (foundational improvement)
- **Affected Components**:
  - Prodigy workflow engine
  - `.claude/commands/prodigy-validate-spec.md`
  - `.claude/commands/prodigy-complete-spec.md`
  - `workflows/implement.yml`
  - Spec template
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

**Test gap classification logic**:
```rust
#[test]
fn test_classify_deferred_validation_test() {
    let gap = ValidationGap {
        requirement_id: "ripgrep_validation",
        gap_type: GapType::ValidationTest,
        ...
    };

    let spec = Specification {
        deferred_requirements: vec!["ripgrep_validation".to_string()],
        ...
    };

    assert_eq!(classify_gap(&gap, &spec), RequirementType::Optional);
}

#[test]
fn test_classify_missing_core_function() {
    let gap = ValidationGap {
        gap_type: GapType::MissingCoreFunction,
        ...
    };

    assert_eq!(classify_gap(&gap, &spec), RequirementType::Required);
}

#[test]
fn test_functional_completion_calculation() {
    let validation = ValidationResult {
        summary: ValidationSummary {
            required_total: 10,
            required_implemented: 10,
            optional_total: 1,
            optional_implemented: 0,
            ...
        },
        ...
    };

    assert_eq!(validation.functional_completion_percentage, 100.0);
    assert_eq!(validation.completion_percentage, 90.9);
}
```

### Integration Tests

**Test spec 137 scenario**:
```bash
# Should pass with functional completion
/prodigy-validate-spec 137 --output test-result.json

# Expected output:
{
  "functional_completion_percentage": 100.0,
  "completion_percentage": 91.7,
  "status": "functionally_complete",
  "required_gaps": [],
  "optional_gaps": ["Integration test for ripgrep standard.rs"]
}

# Workflow should NOT trigger recovery for optional gaps
# If it does, recovery should return success without commit
```

**Test workflow threshold evaluation**:
```yaml
# Test Case 1: Functional threshold met, total threshold not met
validate:
  thresholds:
    functional: 100
    total: 95
# Should: Pass if functional=100%, total=91.7% (no recovery needed)

# Test Case 2: Functional threshold not met
validate:
  thresholds:
    functional: 100
# Should: Trigger recovery if functional<100%

# Test Case 3: Backward compatibility
validate:
  threshold: 100
# Should: Behave as before (both functional and total must be 100%)
```

### Manual Validation Tests

1. **Run spec 137 through workflow**:
   - Should pass validation with 100% functional completion
   - Should NOT fail on missing optional ripgrep test
   - Should create commit for implementation, but not for recovery

2. **Test with spec having missing required gaps**:
   - Should trigger recovery
   - Recovery should fix required gaps
   - Recovery should create commit
   - Should fail if required gaps can't be fixed

3. **Test with spec having only important gaps**:
   - Should trigger recovery (configurable)
   - Recovery should fix important gaps
   - Should pass at lower total threshold if configured

## Documentation Requirements

### Code Documentation

- Document gap classification algorithm
- Explain requirement type semantics
- Provide examples of each requirement type
- Document workflow threshold logic

### User Documentation

**Add to workflow documentation**:
- How to classify requirements in specs
- How to configure thresholds
- Understanding validation output
- When to use functional vs total thresholds

**Add to spec template documentation**:
- How to mark deferred requirements
- Best practices for requirement classification
- Examples of required vs optional items

### Architecture Updates

Update ARCHITECTURE.md:
- Add section on validation and gap classification
- Explain workflow threshold evaluation
- Document the distinction between functional and total completion

## Implementation Notes

### Backward Compatibility

**Maintain existing behavior**:
- Single `threshold` parameter works as before (100% total)
- Existing validation output format still supported
- Gaps without `requirement_type` default to "required"
- Workflows without threshold config default to 100% total

**Migration path**:
1. Phase 1: Add new fields to validation output (additive change)
2. Phase 2: Update workflows to use new threshold options (opt-in)
3. Phase 3: Update spec template with requirement classification (opt-in)
4. Phase 4: Deprecate old format (long-term, optional)

### Requirement Type Guidelines

**Required**:
- Core functionality specified in spec objective
- Critical integrations
- Security features
- Data integrity features

**Important**:
- Unit tests for new functionality
- Error handling
- Documentation
- Integration with existing systems

**Optional**:
- Validation tests on external codebases
- Performance optimizations
- Nice-to-have features
- Deferred items with explicit rationale

### Default Classification Rules

When spec doesn't provide explicit classification:
1. Check acceptance criteria tags (required/important/optional)
2. Check if item is marked as deferred
3. Classify by gap type (core function = required, tests = important, etc.)
4. Default to "required" if uncertain (safe default)

## Migration and Compatibility

### Breaking Changes

None - fully backward compatible

### Backward Compatibility Strategy

1. **Validation output**: Include both old and new fields
2. **Workflow config**: Support both old `threshold` and new `thresholds`
3. **Gap structure**: Make `requirement_type` optional (default: "required")
4. **Commands**: Support both old and new argument formats

### Migration Guide

**For existing workflows**:
```yaml
# Old format (still works)
validate:
  threshold: 100

# New format (recommended)
validate:
  thresholds:
    functional: 100  # Must complete all required items
    total: 90        # Can leave optional items for later
```

**For existing specs**:
- No changes required immediately
- Can add requirement classification to frontmatter
- Can mark deferred items in acceptance criteria
- Default classification will be applied automatically

## Success Metrics

- Spec 137 workflow completes successfully with 91.7% total, 100% functional
- Zero false failures due to optional/deferred gaps
- Workflows still catch legitimate incomplete implementations
- >90% of specs work without explicit requirement classification
- Clear understanding of why workflows pass or fail
- Reduced manual intervention in workflow completion

## Example Scenarios

### Scenario 1: Functional Complete, Optional Gap

**Input**: Spec 137 validation
```json
{
  "functional_completion_percentage": 100.0,
  "completion_percentage": 91.7,
  "required_gaps": [],
  "optional_gaps": ["ripgrep validation test"]
}
```

**Workflow behavior**:
- Threshold: `functional: 100, total: 90`
- Result: PASS (functional=100%, total=91.7% > 90%)
- Recovery: NOT triggered
- Commit: Initial implementation only

### Scenario 2: Missing Required Functionality

**Input**: Spec validation
```json
{
  "functional_completion_percentage": 80.0,
  "completion_percentage": 85.0,
  "required_gaps": ["Interface size estimation function"],
  "important_gaps": ["Unit tests for estimation"]
}
```

**Workflow behavior**:
- Threshold: `functional: 100`
- Result: FAIL (functional=80% < 100%)
- Recovery: TRIGGERED for required gaps
- Commit: Required after recovery

### Scenario 3: Missing Tests Only

**Input**: Spec validation
```json
{
  "functional_completion_percentage": 100.0,
  "completion_percentage": 85.0,
  "required_gaps": [],
  "important_gaps": ["Unit tests for feature X"]
}
```

**Workflow behavior**:
- Threshold: `functional: 100, total: 90`
- Result: PASS or FAIL depending on `total` threshold
- Recovery: Triggered only if `total < 90%`
- Commit: Required if recovery runs

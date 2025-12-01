---
number: 199
title: Action Verb Standardization for Recommendations
category: optimization
priority: medium
status: draft
dependencies: [194, 197]
created: 2025-11-30
---

# Specification 199: Action Verb Standardization for Recommendations

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 194 (Scannable Summary Mode), 197 (Consistent Card Structure)

## Context

Debtmap currently uses inconsistent action language across recommendations:

```
#1: "Split by analysis phase: 1) Data collection 2) Pattern detection..."
#3: "Split into: 1) Core formatter 2) Section writers..."
#5: "Reduce nesting from 5 to 2 levels"
#7: "Extract 6 state transitions into named functions"
```

**Problems**:
- **Mixed abstraction levels** - Some high-level ("split by phase"), some tactical ("reduce nesting")
- **Inconsistent verbs** - "Split", "Reduce", "Extract" used interchangeably
- **Unclear effort** - Can't tell if actions are similar in scope
- **No categorization** - Can't filter by action type

This makes it hard to:
- Group similar refactoring types
- Estimate relative effort
- Build mental model of action types
- Filter recommendations by action category

## Objective

Standardize action language by:

1. **Defining action taxonomy** - Consistent verbs for action types
2. **Using predictable structure** - Same verb for similar operations
3. **Enabling filtering** - Actions categorized for `--type` filtering
4. **Clarifying effort** - Action types imply effort level

**Success Metric**: Users can predict action type from verb and filter by action category.

## Requirements

### Functional Requirements

1. **Standard Action Verbs**
   - **REFACTOR** - Reorganize existing code without changing behavior
   - **TEST** - Add test coverage for untested code
   - **SIMPLIFY** - Reduce complexity (nesting, branching)
   - **SPLIT** - Break apart god objects or large modules
   - **DOCUMENT** - Add documentation or comments
   - **EXTRACT** - Pull out cohesive logic into separate functions

2. **Action Format**
   ```
   ACTION: [VERB] <specific instruction>
   ```

3. **Verb-Specific Templates**
   - **REFACTOR**: "Refactor <entity> to <pattern>"
   - **TEST**: "Add <N> tests for <coverage_target>"
   - **SIMPLIFY**: "Reduce <metric> from <current> to <target>"
   - **SPLIT**: "Split into <N> modules by <criterion>"
   - **EXTRACT**: "Extract <N> <entities> into <destination>"

4. **Consistent Descriptions**
   - Same verb for same operation type
   - Specific numbers included (N tests, N modules)
   - Target state mentioned when relevant

5. **CLI Filtering** (Future Enhancement)
   ```bash
   debtmap analyze . --action-type=TEST
   debtmap analyze . --action-type=SPLIT,REFACTOR
   ```

### Non-Functional Requirements

1. **Consistency** - Same verb for similar actions
2. **Clarity** - Action type obvious from verb
3. **Specificity** - Concrete instructions, not vague suggestions
4. **Actionability** - Developer knows what to do next

## Acceptance Criteria

- [ ] All recommendations use standard action verbs
- [ ] REFACTOR verb used for code reorganization
- [ ] TEST verb used for adding test coverage
- [ ] SIMPLIFY verb used for complexity reduction
- [ ] SPLIT verb used for breaking apart modules
- [ ] EXTRACT verb used for pulling out functions
- [ ] Action format: "[VERB] specific instruction"
- [ ] Numbers included where relevant (N tests, N modules)
- [ ] Target state mentioned for SIMPLIFY actions
- [ ] God object recommendations use SPLIT
- [ ] Complex function recommendations use SIMPLIFY or EXTRACT
- [ ] Coverage gap recommendations use TEST
- [ ] Action taxonomy documented
- [ ] User testing confirms clarity

## Technical Details

### Implementation

```rust
// src/priority/actions.rs (new module)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionVerb {
    Refactor,
    Test,
    Simplify,
    Split,
    Document,
    Extract,
}

impl ActionVerb {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionVerb::Refactor => "REFACTOR",
            ActionVerb::Test => "TEST",
            ActionVerb::Simplify => "SIMPLIFY",
            ActionVerb::Split => "SPLIT",
            ActionVerb::Document => "DOCUMENT",
            ActionVerb::Extract => "EXTRACT",
        }
    }

    pub fn typical_effort(&self) -> EffortEstimate {
        match self {
            ActionVerb::Test => EffortEstimate::Small,
            ActionVerb::Document => EffortEstimate::Small,
            ActionVerb::Extract => EffortEstimate::Small,
            ActionVerb::Simplify => EffortEstimate::Medium,
            ActionVerb::Refactor => EffortEstimate::Medium,
            ActionVerb::Split => EffortEstimate::Large,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Action {
    pub verb: ActionVerb,
    pub description: String,
}

impl Action {
    pub fn split(description: String) -> Self {
        Self {
            verb: ActionVerb::Split,
            description,
        }
    }

    pub fn test(test_count: usize, target: &str) -> Self {
        Self {
            verb: ActionVerb::Test,
            description: format!("Add {} tests for {}", test_count, target),
        }
    }

    pub fn simplify(metric: &str, current: u32, target: u32) -> Self {
        Self {
            verb: ActionVerb::Simplify,
            description: format!("Reduce {} from {} to {}", metric, current, target),
        }
    }

    pub fn extract(count: usize, entity_type: &str, destination: &str) -> Self {
        Self {
            verb: ActionVerb::Extract,
            description: format!("Extract {} {} into {}", count, entity_type, destination),
        }
    }

    pub fn format(&self) -> String {
        format!("ACTION: [{}] {}", self.verb.as_str(), self.description)
    }
}

// Action builders per debt type

impl GodObjectRecommendation {
    pub fn generate_action(&self) -> Action {
        Action::split(format!(
            "Split into {} modules by {}",
            self.recommended_module_count,
            self.split_criterion
        ))
    }
}

impl ComplexFunctionRecommendation {
    pub fn generate_action(&self) -> Action {
        match self.primary_driver {
            ComplexityDriver::Nesting { depth } => {
                Action::simplify("nesting", depth, 2)
            }
            ComplexityDriver::Branching { count } => {
                Action::extract(
                    count / 3, // Extract ~1/3 of decision logic
                    "decision clusters",
                    "focused functions",
                )
            }
            ComplexityDriver::Mixed { .. } => {
                Action::simplify("complexity", self.cognitive_complexity, self.cognitive_complexity / 2)
            }
            _ => Action::extract(1, "complex logic", "separate function"),
        }
    }
}

impl CoverageGapRecommendation {
    pub fn generate_action(&self) -> Action {
        Action::test(self.tests_needed, "untested branches")
    }
}
```

### Example Output

**Before (Inconsistent)**:
```
#1 ACTION: Split by analysis phase: 1) Data collection 2) Pattern detection...
#3 ACTION: Split into: 1) Core formatter 2) Section writers...
#5 ACTION: Reduce nesting from 5 to 2 levels
#7 ACTION: Extract 6 state transitions into named functions
```

**After (Standardized)**:
```
#1 ACTION: [SPLIT] Split into 4 modules by analysis phase
#3 ACTION: [SPLIT] Split into 3 modules by responsibility
#5 ACTION: [SIMPLIFY] Reduce nesting from 5 to 2 levels
#7 ACTION: [EXTRACT] Extract 6 state transitions into named functions
#4 ACTION: [TEST] Add 9 tests for untested branches
```

## Dependencies

- **Prerequisites**: Specs 194, 197
- **Affected Components**:
  - `src/priority/recommendations.rs`
  - All recommendation generators
- **External Dependencies**: None

## Testing Strategy

```rust
#[test]
fn test_action_verb_consistency() {
    let god_object = create_god_object_recommendation();
    let action = god_object.generate_action();
    assert_eq!(action.verb, ActionVerb::Split);
    assert!(action.format().starts_with("ACTION: [SPLIT]"));
}

#[test]
fn test_action_specificity() {
    let coverage_gap = create_coverage_gap_with_tests(9);
    let action = coverage_gap.generate_action();
    assert_eq!(action.verb, ActionVerb::Test);
    assert!(action.description.contains("9 tests"));
}
```

## Success Metrics

- ✅ All recommendations use standard verbs
- ✅ Same verb for same operation type
- ✅ Actions include specific numbers
- ✅ User testing confirms clarity

## Follow-up Work

1. **CLI filtering** by action type
2. **Action templates** for custom recommendations
3. **Effort correlation** with action verbs

## References

- Spec 194: Scannable Summary Mode
- Spec 197: Consistent Card Structure
- Design Analysis: Debtmap Terminal Output

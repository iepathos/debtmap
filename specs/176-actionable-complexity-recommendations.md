---
number: 176
title: Actionable Complexity Hotspot Recommendations
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-11-16
---

# Specification 176: Actionable Complexity Hotspot Recommendations

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Current complexity hotspot recommendations are **generic and non-actionable**, providing little practical guidance for developers:

**Current recommendation output**:
```
#1 SCORE: 17.7 [ERROR UNTESTED] [CRITICAL]
├─ LOCATION: src/cook/workflow/resume.rs:824 ResumeExecutor::execute_remaining_steps()
├─ IMPACT: -7 complexity, -6.2 risk
├─ COMPLEXITY: cyclomatic=15, cognitive=65, nesting=4, entropy=0.43
├─ WHY THIS MATTERS: High complexity 15/65 makes function hard to test and maintain
├─ RECOMMENDED ACTION: Reduce complexity from 15 to ~10
```

**Problems with current approach**:

1. **No pattern analysis**: Doesn't explain WHY complexity is high
   - cyclomatic=15, cognitive=65 (4.3x ratio!) indicates **high nesting**, not branching
   - entropy=0.43 suggests **moderate structural consistency**
   - nesting=4 is the **primary driver** but not mentioned in recommendation

2. **Vague actions**: "Reduce complexity from 15 to ~10" - HOW?
   - No specific techniques suggested
   - No guidance on what to extract or refactor
   - No intermediate steps or priorities

3. **Missing concrete steps**: Compare to testing gap recommendations which are excellent:
   ```
   ├─ RECOMMENDED ACTION: Add 9 tests for untested branches
   - COVERAGE: [ERROR] UNTESTED - Missing lines: 568-572 [CRITICAL]
   ```
   Testing recommendations are **specific, quantified, and actionable**. Complexity recommendations should be too.

4. **No impact prioritization**: All steps treated equally
   - For cyclomatic=15, cognitive=65, reducing nesting has 10x more impact than extracting functions
   - Current recommendations don't communicate this

### Successful Pattern: Testing Gap Recommendations

Testing recommendations work well because they:
- **Quantify the problem**: "9 tests needed"
- **Locate the issue**: "Missing lines: 568-572"
- **Explain the impact**: "+50% function coverage"
- **Provide clear action**: "Add 9 tests for untested branches"

Complexity recommendations should follow the same formula.

## Objective

Transform complexity hotspot recommendations from **generic advice** into **specific, prioritized, actionable guidance** by:

1. **Analyzing complexity patterns** to identify root causes
2. **Recommending specific refactoring techniques** based on patterns
3. **Quantifying expected impact** for each recommended step
4. **Prioritizing steps** by impact (biggest wins first)
5. **Providing concrete examples** of what to look for

## Requirements

### Functional Requirements

**FR-1: Pattern Detection and Classification**

Analyze complexity metrics to classify the primary complexity driver:

| Pattern | Detection Criteria | Primary Driver |
|---------|-------------------|----------------|
| **High Nesting** | `cognitive/cyclomatic > 3.0` AND `nesting >= 4` | Deep nesting depth |
| **High Branching** | `cyclomatic >= 15` AND `cognitive/cyclomatic < 2.5` | Many decision points |
| **Mixed Complexity** | `cyclomatic >= 12` AND `cognitive >= 40` AND `2.5 <= ratio <= 3.5` | Both nesting and branching |
| **Chaotic Structure** | `entropy >= 0.45` | Inconsistent patterns |
| **Moderate Complexity** | `10 < cyclomatic < 15` | Approaching threshold |

**FR-2: Pattern-Specific Recommendations**

Generate tailored recommendations based on detected pattern:

**High Nesting Pattern** (cognitive >> cyclomatic):
```
RECOMMENDED ACTION: Reduce nesting from {current} to 2 levels (primary impact: -{est} cognitive)

COMPLEXITY ANALYSIS:
- Primary driver: Deep nesting (level {nesting})
- Cognitive/Cyclomatic ratio: {ratio}x (indicates nesting problem)
- Estimated reduction: -{cognitive_reduction} cognitive complexity

SPECIFIC STEPS (in priority order):
1. Apply early returns for error conditions
   - Move validation checks to function start
   - Return early on invalid states
   - Pattern to find: nested if/match statements
   - Impact: -{est1} cognitive complexity, clearer control flow

2. Extract nested conditionals into predicate functions
   - Look for: nested if within if/match
   - Create well-named boolean functions (is_valid, should_process, etc.)
   - Impact: -{est2} cognitive complexity, improved readability

3. Consider guard clauses for preconditions
   - Replace: if (condition) { main_logic } with guard + main_logic
   - Impact: Flattened structure, -1 nesting level

VERIFICATION:
- Target: nesting < 3, cognitive < 25
- Run: cargo clippy -- -W clippy::cognitive_complexity
```

**High Branching Pattern** (cyclomatic high, moderate cognitive):
```
RECOMMENDED ACTION: Split into {n} focused functions by decision clusters

COMPLEXITY ANALYSIS:
- Primary driver: Many decision points ({cyclomatic} branches)
- Cognitive complexity: {cognitive} (moderate relative to cyclomatic)
- Estimated reduction: -{cyclo_reduction} cyclomatic after extraction

SPECIFIC STEPS (in priority order):
1. Identify decision clusters (related conditional logic)
   - Group related if/match statements handling same concern
   - Each cluster becomes a focused function
   - Pattern to find: Sequential if/else or match on related conditions
   - Impact: -{est1} complexity per extraction

2. Extract setup/validation logic to separate function
   - Move parameter validation and setup to dedicated function
   - Returns Result<PreparedState, Error>
   - Impact: -{est2} complexity, clearer entry point

3. Consider lookup tables or match expressions
   - Replace long if-else chains with match or HashMap
   - Especially for: value mapping, dispatch logic
   - Impact: Easier to extend, test each path independently

VERIFICATION:
- Target: cyclomatic < 10 per function
- Each extracted function should have cyclomatic < 8
```

**Mixed Complexity Pattern** (both high):
```
RECOMMENDED ACTION: Reduce nesting FIRST, then extract functions (two-phase approach)

COMPLEXITY ANALYSIS:
- Primary drivers: Both nesting ({nesting} levels) AND branching ({cyclomatic} branches)
- Mixed complexity requires phased refactoring
- Phase 1 impact: -{nesting_reduction} cognitive
- Phase 2 impact: -{branch_reduction} cyclomatic

PHASE 1: Reduce Nesting (weeks impact)
1. Apply early returns and guard clauses
   - Impact: -{est1} cognitive complexity
   - Makes branching structure clearer

2. Flatten nested conditionals
   - Impact: -{est2} cognitive complexity

PHASE 2: Extract Functions (after nesting reduced)
3. Identify decision clusters from flattened structure
   - Impact: -{est3} cyclomatic complexity

4. Extract validation and setup logic
   - Impact: -{est4} cyclomatic complexity

VERIFICATION:
- After Phase 1: nesting < 3
- After Phase 2: cyclomatic < 10, cognitive < 25
```

**Chaotic Structure Pattern** (high entropy):
```
RECOMMENDED ACTION: Standardize control flow patterns before refactoring

COMPLEXITY ANALYSIS:
- Primary driver: Inconsistent structure (entropy={entropy})
- High entropy indicates mixed patterns, making refactoring risky
- Standardization enables safe refactoring

SPECIFIC STEPS (in priority order):
1. Standardize error handling patterns
   - Some paths use Result<?>, others unwrap() or expect()
   - Convert all error handling to consistent Result propagation
   - Impact: More predictable control flow, safer refactoring

2. Group related state transitions
   - Scattered state changes create unpredictability
   - Collect related state updates into cohesive blocks
   - Impact: Clear state evolution, fewer bugs

3. Extract inconsistent code sections for review
   - Identify outlier patterns that don't match codebase norms
   - Rewrite using standard project patterns
   - Impact: Reduced cognitive load, consistent codebase

VERIFICATION:
- Re-run entropy calculation after standardization
- Target: entropy < 0.35, then proceed with complexity reduction
```

**FR-3: Impact Quantification**

For each recommended step, calculate and display:

1. **Complexity reduction estimate**:
   - Early returns: `-20 to -30 cognitive complexity` (based on nesting depth)
   - Extract function: `-5 to -8 cyclomatic` (based on cluster size)
   - Guard clauses: `-1 nesting level, -10 to -15 cognitive`
   - Lookup tables: `-N cyclomatic` (where N = branch count in if-else chain)

2. **Impact calculation formulas**:
   ```rust
   // Early returns impact (per nesting level reduced)
   fn calculate_early_return_impact(current_nesting: u32) -> u32 {
       (current_nesting - 2) * 10 // Each level of nesting ≈ 10 cognitive points
   }

   // Function extraction impact
   fn calculate_extraction_impact(cluster_size: u32) -> u32 {
       cluster_size.min(8) // Max benefit around 8 complexity per extraction
   }

   // Guard clause impact
   fn calculate_guard_impact(nesting: u32) -> (u32, u32) {
       let nesting_reduction = 1;
       let cognitive_reduction = 10 + (nesting * 3); // Base + per-level
       (nesting_reduction, cognitive_reduction)
   }
   ```

3. **Confidence indicators**:
   - "Estimated" for heuristic-based calculations
   - "Expected" for well-understood patterns
   - "Up to" for upper bounds

**FR-4: Language-Specific Recommendations**

Provide language-appropriate refactoring techniques:

**Rust-specific**:
- "Use `?` operator instead of match for error propagation"
- "Replace nested matches with guard patterns: `if let ... else { return }`"
- "Consider `Option::map` chains to reduce nesting"
- "Use early `?` returns for Result unwrapping"

**Python-specific**:
- "Use context managers (with) to reduce try-except nesting"
- "Replace nested if with early returns and guard clauses"
- "Consider dict lookup tables for dispatch logic"
- "Use list/dict comprehensions instead of nested loops"

**JavaScript/TypeScript-specific**:
- "Use optional chaining (?.) to reduce null checks"
- "Replace nested if with early returns"
- "Use async/await instead of nested promises"
- "Consider strategy pattern for conditional logic"

**FR-5: Code Location Hints**

Similar to how testing recommendations show "Missing lines: 568-572", provide specific guidance:

```
HIGH NESTING DETECTED:
- Lines {start}-{end}: Nested if depth {N}
- Look for: if statements inside if/match blocks
- Priority: Extract deepest nesting first

DECISION CLUSTERS DETECTED:
- Lines {start1}-{end1}: {N} related conditions on {variable}
- Lines {start2}-{end2}: {M} related conditions on {variable2}
- Extraction candidates: validation_logic, error_handling, state_machine
```

### Non-Functional Requirements

**NFR-1: Consistency with Testing Recommendations**

Follow the same structure and quality bar as testing gap recommendations:
- Specific, quantified actions
- Clear impact statements
- Located guidance (line numbers where possible)
- Measurable verification criteria

**NFR-2: Readability and Scannability**

Recommendations must be:
- Skimmable (clear headers, bullets)
- Actionable (concrete next steps)
- Prioritized (most impactful first)
- Verifiable (clear success criteria)

**NFR-3: Performance**

Pattern detection must not significantly impact analysis time:
- Target: < 5% overhead on total analysis time
- Pattern classification: O(1) per function (simple metric comparisons)
- Impact calculations: Pre-computed formulas

## Acceptance Criteria

**AC-1: Pattern Detection Accuracy**
- [ ] Correctly identifies "High Nesting" when cognitive/cyclomatic > 3.0 AND nesting >= 4
- [ ] Correctly identifies "High Branching" when cyclomatic >= 15 AND ratio < 2.5
- [ ] Correctly identifies "Mixed Complexity" for combined conditions
- [ ] Correctly identifies "Chaotic Structure" when entropy >= 0.45
- [ ] Unit tests cover all pattern detection scenarios

**AC-2: Actionable Recommendations**
- [ ] Every complexity recommendation includes specific refactoring technique
- [ ] Every step includes quantified impact estimate ("-X complexity")
- [ ] Steps are prioritized by expected impact (highest first)
- [ ] Recommendations include language-specific guidance where applicable
- [ ] Sample output passes user review for actionability

**AC-3: Impact Quantification**
- [ ] Early return impact calculation based on nesting depth
- [ ] Function extraction impact based on cluster size
- [ ] All impacts include confidence indicators ("Estimated", "Expected", "Up to")
- [ ] Impact calculations tested against known refactoring examples
- [ ] Estimated vs actual impact deviation < 30% in validation set

**AC-4: Verification Criteria**
- [ ] Every recommendation includes measurable success criteria
- [ ] Verification commands provided (e.g., clippy invocations)
- [ ] Target metrics specified (e.g., "nesting < 3, cyclomatic < 10")
- [ ] Validation steps documented in recommendation

**AC-5: Output Quality**
- [ ] Complexity recommendations match testing recommendations for specificity
- [ ] User testing confirms recommendations are more actionable than current
- [ ] Average recommendation length: 8-15 lines (concise but complete)
- [ ] No generic advice like "reduce complexity" without specific steps
- [ ] Integration test validates full recommendation generation pipeline

**AC-6: Consistency**
- [ ] Recommendation format matches existing debtmap output structure
- [ ] Section ordering consistent: Pattern → Steps → Verification
- [ ] Impact format consistent: "-X complexity" or "+Y benefit"
- [ ] Language consistent across all pattern types

## Technical Details

### Implementation Approach

**Phase 1: Pattern Detection Module**

Create `src/priority/complexity_patterns.rs`:

```rust
pub enum ComplexityPattern {
    HighNesting {
        nesting_depth: u32,
        cognitive_score: u32,
        ratio: f64, // cognitive/cyclomatic
    },
    HighBranching {
        branch_count: u32,
        cyclomatic: u32,
    },
    MixedComplexity {
        nesting_depth: u32,
        cyclomatic: u32,
        cognitive: u32,
    },
    ChaoticStructure {
        entropy: f64,
        cyclomatic: u32,
    },
    ModerateComplexity {
        cyclomatic: u32,
        cognitive: u32,
    },
}

impl ComplexityPattern {
    /// Classify complexity pattern from metrics
    pub fn detect(metrics: &ComplexityMetrics) -> Self {
        let ratio = metrics.cognitive as f64 / metrics.cyclomatic.max(1) as f64;

        // High nesting: cognitive dominates
        if ratio > 3.0 && metrics.nesting >= 4 {
            return ComplexityPattern::HighNesting {
                nesting_depth: metrics.nesting,
                cognitive_score: metrics.cognitive,
                ratio,
            };
        }

        // Chaotic: high entropy
        if let Some(entropy) = metrics.entropy_score {
            if entropy >= 0.45 {
                return ComplexityPattern::ChaoticStructure {
                    entropy,
                    cyclomatic: metrics.cyclomatic,
                };
            }
        }

        // High branching: cyclomatic high, ratio moderate
        if metrics.cyclomatic >= 15 && ratio < 2.5 {
            return ComplexityPattern::HighBranching {
                branch_count: metrics.cyclomatic,
                cyclomatic: metrics.cyclomatic,
            };
        }

        // Mixed: both high
        if metrics.cyclomatic >= 12 && metrics.cognitive >= 40
           && ratio >= 2.5 && ratio <= 3.5 {
            return ComplexityPattern::MixedComplexity {
                nesting_depth: metrics.nesting,
                cyclomatic: metrics.cyclomatic,
                cognitive: metrics.cognitive,
            };
        }

        // Default: moderate
        ComplexityPattern::ModerateComplexity {
            cyclomatic: metrics.cyclomatic,
            cognitive: metrics.cognitive,
        }
    }
}
```

**Phase 2: Impact Calculator**

Create `src/priority/refactoring_impact.rs`:

```rust
pub struct RefactoringImpact {
    pub complexity_reduction: u32,
    pub risk_reduction: f64,
    pub confidence: ImpactConfidence,
    pub technique: RefactoringTechnique,
}

pub enum ImpactConfidence {
    Estimated,  // Heuristic-based
    Expected,   // Well-understood pattern
    UpTo,       // Upper bound
}

pub enum RefactoringTechnique {
    EarlyReturns,
    GuardClauses,
    ExtractFunction,
    LookupTable,
    StatePattern,
}

impl RefactoringImpact {
    /// Calculate impact of early returns
    pub fn early_returns(current_nesting: u32) -> Self {
        let reduction = (current_nesting.saturating_sub(2)) * 10;
        Self {
            complexity_reduction: reduction,
            risk_reduction: reduction as f64 * 0.05,
            confidence: ImpactConfidence::Expected,
            technique: RefactoringTechnique::EarlyReturns,
        }
    }

    /// Calculate impact of function extraction
    pub fn extract_function(cluster_size: u32) -> Self {
        let reduction = cluster_size.min(8);
        Self {
            complexity_reduction: reduction,
            risk_reduction: reduction as f64 * 0.04,
            confidence: ImpactConfidence::Estimated,
            technique: RefactoringTechnique::ExtractFunction,
        }
    }

    /// Calculate impact of guard clauses
    pub fn guard_clauses(nesting: u32) -> Self {
        let reduction = 10 + (nesting * 3);
        Self {
            complexity_reduction: reduction,
            risk_reduction: reduction as f64 * 0.04,
            confidence: ImpactConfidence::Expected,
            technique: RefactoringTechnique::GuardClauses,
        }
    }
}
```

**Phase 3: Recommendation Generator**

Extend `src/priority/scoring/concise_recommendation.rs`:

```rust
fn generate_complexity_steps(
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    // Detect pattern
    let pattern = ComplexityPattern::detect(&ComplexityMetrics {
        cyclomatic,
        cognitive,
        nesting: metrics.nesting,
        entropy_score: metrics.entropy_score,
    });

    // Generate pattern-specific recommendation
    match pattern {
        ComplexityPattern::HighNesting { nesting_depth, cognitive_score, ratio } => {
            generate_nesting_recommendation(nesting_depth, cognitive_score, ratio, metrics)
        }
        ComplexityPattern::HighBranching { branch_count, .. } => {
            generate_branching_recommendation(branch_count, metrics)
        }
        ComplexityPattern::MixedComplexity { .. } => {
            generate_mixed_recommendation(cyclomatic, cognitive, metrics)
        }
        ComplexityPattern::ChaoticStructure { entropy, .. } => {
            generate_chaotic_recommendation(entropy, metrics)
        }
        ComplexityPattern::ModerateComplexity { .. } => {
            generate_moderate_recommendation(cyclomatic, cognitive, metrics)
        }
    }
}

fn generate_nesting_recommendation(
    nesting: u32,
    cognitive: u32,
    ratio: f64,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    // Calculate impacts
    let early_return_impact = RefactoringImpact::early_returns(nesting);
    let guard_impact = RefactoringImpact::guard_clauses(nesting);

    let steps = vec![
        ActionStep {
            description: "Apply early returns for error conditions".to_string(),
            impact: format!(
                "-{} cognitive complexity ({})",
                early_return_impact.complexity_reduction,
                early_return_impact.confidence
            ),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Look for nested if statements".to_string(),
                "# Move validation to function start with early returns".to_string(),
            ],
        },
        ActionStep {
            description: "Extract nested conditionals into predicate functions".to_string(),
            impact: format!("-15 to -20 cognitive (estimated)"),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Find: nested if within if/match".to_string(),
                "# Create: is_valid(), should_process() functions".to_string(),
            ],
        },
        ActionStep {
            description: format!("Verify nesting reduced to < 3 levels"),
            impact: format!("Target: nesting < 3, cognitive < 25"),
            difficulty: Difficulty::Easy,
            commands: vec!["cargo clippy -- -W clippy::cognitive_complexity".to_string()],
        },
    ];

    ActionableRecommendation {
        primary_action: format!(
            "Reduce nesting from {} to 2 levels (primary impact: -{})",
            nesting,
            early_return_impact.complexity_reduction + 15
        ),
        rationale: format!(
            "High nesting (depth {}) drives cognitive complexity to {}. \
             Cognitive/Cyclomatic ratio of {:.1}x confirms nesting is primary issue.",
            nesting, cognitive, ratio
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some((nesting as f32 - 2.0) * 0.5),
    }
}
```

**Phase 4: Language-Specific Extensions**

Create `src/priority/language_specific_recommendations.rs`:

```rust
pub fn add_language_specific_hints(
    recommendation: &mut ActionableRecommendation,
    language: Language,
    pattern: &ComplexityPattern,
) {
    match (language, pattern) {
        (Language::Rust, ComplexityPattern::HighNesting { .. }) => {
            recommendation.add_hint(
                "Use `?` operator for Result propagation instead of nested match"
            );
            recommendation.add_hint(
                "Consider `if let ... else { return }` guard patterns"
            );
        }
        (Language::Python, ComplexityPattern::HighBranching { .. }) => {
            recommendation.add_hint(
                "Use dict lookup tables for dispatch logic"
            );
            recommendation.add_hint(
                "Consider match/case (Python 3.10+) for pattern matching"
            );
        }
        (Language::JavaScript | Language::TypeScript, ComplexityPattern::HighNesting { .. }) => {
            recommendation.add_hint(
                "Use optional chaining (?.) to reduce null checks"
            );
            recommendation.add_hint(
                "Use async/await instead of nested promises"
            );
        }
        _ => {}
    }
}
```

### Architecture Changes

**New Modules**:
- `src/priority/complexity_patterns.rs` - Pattern detection
- `src/priority/refactoring_impact.rs` - Impact calculation
- `src/priority/language_specific_recommendations.rs` - Language hints

**Modified Modules**:
- `src/priority/scoring/concise_recommendation.rs` - Use new pattern-based generation
- `src/priority/formatter/sections.rs` - Format new recommendation structure

**Data Flow**:
```
FunctionMetrics
  → ComplexityPattern::detect()
  → generate_pattern_recommendation()
  → add_language_specific_hints()
  → ActionableRecommendation
  → format_for_display()
```

### Data Structures

```rust
pub struct ComplexityMetrics {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub entropy_score: Option<f64>,
}

pub struct RecommendationStep {
    pub description: String,
    pub technique: RefactoringTechnique,
    pub impact: RefactoringImpact,
    pub code_hints: Vec<String>,
    pub priority: u8, // 1-5, lower is higher priority
}

pub struct ComplexityRecommendation {
    pub pattern: ComplexityPattern,
    pub primary_action: String,
    pub analysis: String, // WHY THIS MATTERS
    pub steps: Vec<RecommendationStep>,
    pub verification: VerificationCriteria,
    pub language_hints: Vec<String>,
}

pub struct VerificationCriteria {
    pub target_cyclomatic: Option<u32>,
    pub target_cognitive: Option<u32>,
    pub target_nesting: Option<u32>,
    pub commands: Vec<String>,
}
```

## Dependencies

**Internal**:
- Existing complexity metrics (cyclomatic, cognitive, nesting, entropy)
- Current recommendation infrastructure (`ActionableRecommendation`, `ActionStep`)
- Language detection from file extension

**External**:
- None (uses existing dependencies)

## Testing Strategy

### Unit Tests

**Pattern Detection** (`tests/complexity_patterns_test.rs`):
```rust
#[test]
fn detect_high_nesting_pattern() {
    let metrics = ComplexityMetrics {
        cyclomatic: 12,
        cognitive: 50,  // 4.2x ratio
        nesting: 5,
        entropy_score: Some(0.35),
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(matches!(pattern, ComplexityPattern::HighNesting { .. }));
}

#[test]
fn detect_high_branching_pattern() {
    let metrics = ComplexityMetrics {
        cyclomatic: 18,
        cognitive: 35,  // 1.9x ratio
        nesting: 2,
        entropy_score: Some(0.30),
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(matches!(pattern, ComplexityPattern::HighBranching { .. }));
}
```

**Impact Calculation** (`tests/refactoring_impact_test.rs`):
```rust
#[test]
fn early_return_impact_scales_with_nesting() {
    let impact_2 = RefactoringImpact::early_returns(2);
    let impact_5 = RefactoringImpact::early_returns(5);

    assert_eq!(impact_2.complexity_reduction, 0);  // (2-2)*10 = 0
    assert_eq!(impact_5.complexity_reduction, 30); // (5-2)*10 = 30
}

#[test]
fn function_extraction_capped_at_8() {
    let impact = RefactoringImpact::extract_function(20);
    assert_eq!(impact.complexity_reduction, 8); // Capped
}
```

### Integration Tests

**Full Recommendation Generation** (`tests/complexity_recommendation_integration_test.rs`):
```rust
#[test]
fn high_nesting_generates_actionable_steps() {
    let func = create_test_function(12, 50, 5); // cyclo, cog, nest

    let rec = generate_complexity_steps(12, 50, &func);

    assert!(rec.primary_action.contains("nesting"));
    assert!(rec.steps.is_some());

    let steps = rec.steps.unwrap();
    assert!(steps[0].description.contains("early return"));
    assert!(steps[0].impact.contains("-")); // Quantified reduction
}
```

### Validation Tests

**Real-World Functions** (`tests/real_world_validation_test.rs`):
```rust
#[test]
fn validate_against_known_refactorings() {
    // Test against functions we've actually refactored
    // with known before/after complexity scores

    let cases = vec![
        ("execute_remaining_steps", 15, 65, 4, vec![
            "early returns reduced cognitive by 28",
            "extraction reduced cyclomatic by 6",
        ]),
    ];

    for (name, cyclo, cog, nest, expected_techniques) in cases {
        let rec = generate_for_function(name, cyclo, cog, nest);
        for technique in expected_techniques {
            assert!(
                rec.contains_technique(technique),
                "Missing technique: {}", technique
            );
        }
    }
}
```

### User Acceptance Testing

**Actionability Survey**:
- Present 10 developers with current vs. new recommendations
- Measure: "Which is more actionable?" (5-point scale)
- Target: New recommendations score 4+ average
- Success: 80% prefer new recommendations

## Documentation Requirements

### Code Documentation

**Module docs** (`src/priority/complexity_patterns.rs`):
```rust
//! # Complexity Pattern Detection
//!
//! Classifies complexity hotspots by their primary driver:
//! - High Nesting: Cognitive >> Cyclomatic (deep conditionals)
//! - High Branching: Many decision points, moderate depth
//! - Mixed Complexity: Both nesting and branching high
//! - Chaotic Structure: High entropy, inconsistent patterns
//!
//! Each pattern gets tailored refactoring recommendations.
```

**Function docs with examples**:
```rust
/// Detect complexity pattern from metrics.
///
/// # Examples
///
/// ```
/// let metrics = ComplexityMetrics { cyclomatic: 12, cognitive: 50, nesting: 5, ... };
/// let pattern = ComplexityPattern::detect(&metrics);
/// assert!(matches!(pattern, ComplexityPattern::HighNesting { .. }));
/// ```
pub fn detect(metrics: &ComplexityMetrics) -> ComplexityPattern
```

### User Documentation

**Update `book/src/recommendations.md`**:

Add section "Understanding Complexity Recommendations":
- Explain pattern types and what they mean
- Show before/after examples for each pattern
- Provide decision tree for choosing refactoring approach
- Link to refactoring guides (external resources)

**Add examples to `docs/output-format-guide.md`**:
- Show sample output for each pattern type
- Explain how to interpret impact estimates
- Provide verification command examples

### Architecture Documentation

**Update `ARCHITECTURE.md`**:

Add section "Recommendation System Architecture":
```markdown
## Recommendation Generation

### Complexity Recommendations

Complexity hotspots use pattern-based recommendations:

1. **Pattern Detection** (`complexity_patterns.rs`)
   - Classifies root cause: nesting, branching, mixed, or chaotic
   - Uses ratio analysis: cognitive/cyclomatic ratio indicates nesting
   - Entropy analysis for structural consistency

2. **Impact Calculation** (`refactoring_impact.rs`)
   - Estimates complexity reduction per technique
   - Formulas based on empirical data and research
   - Confidence levels: Estimated, Expected, Up To

3. **Recommendation Generation** (`concise_recommendation.rs`)
   - Pattern-specific steps prioritized by impact
   - Language-specific hints where applicable
   - Verification criteria for measuring success
```

## Implementation Notes

### Pattern Detection Edge Cases

**Boundary conditions**:
- `cognitive/cyclomatic` ratio can be undefined if `cyclomatic = 0` (rare but possible for generated code)
  - Solution: Use `max(1)` when calculating ratio
- Very low complexity (cyclomatic < 5) should not trigger complex recommendations
  - Solution: Add minimum threshold before pattern detection

**Pattern overlap**:
- A function could match multiple patterns (e.g., high nesting AND high entropy)
- Solution: Priority order - Chaotic > HighNesting > MixedComplexity > HighBranching

### Impact Estimation Calibration

**Validation approach**:
1. Analyze 50+ real refactorings from project history
2. Calculate actual before/after complexity deltas
3. Adjust formulas to match observed reductions ±20%
4. Document variance in confidence levels

**Example calibration**:
```rust
// Based on analysis of 32 early-return refactorings:
// - Average reduction: 23.5 cognitive complexity
// - Std dev: 8.2
// - Formula: (nesting - 2) * 10 yields ±25% accuracy
```

### Language-Specific Extensibility

**Adding new language**:
1. Extend `Language` enum
2. Add match arm in `add_language_specific_hints()`
3. Define language idioms for each pattern
4. Add tests for language-specific generation

**Example (Go)**:
```rust
(Language::Go, ComplexityPattern::HighNesting { .. }) => {
    recommendation.add_hint(
        "Use early returns with error checks: if err != nil { return err }"
    );
    recommendation.add_hint(
        "Consider table-driven tests to simplify test complexity"
    );
}
```

### Performance Optimization

**Pattern detection**: O(1) - just metric comparisons
**Impact calculation**: O(1) - formula evaluation
**Recommendation generation**: O(n) where n = number of steps (max 5)

Total overhead: Negligible (<0.1ms per function)

## Migration and Compatibility

### Backward Compatibility

**Existing recommendations still work**:
- New system is additive, doesn't break existing output
- Old format still available via flag: `--legacy-recommendations`
- Transition period: Support both formats for 1 release

### Migration Path

**Phase 1: Opt-in** (v0.3.6):
- New recommendations available via `--actionable-complexity`
- Default unchanged
- Gather user feedback

**Phase 2: Default** (v0.4.0):
- New recommendations become default
- Legacy available via `--legacy-recommendations`
- Update documentation

**Phase 3: Legacy deprecation** (v0.5.0):
- Remove legacy format
- All complexity recommendations use new system

### Configuration

Add to `debtmap.toml`:
```toml
[recommendations]
# Use pattern-based complexity recommendations
actionable_complexity = true

# Show impact estimates in recommendations
show_impact_estimates = true

# Include language-specific hints
language_specific_hints = true
```

## Success Metrics

**Quantitative**:
- [ ] 80%+ of surveyed users prefer new recommendations
- [ ] Average recommendation rating: 4+/5 for actionability
- [ ] Impact estimate accuracy: ±30% of actual reductions
- [ ] Pattern detection accuracy: 90%+ on validation set

**Qualitative**:
- [ ] Developers report acting on recommendations (vs. ignoring)
- [ ] Recommendations cited in code review discussions
- [ ] External users highlight recommendations in reviews/testimonials

## Future Enhancements

**Spec 177: AST-Based Code Hints** (future)
- Analyze actual code structure to provide specific line numbers
- "High nesting at lines 100-150" instead of generic guidance
- Requires AST preservation during analysis

**Spec 178: Refactoring Success Tracking** (future)
- Track which recommendations were acted upon
- Measure actual vs. estimated impact
- Use data to improve future recommendations

**Spec 179: IDE Integration** (future)
- VSCode extension showing recommendations inline
- Quick-fix actions for common refactorings
- Real-time complexity feedback during editing

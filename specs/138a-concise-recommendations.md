---
number: 138a
title: Concise Actionable Recommendations
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-29
replaces: 138 (split into 138a/b/c)
---

# Specification 138a: Concise Actionable Recommendations

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None
**Supersedes**: Spec 138 (split into three focused specs)

## Context

Current refactoring recommendations suffer from information overload and lack clear prioritization:

**Current Issues**:
1. **Too Many Steps**: Some recommendations have 13+ detailed steps
2. **No Impact Estimates**: Users can't assess effort vs. benefit
3. **Missing Prioritization**: All steps appear equally important
4. **No Commands**: Users must figure out how to execute recommendations

**Example (Current Output)**:
```
ACTION: Add 26 tests for 62% coverage gap, then refactor complexity 42 into 15 functions
  - 1. Add tests for uncovered lines: 122, 146-147, 149, 156, 160-161 and 7 more ranges
  - 2. Currently ~26 of 42 branches are uncovered (38% coverage)
  - 3. Write 21 tests to cover critical uncovered branches first
  - 4. Extract 15 pure functions from 42 branches:
  - 5.   • Group ~2 related branches per function
  - 6.   • Target complexity ≤3 per extracted function
  ... (continues for 13 steps)
```

**Problems**:
- Overwhelming number of steps
- Unclear what to do first
- No sense of impact per step
- Missing concrete commands to run

## Objective

Refactor recommendation generation to produce **concise, prioritized, actionable guidance** with clear impact estimates and executable commands, using existing metrics only.

## Requirements

### Functional Requirements

1. **Concise Action Plans**
   - **Maximum 5 high-level steps** per recommendation
   - Group related micro-steps into single actions
   - Focus on what to do, not granular how-to details
   - Prioritize steps by impact (highest impact first)

2. **Impact Estimates**
   - Each step shows estimated impact (e.g., "-10 complexity", "+5 tests")
   - Use existing metrics (cyclomatic, cognitive, coverage) for estimates
   - Include estimated effort in hours for entire recommendation

3. **Executable Commands**
   - Provide specific commands for each step (e.g., "cargo test", "cargo clippy")
   - Commands should be copy-pasteable
   - Use project-specific test/build commands when detectable

4. **Clear Prioritization**
   - Steps ordered by impact/effort ratio
   - Indicate difficulty level (Easy/Medium/Hard)
   - Show prerequisites if any step depends on another

### Non-Functional Requirements

1. **Performance**: No performance regression (max +5ms per recommendation)
2. **Backward Compatibility**: Preserve existing JSON output structure
3. **Consistency**: All recommendation types use same format
4. **Maintainability**: Pure functions, no trait hierarchies

## Acceptance Criteria

- [ ] All recommendations have ≤5 high-level steps
- [ ] Every step includes impact estimate (automated test)
- [ ] Every step includes difficulty indicator (Easy/Medium/Hard)
- [ ] Commands provided for each actionable step
- [ ] Steps ordered by impact (highest first)
- [ ] Effort estimate included in recommendation
- [ ] No performance regression (benchmark shows <5ms overhead)
- [ ] JSON output remains backward compatible
- [ ] Integration test validates ripgrep recommendations are concise
- [ ] All existing tests still pass

## Technical Details

### Implementation Approach

#### 1. Data Structures (Minimal Addition)

```rust
/// Concise recommendation with clear action steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionableRecommendation {
    /// One-line primary action
    pub primary_action: String,
    /// Why this matters
    pub rationale: String,
    /// 3-5 high-level steps, ordered by impact
    pub steps: Vec<ActionStep>,
    /// Estimated total effort in hours
    pub estimated_effort_hours: f32,
}

/// Single actionable step with clear impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    /// What to do (concise, <80 chars)
    pub description: String,
    /// Expected impact (e.g., "-10 complexity", "+5 tests")
    pub impact: String,
    /// Difficulty level
    pub difficulty: Difficulty,
    /// Commands to execute this step
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,    // <30 min, straightforward
    Medium,  // 30min-2hr, requires some design
    Hard,    // >2hr, requires significant refactoring
}
```

#### 2. Pure Function Approach

```rust
/// Generate concise recommendation from debt type and metrics
pub fn generate_concise_recommendation(
    debt_type: &DebtType,
    metrics: &FunctionMetrics,
    coverage: &Option<TransitiveCoverage>,
) -> ActionableRecommendation {
    match debt_type {
        DebtType::TestingGap { coverage: cov, cyclomatic, cognitive } => {
            generate_testing_gap_steps(*cov, *cyclomatic, *cognitive, metrics, coverage)
        }
        DebtType::ComplexityHotspot { cyclomatic, cognitive } => {
            generate_complexity_steps(*cyclomatic, *cognitive, metrics)
        }
        DebtType::DeadCode { visibility, .. } => {
            generate_dead_code_steps(visibility, metrics)
        }
        // ... other debt types
    }
}

/// Generate testing gap recommendation with max 5 steps
fn generate_testing_gap_steps(
    coverage_pct: f64,
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
    transitive_cov: &Option<TransitiveCoverage>,
) -> ActionableRecommendation {
    let tests_needed = calculate_tests_needed(cyclomatic, coverage_pct, None).count;
    let coverage_gap = (100.0 - coverage_pct * 100.0) as u32;

    let mut steps = vec![];

    // Step 1: Add tests (highest impact)
    steps.push(ActionStep {
        description: format!("Add {} tests for {coverage_gap}% coverage gap", tests_needed),
        impact: format!("+{} tests, reduce risk", tests_needed),
        difficulty: if tests_needed <= 5 { Difficulty::Easy } else { Difficulty::Medium },
        commands: vec![
            format!("cargo test {}::", metrics.name),
            "# Write focused tests covering critical paths".to_string(),
        ],
    });

    // Step 2: Refactoring (only if complex)
    if cyclomatic > 15 || cognitive > 20 {
        steps.push(ActionStep {
            description: "Extract complex branches into focused functions".to_string(),
            impact: format!("-{} complexity", (cyclomatic - 10).max(5)),
            difficulty: Difficulty::Medium,
            commands: vec!["cargo clippy -- -W clippy::cognitive_complexity".to_string()],
        });
    }

    // Step 3: Verify (always include)
    steps.push(ActionStep {
        description: "Verify tests pass and coverage improved".to_string(),
        impact: format!("Confirmed +{coverage_gap}% coverage"),
        difficulty: Difficulty::Easy,
        commands: vec![
            "cargo test --all".to_string(),
            "# Run coverage tool to verify improvement".to_string(),
        ],
    });

    ActionableRecommendation {
        primary_action: format!("Add {} tests for untested branches", tests_needed),
        rationale: format!(
            "Function has {}% coverage with complexity {}/{}. Needs {} tests minimum.",
            (coverage_pct * 100.0) as u32, cyclomatic, cognitive, tests_needed
        ),
        steps,
        estimated_effort_hours: estimate_effort(cyclomatic, tests_needed),
    }
}

/// Generate complexity hotspot recommendation
fn generate_complexity_steps(
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);
    let target_complexity = 10; // Standard threshold

    let steps = vec![
        ActionStep {
            description: "Add tests before refactoring (if coverage < 80%)".to_string(),
            impact: "+safety net for refactoring".to_string(),
            difficulty: Difficulty::Medium,
            commands: vec![format!("cargo test {}::", metrics.name)],
        },
        ActionStep {
            description: format!("Extract {} focused functions", functions_to_extract),
            impact: format!("-{} complexity", cyclomatic - target_complexity),
            difficulty: Difficulty::Hard,
            commands: vec!["cargo clippy".to_string()],
        },
        ActionStep {
            description: "Verify tests still pass".to_string(),
            impact: "Confirmed refactoring safe".to_string(),
            difficulty: Difficulty::Easy,
            commands: vec!["cargo test --all".to_string()],
        },
    ];

    ActionableRecommendation {
        primary_action: format!("Reduce complexity from {} to ~{}", cyclomatic, target_complexity),
        rationale: format!(
            "High complexity {}/{} makes function hard to test and maintain",
            cyclomatic, cognitive
        ),
        steps,
        estimated_effort_hours: (cyclomatic as f32 / 10.0) * 1.5, // ~1.5hr per 10 complexity
    }
}
```

#### 3. Effort Estimation

```rust
/// Estimate effort in hours based on metrics
fn estimate_effort(cyclomatic: u32, tests_needed: u32) -> f32 {
    // Base: 10-15 min per test
    let test_effort = tests_needed as f32 * 0.2;

    // Refactoring effort (if needed)
    let refactor_effort = if cyclomatic > 15 {
        (cyclomatic as f32 - 10.0) / 10.0 * 1.5 // ~1.5hr per 10 complexity reduction
    } else {
        0.0
    };

    // Round to nearest 0.5 hours
    ((test_effort + refactor_effort) * 2.0).round() / 2.0
}

/// Calculate number of functions to extract based on complexity
fn calculate_functions_to_extract(cyclomatic: u32, cognitive: u32) -> u32 {
    if cyclomatic > 30 || cognitive > 40 {
        4
    } else if cyclomatic > 20 || cognitive > 30 {
        3
    } else if cyclomatic > 15 || cognitive > 20 {
        2
    } else {
        1
    }
}
```

#### 4. Difficulty Assessment

```rust
impl Difficulty {
    /// Determine difficulty based on complexity and test count
    fn for_testing(tests_needed: u32, cyclomatic: u32) -> Self {
        if tests_needed <= 3 && cyclomatic <= 10 {
            Difficulty::Easy
        } else if tests_needed <= 7 || cyclomatic <= 20 {
            Difficulty::Medium
        } else {
            Difficulty::Hard
        }
    }

    /// Determine difficulty for refactoring
    fn for_refactoring(cyclomatic: u32, cognitive: u32) -> Self {
        if cyclomatic <= 15 && cognitive <= 20 {
            Difficulty::Easy
        } else if cyclomatic <= 25 || cognitive <= 35 {
            Difficulty::Medium
        } else {
            Difficulty::Hard
        }
    }
}
```

### Output Format

```
ACTION: Add 5 tests for untested branches
RATIONALE: Function has 38% coverage with complexity 15/22. Needs 5 tests minimum.
EFFORT: 1.5 hours

STEPS:
  1. [Easy] Add 5 tests for 62% coverage gap
     Impact: +5 tests, reduce risk
     Run: cargo test function_name::
          # Write focused tests covering critical paths

  2. [Medium] Extract complex branches into focused functions
     Impact: -5 complexity
     Run: cargo clippy -- -W clippy::cognitive_complexity

  3. [Easy] Verify tests pass and coverage improved
     Impact: Confirmed +62% coverage
     Run: cargo test --all
          # Run coverage tool to verify improvement
```

### Migration from Current System

**Phase 1: Refactor Existing Functions** (Week 1)
- Update `src/priority/scoring/recommendation.rs`
- Replace verbose step generation with concise versions
- Add impact estimates using existing metrics
- Maintain backward compatibility in JSON

**Phase 2: Add Commands** (Week 1)
- Detect project type (Cargo.toml, package.json, etc.)
- Generate appropriate test/build commands
- Add language-specific command variants

**Phase 3: Testing & Validation** (Week 2)
- Unit tests for step count limits
- Integration tests with ripgrep
- Benchmark performance
- Update documentation

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/priority/scoring/recommendation.rs` - Core refactoring
- `src/priority/scoring/recommendation_helpers.rs` - Helper updates
- `src/priority/mod.rs` - Update `ActionableRecommendation` struct
- `src/io/writers/enhanced_markdown/recommendation_writer.rs` - Format updates

**External Dependencies**: None (uses existing metrics)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_max_5_steps_per_recommendation() {
    let metrics = create_test_metrics(20, 25);
    let rec = generate_concise_recommendation(
        &DebtType::ComplexityHotspot { cyclomatic: 20, cognitive: 25 },
        &metrics,
        &None,
    );

    assert!(rec.steps.len() <= 5, "Should have at most 5 steps, got {}", rec.steps.len());
}

#[test]
fn test_all_steps_have_impact() {
    let metrics = create_test_metrics(15, 18);
    let rec = generate_testing_gap_steps(0.5, 15, 18, &metrics, &None);

    for step in &rec.steps {
        assert!(!step.impact.is_empty(), "Step '{}' missing impact", step.description);
        assert!(!step.commands.is_empty(), "Step '{}' missing commands", step.description);
    }
}

#[test]
fn test_steps_ordered_by_impact() {
    let metrics = create_test_metrics(25, 30);
    let rec = generate_testing_gap_steps(0.3, 25, 30, &metrics, &None);

    // First step should be testing (highest impact for testing gap)
    assert!(rec.steps[0].description.contains("test"),
            "First step should address testing: {}", rec.steps[0].description);
}

#[test]
fn test_effort_estimation_reasonable() {
    let metrics = create_test_metrics(15, 20);
    let rec = generate_testing_gap_steps(0.5, 15, 20, &metrics, &None);

    assert!(rec.estimated_effort_hours > 0.0);
    assert!(rec.estimated_effort_hours < 10.0, "Effort seems too high: {}", rec.estimated_effort_hours);
}

#[test]
fn test_difficulty_matches_complexity() {
    // Simple case: Easy difficulty
    let simple = ActionStep {
        description: "Add 2 tests".to_string(),
        impact: "+2 tests".to_string(),
        difficulty: Difficulty::for_testing(2, 5),
        commands: vec![],
    };
    assert!(matches!(simple.difficulty, Difficulty::Easy));

    // Complex case: Hard difficulty
    let hard = ActionStep {
        description: "Add 15 tests".to_string(),
        impact: "+15 tests".to_string(),
        difficulty: Difficulty::for_testing(15, 40),
        commands: vec![],
    };
    assert!(matches!(hard.difficulty, Difficulty::Hard));
}
```

### Integration Tests

```rust
#[test]
fn test_ripgrep_concise_recommendations() {
    // Analyze ripgrep's most complex function
    let results = analyze_file("../ripgrep/crates/core/flags/hiargs.rs")
        .expect("Should analyze ripgrep");

    let complex_item = results.items.iter()
        .find(|item| matches!(item.debt_type, DebtType::ComplexityHotspot { .. }))
        .expect("Should find complexity hotspot");

    let rec = &complex_item.recommendation;

    // Validate conciseness
    assert!(rec.steps.len() <= 5, "Ripgrep recommendation has {} steps (max 5)", rec.steps.len());

    // Validate actionability
    assert!(rec.estimated_effort_hours > 0.0, "Missing effort estimate");
    for step in &rec.steps {
        assert!(!step.impact.is_empty(), "Step missing impact");
        assert!(!step.commands.is_empty(), "Step missing commands");
    }

    // Validate primary action is concise
    assert!(rec.primary_action.len() < 120,
            "Action too verbose: {}", rec.primary_action);
}
```

### Performance Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_generate_recommendation(c: &mut Criterion) {
    let metrics = create_test_metrics(20, 25);
    let debt_type = DebtType::ComplexityHotspot { cyclomatic: 20, cognitive: 25 };

    c.bench_function("generate_concise_recommendation", |b| {
        b.iter(|| {
            generate_concise_recommendation(
                black_box(&debt_type),
                black_box(&metrics),
                black_box(&None),
            )
        })
    });
}

criterion_group!(benches, bench_generate_recommendation);
criterion_main!(benches);
```

## Documentation Requirements

### Code Documentation

- Document effort estimation formulas with rationale
- Explain difficulty classification thresholds
- Provide examples of each recommendation type
- Document command generation logic

### User Documentation

Update README:
- Explain new recommendation format
- Show example output before/after
- Clarify effort estimates are approximations
- Document difficulty levels

## Success Metrics

- **100% of recommendations have ≤5 steps** (automated test)
- **100% of steps include impact estimate** (automated test)
- **100% of steps include difficulty level** (automated test)
- **100% of actionable steps include commands** (automated test)
- **<5ms performance overhead** (benchmark)
- **Zero breaking changes to JSON output** (integration test)
- **All existing tests pass** (CI)

## Migration and Compatibility

### Backward Compatibility

**Preserved**:
- JSON structure for `ActionableRecommendation` unchanged
- All existing fields remain (add new fields as optional)
- Existing formatting functions continue to work

**Added** (optional fields):
- `estimated_effort_hours: Option<f32>`
- Steps now have structured `ActionStep` instead of `String`
  - Old: `implementation_steps: Vec<String>`
  - New: `steps: Vec<ActionStep>` (serializes to include impact/difficulty)

**Migration Path**:
```rust
// Old code continues to work
let old_steps: Vec<String> = rec.implementation_steps.iter()
    .map(|step| step.description.clone())
    .collect();

// New code gets richer information
let impacts: Vec<String> = rec.steps.iter()
    .map(|step| step.impact.clone())
    .collect();
```

## Implementation Notes

### Grouping Micro-Steps

**Current** (too detailed):
```
1. Add tests for uncovered lines: 122, 146-147, 149
2. Currently ~26 of 42 branches uncovered
3. Write 21 tests for critical branches
4. Extract 15 pure functions
5. Group ~2 branches per function
... (8 more steps)
```

**Target** (concise):
```
1. [Easy] Add 5-7 tests for critical uncovered branches
   Impact: +7 tests, reduce risk

2. [Medium] Extract 3-4 focused functions from complex branches
   Impact: -15 complexity

3. [Easy] Verify improvements
   Impact: Confirmed -15 complexity
```

### Command Generation

Detect project type and provide appropriate commands:

```rust
fn detect_project_commands(metrics: &FunctionMetrics) -> ProjectCommands {
    let path = &metrics.file;

    if path.ancestors().any(|p| p.join("Cargo.toml").exists()) {
        ProjectCommands {
            test: "cargo test",
            lint: "cargo clippy",
            build: "cargo build",
        }
    } else if path.ancestors().any(|p| p.join("package.json").exists()) {
        ProjectCommands {
            test: "npm test",
            lint: "npm run lint",
            build: "npm run build",
        }
    } else if path.ancestors().any(|p| p.join("setup.py").exists()) {
        ProjectCommands {
            test: "pytest",
            lint: "pylint",
            build: "python setup.py build",
        }
    } else {
        ProjectCommands::default()
    }
}
```

## Related Specifications

- **Spec 109**: Test Calculation Consistency (must remain consistent)
- **Spec 137**: Call Graph Analysis (future integration for better recommendations)
- **Spec 138b**: Template-Based Code Examples (next phase)
- **Spec 138c**: Pattern Detection Library (optional future enhancement)

## Approval Checklist

- [ ] Reviewed by architect for functional programming compliance
- [ ] Performance benchmarks show <5ms overhead
- [ ] All existing tests pass with changes
- [ ] Backward compatibility verified
- [ ] Documentation updated
- [ ] Integration test with ripgrep passes

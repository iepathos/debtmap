---
number: 179
title: State Machine/Coordinator Pattern Detection
category: foundation
priority: medium
status: draft
dependencies: []
created: 2025-11-16
---

# Specification 179: State Machine/Coordinator Pattern Detection

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently detects general complexity patterns (high nesting, high branching, mixed complexity, chaotic structure, moderate complexity) but lacks specific detection for state machine and coordinator patterns. These patterns are common in production code and have distinct characteristics:

- **State machines**: Functions that match on enum states and transition between them
- **Coordinator patterns**: Functions that orchestrate actions based on state comparisons and complex interdependencies

Example from `state_reconciliation.rs:81` (the `reconcile_state` function):
- Cyclomatic complexity: 9 (moderate)
- Cognitive complexity: 16 (high for the cyclomatic)
- Nesting depth: 4
- Pattern: Nested if/match on enum states with action accumulation
- Current recommendation: Generic "reduce complexity" (not helpful)
- Desired recommendation: "Extract state transition functions" (pattern-specific)

The current complexity pattern detection classifies this as `ModerateComplexity` but misses the opportunity to provide targeted refactoring guidance for state transition logic.

## Objective

Add state machine/coordinator pattern detection to debtmap's complexity analysis pipeline to provide targeted, actionable refactoring recommendations for functions that orchestrate state transitions.

## Requirements

### Functional Requirements

1. **Pattern Detection**
   - Detect state machine patterns: functions with nested conditionals on enum-like states
   - Detect coordinator patterns: functions that accumulate actions based on state comparisons
   - Distinguish from general high-nesting or high-branching patterns
   - Support detection across Rust, Python, JavaScript, and TypeScript

2. **Heuristic Criteria**
   - State enum usage: detect match/if on enum types or state-like variables
   - Action accumulation: detect patterns like `vec![]` + `push()` or similar collection building
   - State comparison: detect comparisons like `current.mode != target.mode`
   - Transition logic: identify conditional blocks that trigger different actions
   - Minimum complexity threshold: cyclomatic >= 6, cognitive >= 12

3. **Pattern-Specific Recommendations**
   - For state machines: "Extract state transition functions" with examples
   - For coordinators: "Extract reconciliation logic into transition map"
   - Include impact estimates: typical complexity reduction of 30-50%
   - Provide language-specific refactoring commands/hints

### Non-Functional Requirements

- Performance: Pattern detection adds < 5% overhead to analysis time
- Accuracy: Precision >= 70% (avoid false positives on generic conditionals)
- Maintainability: Pattern definitions configurable via rules (not hardcoded)
- Extensibility: Easy to add new state machine variants (FSM, actor patterns, etc.)

## Acceptance Criteria

- [ ] `ComplexityPattern` enum extended with `StateMachine` and `Coordinator` variants
- [ ] Pattern detection logic in `ComplexityPattern::detect()` prioritizes state patterns before generic patterns
- [ ] Heuristics detect state machines with >= 70% precision on test corpus
- [ ] Recommendations include "Extract state transition functions" with concrete impact estimates
- [ ] All tests pass, including new property-based tests for pattern detection
- [ ] Documentation updated with state machine pattern examples
- [ ] Integration test validates correct recommendation for `reconcile_state()` example

## Technical Details

### Implementation Approach

#### 1. Extend `ComplexityPattern` Enum

File: `src/priority/complexity_patterns.rs`

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplexityPattern {
    // ... existing variants ...

    /// State machine pattern: nested conditionals on enum states
    StateMachine {
        state_transitions: u32,  // Number of detected state transitions
        cyclomatic: u32,
        cognitive: u32,
        nesting: u32,
    },

    /// Coordinator pattern: orchestrates actions based on state comparisons
    Coordinator {
        action_count: u32,       // Number of actions accumulated
        comparison_count: u32,   // Number of state comparisons
        cyclomatic: u32,
        cognitive: u32,
    },
}
```

#### 2. Detection Heuristics

Add pattern detection logic to `ComplexityPattern::detect()`:

```rust
impl ComplexityPattern {
    pub fn detect(metrics: &ComplexityMetrics) -> Self {
        let ratio = metrics.cognitive as f64 / metrics.cyclomatic.max(1) as f64;

        // Check for state machine/coordinator BEFORE chaotic structure
        // Priority order:
        // 1. State machine (high confidence signals)
        // 2. Coordinator (action accumulation + comparisons)
        // 3. Chaotic structure (high entropy)
        // 4. High nesting / High branching / Mixed
        // 5. Moderate complexity

        if let Some(state_signals) = detect_state_machine_signals(metrics) {
            return ComplexityPattern::StateMachine {
                state_transitions: state_signals.transition_count,
                cyclomatic: metrics.cyclomatic,
                cognitive: metrics.cognitive,
                nesting: metrics.nesting,
            };
        }

        if let Some(coord_signals) = detect_coordinator_signals(metrics) {
            return ComplexityPattern::Coordinator {
                action_count: coord_signals.actions,
                comparison_count: coord_signals.comparisons,
                cyclomatic: metrics.cyclomatic,
                cognitive: metrics.cognitive,
            };
        }

        // ... existing pattern detection logic ...
    }
}
```

#### 3. Signal Detection Functions

**State Machine Signals:**
- Function name contains: `reconcile`, `transition`, `handle_state`, `process_event`
- Presence of match expressions on enum-like types
- Nested conditionals with state comparisons (e.g., `mode != target.mode`)
- Multiple action dispatches within conditional blocks

**Coordinator Signals:**
- Action accumulation: `vec![]` + multiple `push()` calls, or similar
- State comparison patterns: `if current.field != target.field`
- Helper function calls: `calculate_diff`, `compute_delta`, etc.
- Return type is collection of actions or commands

#### 4. AST Analysis Requirements

Extend existing analyzers to track:

**Rust** (`src/analyzers/rust.rs`):
- Detect `match` expressions on enum types
- Track variable patterns: `mut actions = vec![]`
- Count `push()` calls on action vectors
- Identify state comparison expressions

**Python** (`src/analyzers/python.rs`):
- Detect state class attribute comparisons
- Track list building: `actions = []` + `actions.append()`
- Identify state transition function calls

**JavaScript/TypeScript** (`src/analyzers/javascript/*.rs`):
- Detect switch statements on state-like variables
- Track array accumulation: `const actions = []` + `actions.push()`
- Identify state machine libraries (XState, Robot, etc.)

#### 5. Recommendation Generation

File: `src/priority/scoring/concise_recommendation.rs`

```rust
fn generate_state_machine_recommendation(
    transitions: u32,
    cyclomatic: u32,
    cognitive: u32,
    nesting: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let extraction_impact = RefactoringImpact::state_transition_extraction(transitions);
    let language = crate::core::Language::from_path(&metrics.file);

    let steps = vec![
        ActionStep {
            description: "Extract each state transition into a named function".to_string(),
            impact: format!(
                "-{} cognitive, -{} cyclomatic ({} impact)",
                extraction_impact.cognitive_reduction,
                extraction_impact.cyclomatic_reduction,
                extraction_impact.confidence.as_str()
            ),
            difficulty: Difficulty::Medium,
            commands: add_state_transition_hints(&language),
        },
        ActionStep {
            description: "Create transition map or lookup table".to_string(),
            impact: format!("-{} nesting (flatten conditionals)", nesting - 1),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Replace nested if/match with transition table".to_string(),
                "# Example: HashMap<(State, Event), Action>".to_string(),
            ],
        },
        ActionStep {
            description: "Verify state transitions with property tests".to_string(),
            impact: "Ensure correctness of extracted logic".to_string(),
            difficulty: Difficulty::Medium,
            commands: add_state_verification_tests(&language),
        },
    ];

    let estimated_effort = (transitions as f32) * 0.75; // ~45min per transition

    ActionableRecommendation {
        primary_action: format!(
            "Extract {} state transitions into named functions",
            transitions
        ),
        rationale: format!(
            "State machine pattern detected with {} transitions. \
             Extracting transitions will reduce complexity from {}/{} to ~{}/{}.",
            transitions,
            cyclomatic,
            cognitive,
            cyclomatic - extraction_impact.cyclomatic_reduction,
            cognitive - extraction_impact.cognitive_reduction
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort),
    }
}
```

#### 6. Impact Estimation

File: `src/priority/refactoring_impact.rs`

Add new impact estimator:

```rust
impl RefactoringImpact {
    /// Estimate impact of extracting state transition functions
    pub fn state_transition_extraction(transition_count: u32) -> Self {
        // Each extracted transition typically reduces:
        // - Cyclomatic by 2-3 (condition + branches)
        // - Cognitive by 4-6 (nesting + logic)
        let cyclomatic_reduction = (transition_count * 2).min(12);
        let cognitive_reduction = (transition_count * 5).min(20);

        RefactoringImpact {
            complexity_reduction: cyclomatic_reduction,
            cognitive_reduction,
            cyclomatic_reduction,
            confidence: if transition_count >= 3 {
                ImpactConfidence::High
            } else {
                ImpactConfidence::Medium
            },
            technique: "State transition extraction".to_string(),
        }
    }

    /// Estimate impact of coordinator pattern extraction
    pub fn coordinator_extraction(action_count: u32, comparison_count: u32) -> Self {
        // Coordinator refactoring impact depends on:
        // - Number of actions (each action = 1-2 complexity)
        // - Number of comparisons (each comparison = 1-2 complexity)
        let cyclomatic_reduction = (comparison_count * 2).min(10);
        let cognitive_reduction = (action_count + comparison_count).min(15);

        RefactoringImpact {
            complexity_reduction: cyclomatic_reduction,
            cognitive_reduction,
            cyclomatic_reduction,
            confidence: if action_count >= 4 && comparison_count >= 2 {
                ImpactConfidence::High
            } else {
                ImpactConfidence::Medium
            },
            technique: "Coordinator logic extraction".to_string(),
        }
    }
}
```

### Architecture Changes

1. **New Module**: `src/analyzers/state_pattern_signals.rs`
   - Pure functions for detecting state machine signals from AST
   - Language-agnostic signal detection interface
   - Configurable heuristics (thresholds, patterns)

2. **Modified Modules**:
   - `src/priority/complexity_patterns.rs`: Add state pattern variants
   - `src/priority/scoring/concise_recommendation.rs`: Add state-specific recommendations
   - `src/priority/refactoring_impact.rs`: Add impact estimators
   - `src/analyzers/rust.rs`: Track state machine signals during AST traversal
   - `src/analyzers/python.rs`: Track state machine signals
   - `src/analyzers/javascript/*.rs`: Track state machine signals

3. **Configuration**: Add tunable parameters to `debtmap.toml`:
   ```toml
   [patterns.state_machine]
   enabled = true
   min_transitions = 2
   min_cyclomatic = 6
   min_cognitive = 12

   [patterns.coordinator]
   enabled = true
   min_actions = 3
   min_comparisons = 2
   ```

### Data Structures

```rust
/// Signals indicating state machine pattern
#[derive(Debug, Clone)]
pub struct StateMachineSignals {
    pub transition_count: u32,
    pub has_enum_match: bool,
    pub has_state_comparison: bool,
    pub action_dispatch_count: u32,
    pub confidence: f64,
}

/// Signals indicating coordinator pattern
#[derive(Debug, Clone)]
pub struct CoordinatorSignals {
    pub actions: u32,
    pub comparisons: u32,
    pub has_action_accumulation: bool,
    pub has_helper_calls: bool,
    pub confidence: f64,
}
```

### Example Detection

Input code (Rust):
```rust
pub fn reconcile_state(current: State, target: State) -> Result<Vec<Action>> {
    let mut actions = vec![];

    if current.mode != target.mode {
        if current.has_active_connections() {
            if target.mode == Mode::Offline {
                actions.push(drain_connections());
                if current.has_pending_writes() {
                    actions.push(flush_writes());
                }
            }
        } else if target.allows_reconnect() {
            actions.push(establish_connections());
        }
    }

    if let Some(diff) = calculate_config_diff(&current, &target) {
        if diff.requires_restart() {
            actions.push(schedule_restart());
        }
    }

    Ok(actions)
}
```

Detected signals:
- State comparison: `current.mode != target.mode` ✓
- Action accumulation: `let mut actions = vec![]` + 4x `actions.push()` ✓
- Nested conditionals on state: nesting = 4 ✓
- Helper function call: `calculate_config_diff()` ✓
- State method calls: `has_active_connections()`, `allows_reconnect()` ✓

Pattern: **Coordinator** (high confidence)

Recommendation:
```
Extract state reconciliation logic into transition functions

RATIONALE: Coordinator pattern detected with 4 actions and 2 state comparisons.
Extracting transitions will reduce complexity from 9/16 to ~5/8.

STEPS:
1. Extract each state transition into a named function
   Impact: -8 cognitive, -4 cyclomatic (high impact)
   Difficulty: Medium

2. Create transition map or lookup table
   Impact: -3 nesting (flatten conditionals)
   Difficulty: Medium

3. Verify state transitions with property tests
   Impact: Ensure correctness of extracted logic
   Difficulty: Medium

Estimated effort: 3.0 hours
```

## Dependencies

**Prerequisites**: None (uses existing complexity analysis infrastructure)

**Affected Components**:
- `ComplexityPattern` enum (new variants)
- `ComplexityPattern::detect()` (priority changes)
- `generate_complexity_steps()` (new recommendation branches)
- AST analyzers (track additional signals)

**External Dependencies**: None

## Testing Strategy

### Unit Tests

File: `src/priority/complexity_patterns.rs`

```rust
#[test]
fn detect_state_machine_pattern() {
    let metrics = ComplexityMetrics {
        cyclomatic: 9,
        cognitive: 16,
        nesting: 4,
        entropy_score: Some(0.32),
        state_signals: Some(StateMachineSignals {
            transition_count: 3,
            has_enum_match: true,
            has_state_comparison: true,
            action_dispatch_count: 4,
            confidence: 0.85,
        }),
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(matches!(pattern, ComplexityPattern::StateMachine { .. }));

    if let ComplexityPattern::StateMachine { state_transitions, .. } = pattern {
        assert_eq!(state_transitions, 3);
    }
}

#[test]
fn detect_coordinator_pattern() {
    let metrics = ComplexityMetrics {
        cyclomatic: 8,
        cognitive: 14,
        nesting: 3,
        entropy_score: Some(0.28),
        coordinator_signals: Some(CoordinatorSignals {
            actions: 4,
            comparisons: 2,
            has_action_accumulation: true,
            has_helper_calls: true,
            confidence: 0.80,
        }),
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(matches!(pattern, ComplexityPattern::Coordinator { .. }));
}

#[test]
fn state_pattern_takes_precedence_over_nesting() {
    // High nesting metrics BUT state machine signals
    let metrics = ComplexityMetrics {
        cyclomatic: 12,
        cognitive: 50,
        nesting: 5,
        entropy_score: Some(0.35),
        state_signals: Some(StateMachineSignals {
            transition_count: 4,
            has_enum_match: true,
            has_state_comparison: true,
            action_dispatch_count: 6,
            confidence: 0.90,
        }),
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(
        matches!(pattern, ComplexityPattern::StateMachine { .. }),
        "State machine pattern should take precedence over generic high nesting"
    );
}
```

### Integration Tests

File: `tests/state_pattern_detection.rs`

```rust
#[test]
fn analyze_reconcile_state_function() {
    let code = include_str!("../samples/state_reconciliation.rs");
    let result = analyze_rust_code(code, "state_reconciliation.rs");

    // Find reconcile_state function
    let func = result.functions.iter()
        .find(|f| f.name == "reconcile_state")
        .expect("reconcile_state not found");

    // Verify pattern detection
    let pattern = ComplexityPattern::detect(&ComplexityMetrics {
        cyclomatic: func.cyclomatic,
        cognitive: func.cognitive,
        nesting: func.nesting,
        entropy_score: func.entropy_score.as_ref().map(|e| e.token_entropy),
        // Signals should be populated by analyzer
    });

    assert!(
        matches!(pattern, ComplexityPattern::Coordinator { .. }),
        "Expected Coordinator pattern, got: {:?}", pattern
    );

    // Verify recommendation
    let recommendation = generate_concise_recommendation(
        &DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        },
        func,
        FunctionRole::Coordinator,  // Should be classified as coordinator
        &None,
    );

    assert!(
        recommendation.primary_action.contains("transition"),
        "Recommendation should mention state transitions"
    );

    assert!(
        recommendation.estimated_effort_hours.unwrap() >= 2.0,
        "State machine refactoring should estimate >= 2 hours"
    );
}
```

### Property Tests

Use `proptest` to verify pattern detection invariants:

```rust
proptest! {
    #[test]
    fn state_machine_requires_minimum_complexity(
        transitions in 1u32..10,
        cyclomatic in 1u32..30,
        cognitive in 1u32..100,
    ) {
        let signals = StateMachineSignals {
            transition_count: transitions,
            has_enum_match: true,
            has_state_comparison: true,
            action_dispatch_count: transitions * 2,
            confidence: 0.8,
        };

        let metrics = ComplexityMetrics {
            cyclomatic,
            cognitive,
            nesting: 3,
            entropy_score: Some(0.3),
            state_signals: Some(signals),
        };

        let pattern = ComplexityPattern::detect(&metrics);

        // State machine pattern should only trigger if complexity is significant
        if matches!(pattern, ComplexityPattern::StateMachine { .. }) {
            prop_assert!(cyclomatic >= 6 && cognitive >= 12);
        }
    }
}
```

### Performance Tests

Benchmark pattern detection overhead:

```rust
#[bench]
fn bench_pattern_detection_with_state_signals(b: &mut Bencher) {
    let metrics = ComplexityMetrics {
        cyclomatic: 9,
        cognitive: 16,
        nesting: 4,
        entropy_score: Some(0.32),
        state_signals: Some(StateMachineSignals {
            transition_count: 3,
            has_enum_match: true,
            has_state_comparison: true,
            action_dispatch_count: 4,
            confidence: 0.85,
        }),
    };

    b.iter(|| {
        black_box(ComplexityPattern::detect(&metrics))
    });
}
```

Target: < 100ns per detection (negligible overhead)

## Documentation Requirements

### Code Documentation

1. **Module-level docs** in `src/analyzers/state_pattern_signals.rs`:
   - Explain state machine vs coordinator patterns
   - Provide examples of each pattern
   - Document detection heuristics and thresholds

2. **Function docs** for `ComplexityPattern::detect()`:
   - Update priority order documentation
   - Add examples of state pattern detection
   - Document new pattern variants

3. **Examples** in `ComplexityPattern` enum:
   - Show state machine detection example
   - Show coordinator detection example
   - Explain when each pattern is preferred

### User Documentation

Update `README.md` or documentation site:

1. **Pattern Detection Section**:
   - List all supported patterns (including state machine/coordinator)
   - Explain what each pattern means
   - Provide refactoring guidance for each

2. **Configuration Guide**:
   - Document `[patterns.state_machine]` configuration
   - Document `[patterns.coordinator]` configuration
   - Provide tuning recommendations

3. **Examples**:
   - Add `state_reconciliation.rs` to examples directory
   - Show before/after refactoring for state machine pattern
   - Include expected recommendation output

### Architecture Updates

Update `ARCHITECTURE.md`:

1. **Pattern Detection Pipeline**:
   - Document pattern priority order
   - Explain signal detection flow
   - Show integration with recommendation generation

2. **Extensibility**:
   - How to add new pattern types
   - How to add language-specific signal detection
   - Configuration and tuning guidelines

## Implementation Notes

### Detection Priority Order

Pattern detection should follow this priority (highest to lowest):

1. **State Machine / Coordinator** (specific, high-value patterns)
2. **Chaotic Structure** (requires standardization first)
3. **High Nesting** (primary driver is depth)
4. **High Branching** (primary driver is decisions)
5. **Mixed Complexity** (both nesting and branching)
6. **Moderate Complexity** (default/fallback)

Rationale: More specific patterns provide more actionable recommendations.

### Avoiding False Positives

**Challenge**: Generic conditionals might look like state machines.

**Mitigation**:
- Require multiple signals (not just one)
- Use confidence scoring (weighted signals)
- Set minimum complexity thresholds
- Validate against test corpus with known patterns

**Confidence Scoring**:
```rust
fn calculate_state_machine_confidence(signals: &StateMachineSignals) -> f64 {
    let mut confidence = 0.0;

    if signals.has_enum_match { confidence += 0.4; }
    if signals.has_state_comparison { confidence += 0.3; }
    if signals.action_dispatch_count >= 3 { confidence += 0.2; }
    if signals.transition_count >= 2 { confidence += 0.1; }

    confidence.min(1.0)
}
```

Only trigger state machine pattern if confidence >= 0.7.

### Language-Specific Considerations

**Rust**:
- Strong typing makes enum detection reliable
- Match expressions are clear state transition points
- Look for `match` on enum types

**Python**:
- Duck typing makes detection harder
- Look for class attributes like `state`, `mode`, `status`
- Detect `if self.state == State.X` patterns

**JavaScript/TypeScript**:
- TypeScript enums help (when present)
- Look for string constants: `if (state === 'ACTIVE')`
- Detect state machine libraries (XState, Robot)

### Refactoring Examples

For `reconcile_state()`, recommended refactoring:

**Before**:
```rust
pub fn reconcile_state(current: State, target: State) -> Result<Vec<Action>> {
    let mut actions = vec![];
    if current.mode != target.mode {
        if current.has_active_connections() {
            if target.mode == Mode::Offline {
                actions.push(drain_connections());
                if current.has_pending_writes() {
                    actions.push(flush_writes());
                }
            }
        } else if target.allows_reconnect() {
            actions.push(establish_connections());
        }
    }
    // ... more conditionals ...
}
```

**After** (extracted transitions):
```rust
pub fn reconcile_state(current: State, target: State) -> Result<Vec<Action>> {
    let mut actions = vec![];

    if current.mode != target.mode {
        actions.extend(handle_mode_transition(&current, &target));
    }

    if let Some(diff) = calculate_config_diff(&current, &target) {
        actions.extend(handle_config_change(&diff));
    }

    Ok(actions)
}

fn handle_mode_transition(current: &State, target: &State) -> Vec<Action> {
    match (current.mode.clone(), target.mode.clone()) {
        (Mode::Online, Mode::Offline) if current.has_active_connections() => {
            handle_online_to_offline(current)
        }
        (Mode::Offline, Mode::Online) if target.allows_reconnect() => {
            vec![establish_connections()]
        }
        _ => vec![],
    }
}

fn handle_online_to_offline(current: &State) -> Vec<Action> {
    let mut actions = vec![drain_connections()];
    if current.has_pending_writes() {
        actions.push(flush_writes());
    }
    actions
}
```

Complexity reduction: 9 → 5 cyclomatic, 16 → 8 cognitive

## Migration and Compatibility

### Breaking Changes

None. This is a pure addition with no API changes.

### Compatibility

- Existing complexity analysis continues to work
- New patterns detected alongside existing patterns
- Configuration is optional (defaults to enabled)

### Migration Path

1. Deploy with patterns enabled (default)
2. Collect metrics on detection accuracy
3. Tune thresholds based on false positive rate
4. Add language-specific improvements iteratively

### Rollback Plan

If pattern detection causes issues:
1. Set `patterns.state_machine.enabled = false` in config
2. Patterns will fall back to generic detection
3. No code changes required

## Success Metrics

### Quantitative

- **Detection rate**: >= 50% of state machine functions detected
- **Precision**: >= 70% (avoid false positives)
- **Performance**: < 5% analysis time overhead
- **Recommendation quality**: User satisfaction >= 80% (survey)

### Qualitative

- Recommendations are more specific and actionable
- Users report state machine recommendations as helpful
- Refactoring guidance matches expert manual recommendations

### Validation

1. **Test corpus**: Create 50+ examples of state machines across languages
2. **Benchmark**: Measure detection accuracy on corpus
3. **User study**: Survey 10+ users on recommendation quality
4. **Performance**: Profile analysis time with/without pattern detection

## Open Questions

1. **Should we detect finite state machine (FSM) libraries explicitly?**
   - Pros: Higher confidence, library-specific recommendations
   - Cons: Maintenance burden, language-specific
   - Decision: Start with generic detection, add library support later

2. **How to handle partial state machines (only 1-2 transitions)?**
   - Current plan: Require >= 2 transitions for pattern
   - Alternative: Lower threshold but reduce recommendation confidence
   - Decision: Stick with >= 2 to avoid noise

3. **Should coordinator pattern be separate from state machine?**
   - Current plan: Yes, they have different refactoring strategies
   - Alternative: Single "orchestration" pattern with sub-types
   - Decision: Keep separate for clarity

4. **How to validate impact estimates (30-50% complexity reduction)?**
   - Current plan: Empirical testing on sample refactorings
   - Need: Collect real-world refactoring data
   - Decision: Start with conservative estimates, refine based on data

## Future Enhancements

1. **Actor pattern detection**: Functions that process messages and update state
2. **Event sourcing pattern**: Functions that accumulate events
3. **Command pattern**: Functions that dispatch commands based on type
4. **FSM library integration**: Detect XState, Robot, statecharts.js usage
5. **Automated refactoring**: Generate skeleton code for extracted transitions
6. **State diagram generation**: Visualize detected state machines

## Related Work

- **Spec 176**: Entropy vs effective complexity - state patterns may have moderate entropy
- **Spec 177**: Role-aware complexity recommendations - coordinators are a role
- **Spec 178**: Fix moderate complexity recommendation logic - state patterns often moderate
- **Spec 138a**: Template code examples - state machine templates
- **Spec 116**: Confidence scoring - applies to pattern detection confidence

## References

- Martin Fowler, "Refactoring: Improving the Design of Existing Code" (state/strategy patterns)
- "State Pattern" in Gang of Four design patterns
- XState documentation (state machine library examples)
- Debtmap issue #XXX (if created for this spec)

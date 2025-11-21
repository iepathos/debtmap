---
number: 189
title: Dispatcher Pattern Detection and Scoring Adjustment
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-11-20
---

# Specification 189: Dispatcher Pattern Detection and Scoring Adjustment

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None (uses existing entropy and pattern infrastructure)

## Context

### Current False Positive Problem

Debtmap currently over-penalizes simple dispatcher/router patterns, ranking them alongside genuinely problematic functions:

**Example from prodigy analysis:**
```
#8 SCORE: 21.4 [CRITICAL]
├─ LOCATION: ./src/cli/router.rs:12 execute_command()
├─ COMPLEXITY: cyclomatic=38 (dampened: 25, factor: 0.66),
│              cognitive=11, nesting=4, entropy=0.25
├─ WHY THIS MATTERS: Many decision points (38 branches) drive cyclomatic complexity
├─ RECOMMENDED ACTION: Split into 4 focused functions ❌ WRONG
```

**Actual code:**
```rust
pub async fn execute_command(command: Option<Commands>) -> Result<()> {
    match command {
        Some(Commands::Run { .. }) => run_workflow(...).await,
        Some(Commands::Exec { .. }) => run_exec(...).await,
        Some(Commands::Batch { .. }) => run_batch(...).await,
        // ... 15 more simple delegations (1-2 lines each)
    }
}
```

**Why This Is Wrong:**
1. ✅ **This IS the correct pattern** for a command router
2. ✅ **Low cognitive complexity (11)** proves it's simple shallow branching
3. ✅ **Low entropy (0.25)** confirms repetitive structure
4. ✅ **Cognitive/cyclomatic ratio (0.29)** is the key signal for dispatchers
5. ❌ **Recommendation would make code worse**, not better

### Root Cause Analysis

**Current scoring formula** (ComplexityRiskStage:101-103):
```rust
let complexity_score = (cyclomatic + cognitive) / 2.0;
// router.rs: (25 + 11) / 2.0 = 18 → HIGH SCORE
```

**Missing:** The cognitive/cyclomatic ratio which distinguishes pattern types:
- **Dispatcher**: ratio < 0.5 (many shallow branches) → Should be LOW priority
- **Deep Nesting**: ratio > 3.0 (nested logic) → Should be HIGH priority

### Why Existing Systems Don't Catch This

**Entropy dampening works but isn't enough:**
- Already reduced 38 → 25 (34% reduction)
- But capped at 50% maximum dampening
- Dispatcher patterns need >70% reduction

**Coordinator detection exists but is different:**
- Coordinator: Action accumulation (`vec.push`) + state comparisons
- Dispatcher: Simple routing with NO accumulation or comparisons
- Router.rs has no coordinator signals, falls through to generic "HighBranching"

### Pattern Comparison

| Pattern | Vec.push? | Comparisons? | Cognitive Ratio | Example |
|---------|-----------|--------------|-----------------|---------|
| **Coordinator** | ✓ Yes | ✓ Yes | 1.5-2.5 | tokenize() - reconciles state |
| **Dispatcher** | ✗ No | ✗ No | < 0.5 | execute_command() - routes commands |

## Objective

Add dispatcher pattern detection to debtmap that:
1. **Accurately identifies** simple routing/dispatching patterns
2. **Reduces false positive scoring** for acceptable dispatcher complexity
3. **Provides specific recommendations** tailored to dispatcher patterns
4. **Preserves high priority** for genuinely problematic deep nesting

## Requirements

### Functional Requirements

1. **Pattern Detection**
   - Detect dispatcher pattern when:
     - Cyclomatic complexity ≥ 15 (high branching)
     - Cognitive/cyclomatic ratio < 0.5 (shallow branching)
     - No coordinator signals (not action accumulation pattern)
   - Count inline logic branches (where cognitive exceeds expected for dispatcher)
   - Distinguish clean dispatcher from dispatcher needing cleanup

2. **Severity Classification**
   - Info severity: Clean dispatcher (0 inline logic branches)
   - Low severity: 1-3 branches need extraction
   - Medium severity: 4-8 branches need extraction
   - High severity: >8 branches need extraction

3. **Scoring Adjustment**
   - Apply cognitive ratio weighting to complexity score
   - Dispatcher pattern (ratio < 0.5): 70% score reduction
   - Simple branching (ratio < 1.0): 40% score reduction
   - Balanced (ratio 1.0-2.0): No adjustment
   - Deep nesting (ratio 2.0-3.0): 30% score increase
   - Very deep nesting (ratio > 3.0): 50% score increase

4. **Recommendations**
   - Info severity: "Acceptable dispatcher pattern, no action needed"
   - Low/Medium: "Extract inline logic from N specific branches"
   - High: "Extract inline logic + consider splitting dispatcher if branches exceed 25"
   - Never recommend "split into 4 functions" for dispatchers

### Non-Functional Requirements

1. **Accuracy**
   - Zero false negatives: Deep nesting patterns must not be misclassified
   - <5% false positives: Minimize incorrect dispatcher classification
   - Validated against 100+ real-world functions

2. **Performance**
   - Pattern detection adds <10ms per function analyzed
   - No impact on existing analysis pipeline
   - Maintains incremental analysis capabilities

3. **Integration**
   - Works seamlessly with existing entropy dampening
   - Compatible with coordinator pattern detection
   - Uses existing ComplexityPattern enum structure
   - No breaking changes to output format

## Acceptance Criteria

### Pattern Detection
- [ ] Dispatcher pattern detected for router.rs execute_command() (ratio 0.29)
- [ ] Coordinator pattern still detected for tokenize() (ratio 1.73)
- [ ] Deep nesting still detected for execute_mapreduce_resume() (ratio 4.0)
- [ ] No false negatives on 20+ known deep nesting functions
- [ ] <3 false positives on 100+ analyzed functions

### Scoring Adjustment
- [ ] router.rs drops from rank #8 to #15-20 (score: 21.4 → ~6-8)
- [ ] execute_mapreduce_resume() stays in top 3 (score increases from 40.5)
- [ ] Clean dispatchers get Info severity with 90% score reduction
- [ ] Dispatchers with inline logic get appropriate severity (Low/Medium)

### Recommendations
- [ ] router.rs recommendation: "Extract inline logic from 3 branches" (specific)
- [ ] No "split into N functions" for clean dispatchers
- [ ] Deep nesting functions keep "reduce nesting" recommendations
- [ ] Each recommendation includes cognitive ratio in explanation

### Integration
- [ ] Works with existing entropy dampening (38 → 25 → 7)
- [ ] Compatible with all existing pattern types
- [ ] No regression in coordinator pattern detection
- [ ] Output format remains backward compatible

### Testing
- [ ] Unit tests for dispatcher detection logic (10+ cases)
- [ ] Integration test with prodigy codebase analysis
- [ ] Property test: ratio < 0.5 always reduces score
- [ ] Regression tests: existing patterns unchanged

## Technical Details

### Implementation Approach

**Phase 1: Add Dispatcher Pattern Variant**

Location: `src/priority/complexity_patterns.rs`

Add to ComplexityPattern enum (after Coordinator, before ChaoticStructure):
```rust
/// Simple dispatcher with many shallow branches (e.g., command router)
/// Cognitive/Cyclomatic ratio < 0.5 indicates shallow branching
Dispatcher {
    branch_count: u32,
    cognitive_ratio: f64,
    inline_logic_branches: u32, // Branches exceeding expected cognitive load
},
```

**Phase 2: Add Detection Logic**

Insert in `ComplexityPattern::detect()` after coordinator check (line ~220):
```rust
// Check for dispatcher pattern (after coordinator, before chaotic)
if metrics.cyclomatic >= 15
    && ratio < 0.5
    && metrics.coordinator_signals.is_none()
{
    // Estimate inline logic: if cognitive exceeds expected for dispatcher
    let expected_cognitive = (metrics.cyclomatic as f64 * 0.3) as u32;
    let inline_logic_branches = if metrics.cognitive > expected_cognitive {
        ((metrics.cognitive - expected_cognitive) as f64 / 2.0) as u32
    } else {
        0
    };

    return ComplexityPattern::Dispatcher {
        branch_count: metrics.cyclomatic,
        cognitive_ratio: ratio,
        inline_logic_branches,
    };
}
```

**Phase 3: Add Severity Method**

Add to ComplexityPattern impl:
```rust
pub fn severity(&self) -> PatternSeverity {
    match self {
        ComplexityPattern::Dispatcher { inline_logic_branches, .. } => {
            match inline_logic_branches {
                0 => PatternSeverity::Info,        // Clean dispatcher
                1..=3 => PatternSeverity::Low,     // Minor cleanup
                4..=8 => PatternSeverity::Medium,  // Moderate cleanup
                _ => PatternSeverity::High,        // Needs refactoring
            }
        }
        // ... existing patterns
    }
}
```

**Phase 4: Weighted Scoring**

Location: `src/risk/priority/stages.rs`

Replace ComplexityRiskStage implementation (lines 99-107):
```rust
impl PrioritizationStage for ComplexityRiskStage {
    fn process(&self, mut targets: Vec<TestTarget>) -> Vec<TestTarget> {
        for target in &mut targets {
            let cyclo = target.complexity.cyclomatic_complexity as f64;
            let cognitive = target.complexity.cognitive_complexity as f64;
            let ratio = cognitive / cyclo.max(1.0);

            // Apply cognitive ratio weighting
            let weight = match ratio {
                r if r < 0.5 => 0.3,   // Dispatcher: 70% reduction
                r if r < 1.0 => 0.6,   // Simple: 40% reduction
                r if r < 2.0 => 1.0,   // Balanced: no change
                r if r < 3.0 => 1.3,   // Nested: 30% increase
                _ => 1.5,              // Very nested: 50% increase
            };

            let complexity_score = ((cyclo + cognitive) / 2.0) * weight;
            target.priority_score += complexity_score * target.current_risk / 10.0;
        }
        targets
    }
}
```

**Phase 5: Enhanced Recommendations**

Location: `src/priority/formatter/sections.rs`

Add dispatcher-specific recommendation generation:
```rust
match pattern {
    ComplexityPattern::Dispatcher {
        inline_logic_branches,
        cognitive_ratio,
        branch_count,
    } => {
        if *inline_logic_branches == 0 {
            format!(
                "Clean dispatcher pattern ({} branches, ratio: {:.2})\n\
                 No action needed - this is acceptable complexity for a router.",
                branch_count, cognitive_ratio
            )
        } else {
            format!(
                "Dispatcher with {} branches needing cleanup (ratio: {:.2})\n\
                 Extract inline logic from {} branches into helper functions.\n\
                 Keep dispatcher as thin router (1-2 lines per branch).",
                branch_count, cognitive_ratio, inline_logic_branches
            )
        }
    }
    // ... existing patterns
}
```

### Data Structures

**New Enum Variant:**
```rust
pub enum PatternSeverity {
    Info,     // Acceptable pattern, no action needed
    Low,      // Minor issues, optional cleanup
    Medium,   // Should be addressed soon
    High,     // Address soon
    Critical, // Urgent refactoring needed
}
```

**Metrics Required:**
- `cyclomatic_complexity: u32` (already exists)
- `cognitive_complexity: u32` (already exists)
- `coordinator_signals: Option<CoordinatorSignals>` (already exists)
- `entropy_score: Option<f64>` (already exists)

No new data collection needed - uses existing metrics!

### Integration Points

**1. Pattern Detection Pipeline:**
```
Existing Flow:
1. RepetitiveValidation
2. StateMachine
3. Coordinator
4. ChaoticStructure ← INSERT DISPATCHER HERE
5. HighNesting
6. HighBranching
7. MixedComplexity
```

**2. Scoring Pipeline:**
```
Existing Flow:
1. Entropy dampening: 38 → 25 (0.66 factor)
2. Base score: (25 + 11) / 2 = 18
3. NEW: Cognitive weighting: 18 × 0.3 = 5.4
4. Risk adjustment: 5.4 × risk / 10
```

**3. Output Format:**
```
Before:
#8 SCORE: 21.4 [CRITICAL]
├─ WHY: Many decision points (38 branches)
└─ ACTION: Split into 4 focused functions

After:
#18 SCORE: 6.4 [LOW]
├─ PATTERN: Dispatcher (ratio: 0.29)
├─ WHY: Simple routing with 3 branches needing cleanup
└─ ACTION: Extract inline logic from 3 specific branches
```

## Dependencies

### Prerequisites
None - leverages existing infrastructure:
- ✅ Entropy analysis (already implemented)
- ✅ Complexity metrics (cyclomatic, cognitive)
- ✅ Pattern detection framework (ComplexityPattern enum)
- ✅ Coordinator detection (provides negative signal)

### Affected Components
- `src/priority/complexity_patterns.rs` - Add Dispatcher variant
- `src/risk/priority/stages.rs` - Update scoring formula
- `src/priority/formatter/sections.rs` - Add recommendations
- Tests in respective modules

### External Dependencies
None - pure Rust, no new crates needed

## Testing Strategy

### Unit Tests

**Pattern Detection Tests:**
```rust
#[test]
fn detect_dispatcher_clean() {
    let metrics = ComplexityMetrics {
        cyclomatic: 20,
        cognitive: 6,  // ratio: 0.30
        nesting: 2,
        entropy_score: Some(0.25),
        coordinator_signals: None,
        // ...
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(matches!(pattern, ComplexityPattern::Dispatcher { .. }));

    if let ComplexityPattern::Dispatcher { inline_logic_branches, .. } = pattern {
        assert_eq!(inline_logic_branches, 0);
    }
}

#[test]
fn detect_dispatcher_with_inline_logic() {
    let metrics = ComplexityMetrics {
        cyclomatic: 38,
        cognitive: 11,  // ratio: 0.29
        // ...
    };

    let pattern = ComplexityPattern::detect(&metrics);
    if let ComplexityPattern::Dispatcher { inline_logic_branches, .. } = pattern {
        assert!(inline_logic_branches > 0);
    }
}

#[test]
fn coordinator_not_misclassified_as_dispatcher() {
    let metrics = ComplexityMetrics {
        cyclomatic: 20,
        cognitive: 10,  // ratio: 0.50 (borderline)
        coordinator_signals: Some(CoordinatorSignals {
            actions: 5,
            comparisons: 3,
            confidence: 0.8,
            // ...
        }),
        // ...
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(matches!(pattern, ComplexityPattern::Coordinator { .. }));
}

#[test]
fn deep_nesting_preserved() {
    let metrics = ComplexityMetrics {
        cyclomatic: 20,
        cognitive: 80,  // ratio: 4.0
        nesting: 6,
        // ...
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(matches!(pattern, ComplexityPattern::HighNesting { .. }));
}
```

**Scoring Tests:**
```rust
#[test]
fn dispatcher_score_reduction() {
    // router.rs: cyclo=38, cognitive=11, ratio=0.29
    let cyclo = 38.0;
    let cognitive = 11.0;
    let ratio = cognitive / cyclo;

    assert!(ratio < 0.5);
    let weight = 0.3;  // 70% reduction

    let score = ((cyclo + cognitive) / 2.0) * weight;
    assert!((score - 7.35).abs() < 0.1);
}

#[test]
fn deep_nesting_score_increase() {
    // execute_mapreduce_resume: cyclo=20, cognitive=80, ratio=4.0
    let cyclo = 20.0;
    let cognitive = 80.0;
    let ratio = cognitive / cyclo;

    assert!(ratio > 3.0);
    let weight = 1.5;  // 50% increase

    let score = ((cyclo + cognitive) / 2.0) * weight;
    assert!((score - 75.0).abs() < 0.1);
}
```

### Integration Tests

**Prodigy Codebase Analysis:**
```rust
#[test]
fn prodigy_router_prioritization() {
    let results = analyze_prodigy_codebase();

    let router = results.find_function("execute_command");
    let deep_nesting = results.find_function("execute_mapreduce_resume");

    // Router should be deprioritized
    assert!(router.priority_score < 10.0);
    assert!(router.rank > 15);
    assert!(matches!(router.pattern, ComplexityPattern::Dispatcher { .. }));

    // Deep nesting should remain high priority
    assert!(deep_nesting.priority_score > 30.0);
    assert!(deep_nesting.rank <= 3);
}
```

**Regression Tests:**
```rust
#[test]
fn no_false_negatives_on_known_issues() {
    let known_problems = load_verified_complex_functions();

    for func in known_problems {
        let pattern = detect_pattern(&func);

        // None should be classified as Info severity dispatcher
        if let ComplexityPattern::Dispatcher { .. } = pattern {
            let severity = pattern.severity();
            assert_ne!(severity, PatternSeverity::Info);
        }
    }
}
```

### Property Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn low_ratio_always_reduces_score(
        cyclomatic in 15u32..100,
        cognitive in 1u32..50
    ) {
        let ratio = cognitive as f64 / cyclomatic as f64;
        prop_assume!(ratio < 0.5);

        let base_score = (cyclomatic as f64 + cognitive as f64) / 2.0;
        let weighted_score = calculate_weighted_score(cyclomatic, cognitive);

        prop_assert!(weighted_score < base_score);
    }

    #[test]
    fn high_ratio_always_increases_score(
        cyclomatic in 10u32..50,
        cognitive in 60u32..150
    ) {
        let ratio = cognitive as f64 / cyclomatic as f64;
        prop_assume!(ratio > 3.0);

        let base_score = (cyclomatic as f64 + cognitive as f64) / 2.0;
        let weighted_score = calculate_weighted_score(cyclomatic, cognitive);

        prop_assert!(weighted_score > base_score);
    }
}
```

### Performance Tests

```rust
#[bench]
fn bench_dispatcher_detection(b: &mut Bencher) {
    let metrics = create_test_metrics();

    b.iter(|| {
        black_box(ComplexityPattern::detect(&metrics));
    });
}

#[test]
fn pattern_detection_overhead() {
    let start = Instant::now();

    for _ in 0..1000 {
        let pattern = ComplexityPattern::detect(&test_metrics());
    }

    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(10));
}
```

## Documentation Requirements

### Code Documentation

**Pattern Module:**
```rust
/// Dispatcher pattern: Simple routing/dispatching with many shallow branches
///
/// # Characteristics
/// - High cyclomatic complexity (many branches)
/// - Low cognitive complexity (shallow nesting)
/// - Cognitive/cyclomatic ratio < 0.5
/// - No action accumulation (unlike Coordinator)
///
/// # Examples
/// - Command routers (CLI, HTTP endpoints)
/// - Event dispatchers
/// - State machine dispatchers
/// - Switch-based handlers
///
/// # Detection Criteria
/// - Cyclomatic ≥ 15
/// - Cognitive/Cyclomatic < 0.5
/// - No coordinator signals
///
/// # Severity Levels
/// - Info: Clean dispatcher (0 inline logic branches)
/// - Low: 1-3 branches need extraction
/// - Medium: 4-8 branches need extraction
/// - High: >8 branches need extraction
```

**Scoring Documentation:**
```rust
/// Apply cognitive ratio weighting to complexity score
///
/// The cognitive/cyclomatic ratio indicates the type of complexity:
/// - < 0.5: Dispatcher (many shallow branches) → 70% reduction
/// - < 1.0: Simple branching → 40% reduction
/// - 1.0-2.0: Balanced → No adjustment
/// - 2.0-3.0: Deep nesting → 30% increase
/// - > 3.0: Very deep nesting → 50% increase
///
/// # Rationale
/// Dispatcher patterns are acceptable complexity for routers and handlers.
/// Deep nesting is harder to understand and maintain.
```

### User Documentation

**Update README:**
```markdown
### Pattern Detection

Debtmap detects several complexity patterns:

**Dispatcher Pattern**: Simple routing/dispatching
- Many branches with shallow logic
- Example: Command routers, event handlers
- Recommendation: Extract inline logic, keep dispatcher thin

**Coordinator Pattern**: Action accumulation and reconciliation
- State comparisons with action building
- Example: Kubernetes-style reconciliation loops
- Recommendation: Extract reconciliation logic

**Deep Nesting**: Nested conditionals driving complexity
- Cognitive complexity >> Cyclomatic complexity
- Example: Nested error handling, state validation
- Recommendation: Reduce nesting depth with guard clauses
```

**Add to ARCHITECTURE.md:**
```markdown
## Complexity Pattern Detection

### Dispatcher Pattern

Dispatchers route requests/commands to appropriate handlers through
large match/switch statements. While this creates high cyclomatic
complexity, it's acceptable when:

1. Each branch is 1-2 lines (simple delegation)
2. Cognitive complexity remains low (< 15)
3. No action accumulation or state comparisons

Detection uses cognitive/cyclomatic ratio < 0.5 as key signal.

Example:
```rust
match command {
    Cmd::A => handle_a(),  // Simple delegation
    Cmd::B => handle_b(),
    // ... 20 more branches
}
```

Scoring applies 70% reduction to reflect acceptable complexity.
```

## Implementation Notes

### Edge Cases

**1. Hybrid Patterns**
```rust
match command {
    Cmd::A => handle_a(),  // Simple
    Cmd::B => {            // Inline logic (7 lines)
        let x = prepare();
        if validate(x) {
            execute(x)
        }
    }
    Cmd::C => handle_c(),  // Simple
}
```
**Handling:** Count inline logic branches, classify as Medium severity

**2. Nested Dispatchers**
```rust
match outer {
    A => match inner { ... },  // Nested match
    B => handle_b(),
}
```
**Handling:** Nesting depth increases cognitive complexity, ratio rises above 0.5, won't be classified as dispatcher

**3. Large Clean Dispatchers**
```rust
match command {
    // 50+ simple delegation branches
}
```
**Handling:** Still Info severity, but recommendation may suggest splitting if >25 branches for maintainability

### Gotchas

1. **Don't misclassify coordinators:**
   - Check for coordinator signals BEFORE dispatcher
   - Ratio can overlap (0.4-0.6 range)
   - Use coordinator_signals as discriminator

2. **Entropy dampening interaction:**
   - Dispatcher detection happens AFTER entropy dampening
   - Use dampened cyclomatic in ratio calculation
   - Combined effect: entropy 34% + dispatcher 70% = 90% total reduction

3. **Language differences:**
   - Python: if/elif chains instead of match
   - JavaScript: switch statements
   - TypeScript: discriminated unions
   - Adjust detection heuristics per language

## Migration and Compatibility

### Breaking Changes
None - purely additive

### Backward Compatibility
- ✅ Output format unchanged (new pattern type in enum)
- ✅ Existing tests pass (no changes to other patterns)
- ✅ JSON output remains valid (new variant serializes)
- ✅ CLI flags unchanged

### Migration Path
1. Deploy new version
2. Re-analyze codebases
3. Compare before/after rankings
4. Adjust thresholds if needed (ratio cutoff, inline logic estimation)

### Rollback Plan
If issues found:
1. Disable dispatcher detection with feature flag
2. Revert to HighBranching for these functions
3. No data migration needed

## Success Metrics

### Accuracy Metrics
- **False Positive Rate**: <5% of dispatchers misclassified
- **False Negative Rate**: 0% of deep nesting missed
- **Precision**: >95% of dispatcher classifications correct
- **Recall**: >90% of dispatchers detected

### Impact Metrics
- **Priority Shift**: Clean dispatchers drop 10-15 positions
- **Score Reduction**: 70-80% for clean dispatchers
- **Recommendation Quality**: 100% specific (no generic "split function")

### Performance Metrics
- **Analysis Overhead**: <10ms per function
- **Memory Usage**: No increase (uses existing metrics)
- **Throughput**: No regression on large codebases

## Future Enhancements

### Phase 2: Language-Specific Detection
- Python if/elif chain detection
- JavaScript switch statement analysis
- Go select statement handling

### Phase 3: Inline Logic Extraction Automation
- Suggest specific helper function names
- Generate extraction refactoring
- Automate test generation for extracted functions

### Phase 4: Dispatcher Quality Metrics
- Measure branch size variance
- Detect inconsistent patterns (some delegations, some inline)
- Recommend standardization

### Phase 5: Integration with Refactoring Tools
- Generate rust-analyzer quick fixes
- Provide IDE integration
- Automated refactoring suggestions

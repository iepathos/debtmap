---
number: 192
title: State-Aware Coordinator Pattern Detection
category: optimization
priority: high
status: draft
dependencies: [179]
created: 2025-11-23
---

# Specification 192: State-Aware Coordinator Pattern Detection

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 179 (State Machine Pattern Detection)

## Context

The coordinator pattern detector (`src/analyzers/state_machine_pattern_detector.rs`) currently counts **all** comparisons and vector pushes as signals of a coordinator pattern, leading to false positives on validation code.

**Current False Positive Example** (from stillwater):
```rust
// examples/form_validation.rs:20
fn example_contact_form() {
    fn validate_email(email: &str) -> Validation<(), Vec<String>> {
        let mut errors = Vec::new();

        if email.is_empty() {           // ❌ Counted as "state comparison"
            errors.push("required");     // ❌ Counted as "action accumulation"
        }
        if !email.contains('@') {        // ❌ Counted as "state comparison"
            errors.push("invalid");      // ❌ Counted as "action accumulation"
        }

        if errors.is_empty() {
            Validation::success(())
        } else {
            Validation::failure(errors)
        }
    }
    // ... more validation functions ...
}
```

**Debtmap Output**:
```
#2 SCORE: 11.0 [CRITICAL]
├─ LOCATION: ./examples/form_validation.rs:20 example_contact_form()
├─ WHY THIS MATTERS: Coordinator pattern detected with 8 actions and 3 state comparisons.
├─ RECOMMENDED ACTION: Extract state reconciliation logic into transition functions
```

**Problem**: This is **NOT** a coordinator pattern—it's validation code. The detector is conflating:
- **Validation checks** (`if email.is_empty()`) with **state comparisons** (`if current.state != desired.state`)
- **Error accumulation** (`errors.push()`) with **action accumulation** (`actions.push(Action::DoSomething)`)

**True Coordinator Pattern** (what we SHOULD detect):
```rust
fn reconcile_desired_state(current: &State, desired: &State) -> Vec<Action> {
    let mut actions = vec![];

    // State comparison ✓
    if current.mode != desired.mode {
        actions.push(Action::TransitionMode);  // Action accumulation ✓
    }

    // State comparison ✓
    if current.config.replicas < desired.config.replicas {
        actions.push(Action::ScaleUp);         // Action accumulation ✓
    }

    actions  // Return actions for dispatch ✓
}
```

## Objective

Improve coordinator pattern detection to distinguish between:
1. **True coordinators**: State reconciliation with action accumulation
2. **Validation code**: Error accumulation with validation checks
3. **Simple conditionals**: Unrelated comparisons and vector operations

Target: **Reduce false positives by 70-80%** while maintaining 90%+ true positive detection.

## Requirements

### Functional Requirements

**1. State-Specific Comparison Detection**

Only count comparisons that reference state-related identifiers:

```rust
fn is_state_comparison(binary: &ExprBinary) -> bool {
    // Check if left or right side contains state-related identifiers
    contains_state_identifier(&binary.left) || contains_state_identifier(&binary.right)
}

fn contains_state_identifier(expr: &Expr) -> bool {
    match expr {
        Expr::Field(field) => {
            let field_name = get_field_name(field).to_lowercase();
            // State-related field names
            field_name.contains("state")
                || field_name.contains("mode")
                || field_name.contains("status")
                || field_name.contains("phase")
                || field_name.contains("desired")
                || field_name.contains("current")
        }
        Expr::Path(path) => {
            // State-related variable or enum names
            path.path.segments.iter().any(|seg| {
                let name = seg.ident.to_string().to_lowercase();
                name.contains("state") || name.contains("mode") || name.contains("status")
            })
        }
        _ => false,
    }
}
```

**2. Action vs Error Accumulation Distinction**

Differentiate between action accumulation and error accumulation:

```rust
fn is_action_accumulation(method: &ExprMethodCall) -> bool {
    if method.method != "push" {
        return false;
    }

    // Check receiver variable name
    let receiver_name = get_receiver_name(&method.receiver).to_lowercase();

    // Error accumulation (NOT coordinator)
    if receiver_name.contains("error")
        || receiver_name.contains("issue")
        || receiver_name.contains("warning")
        || receiver_name.contains("validation")
    {
        return false;
    }

    // Action accumulation (likely coordinator)
    if receiver_name.contains("action")
        || receiver_name.contains("command")
        || receiver_name.contains("operation")
        || receiver_name.contains("task")
    {
        return true;
    }

    // Ambiguous - check argument type
    is_action_type(&method.args)
}

fn is_action_type(args: &Punctuated<Expr, Token![,]>) -> bool {
    // Check if pushing enum variants that look like actions
    args.iter().any(|arg| {
        if let Expr::Path(path) = arg {
            let path_str = path_to_string(&path.path);
            path_str.contains("Action::")
                || path_str.contains("Command::")
                || path_str.contains("Operation::")
        } else {
            false
        }
    })
}
```

**3. Structural Pattern Recognition**

Require coordinator-specific structure:

```rust
pub fn detect_coordinator(&self, block: &Block) -> Option<CoordinatorSignals> {
    let mut visitor = EnhancedCoordinatorVisitor::new();
    visitor.visit_block(block);

    // NEW: Require evidence of coordinator pattern
    if visitor.state_aware_push_count < 3      // Action pushes with state context
        || visitor.state_comparison_count < 2   // State-related comparisons
    {
        return None;
    }

    // NEW: Penalty for error accumulation patterns
    if visitor.error_accumulation_ratio > 0.5 {
        return None;  // Likely validation code
    }

    // NEW: Bonus for explicit action types
    let has_action_types = visitor.explicit_action_type_count > 0;

    // Calculate confidence with new signals
    let confidence = calculate_enhanced_coordinator_confidence(
        visitor.state_aware_push_count,
        visitor.state_comparison_count,
        visitor.has_helper_calls,
        has_action_types,
        visitor.has_final_dispatch,
    );

    if confidence < 0.7 {  // Raised from 0.6
        return None;
    }

    Some(CoordinatorSignals {
        actions: visitor.state_aware_push_count,
        comparisons: visitor.state_comparison_count,
        has_action_accumulation: true,
        has_helper_calls: visitor.has_helper_calls,
        confidence,
    })
}
```

**4. Enhanced Confidence Scoring**

Incorporate new signals into confidence calculation:

```rust
fn calculate_enhanced_coordinator_confidence(
    state_aware_pushes: u32,
    state_comparisons: u32,
    has_helper_calls: bool,
    has_action_types: bool,
    has_final_dispatch: bool,
) -> f64 {
    let mut confidence = 0.0;

    // State-aware action accumulation (up to 0.4)
    confidence += (state_aware_pushes as f64 / 10.0).min(0.4);

    // State-related comparisons (up to 0.3)
    confidence += (state_comparisons as f64 / 10.0).min(0.3);

    // Helper function calls (0.1)
    if has_helper_calls {
        confidence += 0.1;
    }

    // NEW: Explicit action types (0.15 bonus)
    if has_action_types {
        confidence += 0.15;
    }

    // NEW: Final dispatch pattern (0.1 bonus)
    if has_final_dispatch {
        confidence += 0.1;
    }

    confidence.min(1.0)
}
```

### Non-Functional Requirements

**Precision**:
- False positive rate < 20% (down from current ~60%)
- True positive rate > 90% (maintain current level)
- Confidence scores accurately reflect pattern strength

**Performance**:
- AST analysis overhead < 10% increase
- Pattern detection remains sub-millisecond per function

**Maintainability**:
- State identifier keywords configurable
- Clear separation between heuristics
- Comprehensive test coverage for edge cases

## Acceptance Criteria

- [ ] Stillwater validation examples no longer trigger coordinator pattern:
  - `form_validation.rs:20 example_contact_form()` - No coordinator detection
  - `form_validation.rs:352 example_cross_field_validation()` - No coordinator detection
  - `form_validation.rs:237 example_payment_form()` - No coordinator detection
- [ ] True coordinator patterns still detected with high confidence (≥0.7):
  - State machine reconciliation functions
  - Action orchestration code
  - Event-driven coordination logic
- [ ] State identifier detection works for common patterns:
  - `current.state != desired.state`
  - `if mode == Mode::Active`
  - `status.phase != target.phase`
- [ ] Error accumulation excluded from action counting:
  - `errors.push("message")` not counted
  - `issues.push(Issue::new())` not counted
  - `warnings.push(Warning)` not counted
- [ ] Action accumulation correctly identified:
  - `actions.push(Action::DoX)` counted
  - `commands.push(Command::Y)` counted
  - `operations.push(Op::Z)` counted
- [ ] Confidence scoring reflects pattern strength:
  - High confidence (>0.8): Clear state reconciliation
  - Medium confidence (0.6-0.8): Likely coordinator
  - Low confidence (<0.6): Rejected as false positive
- [ ] Unit tests cover all heuristics
- [ ] Integration tests validate false positive reduction
- [ ] Documentation explains detection logic

## Technical Details

### Implementation Approach

**Phase 1: Enhance Visitor (2 hours)**

Modify `CoordinatorVisitor` in `src/analyzers/state_machine_pattern_detector.rs`:

```rust
struct EnhancedCoordinatorVisitor {
    // Existing fields
    vec_push_count: u32,
    comparison_count: u32,
    has_helper_calls: bool,

    // NEW: State-aware fields
    state_aware_push_count: u32,      // Pushes in state-conditional blocks
    state_comparison_count: u32,       // Comparisons on state-related fields
    error_accumulation_count: u32,     // Pushes to error/issue vectors
    explicit_action_type_count: u32,   // Pushes of Action:: variants
    has_final_dispatch: bool,          // Returns action vector

    // NEW: Context tracking
    current_conditional_is_state_related: bool,
}

impl<'ast> Visit<'ast> for EnhancedCoordinatorVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Binary(binary) => {
                if is_comparison_op(&binary.op) {
                    self.comparison_count += 1;

                    // NEW: Check if state-related
                    if is_state_comparison(binary) {
                        self.state_comparison_count += 1;
                        self.current_conditional_is_state_related = true;
                    }
                }
            }
            Expr::MethodCall(method) if method.method == "push" => {
                self.vec_push_count += 1;

                // NEW: Classify push type
                if is_error_accumulation(method) {
                    self.error_accumulation_count += 1;
                } else {
                    // Check if in state-conditional context
                    if self.current_conditional_is_state_related {
                        self.state_aware_push_count += 1;
                    }

                    // Check if explicit action type
                    if is_action_type(&method.args) {
                        self.explicit_action_type_count += 1;
                    }
                }
            }
            _ => {}
        }

        syn::visit::visit_expr(self, expr);

        // Reset context after visiting conditional
        if matches!(expr, Expr::If(_)) {
            self.current_conditional_is_state_related = false;
        }
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        // Check for final dispatch pattern (return actions)
        if let Stmt::Expr(Expr::Path(path), None) = stmt {
            let path_str = path_to_string(&path.path);
            if path_str.contains("action") || path_str.contains("command") {
                self.has_final_dispatch = true;
            }
        }

        syn::visit::visit_stmt(self, stmt);
    }
}
```

**Phase 2: Add Heuristic Functions (1 hour)**

```rust
// Helper functions for semantic analysis
fn is_state_comparison(binary: &ExprBinary) -> bool {
    contains_state_identifier(&binary.left) || contains_state_identifier(&binary.right)
}

fn contains_state_identifier(expr: &Expr) -> bool {
    match expr {
        Expr::Field(field) => {
            let field_name = get_field_name(field).to_lowercase();
            STATE_FIELD_KEYWORDS.iter().any(|kw| field_name.contains(kw))
        }
        Expr::Path(path) => {
            path.path.segments.iter().any(|seg| {
                let name = seg.ident.to_string().to_lowercase();
                STATE_PATH_KEYWORDS.iter().any(|kw| name.contains(kw))
            })
        }
        _ => false,
    }
}

const STATE_FIELD_KEYWORDS: &[&str] = &[
    "state", "mode", "status", "phase", "stage",
    "desired", "current", "target", "actual",
];

const STATE_PATH_KEYWORDS: &[&str] = &[
    "state", "mode", "status", "phase",
];

fn is_error_accumulation(method: &ExprMethodCall) -> bool {
    let receiver_name = get_receiver_name(&method.receiver).to_lowercase();
    ERROR_ACCUMULATION_KEYWORDS.iter().any(|kw| receiver_name.contains(kw))
}

const ERROR_ACCUMULATION_KEYWORDS: &[&str] = &[
    "error", "err", "issue", "warning", "warn",
    "validation", "invalid", "problem",
];

fn is_action_type(args: &Punctuated<Expr, Token![,]>) -> bool {
    args.iter().any(|arg| {
        if let Expr::Path(path) = arg {
            let path_str = path_to_string(&path.path);
            ACTION_TYPE_PATTERNS.iter().any(|pat| path_str.contains(pat))
        } else {
            false
        }
    })
}

const ACTION_TYPE_PATTERNS: &[&str] = &[
    "Action::", "Command::", "Operation::", "Task::",
    "Event::", "Message::",
];
```

**Phase 3: Update Confidence Calculation (30 minutes)**

Modify `calculate_coordinator_confidence()` to use new signals.

**Phase 4: Testing (1 hour)**

Add comprehensive tests for all heuristics and edge cases.

### Architecture Changes

**Modified Files**:
- `src/analyzers/state_machine_pattern_detector.rs` - Enhanced visitor and detection
- `src/priority/complexity_patterns.rs` - Updated confidence thresholds

**New Concepts**:
- **State-aware push counting**: Only count pushes in state-conditional blocks
- **Semantic vector classification**: Distinguish errors from actions
- **Action type detection**: Recognize explicit action enum variants

### Configuration Support

Allow customization of detection keywords:

```toml
# debtmap.toml
[pattern_detection.coordinator]
# Minimum thresholds
min_state_comparisons = 2
min_state_aware_actions = 3
confidence_threshold = 0.7

# Keywords for state identifiers
state_field_keywords = ["state", "mode", "status", "phase", "desired", "current"]
state_path_keywords = ["state", "mode", "status"]

# Keywords for error accumulation (excluded from action counting)
error_keywords = ["error", "issue", "warning", "validation"]

# Patterns for action types (boost confidence)
action_type_patterns = ["Action::", "Command::", "Operation::"]
```

## Dependencies

**Prerequisites**:
- Spec 179: State Machine Pattern Detection (shares visitor infrastructure)

**Affected Components**:
- Coordinator pattern detection
- Complexity pattern classification
- Recommendation generation

**External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn detects_true_coordinator_pattern() {
    let block: Block = parse_quote! {
        {
            let mut actions = vec![];
            if current.state != desired.state {
                actions.push(Action::TransitionState);
            }
            if current.mode != desired.mode {
                actions.push(Action::ChangeMode);
            }
            actions
        }
    };

    let detector = StateMachinePatternDetector::new();
    let signals = detector.detect_coordinator(&block);

    assert!(signals.is_some());
    let signals = signals.unwrap();
    assert_eq!(signals.actions, 2);
    assert_eq!(signals.comparisons, 2);
    assert!(signals.confidence >= 0.7);
}

#[test]
fn rejects_validation_code() {
    let block: Block = parse_quote! {
        {
            let mut errors = vec![];
            if email.is_empty() {
                errors.push("Email is required");
            }
            if !email.contains('@') {
                errors.push("Invalid email format");
            }
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    };

    let detector = StateMachinePatternDetector::new();
    let signals = detector.detect_coordinator(&block);

    assert!(signals.is_none(), "Validation code should not trigger coordinator pattern");
}

#[test]
fn distinguishes_error_from_action_accumulation() {
    // Error accumulation
    let error_push: ExprMethodCall = parse_quote! {
        errors.push("validation failed")
    };
    assert!(!is_action_accumulation(&error_push));

    // Action accumulation
    let action_push: ExprMethodCall = parse_quote! {
        actions.push(Action::DoSomething)
    };
    assert!(is_action_accumulation(&action_push));
}

#[test]
fn recognizes_state_comparisons() {
    // State comparison (should count)
    let state_comp: ExprBinary = parse_quote! {
        current.state != desired.state
    };
    assert!(is_state_comparison(&state_comp));

    // Non-state comparison (should not count)
    let value_comp: ExprBinary = parse_quote! {
        email.is_empty()
    };
    assert!(!is_state_comparison(&value_comp));
}

#[test]
fn confidence_scoring_works() {
    // High confidence: clear coordinator
    let high = calculate_enhanced_coordinator_confidence(
        5,      // state-aware pushes
        4,      // state comparisons
        true,   // has helper calls
        true,   // has action types
        true,   // has final dispatch
    );
    assert!(high >= 0.8);

    // Low confidence: ambiguous pattern
    let low = calculate_enhanced_coordinator_confidence(
        2,      // few pushes
        1,      // few comparisons
        false,  // no helpers
        false,  // no action types
        false,  // no dispatch
    );
    assert!(low < 0.6);
}
```

### Integration Tests

```rust
#[test]
fn stillwater_validation_not_coordinator() {
    let source = std::fs::read_to_string("../stillwater/examples/form_validation.rs")
        .expect("Failed to read stillwater example");

    let results = analyze_file_for_patterns(&source);

    // Validation functions should not be flagged as coordinators
    for result in results {
        if result.function_name.contains("validate") {
            assert!(
                result.coordinator_pattern.is_none(),
                "Validation function {} incorrectly flagged as coordinator",
                result.function_name
            );
        }
    }
}

#[test]
fn real_coordinators_still_detected() {
    // Test on known coordinator pattern in debtmap codebase
    // (find actual coordinator if exists, or create synthetic test)
    let source = r#"
        fn reconcile_state(current: &State, desired: &State) -> Vec<Action> {
            let mut actions = vec![];

            if current.mode != desired.mode {
                actions.push(Action::TransitionMode { target: desired.mode });
            }

            if current.replicas < desired.replicas {
                actions.push(Action::ScaleUp { count: desired.replicas - current.replicas });
            }

            actions
        }
    "#;

    let results = analyze_source_for_patterns(source);

    assert!(results[0].coordinator_pattern.is_some());
    assert!(results[0].coordinator_pattern.unwrap().confidence >= 0.7);
}
```

### Property Tests

```rust
#[test]
fn error_accumulation_never_triggers_coordinator() {
    proptest!(|(
        error_count in 2..10usize,
        comparison_count in 2..10usize
    )| {
        // Generate validation-like code
        let block = generate_validation_pattern(error_count, comparison_count);

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_coordinator(&block);

        prop_assert!(
            signals.is_none(),
            "Validation pattern with {} errors and {} checks should not trigger coordinator",
            error_count, comparison_count
        );
    });
}

#[test]
fn state_comparisons_required_for_coordinator() {
    proptest!(|(
        action_count in 3..10usize,
        non_state_comparison_count in 2..10usize
    )| {
        // Generate code with actions but no state comparisons
        let block = generate_non_state_pattern(action_count, non_state_comparison_count);

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_coordinator(&block);

        prop_assert!(
            signals.is_none(),
            "Actions without state comparisons should not trigger coordinator"
        );
    });
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Detect coordinator pattern with state-awareness.
///
/// A true coordinator pattern has:
/// - State-related comparisons (e.g., `current.state != desired.state`)
/// - Action accumulation in state-conditional blocks
/// - Explicit action types (e.g., `Action::DoSomething`)
///
/// This distinguishes coordinators from:
/// - Validation code (error accumulation)
/// - Simple conditional logic (non-state comparisons)
///
/// # False Positive Avoidance
///
/// The detector rejects:
/// - Error accumulation patterns (`errors.push(...)`)
/// - Validation checks (`if value.is_empty()`)
/// - Non-state comparisons
///
/// # Examples
///
/// ```rust
/// // TRUE COORDINATOR (detected)
/// let mut actions = vec![];
/// if current.state != desired.state {
///     actions.push(Action::Transition);
/// }
///
/// // VALIDATION CODE (rejected)
/// let mut errors = vec![];
/// if email.is_empty() {
///     errors.push("required");
/// }
/// ```
pub fn detect_coordinator(&self, block: &Block) -> Option<CoordinatorSignals>
```

### User Documentation

Add to README.md:

```markdown
## Pattern Detection: Coordinators

Debtmap detects coordinator patterns—functions that orchestrate actions
based on state comparisons.

### What is a Coordinator?

```rust
fn reconcile(current: &State, desired: &State) -> Vec<Action> {
    let mut actions = vec![];

    if current.mode != desired.mode {           // State comparison
        actions.push(Action::ChangeMode);       // Action accumulation
    }

    if current.replicas < desired.replicas {
        actions.push(Action::ScaleUp);
    }

    actions  // Dispatch
}
```

### Not a Coordinator

Validation code is **not** a coordinator:

```rust
fn validate(email: &str) -> Result<(), Vec<String>> {
    let mut errors = vec![];

    if email.is_empty() {           // Value check (not state)
        errors.push("required");     // Error accumulation (not action)
    }

    // ...
}
```

### Detection Criteria

Debtmap requires:
- 2+ state-related comparisons (`current.state`, `mode`, `status`)
- 3+ action accumulations in state-conditional blocks
- 70%+ confidence score

This avoids false positives on validation and utility code.
```

### Architecture Documentation

Update `ARCHITECTURE.md`:

```markdown
## Coordinator Pattern Detection

### State-Aware Analysis

The coordinator detector uses semantic analysis to distinguish:

1. **State Comparisons** vs **Value Checks**:
   - State: `current.state != desired.state`
   - Value: `email.is_empty()`

2. **Action Accumulation** vs **Error Accumulation**:
   - Actions: `actions.push(Action::DoX)`
   - Errors: `errors.push("message")`

### Heuristics

- **State Identifiers**: `state`, `mode`, `status`, `phase`, `desired`, `current`
- **Error Keywords**: `error`, `issue`, `warning`, `validation`
- **Action Patterns**: `Action::`, `Command::`, `Operation::`

### Confidence Scoring

- High (>0.8): Clear state reconciliation with explicit action types
- Medium (0.6-0.8): Likely coordinator, some ambiguity
- Low (<0.6): Rejected as false positive
```

## Implementation Notes

### Edge Cases

**1. Mixed Patterns**:
```rust
fn process(data: &Data, state: &State) -> Result<(), Vec<String>> {
    let mut errors = vec![];     // Error accumulation
    let mut actions = vec![];    // Action accumulation

    // Validation
    if data.is_empty() {
        errors.push("empty");
    }

    // Coordination
    if state.mode != Mode::Active {
        actions.push(Action::Activate);
    }

    // Which pattern is this?
}
```
**Decision**: Count separately, require dominance (>60%) for classification.

**2. Nested State Access**:
```rust
if resource.metadata.status.phase != Phase::Ready {
    // Should this be detected as state comparison?
}
```
**Decision**: Yes—check full path for state keywords.

**3. Method Call Comparisons**:
```rust
if current.get_state() != desired.get_state() {
    // Not syntactically a field access
}
```
**Decision**: Phase 1 won't detect. Add method-based state detection in future.

### Performance Optimization

**Keyword Matching**:
- Use `contains()` for substring matching (fast)
- Consider trie for many keywords (future optimization)
- Cache lowercase conversions

**AST Traversal**:
- Single-pass visitor (no re-traversal)
- Early exit on confidence threshold

## Migration and Compatibility

### Breaking Changes

None—purely improves existing detection.

### Backward Compatibility

- May reduce detected coordinator patterns (by design—removing false positives)
- Users with custom thresholds may need adjustment
- Confidence scores will be more accurate

### Migration Path

1. Deploy with existing configuration
2. Monitor false positive reduction
3. Adjust confidence threshold if needed (via config)

## Success Metrics

**Quantitative**:
- False positive rate: <20% (down from ~60%)
- True positive rate: >90% (maintain)
- Stillwater validation examples: 0 coordinator detections
- Real coordinator patterns: 90%+ detection rate

**Qualitative**:
- User feedback: "Coordinator recommendations are now accurate"
- Reduced confusion about validation vs coordination
- Increased confidence in pattern detection

## Future Enhancements

**Method-Based State Detection** (Spec 193):
```rust
if current.get_state() != desired.get_state() {
    // Detect via method name patterns
}
```

**Machine Learning Classification** (future):
- Train on labeled examples
- Learn optimal keyword sets
- Adaptive confidence thresholds

**User-Defined Patterns** (future):
```toml
[pattern_detection.coordinator.custom]
state_getters = ["get_state", "current_mode", "status"]
action_types = ["MyAction", "CustomCommand"]
```

---
number: 204
title: Consolidate Pattern Display System
category: refactor
priority: high
status: draft
dependencies: [202, 203]
created: 2025-12-02
---

# Specification 204: Consolidate Pattern Display System

**Category**: refactor
**Priority**: high
**Status**: draft
**Dependencies**: [202, 203]

## Context

Debtmap has **two separate pattern detection systems** that produce inconsistent results:

1. **ComplexityPattern** (src/priority/complexity_patterns.rs): Used for recommendation rationale
2. **PatternInfo** (src/priority/formatter/context.rs): Used for display formatting

This duplication causes:
- Pattern detected in rationale ("Coordinator pattern with 4 actions") but no PATTERN line shown
- Repeated pattern detection logic (detect once for rationale, again for display)
- Inconsistent confidence thresholds and metrics
- Difficult debugging (which system is wrong?)

### Current Implementation Locations

**Pattern Detection**:
- `src/priority/complexity_patterns.rs:ComplexityPattern::detect()` → Used for recommendations
- `src/priority/formatter/context.rs:PatternInfo::from_item()` → Used for display
- `src/priority/formatter_verbosity.rs:format_pattern_detection()` → Terminal output
- `src/priority/formatter/sections.rs:format_pattern_section()` → Alternative formatter

**Root Cause**: Pattern detection is **imperative** (detect during formatting) instead of **declarative** (detect once, store result, format many times).

### Stillwater Violation

Current code violates **Single Source of Truth**:
- Pattern detection logic duplicated across 2+ locations
- Detection happens during formatting (should happen during analysis)
- Different code paths produce different results

## Objective

Consolidate pattern detection into a **single source of truth** in the core analysis phase, storing detected patterns in `UnifiedDebtItem` for all formatters to use consistently.

## Requirements

### Functional Requirements

1. **Single Pattern Detection**
   - Detect patterns once during analysis phase
   - Store result in `UnifiedDebtItem.detected_pattern: Option<DetectedPattern>`
   - All formatters read from stored result (no re-detection)

2. **Unified Pattern Type**
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub struct DetectedPattern {
       pub pattern_type: PatternType,
       pub confidence: f64,
       pub metrics: PatternMetrics,
   }

   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum PatternType {
       StateMachine,
       Coordinator,
       Validator,
   }

   #[derive(Debug, Clone, PartialEq)]
   pub struct PatternMetrics {
       pub state_transitions: Option<usize>,
       pub match_expressions: Option<usize>,
       pub action_dispatches: Option<usize>,
       pub comparisons: Option<usize>,
   }
   ```

3. **Display Helpers**
   - `DetectedPattern::icon()` → "🔄", "🎯", "✓"
   - `DetectedPattern::type_name()` → "State Machine", "Coordinator", "Validator"
   - `DetectedPattern::display_metrics()` → ["transitions: 4", "matches: 2"]

4. **Backward Compatibility**
   - All existing output formats produce identical results
   - No breaking changes to public APIs
   - Migrate incrementally with deprecated wrappers

### Non-Functional Requirements

1. **Performance**: Detect patterns once, not multiple times during formatting
2. **Consistency**: All output formats show same pattern for same function
3. **Maintainability**: Single location to update pattern detection logic
4. **Testability**: Pure detection function, easy to test

## Acceptance Criteria

- [ ] `src/priority/detected_pattern.rs` exists with `DetectedPattern` type
- [ ] `UnifiedDebtItem.detected_pattern: Option<DetectedPattern>` field added
- [ ] Pattern detection happens in scoring phase (unified_scorer.rs or priority_analyzer.rs)
- [ ] `formatter_verbosity.rs` uses stored pattern (no re-detection)
- [ ] `formatter/sections.rs` uses stored pattern (no re-detection)
- [ ] `formatter_markdown.rs` uses stored pattern for markdown tables
- [ ] `ComplexityPattern` enum removed or deprecated
- [ ] `PatternInfo` struct removed (replaced by `DetectedPattern`)
- [ ] All tests pass with identical output
- [ ] Pattern detection code reduced from 4+ locations to 1

## Technical Details

### Implementation Approach

**Stage 1: Create DetectedPattern Type**

```rust
// src/priority/detected_pattern.rs

/// Detected complexity pattern with confidence and metrics
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub metrics: PatternMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    StateMachine,
    Coordinator,
    Validator,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatternMetrics {
    pub state_transitions: Option<usize>,
    pub match_expressions: Option<usize>,
    pub action_dispatches: Option<usize>,
    pub comparisons: Option<usize>,
}

impl DetectedPattern {
    /// Detect pattern from language-specific signals
    pub fn detect(language_specific: &Option<LanguageSpecificData>) -> Option<Self> {
        let rust_data = match language_specific {
            Some(LanguageSpecificData::Rust(data)) => data,
            _ => return None,
        };

        // Check state machine first (higher priority)
        if let Some(sm_signals) = &rust_data.state_machine_signals {
            if sm_signals.confidence >= 0.7 {
                return Some(Self {
                    pattern_type: PatternType::StateMachine,
                    confidence: sm_signals.confidence,
                    metrics: PatternMetrics {
                        state_transitions: Some(sm_signals.transition_count),
                        match_expressions: Some(sm_signals.match_expression_count),
                        action_dispatches: Some(sm_signals.action_dispatch_count),
                        comparisons: None,
                    },
                });
            }
        }

        // Check coordinator second
        if let Some(coord_signals) = &rust_data.coordinator_signals {
            if coord_signals.confidence >= 0.7 {
                return Some(Self {
                    pattern_type: PatternType::Coordinator,
                    confidence: coord_signals.confidence,
                    metrics: PatternMetrics {
                        state_transitions: None,
                        match_expressions: None,
                        action_dispatches: Some(coord_signals.actions),
                        comparisons: Some(coord_signals.comparisons),
                    },
                });
            }
        }

        None
    }

    /// Display icon for terminal output
    pub const fn icon(&self) -> &'static str {
        match self.pattern_type {
            PatternType::StateMachine => "🔄",
            PatternType::Coordinator => "🎯",
            PatternType::Validator => "✓",
        }
    }

    /// Display name for all output formats
    pub const fn type_name(&self) -> &'static str {
        match self.pattern_type {
            PatternType::StateMachine => "State Machine",
            PatternType::Coordinator => "Coordinator",
            PatternType::Validator => "Validator",
        }
    }

    /// Display metrics as formatted strings
    pub fn display_metrics(&self) -> Vec<String> {
        let mut metrics = Vec::new();

        if let Some(transitions) = self.metrics.state_transitions {
            metrics.push(format!("transitions: {}", transitions));
        }
        if let Some(matches) = self.metrics.match_expressions {
            metrics.push(format!("matches: {}", matches));
        }
        if let Some(actions) = self.metrics.action_dispatches {
            metrics.push(format!("actions: {}", actions));
        }
        if let Some(comparisons) = self.metrics.comparisons {
            metrics.push(format!("comparisons: {}", comparisons));
        }

        metrics
    }
}
```

**Stage 2: Add Field to UnifiedDebtItem**

```rust
// src/priority/unified_scorer.rs

pub struct UnifiedDebtItem {
    // ... existing fields
    pub detected_pattern: Option<DetectedPattern>,
}
```

**Stage 3: Detect During Analysis**

```rust
// src/priority/unified_scorer.rs or priority_analyzer.rs

fn create_unified_debt_item(/* ... */) -> UnifiedDebtItem {
    let detected_pattern = DetectedPattern::detect(&language_specific);

    UnifiedDebtItem {
        // ... existing fields
        detected_pattern,
    }
}
```

**Stage 4: Update Formatters**

```rust
// src/priority/formatter_verbosity.rs

fn format_pattern_detection(output: &mut String, item: &UnifiedDebtItem) {
    if let Some(pattern) = &item.detected_pattern {
        let metrics_str = pattern.display_metrics().join(", ");
        writeln!(
            output,
            "├─ PATTERN: {} {} ({}, confidence: {:.2})",
            pattern.icon(),
            pattern.type_name().bright_magenta().bold(),
            metrics_str.cyan(),
            pattern.confidence
        ).unwrap();
    }
}
```

**Stage 5: Remove Duplicates**

1. Remove `PatternInfo` from `formatter/context.rs`
2. Remove `format_pattern_section` duplication
3. Deprecate `ComplexityPattern::detect()` (if still needed for rationale generation)
4. Update all pattern references to use `item.detected_pattern`

### Architecture Changes

**Before**:
```
Pattern Detection Locations:
├── complexity_patterns.rs (ComplexityPattern::detect)
├── formatter/context.rs (PatternInfo::from_item)
├── formatter_verbosity.rs (format_pattern_detection - reconstructs ComplexityMetrics)
└── formatter/sections.rs (format_pattern_section - uses PatternInfo)
```

**After**:
```
Pattern Detection:
├── detected_pattern.rs (DetectedPattern::detect - SINGLE SOURCE OF TRUTH)
└── unified_scorer.rs (stores result in UnifiedDebtItem)

Pattern Display:
├── formatter_verbosity.rs (reads item.detected_pattern)
├── formatter/sections.rs (reads item.detected_pattern)
└── formatter_markdown.rs (reads item.detected_pattern)
```

### Data Structures

```rust
// Core pattern data (stored in UnifiedDebtItem)
pub struct DetectedPattern {
    pub pattern_type: PatternType,      // What kind of pattern?
    pub confidence: f64,                 // How confident are we?
    pub metrics: PatternMetrics,         // Pattern-specific metrics
}

// Pattern-specific metrics (flexible for different patterns)
pub struct PatternMetrics {
    pub state_transitions: Option<usize>,   // State Machine: transitions
    pub match_expressions: Option<usize>,   // State Machine: match arms
    pub action_dispatches: Option<usize>,   // State Machine + Coordinator: actions
    pub comparisons: Option<usize>,         // Coordinator: comparisons
}
```

## Dependencies

- **Prerequisites**:
  - Spec 202 (Extract Severity/Coverage Classification) - establishes pure classification pattern
  - Spec 203 (Separate Pure Formatting from I/O) - establishes format/write separation
- **Affected Components**:
  - `src/priority/unified_scorer.rs` (adds detected_pattern field)
  - `src/priority/detected_pattern.rs` (new file)
  - `src/priority/complexity_patterns.rs` (deprecate or remove)
  - `src/priority/formatter_verbosity.rs` (simplify pattern display)
  - `src/priority/formatter/context.rs` (remove PatternInfo)
  - `src/priority/formatter/sections.rs` (simplify pattern section)
  - `src/priority/formatter_markdown.rs` (use stored pattern)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_state_machine_pattern() {
        let rust_data = create_test_rust_data_with_state_machine();
        let pattern = DetectedPattern::detect(&Some(LanguageSpecificData::Rust(rust_data)));

        assert!(pattern.is_some());
        let pattern = pattern.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::StateMachine);
        assert!(pattern.confidence >= 0.7);
        assert_eq!(pattern.metrics.state_transitions, Some(4));
    }

    #[test]
    fn detect_coordinator_pattern() {
        let rust_data = create_test_rust_data_with_coordinator();
        let pattern = DetectedPattern::detect(&Some(LanguageSpecificData::Rust(rust_data)));

        assert!(pattern.is_some());
        let pattern = pattern.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::Coordinator);
        assert_eq!(pattern.metrics.action_dispatches, Some(4));
        assert_eq!(pattern.metrics.comparisons, Some(2));
    }

    #[test]
    fn no_pattern_below_threshold() {
        let mut rust_data = create_test_rust_data_with_state_machine();
        rust_data.state_machine_signals.as_mut().unwrap().confidence = 0.6;

        let pattern = DetectedPattern::detect(&Some(LanguageSpecificData::Rust(rust_data)));
        assert!(pattern.is_none());
    }

    #[test]
    fn display_metrics_formatting() {
        let pattern = DetectedPattern {
            pattern_type: PatternType::Coordinator,
            confidence: 0.85,
            metrics: PatternMetrics {
                action_dispatches: Some(4),
                comparisons: Some(2),
                state_transitions: None,
                match_expressions: None,
            },
        };

        let metrics = pattern.display_metrics();
        assert_eq!(metrics, vec!["actions: 4", "comparisons: 2"]);
    }
}
```

### Integration Tests

```rust
#[test]
fn pattern_detected_once_used_everywhere() {
    let analysis = analyze_test_codebase();
    let item = &analysis.items[0];

    // Pattern should be detected and stored
    assert!(item.detected_pattern.is_some());

    // All formatters should show same pattern
    let terminal_output = format_terminal(&item);
    let markdown_output = format_markdown(&item);

    assert!(terminal_output.contains("🎯 Coordinator"));
    assert!(markdown_output.contains("Coordinator"));
    assert!(terminal_output.contains("confidence: 0.85"));
    assert!(markdown_output.contains("0.85"));
}
```

### Regression Tests

```rust
#[test]
fn output_unchanged_after_consolidation() {
    let analysis = load_test_analysis();

    // Capture output before consolidation
    let expected = include_str!("../test_data/expected_pattern_output.txt");

    // Generate output after consolidation
    let actual = format_priorities_terminal(&analysis);

    assert_eq!(actual, expected);
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Detected complexity pattern with confidence and metrics.
///
/// Patterns are detected once during analysis and stored in `UnifiedDebtItem`.
/// All output formatters read from this stored result for consistency.
///
/// # Pattern Types
///
/// - **State Machine**: Functions with explicit state transitions, match expressions
/// - **Coordinator**: Functions that orchestrate actions based on comparisons
/// - **Validator**: Functions with validation logic (future)
///
/// # Confidence Threshold
///
/// Patterns are only reported if confidence ≥ 0.7
///
/// # Examples
///
/// ```
/// use debtmap::priority::detected_pattern::DetectedPattern;
///
/// let pattern = DetectedPattern::detect(&language_specific);
/// if let Some(p) = pattern {
///     println!("{} {}", p.icon(), p.type_name());
/// }
/// ```
pub struct DetectedPattern { ... }
```

### Architecture Updates

Update ARCHITECTURE.md:

```markdown
## Pattern Detection

The `priority::detected_pattern` module provides pattern detection:

- Detects complexity patterns once during analysis phase
- Stores result in `UnifiedDebtItem.detected_pattern`
- All formatters read from stored result (single source of truth)

Pattern types: State Machine, Coordinator, Validator
Confidence threshold: 0.7
```

## Implementation Notes

### Key Principles

1. **Detect Once, Use Many**: Pattern detection is expensive, do it once
2. **Store Results**: Keep detected pattern in core data structure
3. **Pure Display**: Formatters only display, never detect
4. **Single Source of Truth**: One detection function, one storage location

### Common Pitfalls

- **Don't** re-detect patterns during formatting
- **Don't** have format-specific pattern logic
- **Do** ensure all formatters read from `item.detected_pattern`
- **Do** test that all formats show identical pattern info

### Performance Considerations

- Pattern detection happens once during analysis (not per formatter)
- Display helpers are lightweight (no re-computation)
- Memory overhead: ~50 bytes per item (negligible)

## Migration and Compatibility

### Breaking Changes

None. This is internal refactoring with backward-compatible output.

### Migration Path

1. **v0.8.0**: Add `detected_pattern` field, detect during analysis
2. **v0.8.1**: Migrate all formatters to use stored pattern
3. **v0.8.2**: Remove duplicate detection code

### Rollback Strategy

If issues are discovered:
1. Revert to format-time detection temporarily
2. Fix detection logic in `detected_pattern.rs`
3. Re-run migration

## Success Metrics

- **Code Reduction**: Remove 3+ duplicate detection implementations → 1
- **Performance**: Pattern detection happens once (not 3+ times per item)
- **Consistency**: All formats show identical pattern info
- **Maintainability**: Single location to update pattern detection logic

## Timeline

- **Day 1**: Create `detected_pattern.rs` with detection logic (4h)
- **Day 2**: Add field to `UnifiedDebtItem`, detect during analysis (3h)
- **Day 3**: Update `formatter_verbosity.rs` to use stored pattern (2h)
- **Day 4**: Update `formatter/sections.rs` and `formatter_markdown.rs` (3h)
- **Day 5**: Remove duplicate code, testing, documentation (4h)

**Total Effort**: 16 hours (2 person-days)
